//! 把推荐写入客户端：装备方案、召唤师技能（海克斯大乱斗禁用符文，所以不处理符文页）。
//! 只通过 LCU 接口操作，等同于玩家在客户端里手动设置。
//!
//! 安全原则：只创建/替换本工具自己创建的装备方案（uid 以 `haidou-` 开头），
//! 绝不删除或覆盖玩家自己的方案。

use crate::data::{Catalog, ChampStats, ItemBuild};
use crate::lcu::http::LcuClient;
use anyhow::{bail, Context, Result};
use reqwest::Method;
use serde_json::{json, Value};

const PAGE_PREFIX: &str = "海斗";
const SET_PREFIX: &str = "haidou-";

pub fn champ_label(cat: &Catalog, champ_id: i64) -> String {
    cat.champions
        .iter()
        .find(|c| c.id == champ_id)
        .map(|c| c.name.clone())
        .unwrap_or_else(|| champ_id.to_string())
}

pub async fn items(client: &LcuClient, champ_id: i64, label: &str, st: &ChampStats, cat: &Catalog) -> Result<()> {
    let me = client.get("/lol-summoner/v1/current-summoner").await?;
    let sid = me["summonerId"].as_i64().context("读取召唤师 ID 失败")?;
    let path = format!("/lol-item-sets/v1/item-sets/{sid}/sets");
    let mut all = client.get(&path).await?;

    let blocks = item_blocks(st);
    if blocks.is_empty() {
        bail!("数据中没有该英雄的出装");
    }

    let uid = format!("{SET_PREFIX}{champ_id}");
    let set = json!({
        "uid": uid,
        "title": set_title(label, st, cat),
        "type": "custom",
        "map": "any",
        "mode": "any",
        "startedFrom": "blank",
        "sortrank": 0,
        "associatedChampions": [champ_id],
        "associatedMaps": [],
        "preferredItemSlots": [],
        "blocks": blocks,
    });

    let sets = all["itemSets"].as_array_mut().context("装备方案格式异常")?;
    sets.retain(|s| s["uid"].as_str() != Some(uid.as_str()));
    sets.push(set);
    client.request(Method::PUT, &path, Some(&all)).await?;
    Ok(())
}

pub async fn spells(client: &LcuClient, ids: &[i64]) -> Result<()> {
    let [a, b] = ids else { bail!("数据包中的召唤师技能应为 2 个") };
    client
        .request(
            Method::PATCH,
            "/lol-champ-select/v1/session/my-selection",
            Some(&json!({"spell1Id": a, "spell2Id": b})),
        )
        .await?;
    Ok(())
}

fn pct(v: Option<f64>) -> String {
    v.map(|x| format!("{:.1}%", x * 100.0)).unwrap_or_default()
}

/// 游戏商店里显示的方案名，例如「海斗 · 阿狸 S 胜率52.3% · v16.23」
fn set_title(label: &str, st: &ChampStats, cat: &Catalog) -> String {
    let wr = st.win_rate.map(|_| format!(" 胜率{}", pct(st.win_rate))).unwrap_or_default();
    format!("{PAGE_PREFIX} · {label} {}{wr} · v{}", st.tier, cat.patch).replace("  ", " ")
}

/// 装备方案分组：鞋子 → 每个流派一组（标题带档位/胜率/选取率）→ 可选装。
/// 没有流派数据时退回 出门/鞋子/核心/可选 四组。
pub fn item_blocks(st: &ChampStats) -> Vec<Value> {
    let block = |title: String, ids: &[i64]| {
        json!({
            "type": title,
            "items": ids.iter().map(|i| json!({"id": i.to_string(), "count": 1})).collect::<Vec<_>>(),
        })
    };
    let empty = ItemBuild::default();
    let b = st.items.as_ref().unwrap_or(&empty);
    let mut out = Vec::new();
    if !b.start.is_empty() {
        out.push(block("出门装".into(), &b.start));
    }
    if !b.boots.is_empty() {
        out.push(block("鞋子".into(), &b.boots));
    }
    if st.archetypes.is_empty() {
        if !b.core.is_empty() {
            out.push(block("核心装".into(), &b.core));
        }
    } else {
        for a in &st.archetypes {
            let ids: Vec<i64> = a.core.iter().chain(a.options.iter()).copied().collect();
            if ids.is_empty() {
                continue;
            }
            let mut title = format!("{} [{}]", a.name, a.tier);
            if a.win_rate.is_some() {
                title += &format!(" 胜率{}", pct(a.win_rate));
            }
            if a.pick_rate.is_some() {
                title += &format!(" 选取{}", pct(a.pick_rate));
            }
            out.push(block(title, &ids));
        }
    }
    if !b.situational.is_empty() {
        out.push(block("可选装".into(), &b.situational));
    }
    out
}

pub struct Which {
    pub items: bool,
    pub spells: bool,
}

/// 按开关依次应用，返回人类可读的结果摘要
pub async fn apply_all(client: &LcuClient, cat: &Catalog, champ_id: i64, st: &ChampStats, w: Which) -> Vec<String> {
    let label = champ_label(cat, champ_id);
    let mut out = Vec::new();
    if w.items {
        if st.items.is_some() || !st.archetypes.is_empty() {
            out.push(match items(client, champ_id, &label, st, cat).await {
                Ok(_) => "出装 ✓".into(),
                Err(e) => format!("出装 ✗ {e:#}"),
            });
        }
    }
    if w.spells && !st.spells.is_empty() {
        out.push(match spells(client, &st.spells).await {
            Ok(_) => "技能 ✓".into(),
            Err(e) => format!("技能 ✗ {e:#}"),
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::Archetype;

    #[test]
    fn blocks_per_archetype() {
        let st = ChampStats {
            tier: "S".into(),
            win_rate: Some(0.5233),
            items: Some(ItemBuild { boots: vec![3020], situational: vec![3157], ..Default::default() }),
            archetypes: vec![Archetype {
                name: "AP爆發".into(), tier: "S".into(), core: vec![6655, 4645], options: vec![3089],
                win_rate: Some(0.522), pick_rate: Some(0.384), games: None,
            }],
            ..Default::default()
        };
        let b = item_blocks(&st);
        let titles: Vec<&str> = b.iter().map(|x| x["type"].as_str().unwrap()).collect();
        assert_eq!(titles, vec!["鞋子", "AP爆發 [S] 胜率52.2% 选取38.4%", "可选装"]);
        assert_eq!(b[1]["items"][2]["id"], "3089");
        let cat = Catalog { patch: "16.23".into(), ..Default::default() };
        assert_eq!(set_title("阿狸", &st, &cat), "海斗 · 阿狸 S 胜率52.3% · v16.23");
    }
}
