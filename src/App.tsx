import { listen } from "@tauri-apps/api/event";
import { createEffect, createMemo, createSignal, For, JSX, on, onCleanup, onMount, Show } from "solid-js";
import {
  api, Archetype, augInfo, Augment, Catalog, Champion, champScore, ChampStats, championUrl, inPool,
  MAYHEM_QUEUE, pct, Settings, Snapshot, Thing, tierRank,
} from "./api";
import { ago, Key, lang, localName, phaseText, setLang, t } from "./i18n";

type Tab = "select" | "augments" | "settings";

const EMPTY: Snapshot = {
  connected: false, phase: "None", queueId: null, champSelect: null, lastChampionId: 0,
  liveLevel: null, liveDead: false, windowMode: null, clientLocale: null, ocrLang: null, ocrInstalled: [], offer: [], ocrError: null, sync: null, ocrDebug: null,
};

const rarityText = (r: string) => t((["silver", "gold", "prismatic"].includes(r) ? r : "other") as Key);

export default function App() {
  const [snap, setSnap] = createSignal<Snapshot>(EMPTY);
  const [cat, setCat] = createSignal<Catalog | null>(null);
  const [tab, setTab] = createSignal<Tab>((new URLSearchParams(location.search).get("tab") as Tab) || "select");
  const [toast, setToast] = createSignal("");
  let toastTimer: ReturnType<typeof setTimeout> | undefined;

  const notify = (msg: string) => {
    setToast(msg);
    clearTimeout(toastTimer);
    toastTimer = setTimeout(() => setToast(""), 3500);
  };

  onMount(async () => {
    const un1 = await listen<Snapshot>("snapshot", (e) => setSnap(e.payload));
    const un2 = await listen("catalog-updated", async () => setCat(await api.catalog()));
    const un3 = await listen<string>("toast", (e) => notify(e.payload));
    const un4 = await listen<string>("ui-lang", (e) => setLang(e.payload));
    onCleanup(() => { un1(); un2(); un3(); un4(); });
    setLang(await api.uiLang());
    setSnap(await api.snapshot());
    setCat(await api.catalog());
  });

  createEffect(() => { document.documentElement.lang = lang(); });

  const ctx = createMemo<Ctx>(() => {
    const c = cat();
    return {
      cat: c,
      champs: new Map((c?.champions ?? []).map((x) => [x.id, x])),
      augs: new Map((c?.augments ?? []).map((x) => [x.id, x])),
      items: new Map((c?.items ?? []).map((x) => [x.id, x])),
      spells: new Map((c?.spells ?? []).map((x) => [x.id, x])),
      stat: (id: number) => c?.stats?.champions[String(id)],
      notify,
    };
  });

  return (
    <div class="app">
      <header>
        <div class="brand"><img src="/icon.png" alt="" /><span>HexPilot</span></div>
        <div class="status">
          <span class="pill"><span class={`dot ${snap().connected ? "on" : ""}`} />{snap().connected ? phaseText(snap().phase) : t("waiting")}</span>
          <Show when={snap().queueId}>
            <span class="pill" classList={{ mayhem: snap().queueId === MAYHEM_QUEUE }}>
              {snap().queueId === MAYHEM_QUEUE ? t("mayhem") : t("queue", { q: snap().queueId! })}
            </span>
          </Show>
          <Show when={snap().liveLevel}>
            <span class="pill lvl" classList={{ dead: snap().liveDead }}>Lv.{snap().liveLevel}{snap().liveDead ? ` · ${t("dead")}` : ""}</span>
          </Show>
          <Show when={!snap().queueId && cat()?.patch}><span class="pill">v{cat()!.patch}</span></Show>
        </div>
        <SyncLine sync={snap().sync} />
      </header>
      <Show when={snap().sync}><div class="synctext">{syncLabel(snap().sync!)}</div></Show>

      <nav>
        <button classList={{ active: tab() === "select" }} onClick={() => setTab("select")}>{t("tab_select")}</button>
        <button classList={{ active: tab() === "augments" }} onClick={() => setTab("augments")}>{t("tab_augments")}</button>
        <button classList={{ active: tab() === "settings" }} onClick={() => setTab("settings")}>{t("tab_settings")}</button>
      </nav>

      <main>
        <Show when={tab() === "select"}><SelectView snap={snap()} x={ctx()} /></Show>
        <Show when={tab() === "augments"}><AugmentView snap={snap()} x={ctx()} /></Show>
        <Show when={tab() === "settings"}><SettingsView snap={snap()} cat={cat()} notify={notify} /></Show>
      </main>

      <Show when={toast()}><div class="toast">{toast()}</div></Show>
    </div>
  );
}

const syncLabel = (s: NonNullable<Snapshot["sync"]>) =>
  s.stage === "static" ? t("sync_static", { done: s.done, total: s.total })
    : s.stage === "stats" ? t("sync_stats") : t("sync_checking");

/** 数据同步进度：顶栏底部细线（检查 → 下载游戏数据 n/m → 统计数据），结束后消失 */
function SyncLine(p: { sync: Snapshot["sync"] }) {
  const width = () => {
    const s = p.sync!;
    if (s.stage === "checking") return 12;
    if (s.stage === "static") return 15 + (70 * s.done) / Math.max(1, s.total);
    return 92;
  };
  return (
    <Show when={p.sync}>
      <div class="syncline" classList={{ indeterminate: p.sync!.stage !== "static" }}>
        <div class="fill" style={{ width: `${width()}%` }} />
      </div>
    </Show>
  );
}

interface Ctx {
  cat: Catalog | null;
  champs: Map<number, Champion>;
  augs: Map<number, Augment>;
  items: Map<number, Thing>;
  spells: Map<number, Thing>;
  stat: (id: number) => ChampStats | undefined;
  notify: (m: string) => void;
}

/** 数据来源署名：凡是展示统计数字的地方都显示，可点击 */
function Source(p: { cat: Catalog | null; url?: string }) {
  const a = () => p.cat?.stats?.attribution;
  return (
    <Show when={a()}>
      <a class="source" href="#" onClick={(e) => { e.preventDefault(); api.openLink(p.url ?? a()!.url).catch(() => {}); }}>
        {a()!.label}
      </a>
    </Show>
  );
}

/** 显示模式提醒：全屏时无法识别强化，提供一键改为无边框 */
function WindowModeHint(p: { snap: Snapshot; notify: (m: string) => void; always?: boolean }) {
  const [busy, setBusy] = createSignal(false);
  const fix = async () => {
    setBusy(true);
    try { await api.setBorderless(); p.notify(t("wm_fixed")); } catch (e) { p.notify(String(e)); }
    setBusy(false);
  };
  const mode = () => p.snap.windowMode;
  const modeText = () => {
    const m = mode();
    return m == null ? t("wm_unknown") : m >= 0 && m <= 2 ? t(`wm_${m}` as Key) : String(m);
  };
  const canFix = () => p.snap.connected && p.snap.phase !== "InProgress" && mode() !== 2;
  return (
    <Show when={p.always || mode() === 0}>
      <div class="hint" classList={{ warn: mode() === 0 }}>
        <span>{t("wm_label")}<b>{modeText()}</b></span>
        <Show when={canFix()}>
          <button class="small right" disabled={busy()} onClick={fix}>{t("wm_fix")}</button>
        </Show>
        <Show when={mode() === 0}><span class="hint-line">{t("wm_fullscreen_warn")}</span></Show>
        <Show when={p.snap.phase === "InProgress" && mode() !== 2}>
          <span class="hint-line muted">{t("wm_in_game")}</span>
        </Show>
      </div>
    </Show>
  );
}

function Tier(p: { t?: string }) {
  return <Show when={p.t}><span class={`tier t${tierRank(p.t)}`}>{p.t}</span></Show>;
}

function Stat(p: { label: string; v: number | null | undefined }) {
  return <Show when={p.v != null}><span class="stat">{p.label} <b>{pct(p.v)}</b></span></Show>;
}

function Icons(p: { ids: number[]; from: Map<number, Thing>; size?: number }) {
  return (
    <span class="icons">
      <For each={p.ids}>
        {(id) => {
          const it = p.from.get(id);
          return <img src={it?.icon} title={it?.name ?? String(id)} alt={it?.name ?? ""}
            style={{ width: `${p.size ?? 24}px`, height: `${p.size ?? 24}px` }} loading="lazy" />;
        }}
      </For>
    </span>
  );
}

function ChampChip(p: { c?: Champion; s?: ChampStats; best?: boolean; me?: boolean; right?: JSX.Element }) {
  return (
    <div class="chip" classList={{ best: p.best, me: p.me }}>
      <img src={p.c?.icon} alt="" loading="lazy" />
      <div class="meta">
        <div class="name">{p.c ? p.c.name : t("not_selected")} <span class="muted">{p.c?.title}</span></div>
        <div class="sub">
          <Show when={p.s} fallback={<span class="muted">{t("no_data")}</span>}>
            <Tier t={p.s!.tier} />
            <Stat label={t("win_rate")} v={p.s!.winRate} />
            <Stat label={t("pick_rate")} v={p.s!.pickRate} />
          </Show>
        </div>
      </div>
      {p.right}
    </div>
  );
}

/** 出装流派、强化风格、召唤师技能，以及一键导入 */
function BuildPanel(p: { x: Ctx; champId: number; inChampSelect: boolean }) {
  const st = () => p.x.stat(p.champId);
  createEffect(on(() => p.champId, (id) => { if (id > 0) api.ensureChampion(id).catch(() => {}); }));
  const apply = async (what: "items" | "spells") => {
    try { p.x.notify(await api.applyBuild(p.champId, what)); } catch (e) { p.x.notify(String(e)); }
  };
  const ib = () => st()?.items;

  return (
    <Show when={st()}>
      <Show when={st()!.detailed} fallback={<div class="muted">{t("loading_build")}</div>}>
        <div class="build">
          <Show when={st()!.archetypes.length}>
            <div class="bhead">{t("archetypes")}
              <button class="small right" onClick={() => apply("items")} title={t("import_items_tip")}>{t("import_items")}</button>
            </div>
            <For each={st()!.archetypes}>
              {(a: Archetype) => (
                <div class="arch">
                  <div class="arow">
                    <Tier t={a.tier} />
                    <span class="aname">{localName(a)}</span>
                    <Stat label={t("win_rate")} v={a.winRate} />
                    <Stat label={t("pick_rate")} v={a.pickRate} />
                  </div>
                  <div class="arow">
                    <Icons ids={a.core} from={p.x.items} size={26} />
                    <span class="arrow">→</span>
                    <Icons ids={a.options} from={p.x.items} size={22} />
                  </div>
                </div>
              )}
            </For>
          </Show>
          <Show when={ib()?.boots.length}>
            <div class="brow"><span class="blabel">{t("boots")}</span><Icons ids={ib()!.boots} from={p.x.items} size={22} /></div>
          </Show>
          <Show when={ib()?.situational.length}>
            <div class="brow"><span class="blabel">{t("situational")}</span><Icons ids={ib()!.situational} from={p.x.items} size={22} /></div>
          </Show>
          <Show when={st()!.spells.length}>
            <div class="brow"><span class="blabel">{t("spells")}</span>
              <Icons ids={st()!.spells} from={p.x.spells} />
              <Show when={p.inChampSelect}><button class="small right" onClick={() => apply("spells")}>{t("set_spells")}</button></Show>
            </div>
          </Show>
          <Show when={st()!.augStyles.length}>
            <div class="brow"><span class="blabel">{t("aug_styles")}</span>
              <div class="styles">
                <For each={st()!.augStyles}>
                  {(s) => <span class="style">{localName(s)} <span class="muted">{pct(s.pickRate)}</span></span>}
                </For>
              </div>
            </div>
          </Show>
          <Source cat={p.x.cat} url={championUrl(p.x.cat, p.x.champs.get(p.champId))} />
        </div>
      </Show>
    </Show>
  );
}

function AugRow(p: { a?: Augment; tier?: string; pickRate?: number | null; scope?: string }) {
  return (
    <Show when={p.a}>
      <div class={`aug r-${p.a!.rarity}`}>
        <img src={p.a!.icon} alt="" loading="lazy" />
        <span class="name">{p.a!.name}</span>
        <span class="rar">{rarityText(p.a!.rarity)}</span>
        <Show when={p.pickRate != null}>
          <span class="stat" title={p.scope === "global" ? t("pick_global_tip") : t("pick_champ_tip")}>
            {t("pick_rate")} <b>{pct(p.pickRate)}</b>
          </span>
        </Show>
        <Tier t={p.tier} />
      </div>
    </Show>
  );
}

function TopAugments(p: { x: Ctx; champId: number; limit?: number }) {
  const recs = createMemo(() =>
    [...(p.x.stat(p.champId)?.augments ?? [])]
      .sort((a, b) => tierRank(b.tier) - tierRank(a.tier) || (b.pickRate ?? 0) - (a.pickRate ?? 0))
      .slice(0, p.limit ?? 200));
  return (
    <div class="list">
      <For each={recs()} fallback={<div class="muted">{t("no_aug_data")}</div>}>
        {(r) => <AugRow a={p.x.augs.get(r.id)} tier={r.tier} pickRate={r.pickRate} />}
      </For>
    </div>
  );
}

const matchChamp = (c: Champion, s: string) =>
  c.name.toLowerCase().includes(s) || c.title.toLowerCase().includes(s) || c.alias.toLowerCase().includes(s);

function TierList(p: { x: Ctx }) {
  const [q, setQ] = createSignal("");
  const list = createMemo(() => {
    const s = q().trim().toLowerCase();
    return Object.entries(p.x.cat?.stats?.champions ?? {})
      .map(([id, st]) => ({ c: p.x.champs.get(Number(id)), st }))
      .filter((v) => v.c && (!s || matchChamp(v.c, s)))
      .sort((a, b) => champScore(b.st) - champScore(a.st));
  });
  return (
    <section>
      <h3>{t("tier_list")} <span class="muted">{p.x.cat?.stats ? `v${p.x.cat.stats.patch}` : ""}</span><span class="right"><Source cat={p.x.cat} /></span></h3>
      <input placeholder={t("search_champ")} value={q()} onInput={(e) => setQ(e.currentTarget.value)} />
      <div class="list" style={{ "margin-top": "6px" }}>
        <For each={list().slice(0, 80)} fallback={<div class="muted">{t("stats_not_loaded")}</div>}>
          {(v) => <ChampChip c={v.c} s={v.st} />}
        </For>
      </div>
    </section>
  );
}

function SelectView(p: { snap: Snapshot; x: Ctx }) {
  const cs = () => p.snap.champSelect;
  const myScore = () => champScore(p.x.stat(cs()?.myChampionId ?? 0));
  const bench = createMemo(() =>
    [...(cs()?.bench ?? [])].sort((a, b) => champScore(p.x.stat(b)) - champScore(p.x.stat(a))));
  // 只“高亮”更强的板凳英雄，是否交换由玩家决定
  const bestBench = () => {
    const top = bench()[0];
    return top && champScore(p.x.stat(top)) > myScore() + 2 ? top : 0;
  };
  const swap = async (id: number) => {
    try { await api.benchSwap(id); } catch (e) { p.x.notify(t("swap_failed") + e); }
  };
  const reroll = async () => {
    try { await api.reroll(); } catch (e) { p.x.notify(t("reroll_failed") + e); }
  };

  return (
    <Show when={cs()} fallback={<>
      <section>
        <div class="empty">{p.snap.connected ? t("wait_champ_select") : t("start_client")}</div>
        <WindowModeHint snap={p.snap} notify={p.x.notify} />
      </section>
      <TierList x={p.x} />
    </>}>
      <section>
        <Show when={!p.x.cat?.stats}><div class="hint">{t("stats_not_loaded_hint")}</div></Show>
        <WindowModeHint snap={p.snap} notify={p.x.notify} />
        <h3>{t("my_champ")}
          <Show when={cs()!.rerollsRemaining > 0}>
            <button class="small right" onClick={reroll}>{t("reroll", { n: cs()!.rerollsRemaining })}</button>
          </Show>
        </h3>
        <ChampChip me c={p.x.champs.get(cs()!.myChampionId)} s={p.x.stat(cs()!.myChampionId)} />
        <BuildPanel x={p.x} champId={cs()!.myChampionId} inChampSelect />
      </section>

      <Show when={cs()!.benchEnabled}>
        <section>
          <h3>{t("bench")} <span class="muted">{t("by_rating")}</span></h3>
          <div class="list">
            <For each={bench()} fallback={<div class="muted">{t("none")}</div>}>
              {(id) => <ChampChip c={p.x.champs.get(id)} s={p.x.stat(id)} best={id === bestBench()}
                right={<button class="small" onClick={() => swap(id)}>{t("swap")}</button>} />}
            </For>
          </div>
        </section>
      </Show>

      <section>
        <h3>{t("top_augs")} <span class="muted">{t("top8")}</span></h3>
        <TopAugments x={p.x} champId={cs()!.myChampionId} limit={8} />
      </section>

      <section>
        <h3>{t("teammates")}</h3>
        <div class="list">
          <For each={cs()!.team.filter((m) => !m.isMe)}>
            {(m) => <ChampChip c={p.x.champs.get(m.championId)} s={p.x.stat(m.championId)} />}
          </For>
        </div>
      </section>
    </Show>
  );
}

function AugmentView(p: { snap: Snapshot; x: Ctx }) {
  const [champId, setChampId] = createSignal(p.snap.lastChampionId);
  const [champQuery, setChampQuery] = createSignal("");
  const [q, setQ] = createSignal("");
  const [rarity, setRarity] = createSignal("all");
  createEffect(on(() => p.snap.lastChampionId, (id) => { if (id) setChampId(id); }, { defer: true }));

  const champMatches = createMemo(() => {
    const s = champQuery().trim().toLowerCase();
    if (!s) return [];
    return (p.x.cat?.champions ?? []).filter((c) => matchChamp(c, s)).slice(0, 8);
  });

  const pool = createMemo(() => {
    const s = q().trim().toLowerCase();
    return (p.x.cat?.augments ?? [])
      .filter((a) => inPool(p.x.cat, a) && (rarity() === "all" || a.rarity === rarity()))
      .filter((a) => !s || [a.name, ...a.altNames].some((n) => n.toLowerCase().includes(s)))
      .map((a) => ({ a, info: augInfo(p.x.cat, champId(), a.id) }))
      .sort((x, y) => tierRank(y.info.tier) - tierRank(x.info.tier) || (y.info.pickRate ?? 0) - (x.info.pickRate ?? 0));
  });

  return (
    <>
      <section>
        <h3>{t("current_offer")}
          <Show when={p.snap.ocrLang}><span class="muted">{p.snap.ocrLang}</span></Show>
          <button class="small right" onClick={() => api.ocrScanNow()}>{t("scan_now")}</button>
        </h3>
        <Show when={p.snap.ocrError}><div class="hint err">{p.snap.ocrError}</div></Show>
        <WindowModeHint snap={p.snap} notify={p.x.notify} />
        <Show when={p.snap.ocrDebug}>
          <div class="dbg">
            {t("dbg_last")}（{ago(p.snap.ocrDebug!.at)}）：
            {p.snap.ocrDebug!.status === "scanned"
              ? t("dbg_scanned", { lines: p.snap.ocrDebug!.lines, matched: p.snap.ocrDebug!.matched })
              : t(p.snap.ocrDebug!.status === "no_game" ? "dbg_no_game"
                : p.snap.ocrDebug!.status === "no_cards" ? "dbg_no_cards" : "dbg_background")}
          </div>
        </Show>
        <div class="list">
          <For each={p.snap.offer} fallback={<div class="muted">
            {p.snap.phase === "InProgress" ? t("offer_hint_ingame") : t("offer_hint_idle")}
          </div>}>
            {(o) => {
              const i = () => augInfo(p.x.cat, champId(), o.augmentId);
              return <AugRow a={p.x.augs.get(o.augmentId)} tier={i().tier} pickRate={i().pickRate} scope={i().scope} />;
            }}
          </For>
        </div>
      </section>

      <section>
        <h3>{t("champion")}<span class="right"><Source cat={p.x.cat} url={championUrl(p.x.cat, p.x.champs.get(champId()))} /></span></h3>
        <div class="picker">
          <ChampChip c={p.x.champs.get(champId())} s={p.x.stat(champId())} />
          <input placeholder={t("search_champ_long")} value={champQuery()} onInput={(e) => setChampQuery(e.currentTarget.value)} />
          <Show when={champMatches().length}>
            <div class="dropdown">
              <For each={champMatches()}>
                {(c) => (
                  <button onClick={() => { setChampId(c.id); setChampQuery(""); }}>
                    <img src={c.icon} alt="" />{c.name} · {c.title}
                  </button>
                )}
              </For>
            </div>
          </Show>
        </div>
        <BuildPanel x={p.x} champId={champId()} inChampSelect={p.snap.phase === "ChampSelect"} />
      </section>

      <section>
        <h3>{t("champ_aug_tiers")}</h3>
        <TopAugments x={p.x} champId={champId()} limit={30} />
      </section>

      <section>
        <h3>{t("aug_search")}</h3>
        <input placeholder={t("aug_search_ph")} value={q()} onInput={(e) => setQ(e.currentTarget.value)} />
        <div class="chips">
          <For each={["all", "silver", "gold", "prismatic"]}>
            {(r) => (
              <button classList={{ active: rarity() === r }} onClick={() => setRarity(r)}>
                {r === "all" ? t("all") : rarityText(r)}
              </button>
            )}
          </For>
        </div>
        <div class="list">
          <For each={pool().slice(0, 80)}>
            {(v) => <AugRow a={v.a} tier={v.info.tier} pickRate={v.info.pickRate} scope={v.info.scope} />}
          </For>
        </div>
        <div class="muted foot">{t("no_aug_wr")} <Source cat={p.x.cat} /></div>
      </section>
    </>
  );
}

/** 每种语言用自己的文字显示，方便看不懂当前界面的人切回来 */
const UI_LANGS: [string, string][] = [
  ["zh-Hans", "简体中文"], ["zh-Hant", "繁體中文"], ["en", "English"],
];

type TextKey = "acceptDelayMs" | "ocrHotkey" | "statsUrl" | "cdragonBase" | "leagueDir";

function SettingsView(p: { snap: Snapshot; cat: Catalog | null; notify: (m: string) => void }) {
  const [s, setS] = createSignal<Settings | null>(null);
  const [busy, setBusy] = createSignal(false);
  const [saved, setSaved] = createSignal(false);
  let timer: ReturnType<typeof setTimeout> | undefined;
  onMount(async () => setS(await api.settings()));

  // 自动保存：开关、下拉框立即保存；输入框停止输入 600ms 后保存
  const persist = async () => {
    try {
      await api.saveSettings(s()!);
      setLang(await api.uiLang());
      setSaved(true);
      setTimeout(() => setSaved(false), 1500);
    } catch (e) {
      p.notify(t("s_save_failed") + e);
    }
  };
  const patch = <K extends keyof Settings>(k: K, v: Settings[K], debounce = false) => {
    setS({ ...s()!, [k]: v });
    clearTimeout(timer);
    if (debounce) timer = setTimeout(persist, 600);
    else persist();
  };
  const refresh = async () => {
    setBusy(true);
    try { await api.refreshData(); p.notify(t("s_refreshed")); } catch (e) { p.notify(t("s_refresh_failed") + e); }
    setBusy(false);
  };
  const check = (k: "autoAccept" | "alwaysOnTop" | "autoItems" | "autoSpells" | "ocrEnabled" | "overlayEnabled" | "autoUpdate" | "floatEnabled", key: Key) => (
    <label class="toggle"><span>{t(key)}</span><input type="checkbox" checked={s()![k]} onChange={(e) => patch(k, e.currentTarget.checked)} /></label>
  );
  const text = (k: TextKey, key: Key, ph = "") => (
    <label>{t(key)}
      <input value={String(s()![k])} placeholder={ph}
        onInput={(e) => patch(k, (k === "acceptDelayMs" ? Math.max(0, Number(e.currentTarget.value) || 0) : e.currentTarget.value) as never, true)} />
    </label>
  );

  return (
    <Show when={s()}>
      <Show when={saved()}><div class="saved-float">✓ {t("s_saved")}</div></Show>
      <section class="form">
        <h3>{t("s_ui")}</h3>
        <label>{t("s_ui_lang")}
          <select value={s()!.uiLang} onChange={(e) => patch("uiLang", e.currentTarget.value)}>
            <option value="auto">{t("s_ui_auto")}</option>
            <For each={UI_LANGS}>{([v, label]) => <option value={v}>{label}</option>}</For>
          </select>
        </label>
        <Show when={lang() === "zh-Hans"}>
          <label>{t("s_zh_variant")}
            <select value={s()!.zhVariant} onChange={(e) => patch("zhVariant", e.currentTarget.value)}>
              <option value="cn">{t("s_zh_cn")}</option>
              <option value="my">{t("s_zh_my")}</option>
            </select>
          </label>
        </Show>
        {check("alwaysOnTop", "s_on_top")}
      </section>

      <section class="form">
        <h3>{t("s_game")}</h3>
        {check("floatEnabled", "s_float")}
        {check("autoAccept", "s_auto_accept")}
        {text("acceptDelayMs", "s_accept_delay")}
        <div class="subgroup">
          <span class="caption">{t("s_auto_when")}</span>
          {check("autoItems", "s_auto_items")}
          {check("autoSpells", "s_auto_spells")}
        </div>

      </section>

      <section class="form">
        <h3>{t("s_ocr")}</h3>
        <WindowModeHint snap={p.snap} notify={p.notify} always />
        {check("ocrEnabled", "s_ocr_enabled")}
        {check("overlayEnabled", "s_overlay")}
        <label>{t("s_overlay_y")}
          <div class="slider">
            <input type="range" min="0.10" max="0.60" step="0.005" value={s()!.overlayBottom}
              onInput={(e) => patch("overlayBottom", Number(e.currentTarget.value), true)} />
            <button class="small" title={t("s_overlay_test_tip")}
              onClick={async () => { await persist(); api.overlayTest().catch((e) => p.notify(String(e))); }}>{t("s_overlay_test")}</button>
          </div>
        </label>
        <label>{t("s_ocr_lang")}
          <select value={s()!.ocrLang} onChange={(e) => patch("ocrLang", e.currentTarget.value)}>
            <option value="auto">{t("s_ocr_auto")}</option>
            <option value="zh-Hans">{t("s_ocr_hans")}</option>
            <option value="zh-Hant">{t("s_ocr_hant")}</option>
            <option value="en-US">English</option>
          </select>
          <span class="caption">
            {(p.snap.ocrInstalled ?? []).length
              ? `${t("s_ocr_detected")}：${p.snap.ocrInstalled.join(" · ")}`
              : t("s_ocr_none")}
          </span>
        </label>
        {text("ocrHotkey", "s_hotkey", "Ctrl+Shift+A")}

      </section>

      <section class="form">
        <h3>{t("s_data")}</h3>
        <label>{t("s_source")}
          <select value={s()!.statsSource} onChange={(e) => patch("statsSource", e.currentTarget.value)}>
            <option value="arammeta">AramMeta（arammeta.com）</option>
            <option value="custom">{t("s_source_custom")}</option>
          </select>
        </label>
        <Show when={s()!.statsSource === "custom"}>{text("statsUrl", "s_pack", "https://.../mayhem-stats.json")}</Show>
        {check("autoUpdate", "s_auto_update")}
        {text("cdragonBase", "s_cdragon")}
        {text("leagueDir", "s_league_dir", "C:\\Riot Games\\League of Legends")}

        <div class="row gap">
          <button class="primary" onClick={refresh} disabled={busy()}>{busy() ? t("s_refreshing") : t("s_refresh")}</button>
        </div>
        <div class="muted foot">
          {t("s_checked")}：{ago(p.cat?.checkedAt ?? 0)} · {t("s_client_lang")}：{p.snap.clientLocale ?? "?"} · {t("s_data_lang")}：{p.cat?.locale ?? "?"}
          <br />
          {t("s_static")}：v{p.cat?.patch || "?"} · {p.cat?.champions.length ?? 0} {t("s_champs")} · {p.cat?.augments.filter((a) => inPool(p.cat, a)).length ?? 0} {t("s_augs")}
          <br />
          {t("s_stats")}：{p.cat?.stats ? `${p.cat.stats.source}（v${p.cat.stats.patch}，${p.cat.stats.updatedAt}）` : t("s_not_loaded")} <Source cat={p.cat} />
          <Show when={p.cat?.statsError}><br /><span class="err">{p.cat!.statsError}</span></Show>
        </div>
      </section>
    </Show>
  );
}
