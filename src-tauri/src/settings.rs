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
    /// 界面语言："auto"（跟随系统）| "zh-Hans" | "zh-Hant" | "en"；数据语言随之变化
    pub ui_lang: String,
    /// 简体中文用哪套译名："cn" 国服（默认）| "my" 国际服
    pub zh_variant: String,
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
    /// 后台自动检查并更新数据（对局和选人期间不更新）
    pub auto_update: bool,
    /// 选人期间显示悬浮窗
    pub float_enabled: bool,
    /// 强化悬浮条下沿位置（占游戏画面高度的比例）。默认在卡片上方，
    /// 不能放在卡片下方：那里是重随按钮
    pub overlay_bottom: f32,
    /// 悬浮窗位置（物理像素），None 表示用默认位置
    pub float_pos: Option<(i32, i32)>,
    /// 选人时悬浮窗相对客户端窗口的偏移，None 表示默认（板凳卡片下方居中）
    pub float_offset: Option<(i32, i32)>,
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
            ui_lang: "zh-Hans".into(),
            zh_variant: "cn".into(),
            auto_items: true,
            auto_spells: true,
            ocr_enabled: true,
            overlay_enabled: true,
            ocr_lang: "auto".into(),
            ocr_hotkey: "Ctrl+Shift+A".into(),
            auto_update: true,
            float_enabled: true,
            overlay_bottom: crate::ocr::CARD_TOP - 0.012,
            float_pos: None,
            float_offset: None,
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

/// OCR 引擎优先级列表（BCP-47 标签）。中文环境下两种字形都列上，识别时轮换并锁定命中的引擎。
pub fn ocr_langs(setting: &str, client_locale: Option<&str>) -> Vec<String> {
    let both_zh = |first: &str| {
        let other = if first == "zh-Hans" { "zh-Hant" } else { "zh-Hans" };
        vec![first.to_string(), other.to_string()]
    };
    match setting {
        "zh-Hans" | "zh-Hant" => both_zh(setting),
        // 游戏画面的语言由客户端决定；客户端语言未知时按界面语言猜
        "auto" | "" => match client_locale
            .map(str::to_string)
            .unwrap_or_else(|| crate::i18n::ocr_tag(crate::i18n::get()).to_string())
            .to_lowercase()
            .as_str()
        {
            "zh_tw" | "zh_hk" => both_zh("zh-Hant"),
            "zh_cn" | "zh_my" | "zh_sg" => both_zh("zh-Hans"),
            "" => both_zh("zh-Hans"),
            l => vec![l.replace('_', "-")],
        },
        other => vec![other.to_string()],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locales() {
        assert_eq!(ocr_langs("auto", Some("zh_TW")), vec!["zh-Hant", "zh-Hans"]);
        assert_eq!(ocr_langs("auto", Some("zh_MY")), vec!["zh-Hans", "zh-Hant"]);
        assert_eq!(ocr_langs("auto", Some("en_GB")), vec!["en-gb"]);
        assert_eq!(ocr_langs("en-US", Some("zh_TW")), vec!["en-US"]);
    }
}
