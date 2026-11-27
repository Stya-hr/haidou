//! arammeta.com 适配器。
//!
//! 遵守其数据使用条款：
//! - 署名：所有显示其数字的地方都显示可点击的 "Data: AramMeta"，链接带 `?ref=hexpilot`；
//! - 轻量访问：总表每个版本拉一次（同版本内最多 3 天刷新一次）；英雄详情只在需要时拉，
//!   并按数据版本永久缓存，同一版本不会重复请求；
//! - 不导出、不转发原始表：本地只缓存换算后的档位/选取率/出装，不保存原始 JSON。
//!
//! 强化胜率只用于在本地换算档位，不写入结果（Riot 政策禁止展示强化胜率）。

use crate::data::{fetch_json, Archetype, AugStats, AugStyle, Attribution, ChampAug, ChampStats, ItemBuild, StatsPack};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

pub const SITE: &str = "https://arammeta.com";
pub const REF: &str = "ref=hexpilot";
const REFRESH_SECS: u64 = 3 * 24 * 3600;

const RARITIES: [&str; 3] = ["kPrismatic", "kGold", "kSilver"];

#[derive(Serialize, Deserialize)]
struct IndexCache {
    fetched_at: u64,
    cdragon_patch: String,
    pack: StatsPack,
}

fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

/// 英雄胜率在全体英雄中的百分位 → 档位
fn champ_tier(p: f64) -> &'static str {
    match p {
        p if p < 0.05 => "S+",
        p if p < 0.20 => "S",
        p if p < 0.45 => "A",
        p if p < 0.70 => "B",
        p if p < 0.90 => "C",
        _ => "D",
    }
}

/// 该英雄选此强化相对该英雄平均表现的提升分（arammeta 的 score，已做小样本收缩）→ 档位
fn champ_aug_tier(score: f64) -> &'static str {
    match score {
        s if s >= 0.035 => "S+",
        s if s >= 0.02 => "S",
        s if s >= 0.008 => "A",
        s if s >= -0.008 => "B",
        s if s >= -0.03 => "C",
        _ => "D",
    }
}

/// 出装流派相对该英雄平均胜率的提升 → 档位
fn archetype_tier(lift: f64) -> &'static str {
    match lift {
        l if l >= 0.02 => "S+",
        l if l >= 0.01 => "S",
        l if l >= 0.003 => "A",
        l if l >= -0.003 => "B",
        l if l >= -0.01 => "C",
        _ => "D",
    }
}

/// 强化的全局提升（lift）→ 档位
fn global_aug_tier(lift: f64) -> &'static str {
    match lift {
        l if l >= 0.03 => "S+",
        l if l >= 0.015 => "S",
        l if l >= 0.005 => "A",
        l if l >= -0.005 => "B",
        l if l >= -0.02 => "C",
        _ => "D",
    }
}

/// 在同一英雄的全部强化里按分数排名的百分位 → 档位
fn rank_tier(p: f64) -> &'static str {
    match p {
        p if p < 0.05 => "S+",
        p if p < 0.15 => "S",
        p if p < 0.35 => "A",
        p if p < 0.65 => "B",
        p if p < 0.85 => "C",
        _ => "D",
    }
}

/// 详情页给的是该英雄的完整强化表：按分数排名分档
fn ranked_aug_list(lists: &[&Value]) -> Vec<ChampAug> {
    let mut seen = HashSet::new();
    let mut all: Vec<(i64, f64, Option<f64>)> = lists
        .iter()
        .flat_map(|v| RARITIES.iter().flat_map(move |r| v[*r].as_array().into_iter().flatten()))
        .filter_map(|a| Some((a["id"].as_i64()?, a["score"].as_f64().unwrap_or(0.0), a["pick"].as_f64())))
        .filter(|(id, _, _)| seen.insert(*id))
        .collect();
    all.sort_by(|a, b| b.1.total_cmp(&a.1));
    let n = all.len().max(1) as f64;
    all.into_iter()
        .enumerate()
        .map(|(i, (id, _, pick))| ChampAug { id, tier: rank_tier(i as f64 / n).into(), pick_rate: pick })
        .collect()
}

/// 总表里每个英雄只有各稀有度最好的 3 个：用绝对阈值分档
fn aug_list(v: &Value) -> Vec<ChampAug> {
    let mut seen = HashSet::new();
    RARITIES
        .iter()
        .flat_map(|r| v[*r].as_array().into_iter().flatten())
        .filter_map(|a| {
            let id = a["id"].as_i64()?;
            seen.insert(id).then(|| ChampAug {
                id,
                tier: champ_aug_tier(a["score"].as_f64().unwrap_or(0.0)).into(),
                pick_rate: a["pick"].as_f64(),
            })
        })
        .collect()
}

pub fn convert_index(v: &Value) -> Result<StatsPack> {
    let champs = v["champs"].as_object().context("arammeta 数据格式已变化：缺少 champs")?;
    let mut ranked: Vec<(&String, &Value, f64)> = champs
        .iter()
        .filter_map(|(id, c)| Some((id, c, c["wr"].as_f64()?)))
        .collect();
    ranked.sort_by(|a, b| b.2.total_cmp(&a.2));
    let n = ranked.len().max(1) as f64;
    // 选取率 = 出场局数 / 总局数（每局 10 人，所以英雄选取率之和约为 1000%）
    let total_games = v["patchChanges"]["currentGames"].as_f64().filter(|g| *g > 0.0);

    let champions = ranked
        .iter()
        .enumerate()
        .map(|(i, (id, c, wr))| {
            let p = i as f64 / n;
            let st = ChampStats {
                tier: champ_tier(p).into(),
                score: Some(((1.0 - p) * 100.0).round()),
                win_rate: Some(*wr),
                pick_rate: total_games.zip(c["g"].as_f64()).map(|(t, g)| g / t),
                games: c["g"].as_i64(),
                augments: aug_list(&c["top"]),
                ..Default::default()
            };
            ((*id).clone(), st)
        })
        .collect();

    let augments = v["augs"]
        .as_object()
        .into_iter()
        .flatten()
        .map(|(id, a)| {
            let tier = global_aug_tier(a["lift"].as_f64().unwrap_or(0.0));
            // 选取率 = 拿到该强化的玩家占全部玩家的比例
            let pick_rate = total_games.zip(a["g"].as_f64()).map(|(t, g)| g / (t * 10.0));
            (id.clone(), AugStats { tier: tier.into(), pick_rate })
        })
        .collect::<HashMap<_, _>>();

    let version = v["detailVersion"].as_str().unwrap_or_default().to_string();
    let base = v["detailBase"].as_str().unwrap_or("api/champions").trim_matches('/');
    let date = version.split('-').next().unwrap_or_default();
    let updated_at = if date.len() == 8 {
        format!("{}-{}-{}", &date[..4], &date[4..6], &date[6..])
    } else {
        String::new()
    };

    Ok(StatsPack {
        schema: 1,
        patch: v["patch_prefix"].as_str().unwrap_or_default().to_string(),
        source: "AramMeta".into(),
        updated_at,
        attribution: Some(Attribution {
            label: "Data: AramMeta".into(),
            url: format!("{SITE}/?{REF}"),
            champion_url: format!("{SITE}/champions/{{alias}}/?{REF}"),
        }),
        champions,
        augments,
        detail_url: format!("{SITE}/{base}/{{id}}.json?v={version}&{REF}"),
        detail_version: version,
    })
}

fn ids_of(entry: &Value) -> Vec<i64> {
    entry["items"].as_array().into_iter().flatten().filter_map(|x| x["id"].as_i64()).collect()
}

fn by_pick_desc(list: &Value) -> Vec<&Value> {
    let mut v: Vec<&Value> = list.as_array().into_iter().flatten().collect();
    v.sort_by(|a, b| b["pick"].as_f64().unwrap_or(0.0).total_cmp(&a["pick"].as_f64().unwrap_or(0.0)));
    v
}

pub fn convert_detail(v: &Value, base: &ChampStats) -> ChampStats {
    // 完整强化表：推荐（top）+ 不推荐（bot），按排名分档
    let mut augments = ranked_aug_list(&[&v["top"], &v["bot"]]);
    if augments.is_empty() {
        augments = base.augments.clone();
    }

    // 鞋子按选取率排序（最常见的在前），核心 = 最常见的核心组合 + 其最常见的后续选择
    let boots: Vec<i64> = by_pick_desc(&v["boots"]["top"]).iter().flat_map(|e| ids_of(e)).take(2).collect();
    let group = &v["itemClusters"]["groups"][0];
    let mut core: Vec<i64> = group["core"].as_array().into_iter().flatten().filter_map(|x| x["id"].as_i64()).collect();
    for o in by_pick_desc(&group["options"]) {
        if core.len() >= 5 {
            break;
        }
        if let Some(id) = o["id"].as_i64().filter(|id| !core.contains(id) && !boots.contains(id)) {
            core.push(id);
        }
    }
    let situational: Vec<i64> = v["singleItems"]["top"]
        .as_array()
        .into_iter()
        .flatten()
        .flat_map(ids_of)
        .filter(|id| !core.contains(id) && !boots.contains(id))
        .take(6)
        .collect();
    let items = (!core.is_empty() || !boots.is_empty()).then_some(ItemBuild {
        start: Vec::new(),
        boots,
        core,
        situational,
    });

    let archetypes: Vec<Archetype> = v["itemClusters"]["groups"]
        .as_array()
        .into_iter()
        .flatten()
        .take(4)
        .map(|g| {
            let core: Vec<i64> = g["core"].as_array().into_iter().flatten().filter_map(|x| x["id"].as_i64()).collect();
            let options = by_pick_desc(&g["options"])
                .iter()
                .filter_map(|o| o["id"].as_i64())
                .filter(|id| !core.contains(id))
                .take(4)
                .collect();
            Archetype {
                name: g["name_zh"].as_str().or_else(|| g["name_en"].as_str()).unwrap_or("流派").to_string(),
                tier: archetype_tier(g["lift"].as_f64().unwrap_or(0.0)).into(),
                core,
                options,
                win_rate: g["wr"].as_f64(),
                pick_rate: g["pick"].as_f64(),
                games: g["g"].as_i64(),
            }
        })
        .collect();
    let aug_styles = v["augTypes"]["top"]
        .as_array()
        .into_iter()
        .flatten()
        .take(4)
        .filter_map(|t| {
            Some(AugStyle {
                name: t["name_zh"].as_str().or_else(|| t["name_en"].as_str())?.to_string(),
                pick_rate: t["pick"].as_f64(),
            })
        })
        .collect();

    ChampStats {
        augments,
        spells: ids_of(&v["spells"]["top"][0]),
        items,
        archetypes,
        aug_styles,
        detailed: true,
        ..base.clone()
    }
}

/// 总表：同一 CDragon 版本内最多 3 天刷新一次；网络失败时回退到旧缓存。
pub async fn index(http: &reqwest::Client, cache_dir: &Path, cdragon_patch: &str, force: bool) -> Result<StatsPack> {
    let file = cache_dir.join("arammeta-index.json");
    let cached: Option<IndexCache> = std::fs::read(&file).ok().and_then(|b| serde_json::from_slice(&b).ok());
    if let Some(c) = &cached {
        let fresh = now().saturating_sub(c.fetched_at) < REFRESH_SECS;
        if !force && fresh && c.cdragon_patch == cdragon_patch {
            return Ok(c.pack.clone());
        }
    }
    let fetched = fetch_json(http, &format!("{SITE}/api/tier-list.json?{REF}"))
        .await
        .and_then(|v| convert_index(&v));
    match (fetched, cached) {
        (Ok(pack), _) => {
            let _ = std::fs::create_dir_all(cache_dir);
            let c = IndexCache { fetched_at: now(), cdragon_patch: cdragon_patch.into(), pack: pack.clone() };
            let _ = std::fs::write(&file, serde_json::to_vec(&c)?);
            Ok(pack)
        }
        (Err(e), Some(c)) => {
            log::warn!("arammeta 更新失败，使用缓存：{e:#}");
            Ok(c.pack)
        }
        (Err(e), None) => Err(e),
    }
}

fn safe(s: &str) -> String {
    s.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '-').collect()
}

/// 英雄详情：按数据版本缓存在 `cache/arammeta/<version>/<id>.json`
pub async fn detail(
    http: &reqwest::Client,
    cache_dir: &Path,
    url_template: &str,
    version: &str,
    champ: i64,
    base: &ChampStats,
) -> Result<ChampStats> {
    let dir = cache_dir.join("arammeta").join(safe(version));
    let file = dir.join(format!("{champ}.json"));
    if let Some(st) = std::fs::read(&file).ok().and_then(|b| serde_json::from_slice::<ChampStats>(&b).ok()) {
        // 档位/胜率以最新总表为准，详情部分用缓存
        return Ok(ChampStats { tier: base.tier.clone(), score: base.score, win_rate: base.win_rate, games: base.games, ..st });
    }
    let v = fetch_json(http, &url_template.replace("{id}", &champ.to_string())).await?;
    let st = convert_detail(&v, base);
    // 旧版本的详情缓存没用了，顺手清掉
    if let Some(parent) = dir.parent() {
        if let Ok(rd) = std::fs::read_dir(parent) {
            for e in rd.flatten() {
                if e.file_name().to_string_lossy() != safe(version) {
                    let _ = std::fs::remove_dir_all(e.path());
                }
            }
        }
    }
    std::fs::create_dir_all(&dir)?;
    std::fs::write(&file, serde_json::to_vec(&st)?)?;
    Ok(st)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn index_fixture() -> Value {
        json!({
            "patch_prefix": "16.23",
            "detailBase": "api/champions",
            "detailVersion": "20261124-87dcad327aa5",
            "patchChanges": {"currentGames": 10000},
            "champs": {
                "103": {"wr": 0.52, "g": 1000, "top": {
                    "kPrismatic": [{"id": 1156, "score": 0.0468, "pick": 0.05, "wr": 0.57}],
                    "kGold": [{"id": 1029, "score": 0.01, "pick": 0.02}],
                    "kSilver": []
                }},
                "10": {"wr": 0.57, "g": 900, "top": {}},
                "22": {"wr": 0.45, "g": 800, "top": {}}
            },
            "augs": {"1156": {"lift": 0.05, "wr": 0.6, "g": 5000}, "2009": {"lift": -0.03}}
        })
    }

    #[test]
    fn index_conversion() {
        let p = convert_index(&index_fixture()).unwrap();
        assert_eq!(p.patch, "16.23");
        assert_eq!(p.updated_at, "2026-11-24");
        assert_eq!(p.champions["10"].tier, "S+");
        // 3 个英雄时第 3 名的百分位是 0.67 → B
        assert_eq!(p.champions["22"].tier, "B");
        assert!((p.champions["103"].pick_rate.unwrap() - 0.1).abs() < 1e-9);
        assert!((p.augments["1156"].pick_rate.unwrap() - 0.05).abs() < 1e-9);
        let ahri = &p.champions["103"];
        assert_eq!(ahri.win_rate, Some(0.52));
        assert_eq!(ahri.augments[0].tier, "S+");
        assert_eq!(ahri.augments[1].tier, "A");
        assert_eq!(p.augments["1156"].tier, "S+");
        assert_eq!(p.augments["2009"].tier, "D");
        assert_eq!(
            p.detail_url,
            "https://arammeta.com/api/champions/{id}.json?v=20261124-87dcad327aa5&ref=hexpilot"
        );
        // 强化胜率不会出现在结果里
        let s = serde_json::to_string(&p.augments).unwrap();
        assert!(!s.contains("0.6") && !s.contains("winRate"));
    }

    #[test]
    fn detail_conversion() {
        let v = json!({
            "top": {"kGold": [{"id": 1, "score": 0.03, "pick": 0.1}]},
            "bot": {"kGold": [{"id": 2, "score": -0.1, "pick": 0.01}, {"id": 1, "score": 0.0}]},
            "boots": {"top": [
                {"pick": 0.05, "items": [{"id": 3008}]},
                {"pick": 0.58, "items": [{"id": 3020}]}
            ]},
            "augTypes": {"top": [{"name_zh": "近戰通用", "pick": 0.07, "wr": 0.52}]},
            "itemClusters": {"groups": [{"name_zh": "AP爆發", "wr": 0.522, "lift": 0.012, "pick": 0.38, "g": 45332,
                "core": [{"id": 6655}, {"id": 4645}], "options": [
                {"id": 123430, "pick": 0.004}, {"id": 3089, "pick": 0.26}, {"id": 3135, "pick": 0.14}, {"id": 4646, "pick": 0.24}
            ]}]},
            "singleItems": {"top": [{"items": [{"id": 3089}]}, {"items": [{"id": 3157}]}]},
            "spells": {"top": [{"items": [{"id": 4}, {"id": 6}]}]}
        });
        let base = ChampStats { tier: "A".into(), ..Default::default() };
        let d = convert_detail(&v, &base);
        assert!(d.detailed);
        assert_eq!(d.tier, "A");
        assert_eq!(d.augments.iter().map(|a| (a.id, a.tier.as_str())).collect::<Vec<_>>(), vec![(1, "S+"), (2, "B")]);
        let it = d.items.unwrap();
        assert_eq!(it.boots, vec![3020, 3008]);
        assert_eq!(it.core, vec![6655, 4645, 3089, 4646, 3135]);
        assert_eq!(it.situational, vec![3157]);
        assert_eq!(d.spells, vec![4, 6]);
        let a = &d.archetypes[0];
        assert_eq!((a.name.as_str(), a.tier.as_str(), a.win_rate), ("AP爆發", "S", Some(0.522)));
        assert_eq!(a.options, vec![3089, 4646, 3135, 123430]);
        assert_eq!(d.aug_styles[0].name, "近戰通用");
        // 强化风格不带胜率
        assert!(!serde_json::to_string(&d.aug_styles).unwrap().contains("0.52"));
    }

    /// 用真实数据冒烟：ARAMMETA_INDEX=/path/tier-list.json ARAMMETA_DETAIL=/path/103.json cargo test -- --ignored
    #[test]
    #[ignore]
    fn real_files() {
        let idx: Value = serde_json::from_slice(&std::fs::read(std::env::var("ARAMMETA_INDEX").unwrap()).unwrap()).unwrap();
        let p = convert_index(&idx).unwrap();
        let mut tiers: HashMap<String, usize> = HashMap::new();
        for c in p.champions.values() {
            *tiers.entry(c.tier.clone()).or_default() += 1;
        }
        println!("champions {} tiers {:?} augs {}", p.champions.len(), tiers, p.augments.len());
        let mut at: HashMap<String, usize> = HashMap::new();
        for a in p.augments.values() {
            *at.entry(a.tier.clone()).or_default() += 1;
        }
        println!("global aug tiers {at:?}");
        let det: Value = serde_json::from_slice(&std::fs::read(std::env::var("ARAMMETA_DETAIL").unwrap()).unwrap()).unwrap();
        let d = convert_detail(&det, &p.champions["103"]);
        let mut dt: HashMap<String, usize> = HashMap::new();
        for a in &d.augments {
            *dt.entry(a.tier.clone()).or_default() += 1;
        }
        println!("ahri {} {:?} augs {} {:?}", d.tier, d.win_rate, d.augments.len(), dt);
        println!("items {:?} spells {:?}", d.items, d.spells);
    }
}
