//! 海克斯强化识别：截取游戏窗口画面 → Windows 系统自带 OCR → 与强化名称模糊匹配。
//!
//! 合规与性能：
//! - 只截屏（与录屏软件相同的 GDI 方式），不读游戏内存、不注入、不模拟输入；
//! - 只有游戏窗口在前台时才截图；
//! - 平时不扫描，只在「升级后 30 秒内」或「按下热键后 30 秒内」以约 1 次/秒的频率扫描，
//!   识别到卡片后持续跟踪直到卡片消失。

#[cfg(windows)]
mod win;

use crate::data::Catalog;
use crate::state::AppState;
use std::collections::HashSet;
use serde::Serialize;
use std::sync::atomic::Ordering;
use std::sync::Arc;
#[cfg(windows)]
use std::sync::OnceLock;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Manager, PhysicalPosition, PhysicalSize};

/// 一行 OCR 结果，坐标为相对游戏画面的比例（0–1）
#[derive(Debug, Clone)]
pub struct Line {
    pub text: String,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

pub struct Scan {
    /// 游戏画面在屏幕上的物理像素区域：x, y, w, h
    pub rect: (i32, i32, i32, i32),
    pub lines: Vec<Line>,
}

#[derive(Serialize, Clone, PartialEq, Debug)]
#[serde(rename_all = "camelCase")]
pub struct OfferCard {
    pub augment_id: i64,
    /// 卡片标题中心位置（相对游戏画面比例）
    pub x: f32,
    pub y: f32,
    pub confidence: f32,
}

pub const SCAN_WINDOW: Duration = Duration::from_secs(30);

/// 打开一个扫描窗口（升级、热键、手动按钮都会调用）
pub fn arm(state: &AppState) {
    *state.ocr_until.lock().unwrap() = Some(Instant::now() + SCAN_WINDOW);
}

fn normalize(s: &str) -> Vec<char> {
    s.chars().filter(|c| c.is_alphanumeric()).flat_map(char::to_lowercase).collect()
}

fn levenshtein(a: &[char], b: &[char]) -> usize {
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    let mut cur = vec![0; b.len() + 1];
    for (i, ca) in a.iter().enumerate() {
        cur[0] = i + 1;
        for (j, cb) in b.iter().enumerate() {
            cur[j + 1] = (prev[j + 1] + 1).min(cur[j] + 1).min(prev[j] + usize::from(ca != cb));
        }
        std::mem::swap(&mut prev, &mut cur);
    }
    prev[b.len()]
}

fn contains(hay: &[char], needle: &[char]) -> bool {
    needle.len() <= hay.len() && hay.windows(needle.len()).any(|w| w == needle)
}

/// 一行文字与某个强化名的相似度（0–1）
fn similarity(line: &[char], name: &[char]) -> f32 {
    if name.is_empty() || line.is_empty() {
        return 0.0;
    }
    if line == name {
        return 1.0;
    }
    // 标题行可能夹带其他字符（如稀有度标签），名字较长时允许包含匹配
    if name.len() >= 3 && contains(line, name) {
        return 0.95;
    }
    if name.len() <= 2 || line.len().abs_diff(name.len()) > 2 {
        return 0.0;
    }
    1.0 - levenshtein(line, name) as f32 / line.len().max(name.len()) as f32
}

fn threshold(name_len: usize) -> f32 {
    match name_len {
        0..=2 => 1.0,
        3 => 0.66,
        _ => 0.74,
    }
}

/// 从 OCR 行中找出当前提供的强化卡片。
/// 要求至少 2 张卡片的标题处于同一水平线，以排除画面中零星的误匹配。
pub fn match_offer(lines: &[Line], augs: &[(i64, Vec<char>)]) -> Vec<OfferCard> {
    let mut hits: Vec<OfferCard> = Vec::new();
    for l in lines {
        let t = normalize(&l.text);
        let best = augs
            .iter()
            .map(|(id, n)| (*id, similarity(&t, n), n.len()))
            .filter(|(_, s, len)| *s >= threshold(*len))
            .max_by(|a, b| a.1.total_cmp(&b.1));
        if let Some((id, score, _)) = best {
            hits.push(OfferCard { augment_id: id, x: l.x + l.w / 2.0, y: l.y + l.h / 2.0, confidence: score });
        }
    }

    // 找到同一水平带内（去重后）数量最多的一组；同名强化只保留置信度最高的那行
    let mut best: Vec<OfferCard> = Vec::new();
    for anchor in &hits {
        let mut group: Vec<OfferCard> = Vec::new();
        for h in hits.iter().filter(|h| (h.y - anchor.y).abs() < 0.04) {
            match group.iter_mut().find(|g| g.augment_id == h.augment_id) {
                Some(g) if g.confidence < h.confidence => *g = h.clone(),
                Some(_) => {}
                None => group.push(h.clone()),
            }
        }
        if group.len() > best.len() {
            best = group;
        }
    }
    if best.len() < 2 {
        return Vec::new();
    }
    best.sort_by(|a, b| b.confidence.total_cmp(&a.confidence));
    best.truncate(4);
    best.sort_by(|a, b| a.x.total_cmp(&b.x));
    best
}

#[cfg(windows)]
fn worker() -> &'static win::Worker {
    static W: OnceLock<win::Worker> = OnceLock::new();
    W.get_or_init(win::Worker::spawn)
}

async fn scan_once(lang: &str) -> anyhow::Result<Option<Scan>> {
    #[cfg(windows)]
    {
        worker().scan(lang).await
    }
    #[cfg(not(windows))]
    {
        let _ = lang;
        Ok(None)
    }
}

/// 候选名称表：每个强化的所有语言译名都参与匹配（简中国服 / 简中国际服 / 繁中 / 英文），
/// 强化池以统计源为准（统计源没有强化池时退回 ARAM_/Kiwi 规则）。
pub fn candidate_names(cat: &Catalog) -> Vec<(i64, Vec<char>)> {
    let pool = cat.stats.as_ref().map(|s| &s.augments).filter(|m| !m.is_empty());
    cat.augments
        .iter()
        .filter(|a| match pool {
            Some(p) => p.contains_key(&a.id.to_string()),
            None => a.mayhem,
        })
        .flat_map(|a| {
            std::iter::once(&a.name)
                .chain(a.alt_names.iter())
                .map(move |n| (a.id, normalize(n)))
        })
        .filter(|(_, n)| !n.is_empty())
        .collect()
}

/// 对局中常驻；非扫描时段只做廉价的状态检查。
pub async fn run(app: AppHandle, state: Arc<AppState>) {
    if state.ocr_running.swap(true, Ordering::SeqCst) {
        return;
    }
    let mut names: Vec<(i64, Vec<char>)> = Vec::new();
    let mut names_from: usize = 0;
    let mut misses = 0;
    // 多个 OCR 语言轮流尝试，命中后本局锁定该语言
    let mut locked: Option<String> = None;
    let mut unavailable: HashSet<String> = HashSet::new();
    let mut turn = 0usize;

    while state.phase() == "InProgress" {
        let (enabled, overlay, lang_setting) = {
            let s = state.settings.read().unwrap();
            (s.ocr_enabled, s.overlay_enabled, s.ocr_lang.clone())
        };
        let armed = state.ocr_until.lock().unwrap().is_some_and(|t| Instant::now() < t);
        let showing = !state.snapshot.lock().unwrap().offer.is_empty();
        if !enabled || !(armed || showing) {
            tokio::time::sleep(Duration::from_millis(500)).await;
            continue;
        }

        if let Some(cat) = state.catalog.read().unwrap().clone() {
            let key = Arc::as_ptr(&cat) as usize;
            if key != names_from {
                names = candidate_names(&cat);
                names_from = key;
            }
        }

        let client_locale = state.client_locale.read().unwrap().clone();
        let langs: Vec<String> = crate::settings::ocr_langs(&lang_setting, client_locale.as_deref())
            .into_iter()
            .filter(|l| !unavailable.contains(l))
            .collect();
        if langs.is_empty() {
            tokio::time::sleep(Duration::from_secs(2)).await;
            continue;
        }
        let lang = match &locked {
            Some(l) if langs.contains(l) => l.clone(),
            _ => {
                turn = turn.wrapping_add(1);
                langs[turn % langs.len()].clone()
            }
        };

        let started = Instant::now();
        match scan_once(&lang).await {
            Ok(Some(scan)) => {
                let offer = match_offer(&scan.lines, &names);
                if offer.is_empty() {
                    misses += 1;
                } else {
                    misses = 0;
                    locked = Some(lang.clone());
                    // 识别到卡片时延长扫描窗口，方便跟踪重随后的新卡片
                    arm(&state);
                }
                if !offer.is_empty() || misses >= 2 {
                    let visible = !offer.is_empty();
                    state.update(&app, |s| {
                        s.offer = offer;
                        s.ocr_lang = Some(lang.clone());
                        s.ocr_error = None;
                    });
                    position_overlay(&app, overlay && visible, scan.rect);
                }
            }
            Ok(None) => {}
            Err(e) if e.downcast_ref::<EngineUnavailable>().is_some() => {
                log::warn!("OCR 语言 {lang} 不可用：{e:#}");
                unavailable.insert(lang.clone());
                if langs.len() == 1 {
                    state.update(&app, |s| s.ocr_error = Some(format!("{e:#}")));
                }
            }
            Err(e) => {
                log::warn!("OCR 失败：{e:#}");
                state.update(&app, |s| s.ocr_error = Some(format!("{e:#}")));
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        }
        let spent = started.elapsed();
        tokio::time::sleep(Duration::from_millis(1000).saturating_sub(spent)).await;
    }

    state.update(&app, |s| s.offer.clear());
    position_overlay(&app, false, (0, 0, 0, 0));
    state.ocr_running.store(false, Ordering::SeqCst);
}

/// 系统没有安装对应语言的 OCR 组件
#[derive(Debug)]
pub struct EngineUnavailable(pub String);

impl std::fmt::Display for EngineUnavailable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.0, crate::i18n::t("ocr_missing"))
    }
}

impl std::error::Error for EngineUnavailable {}

fn position_overlay(app: &AppHandle, show: bool, (x, y, w, h): (i32, i32, i32, i32)) {
    let Some(win) = app.get_webview_window("overlay") else { return };
    if show && w > 0 && h > 0 {
        let _ = win.set_position(PhysicalPosition::new(x, y));
        let _ = win.set_size(PhysicalSize::new(w as u32, h as u32));
        let _ = win.show();
        let _ = win.set_ignore_cursor_events(true);
    } else {
        let _ = win.hide();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(t: &str, x: f32, y: f32) -> Line {
        Line { text: t.into(), x, y, w: 0.1, h: 0.03 }
    }

    fn names() -> Vec<(i64, Vec<char>)> {
        [(1, "狙神飞星"), (2, "物理转魔法"), (3, "蜜蜂！！！"), (4, "大力"), (5, "冰寒")]
            .into_iter()
            .map(|(i, n)| (i, normalize(n)))
            .collect()
    }

    #[test]
    fn matches_three_cards_with_ocr_noise() {
        let lines = vec![
            line("狙 神 飞 星", 0.2, 0.30),
            line("物理转魔沄", 0.45, 0.31), // OCR 错一个字
            line("蜜蜂!!!", 0.7, 0.30),
            line("物理转魔法", 0.5, 0.9), // 不在同一行的干扰
        ];
        let o = match_offer(&lines, &names());
        assert_eq!(o.iter().map(|c| c.augment_id).collect::<Vec<_>>(), vec![1, 2, 3]);
    }

    #[test]
    fn single_or_short_noise_is_ignored() {
        assert!(match_offer(&[line("狙神飞星", 0.2, 0.3)], &names()).is_empty());
        // 两字名称必须完全一致
        let lines = vec![line("大刀", 0.2, 0.3), line("冰寒", 0.5, 0.3), line("狙神飞星", 0.7, 0.3)];
        let ids: Vec<i64> = match_offer(&lines, &names()).iter().map(|c| c.augment_id).collect();
        assert_eq!(ids, vec![5, 1]);
    }

    #[test]
    fn traditional_and_simplified_names_both_match() {
        let cat = Catalog {
            augments: vec![crate::data::Augment {
                id: 2103, name: "超遠射擊".into(), alt_names: vec!["狙神飞星".into(), "超远射击".into(), "From Downtown".into()],
                rarity: "gold".into(), icon: String::new(), tags: vec![], mayhem: true,
            }, crate::data::Augment {
                id: 1025, name: "自爆炸彈客".into(), alt_names: vec!["俯冲轰炸".into(), "Dive Bomber".into()],
                rarity: "silver".into(), icon: String::new(), tags: vec![], mayhem: true,
            }],
            ..Default::default()
        };
        let n = candidate_names(&cat);
        for (a, b) in [("超遠射擊", "自爆炸彈客"), ("超远射击", "俯冲轰炸"), ("狙神飞星", "俯冲轰炸"), ("From Downtown", "Dive Bomber")] {
            let o = match_offer(&[line(a, 0.2, 0.3), line(b, 0.5, 0.3)], &n);
            assert_eq!(o.iter().map(|c| c.augment_id).collect::<Vec<_>>(), vec![2103, 1025], "{a} {b}");
        }
    }

    #[test]
    fn lev() {
        assert_eq!(levenshtein(&normalize("kitten"), &normalize("sitting")), 3);
    }
}
