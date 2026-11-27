// 把 release 版 exe 打成 zip 便携包：dist-portable/hexpilot-v<版本>-win-x64-portable.zip
// 包内结构：hexpilot/hexpilot.exe + hexpilot/使用说明.txt；首次运行会在旁边生成 hexpilot-data/（设置与缓存）
import { execFileSync } from "node:child_process";
import { appendFileSync, copyFileSync, existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

const root = fileURLToPath(new URL("..", import.meta.url));
const conf = JSON.parse(readFileSync(join(root, "src-tauri", "tauri.conf.json"), "utf8"));
const exe = join(root, "src-tauri", "target", "release", "hexpilot.exe");
if (!existsSync(exe)) {
  console.error(`找不到 ${exe}，请先运行 pnpm tauri build --no-bundle`);
  process.exit(1);
}

const out = join(root, "dist-portable");
const stage = join(out, "hexpilot");
rmSync(out, { recursive: true, force: true });
mkdirSync(stage, { recursive: true });
copyFileSync(exe, join(stage, "hexpilot.exe"));
writeFileSync(join(stage, "使用说明.txt"), `海克斯领航 v${conf.version}（便携版）

1. 解压到任意可写目录（不要放在 Program Files），双击 hexpilot.exe。
2. 设置和缓存保存在同目录的 hexpilot-data 文件夹，删除整个文件夹即完全卸载。
3. 游戏请使用「无边框」或「窗口」模式，否则无法识别强化、悬浮层也不显示。
4. 需要 Windows 10/11 自带的 WebView2；极少数精简系统缺少时请从微软官网安装 WebView2 Runtime。
5. 强化识别使用系统 OCR：设置 → 时间和语言 → 语言和区域 → 中文(简体/繁体) → 语言选项 → 安装「光学字符识别」。

统计数据：Data: AramMeta (https://arammeta.com/?ref=hexpilot)，与 Riot Games 及 AramMeta 均无隶属关系。
`, "utf8");

// 可用 PACK_NAME 覆盖包名（nightly 用）
const name = process.env.PACK_NAME || `hexpilot-v${conf.version}-win-x64-portable`;
const zip = join(out, `${name}.zip`);
// 在 GitHub Actions 里把包名传给后续步骤（Artifact 名称）
if (process.env.GITHUB_OUTPUT) appendFileSync(process.env.GITHUB_OUTPUT, `name=${name}\n`);
execFileSync("powershell", ["-NoProfile", "-Command",
  `Compress-Archive -Path '${stage}' -DestinationPath '${zip}' -Force`], { stdio: "inherit" });
console.log(`已生成 ${zip}`);
