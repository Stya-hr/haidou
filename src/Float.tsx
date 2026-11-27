import { listen } from "@tauri-apps/api/event";
import { createEffect, createMemo, createSignal, For, on, onCleanup, onMount, Show } from "solid-js";
import { api, Catalog, champScore, ChampStats, pct, Snapshot, tierRank } from "./api";
import { localName, setLang, t } from "./i18n";

/**
 * 选人悬浮窗：吸附在客户端板凳卡片下方的横条。
 * 左：我的英雄；中：板凳英雄（点击交换，最强的高亮）；右：主流出装 + 导入、召唤师技能。
 */
export default function Float() {
  const [snap, setSnap] = createSignal<Snapshot | null>(null);
  const [cat, setCat] = createSignal<Catalog | null>(null);
  const [msg, setMsg] = createSignal("");
  let msgTimer: ReturnType<typeof setTimeout> | undefined;
  const flash = (m: string) => { setMsg(m); clearTimeout(msgTimer); msgTimer = setTimeout(() => setMsg(""), 2500); };

  onMount(async () => {
    document.documentElement.classList.add("floatwin");
    const un = await Promise.all([
      listen<Snapshot>("snapshot", (e) => setSnap(e.payload)),
      listen("catalog-updated", async () => setCat(await api.catalog())),
      listen<string>("ui-lang", (e) => setLang(e.payload)),
      listen<string>("toast", (e) => flash(e.payload)),
    ]);
    onCleanup(() => un.forEach((f) => f()));
    setLang(await api.uiLang());
    setSnap(await api.snapshot());
    setCat(await api.catalog());
  });

  const cs = () => snap()?.champSelect;
  const me = () => cs()?.myChampionId ?? 0;
  const champ = (id: number) => cat()?.champions.find((c) => c.id === id);
  const stat = (id: number): ChampStats | undefined => cat()?.stats?.champions[String(id)];
  const item = (id: number) => cat()?.items.find((i) => i.id === id);
  const spell = (id: number) => cat()?.spells.find((s) => s.id === id);

  createEffect(on(me, (id) => { if (id > 0) api.ensureChampion(id).catch(() => {}); }));

  const bench = createMemo(() => [...(cs()?.bench ?? [])].sort((a, b) => champScore(stat(b)) - champScore(stat(a))));
  const best = () => {
    const top = bench()[0];
    return top && champScore(stat(top)) > champScore(stat(me())) + 2 ? top : 0;
  };
  const arch = () => stat(me())?.archetypes[0];

  const swap = async (id: number) => {
    try { await api.benchSwap(id); } catch (e) { flash(String(e)); }
  };
  const importItems = async () => {
    try { flash(await api.applyBuild(me(), "items")); } catch (e) { flash(String(e)); }
  };

  return (
    <Show when={cs()}>
      <div class="fl-panel">
        <div class="fl-bar" data-tauri-drag-region>
          <img src="/icon.png" alt="" data-tauri-drag-region />
          <span class="fl-brand" data-tauri-drag-region>HexPilot</span>
          <Show when={msg()} fallback={<span class="fl-src" data-tauri-drag-region>{cat()?.stats?.attribution?.label}</span>}>
            <span class="fl-msg">{msg()}</span>
          </Show>
          <button class="fl-x" title={t("fl_close")} onClick={() => api.floatDismiss()}>×</button>
        </div>

        <div class="fl-body">
          {/* 我的英雄 */}
          <div class="fl-me">
            <img class="fl-avatar" src={champ(me())?.icon} alt="" />
            <div class="fl-meta">
              <div class="fl-name">{champ(me())?.name ?? t("not_selected")}</div>
              <div class="fl-sub">
                <Show when={stat(me())}>
                  <span class={`tier t${tierRank(stat(me())!.tier)}`}>{stat(me())!.tier}</span>
                  <span class="stat">{t("win_rate")} <b>{pct(stat(me())!.winRate)}</b></span>
                </Show>
              </div>
            </div>
          </div>

          {/* 板凳 */}
          <Show when={cs()!.benchEnabled && bench().length}>
            <div class="fl-sep" />
            <div class="fl-bench">
              <For each={bench()}>
                {(id) => (
                  <button class="fl-benchbtn" classList={{ best: id === best() }} title={`${champ(id)?.name ?? ""} · ${t("fl_swap_tip")}`} onClick={() => swap(id)}>
                    <img src={champ(id)?.icon} alt="" />
                    <Show when={stat(id)}><span class={`tier mini t${tierRank(stat(id)!.tier)}`}>{stat(id)!.tier}</span></Show>
                  </button>
                )}
              </For>
            </div>
          </Show>

          {/* 出装与技能 */}
          <Show when={arch() || stat(me())?.spells.length}>
            <div class="fl-sep" />
            <div class="fl-build">
              <Show when={arch()}>
                <div class="fl-archname">
                  <span class={`tier mini t${tierRank(arch()!.tier)}`}>{arch()!.tier}</span>
                  <span>{localName(arch()!)}</span>
                  <span class="stat">{pct(arch()!.winRate)}</span>
                </div>
                <div class="fl-items">
                  <For each={[...arch()!.core, ...arch()!.options].slice(0, 6)}>
                    {(id) => <img src={item(id)?.icon} title={item(id)?.name} alt="" />}
                  </For>
                </div>
              </Show>
            </div>
            <div class="fl-actions">
              <div class="fl-spells">
                <For each={stat(me())?.spells ?? []}>{(id) => <img src={spell(id)?.icon} title={spell(id)?.name} alt="" />}</For>
              </div>
              <Show when={arch()}><button class="fl-import" onClick={importItems}>{t("fl_import")}</button></Show>
            </div>
          </Show>
        </div>
      </div>
    </Show>
  );
}
