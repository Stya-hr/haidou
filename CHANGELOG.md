# 更新日志 · Changelog

## v0.1.0 · 2026-11-27

首个版本。First release.

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

### 使用要求 · Requirements

- Windows 10 / 11（WebView2 一般已自带 · WebView2 usually preinstalled）
- 游戏使用无边框或窗口模式 · Borderless or windowed mode in game
- 安装中文或英文「光学字符识别」组件 · Windows *Optical character recognition* language component

### 已知限制 · Known limitations

- 尚未在真实对局中完整测试；OCR 截图区域与匹配阈值可能需要调整
- 显示模式取值（0 全屏 / 1 窗口 / 2 无边框）来自社区资料，修改后会读回校验
- Not yet fully tested in live games; OCR capture region and thresholds may need tuning
- Display-mode values (0 fullscreen / 1 windowed / 2 borderless) come from community docs and are verified by read-back

数据来自 [AramMeta](https://arammeta.com/?ref=hexpilot) 与 [CommunityDragon](https://www.communitydragon.org/)。不展示强化胜率（Riot 开发者政策）。
Data from [AramMeta](https://arammeta.com/?ref=hexpilot) and [CommunityDragon](https://www.communitydragon.org/). Augment win rates are not shown (Riot developer policy).
