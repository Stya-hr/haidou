import { invoke } from "@tauri-apps/api/core";

export interface TeamMember { cellId: number; championId: number; isMe: boolean }
export interface ChampSelectView {
  myChampionId: number;
  benchEnabled: boolean;
  bench: number[];
  team: TeamMember[];
  rerollsRemaining: number;
}
export interface OfferCard { augmentId: number; x: number; y: number; confidence: number }
export interface Snapshot {
  connected: boolean;
  phase: string;
  queueId: number | null;
  champSelect: ChampSelectView | null;
  lastChampionId: number;
  liveLevel: number | null;
  liveDead: boolean;
  windowMode: number | null;
  clientLocale: string | null;
  ocrLang: string | null;
  offer: OfferCard[];
  ocrError: string | null;
  sync: { stage: "checking" | "static" | "stats"; done: number; total: number } | null;
}

export interface Champion { id: number; name: string; title: string; alias: string; icon: string }
export interface Augment {
  id: number; name: string; altNames: string[]; rarity: string; icon: string; tags: string[]; mayhem: boolean;
}
export interface ChampAug { id: number; tier: string; pickRate: number | null }
export interface ItemBuild { start: number[]; boots: number[]; core: number[]; situational: number[] }
export interface Archetype {
  name: string; nameHans: string; nameEn: string; tier: string; core: number[]; options: number[];
  winRate: number | null; pickRate: number | null; games: number | null;
}
export interface AugStyle { name: string; nameHans: string; nameEn: string; pickRate: number | null }
export interface ChampStats {
  tier: string; score: number | null; winRate: number | null; pickRate: number | null; games: number | null;
  augments: ChampAug[]; spells: number[]; items: ItemBuild | null;
  archetypes: Archetype[]; augStyles: AugStyle[]; note: string; detailed: boolean;
}
export interface Thing { id: number; name: string; icon: string }
export interface AugStats { tier: string; pickRate: number | null }
export interface Attribution { label: string; url: string; championUrl: string }
export interface StatsPack {
  schema: number; patch: string; source: string; updatedAt: string;
  attribution: Attribution | null;
  champions: Record<string, ChampStats>;
  augments: Record<string, AugStats>;
}
export interface Catalog {
  patch: string; locale: string;
  champions: Champion[]; augments: Augment[];
  items: Thing[]; spells: Thing[];
  stats: StatsPack | null; statsError: string | null;
  checkedAt: number;
}
export interface Settings {
  autoAccept: boolean; acceptDelayMs: number; alwaysOnTop: boolean;
  leagueDir: string; statsSource: string; statsUrl: string; cdragonBase: string; uiLang: string;
  autoItems: boolean; autoSpells: boolean;
  ocrEnabled: boolean; overlayEnabled: boolean; ocrLang: string; ocrHotkey: string;
  autoUpdate: boolean;
}

export const api = {
  snapshot: () => invoke<Snapshot>("get_snapshot"),
  catalog: () => invoke<Catalog | null>("get_catalog"),
  settings: () => invoke<Settings>("get_settings"),
  uiLang: () => invoke<string>("get_ui_lang"),
  saveSettings: (settings: Settings) => invoke<void>("save_settings", { settings }),
  refreshData: () => invoke<void>("refresh_data"),
  benchSwap: (championId: number) => invoke<void>("bench_swap", { championId }),
  reroll: () => invoke<void>("reroll"),
  applyBuild: (championId: number, what: "items" | "spells" | "all") =>
    invoke<string>("apply_build", { championId, what }),
  ensureChampion: (championId: number) => invoke<void>("ensure_champion", { championId }),
  openLink: (url: string) => invoke<void>("open_link", { url }),
  ocrScanNow: () => invoke<void>("ocr_scan_now"),
  setBorderless: () => invoke<void>("set_borderless"),
};

export const MAYHEM_QUEUE = 2400;

const TIERS = ["S+", "S", "A", "B", "C", "D"];
/** 数值越大越强；未知档位为 0 */
export const tierRank = (t?: string) => {
  const i = TIERS.indexOf((t ?? "").toUpperCase());
  return i < 0 ? 0 : TIERS.length - i;
};

/** 用于排序的英雄分：优先 score，否则按档位折算 */
export const champScore = (s?: ChampStats) =>
  s ? (s.score ?? tierRank(s.tier) * 10 + 20) : -1;

/** 某英雄视角下的强化档位与选取率：英雄专属数据优先，其次全局 */
export const augInfo = (cat: Catalog | null, champId: number, augId: number) => {
  const mine = cat?.stats?.champions[String(champId)]?.augments.find((a) => a.id === augId);
  const global = cat?.stats?.augments[String(augId)];
  return {
    tier: mine?.tier ?? global?.tier,
    pickRate: mine?.pickRate ?? global?.pickRate ?? null,
    scope: mine ? "champ" : global ? "global" : "none",
  } as const;
};

/** 强化池：统计源提供时以其为准，否则用 ARAM_/Kiwi 规则 */
export const inPool = (cat: Catalog | null, a: Augment) => {
  const pool = cat?.stats?.augments;
  return pool && Object.keys(pool).length ? String(a.id) in pool : a.mayhem;
};

export const pct = (v: number | null | undefined) =>
  v == null ? "" : `${(v * 100).toFixed(v < 0.01 ? 2 : 1)}%`;

export const championUrl = (cat: Catalog | null, c?: Champion) => {
  const t = cat?.stats?.attribution?.championUrl;
  return t && c ? t.replace("{alias}", c.alias.toLowerCase().replace(/[^a-z0-9]/g, "")) : undefined;
};
