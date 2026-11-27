//! 静态数据（英雄、强化、装备、召唤师技能）来自 CommunityDragon，按版本号缓存；
//! 统计数据来自 arammeta.com（默认）或自定义数据包。
//!
//! 政策：Riot 禁止展示强化胜率。强化统计只保留 tier / pickRate，
//! 源数据里的胜率只用于在本地换算档位，不会进入 StatsPack，也不会显示。

use crate::state::AppState;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter};

/// 除当前语言外，额外拉取这些语言的强化名供 OCR 匹配（简中国服译名 / 简中国际服译名 / 繁中 / 英文）
pub const OCR_NAME_LOCALES: [&str; 4] = ["zh_cn", "zh_my", "zh_tw", "default"];

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Champion {
    pub id: i64,
    /// 常用名（安妮 / Annie）
    pub name: String,
    /// 称号（黑暗之女 / the Dark Child）
    pub title: String,
    pub alias: String,
    pub icon: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Augment {
    pub id: i64,
    pub name: String,
    /// 其他语言/译名，仅用于 OCR 匹配
    #[serde(default)]
    pub alt_names: Vec<String>,
    pub rarity: String,
    pub icon: String,
    pub tags: Vec<String>,
    /// 是否判定为大乱斗强化（ARAM_ 前缀或 Kiwi 资源路径）；统计源提供强化池时以统计源为准
    pub mayhem: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct ChampAug {
    pub id: i64,
    pub tier: String,
    pub pick_rate: Option<f64>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct ItemBuild {
    pub start: Vec<i64>,
    pub boots: Vec<i64>,
    pub core: Vec<i64>,
    pub situational: Vec<i64>,
}

/// 出装流派（如「AP 爆发」）：核心装备 + 常见后续，胜率可以展示（这是装备胜率，不是强化胜率）
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Archetype {
    /// 繁体中文原名
    pub name: String,
    pub name_hans: String,
    pub name_en: String,
    pub tier: String,
    pub core: Vec<i64>,
    pub options: Vec<i64>,
    pub win_rate: Option<f64>,
    pub pick_rate: Option<f64>,
    pub games: Option<i64>,
}

impl Archetype {
    pub fn display_name(&self) -> &str {
        localized(&self.name, &self.name_hans, &self.name_en)
    }
}

fn localized<'a>(hant: &'a str, hans: &'a str, en: &'a str) -> &'a str {
    let pick = match crate::i18n::get() {
        crate::i18n::Lang::Hans => hans,
        crate::i18n::Lang::Hant => hant,
        crate::i18n::Lang::En => en,
    };
    if pick.is_empty() { hant } else { pick }
}

/// 强化风格（如「近战普攻型·魔攻」）：只展示选取率
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct AugStyle {
    pub name: String,
    pub name_hans: String,
    pub name_en: String,
    pub pick_rate: Option<f64>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct ChampStats {
    pub tier: String,
    /// 0–100 的综合分，用于排序板凳英雄
    pub score: Option<f64>,
    pub win_rate: Option<f64>,
    pub pick_rate: Option<f64>,
    pub games: Option<i64>,
    pub augments: Vec<ChampAug>,
    /// 召唤师技能 id，两个
    pub spells: Vec<i64>,
    pub items: Option<ItemBuild>,
    pub archetypes: Vec<Archetype>,
    pub aug_styles: Vec<AugStyle>,
    pub note: String,
    /// 是否已加载英雄详情（出装、完整强化表）
    pub detailed: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct AugStats {
    pub tier: String,
    pub pick_rate: Option<f64>,
}

/// 数据来源署名：凡是展示统计数字的地方都要显示
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Attribution {
    pub label: String,
    pub url: String,
    /// 英雄页链接模板，`{alias}` 会被替换为小写英文名
    pub champion_url: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct StatsPack {
    pub schema: u32,
    pub patch: String,
    pub source: String,
    pub updated_at: String,
    pub attribution: Option<Attribution>,
    pub champions: HashMap<String, ChampStats>,
    /// 同时也是强化池：非空时，只有这里出现的强化参与展示和 OCR 匹配
    pub augments: HashMap<String, AugStats>,
    /// 英雄详情地址模板（`{id}`），留空表示数据包已包含全部内容
    pub detail_url: String,
    pub detail_version: String,
}

/// 装备 / 召唤师技能的展示信息
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Thing {
    pub id: i64,
    pub name: String,
    pub icon: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct Catalog {
    pub patch: String,
    pub locale: String,
    pub champions: Vec<Champion>,
    pub augments: Vec<Augment>,
    #[serde(default)]
    pub items: Vec<Thing>,
    #[serde(default)]
    pub spells: Vec<Thing>,
    #[serde(default)]
    pub stats: Option<StatsPack>,
    #[serde(default)]
    pub stats_error: Option<String>,
}

/// `/lol-game-data/assets/ASSETS/UX/...` → CommunityDragon 上的小写路径
pub fn asset_url(base: &str, path: &str) -> String {
    let p = path.trim_start_matches("/lol-game-data/assets/").to_lowercase();
    format!("{base}/latest/plugins/rcp-be-lol-game-data/global/default/{p}")
}

fn rarity(s: &str) -> &'static str {
    match s {
        "kSilver" => "silver",
        "kGold" => "gold",
        "kPrismatic" => "prismatic",
        _ => "other",
    }
}

pub fn build_champions(base: &str, locale: &str, v: &Value) -> Vec<Champion> {
    // 只有 zh_cn 的 name 是称号、description 是常用名，其他语言相反
    let swapped = locale == "zh_cn";
    let mut out: Vec<Champion> = v
        .as_array()
        .into_iter()
        .flatten()
        .filter(|c| c["id"].as_i64().unwrap_or(-1) > 0)
        .map(|c| {
            let (n, d) = (
                c["name"].as_str().unwrap_or_default().to_string(),
                c["description"].as_str().unwrap_or_default().to_string(),
            );
            let (name, title) = if swapped { (d, n) } else { (n, d) };
            Champion {
                id: c["id"].as_i64().unwrap_or(0),
                name,
                title,
                alias: c["alias"].as_str().unwrap_or_default().to_string(),
                icon: asset_url(base, c["squarePortraitPath"].as_str().unwrap_or_default()),
            }
        })
        .collect();
    out.sort_by_key(|c| c.id);
    out
}

pub fn build_augments(base: &str, v: &Value) -> Vec<Augment> {
    v.as_array()
        .into_iter()
        .flatten()
        .map(|a| {
            let name_id = a["augmentNameId"].as_str().unwrap_or_default();
            let icon_path = a["augmentLargeIconPath"]
                .as_str()
                .or_else(|| a["augmentSmallIconPath"].as_str())
                .unwrap_or_default();
            Augment {
                id: a["id"].as_i64().unwrap_or(0),
                name: a["nameTRA"].as_str().unwrap_or_default().to_string(),
                alt_names: Vec::new(),
                rarity: rarity(a["rarity"].as_str().unwrap_or_default()).into(),
                icon: asset_url(base, icon_path),
                tags: a["augmentDisplayTags"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|t| t.as_str().map(|s| s.trim_start_matches('k').to_string()))
                    .collect(),
                mayhem: name_id.starts_with("ARAM_") || icon_path.contains("/Kiwi/"),
            }
        })
        .filter(|a| a.id > 0 && !a.name.is_empty())
        .collect()
}

/// 把其他语言的强化名并入 alt_names（去重）
pub fn merge_alt_names(augs: &mut [Augment], other: &Value) {
    let names: HashMap<i64, &str> = other
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|a| Some((a["id"].as_i64()?, a["nameTRA"].as_str()?)))
        .collect();
    for a in augs.iter_mut() {
        if let Some(n) = names.get(&a.id) {
            if !n.is_empty() && *n != a.name && !a.alt_names.iter().any(|x| x == n) {
                a.alt_names.push(n.to_string());
            }
        }
    }
}

/// items.json / summoner-spells.json 通用：id + name + iconPath
pub fn build_things(base: &str, v: &Value) -> Vec<Thing> {
    v.as_array()
        .into_iter()
        .flatten()
        .filter_map(|x| {
            Some(Thing {
                id: x["id"].as_i64().filter(|id| *id > 0)?,
                name: x["name"].as_str()?.to_string(),
                icon: asset_url(base, x["iconPath"].as_str().unwrap_or_default()),
            })
        })
        .collect()
}

pub async fn fetch_json(http: &reqwest::Client, url: &str) -> Result<Value> {
    let r = http
        .get(url)
        .header("User-Agent", "hexpilot/0.1")
        .timeout(Duration::from_secs(20))
        .send()
        .await
        .with_context(|| format!("{} {url}", crate::i18n::t("request_failed")))?;
    anyhow::ensure!(r.status().is_success(), "{url} → HTTP {}", r.status());
    Ok(r.json().await?)
}

async fn load_custom(http: &reqwest::Client, src: &str) -> Result<StatsPack> {
    let raw: Value = if src.starts_with("http://") || src.starts_with("https://") {
        fetch_json(http, src).await?
    } else {
        serde_json::from_str(&tokio::fs::read_to_string(src).await.context(crate::i18n::t("custom_pack_read_failed"))?)?
    };
    let pack: StatsPack = serde_json::from_value(raw).context(crate::i18n::t("custom_pack_bad"))?;
    anyhow::ensure!(pack.schema == 1, "schema {} ≠ 1", pack.schema);
    Ok(pack)
}

fn publish(app: &AppHandle, state: &AppState, cat: Catalog) {
    *state.catalog.write().unwrap() = Some(Arc::new(cat));
    let _ = app.emit("catalog-updated", ());
}

/// 当前生效的数据语言：跟随界面语言
pub fn effective_locale(_state: &AppState) -> String {
    crate::i18n::data_locale(crate::i18n::get()).to_string()
}

/// 启动时调用：先用缓存秒开，再后台检查版本更新。
pub async fn sync(app: AppHandle, state: Arc<AppState>, force: bool) -> Result<()> {
    let _guard = state.sync_lock.lock().await;
    let (base, source, custom_url) = {
        let s = state.settings.read().unwrap();
        (s.cdragon_base.trim_end_matches('/').to_string(), s.stats_source.clone(), s.stats_url.clone())
    };
    let cache_dir = crate::paths::cache_dir(&app);
    let cache_file = cache_dir.join("catalog.json");

    let cached: Option<Catalog> = std::fs::read(&cache_file)
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok());
    if let Some(c) = &cached {
        if state.catalog.read().unwrap().is_none() {
            publish(&app, &state, c.clone());
        }
    }

    let locale = effective_locale(&state);
    let http = state.http.clone();
    let version = fetch_json(&http, &format!("{base}/latest/content-metadata.json"))
        .await
        .ok()
        .and_then(|v| v["version"].as_str().map(String::from))
        .map(|v| v.split('.').take(2).collect::<Vec<_>>().join("."))
        .unwrap_or_default();

    let mut cat = match cached {
        Some(c) if !force && c.locale == locale && !c.items.is_empty()
            && (version.is_empty() || c.patch == version) => c,
        _ => {
            let root = |l: &str| format!("{base}/latest/plugins/rcp-be-lol-game-data/global/{l}/v1");
            let r = root(&locale);
            let urls = ["champion-summary", "cherry-augments", "items", "summoner-spells"]
                .map(|n| format!("{r}/{n}.json"));
            let (champs, augs, items, spells) = tokio::try_join!(
                fetch_json(&http, &urls[0]),
                fetch_json(&http, &urls[1]),
                fetch_json(&http, &urls[2]),
                fetch_json(&http, &urls[3]),
            )?;
            let mut augments = build_augments(&base, &augs);
            for l in OCR_NAME_LOCALES.iter().filter(|l| **l != locale) {
                // 备用语言拉取失败不影响主流程
                if let Ok(v) = fetch_json(&http, &format!("{}/cherry-augments.json", root(l))).await {
                    merge_alt_names(&mut augments, &v);
                }
            }
            Catalog {
                patch: if version.is_empty() { "?".into() } else { version.clone() },
                locale: locale.clone(),
                champions: build_champions(&base, &locale, &champs),
                augments,
                items: build_things(&base, &items),
                spells: build_things(&base, &spells),
                stats: None,
                stats_error: None,
            }
        }
    };

    let stats = match source.as_str() {
        "custom" if !custom_url.trim().is_empty() => load_custom(&http, custom_url.trim()).await.map(Some),
        "custom" => Ok(None),
        _ => crate::arammeta::index(&http, &cache_dir, &cat.patch, force).await.map(Some),
    };
    match stats {
        Ok(s) => {
            cat.stats = s;
            cat.stats_error = None;
        }
        Err(e) => {
            // 拉取失败时保留旧统计（如果有）
            cat.stats_error = Some(format!("{e:#}"));
        }
    }

    let _ = std::fs::create_dir_all(&cache_dir);
    let _ = std::fs::write(&cache_file, serde_json::to_vec(&cat)?);
    publish(&app, &state, cat);
    Ok(())
}

/// 按需加载英雄详情（出装、技能、完整强化表），合并进当前目录。
pub async fn ensure_detail(app: &AppHandle, state: &Arc<AppState>, champ: i64) -> Result<()> {
    let Some(cat) = state.catalog.read().unwrap().clone() else { return Ok(()) };
    let Some(pack) = &cat.stats else { return Ok(()) };
    let key = champ.to_string();
    if pack.detail_url.is_empty() || pack.champions.get(&key).is_some_and(|c| c.detailed) {
        return Ok(());
    }
    if !state.detail_inflight.lock().unwrap().insert(champ) {
        return Ok(());
    }
    let base = pack.champions.get(&key).cloned().unwrap_or_default();
    let res = crate::arammeta::detail(
        &state.http,
        &crate::paths::cache_dir(app),
        &pack.detail_url,
        &pack.detail_version,
        champ,
        &base,
    )
    .await;
    state.detail_inflight.lock().unwrap().remove(&champ);
    let st = res?;

    let Some(latest) = state.catalog.read().unwrap().clone() else { return Ok(()) };
    let mut next = (*latest).clone();
    if let Some(p) = next.stats.as_mut() {
        p.champions.insert(key, st);
    }
    publish(app, state, next);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn asset_paths() {
        assert_eq!(
            asset_url("https://x", "/lol-game-data/assets/ASSETS/UX/Kiwi/Augments/Icons/QuestBangBang_large.png"),
            "https://x/latest/plugins/rcp-be-lol-game-data/global/default/assets/ux/kiwi/augments/icons/questbangbang_large.png"
        );
    }

    #[test]
    fn augment_win_rate_is_dropped() {
        let v = serde_json::json!({"tier": "S", "pickRate": 0.2, "winRate": 0.6});
        let a: AugStats = serde_json::from_value(v).unwrap();
        let back = serde_json::to_value(&a).unwrap();
        assert!(back.get("winRate").is_none());
    }

    #[test]
    fn sample_pack_parses() {
        let raw = include_str!("../../samples/mayhem-stats.sample.json");
        let p: StatsPack = serde_json::from_str(raw).unwrap();
        assert_eq!(p.schema, 1);
        let ahri = &p.champions["103"];
        assert_eq!(ahri.spells, vec![4, 32]);
        assert_eq!(ahri.items.as_ref().unwrap().core.len(), 3);
    }

    #[test]
    fn champion_name_order_by_locale() {
        let v = serde_json::json!([{"id": 1, "name": "黑暗之女", "description": "安妮", "alias": "Annie", "squarePortraitPath": ""}]);
        assert_eq!(build_champions("b", "zh_cn", &v)[0].name, "安妮");
        let v = serde_json::json!([{"id": 1, "name": "安妮", "description": "黑暗之女", "alias": "Annie", "squarePortraitPath": ""}]);
        assert_eq!(build_champions("b", "zh_tw", &v)[0].name, "安妮");
    }

    #[test]
    fn alt_names_merge() {
        let mut a = build_augments("b", &serde_json::json!([
            {"id": 2103, "augmentNameId": "ARAM_BangBang", "nameTRA": "超遠射擊", "rarity": "kGold", "augmentLargeIconPath": "/x/Kiwi/a.png"}
        ]));
        merge_alt_names(&mut a, &serde_json::json!([{"id": 2103, "nameTRA": "狙神飞星"}]));
        merge_alt_names(&mut a, &serde_json::json!([{"id": 2103, "nameTRA": "超远射击"}]));
        merge_alt_names(&mut a, &serde_json::json!([{"id": 2103, "nameTRA": "超遠射擊"}]));
        assert_eq!(a[0].alt_names, vec!["狙神飞星", "超远射击"]);
    }

    #[test]
    fn augment_filter() {
        let v = serde_json::json!([
            {"id": 1, "augmentNameId": "ARAM_X", "nameTRA": "甲", "rarity": "kGold", "augmentLargeIconPath": "/lol-game-data/assets/ASSETS/UX/Cherry/a.png"},
            {"id": 2, "augmentNameId": "Foo", "nameTRA": "乙", "rarity": "kSilver", "augmentLargeIconPath": "/lol-game-data/assets/ASSETS/UX/Kiwi/b.png"},
            {"id": 3, "augmentNameId": "Bar", "nameTRA": "丙", "rarity": "kPrismatic", "augmentLargeIconPath": "/lol-game-data/assets/ASSETS/UX/Cherry/c.png"}
        ]);
        let a = build_augments("b", &v);
        assert_eq!(a.iter().map(|x| x.mayhem).collect::<Vec<_>>(), vec![true, true, false]);
        assert_eq!(a[2].rarity, "prismatic");
    }
}
