//! 游戏内只读轮询：Riot 官方 Live Client Data API（https://127.0.0.1:2999）。
//! 每 2 秒读一次玩家列表，只取自己的等级和是否阵亡，不碰游戏进程本身。
//!
//! 海克斯大乱斗在开局和 3/7/11/15 级提供强化，且只能在阵亡时选择，
//! 所以「升级」和「阵亡期间」都会打开 OCR 扫描窗口。

use crate::lcu::http::local_https_client;
use crate::state::AppState;
use serde_json::Value;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;
use tauri::AppHandle;

const BASE: &str = "https://127.0.0.1:2999/liveclientdata";

fn find_me<'a>(list: &'a Value, me: &str) -> Option<&'a Value> {
    list.as_array()?.iter().find(|p| {
        p["riotId"].as_str() == Some(me) || p["summonerName"].as_str() == Some(me)
    })
}

pub async fn poll(app: AppHandle, state: Arc<AppState>) {
    if state.live_running.swap(true, Ordering::SeqCst) {
        return;
    }
    if let Ok(client) = local_https_client(Duration::from_secs(2)) {
        let mut me: Option<String> = None;
        while state.phase() == "InProgress" {
            if me.is_none() {
                // 返回的是 JSON 字符串，如 "Name#TAG"
                me = match client.get(format!("{BASE}/activeplayername")).send().await {
                    Ok(r) if r.status().is_success() => r.json::<String>().await.ok(),
                    _ => None,
                };
            }
            let mine = match (&me, client.get(format!("{BASE}/playerlist")).send().await) {
                (Some(name), Ok(r)) if r.status().is_success() => r
                    .json::<Value>()
                    .await
                    .ok()
                    .and_then(|v| find_me(&v, name).map(|p| (p["level"].as_i64(), p["isDead"].as_bool().unwrap_or(false)))),
                _ => None,
            };
            let (level, dead) = mine.unwrap_or((None, false));
            let prev = state.snapshot.lock().unwrap().live_level;
            if dead || (level.is_some() && level != prev) {
                crate::ocr::arm(&state);
            }
            state.update(&app, |s| {
                s.live_level = level;
                s.live_dead = dead;
            });
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
    }
    state.update(&app, |s| {
        s.live_level = None;
        s.live_dead = false;
    });
    state.live_running.store(false, Ordering::SeqCst);
}

#[cfg(test)]
mod tests {
    #[test]
    fn find_me() {
        let v = serde_json::json!([
            {"riotId": "A#1", "level": 3, "isDead": false},
            {"riotId": "Me#TAG", "summonerName": "Me", "level": 7, "isDead": true}
        ]);
        assert_eq!(super::find_me(&v, "Me#TAG").unwrap()["level"], 7);
        assert!(super::find_me(&v, "X#0").is_none());
    }
}
