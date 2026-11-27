use serde::{Deserialize, Serialize};
use tauri::AppHandle;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    /// 自动接受对局（找到对局后延迟一段时间再接受）。
    pub auto_accept: bool,
    pub accept_delay_ms: u64,
    pub always_on_top: bool,
    /// 可选：手动指定客户端目录（含 lockfile），自动发现失败时使用。
    pub league_dir: String,
    /// 统计来源："arammeta" | "custom"
    pub stats_source: String,
    /// 自定义数据包：http(s) URL 或本地 JSON 文件路径（stats_source = custom 时使用）
    pub stats_url: String,
    /// CommunityDragon 根地址，网络不通时可换成镜像。
    pub cdragon_base: String,
    /// 数据语言："auto"（跟随客户端）或 CommunityDragon 语言目录，如 zh_tw / zh_my / zh_cn / default
    pub locale: String,
    /// 大乱斗禁用符文，所以只有出装和召唤师技能
    pub auto_items: bool,
    pub auto_spells: bool,
    /// 对局中 OCR 识别海克斯强化
    pub ocr_enabled: bool,
    pub overlay_enabled: bool,
    /// OCR 语言："auto" | "zh-Hans" | "zh-Hant" | "en-US"
    pub ocr_lang: String,
    /// 手动触发识别的全局热键
    pub ocr_hotkey: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            auto_accept: false,
            accept_delay_ms: 1500,
            always_on_top: false,
            league_dir: String::new(),
            stats_source: "arammeta".into(),
            stats_url: String::new(),
            cdragon_base: "https://raw.communitydragon.org".into(),
            locale: "auto".into(),
            auto_items: true,
            auto_spells: false,
            ocr_enabled: true,
            overlay_enabled: true,
            ocr_lang: "auto".into(),
            ocr_hotkey: "Ctrl+Shift+A".into(),
        }
    }
}

pub fn load(app: &AppHandle) -> Settings {
    std::fs::read_to_string(crate::paths::data_dir(app).join("settings.json"))
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save(app: &AppHandle, s: &Settings) -> anyhow::Result<()> {
    let dir = crate::paths::data_dir(app);
    std::fs::create_dir_all(&dir)?;
    std::fs::write(dir.join("settings.json"), serde_json::to_vec_pretty(s)?)?;
    Ok(())
}

/// 客户端语言（如 "zh_TW"）→ CommunityDragon 目录名
pub fn cdragon_locale(client_locale: &str) -> String {
    let l = client_locale.to_lowercase();
    match l.as_str() {
        "" => "default".into(),
        "en_us" => "default".into(),
        _ => l,
    }
}

/// OCR 引擎优先级列表（BCP-47 标签）。中文环境下两种字形都列上，识别时轮换并锁定命中的引擎。
pub fn ocr_langs(setting: &str, client_locale: Option<&str>) -> Vec<String> {
    let both_zh = |first: &str| {
        let other = if first == "zh-Hans" { "zh-Hant" } else { "zh-Hans" };
        vec![first.to_string(), other.to_string()]
    };
    match setting {
        "zh-Hans" | "zh-Hant" => both_zh(setting),
        "auto" | "" => match client_locale.map(str::to_lowercase).as_deref() {
            Some("zh_tw" | "zh_hk") => both_zh("zh-Hant"),
            Some("zh_cn" | "zh_my" | "zh_sg") => both_zh("zh-Hans"),
            Some(l) if !l.is_empty() => vec![l.replace('_', "-")],
            // 还不知道客户端语言：简繁都试
            _ => both_zh("zh-Hans"),
        },
        other => vec![other.to_string()],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locales() {
        assert_eq!(cdragon_locale("en_US"), "default");
        assert_eq!(cdragon_locale("zh_TW"), "zh_tw");
        assert_eq!(ocr_langs("auto", Some("zh_TW")), vec!["zh-Hant", "zh-Hans"]);
        assert_eq!(ocr_langs("auto", Some("zh_MY")), vec!["zh-Hans", "zh-Hant"]);
        assert_eq!(ocr_langs("auto", None), vec!["zh-Hans", "zh-Hant"]);
        assert_eq!(ocr_langs("auto", Some("en_GB")), vec!["en-gb"]);
        assert_eq!(ocr_langs("en-US", Some("zh_TW")), vec!["en-US"]);
    }
}
