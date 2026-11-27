import { listen } from "@tauri-apps/api/event";
import { createMemo, createSignal, For, onCleanup, onMount, Show } from "solid-js";
import { api, augInfo, Catalog, OfferCard, pct, Settings, Snapshot, tierRank } from "./api";
import { setLang, t } from "./i18n";

const RAR: Record<string, string> = { silver: "r-silver", gold: "r-gold", prismatic: "r-prismatic" };

/**
 * 强化卡片悬浮条：透明、鼠标穿透的窗口覆盖整个游戏画面，
 * 在三张卡片正下方画一条面板，每张卡一格：档位、强化名、选取率、本英雄排名，最好的一张标「首选」。
 */
export default function Overlay() {
  const [snap, setSnap] = createSignal<Snapshot | null>(null);
  const [cat, setCat] = createSignal<Catalog | null>(null);
  const [settings, setSettings] = createSignal<Settings | null>(null);
  const [test, setTest] = createSignal(false);
  const [vw, setVw] = createSignal(window.innerWidth);

  onMount(async () => {
    document.documentElement.classList.add("overlay");
    const onResize = () => setVw(window.innerWidth);
    window.addEventListener("resize", onResize);
    const un = await Promise.all([
      listen<Snapshot>("snapshot", (e) => setSnap(e.payload)),
      listen("catalog-updated", async () => setCat(await api.catalog())),
      listen<string>("ui-lang", (e) => setLang(e.payload)),
      listen<boolean>("overlay-test", async (e) => { setSettings(await api.settings()); setTest(e.payload); }),
    ]);
    onCleanup(() => { un.forEach((f) => f()); window.removeEventListener("resize", onResize); });
    setLang(await api.uiLang());
    setSettings(await api.settings());
    setSnap(await api.snapshot());
    setCat(await api.catalog());
  });

  const champ = () => snap()?.lastChampionId || 0;

  /** 测试模式：用该英雄前 3 个强化（没有就用强化池前 3 个）做示例 */
  const sample = (): OfferCard[] => {
    const recs = cat()?.stats?.champions[String(champ())]?.augments ?? [];
    const ids = recs.length >= 3 ? recs.slice(0, 3).map((a) => a.id)
      : (cat()?.augments ?? []).filter((a) => a.mayhem).slice(0, 3).map((a) => a.id);
    return ids.map((id, i) => ({ augmentId: id, x: 0.32 + i * 0.18, y: 0.42, confidence: 1 }));
  };

  const cards = createMemo(() => {
    const offer = test() ? sample() : snap()?.offer ?? [];
    const recs = cat()?.stats?.champions[String(champ())]?.augments ?? [];
    const list = offer.map((o) => {
      const info = augInfo(cat(), champ(), o.augmentId);
      const idx = recs.findIndex((r) => r.id === o.augmentId);
      return {
        ...o, ...info,
        aug: cat()?.augments.find((a) => a.id === o.augmentId),
        rank: idx >= 0 ? idx + 1 : 0,
        total: recs.length,
      };
    }).sort((a, b) => a.x - b.x);
    const top = Math.max(0, ...list.map((c) => tierRank(c.tier)));
    return list.map((c) => ({ ...c, best: top > 0 && tierRank(c.tier) === top }));
  });

  const [vh, setVh] = createSignal(window.innerHeight);
  onMount(() => {
    const f = () => setVh(window.innerHeight);
    window.addEventListener("resize", f);
    onCleanup(() => window.removeEventListener("resize", f));
  });

  /**
   * 真实版式（1080p 基准）：三张卡水平居中，中心间距 368、卡宽约 310，上沿约在画面高度 26.5%。
   * 窗口与游戏画面同尺寸，所以直接按窗口宽高换算。
   */
  const unit = () => vh() / 1080;
  const slotX = (k: number) => vw() / 2 + k * 368 * unit();
  const cellW = () => Math.max(150, 300 * unit());
  /** 每个识别结果吸附到最近的卡位，标题只认出半行或只认出两张也不会错位 */
  const slotted = createMemo(() => {
    const used = new Set<number>();
    return cards().map((c) => {
      const order = [-1, 0, 1].sort((a, b) => Math.abs(slotX(a) - c.x * vw()) - Math.abs(slotX(b) - c.x * vw()));
      const k = order.find((s) => !used.has(s)) ?? order[0];
      used.add(k);
      return { ...c, left: slotX(k) - cellW() / 2 };
    });
  });
  /** 悬浮条下沿：默认紧贴卡片上沿（卡片下方是重随按钮，不能挡） */
  const bottom = () => Math.max(0.08, Math.min(0.95, settings()?.overlayBottom ?? 0.253));
  const span = () => ({ left: slotX(-1) - cellW() / 2 - 10, right: slotX(1) + cellW() / 2 + 10 });

  /** 标签随游戏画面高度缩放（以 1080p 为 1），高分辨率下不会显得太小 */
  const scale = () => Math.max(0.85, Math.min(1.6, unit()));
  /** 标签下沿距卡片上沿 6px；设置里可整体上下移动 */
  const anchorBottom = () => `calc(${(1 - bottom()) * 100}% + 6px)`;

  return (
    <Show when={cards().length}>
      <For each={slotted()}>
        {(c, i) => (
          <div class={`ov-pill ${RAR[c.aug?.rarity ?? ""] ?? ""}`} classList={{ best: c.best }}
            style={{ left: `${c.left + cellW() / 2}px`, bottom: anchorBottom(), transform: `translateX(-50%) scale(${scale()})` }}>
            <Show when={c.best}><span class="ov-star">★</span></Show>
            <Show when={c.tier} fallback={<span class="ov-none">{t("no_tier")}</span>}>
              <span class={`tier t${tierRank(c.tier)}`}>{c.tier}</span>
            </Show>
            <Show when={c.pickRate != null}><span class="ov-pick">{pct(c.pickRate)}</span></Show>
            <Show when={c.rank}><span class="ov-rank">#{c.rank}</span></Show>
            <Show when={test() && i() === 0}><span class="ov-tag">{t("ov_sample")}</span></Show>
          </div>
        )}
      </For>
      <div class="ov-src" style={{ left: `${span().right - 10}px`, bottom: anchorBottom(), transform: `translateY(-4px) scale(${scale()})` }}>
        {cat()?.stats?.attribution?.label}
      </div>
    </Show>
  );
}
