//! 选人悬浮窗：横条小窗（我的英雄、板凳英雄、出装、召唤师技能）。
//!
//! 吸附在英雄联盟客户端窗口上，默认位于顶部板凳卡片下方、水平居中；
//! 客户端移动时跟随；用户拖动后记住相对客户端的偏移。
//! 对局中的强化卡片由透明悬浮层（ocr 模块）负责。
//!
//! 只按窗口类名查询客户端窗口的位置，不打开任何进程。

use crate::state::AppState;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Manager, PhysicalPosition};

pub const LABEL: &str = "float";

/// 板凳卡片下沿大约在客户端高度的这个比例处
const BELOW_CARDS: f64 = 0.22;

/// 用户本阶段手动关掉了悬浮窗；阶段切换后重置
static DISMISSED: AtomicBool = AtomicBool::new(false);
/// 拖动时防抖保存位置
static MOVE_GEN: AtomicU64 = AtomicU64::new(0);
static FOLLOWING: AtomicBool = AtomicBool::new(false);

/// 英雄联盟客户端窗口（x, y, w, h）；最小化或找不到时为 None
#[cfg(windows)]
fn client_rect() -> Option<(i32, i32, i32, i32)> {
    use windows::core::{w, PCWSTR};
    use windows::Win32::Foundation::RECT;
    use windows::Win32::UI::WindowsAndMessaging::{FindWindowW, GetWindowRect, IsIconic};
    let hwnd = unsafe { FindWindowW(w!("RCLIENT"), PCWSTR::null()) }.ok()?;
    if hwnd.is_invalid() || unsafe { IsIconic(hwnd) }.as_bool() {
        return None;
    }
    let mut r = RECT::default();
    unsafe { GetWindowRect(hwnd, &mut r) }.ok()?;
    let (w, h) = (r.right - r.left, r.bottom - r.top);
    (w > 400 && h > 300).then_some((r.left, r.top, w, h))
}

#[cfg(not(windows))]
fn client_rect() -> Option<(i32, i32, i32, i32)> {
    None
}

fn in_champ_select(state: &AppState) -> bool {
    state.phase() == "ChampSelect"
}

fn wanted(state: &AppState) -> bool {
    state.settings.read().unwrap().float_enabled
        && !DISMISSED.load(Ordering::Relaxed)
        && in_champ_select(state)
}

/// 阶段变化时调用：新阶段重新允许显示，并按新阶段重新定位
pub fn on_phase(app: &AppHandle, state: &Arc<AppState>) {
    DISMISSED.store(false, Ordering::Relaxed);
    if let Some(w) = app.get_webview_window(LABEL) {
        let _ = w.hide();
    }
    refresh(app, state);
}

/// 按当前阶段与设置显示 / 隐藏
pub fn refresh(app: &AppHandle, state: &Arc<AppState>) {
    let Some(w) = app.get_webview_window(LABEL) else { return };
    if !wanted(state) {
        let _ = w.hide();
        return;
    }
    if !w.is_visible().unwrap_or(false) {
        if let Some((x, y)) = target(&w, state) {
            let _ = w.set_position(PhysicalPosition::new(x, y));
        }
        let _ = w.show();
    }
    if in_champ_select(state) && !FOLLOWING.swap(true, Ordering::SeqCst) {
        tauri::async_runtime::spawn(follow_client(app.clone(), state.clone()));
    }
}

/// 用户点了关闭：本阶段不再自动弹出
pub fn dismiss(app: &AppHandle) {
    DISMISSED.store(true, Ordering::Relaxed);
    if let Some(w) = app.get_webview_window(LABEL) {
        let _ = w.hide();
    }
}

/// 计算应在的位置
fn target(w: &tauri::WebviewWindow, state: &AppState) -> Option<(i32, i32)> {
    let size = w.outer_size().ok()?;
    let (fw, fh) = (size.width as i32, size.height as i32);
    if in_champ_select(state) {
        if let Some((cx, cy, cw, ch)) = client_rect() {
            let (dx, dy) = state.settings.read().unwrap().float_offset.unwrap_or((
                (cw - fw) / 2,
                (ch as f64 * BELOW_CARDS) as i32,
            ));
            return Some((cx + dx, cy + dy));
        }
    }
    if let Some((x, y)) = state.settings.read().unwrap().float_pos {
        // 位置仍在某个显示器内才使用（显示器可能已拔掉）
        let inside = w.available_monitors().unwrap_or_default().iter().any(|m| {
            let (p, s) = (m.position(), m.size());
            x >= p.x && y >= p.y && x < p.x + s.width as i32 - 40 && y < p.y + s.height as i32 - 40
        });
        if inside {
            return Some((x, y));
        }
    }
    let m = w.primary_monitor().ok().flatten()?;
    let (p, s) = (m.position(), m.size());
    Some((p.x + s.width as i32 - fw - 24, p.y + s.height as i32 / 6 - fh / 6))
}

/// 选人期间每 100ms 检查一次客户端位置（只读窗口矩形，开销可忽略），客户端移动时跟随
async fn follow_client(app: AppHandle, state: Arc<AppState>) {
    while in_champ_select(&state) {
        if let Some(w) = app.get_webview_window(LABEL) {
            if w.is_visible().unwrap_or(false) {
                if let (Some((x, y)), Ok(cur)) = (target(&w, &state), w.outer_position()) {
                    if (cur.x - x).abs() > 2 || (cur.y - y).abs() > 2 {
                        let _ = w.set_position(PhysicalPosition::new(x, y));
                    }
                }
            }
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    FOLLOWING.store(false, Ordering::SeqCst);
}

/// 窗口被拖动：选人时记相对客户端的偏移，否则记绝对位置；停止拖动 600ms 后写入设置
pub fn on_moved(app: &AppHandle, state: &Arc<AppState>, x: i32, y: i32) {
    {
        let mut s = state.settings.write().unwrap();
        match (in_champ_select(state), client_rect()) {
            (true, Some((cx, cy, _, _))) => s.float_offset = Some((x - cx, y - cy)),
            _ => s.float_pos = Some((x, y)),
        }
    }
    let gen = MOVE_GEN.fetch_add(1, Ordering::SeqCst) + 1;
    let (app, state) = (app.clone(), state.clone());
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(Duration::from_millis(600)).await;
        if MOVE_GEN.load(Ordering::SeqCst) == gen {
            let s = state.settings.read().unwrap().clone();
            let _ = crate::settings::save(&app, &s);
        }
    });
}
