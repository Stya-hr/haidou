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

  /** 格宽：按卡片间距算，限制在 150–250px */
  const cellW = () => {
    const xs = cards().map((c) => c.x);
    const gap = xs.length > 1 ? (xs[xs.length - 1] - xs[0]) / (xs.length - 1) : 0.18;
    return Math.max(150, Math.min(250, gap * vw() * 0.9));
  };
  const top = () => {
    const ys = cards().map((c) => c.y);
    return Math.min(0.92, Math.max(...ys, 0) + (settings()?.overlayOffsetY ?? 0.24));
  };
  const span = () => {
    const xs = cards().map((c) => c.x * vw());
    return { left: Math.min(...xs) - cellW() / 2 - 10, right: Math.max(...xs) + cellW() / 2 + 10 };
  };
  const champInfo = () => cat()?.champions.find((c) => c.id === champ());

  return (
    <Show when={cards().length}>
      <div class="ov-panel" style={{ left: `${span().left}px`, width: `${span().right - span().left}px`, top: `${top() * 100}%` }}>
        <div class="ov-head">
          <Show when={champInfo()}><img src={champInfo()!.icon} alt="" /><b>{champInfo()!.name}</b></Show>
          <Show when={snap()?.liveLevel}><span>Lv.{snap()!.liveLevel}</span></Show>
          <Show when={test()}><span class="ov-tag">{t("ov_sample")}</span></Show>
          <span class="ov-src">{cat()?.stats?.attribution?.label}</span>
        </div>
        <div class="ov-cells">
          <For each={cards()}>
            {(c) => (
              <div class={`ov-cell ${RAR[c.aug?.rarity ?? ""] ?? ""}`} classList={{ best: c.best }}
                style={{ left: `${c.x * vw() - span().left - cellW() / 2}px`, width: `${cellW()}px` }}>
                <div class="ov-row">
                  <Show when={c.tier} fallback={<span class="ov-none">{t("no_tier")}</span>}>
                    <span class={`tier big t${tierRank(c.tier)}`}>{c.tier}</span>
                  </Show>
                  <Show when={c.best}><span class="ov-best">{t("ov_best")}</span></Show>
                </div>
                <div class="ov-name">{c.aug?.name}</div>
                <div class="ov-meta">
                  <Show when={c.pickRate != null}><span>{t("ov_pick")} <b>{pct(c.pickRate)}</b></span></Show>
                  <Show when={c.rank}><span>{t("ov_rank", { r: c.rank, n: c.total })}</span></Show>
                </div>
              </div>
            )}
          </For>
        </div>
      </div>
    </Show>
  );
}
