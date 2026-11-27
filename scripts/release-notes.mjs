// 从 CHANGELOG.md 取出指定版本的段落，作为 Release 说明：node scripts/release-notes.mjs v0.1.0
import { readFileSync } from "node:fs";

const tag = process.argv[2];
const text = readFileSync(new URL("../CHANGELOG.md", import.meta.url), "utf8");
const lines = text.split(/\r?\n/);
const start = lines.findIndex((l) => l.startsWith(`## ${tag} `) || l.trim() === `## ${tag}`);
if (start < 0) {
  console.error(`CHANGELOG.md 中没有 ${tag}`);
  process.exit(1);
}
let end = lines.findIndex((l, i) => i > start && l.startsWith("## "));
if (end < 0) end = lines.length;
const body = lines.slice(start + 1, end).join("\n").trim();
const repo = process.env.GITHUB_REPOSITORY ?? "Stya-hr/hexpilot";
console.log(`${body}\n\n---\n\n**下载 · Download**：\`hexpilot-${tag}-win-x64-portable.zip\`，解压后运行 \`hexpilot.exe\` · unzip and run \`hexpilot.exe\`\n\n**Full Changelog**: https://github.com/${repo}/commits/${tag}`);
