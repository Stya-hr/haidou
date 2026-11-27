mod apply;
mod arammeta;
mod data;
mod lcu;
mod live;
mod ocr;
mod paths;
mod settings;
mod state;

use settings::Settings;
use state::{AppState, Snapshot};
use std::sync::Arc;
use tauri::{AppHandle, Manager, State, WebviewUrl, WebviewWindowBuilder};
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

#[tauri::command]
fn get_settings(state: S) -> Settings {
    state.settings.read().unwrap().clone()
}

#[tauri::command]
async fn save_settings(app: AppHandle, state: S<'_>, settings: Settings) -> Result<(), String> {
    if !settings.ocr_hotkey.trim().is_empty() {
        settings
            .ocr_hotkey
            .parse::<tauri_plugin_global_shortcut::Shortcut>()
            .map_err(|e| format!("热键格式不正确：{e}"))?;
    }
    settings::save(&app, &settings).map_err(|e| e.to_string())?;
    let old = std::mem::replace(&mut *state.settings.write().unwrap(), settings.clone());
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.set_always_on_top(settings.always_on_top);
    }
    if old.ocr_hotkey != settings.ocr_hotkey {
        register_hotkey(&app, &settings.ocr_hotkey);
    }
    let data_changed = old.stats_url != settings.stats_url
        || old.stats_source != settings.stats_source
        || old.cdragon_base != settings.cdragon_base
        || old.locale != settings.locale;
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
    let c = state.lcu().ok_or("未连接客户端")?;
    c.post(&format!("/lol-champ-select/v1/session/bench/swap/{champion_id}"))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn reroll(state: S<'_>) -> Result<(), String> {
    let c = state.lcu().ok_or("未连接客户端")?;
    c.post("/lol-champ-select/v1/session/my-selection/reroll")
        .await
        .map_err(|e| e.to_string())
}

/// what: "items" | "spells" | "all"
#[tauri::command]
async fn apply_build(app: AppHandle, state: S<'_>, champion_id: i64, what: String) -> Result<String, String> {
    let client = state.lcu().ok_or("未连接客户端")?;
    data::ensure_detail(&app, state.inner(), champion_id).await.map_err(|e| format!("{e:#}"))?;
    let cat = state.catalog.read().unwrap().clone().ok_or("数据尚未加载")?;
    let st = cat
        .stats
        .as_ref()
        .and_then(|s| s.champions.get(&champion_id.to_string()))
        .ok_or("数据包中没有该英雄")?;
    let all = what == "all";
    let which = apply::Which {
        items: all || what == "items",
        spells: all || what == "spells",
    };
    let msgs = apply::apply_all(&client, &cat, champion_id, st, which).await;
    if msgs.is_empty() {
        return Err("数据包中没有可导入的内容".into());
    }
    Ok(msgs.join("  "))
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
        return Err("不允许打开该链接".into());
    }
    app.opener().open_url(url, None::<&str>).map_err(|e| e.to_string())
}

#[tauri::command]
fn ocr_scan_now(state: S) {
    ocr::arm(&state);
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

            // 便携版：WebView2 用户数据也放在 exe 旁边
            let webview_dir = paths::data_dir(app.handle()).join("webview");
            WebviewWindowBuilder::new(app, "main", WebviewUrl::default())
                .title("海斗助手")
                .inner_size(380.0, 680.0)
                .min_inner_size(340.0, 420.0)
                .always_on_top(s.always_on_top)
                .data_directory(webview_dir.clone())
                .build()?;
            // 悬浮层：透明、置顶、鼠标穿透、不抢焦点、不进任务栏
            let overlay = WebviewWindowBuilder::new(app, "overlay", WebviewUrl::default())
                .title("海斗悬浮层")
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
            #[cfg(not(windows))]
            let _ = overlay;
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
            tauri::async_runtime::spawn(lcu::supervise(app.handle().clone(), state));
            Ok(())
        })
        .on_window_event(|w, ev| {
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
            save_settings,
            refresh_data,
            bench_swap,
            reroll,
            apply_build,
            ensure_champion,
            open_link,
            ocr_scan_now
        ])
        .run(tauri::generate_context!())
        .expect("启动失败");
}
