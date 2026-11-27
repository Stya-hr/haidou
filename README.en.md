<p align="center">
  <img src="assets/banner.png" alt="HexPilot · ARAM Mayhem Companion" width="100%">
</p>

<p align="center">
  <a href="README.md">简体中文</a> · <b>English</b>
</p>

<p align="center">
  <a href="https://github.com/Stya-hr/hexpilot/actions/workflows/portable.yml"><img src="https://github.com/Stya-hr/hexpilot/actions/workflows/portable.yml/badge.svg" alt="Build"></a>
  <a href="https://github.com/Stya-hr/hexpilot/releases"><img src="https://img.shields.io/github/v/release/Stya-hr/hexpilot?include_prereleases&label=release&color=c8a14b" alt="Release"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-c8a14b" alt="License: MIT"></a>
</p>
<p align="center">
  <img src="https://img.shields.io/badge/platform-Windows%2010%20%7C%2011-0078D6" alt="Platform: Windows 10 | 11">
  <img src="https://img.shields.io/badge/Tauri-2-24C8DB?logo=tauri&logoColor=white" alt="Tauri 2">
  <img src="https://img.shields.io/badge/Rust-stable-000000?logo=rust&logoColor=white" alt="Rust">
  <img src="https://img.shields.io/badge/SolidJS-1.9-2C4F7C?logo=solid&logoColor=white" alt="SolidJS">
</p>
<p align="center">
  <a href="https://arammeta.com/?ref=hexpilot"><img src="https://img.shields.io/badge/data-AramMeta-4fb3bf" alt="Data: AramMeta"></a>
  <a href="https://www.communitydragon.org/"><img src="https://img.shields.io/badge/assets-CommunityDragon-6e7681" alt="Assets: CommunityDragon"></a>
</p>

<p align="center">
  League of Legends <b>ARAM: Mayhem</b> companion: champ-select advice, item import, augment OCR<br>
  <b>Lightweight</b> · <b>Instant</b> · <b>Policy-friendly</b>
</p>

---

## Features

- **Champ select**: bench sorted by rating with the stronger pick highlighted, one-click swap / reroll; auto-accept
- **Stats**: champion tier, win rate and pick rate; augment tier and pick rate; build archetypes with win/pick rates
- **Import**: item sets that show up in the in-game shop; summoner spells set automatically
- **In game**: augment OCR (EN / 简 / 繁) with a tier overlay under each card
- **Also**: three-language UI, auto-updating data, portable zip

## Usage

1. Download the zip from [Releases](https://github.com/Stya-hr/hexpilot/releases), extract it and run `hexpilot.exe`
2. Use **borderless** mode in game (one click in the app) so augment recognition works
3. Install the Windows *Optical character recognition* language component (Settings → Language → Language options)

## Fair play

- No game-memory access, no injection, no input automation; only the client's local API, Riot's in-game API and screen capture
- Follows Riot's developer policy: no augment win rates, recommendations are highlights only
- The LCU is not officially supported by Riot; use at your own risk

## Build

```bash
pnpm install
pnpm tauri dev    # development
pnpm portable     # portable zip
```

See the [CHANGELOG](CHANGELOG.md) for release notes.

## Credits & legal

Data from [AramMeta](https://arammeta.com/?ref=hexpilot) and [CommunityDragon](https://www.communitydragon.org/); built with [Tauri](https://tauri.app/) and [SolidJS](https://www.solidjs.com/). Code under [MIT](LICENSE); third-party data is governed by its own terms.

HexPilot isn't endorsed by Riot Games and doesn't reflect the views or opinions of Riot Games or anyone officially involved in producing or managing Riot Games properties. Riot Games, and all associated properties are trademarks or registered trademarks of Riot Games, Inc. This project is not affiliated with AramMeta.
