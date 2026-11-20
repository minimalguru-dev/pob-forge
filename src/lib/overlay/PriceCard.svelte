<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { engine, openUrl, type BuySimilarInfo } from "$lib/engine.svelte";
  import { game } from "$lib/state/game.svelte";
  import { m } from "$lib/paraglide/messages";
  import {
    age, applyFilters, defaultChoices, formatListing, formatValue, matchNinja, ninjaSources, parseCopied, poedbUrl,
    pseudoTotals, sparkPoints, wikiUrl, PSEUDO, defaultQuality, weaponChoices, itemMarks, mapStats, blight, type DpsChoice, type Mark, type MapStat, modTags, templeRooms, roomFilter, isTemple, isMap, simpleQuery, VALUABLE_ROOMS,
    type Choice, type NinjaLine, type NinjaTable, type PseudoTotal,
  } from "./price";

  let { raw, onclose }: { raw: string; onclose: () => void } = $props();

  interface Listing { amount: number | null; currency: string | null; indexed: string | null; account: string | null; online: boolean; stack: number | null }
  interface TradeResult { total: number; url: string; listings: Listing[] }
  interface BulkResult { total: number; have: string; want: string; url: string; offers: { each: number; stock: number | null; indexed: string | null }[] }

  const item = $derived(parseCopied(raw));
  const leagueKey = () => `pob-forge:price-league:${game.current}`;

  let leagues = $state<{ id: string; text: string }[]>([]);
  let league = $state("");
  let ninja = $state<{ line: NinjaLine; table: NinjaTable } | null | "loading">(null);
  let bulk = $state<BulkResult | null | "loading">(null);
  let info = $state<BuySimilarInfo | null>(null);
  let mods = $state<Choice[]>([]);
  let defences = $state<Choice[]>([]);
  let dps = $state<DpsChoice[]>([]);
  /** Influences, Fractured Item, Mirrored…: ticked, since they are part of what the item is. */
  let marks = $state<Mark[]>([]);
  let maps = $state<MapStat[]>([]);
  let blighted = $state(false);
  /** A Foulborn unique's mutated mods, as PoB read them (the game's copy does not mark them). */
  let mutatedLines = $state<string[]>([]);
  const foulborn = $derived(!!item && (item.mutated.length > 0 || mutatedLines.length > 0 || /^Foulborn /.test(item.name)));
  let pseudo = $state<(PseudoTotal & Choice)[]>([]);
  let baseType = $state(false);
  let listed = $state(1);
  let itemLevel = $state("");
  /** Item level is a toggle: off by default, its minimum the copied item's own. */
  let useItemLevel = $state(false);
  let quality = $state("");
  let links = $state("");
  let corrupted = $state<"any" | "yes" | "no">("any");
  let priceIn = $state("any");
  let emptyPrefix = $state("");
  let emptySuffix = $state("");
  /** Temples, gems, maps and anything Buy Similar cannot read get a search built here. */
  let simple = $state(false);
  let rooms = $state<{ name: string; id: string | null; checked: boolean }[]>([]);
  let gemLevel = $state("");
  let mapTier = $state("");
  let areaLevel = $state("");
  /** The base alone, as for crafting: mod filters are left out (not lost) while it is on. */
  let baseOnly = $state(false);
  let result = $state<TradeResult | null>(null);
  /** Every choice that changes the search; a change searches again by itself. */
  const choices = () =>
    JSON.stringify([
      simple, baseOnly, baseType, listed, useItemLevel, itemLevel, quality, links, corrupted, priceIn, emptyPrefix, emptySuffix,
      mods, defences, dps, pseudo, marks, rooms, gemLevel, mapTier, areaLevel, maps, blighted,
    ]);
  let searchedWith = "";
  let again = false;
  /** Changes made by typing or ticking search a moment after the last one. */
  const AUTO_SEARCH_MS = 500;
  $effect(() => {
    const now = choices();
    if (!(info || simple) || !league || now === searchedWith) return;
    // A search started meanwhile (the first one, a league change) may already have these choices.
    const timer = setTimeout(() => { if (choices() !== searchedWith) void search(); }, AUTO_SEARCH_MS);
    return () => clearTimeout(timer);
  });
  let searching = $state(false);
  let error = $state<string | null>(null);

  const friendly = (e: unknown) => String(e).split("\n")[0].replace(/^EngineError: [a-z_]+: /, "").replace(/^(runtime )?error: /i, "");
  /** PoB colours its lines with ^xRRGGBB and ^N codes. */
  const plain = (s: string) => s.replace(/\^x[0-9a-f]{6}|\^\d/gi, "");
  const num = (s: string) => (s.trim() === "" || Number.isNaN(Number(s)) ? null : Number(s));
  /** Mods counted in a ticked pseudo total are not searched on their own as well. */
  const inTotal = $derived(new Set(pseudo.filter((p) => p.checked).flatMap((p) => p.mods)));
  const currencies = $derived(game.isPoe1 ? ["any", "chaos", "divine"] : ["any", "exalted", "divine"]);
  const BULKABLE = new Set(["Currency", "Divination Card"]);

  async function lookNinja() {
    const it = item;
    if (!it || !league) return;
    const sources = ninjaSources(game.current, it);
    if (!sources.length) { ninja = null; return; }
    ninja = "loading";
    for (const source of sources) {
      try {
        const table = await invoke<NinjaTable>("price_ninja", { game: game.current, league, ...source });
        const line = matchNinja(it.mutated.length ? it : { ...it, mutated: mutatedLines }, table);
        if (line) { ninja = { line, table }; return; }
      } catch {
        // A table poe.ninja lacks for this league is not an error for the next one.
      }
    }
    ninja = null;
  }

  /** Stackable items: what online sellers ask on the currency exchange. */
  async function lookBulk() {
    const it = item;
    if (!it || !league || !(BULKABLE.has(it.rarity) || (it.stack ?? 0) > 1)) { bulk = null; return; }
    bulk = "loading";
    try {
      bulk = await invoke<BulkResult | null>("price_bulk", { game: game.current, league, name: it.name });
    } catch {
      bulk = null;
    }
  }

  async function searchSimple() {
    if (!item || !league) return;
    if (searching) { again = true; return; }
    searching = true;
    searchedWith = choices();
    error = null;
    try {
      const query = simpleQuery(item, {
        rooms: rooms.filter((r) => r.checked && r.id).map((r) => r.id!),
        gemLevel: num(gemLevel),
        quality: num(quality),
        mapTier: num(mapTier),
        corrupted,
        mapStats: maps.filter((s) => s.checked).map((s) => ({ filter: s.filter, min: num(s.min) })),
        blight: blighted && item ? blight(item) : null,
        areaLevel: num(areaLevel),
        game: game.current,
      });
      result = await invoke<TradeResult>("price_trade", { game: game.current, league, query });
    } catch (e) {
      error = friendly(e);
    } finally {
      searching = false;
      searchAgain();
    }
  }

  /** A change made while a search ran gets its own search once that one is back. */
  function searchAgain() {
    if (again && choices() !== searchedWith) void search();
    again = false;
  }

  /** The temple's rooms, each with its trade filter; the rooms players pay for are ticked. */
  async function loadRooms() {
    if (!item) return;
    const stats = await invoke<{ id: string; text: string }[]>("price_stats", { game: game.current, group: "pseudo" }).catch(() => []);
    rooms = templeRooms(raw).open.map((name) => ({ name, id: roomFilter(name, stats), checked: VALUABLE_ROOMS.some((re) => re.test(name)) }));
  }

  function startSimple() {
    if (!item) return;
    simple = true;
    corrupted = item.corrupted ? "yes" : "any";
    areaLevel = item.areaLevel !== null && !isMap(item) ? String(item.areaLevel) : "";
    if (item.rarity === "Gem") {
      gemLevel = item.gemLevel !== null ? String(item.gemLevel) : "";
      quality = item.gemQuality ? String(item.gemQuality) : "";
    } else {
      quality = defaultQuality(item);
    }
    if (isMap(item)) {
      if (item.mapTier !== null) mapTier = String(item.mapTier);
      maps = mapStats(game.current, raw);
      blighted = game.isPoe1 && blight(item) !== null;
    }
  }

  async function search() {
    if (simple) return searchSimple();
    if (!info || !league) return;
    if (searching) { again = true; return; }
    searching = true;
    searchedWith = choices();
    error = null;
    const modsOn = !baseOnly;
    try {
      const { query } = await engine.buySimilarUrl({
        raw,
        realm: info.realms[0],
        league,
        listed,
        baseType: baseType || baseOnly,
        ilvlMin: null,
        ilvlMax: null,
        defences: defences.map((d) => ({ checked: modsOn && d.checked, min: num(d.min), max: num(d.max) })),
        mods: mods.map((d, i) => ({ checked: modsOn && d.checked && !inTotal.has(i), min: num(d.min), max: num(d.max) })),
      });
      if (!query) throw new Error("this Path of Building version builds no trade query");
      const filtered = applyFilters(query, {
        pseudo: modsOn
          ? [
              ...pseudo.filter((p) => p.checked).map((p) => ({ id: PSEUDO[p.key].id, min: num(p.min) })),
              ...(num(emptyPrefix) !== null ? [{ id: "pseudo.pseudo_number_of_empty_prefix_mods", min: num(emptyPrefix) }] : []),
              ...(num(emptySuffix) !== null ? [{ id: "pseudo.pseudo_number_of_empty_suffix_mods", min: num(emptySuffix) }] : []),
            ]
          : [],
        dps: modsOn ? dps.filter((d) => d.checked).map((d) => ({ key: d.key, min: num(d.min) })) : [],
        marks: modsOn ? marks.filter((mk) => mk.checked) : [],
        game: game.current,
        itemLevel: useItemLevel ? num(itemLevel) : null,
        quality: num(quality),
        links: game.isPoe1 ? num(links) : null,
        corrupted,
        priceIn,
      });
      result = await invoke<TradeResult>("price_trade", { game: game.current, league, query: filtered });
    } catch (e) {
      error = friendly(e);
    } finally {
      searching = false;
      searchAgain();
    }
  }

  function setLeague(id: string) {
    league = id;
    try { localStorage.setItem(leagueKey(), id); } catch {}
    result = null;
    void lookNinja();
    void lookBulk();
    if (info || simple) void search();
  }

  function close() {
    void invoke("overlay_price_done").catch(() => {});
    onclose();
  }

  onMount(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") close();
      if (e.key === "Enter" && (info || simple)) void search();
    };
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
      void lookBulk();
      if (!item) return;
      if (isTemple(item)) {
        startSimple();
        await loadRooms();
        void search();
        return;
      }
      if (item.rarity === "Gem" || isMap(item)) {
        startSimple();
        void search();
        return;
      }
      // Currency and cards are poe.ninja's and the exchange's; equipment goes through Buy Similar.
      if (!["Currency", "Divination Card"].includes(item.rarity)) {
        try {
          info = await engine.buySimilarInfo({ raw });
          ({ mods, defences } = defaultChoices(info));
          dps = weaponChoices(info);
          quality = defaultQuality(item);
          marks = itemMarks(game.current, item);
          mutatedLines = info.mods.filter((mod) => mod.kinds?.includes("mutated")).flatMap((mod) => mod.lines.map(plain));
          if (mutatedLines.length) void lookNinja();
          // Crafted mods cost little to put back, so they are not searched at first. A Foulborn
          // unique's mutated mods are what it is priced by, so those are.
          const tagged = info.mods.map((mod) => modTags(raw, mod));
          mods = mods.map((c, i) => {
            if (tagged[i].includes("crafted")) return { ...c, checked: false };
            if (tagged[i].includes("mutated") && info!.mods[i].searchable) return { ...c, checked: true };
            return c;
          });
          // Awakened's default: totals for resistances and life, at 90% of the item's.
          pseudo = info.unique ? [] : pseudoTotals(info).map((p) => ({ ...p, checked: true, min: String(Math.floor(p.total * 0.9)), max: "" }));
          baseType = false;
          itemLevel = info.itemLevel ? String(info.itemLevel) : "";
          listed = Math.max(1, info.listed.indexOf("Instant Buyout & In Person") + 1);
          corrupted = item.corrupted ? "yes" : "any";
          if (game.isPoe1 && (item.links ?? 0) >= 5) links = String(item.links);
          void search();
        } catch {
          // Buy Similar cannot read it: search by its name and base instead.
          startSimple();
          void search();
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
      {#if info?.itemLevel || item?.corrupted || item?.quality || foulborn}
        <span class="dim small">
          {#if info?.itemLevel}{m.price_item_level({ level: info.itemLevel })}{/if}
          {#if item?.quality}{#if info?.itemLevel} · {/if}{m.price_item_quality({ quality: item.quality })}{/if}
          {#if item?.corrupted} · <span class="bad">{m.price_corrupted()}</span>{/if}
          {#if foulborn} · <span class="foul">Foulborn</span>{/if}
        </span>
      {/if}
    </div>
    <div class="tools">
      {#if item && wikiUrl(game.current, item)}
        <button class="btn sm ghost" onclick={() => void openUrl(wikiUrl(game.current, item!)!)} title={m.price_wiki_title()}>{m.price_wiki()}</button>
      {/if}
      {#if item}<button class="btn sm ghost" onclick={() => void openUrl(poedbUrl(game.current, item!))}>poedb</button>{/if}
      <button class="x" onclick={close} title={m.price_close()} aria-label={m.price_close()}>✕</button>
    </div>
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
    {@const line = ninja.line}
    {@const points = sparkPoints(line.trend, 84, 22)}
    <div class="ninja">
      <span class="label small">poe.ninja</span>
      <div class="ninja-row">
        <strong class="value">{formatValue(line.value, ninja.table)}</strong>
        {#if points}
          <svg class="spark" class:up={(line.change ?? 0) >= 0} viewBox="0 0 84 22" width="84" height="22" aria-hidden="true">
            <polyline points={points} fill="none" stroke="currentColor" stroke-width="1.5" stroke-linejoin="round" />
          </svg>
        {/if}
        {#if line.change !== null}
          <span class="small" class:good={line.change >= 0} class:bad={line.change < 0} title={m.price_change_title()}>
            {line.change >= 0 ? "+" : ""}{Math.round(line.change)}%
          </span>
        {/if}
      </div>
      <div class="ninja-meta dim small">
        {#if item?.stack && item.stack > 1}<span>{m.price_ninja_stack({ count: item.stack, total: formatValue(line.value * item.stack, ninja.table) })}</span>{/if}
        {#if line.variant}<span>{line.variant}</span>{/if}
        {#if line.listings}<span>{m.price_ninja_listed({ count: line.listings })}</span>{/if}
      </div>
    </div>
  {:else if item && ninjaSources(game.current, item).length}
    <p class="dim small">{m.price_ninja_none()}</p>
  {/if}

  {#if bulk === "loading"}
    <p class="dim small">{m.price_bulk_loading()}</p>
  {:else if bulk}
    {@const b = bulk}
    <div class="trade">
      <span class="label small">{m.price_bulk({ count: b.total })}</span>
      {#if b.offers.length}
        <div class="listings">
          {#each b.offers.slice(0, 5) as o, i (i)}
            <span class="mono">{formatListing(o.each, b.have)}</span>
            <span class="dim small">{o.stock ? m.price_bulk_stock({ count: o.stock }) : ""}</span>
            <span class="dim small">{age(o.indexed)}</span>
          {/each}
        </div>
      {:else}
        <p class="dim small">{m.price_no_results()}</p>
      {/if}
      <button class="btn sm ghost" onclick={() => void openUrl(b.url)}>{m.price_open_trade()}</button>
    </div>
  {/if}

  {#if info}
    <div class="trade">
      <span class="label small">{m.price_trade()}</span>
      <div class="mods" class:muted={baseOnly} inert={baseOnly}>
        {#each marks as mk, i (mk.line)}
          <label class="mod" class:off={!mk.checked}>
            <input type="checkbox" bind:checked={marks[i].checked} />
            <span class="small">{mk.line}</span>
          </label>
        {/each}
        {#each pseudo as p, i (p.key)}
          <label class="mod pseudo" class:off={!p.checked}>
            <input type="checkbox" bind:checked={pseudo[i].checked} />
            <span class="small">{PSEUDO[p.key].label.replace("#", String(p.total))}<span class="tags"><span class="tag pseudo">pseudo</span></span></span>
            <input class="input sm mono" bind:value={pseudo[i].min} aria-label={m.price_min()} />
          </label>
        {/each}
        {#each dps as d, i (d.key)}
          <label class="mod" class:off={!d.checked}>
            <input type="checkbox" bind:checked={dps[i].checked} />
            <span class="small">{d.key === "pdps" ? m.price_pdps() : m.price_edps()} {d.value}{#if info.tradeQuality}<span class="tags"><span class="tag q20" title={m.price_q20_title()}>Q20</span></span>{/if}</span>
            <input class="input sm mono" bind:value={dps[i].min} aria-label={m.price_min()} />
          </label>
        {/each}
        {#each info.defences as d, i (`d${i}`)}
          <label class="mod" class:off={!defences[i].checked}>
            <input type="checkbox" bind:checked={defences[i].checked} />
            <span class="small">{d.label} {d.value}{#if info.tradeQuality}<span class="tags"><span class="tag q20" title={m.price_q20_title()}>Q20</span></span>{/if}</span>
            <input class="input sm mono" bind:value={defences[i].min} aria-label={m.price_min()} />
          </label>
        {/each}
        {#each info.mods as mod, i (`m${i}`)}
          {@const counted = inTotal.has(i)}
          <label class="mod" class:off={!mods[i].checked || counted} class:dead={!mod.searchable || counted} title={counted ? m.price_in_total() : ""}>
            <input type="checkbox" bind:checked={mods[i].checked} disabled={!mod.searchable || counted} />
            <span class="small">
              {mod.lines.map(plain).join(" / ")}{#if counted}<span class="dim">&nbsp;· {m.price_in_total()}</span>{/if}
              {#if modTags(raw, mod).length}<span class="tags">{#each modTags(raw, mod) as tag (tag)}<span class="tag {tag}">{tag}</span>{/each}</span>{/if}
            </span>
            {#if mod.ranged && !counted}<input class="input sm mono" bind:value={mods[i].min} aria-label={m.price_min()} />{/if}
          </label>
        {/each}
      </div>

      <div class="filters small">
        <label><span class="dim">{m.price_quality()}</span><input class="input sm mono" bind:value={quality} placeholder="—" /></label>
        {#if game.isPoe1}
          <label><span class="dim">{m.price_links()}</span><input class="input sm mono" bind:value={links} placeholder="—" /></label>
        {/if}
        {#if !info.unique}
          <label><span class="dim">{m.price_empty_prefix()}</span><input class="input sm mono" bind:value={emptyPrefix} placeholder="—" /></label>
          <label><span class="dim">{m.price_empty_suffix()}</span><input class="input sm mono" bind:value={emptySuffix} placeholder="—" /></label>
        {/if}
        <label>
          <span class="dim">{m.price_corrupted_filter()}</span>
          <select class="select sm" bind:value={corrupted}>
            <option value="any">{m.price_any()}</option>
            <option value="yes">{m.price_yes()}</option>
            <option value="no">{m.price_no()}</option>
          </select>
        </label>
        <label>
          <span class="dim">{m.price_listed()}</span>
          <select class="select sm" onchange={(e) => (listed = Number((e.target as HTMLSelectElement).value))}>
            {#each info.listed as l, i (l)}<option value={i + 1} selected={listed === i + 1}>{l}</option>{/each}
          </select>
        </label>
        <label>
          <span class="dim">{m.price_in()}</span>
          <select class="select sm" bind:value={priceIn}>
            {#each currencies as c (c)}<option value={c}>{c === "any" ? m.price_any() : c}</option>{/each}
          </select>
        </label>
      </div>

      <div class="toggles small">
        {#if !info.unique}
          <label class="toggle" title={m.price_base_only_title()}><input type="checkbox" bind:checked={baseOnly} /> {m.price_base_only()}</label>
          <label class="toggle" class:off={baseOnly}><input type="checkbox" checked={baseType || baseOnly} disabled={baseOnly} onchange={(e) => (baseType = (e.target as HTMLInputElement).checked)} /> {m.price_base_type()}</label>
        {/if}
        <label class="toggle">
          <input type="checkbox" bind:checked={useItemLevel} /> {m.price_ilvl()} ≥
          <input class="input sm mono ilvl" bind:value={itemLevel} placeholder="—" disabled={!useItemLevel} />
        </label>
      </div>
      <div class="row">
        <span class="dim small">{searching ? m.price_searching() : ""}</span>
        <button class="btn sm ghost" onclick={() => void search()} disabled={searching} title={m.price_search_again_title()}>{m.price_search_again()}</button>
      </div>
      {#if error}<p class="bad small">{error}</p>{/if}
      {#if result}
        {#if result.listings.length}
          <p class="dim small">{m.price_total({ count: result.total })}</p>
          <div class="listings">
            {#each result.listings as l, i (i)}
              <span class="mono">{formatListing(l.amount, l.currency)}</span>
              <span class="dim small">{l.stack && l.stack > 1 ? `×${l.stack}` : ""}</span>
              <span class="dim small">{age(l.indexed)}</span>
            {/each}
          </div>
        {:else}
          <p class="dim small">{m.price_no_results()}</p>
        {/if}
        <button class="btn sm ghost" onclick={() => void openUrl(result!.url)}>{m.price_open_trade()}</button>
      {/if}
    </div>
  {:else if simple && item}
    <div class="trade">
      <span class="label small">{m.price_trade()}</span>
      {#if rooms.length}
        <div class="mods">
          {#each rooms as r, i (r.name)}
            <label class="mod" class:off={!r.checked} class:dead={!r.id}>
              <input type="checkbox" bind:checked={rooms[i].checked} disabled={!r.id} />
              <span class="small">{r.name}<span class="tags"><span class="tag pseudo">room</span></span></span>
            </label>
          {/each}
        </div>
      {/if}
      {#if maps.length || (blighted && blight(item))}
        <div class="mods">
          {#if blight(item) && game.isPoe1}
            <label class="mod" class:off={!blighted}>
              <input type="checkbox" bind:checked={blighted} />
              <span class="small">{blight(item) === "map_uberblighted" ? "Blight-ravaged" : "Blighted"}</span>
            </label>
          {/if}
          {#each maps as s, i (s.filter)}
            <label class="mod" class:off={!s.checked}>
              <input type="checkbox" bind:checked={maps[i].checked} />
              <span class="small">{s.label} {s.value}%</span>
              <input class="input sm mono" bind:value={maps[i].min} aria-label={m.price_min()} />
            </label>
          {/each}
        </div>
      {/if}
      <div class="filters small">
        {#if item.rarity === "Gem"}
          <label><span class="dim">{m.price_gem_level()}</span><input class="input sm mono" bind:value={gemLevel} placeholder="—" /></label>
        {/if}
        {#if isMap(item)}
          <label><span class="dim">{m.price_map_tier()}</span><input class="input sm mono" bind:value={mapTier} placeholder="—" /></label>
        {/if}
        {#if item.areaLevel !== null && !isMap(item)}
          <label><span class="dim">{m.price_area_level()}</span><input class="input sm mono" bind:value={areaLevel} placeholder="—" /></label>
        {/if}
        {#if !isTemple(item)}
          <label><span class="dim">{m.price_quality()}</span><input class="input sm mono" bind:value={quality} placeholder="—" /></label>
        {/if}
        <label>
          <span class="dim">{m.price_corrupted_filter()}</span>
          <select class="select sm" bind:value={corrupted}>
            <option value="any">{m.price_any()}</option>
            <option value="yes">{m.price_yes()}</option>
            <option value="no">{m.price_no()}</option>
          </select>
        </label>
      </div>
      <div class="row">
        <span class="dim small">{searching ? m.price_searching() : ""}</span>
        <button class="btn sm ghost" onclick={() => void search()} disabled={searching} title={m.price_search_again_title()}>{m.price_search_again()}</button>
      </div>
      {#if error}<p class="bad small">{error}</p>{/if}
      {#if result}
        {#if result.listings.length}
          <p class="dim small">{m.price_total({ count: result.total })}</p>
          <div class="listings">
            {#each result.listings as l, i (i)}
              <span class="mono">{formatListing(l.amount, l.currency)}</span>
              <span class="dim small">{l.stack && l.stack > 1 ? `×${l.stack}` : ""}</span>
              <span class="dim small">{age(l.indexed)}</span>
            {/each}
          </div>
        {:else}
          <p class="dim small">{m.price_no_results()}</p>
        {/if}
        <button class="btn sm ghost" onclick={() => void openUrl(result!.url)}>{m.price_open_trade()}</button>
      {/if}
    </div>
  {/if}
</div>

<style>
  .price { display: flex; flex-direction: column; gap: 8px; user-select: text; }
  .head { display: flex; justify-content: space-between; align-items: flex-start; gap: 8px; }
  .title { display: flex; flex-direction: column; min-width: 0; }
  .tools { display: flex; align-items: center; gap: 2px; flex: none; }
  .x { width: 22px; height: 22px; padding: 0; border: 0; border-radius: 5px; cursor: pointer; color: var(--fg-3); background: transparent; }
  .x:hover { color: var(--fg-0); background: color-mix(in oklch, var(--bad) 45%, transparent); }
  .row { display: flex; align-items: center; justify-content: space-between; gap: 8px; }
  .label { text-transform: uppercase; letter-spacing: 0.06em; color: var(--fg-3); font-size: 11px; }
  .ninja { display: flex; flex-direction: column; gap: 2px; padding: 8px 10px; border-radius: 8px;
    background: color-mix(in oklch, var(--brand) 12%, transparent); border: 1px solid color-mix(in oklch, var(--brand) 35%, transparent); }
  .ninja-row { display: flex; align-items: center; gap: 10px; }
  .ninja-meta { display: flex; flex-wrap: wrap; gap: 2px 10px; }
  .value { font-size: 18px; color: var(--fg-0); font-variant-numeric: tabular-nums; }
  .spark { color: var(--bad); } .spark.up { color: var(--ok); }
  .trade { display: flex; flex-direction: column; gap: 6px; padding-top: 8px; border-top: 1px solid var(--line-1); }
  .mods { display: flex; flex-direction: column; gap: 2px; }
  .mod { display: grid; grid-template-columns: auto 1fr auto; align-items: center; gap: 6px; padding: 2px 0; cursor: pointer; }
  .mod.pseudo span { color: var(--fg-0); }
  .mod.off span { color: var(--fg-3); }
  .mod.dead { opacity: 0.55; cursor: default; }
  .mod .input { width: 52px; }
  .filters { display: grid; grid-template-columns: repeat(3, 1fr); gap: 4px 8px; }
  .filters label { display: flex; flex-direction: column; gap: 2px; min-width: 0; }
  .filters .input, .filters .select { width: 100%; box-sizing: border-box; }
  .tags { display: inline-flex; flex-wrap: wrap; gap: 3px; margin-left: 6px; vertical-align: 1px; }
  .tag { font-size: 10px; line-height: 1; padding: 2px 4px; border-radius: 3px; color: var(--fg-2);
    background: color-mix(in oklch, var(--fg-3) 18%, transparent); text-transform: lowercase; }
  .foul { color: oklch(68% 0.16 350); }
  .tag.mutated { color: oklch(72% 0.15 350); background: color-mix(in oklch, oklch(60% 0.16 350) 18%, transparent); }
  .mods.muted { opacity: 0.4; }
  .toggle.off { opacity: 0.55; }
  .tag.q20 { color: var(--fg-1); background: var(--bg-3); }
  .tag.pseudo { color: var(--brand); background: color-mix(in oklch, var(--brand) 16%, transparent); }
  .tag.eldritch { color: oklch(72% 0.14 300); background: color-mix(in oklch, oklch(60% 0.15 300) 20%, transparent); }
  .tag.crafted { color: oklch(78% 0.08 230); } .tag.fractured { color: oklch(80% 0.12 85); }
  .tag.implicit, .tag.enchant { color: oklch(80% 0.06 250); }
  .toggles { display: flex; flex-wrap: wrap; align-items: center; gap: 4px 14px; }
  .toggle { display: inline-flex; align-items: center; gap: 5px; cursor: pointer; }
  .toggle .ilvl { width: 44px; }
  .listings { display: grid; grid-template-columns: 1fr auto auto; gap: 2px 10px; font-variant-numeric: tabular-nums; }
  .dim { color: var(--fg-3); } .small { font-size: 12px; } .bad { color: var(--bad); } .good { color: var(--ok); }
  p { margin: 2px 0; }
  .rarity.unique { color: oklch(70% 0.12 60); } .rarity.rare { color: oklch(86% 0.13 95); } .rarity.magic { color: oklch(72% 0.12 260); }
  .rarity.gem { color: var(--c-gem); } .rarity.currency { color: var(--c-currency); }
</style>
