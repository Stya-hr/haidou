pub mod auth;
pub mod http;
pub mod ws;

use crate::state::{AppState, ChampSelectView, TeamMember};
use http::LcuClient;
use serde_json::Value;
use std::sync::atomic::Ordering;
use tauri::Emitter;
use std::sync::Arc;
use std::time::Duration;
use tauri::AppHandle;
use tokio::sync::mpsc;

pub const MAYHEM_QUEUE: i64 = 2400;

/// 常驻任务：发现客户端 → 连接 → 订阅事件；断线后自动重连。
pub async fn supervise(app: AppHandle, state: Arc<AppState>) {
    let mut disc = auth::Discoverer::new();
    loop {
        let dir = state.settings.read().unwrap().league_dir.clone();
        let Some(creds) = disc.discover(Some(&dir)) else {
            set_disconnected(&app, &state);
            tokio::time::sleep(Duration::from_secs(3)).await;
            continue;
        };
        let Ok(client) = LcuClient::new(&creds) else {
            tokio::time::sleep(Duration::from_secs(3)).await;
            continue;
        };
        // 客户端刚启动时接口可能尚未就绪（或 lockfile 是上次残留），先探活。
        let Ok(phase) = client.get("/lol-gameflow/v1/gameflow-phase").await else {
            tokio::time::sleep(Duration::from_secs(2)).await;
            continue;
        };

        *state.lcu.write().unwrap() = Some(client.clone());
        let mode = read_window_mode(&client).await;
        state.update(&app, |s| {
            s.connected = true;
            s.window_mode = mode;
        });
        log::info!("已连接 LCU，端口 {}", creds.port);

        let (tx, mut rx) = mpsc::channel::<ws::LcuEvent>(64);
        let ws_client = client.clone();
        let ws_task = tokio::spawn(async move { ws::run(creds.port, &ws_client, tx).await });

        // 订阅建立后再补一次当前状态，防止错过连接前已发生的事件。
        detect_locale(&app, &state, &client).await;
        on_phase(&app, &state, &client, phase.as_str().unwrap_or("None")).await;
        if let Ok(rc) = client.get("/lol-matchmaking/v1/ready-check").await {
            on_ready_check(&state, &client, &rc);
        }

        while let Some(ev) = rx.recv().await {
            match ev.uri.as_str() {
                "/lol-gameflow/v1/gameflow-phase" => {
                    on_phase(&app, &state, &client, ev.data.as_str().unwrap_or("None")).await
                }
                "/lol-champ-select/v1/session" => {
                    if ev.event_type == "Delete" {
                        state.update(&app, |s| s.champ_select = None);
                    } else {
                        on_session(&app, &state, &ev.data);
                    }
                }
                "/lol-matchmaking/v1/ready-check" => on_ready_check(&state, &client, &ev.data),
                _ => {}
            }
        }

        if let Ok(Err(e)) = ws_task.await {
            log::warn!("LCU WebSocket 断开：{e}");
        }
        set_disconnected(&app, &state);
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
}

fn set_disconnected(app: &AppHandle, state: &AppState) {
    *state.lcu.write().unwrap() = None;
    state.update(app, |s| {
        s.connected = false;
        s.phase = "None".into();
        s.champ_select = None;
        s.queue_id = None;
        s.window_mode = None;
    });
}

async fn on_phase(app: &AppHandle, state: &Arc<AppState>, client: &LcuClient, phase: &str) {
    state.update(app, |s| {
        s.phase = phase.to_string();
        if phase != "ChampSelect" {
            s.champ_select = None;
        }
    });
    if phase != "ChampSelect" {
        state.applied_champ.store(0, Ordering::SeqCst);
    }
    match phase {
        "ChampSelect" | "InProgress" | "GameStart" | "Reconnect" => {
            if let Ok(sess) = client.get("/lol-gameflow/v1/session").await {
                let q = sess["gameData"]["queue"]["id"].as_i64();
                state.update(app, |s| s.queue_id = q);
            }
            if phase == "ChampSelect" {
                if let Ok(cs) = client.get("/lol-champ-select/v1/session").await {
                    on_session(app, state, &cs);
                }
                // 进游戏前检查显示模式：独占全屏下无法识别强化、悬浮层也无法显示
                let mode = read_window_mode(client).await;
                state.update(app, |s| s.window_mode = mode);
                if mode == Some(0) && state.settings.read().unwrap().ocr_enabled {
                    let _ = app.emit("toast", "当前是全屏模式，强化识别不可用。可在「选人」页一键改为无边框".to_string());
                }
            }
            if phase == "InProgress" {
                tauri::async_runtime::spawn(crate::live::poll(app.clone(), state.clone()));
                tauri::async_runtime::spawn(crate::ocr::run(app.clone(), state.clone()));
            }
        }
        "None" | "Lobby" => state.update(app, |s| s.queue_id = None),
        _ => {}
    }
}

pub fn parse_session(v: &Value) -> ChampSelectView {
    let me = v["localPlayerCellId"].as_i64().unwrap_or(-1);
    let team: Vec<TeamMember> = v["myTeam"]
        .as_array()
        .map(|a| {
            a.iter()
                .map(|m| {
                    let cell_id = m["cellId"].as_i64().unwrap_or(-1);
                    TeamMember {
                        cell_id,
                        champion_id: m["championId"].as_i64().unwrap_or(0),
                        is_me: cell_id == me,
                    }
                })
                .collect()
        })
        .unwrap_or_default();
    // 新版客户端是 benchChampions: [{championId}]，旧版是 benchChampionIds: [id]
    let bench = v["benchChampions"]
        .as_array()
        .map(|a| a.iter().filter_map(|b| b["championId"].as_i64()).collect())
        .or_else(|| {
            v["benchChampionIds"]
                .as_array()
                .map(|a| a.iter().filter_map(Value::as_i64).collect())
        })
        .unwrap_or_default();
    ChampSelectView {
        my_champion_id: team.iter().find(|m| m.is_me).map(|m| m.champion_id).unwrap_or(0),
        bench_enabled: v["benchEnabled"].as_bool().unwrap_or(false),
        bench,
        team,
        rerolls_remaining: v["rerollsRemaining"].as_i64().unwrap_or(0),
    }
}

fn on_session(app: &AppHandle, state: &Arc<AppState>, data: &Value) {
    let view = parse_session(data);
    schedule_auto_apply(app, state, view.my_champion_id);
    state.update(app, |s| {
        if view.my_champion_id > 0 {
            s.last_champion_id = view.my_champion_id;
        }
        s.champ_select = Some(view);
    });
}

/// 我的英雄变化后 1 秒内没有再变，才加载详情并自动导入（大乱斗里换英雄很频繁）
fn schedule_auto_apply(app: &AppHandle, state: &Arc<AppState>, champ: i64) {
    if champ <= 0 || state.applied_champ.load(Ordering::SeqCst) == champ {
        return;
    }
    state.applied_champ.store(champ, Ordering::SeqCst);
    let gen = state.apply_gen.fetch_add(1, Ordering::SeqCst) + 1;
    let (app, state) = (app.clone(), state.clone());
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(Duration::from_millis(1000)).await;
        if state.apply_gen.load(Ordering::SeqCst) != gen {
            return;
        }
        if let Err(e) = crate::data::ensure_detail(&app, &state, champ).await {
            log::warn!("加载英雄详情失败：{e:#}");
        }
        let which = {
            let s = state.settings.read().unwrap();
            crate::apply::Which { items: s.auto_items, spells: s.auto_spells }
        };
        // 统计是大乱斗专用的，其他模式不自动改玩家的出装/技能
        if !(which.items || which.spells) || state.snapshot.lock().unwrap().queue_id != Some(MAYHEM_QUEUE) {
            return;
        }
        let (Some(client), Some(cat)) = (state.lcu(), state.catalog.read().unwrap().clone()) else {
            return;
        };
        let Some(st) = cat.stats.as_ref().and_then(|s| s.champions.get(&champ.to_string())) else {
            return;
        };
        let msgs = crate::apply::apply_all(&client, &cat, champ, st, which).await;
        if !msgs.is_empty() {
            let label = crate::apply::champ_label(&cat, champ);
            let _ = app.emit("toast", format!("{label}：{}", msgs.join("  ")));
        }
    });
}

pub const WINDOW_MODE_BORDERLESS: i64 = 2;

/// 读取游戏显示模式（下次进游戏生效的设置）
pub async fn read_window_mode(client: &LcuClient) -> Option<i64> {
    client.get("/lol-game-settings/v1/game-settings").await.ok()?["General"]["WindowMode"].as_i64()
}

/// 把游戏显示模式改成无边框；与在客户端设置里手动修改等价，下次进游戏生效。
pub async fn set_borderless(client: &LcuClient) -> anyhow::Result<i64> {
    use reqwest::Method;
    let body = serde_json::json!({ "General": { "WindowMode": WINDOW_MODE_BORDERLESS } });
    client.request(Method::PATCH, "/lol-game-settings/v1/game-settings", Some(&body)).await?;
    // 持久化到 game.cfg；部分客户端版本没有这个接口，失败不影响 PATCH 结果
    let _ = client.request(Method::POST, "/lol-game-settings/v1/save", None).await;
    let now = read_window_mode(client).await.unwrap_or(-1);
    anyhow::ensure!(now == WINDOW_MODE_BORDERLESS, "写入后读回的值是 {now}，请在游戏内 Esc → 视频 → 窗口模式 手动确认");
    Ok(now)
}

/// 读取客户端语言；数据语言为「跟随客户端」且与当前不同时重新同步
async fn detect_locale(app: &AppHandle, state: &Arc<AppState>, client: &LcuClient) {
    let Ok(v) = client.get("/riotclient/region-locale").await else { return };
    let Some(loc) = v["locale"].as_str().map(String::from) else { return };
    *state.client_locale.write().unwrap() = Some(loc.clone());
    state.update(app, |s| s.client_locale = Some(loc));
    let want = crate::data::effective_locale(state);
    let have = state.catalog.read().unwrap().as_ref().map(|c| c.locale.clone());
    if have.as_deref() != Some(want.as_str()) {
        let (app, state) = (app.clone(), state.clone());
        tauri::async_runtime::spawn(async move {
            if let Err(e) = crate::data::sync(app, state, false).await {
                log::warn!("数据同步失败：{e:#}");
            }
        });
    }
}

fn on_ready_check(state: &Arc<AppState>, client: &LcuClient, data: &Value) {
    let (enabled, delay) = {
        let s = state.settings.read().unwrap();
        (s.auto_accept, s.accept_delay_ms)
    };
    let pending = data["state"] == "InProgress" && data["playerResponse"] == "None";
    if !enabled || !pending || state.accept_pending.swap(true, Ordering::SeqCst) {
        return;
    }
    let client = client.clone();
    let state = state.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(Duration::from_millis(delay)).await;
        // 延迟期间玩家可能已手动处理或对局已取消，再确认一次。
        if let Ok(rc) = client.get("/lol-matchmaking/v1/ready-check").await {
            if rc["state"] == "InProgress" && rc["playerResponse"] == "None" {
                if let Err(e) = client.post("/lol-matchmaking/v1/ready-check/accept").await {
                    log::warn!("自动接受失败：{e}");
                }
            }
        }
        state.accept_pending.store(false, Ordering::SeqCst);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_parsing() {
        let v = serde_json::json!({
            "localPlayerCellId": 2,
            "benchEnabled": true,
            "rerollsRemaining": 1,
            "benchChampions": [{"championId": 103, "isPriority": false}, {"championId": 7}],
            "myTeam": [{"cellId": 1, "championId": 22}, {"cellId": 2, "championId": 99}]
        });
        let s = parse_session(&v);
        assert_eq!(s.my_champion_id, 99);
        assert_eq!(s.bench, vec![103, 7]);
        assert!(s.team[1].is_me && !s.team[0].is_me);
        assert_eq!(s.rerolls_remaining, 1);

        let old = serde_json::json!({"localPlayerCellId": 0, "benchChampionIds": [1, 2], "myTeam": []});
        assert_eq!(parse_session(&old).bench, vec![1, 2]);
    }
}
