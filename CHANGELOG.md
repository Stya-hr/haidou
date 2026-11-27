# 更新日志 · Changelog

## v0.2.0 · 2026-11-28

### 功能

- **强化卡片悬浮条**：在三张强化卡片正下方显示一条面板，每张卡一格：档位、强化名、选取率、本英雄排名，最佳一张标「首选」；卡片界面期间持续显示
- **悬浮条位置调节**：设置中可调垂直位置，并可「测试悬浮层」显示示例用于对准卡片
- **选人悬浮窗**：吸附在客户端板凳卡片下方，随客户端移动；显示我的英雄档位与胜率、板凳英雄（点击交换）、主流出装与一键导入、召唤师技能；可拖动并记住位置
- **更快响应**：升级、阵亡后更快开始识别，扫描频率提高，悬浮层不再闪烁
- **识别诊断**：强化页显示最近一次识别结果，便于排查

### Features

- **Augment overlay bar**: a panel right under the three augment cards, one cell per card with tier, name, pick rate and rank on your champion; the best one is marked; stays visible while the cards are on screen
- **Overlay position**: adjustable vertical position in Settings, plus a test button that shows a sample to line it up
- **Champ select bar**: docked below the bench in the client and follows it; shows your champion's tier and win rate, bench champions (click to swap), the main build with one-click import, and summoner spells; draggable with remembered position
- **Faster response**: scanning starts sooner after level-up or death, scans more often, and the overlay no longer flickers
- **Scan diagnostics**: the Augments tab shows the result of the last scan for troubleshooting

## v0.1.0 · 2026-11-27

### 功能

- **选人推荐**：板凳席按评分排序并高亮更强的英雄，一键交换 / 重随；自动接受对局（可设延迟）
- **数据**：英雄档位（S+ ~ D）、胜率、选取率；强化档位与选取率；出装流派（核心装、后续选择、胜率、选取率）
- **出装导入**：写入客户端装备方案，游戏内商店直接可用，每个流派一组；英雄确定后可自动导入
- **召唤师技能**：一键或自动设置
- **强化识别**：开局、升级与阵亡时自动 OCR 识别海克斯强化，支持简体（国服 / 国际服译名）、繁体、英文；自动检测已安装的 OCR 引擎
- **游戏内悬浮层**：在强化卡片下方显示档位与选取率，鼠标穿透
- **显示模式**：检测全屏并一键改为无边框
- **其他**：三语界面（简体 / 繁體 / English）、数据自动更新、设置自动保存、zip 便携版

### Features

- **Champ select**: bench sorted by rating with the stronger pick highlighted, one-click swap / reroll; auto-accept with delay
- **Stats**: champion tier (S+ ~ D), win rate and pick rate; augment tier and pick rate; build archetypes with win/pick rates
- **Item import**: client item sets usable in the in-game shop, one block per archetype; optional auto-import
- **Summoner spells**: one click or automatic
- **Augment OCR**: scans at game start, on level-up and while dead; Simplified (CN / global naming), Traditional Chinese and English; detects installed OCR engines
- **In-game overlay**: tier and pick rate under each augment card, click-through
- **Display mode**: detects fullscreen and switches to borderless in one click
- **Also**: three-language UI, auto-updating data, auto-saved settings, portable zip
