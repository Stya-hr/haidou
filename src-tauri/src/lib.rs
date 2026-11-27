mod apply;
mod arammeta;
mod data;
mod float;
mod i18n;
mod lcu;
mod live;
mod ocr;
mod paths;
mod settings;
mod state;

use settings::Settings;
use state::{AppState, Snapshot};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager, State, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_opener::OpenerExt;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

type S<'a> = State<'a, Arc<AppState>>;

#[tauri::command]
fn get_snapshot(state: S) -> Snapshot {
    state.snapshot.lock().unwrap().clone()
}

#[tauri::command]
fn get_catalog(state: S) -> Option<data::Catalog> {
    state.catalog.read().unwrap().as_deref().cloned()
}

/// 实际生效的界面语言（"auto" 已按系统语言解析）
#[tauri::command]
fn get_ui_lang() -> &'static str {
    i18n::code(i18n::get())
}

#[tauri::command]
fn get_settings(state: S) -> Settings {
    state.settings.read().unwrap().clone()
}

#[tauri::command]
async fn save_settings(app: AppHandle, state: S<'_>, mut settings: Settings) -> Result<(), String> {
    // 悬浮窗位置由拖动事件在后端记录，前端传来的可能是旧值
    {
        let cur = state.settings.read().unwrap();
        settings.float_pos = cur.float_pos;
        settings.float_offset = cur.float_offset;
    }
    if !settings.ocr_hotkey.trim().is_empty() {
        settings
            .ocr_hotkey
            .parse::<tauri_plugin_global_shortcut::Shortcut>()
            .map_err(|e| format!("{}: {e}", i18n::t("bad_hotkey")))?;
    }
    settings::save(&app, &settings).map_err(|e| e.to_string())?;
    i18n::set(i18n::resolve(&settings.ui_lang));
    let _ = app.emit("ui-lang", i18n::code(i18n::get()));
    let old = std::mem::replace(&mut *state.settings.write().unwrap(), settings.clone());
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.set_always_on_top(settings.always_on_top);
    }
    float::refresh(&app, state.inner());
    if old.ocr_hotkey != settings.ocr_hotkey {
        register_hotkey(&app, &settings.ocr_hotkey);
    }
    let data_changed = old.stats_url != settings.stats_url
        || old.stats_source != settings.stats_source
        || old.cdragon_base != settings.cdragon_base
        || old.ui_lang != settings.ui_lang
        || old.zh_variant != settings.zh_variant;
    if data_changed {
        let st = state.inner().clone();
        tauri::async_runtime::spawn(async move {
            if let Err(e) = data::sync(app, st, false).await {
                log::warn!("数据同步失败：{e:#}");
            }
        });
    }
    Ok(())
}

#[tauri::command]
async fn refresh_data(app: AppHandle, state: S<'_>) -> Result<(), String> {
    data::sync(app, state.inner().clone(), true)
        .await
        .map_err(|e| format!("{e:#}"))
}

/// 以下操作都由玩家点击按钮触发，等同于在客户端里手动操作。
#[tauri::command]
async fn bench_swap(state: S<'_>, champion_id: i64) -> Result<(), String> {
    let c = state.lcu().ok_or(i18n::t("not_connected"))?;
    c.post(&format!("/lol-champ-select/v1/session/bench/swap/{champion_id}"))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn reroll(state: S<'_>) -> Result<(), String> {
    let c = state.lcu().ok_or(i18n::t("not_connected"))?;
    c.post("/lol-champ-select/v1/session/my-selection/reroll")
        .await
        .map_err(|e| e.to_string())
}

/// what: "items" | "spells" | "all"
#[tauri::command]
async fn apply_build(app: AppHandle, state: S<'_>, champion_id: i64, what: String) -> Result<String, String> {
    let client = state.lcu().ok_or(i18n::t("not_connected"))?;
    data::ensure_detail(&app, state.inner(), champion_id).await.map_err(|e| format!("{e:#}"))?;
    let cat = state.catalog.read().unwrap().clone().ok_or(i18n::t("data_not_loaded"))?;
    let st = cat
        .stats
        .as_ref()
        .and_then(|s| s.champions.get(&champion_id.to_string()))
        .ok_or(i18n::t("no_champ_data"))?;
    let all = what == "all";
    let which = apply::Which {
        items: all || what == "items",
        spells: all || what == "spells",
    };
    let msgs = apply::apply_all(&client, &cat, champion_id, st, which).await;
    if msgs.is_empty() {
        return Err(i18n::t("nothing_to_import").into());
    }
    Ok(msgs.join("  "))
}

/// 把游戏显示模式改成无边框（下次进游戏生效）
#[tauri::command]
async fn set_borderless(app: AppHandle, state: S<'_>) -> Result<(), String> {
    let c = state.lcu().ok_or(i18n::t("not_connected"))?;
    if state.phase() == "InProgress" {
        return Err(i18n::t("in_game_no_change").into());
    }
    let mode = lcu::set_borderless(&c).await.map_err(|e| format!("{e:#}"))?;
    state.update(&app, |s| s.window_mode = Some(mode));
    Ok(())
}

/// 界面上显示某个英雄时按需加载其详情（出装、完整强化表）
#[tauri::command]
async fn ensure_champion(app: AppHandle, state: S<'_>, champion_id: i64) -> Result<(), String> {
    data::ensure_detail(&app, state.inner(), champion_id)
        .await
        .map_err(|e| format!("{e:#}"))
}

/// 只允许打开数据来源站点的链接
#[tauri::command]
fn open_link(app: AppHandle, url: String) -> Result<(), String> {
    if !url.starts_with(&format!("{}/", arammeta::SITE)) {
        return Err(i18n::t("link_denied").into());
    }
    app.opener().open_url(url, None::<&str>).map_err(|e| e.to_string())
}

/// 悬浮窗右上角的关闭按钮：本阶段不再自动弹出
#[tauri::command]
fn float_dismiss(app: AppHandle) {
    float::dismiss(&app);
}

/// 测试悬浮层：不依赖 OCR，在游戏画面（找不到时为主显示器）上显示几秒示例徽章
#[tauri::command]
async fn overlay_test(app: AppHandle, state: S<'_>) -> Result<(), String> {
    let rect = ocr::game_rect().or_else(|| {
        let w = app.get_webview_window("main")?;
        let m = w.primary_monitor().ok()??;
        let (p, s) = (m.position(), m.size());
        Some((p.x, p.y, s.width as i32, s.height as i32))
    });
    let rect = rect.ok_or("no monitor")?;
    ocr::position_overlay(&app, true, rect);
    let _ = app.emit("overlay-test", true);
    let st = state.inner().clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        let _ = app.emit("overlay-test", false);
        if st.snapshot.lock().unwrap().offer.is_empty() {
            ocr::position_overlay(&app, false, rect);
        }
    });
    Ok(())
}

#[tauri::command]
fn ocr_scan_now(state: S) {
    ocr::arm(&state);
}

/// 自己的悬浮窗口不进入截屏（Windows 10 2004+）：否则 OCR 会读到悬浮条上的强化名，
/// 卡片关掉后悬浮条也不会消失。录屏 / 截图软件同样看不到这些窗口。
fn exclude_from_capture(w: &tauri::WebviewWindow) {
    #[cfg(windows)]
    {
        use windows::Win32::Foundation::HWND;
        use windows::Win32::UI::WindowsAndMessaging::{SetWindowDisplayAffinity, WDA_EXCLUDEFROMCAPTURE};
        if let Ok(h) = w.hwnd() {
            if let Err(e) = unsafe { SetWindowDisplayAffinity(HWND(h.0), WDA_EXCLUDEFROMCAPTURE) } {
                log::warn!("排除截屏失败（系统低于 Windows 10 2004？）：{e}");
            }
        }
    }
    #[cfg(not(windows))]
    let _ = w;
}

fn register_hotkey(app: &AppHandle, key: &str) {
    let gs = app.global_shortcut();
    let _ = gs.unregister_all();
    if key.trim().is_empty() {
        return;
    }
    let res = gs.on_shortcut(key.trim(), |app, _, ev| {
        if ev.state == ShortcutState::Pressed {
            ocr::arm(&app.state::<Arc<AppState>>());
        }
    });
    if let Err(e) = res {
        log::warn!("注册热键 {key} 失败：{e}");
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let state = Arc::new(AppState::default());
            let s = settings::load(app.handle());
            i18n::set(i18n::resolve(&s.ui_lang));

            // 便携版：WebView2 用户数据也放在 exe 旁边
            let webview_dir = paths::data_dir(app.handle()).join("webview");
            WebviewWindowBuilder::new(app, "main", WebviewUrl::default())
                .title("HexPilot · 海克斯领航")
                .inner_size(380.0, 680.0)
                .min_inner_size(340.0, 420.0)
                .always_on_top(s.always_on_top)
                .data_directory(webview_dir.clone())
                .build()?;
            // 悬浮层：透明、置顶、鼠标穿透、不抢焦点、不进任务栏
            let overlay = WebviewWindowBuilder::new(app, "overlay", WebviewUrl::default())
                .title("HexPilot 悬浮层")
                .inner_size(800.0, 600.0)
                .transparent(true)
                .decorations(false)
                .always_on_top(true)
                .skip_taskbar(true)
                .visible(false)
                .focused(false)
                .shadow(false)
                .resizable(false)
                .data_directory(webview_dir)
                .build()?;
            // Windows 上可以对隐藏窗口预先设置鼠标穿透；Linux 上窗口未显示时调用会崩溃，
            // 显示悬浮层时（ocr::position_overlay）还会再设置一次
            #[cfg(windows)]
            let _ = overlay.set_ignore_cursor_events(true);
            exclude_from_capture(&overlay);
            #[cfg(not(windows))]
            let _ = overlay;
            // 悬浮窗：选人 / 对局时显示的横条小窗，可拖动，不进任务栏
            let float_win = WebviewWindowBuilder::new(app, float::LABEL, WebviewUrl::default())
                .title("HexPilot")
                .inner_size(600.0, 60.0)
                .transparent(true)
                .decorations(false)
                .always_on_top(true)
                .skip_taskbar(true)
                .visible(false)
                .focused(false)
                .shadow(false)
                .resizable(false)
                .data_directory(paths::data_dir(app.handle()).join("webview"))
                .build()?;
            exclude_from_capture(&float_win);
            register_hotkey(app.handle(), &s.ocr_hotkey);
            *state.settings.write().unwrap() = s;
            app.manage(state.clone());

            let h = app.handle().clone();
            let st = state.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(e) = data::sync(h, st, false).await {
                    log::warn!("数据同步失败：{e:#}");
                }
            });
            tauri::async_runtime::spawn(data::auto_update(app.handle().clone(), state.clone()));
            {
                let (h, st) = (app.handle().clone(), state.clone());
                tauri::async_runtime::spawn(async move {
                    let langs = ocr::installed().await;
                    log::info!("已安装的 OCR 语言：{langs:?}");
                    st.update(&h, |s| s.ocr_installed = langs);
                });
            }
            tauri::async_runtime::spawn(lcu::supervise(app.handle().clone(), state));
            Ok(())
        })
        .on_window_event(|w, ev| {
            if w.label() == float::LABEL {
                if let tauri::WindowEvent::Moved(p) = ev {
                    let app = w.app_handle();
                    float::on_moved(app, &app.state::<Arc<AppState>>(), p.x, p.y);
                }
            }
            // 关闭主窗口即退出（悬浮层是隐藏窗口，不能让它把进程留在后台）
            if w.label() == "main" {
                if let tauri::WindowEvent::CloseRequested { .. } = ev {
                    w.app_handle().exit(0);
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_snapshot,
            get_catalog,
            get_settings,
            get_ui_lang,
            save_settings,
            refresh_data,
            bench_swap,
            reroll,
            apply_build,
            ensure_champion,
            open_link,
            set_borderless,
            float_dismiss,
            overlay_test,
            ocr_scan_now
        ])
        .run(tauri::generate_context!())
        .expect("启动失败");
}
