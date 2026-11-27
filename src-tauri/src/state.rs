use crate::data::Catalog;
use crate::lcu::http::LcuClient;
use crate::settings::Settings;
use serde::Serialize;
use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU64};
use std::sync::{Arc, Mutex, RwLock};
use std::time::Instant;
use tauri::{AppHandle, Emitter};

#[derive(Serialize, Clone, PartialEq, Default, Debug)]
#[serde(rename_all = "camelCase")]
pub struct TeamMember {
    pub cell_id: i64,
    pub champion_id: i64,
    pub is_me: bool,
}

#[derive(Serialize, Clone, PartialEq, Default, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ChampSelectView {
    pub my_champion_id: i64,
    pub bench_enabled: bool,
    pub bench: Vec<i64>,
    pub team: Vec<TeamMember>,
    pub rerolls_remaining: i64,
}

/// 数据同步进度：stage = checking | static | stats
#[derive(Serialize, Clone, PartialEq, Default, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SyncProgress {
    pub stage: &'static str,
    pub done: u32,
    pub total: u32,
}

/// 推给前端的完整状态，体积很小；只有内容变化时才推送。
#[derive(Serialize, Clone, PartialEq, Default, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub connected: bool,
    pub phase: String,
    pub queue_id: Option<i64>,
    pub champ_select: Option<ChampSelectView>,
    /// 选人结束后保留，进入对局仍可展示本局英雄的强化参考。
    pub last_champion_id: i64,
    pub live_level: Option<i64>,
    pub live_dead: bool,
    /// 游戏显示模式（General.WindowMode）：0 全屏 / 1 窗口 / 2 无边框
    pub window_mode: Option<i64>,
    /// 客户端语言，如 zh_TW
    pub client_locale: Option<String>,
    /// 当前使用的 OCR 语言
    pub ocr_lang: Option<String>,
    /// 系统已安装的 OCR 语言
    pub ocr_installed: Vec<String>,
    /// OCR 识别到的当前强化卡片
    pub offer: Vec<crate::ocr::OfferCard>,
    pub ocr_error: Option<String>,
    /// 正在同步数据时有值
    pub sync: Option<SyncProgress>,
}

#[derive(Default)]
pub struct AppState {
    pub settings: RwLock<Settings>,
    pub lcu: RwLock<Option<LcuClient>>,
    pub snapshot: Mutex<Snapshot>,
    pub catalog: RwLock<Option<Arc<Catalog>>>,
    pub accept_pending: AtomicBool,
    pub live_running: AtomicBool,
    pub ocr_running: AtomicBool,
    pub ocr_until: Mutex<Option<Instant>>,
    /// 本次选人已自动导入过的英雄，避免重复导入
    pub applied_champ: AtomicI64,
    /// 防抖计数：快速连续换英雄时只导入最后一个
    pub apply_gen: AtomicU64,
    pub client_locale: RwLock<Option<String>>,
    /// 防止并发执行数据同步
    pub sync_lock: tokio::sync::Mutex<()>,
    pub http: reqwest::Client,
    pub detail_inflight: Mutex<HashSet<i64>>,
}

impl AppState {
    pub fn update(&self, app: &AppHandle, f: impl FnOnce(&mut Snapshot)) {
        let mut s = self.snapshot.lock().unwrap();
        let before = s.clone();
        f(&mut s);
        if *s != before {
            let _ = app.emit("snapshot", &*s);
        }
    }

    pub fn phase(&self) -> String {
        self.snapshot.lock().unwrap().phase.clone()
    }

    pub fn lcu(&self) -> Option<LcuClient> {
        self.lcu.read().unwrap().clone()
    }
}
