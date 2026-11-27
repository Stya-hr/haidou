<p align="center">
  <img src="assets/banner.svg" alt="海克斯领航 · HexPilot ARAM Mayhem Companion" width="100%">
</p>

<p align="center">
  <b>简体中文</b> · <a href="README.en.md">English</a>
</p>

<p align="center">
  <a href="https://github.com/Stya-hr/hexpilot/actions/workflows/portable.yml"><img src="https://img.shields.io/github/actions/workflow/status/Stya-hr/hexpilot/portable.yml?label=build&logo=githubactions&logoColor=white" alt="Build"></a>
  <a href="https://github.com/Stya-hr/hexpilot/releases"><img src="https://img.shields.io/github/v/release/Stya-hr/hexpilot?include_prereleases&logo=github&label=release" alt="Release"></a>
  <a href="LICENSE"><img src="https://img.shields.io/github/license/Stya-hr/hexpilot?color=c8a14b" alt="License"></a>
  <img src="https://img.shields.io/badge/platform-Windows%2010%20%7C%2011-0078D6?logo=windows&logoColor=white" alt="Platform">
  <img src="https://img.shields.io/badge/Tauri-2-24C8DB?logo=tauri&logoColor=white" alt="Tauri 2">
  <img src="https://img.shields.io/badge/Rust-stable-000000?logo=rust&logoColor=white" alt="Rust">
  <img src="https://img.shields.io/badge/SolidJS-1.9-2C4F7C?logo=solid&logoColor=white" alt="SolidJS">
  <a href="https://arammeta.com/?ref=hexpilot"><img src="https://img.shields.io/badge/data-AramMeta-4fb3bf" alt="Data: AramMeta"></a>
</p>

<p align="center">
  英雄联盟「海克斯大乱斗」（ARAM Mayhem）助手：选人推荐、出装导入、强化 OCR 识别与游戏内档位悬浮层。<br>
  <b>轻量</b>（前端约 50 KB，Rust 后端）· <b>即时</b>（客户端事件推送）· <b>合规</b>（不读内存、不注入、不模拟键鼠）
</p>

---

## ✨ 功能

| | 功能 | 说明 |
|---|---|---|
| ✅ | **自动接受对局** | 可设延迟，默认关闭 |
| 🧭 | **英雄推荐** | 选人时板凳席按评分排序，高亮比你当前英雄更强的那个，一键交换或重随；空闲时显示英雄榜 |
| 📊 | **胜率 · 评级 · 选取率** | 英雄：S+/S/A/B/C/D 档位 + 胜率 + 选取率；强化：档位 + 选取率 |
| 🛠️ | **出装流派** | 如「AP 爆发」：核心装 → 常见后续，带档位、胜率、选取率；另有鞋子、可选装、强化风格 |
| 📥 | **一键导入出装** | 写入客户端「装备方案」，进游戏后商店里直接可用；每个流派一个分组；可在英雄确定后自动导入 |
| ✨ | **召唤师技能** | 一键设置或自动设置（默认关闭） |
| 🔍 | **强化识别（OCR）** | 开局、升级（3/7/11/15）和阵亡期间自动识别，也可以按热键 `Ctrl+Shift+A` 触发；**简中（国服、国际服两套译名）、繁中、英文**都能识别 |
| 🪟 | **游戏内悬浮层** | 在每张强化卡片下方显示档位和选取率，透明、置顶、鼠标穿透 |
| 📦 | **便携版** | zip 解压即用，所有数据都在程序旁边的 `hexpilot-data/` 里 |

> 海克斯大乱斗**禁用符文**（[Riot 官方说明](https://support.riotgames.com/en-us/league-of-legends/events/league-of-legends-aram-mayhem-game-mode)），所以不提供符文导入。

## 📦 下载与使用

1. 在 [Releases](https://github.com/Stya-hr/hexpilot/releases) 下载 `hexpilot-vX.Y.Z-win-x64.zip`，解压到任意**可写**目录（不要放在 Program Files）。
2. 运行 `hexpilot.exe`，启动英雄联盟客户端后会自动连接。
3. 游戏要用**无边框**或**窗口**模式，否则无法识别强化，悬浮层也盖不上去。
4. 强化识别依赖 Windows 自带的 OCR：设置 → 时间和语言 → 语言和区域 → 中文(简体)/中文(繁體) → 语言选项 → 安装「光学字符识别」。
5. 删除整个解压目录就是完全卸载。

需要 Windows 10/11 和 WebView2（系统一般已自带）。

## 🛡️ 合规与反作弊

只使用 Riot 允许第三方工具使用的方式，所以不存在"躲避反作弊"的问题，也没有任何规避检测的代码：

- **不读写游戏内存、不注入、不 Hook、不模拟键鼠。** 游戏进程完全不碰，游戏窗口只通过类名查找后截屏（与录屏软件相同的 GDI 方式）。
- 客户端连接信息来自客户端自己写出的 `lockfile`；游戏内只读 Riot 官方的 [Live Client Data API](https://developer.riotgames.com/docs/lol#game-client-api)（等级、是否阵亡）。
- 遵守 [Riot 开发者政策](https://developer.riotgames.com/docs/lol)：**不展示强化胜率**，源数据里的强化胜率只在本地用来换算档位，不会写入缓存，也不会显示。英雄胜率和出装流派胜率不受这条限制。
- 推荐只做高亮，选择都由玩家完成。出装导入只写本工具自己的方案，不动玩家自己的方案。

> ⚠️ LCU（客户端本地接口）不是 Riot 官方支持的接口，客户端更新后可能有变化；对局中实时识别并提示强化在政策上属于灰色地带，可以在设置里关闭 OCR。**使用风险自负。**

## 📚 数据来源

| 数据 | 来源 | 说明 |
|---|---|---|
| 英雄、强化、装备、技能 | [CommunityDragon](https://raw.communitydragon.org) | 按版本缓存；数据语言跟随客户端 |
| 胜率、档位、选取率、流派 | [AramMeta](https://arammeta.com/?ref=hexpilot) | 遵守其数据使用条款：显示数字的地方都有可点击的署名；总表每个版本拉取一次，英雄详情只在需要时拉取并缓存；不保存、不导出原始表 |
| 自定义 | 本地或在线 JSON | 设置 → 统计来源；格式见 [`samples/mayhem-stats.sample.json`](samples/mayhem-stats.sample.json) |

## 🔧 从源码构建

需要 Windows、Rust（MSVC 工具链）、Node 20+、pnpm：

```bash
pnpm install
pnpm tauri dev      # 开发
pnpm portable       # 生成 dist-portable/hexpilot-vX.Y.Z-win-x64.zip
```

- 测试：`cd src-tauri && cargo test`
- 在 Linux 上对 Windows 目标做类型检查：`cargo check --target x86_64-pc-windows-msvc`
- CI：[`.github/workflows/portable.yml`](.github/workflows/portable.yml)，可以手动触发；推送 `v*` 标签会自动发布 Release

<details>
<summary><b>代码结构</b></summary>

```
src-tauri/src/
  lcu/          客户端发现、REST、WebSocket（只订阅 3 个事件）、选人 / 接受对局 / 自动导入 / 语言检测
  arammeta.rs   AramMeta 适配器：档位换算、流派、缓存
  data.rs       CommunityDragon 同步、多语言强化名、按需加载英雄详情
  apply.rs      装备方案（按流派分组）/ 召唤师技能写入
  live.rs       游戏内等级与阵亡状态（触发 OCR）
  ocr/          扫描调度、简繁英模糊匹配、OCR 引擎轮换、悬浮层；win.rs = GDI 截屏 + Windows.Media.Ocr
  paths.rs      便携版数据目录
src/            前端（App.tsx 主窗口，Overlay.tsx 悬浮层）
scripts/        打 zip 包
```

</details>

## 🙏 致谢

- [AramMeta](https://arammeta.com/?ref=hexpilot)：海克斯大乱斗统计数据
- [CommunityDragon](https://www.communitydragon.org/)：游戏静态数据与图标
- [Tauri](https://tauri.app/) · [SolidJS](https://www.solidjs.com/)

## 📄 许可证与声明

代码以 [MIT](LICENSE) 许可证开源。第三方数据（AramMeta、CommunityDragon）受各自条款约束，不在 MIT 授权范围内。

海克斯领航未获 Riot Games 认可，不代表 Riot Games 或任何官方参与制作、管理 Riot Games 产品人员的观点和意见。Riot Games 及所有相关财产均为 Riot Games, Inc. 的商标或注册商标。本项目与 AramMeta 无隶属关系。
