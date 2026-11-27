//! 界面三语（简体中文 / 繁體中文 / English）。
//! 数据语言跟随界面语言；后端发给用户的提示文本也在这里翻译。

use std::collections::HashMap;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::OnceLock;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Lang {
    Hans = 0,
    Hant = 1,
    En = 2,
}

static LANG: AtomicU8 = AtomicU8::new(0);

pub fn set(l: Lang) {
    LANG.store(l as u8, Ordering::Relaxed);
}

pub fn get() -> Lang {
    match LANG.load(Ordering::Relaxed) {
        1 => Lang::Hant,
        2 => Lang::En,
        _ => Lang::Hans,
    }
}

pub fn code(l: Lang) -> &'static str {
    match l {
        Lang::Hans => "zh-Hans",
        Lang::Hant => "zh-Hant",
        Lang::En => "en",
    }
}

/// 从 BCP-47 / POSIX 语言标签推断：zh-TW、zh-HK、zh-MO、zh-Hant → 繁体；其他 zh → 简体；其余 → 英文
pub fn from_tag(tag: &str) -> Lang {
    let t = tag.to_lowercase().replace('_', "-");
    if !t.starts_with("zh") {
        return Lang::En;
    }
    if ["hant", "-tw", "-hk", "-mo"].iter().any(|k| t.contains(k)) {
        Lang::Hant
    } else {
        Lang::Hans
    }
}

/// 设置值（"auto" | "zh-Hans" | "zh-Hant" | "en"）→ 实际语言
pub fn resolve(setting: &str) -> Lang {
    match setting {
        "zh-Hans" => Lang::Hans,
        "zh-Hant" => Lang::Hant,
        "en" => Lang::En,
        _ => from_tag(&system_tag()),
    }
}

/// 界面语言 → CommunityDragon 数据语言目录（简体用国际服简中译名）
pub fn data_locale(l: Lang) -> &'static str {
    match l {
        Lang::Hans => "zh_my",
        Lang::Hant => "zh_tw",
        Lang::En => "default",
    }
}

/// 界面语言对应的 OCR 语言（客户端语言未知时使用）
pub fn ocr_tag(l: Lang) -> &'static str {
    match l {
        Lang::Hans => "zh_MY",
        Lang::Hant => "zh_TW",
        Lang::En => "en_US",
    }
}

#[cfg(windows)]
fn system_tag() -> String {
    use windows::Win32::Globalization::GetUserDefaultLocaleName;
    let mut buf = [0u16; 85];
    let n = unsafe { GetUserDefaultLocaleName(&mut buf) };
    if n > 1 {
        String::from_utf16_lossy(&buf[..(n as usize - 1)])
    } else {
        String::new()
    }
}

#[cfg(not(windows))]
fn system_tag() -> String {
    ["LC_ALL", "LC_MESSAGES", "LANG"]
        .iter()
        .find_map(|k| std::env::var(k).ok().filter(|v| !v.is_empty()))
        .unwrap_or_default()
}

macro_rules! table {
    ($($key:literal => [$hans:literal, $hant:literal, $en:literal]),* $(,)?) => {
        pub fn t_in(lang: Lang, key: &str) -> &'static str {
            let i = lang as usize;
            match key {
                $($key => [$hans, $hant, $en][i],)*
                _ => "?",
            }
        }

        pub fn t(key: &str) -> &'static str {
            t_in(get(), key)
        }
    };
}

table! {
    "not_connected" => ["未连接客户端", "未連接用戶端", "League client not connected"],
    "data_not_loaded" => ["数据尚未加载", "資料尚未載入", "Data not loaded yet"],
    "no_champ_data" => ["数据中没有该英雄", "資料中沒有該英雄", "No data for this champion"],
    "nothing_to_import" => ["没有可导入的内容", "沒有可匯入的內容", "Nothing to import"],
    "no_build" => ["数据中没有该英雄的出装", "資料中沒有該英雄的出裝", "No item build for this champion"],
    "bad_hotkey" => ["热键格式不正确", "熱鍵格式不正確", "Invalid hotkey"],
    "link_denied" => ["不允许打开该链接", "不允許開啟該連結", "This link is not allowed"],
    "in_game_no_change" => ["对局中无法修改，请在游戏内 Esc → 视频 → 窗口模式 调整", "對戰中無法修改，請在遊戲內 Esc → 影像 → 視窗模式 調整", "Can't change during a game. Use Esc → Video → Window Mode in game"],
    "readback_mismatch" => ["写入后读回的值不是无边框，请在游戏内 Esc → 视频 → 窗口模式 手动确认", "寫入後讀回的值不是無邊框，請在遊戲內 Esc → 影像 → 視窗模式 手動確認", "Read-back value is not borderless; please check Esc → Video → Window Mode in game"],
    "fullscreen_warn" => ["当前是全屏模式，强化识别不可用。可在「选人」页一键改为无边框", "目前是全螢幕模式，無法辨識強化。可在「選角」頁一鍵改為無邊框", "Fullscreen detected: augment recognition won't work. Switch to borderless from the Champ Select tab"],
    "summoner_id_failed" => ["读取召唤师 ID 失败", "讀取召喚師 ID 失敗", "Failed to read summoner ID"],
    "item_sets_bad" => ["装备方案格式异常", "裝備方案格式異常", "Unexpected item-set format"],
    "spells_need_two" => ["召唤师技能应为 2 个", "召喚師技能應為 2 個", "Exactly 2 summoner spells are required"],
    "items" => ["出装", "出裝", "Items"],
    "spells" => ["技能", "技能", "Spells"],
    "block_start" => ["出门装", "出門裝", "Starting items"],
    "block_boots" => ["鞋子", "鞋子", "Boots"],
    "block_core" => ["核心装", "核心裝", "Core items"],
    "block_situational" => ["可选装", "可選裝", "Situational"],
    "win_rate" => ["胜率", "勝率", "WR"],
    "pick_rate" => ["选取", "選取", "Pick"],
    "ocr_missing" => ["系统未安装该语言的 OCR 组件：设置 → 时间和语言 → 语言和区域 → 对应语言 → 语言选项 → 安装「光学字符识别」", "系統未安裝該語言的 OCR 元件：設定 → 時間與語言 → 語言與地區 → 對應語言 → 語言選項 → 安裝「光學字元辨識」", "OCR component for this language is missing: Settings → Time & language → Language & region → the language → Language options → Optical character recognition"],
    "custom_pack_read_failed" => ["读取本地数据包失败", "讀取本機資料包失敗", "Failed to read local data pack"],
    "custom_pack_bad" => ["数据包格式不正确", "資料包格式不正確", "Invalid data pack format"],
    "request_failed" => ["请求失败", "請求失敗", "Request failed"],
}

/// 繁体 → 简体（只覆盖游戏统计里常见的字，够用于流派/风格名称）
pub fn to_simplified(s: &str) -> String {
    static MAP: OnceLock<HashMap<char, char>> = OnceLock::new();
    let map = MAP.get_or_init(|| {
        const PAIRS: &str = "發发 燒烧 戰战 擊击 輸输 裝装 輔辅 遠远 術术 師师 傷伤 體体 屬属 雙双 點点 擴扩 範范 圍围 應应 對对 單单 動动 護护 線线 經经 濟济 機机 卻却 時时 間间 續续 種种 類类 強强 純纯 綜综 輕轻 鬥斗 遊游 殺杀 復复 療疗 癒愈 衛卫 團团 隊队 場场 開开 實实 韌韧 視视 喚唤 寵宠 備备 書书 劍剑 槍枪 錘锤 鎧铠 頭头 環环 鏈链 墜坠 寶宝 燈灯 鐘钟 錶表 風风 電电 聖圣 靈灵 龍龙 獸兽 貓猫 鳥鸟 蟲虫 魚鱼 戲戏 劇剧 樂乐 藝艺 學学 問问 題题 選选 擇择 條条 狀状 態态 數数 據据 統统 計计 勝胜 負负 員员 個个 們们 這这 與与 為为 從从 後后 會会 過过 還还 進进 運运 達达 邊边 號号 稱称 結结 構构 組组 織织 參参 變变 換换 轉转 輪轮 迴回 歸归 總总 價价 錢钱 買买 賣卖 費费 資资 嚴严 厲厉 極极 優优 勢势 憤愤 嘯啸 鳴鸣 聲声 響响 熱热 溫温 濕湿 淨净 潔洁 閃闪 現现 標标 記记 虛虚 傳传 懲惩 晰晰 寬宽 長长 廣广 東东 車车 門门 關关 閉闭 陣阵 營营 壘垒 礎础 職职 業业 專专 處处 務务 傑杰 遲迟 緩缓 準准 確确 擁拥 協协 調调 節节 連连 鎖锁 繞绕 盤盘 渦涡 齊齐 滿满 損损 盡尽 萬万 兩两 啟启 層层 疊叠 積积 豐丰 餘余 貪贪 脛胫 愛爱 歐欧 亞亚 衝冲 擋挡 釋释 減减 緣缘 際际 環环 槌槌 輸输 帶带 領领 導导 魔魔 爭争 側侧 揮挥 發发 揚扬 靈灵 頻频 頓顿 範范 輔辅 牆墙 壓压 縮缩 擾扰 斷断 獨独 驅驱 禦御 補补 給给 鋼钢 鐵铁 銀银 銅铜 劃划 計计 錯错 誤误 讓让 說说 話话 語语 讀读 寫写 聽听 觀观 覺觉 親亲 戀恋 歡欢 樂乐 憂忧 慮虑 驚惊 險险 勵励 獎奖 賞赏 贏赢 輸输 敗败 劫劫 異异 階阶 級级 創创 刪删 鍵键 盤盘 據据 確确 認认 證证 識识 譜谱 詳详 細细 紹绍 嗎吗 麼么 囉啰 ・·";
        PAIRS
            .split_whitespace()
            .filter_map(|p| {
                let mut c = p.chars();
                Some((c.next()?, c.next()?))
            })
            .collect()
    });
    s.chars().map(|c| *map.get(&c).unwrap_or(&c)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tags() {
        assert_eq!(from_tag("zh-TW"), Lang::Hant);
        assert_eq!(from_tag("zh_HK.UTF-8"), Lang::Hant);
        assert_eq!(from_tag("zh-Hant-MO"), Lang::Hant);
        assert_eq!(from_tag("zh-CN"), Lang::Hans);
        assert_eq!(from_tag("zh-SG"), Lang::Hans);
        assert_eq!(from_tag("en-US"), Lang::En);
        assert_eq!(from_tag("ja-JP"), Lang::En);
        assert_eq!(resolve("zh-Hant"), Lang::Hant);
    }

    #[test]
    fn simplify() {
        assert_eq!(to_simplified("AP爆發 / AP燃燒"), "AP爆发 / AP燃烧");
        assert_eq!(to_simplified("近戰普攻型・魔攻"), "近战普攻型·魔攻");
        assert_eq!(to_simplified("貪婪護脛"), "贪婪护胫");
    }

    #[test]
    fn translate() {
        assert_eq!(t_in(Lang::Hant, "items"), "出裝");
        assert_eq!(t_in(Lang::En, "win_rate"), "WR");
        assert_eq!(t_in(Lang::Hans, "not_connected"), "未连接客户端");
    }
}
