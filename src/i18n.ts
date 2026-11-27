import { createSignal } from "solid-js";

export type Lang = "zh-Hans" | "zh-Hant" | "en";
const [lang, setLangSignal] = createSignal<Lang>("zh-Hans");
export { lang };
export const setLang = (l: string) => setLangSignal((["zh-Hans", "zh-Hant", "en"].includes(l) ? l : "zh-Hans") as Lang);

const IDX: Record<Lang, number> = { "zh-Hans": 0, "zh-Hant": 1, en: 2 };

/** [简体, 繁體, English] */
const D = {
  // 顶栏 / 导航
  waiting: ["等待客户端…", "等待用戶端…", "Waiting for client…"],
  mayhem: ["海克斯大乱斗", "海克斯大亂鬥", "ARAM: Mayhem"],
  queue: ["队列 {q}", "佇列 {q}", "Queue {q}"],
  dead: ["阵亡", "陣亡", "Dead"],
  tab_select: ["选人", "選角", "Champ Select"],
  tab_augments: ["强化", "強化", "Augments"],
  tab_settings: ["设置", "設定", "Settings"],
  // 阶段
  ph_None: ["空闲", "閒置", "Idle"],
  ph_Lobby: ["房间中", "房間中", "In lobby"],
  ph_Matchmaking: ["匹配中", "配對中", "Matchmaking"],
  ph_ReadyCheck: ["找到对局", "找到對戰", "Match found"],
  ph_ChampSelect: ["选人中", "選角中", "Champ select"],
  ph_GameStart: ["载入中", "載入中", "Loading"],
  ph_InProgress: ["对局中", "對戰中", "In game"],
  ph_Reconnect: ["等待重连", "等待重新連線", "Reconnect"],
  ph_WaitingForStats: ["结算中", "結算中", "Post-game"],
  ph_PreEndOfGame: ["结算中", "結算中", "Post-game"],
  ph_EndOfGame: ["对局结束", "對戰結束", "Game over"],
  // 通用
  win_rate: ["胜率", "勝率", "WR"],
  pick_rate: ["选取", "選取", "Pick"],
  no_data: ["无数据", "無資料", "No data"],
  not_selected: ["未选择", "未選擇", "None"],
  no_tier: ["无评级", "無評級", "Unrated"],
  silver: ["白银", "白銀", "Silver"],
  gold: ["黄金", "黃金", "Gold"],
  prismatic: ["棱彩", "稜彩", "Prismatic"],
  other: ["其他", "其他", "Other"],
  all: ["全部", "全部", "All"],
  // 显示模式
  wm_label: ["游戏显示模式：", "遊戲顯示模式：", "Game display mode: "],
  wm_0: ["全屏", "全螢幕", "Fullscreen"],
  wm_1: ["窗口", "視窗", "Windowed"],
  wm_2: ["无边框", "無邊框", "Borderless"],
  wm_unknown: ["未知", "未知", "Unknown"],
  wm_fullscreen_warn: ["全屏下无法识别强化，悬浮标签也不显示", "全螢幕下無法辨識強化，懸浮標籤也不會顯示", "Augment OCR and the overlay don't work in fullscreen"],
  wm_fix: ["改为无边框", "改為無邊框", "Switch to borderless"],
  wm_fixed: ["已改为无边框，下次进游戏生效", "已改為無邊框，下次進遊戲生效", "Switched to borderless; takes effect next game"],
  wm_in_game: ["对局中请按 Esc → 视频 → 窗口模式 调整", "對戰中請按 Esc → 影像 → 視窗模式 調整", "In game, use Esc → Video → Window Mode"],
  // 出装面板
  loading_build: ["正在加载出装与流派…", "正在載入出裝與流派…", "Loading builds…"],
  archetypes: ["出装流派", "出裝流派", "Build archetypes"],
  import_items: ["导入出装", "匯入出裝", "Import items"],
  import_items_tip: ["写入客户端装备方案，游戏内商店可直接使用", "寫入用戶端裝備方案，遊戲內商店可直接使用", "Writes a client item set usable in the in-game shop"],
  boots: ["鞋子", "鞋子", "Boots"],
  situational: ["可选", "可選", "Extra"],
  spells: ["技能", "技能", "Spells"],
  set_spells: ["设置技能", "設定技能", "Set spells"],
  aug_styles: ["强化", "強化", "Style"],
  // 强化
  pick_global_tip: ["全体玩家选取率", "全體玩家選取率", "Pick rate across all players"],
  pick_champ_tip: ["该英雄选取率", "該英雄選取率", "Pick rate on this champion"],
  no_aug_data: ["该英雄暂无强化数据", "該英雄暫無強化資料", "No augment data for this champion"],
  // 英雄榜 / 选人
  tier_list: ["英雄榜", "英雄榜", "Tier list"],
  search_champ: ["搜索英雄", "搜尋英雄", "Search champions"],
  stats_not_loaded: ["统计数据未加载", "統計資料未載入", "Stats not loaded"],
  stats_not_loaded_hint: ["统计数据未加载，可在「设置」中检查数据来源。", "統計資料未載入，可在「設定」中檢查資料來源。", "Stats not loaded. Check the data source in Settings."],
  wait_champ_select: ["进入选人阶段后自动显示推荐", "進入選角階段後自動顯示推薦", "Recommendations appear in champ select"],
  start_client: ["请先启动英雄联盟客户端", "請先啟動英雄聯盟用戶端", "Please start the League client"],
  my_champ: ["我的英雄", "我的英雄", "My champion"],
  reroll: ["重随（{n}）", "重骰（{n}）", "Reroll ({n})"],
  bench: ["板凳席", "板凳席", "Bench"],
  by_rating: ["按评分排序", "依評分排序", "sorted by rating"],
  none: ["暂无", "暫無", "None"],
  swap: ["换", "換", "Swap"],
  top_augs: ["推荐强化", "推薦強化", "Top augments"],
  top8: ["前 8", "前 8", "top 8"],
  teammates: ["队友", "隊友", "Teammates"],
  swap_failed: ["交换失败：", "交換失敗：", "Swap failed: "],
  reroll_failed: ["重随失败：", "重骰失敗：", "Reroll failed: "],
  // 强化页
  current_offer: ["当前强化（OCR）", "目前強化（OCR）", "Current offer (OCR)"],
  scan_now: ["立即识别", "立即辨識", "Scan now"],
  offer_hint_ingame: ["升级或阵亡时自动识别，也可按热键 / 点「立即识别」（需无边框模式）", "升級或陣亡時自動辨識，也可按熱鍵 / 點「立即辨識」（需無邊框模式）", "Scans on level-up or death; or use the hotkey / Scan now (borderless mode required)"],
  offer_hint_idle: ["对局中可用", "對戰中可用", "Available in game"],
  champion: ["英雄", "英雄", "Champion"],
  search_champ_long: ["搜索英雄（名称 / 称号 / 英文）", "搜尋英雄（名稱 / 稱號 / 英文）", "Search champions (name / title)"],
  champ_aug_tiers: ["该英雄强化档位", "該英雄強化評級", "Augment tiers for this champion"],
  aug_search: ["强化查询", "強化查詢", "Augment lookup"],
  aug_search_ph: ["输入强化名称（简 / 繁 / 英文均可）", "輸入強化名稱（簡 / 繁 / 英文皆可）", "Type an augment name (EN / 简 / 繁)"],
  no_aug_wr: ["强化只显示档位与选取率：Riot 开发者政策禁止展示强化胜率。", "強化只顯示評級與選取率：Riot 開發者政策禁止顯示強化勝率。", "Augments show tier and pick rate only: Riot's developer policy forbids augment win rates."],
  // 设置
  s_game: ["对局", "對戰", "Game"],
  s_auto_accept: ["自动接受对局", "自動接受對戰", "Auto-accept matches"],
  s_accept_delay: ["接受延迟（毫秒）", "接受延遲（毫秒）", "Accept delay (ms)"],
  s_auto_when: ["英雄确定后自动：", "英雄確定後自動：", "When champion is final:"],
  s_auto_items: ["导入出装", "匯入出裝", "Import items"],
  s_auto_spells: ["设置召唤师技能", "設定召喚師技能", "Set summoner spells"],
  s_ocr: ["强化识别", "強化辨識", "Augment recognition"],
  s_ocr_enabled: ["对局中 OCR 识别海克斯强化", "對戰中以 OCR 辨識海克斯強化", "Recognize augments with OCR in game"],
  s_overlay: ["在强化卡片上方显示档位标签", "在強化卡片上方顯示評級標籤", "Show tier tags above the augment cards"],
  s_ocr_lang: ["OCR 语言", "OCR 語言", "OCR language"],
  s_ocr_auto: ["自动（按已安装引擎与游戏语言）", "自動（依已安裝引擎與遊戲語言）", "Auto (installed engines)"],
  s_ocr_detected: ["已检测到的 OCR 引擎", "已偵測到的 OCR 引擎", "Detected OCR engines"],
  s_ocr_none: ["未检测到 OCR 引擎：请在 Windows 设置中安装中文「光学字符识别」组件", "未偵測到 OCR 引擎：請在 Windows 設定中安裝中文「光學字元辨識」元件", "No OCR engine detected: install the Optical character recognition component in Windows Settings"],
  s_ocr_hans: ["简体优先", "簡體優先", "Simplified Chinese first"],
  s_ocr_hant: ["繁体优先", "繁體優先", "Traditional Chinese first"],
  s_hotkey: ["识别热键（留空禁用）", "辨識熱鍵（留空停用）", "Scan hotkey (empty = off)"],
  s_ui: ["界面", "介面", "Interface"],
  s_ui_lang: ["语言（数据语言随之切换）", "語言（資料語言隨之切換）", "Language (game data follows)"],
  s_ui_auto: ["跟随系统", "跟隨系統", "Follow system"],
  s_zh_variant: ["简体译名", "簡體譯名", "Simplified Chinese naming"],
  s_zh_cn: ["国服译名", "國服譯名", "China server"],
  s_zh_my: ["国际服译名", "國際服譯名", "Global server (SEA)"],
  s_on_top: ["窗口置顶", "視窗置頂", "Always on top"],
  s_data: ["数据", "資料", "Data"],
  s_source: ["统计来源", "統計來源", "Stats source"],
  s_source_custom: ["自定义数据包", "自訂資料包", "Custom data pack"],
  s_pack: ["数据包（URL 或本地 JSON 路径）", "資料包（URL 或本機 JSON 路徑）", "Data pack (URL or local JSON path)"],
  s_cdragon: ["CommunityDragon 地址", "CommunityDragon 位址", "CommunityDragon URL"],
  s_league_dir: ["客户端目录（可选，自动识别失败时填写）", "用戶端目錄（選填，自動偵測失敗時填寫）", "Client folder (optional, if auto-detect fails)"],
  s_auto_update: ["自动更新数据（对局和选人时不更新）", "自動更新資料（對戰與選角時不更新）", "Update data automatically (not during games or champ select)"],
  s_checked: ["上次检查", "上次檢查", "Last checked"],
  ago_now: ["刚刚", "剛剛", "just now"],
  ago_min: ["{n} 分钟前", "{n} 分鐘前", "{n} min ago"],
  ago_hour: ["{n} 小时前", "{n} 小時前", "{n} h ago"],
  ago_day: ["{n} 天前", "{n} 天前", "{n} d ago"],
  sync_checking: ["正在检查数据更新…", "正在檢查資料更新…", "Checking for data updates…"],
  sync_static: ["正在下载游戏数据 {done}/{total}", "正在下載遊戲資料 {done}/{total}", "Downloading game data {done}/{total}"],
  sync_stats: ["正在更新统计数据…", "正在更新統計資料…", "Updating stats…"],
  s_refresh: ["立即检查更新", "立即檢查更新", "Check for updates now"],
  s_refreshing: ["更新中…", "更新中…", "Refreshing…"],
  s_refreshed: ["数据已更新", "資料已更新", "Data refreshed"],
  s_refresh_failed: ["更新失败：", "更新失敗：", "Refresh failed: "],
  s_saved: ["已自动保存", "已自動儲存", "Saved automatically"],
  s_save_failed: ["保存失败：", "儲存失敗：", "Save failed: "],
  s_client_lang: ["客户端语言", "用戶端語言", "Client language"],
  s_data_lang: ["数据语言", "資料語言", "Data language"],
  s_static: ["静态数据", "靜態資料", "Static data"],
  s_champs: ["英雄", "英雄", "champions"],
  s_augs: ["强化", "強化", "augments"],
  s_stats: ["统计", "統計", "Stats"],
  s_not_loaded: ["未加载", "未載入", "not loaded"],
  // 悬浮层 / 悬浮窗
  ov_best: ["首选", "首選", "Best"],
  ov_rank: ["本英雄第 {r}/{n}", "本英雄第 {r}/{n}", "#{r} of {n}"],
  ov_sample: ["示例数据", "示例資料", "Sample"],
  fl_bench: ["板凳", "板凳", "Bench"],
  fl_build: ["出装", "出裝", "Build"],
  fl_import: ["导入", "匯入", "Import"],
  fl_close: ["本阶段隐藏", "本階段隱藏", "Hide for now"],
  fl_more_tip: ["其余板凳英雄见主窗口「选人」页", "其餘板凳英雄見主視窗「選角」頁", "See the rest of the bench in the main window"],
  fl_swap_tip: ["点击交换", "點擊交換", "Click to swap"],
  s_float: ["选人时显示悬浮窗（板凳卡片下方）", "選角時顯示懸浮窗（板凳卡片下方）", "Show floating bar in champ select (below bench)"],
  s_overlay_y: ["强化标签位置（默认在卡片上方）", "強化標籤位置（預設在卡片上方）", "Tag position (above the cards by default)"],
  s_overlay_test: ["测试悬浮层", "測試懸浮層", "Test overlay"],
  s_overlay_test_tip: ["在游戏画面上显示 5 秒示例，用来对准卡片下沿", "在遊戲畫面上顯示 5 秒示例，用來對準卡片下緣", "Shows a 5-second sample on the game screen to line up with the cards"],
  dbg_no_game: ["未找到游戏窗口", "找不到遊戲視窗", "Game window not found"],
  dbg_no_cards: ["画面上没有强化卡片（持续检测中）", "畫面上沒有強化卡片（持續偵測中）", "No augment cards on screen (watching)"],
  dbg_background: ["游戏窗口不在前台", "遊戲視窗不在前景", "Game window not in foreground"],
  dbg_scanned: ["识别到 {lines} 行文字，匹配 {matched} 张卡片", "辨識到 {lines} 行文字，配對 {matched} 張卡片", "Read {lines} lines, matched {matched} cards"],
  dbg_last: ["最近一次识别", "最近一次辨識", "Last scan"],
  // 悬浮层
  ov_pick: ["选取", "選取", "Pick"],
} satisfies Record<string, [string, string, string]>;

export type Key = keyof typeof D;

export function t(key: Key, vars?: Record<string, string | number>): string {
  let s = D[key][IDX[lang()]];
  if (vars) for (const [k, v] of Object.entries(vars)) s = s.replace(`{${k}}`, String(v));
  return s;
}

/** 相对时间：刚刚 / n 分钟前 / n 小时前 / n 天前 */
export const ago = (unixSecs: number) => {
  if (!unixSecs) return "?";
  const s = Math.max(0, Date.now() / 1000 - unixSecs);
  if (s < 60) return t("ago_now");
  if (s < 3600) return t("ago_min", { n: Math.floor(s / 60) });
  if (s < 86400) return t("ago_hour", { n: Math.floor(s / 3600) });
  return t("ago_day", { n: Math.floor(s / 86400) });
};

/** 阶段名：未知阶段原样显示 */
export const phaseText = (p: string) => (`ph_${p}` in D ? t(`ph_${p}` as Key) : p);

/** 流派 / 强化风格名按界面语言取（数据只有繁体与英文，简体由后端转换） */
export const localName = (x: { name: string; nameHans?: string; nameEn?: string }) => {
  const l = lang();
  const pick = l === "zh-Hans" ? x.nameHans : l === "en" ? x.nameEn : x.name;
  return pick || x.name;
};
