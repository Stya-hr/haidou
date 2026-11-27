# 海斗助手 · 海克斯大乱斗（ARAM Mayhem）助手

适用于国际服（Riot Client + Vanguard）。Tauri 2（Rust）+ SolidJS，前端产物约 50 KB，以 zip 便携版分发。

## 功能

| 功能 | 说明 | 实现方式 |
|---|---|---|
| 自动接受对局 | 可设延迟，默认关闭 | LCU |
| 英雄推荐 | 选人时板凳席按评分排序、高亮更强的英雄、一键交换 / 重随；空闲时显示英雄榜 | LCU 选人事件 + 统计 |
| 胜率 · 评级 · 选取率 | 英雄：档位（S+/S/A/B/C/D）+ 胜率 + 选取率；强化：档位 + 选取率 | arammeta |
| 出装流派 | 如「AP 爆发」：核心装 → 常见后续，带档位、胜率、选取率；另有鞋子、可选装、强化风格 | arammeta |
| 出装导入 | 写入客户端「装备方案」，**游戏内商店直接可用**；每个流派一个分组，标题带档位和胜率。可在英雄确定后自动导入（仅限队列 2400） | LCU `/lol-item-sets` |
| 召唤师技能 | 一键设置或自动设置（默认关闭） | LCU |
| 强化识别 | 开局、升级（3/7/11/15）和阵亡期间自动识别卡片，也可以用热键 `Ctrl+Shift+A` 触发；**简中（国服、国际服两套译名）、繁中、英文都能识别** | 截屏 + Windows OCR |
| 游戏内悬浮层 | 在每张卡片下方显示档位和选取率（透明、置顶、鼠标穿透） | 独立窗口 |

海克斯大乱斗**禁用符文**（[Riot 官方说明](https://support.riotgames.com/en-us/league-of-legends/events/league-of-legends-aram-mayhem-game-mode)），所以不提供符文导入。强化套装（Set）已在 26.12 版本随 "Augment Traits" 一起移除，也不再显示。

## 合规与反作弊

只使用 Riot 允许第三方工具使用的方式，不存在"躲避反作弊"的问题，也没有任何规避检测的代码：

- **不读写游戏内存、不注入、不 Hook、不模拟键鼠**。游戏进程完全不碰，游戏窗口只通过类名查找后截屏。
- 客户端凭据来自客户端自己写出的 `lockfile`（兜底方式是通过系统 WMI 查询启动参数）。游戏内只读 Riot 官方的 Live Client Data API（等级、是否阵亡）。
- 遵守 [Riot 开发者政策](https://developer.riotgames.com/docs/lol)：**不展示强化胜率**，源数据里的强化胜率只在本地用来换算档位，不会写入缓存，也不会显示。英雄胜率和装备流派胜率不受这条限制。
- 推荐只做高亮，交换和选择都由玩家点击完成。出装导入只写本工具自己的方案（uid 以 `haidou-` 开头），不动玩家自己的方案。

> 风险提示：LCU 不是 Riot 官方支持的接口，客户端更新后可能有变化；"对局中实时识别并提示强化"在政策上属于灰色地带，可以在设置里关闭 OCR。

## 数据来源

- **统计**：[AramMeta](https://arammeta.com/?ref=haidou)（Data: AramMeta）。按其数据使用条款：
  - 凡是显示其数字的地方都有可点击的署名，链接带 `?ref=haidou`。
  - 访问保持轻量：总表每个版本拉取一次，同一版本内最多 3 天刷新一次；英雄详情只在需要时拉取（被分到该英雄或手动查看时），并按数据版本缓存，同一版本不会重复请求。
  - 本地只缓存换算后的结果，不保存、不导出原始表。
  - 本项目与 AramMeta、Riot Games 均无隶属关系。
- **静态数据**：[CommunityDragon](https://raw.communitydragon.org)，包括英雄、强化、装备、召唤师技能。数据语言默认跟随客户端，也可以在设置里指定。
- 也支持**自定义数据包**（设置 → 统计来源），格式见 `samples/mayhem-stats.sample.json`（示例，不是真实统计）。

## 使用要求

- Windows 10/11，需要 WebView2，系统一般已自带。
- 游戏要用**无边框**或**窗口**模式，独占全屏下截图是黑屏，悬浮层也盖不上去。
- OCR 语言组件：设置 → 时间和语言 → 语言和区域 → 中文(简体)/中文(繁體) → 语言选项 → 安装「光学字符识别」。装了其中一个就能用，两个都装识别效果最好。

## 构建 zip 便携版

在 **Windows** 上执行，需要 Rust（MSVC 工具链）、Node 20+ 和 pnpm：

```bash
pnpm install
pnpm portable     # 生成 dist-portable/haidou-v0.1.0-win-x64.zip
```

压缩包里是 `haidou/haidou.exe` 和 `haidou/使用说明.txt`。解压到任意可写目录即可运行。设置、缓存和 WebView 数据都保存在同目录的 `haidou-data/` 里，删掉整个文件夹就是完全卸载。

也可以用 GitHub Actions 构建：`.github/workflows/portable.yml` 支持手动触发或推送 `v*` 标签，构建产物就是这个 zip。

开发：`pnpm tauri dev`。测试：`cd src-tauri && cargo test`。在 Linux 上可以对 Windows 目标做类型检查：`cargo check --target x86_64-pc-windows-msvc`。

## 代码结构

```
src-tauri/src/
  lcu/          客户端发现（auth）、REST（http）、WebSocket（ws）、选人/接受对局/自动导入/语言检测（mod）
  arammeta.rs   AramMeta 适配器：档位换算、流派、缓存
  data.rs       CommunityDragon 同步、多语言强化名、按需加载英雄详情
  apply.rs      装备方案（按流派分组）/ 召唤师技能写入
  live.rs       游戏内等级与阵亡状态（触发 OCR）
  ocr/          扫描调度、简繁英模糊匹配、OCR 引擎轮换、悬浮层定位；win.rs = GDI 截屏 + Windows.Media.Ocr
  paths.rs      便携版数据目录
src/            前端（App.tsx 主窗口，Overlay.tsx 悬浮层）
scripts/pack-portable.mjs   打 zip 包
```
