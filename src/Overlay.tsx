import { listen } from "@tauri-apps/api/event";
import { createMemo, createSignal, For, onCleanup, onMount, Show } from "solid-js";
import { api, augInfo, Catalog, pct, Snapshot, tierRank } from "./api";
import { setLang, t } from "./i18n";

/** 游戏上方的透明悬浮层：在每张强化卡片标题下方显示档位与选取率。鼠标穿透，不抢焦点。 */
export default function Overlay() {
  const [snap, setSnap] = createSignal<Snapshot | null>(null);
  const [cat, setCat] = createSignal<Catalog | null>(null);

  onMount(async () => {
    document.documentElement.classList.add("overlay");
    const un1 = await listen<Snapshot>("snapshot", (e) => setSnap(e.payload));
    const un2 = await listen("catalog-updated", async () => setCat(await api.catalog()));
    const un3 = await listen<string>("ui-lang", (e) => setLang(e.payload));
    onCleanup(() => { un1(); un2(); un3(); });
    setLang(await api.uiLang());
    setSnap(await api.snapshot());
    setCat(await api.catalog());
  });

  const cards = createMemo(() => {
    const s = snap();
    if (!s) return [];
    const list = s.offer.map((o) => ({ ...o, ...augInfo(cat(), s.lastChampionId, o.augmentId) }));
    const top = Math.max(0, ...list.map((c) => tierRank(c.tier)));
    return list.map((c) => ({ ...c, top: top > 0 && tierRank(c.tier) === top }));
  });

  return (
    <>
      <For each={cards()}>
        {(c) => (
          <div class="ov-badge" classList={{ top: c.top }}
            style={{ left: `${c.x * 100}%`, top: `calc(${c.y * 100}% + 26px)` }}>
            <Show when={c.tier} fallback={<span class="ov-none">{t("no_tier")}</span>}>
              <span class={`tier t${tierRank(c.tier)}`}>{c.tier}</span>
            </Show>
            <Show when={c.pickRate != null}><span class="ov-pick">{t("ov_pick")} {pct(c.pickRate)}</span></Show>
          </div>
        )}
      </For>
      <Show when={cards().length && cat()?.stats?.attribution}>
        <div class="ov-source">{cat()!.stats!.attribution!.label}</div>
      </Show>
    </>
  );
}
