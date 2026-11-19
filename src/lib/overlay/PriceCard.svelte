<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { engine, type BuySimilarInfo } from "$lib/engine.svelte";
  import { game } from "$lib/state/game.svelte";
  import { m } from "$lib/paraglide/messages";
  import {
    age, defaultChoices, formatListing, formatValue, matchNinja, ninjaSources, parseCopied,
    type Choice, type NinjaLine, type NinjaTable,
  } from "./price";

  let { raw, onclose }: { raw: string; onclose: () => void } = $props();

  interface Listing { amount: number | null; currency: string | null; indexed: string | null; account: string | null; online: boolean; stack: number | null }
  interface TradeResult { total: number; url: string; listings: Listing[] }

  const item = $derived(parseCopied(raw));
  const leagueKey = () => `pob-forge:price-league:${game.current}`;

  let leagues = $state<{ id: string; text: string }[]>([]);
  let league = $state("");
  let ninja = $state<{ line: NinjaLine; table: NinjaTable } | null | "loading">(null);
  let info = $state<BuySimilarInfo | null>(null);
  let unsearchable = $state(false);
  let mods = $state<Choice[]>([]);
  let defences = $state<Choice[]>([]);
  let baseType = $state(false);
  let result = $state<TradeResult | null>(null);
  let searching = $state(false);
  let error = $state<string | null>(null);

  const friendly = (e: unknown) => String(e).split("\n")[0].replace(/^EngineError: [a-z_]+: /, "").replace(/^(runtime )?error: /i, "");
  /** PoB colours its lines with ^xRRGGBB and ^N codes. */
  const plain = (s: string) => s.replace(/\^x[0-9a-f]{6}|\^\d/gi, "");
  const num = (s: string) => (s.trim() === "" || Number.isNaN(Number(s)) ? null : Number(s));

  async function lookNinja() {
    const it = item;
    if (!it || !league) return;
    const sources = ninjaSources(game.current, it);
    if (!sources.length) { ninja = null; return; }
    ninja = "loading";
    for (const source of sources) {
      try {
        const table = await invoke<NinjaTable>("price_ninja", { game: game.current, league, ...source });
        const line = matchNinja(it, table);
        if (line) { ninja = { line, table }; return; }
      } catch {
        // A table poe.ninja lacks for this league is not an error for the next one.
      }
    }
    ninja = null;
  }

  async function search() {
    if (!info || searching || !league) return;
    searching = true;
    error = null;
    try {
      const { query } = await engine.buySimilarUrl({
        raw,
        realm: info.realms[0],
        league,
        // Buyout and in-person sellers both: a price check wants every price, not only merchant tabs.
        listed: Math.max(1, info.listed.indexOf("Instant Buyout & In Person") + 1),
        baseType,
        ilvlMin: null,
        ilvlMax: null,
        defences: defences.map((d) => ({ checked: d.checked, min: num(d.min), max: num(d.max) })),
        mods: mods.map((d) => ({ checked: d.checked, min: num(d.min), max: num(d.max) })),
      });
      if (!query) throw new Error("this Path of Building version builds no trade query");
      result = await invoke<TradeResult>("price_trade", { game: game.current, league, query });
    } catch (e) {
      error = friendly(e);
    } finally {
      searching = false;
    }
  }

  function setLeague(id: string) {
    league = id;
    try { localStorage.setItem(leagueKey(), id); } catch {}
    result = null;
    void lookNinja();
    if (info) void search();
  }

  function close() {
    void invoke("overlay_price_done").catch(() => {});
    onclose();
  }

  onMount(() => {
    const onKey = (e: KeyboardEvent) => { if (e.key === "Escape") close(); };
    window.addEventListener("keydown", onKey);
    void (async () => {
      let saved = "";
      try { saved = localStorage.getItem(leagueKey()) ?? ""; } catch {}
      try {
        leagues = (await engine.tradeLeagues()).leagues;
      } catch {
        leagues = [{ id: "Standard", text: "Standard" }];
      }
      league = leagues.some((l) => l.id === saved) ? saved : (leagues[0]?.id ?? "Standard");
      void lookNinja();
      // Currency, gems and cards are poe.ninja's; the trade search reads equipment.
      if (item && !["Currency", "Gem", "Divination Card"].includes(item.rarity)) {
        try {
          info = await engine.buySimilarInfo({ raw });
          ({ mods, defences } = defaultChoices(info));
          baseType = !info.unique && info.rarity === "RARE";
          void search();
        } catch {
          unsearchable = true;
        }
      }
    })();
    return () => window.removeEventListener("keydown", onKey);
  });
</script>

<div class="price">
  <div class="head">
    <div class="title">
      <strong class="rarity {item?.rarity.toLowerCase()}">{item?.name ?? m.price_title()}</strong>
      {#if item?.baseType}<span class="dim">{item.baseType}</span>{/if}
      {#if info?.itemLevel || item?.corrupted}
        <span class="dim small">
          {#if info?.itemLevel}{m.price_item_level({ level: info.itemLevel })}{/if}
          {#if item?.corrupted} · <span class="bad">{m.price_corrupted()}</span>{/if}
        </span>
      {/if}
    </div>
    <button class="x" onclick={close} title={m.price_close()} aria-label={m.price_close()}>✕</button>
  </div>

  <label class="row small">
    <span class="dim">{m.price_league()}</span>
    <select class="select sm" value={league} onchange={(e) => setLeague((e.target as HTMLSelectElement).value)}>
      {#each leagues as l (l.id)}<option value={l.id}>{l.text}</option>{/each}
    </select>
  </label>

  {#if ninja === "loading"}
    <p class="dim small">{m.price_ninja_loading()}</p>
  {:else if ninja}
    {@const each = formatValue(ninja.line.value, ninja.table)}
    <div class="ninja">
      <span class="label small">poe.ninja</span>
      <strong class="value">{each}</strong>
      {#if item?.stack && item.stack > 1}
        <span class="dim small">{m.price_ninja_stack({ count: item.stack, total: formatValue(ninja.line.value * item.stack, ninja.table) })}</span>
      {/if}
      {#if ninja.line.variant}<span class="dim small">{ninja.line.variant}</span>{/if}
      {#if ninja.line.listings}<span class="dim small">{m.price_ninja_listed({ count: ninja.line.listings })}</span>{/if}
    </div>
  {:else if item && ninjaSources(game.current, item).length}
    <p class="dim small">{m.price_ninja_none()}</p>
  {/if}

  {#if info}
    <div class="trade">
      <span class="label small">{m.price_trade()}</span>
      {#if info.mods.length || info.defences.length}
        <div class="mods">
          {#each info.defences as d, i (`d${i}`)}
            <label class="mod" class:off={!defences[i].checked}>
              <input type="checkbox" bind:checked={defences[i].checked} />
              <span class="small">{d.label} {d.value}</span>
              <input class="input sm mono" bind:value={defences[i].min} aria-label={m.price_min()} />
            </label>
          {/each}
          {#each info.mods as mod, i (`m${i}`)}
            <label class="mod" class:off={!mods[i].checked} class:dead={!mod.searchable}>
              <input type="checkbox" bind:checked={mods[i].checked} disabled={!mod.searchable} />
              <span class="small">{mod.lines.map(plain).join(" / ")}</span>
              {#if mod.ranged}<input class="input sm mono" bind:value={mods[i].min} aria-label={m.price_min()} />{/if}
            </label>
          {/each}
        </div>
      {/if}
      <div class="row">
        {#if !info.unique}
          <label class="small"><input type="checkbox" bind:checked={baseType} /> {m.price_base_type()}</label>
        {/if}
        <button class="btn sm" onclick={() => void search()} disabled={searching}>{searching ? m.price_searching() : m.price_search()}</button>
      </div>
      {#if error}<p class="bad small">{error}</p>{/if}
      {#if result}
        {#if result.listings.length}
          <p class="dim small">{m.price_total({ count: result.total })}</p>
          <div class="listings">
            {#each result.listings as l, i (i)}
              <span class="mono" class:dim={!l.online}>{formatListing(l.amount, l.currency)}</span>
              <span class="dim small">{l.stack && l.stack > 1 ? `×${l.stack}` : ""}</span>
              <span class="dim small">{age(l.indexed)}{l.online ? "" : ` · ${m.price_offline()}`}</span>
            {/each}
          </div>
        {:else}
          <p class="dim small">{m.price_no_results()}</p>
        {/if}
        <button class="btn sm ghost" onclick={() => void invoke("trade_window_open", { url: result!.url })}>{m.price_open_trade()}</button>
      {/if}
    </div>
  {:else if unsearchable}
    <p class="dim small">{m.price_not_searchable()}</p>
  {/if}
</div>

<style>
  .price { display: flex; flex-direction: column; gap: 8px; user-select: text; }
  .head { display: flex; justify-content: space-between; align-items: flex-start; gap: 8px; }
  .title { display: flex; flex-direction: column; }
  .x { width: 22px; height: 22px; padding: 0; border: 0; border-radius: 5px; cursor: pointer; color: var(--fg-3); background: transparent; }
  .x:hover { color: var(--fg-0); background: color-mix(in oklch, var(--bad) 45%, transparent); }
  .row { display: flex; align-items: center; justify-content: space-between; gap: 8px; }
  .label { text-transform: uppercase; letter-spacing: 0.06em; color: var(--fg-3); font-size: 11px; }
  .ninja { display: flex; flex-wrap: wrap; align-items: baseline; gap: 4px 10px; padding: 8px 10px; border-radius: 8px;
    background: color-mix(in oklch, var(--brand) 12%, transparent); border: 1px solid color-mix(in oklch, var(--brand) 35%, transparent); }
  .ninja .label { flex-basis: 100%; }
  .value { font-size: 18px; color: var(--fg-0); font-variant-numeric: tabular-nums; }
  .trade { display: flex; flex-direction: column; gap: 6px; padding-top: 8px; border-top: 1px solid var(--line-1); }
  .mods { display: flex; flex-direction: column; gap: 2px; max-height: 260px; overflow-y: auto; }
  .mod { display: grid; grid-template-columns: auto 1fr auto; align-items: center; gap: 6px; padding: 2px 0; cursor: pointer; }
  .mod.off span { color: var(--fg-3); }
  .mod.dead { opacity: 0.5; cursor: default; }
  .mod .input { width: 52px; }
  .listings { display: grid; grid-template-columns: 1fr auto auto; gap: 2px 10px; font-variant-numeric: tabular-nums; }
  .dim { color: var(--fg-3); } .small { font-size: 12px; } .bad { color: var(--bad); }
  p { margin: 2px 0; }
  .rarity.unique { color: oklch(70% 0.12 60); } .rarity.rare { color: oklch(86% 0.13 95); } .rarity.magic { color: oklch(72% 0.12 260); }
  .rarity.gem { color: var(--c-gem); } .rarity.currency { color: var(--c-currency); }
</style>
