import { listen } from "@tauri-apps/api/event";
import { createEffect, createMemo, createSignal, For, JSX, on, onCleanup, onMount, Show } from "solid-js";
import {
  api, Archetype, augInfo, Augment, Catalog, Champion, champScore, ChampStats, championUrl, inPool,
  MAYHEM_QUEUE, pct, PHASE_TEXT, Settings, Snapshot, Thing, tierRank,
} from "./api";

type Tab = "select" | "augments" | "settings";

const EMPTY: Snapshot = {
  connected: false, phase: "None", queueId: null, champSelect: null, lastChampionId: 0,
  liveLevel: null, liveDead: false, clientLocale: null, ocrLang: null, offer: [], ocrError: null,
};

const RARITY_TEXT: Record<string, string> = { silver: "白银", gold: "黄金", prismatic: "棱彩", other: "其他" };

export default function App() {
  const [snap, setSnap] = createSignal<Snapshot>(EMPTY);
  const [cat, setCat] = createSignal<Catalog | null>(null);
  const [tab, setTab] = createSignal<Tab>("select");
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
    onCleanup(() => { un1(); un2(); un3(); });
    setSnap(await api.snapshot());
    setCat(await api.catalog());
  });

  // 阶段切换时自动跳到对应页面
  createEffect(on(() => snap().phase, (p) => {
    if (p === "ChampSelect") setTab("select");
    else if (p === "InProgress" || p === "GameStart") setTab("augments");
  }));

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
        <span class={`dot ${snap().connected ? "on" : ""}`} />
        <span>{snap().connected ? PHASE_TEXT[snap().phase] ?? snap().phase : "等待客户端…"}</span>
        <Show when={snap().queueId}>
          <span class={`queue ${snap().queueId === MAYHEM_QUEUE ? "mayhem" : ""}`}>
            {snap().queueId === MAYHEM_QUEUE ? "海克斯大乱斗" : `队列 ${snap().queueId}`}
          </span>
        </Show>
        <Show when={snap().liveLevel}>
          <span class="lvl">Lv.{snap().liveLevel}{snap().liveDead ? " · 阵亡" : ""}</span>
        </Show>
        <span class="patch">{cat()?.patch ? `v${cat()!.patch}` : ""}</span>
      </header>

      <nav>
        <button classList={{ active: tab() === "select" }} onClick={() => setTab("select")}>选人</button>
        <button classList={{ active: tab() === "augments" }} onClick={() => setTab("augments")}>强化</button>
        <button classList={{ active: tab() === "settings" }} onClick={() => setTab("settings")}>设置</button>
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
          const t = p.from.get(id);
          return <img src={t?.icon} title={t?.name ?? String(id)} alt={t?.name ?? ""}
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
        <div class="name">{p.c ? p.c.name : "未选择"} <span class="muted">{p.c?.title}</span></div>
        <div class="sub">
          <Show when={p.s} fallback={<span class="muted">无数据</span>}>
            <Tier t={p.s!.tier} />
            <Stat label="胜率" v={p.s!.winRate} />
            <Stat label="选取" v={p.s!.pickRate} />
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
      <Show when={st()!.detailed} fallback={<div class="muted">正在加载出装与流派…</div>}>
        <div class="build">
          <Show when={st()!.archetypes.length}>
            <div class="bhead">出装流派
              <button class="small right" onClick={() => apply("items")} title="写入客户端装备方案，游戏内商店可直接使用">导入出装</button>
            </div>
            <For each={st()!.archetypes}>
              {(a: Archetype) => (
                <div class="arch">
                  <div class="arow">
                    <Tier t={a.tier} />
                    <span class="aname">{a.name}</span>
                    <Stat label="胜率" v={a.winRate} />
                    <Stat label="选取" v={a.pickRate} />
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
            <div class="brow"><span class="blabel">鞋子</span><Icons ids={ib()!.boots} from={p.x.items} size={22} /></div>
          </Show>
          <Show when={ib()?.situational.length}>
            <div class="brow"><span class="blabel">可选</span><Icons ids={ib()!.situational} from={p.x.items} size={22} /></div>
          </Show>
          <Show when={st()!.spells.length}>
            <div class="brow"><span class="blabel">技能</span>
              <Icons ids={st()!.spells} from={p.x.spells} />
              <Show when={p.inChampSelect}><button class="small right" onClick={() => apply("spells")}>设置技能</button></Show>
            </div>
          </Show>
          <Show when={st()!.augStyles.length}>
            <div class="brow"><span class="blabel">强化</span>
              <div class="styles">
                <For each={st()!.augStyles}>
                  {(s) => <span class="style">{s.name} <span class="muted">{pct(s.pickRate)}</span></span>}
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
        <span class="muted">{RARITY_TEXT[p.a!.rarity]}</span>
        <Show when={p.pickRate != null}>
          <span class="stat" title={p.scope === "global" ? "全体玩家选取率" : "该英雄选取率"}>选取 <b>{pct(p.pickRate)}</b></span>
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
      <For each={recs()} fallback={<div class="muted">该英雄暂无强化数据</div>}>
        {(r) => <AugRow a={p.x.augs.get(r.id)} tier={r.tier} pickRate={r.pickRate} />}
      </For>
    </div>
  );
}

function TierList(p: { x: Ctx }) {
  const [q, setQ] = createSignal("");
  const list = createMemo(() => {
    const s = q().trim().toLowerCase();
    return Object.entries(p.x.cat?.stats?.champions ?? {})
      .map(([id, st]) => ({ c: p.x.champs.get(Number(id)), st }))
      .filter((v) => v.c && (!s || v.c.name.includes(s) || v.c.title.includes(s) || v.c.alias.toLowerCase().includes(s)))
      .sort((a, b) => champScore(b.st) - champScore(a.st));
  });
  return (
    <section>
      <h3>英雄榜 <span class="muted">{p.x.cat?.stats ? `v${p.x.cat.stats.patch}` : ""}</span><span class="right"><Source cat={p.x.cat} /></span></h3>
      <input placeholder="搜索英雄" value={q()} onInput={(e) => setQ(e.currentTarget.value)} />
      <div class="list" style={{ "margin-top": "6px" }}>
        <For each={list().slice(0, 80)} fallback={<div class="muted">统计数据未加载</div>}>
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
    try { await api.benchSwap(id); } catch (e) { p.x.notify(`交换失败：${e}`); }
  };
  const reroll = async () => {
    try { await api.reroll(); } catch (e) { p.x.notify(`重随失败：${e}`); }
  };

  return (
    <Show when={cs()} fallback={<>
      <div class="empty">{p.snap.connected ? "进入选人阶段后自动显示推荐" : "请先启动英雄联盟客户端"}</div>
      <TierList x={p.x} />
    </>}>
      <Show when={!p.x.cat?.stats}><div class="hint">统计数据未加载，可在「设置」中检查数据来源。</div></Show>

      <section>
        <h3>我的英雄
          <Show when={cs()!.rerollsRemaining > 0}>
            <button class="small right" onClick={reroll}>重随（{cs()!.rerollsRemaining}）</button>
          </Show>
        </h3>
        <ChampChip me c={p.x.champs.get(cs()!.myChampionId)} s={p.x.stat(cs()!.myChampionId)} />
        <BuildPanel x={p.x} champId={cs()!.myChampionId} inChampSelect />
      </section>

      <Show when={cs()!.benchEnabled}>
        <section>
          <h3>板凳席 <span class="muted">按评分排序</span></h3>
          <div class="list">
            <For each={bench()} fallback={<div class="muted">暂无</div>}>
              {(id) => <ChampChip c={p.x.champs.get(id)} s={p.x.stat(id)} best={id === bestBench()}
                right={<button class="small" onClick={() => swap(id)}>换</button>} />}
            </For>
          </div>
        </section>
      </Show>

      <section>
        <h3>推荐强化 <span class="muted">前 8</span></h3>
        <TopAugments x={p.x} champId={cs()!.myChampionId} limit={8} />
      </section>

      <section>
        <h3>队友</h3>
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
    return (p.x.cat?.champions ?? [])
      .filter((c) => c.name.toLowerCase().includes(s) || c.title.includes(s) || c.alias.toLowerCase().includes(s))
      .slice(0, 8);
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
        <h3>当前强化（OCR）
          <Show when={p.snap.ocrLang}><span class="muted">{p.snap.ocrLang}</span></Show>
          <button class="small right" onClick={() => api.ocrScanNow()}>立即识别</button>
        </h3>
        <Show when={p.snap.ocrError}><div class="hint err">{p.snap.ocrError}</div></Show>
        <div class="list">
          <For each={p.snap.offer} fallback={<div class="muted">
            {p.snap.phase === "InProgress" ? "升级或阵亡时自动识别，也可按热键 / 点「立即识别」（需无边框模式）" : "对局中可用"}
          </div>}>
            {(o) => {
              const i = augInfo(p.x.cat, champId(), o.augmentId);
              return <AugRow a={p.x.augs.get(o.augmentId)} tier={i.tier} pickRate={i.pickRate} scope={i.scope} />;
            }}
          </For>
        </div>
      </section>

      <section>
        <h3>英雄<span class="right"><Source cat={p.x.cat} url={championUrl(p.x.cat, p.x.champs.get(champId()))} /></span></h3>
        <div class="picker">
          <ChampChip c={p.x.champs.get(champId())} s={p.x.stat(champId())} />
          <input placeholder="搜索英雄（名称 / 称号 / 英文）" value={champQuery()} onInput={(e) => setChampQuery(e.currentTarget.value)} />
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
        <h3>该英雄强化档位</h3>
        <TopAugments x={p.x} champId={champId()} limit={30} />
      </section>

      <section>
        <h3>强化查询</h3>
        <input placeholder="输入强化名称（简 / 繁 / 英文均可）" value={q()} onInput={(e) => setQ(e.currentTarget.value)} />
        <div class="chips">
          <For each={["all", "silver", "gold", "prismatic"]}>
            {(r) => (
              <button classList={{ active: rarity() === r }} onClick={() => setRarity(r)}>
                {r === "all" ? "全部" : RARITY_TEXT[r]}
              </button>
            )}
          </For>
        </div>
        <div class="list">
          <For each={pool().slice(0, 80)}>
            {(v) => <AugRow a={v.a} tier={v.info.tier} pickRate={v.info.pickRate} scope={v.info.scope} />}
          </For>
        </div>
        <div class="muted foot">强化只显示档位与选取率：Riot 开发者政策禁止展示强化胜率。<Source cat={p.x.cat} /></div>
      </section>
    </>
  );
}

const LOCALES: [string, string][] = [
  ["auto", "跟随客户端"], ["zh_my", "简体中文（国际服）"], ["zh_tw", "繁體中文"], ["zh_cn", "简体中文（国服）"],
  ["default", "English"], ["ko_kr", "한국어"], ["ja_jp", "日本語"],
];
const OCR_LANGS: [string, string][] = [
  ["auto", "自动（简繁轮换，命中后锁定）"], ["zh-Hans", "简体优先"], ["zh-Hant", "繁体优先"], ["en-US", "English"],
];

function SettingsView(p: { snap: Snapshot; cat: Catalog | null; notify: (m: string) => void }) {
  const [s, setS] = createSignal<Settings | null>(null);
  const [busy, setBusy] = createSignal(false);
  onMount(async () => setS(await api.settings()));

  const patch = <K extends keyof Settings>(k: K, v: Settings[K]) => setS({ ...s()!, [k]: v });
  const save = async () => {
    try { await api.saveSettings(s()!); p.notify("已保存"); } catch (e) { p.notify(`保存失败：${e}`); }
  };
  const refresh = async () => {
    setBusy(true);
    try { await api.refreshData(); p.notify("数据已更新"); } catch (e) { p.notify(`更新失败：${e}`); }
    setBusy(false);
  };
  const check = (k: "autoAccept" | "alwaysOnTop" | "autoItems" | "autoSpells" | "ocrEnabled" | "overlayEnabled", text: string) => (
    <label class="row"><input type="checkbox" checked={s()![k]} onChange={(e) => patch(k, e.currentTarget.checked)} />{text}</label>
  );

  return (
    <Show when={s()}>
      <section class="form">
        <h3>对局</h3>
        {check("autoAccept", "自动接受对局")}
        <label>接受延迟（毫秒）
          <input type="number" min="0" max="8000" value={s()!.acceptDelayMs}
            onInput={(e) => patch("acceptDelayMs", Math.max(0, Number(e.currentTarget.value) || 0))} />
        </label>
        <div class="row gap">
          <span class="muted">英雄确定后自动：</span>
          {check("autoItems", "导入出装")}
          {check("autoSpells", "设置召唤师技能")}
        </div>

        <h3>强化识别</h3>
        {check("ocrEnabled", "对局中 OCR 识别海克斯强化")}
        {check("overlayEnabled", "在游戏画面上显示档位悬浮层")}
        <label>OCR 语言
          <select value={s()!.ocrLang} onChange={(e) => patch("ocrLang", e.currentTarget.value)}>
            <For each={OCR_LANGS}>{([v, t]) => <option value={v}>{t}</option>}</For>
          </select>
        </label>
        <label>识别热键（留空禁用）
          <input value={s()!.ocrHotkey} placeholder="Ctrl+Shift+A" onInput={(e) => patch("ocrHotkey", e.currentTarget.value)} />
        </label>

        <h3>数据</h3>
        <label>统计来源
          <select value={s()!.statsSource} onChange={(e) => patch("statsSource", e.currentTarget.value)}>
            <option value="arammeta">AramMeta（arammeta.com）</option>
            <option value="custom">自定义数据包</option>
          </select>
        </label>
        <Show when={s()!.statsSource === "custom"}>
          <label>数据包（URL 或本地 JSON 路径）
            <input value={s()!.statsUrl} placeholder="https://.../mayhem-stats.json" onInput={(e) => patch("statsUrl", e.currentTarget.value)} />
          </label>
        </Show>
        <label>数据语言
          <select value={s()!.locale} onChange={(e) => patch("locale", e.currentTarget.value)}>
            <For each={LOCALES}>{([v, t]) => <option value={v}>{t}</option>}</For>
          </select>
        </label>
        <label>CommunityDragon 地址
          <input value={s()!.cdragonBase} onInput={(e) => patch("cdragonBase", e.currentTarget.value)} />
        </label>
        <label>客户端目录（可选，自动识别失败时填写）
          <input value={s()!.leagueDir} placeholder="C:\Riot Games\League of Legends" onInput={(e) => patch("leagueDir", e.currentTarget.value)} />
        </label>
        {check("alwaysOnTop", "窗口置顶")}

        <div class="row gap">
          <button onClick={save}>保存</button>
          <button onClick={refresh} disabled={busy()}>{busy() ? "更新中…" : "强制更新数据"}</button>
        </div>
        <div class="muted foot">
          客户端语言：{p.snap.clientLocale ?? "未知"} · 数据语言：{p.cat?.locale ?? "?"}
          <br />
          静态数据：v{p.cat?.patch || "?"} · {p.cat?.champions.length ?? 0} 英雄 · {p.cat?.augments.filter((a) => inPool(p.cat, a)).length ?? 0} 强化
          <br />
          统计：{p.cat?.stats ? `${p.cat.stats.source}（v${p.cat.stats.patch}，${p.cat.stats.updatedAt}）` : "未加载"} <Source cat={p.cat} />
          <Show when={p.cat?.statsError}><br /><span class="err">{p.cat!.statsError}</span></Show>
        </div>
      </section>
    </Show>
  );
}
