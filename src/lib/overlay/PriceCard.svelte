<script lang="ts">
  import { onMount, untrack } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { engine, openUrl, type BuySimilarInfo } from "$lib/engine.svelte";
  import { game } from "$lib/state/game.svelte";
  import { m } from "$lib/paraglide/messages";
  import {
    age, applyFilters, defaultChoices, formatListing, formatValue, matchNinja, ninjaSources, parseCopied, poedbUrl,
    pseudoTotals, sparkPoints, wikiUrl, PSEUDO, defaultQuality, weaponChoices, itemMarks, presetMarks, itemLevelFilter, hasCraftingValue, likelyFinished, exclusions, searchRarity, mapStats, blight, type DpsChoice, type Mark, type MapStat, modTags, templeRooms, roomFilter, isTemple, isMap, simpleQuery, VALUABLE_ROOMS,
    isLogbook, logbookFactions, LOGBOOK_FACTIONS, isPricedBase, matchBase,
    type Choice, type NinjaLine, type NinjaTable, type PseudoTotal,
  } from "./price";
  import FilterChip from "./FilterChip.svelte";
  import { advancedMods, clock, findAdvanced, groupListings, inPrimary, rateWait, rollMin, stripAdvanced, wheelStep, type Listing } from "./advanced";
  import { overlay } from "$lib/state/overlay.svelte";

  let { raw, onclose }: { raw: string; onclose: () => void } = $props();

  interface TradeResult { total: number; url: string; listings: Listing[] }
  interface BulkResult { total: number; have: string; want: string; url: string; offers: { each: number; stock: number | null; indexed: string | null }[] }
  interface Prediction { min: number; max: number; currency: string; confidence: number }

  /** The price check copies with Ctrl+Alt+C: the same item with its mod tiers and roll ranges. */
  const text = $derived(stripAdvanced(raw));
  const adv = $derived(advancedMods(raw));
  const item = $derived(parseCopied(text));
  const range = $derived(overlay.settings?.statRange ?? 10);
  const leagueKey = () => `pob-forge:price-league:${game.current}`;

  let leagues = $state<{ id: string; text: string }[]>([]);
  let league = $state("");
  let ninja = $state<{ line: NinjaLine; table: NinjaTable } | null | "loading">(null);
  let bulk = $state<BulkResult | null | "loading">(null);
  let info = $state<BuySimilarInfo | null>(null);
  let mods = $state<Choice[]>([]);
  let defences = $state<Choice[]>([]);
  let dps = $state<DpsChoice[]>([]);
  /**
   * Influences, Fractured Item, Mirrored…: off in the usual search and on in the base search, as
   * Awakened PoE Trade's two presets start them. Each preset keeps its own choices.
   */
  let marks = $state<Mark[]>([]);
  let otherMarks: Mark[] = [];
  let otherLevel = false;
  let mode: "mods" | "base" = "mods";
  let maps = $state<MapStat[]>([]);
  let blighted = $state(false);
  /** A Foulborn unique's mutated mods, as PoB read them (the game's copy does not mark them). */
  let mutatedLines = $state<string[]>([]);
  const foulborn = $derived(!!item && (item.mutated.length > 0 || mutatedLines.length > 0 || /^Foulborn /.test(item.name)));
  let pseudo = $state<(PseudoTotal & Choice)[]>([]);
  let baseType = $state(false);
  let listed = $state(1);
  let itemLevel = $state("");
  let itemLevelMax = $state<number | null>(null);
  /**
   * Number filters are words to switch on and off, as in Awakened PoE Trade: item level starts
   * off at the copied item's own, quality and links on where they matter.
   */
  let useItemLevel = $state(false);
  let quality = $state("");
  let useQuality = $state(false);
  let links = $state("");
  let useLinks = $state(false);
  /** As in Awakened: "Corrupted" or "Not Corrupted", always part of the search. */
  let corrupted = $state<"any" | "yes" | "no">("no");
  let priceIn = $state("any");
  let emptyPrefix = $state("1");
  let useEmptyPrefix = $state(false);
  let emptySuffix = $state("1");
  let useEmptySuffix = $state(false);
  let useGemLevel = $state(false);
  let useMapTier = $state(false);
  let useAreaLevel = $state(false);
  /** The base's own price on poe.ninja (PoE1 normal, magic and rare gear), as Awakened shows it at the top. */
  let baseTrend = $state<{ line: NinjaLine; table: NinjaTable } | null>(null);
  /** Temples, gems, maps and anything Buy Similar cannot read get a search built here. */
  let simple = $state(false);
  let rooms = $state<{ name: string; id: string | null; checked: boolean }[]>([]);
  let gemLevel = $state("");
  let mapTier = $state("");
  let areaLevel = $state("");
  /**
   * Awakened's base item preset: the exact base, its item level and influences, and its fractured
   * mods; the other mod filters are left out (not lost) while it is on.
   */
  let baseOnly = $state(false);
  /** Awakened offers the base search for a base worth one that is not a finished item. */
  let offerBase = $state(false);
  /** Mods kept in the base search: a fractured mod is part of the base. */
  let baseMods = $state(new Set<number>());
  $effect(() => {
    const want = baseOnly ? "base" : "mods";
    if (want === mode) return;
    untrack(() => {
      const keep = { marks, level: useItemLevel };
      marks = otherMarks;
      useItemLevel = otherLevel;
      otherMarks = keep.marks;
      otherLevel = keep.level;
      mode = want;
    });
  });
  let factions = $state<{ name: string; id: string; checked: boolean }[]>([]);
  let result = $state<TradeResult | null>(null);
  const rows = $derived(result ? groupListings(result.listings) : []);
  /** The league's currency rates, to read every listing in one currency. */
  let rates = $state<{ primary: string; rates: Record<string, number> } | null>(null);
  const spread = $derived.by(() => {
    const prices = rows.map((r) => inPrimary(r.amount, r.currency, rates)).filter((p): p is number => p !== null).sort((a, b) => a - b);
    if (prices.length < 3 || !rates) return null;
    return { low: formatListing(prices[0], rates.primary), mid: formatListing(prices[Math.floor(prices.length / 2)], rates.primary) };
  });
  let prediction = $state<Prediction | null | "loading">(null);
  /** The trade site's wait, counted down; a search asked for meanwhile runs when it is over. */
  let waitUntil = $state(0);
  let now = $state(Date.now());
  let pendingSearch = false;
  const waiting = $derived(waitUntil > now);
  $effect(() => {
    if (!waitUntil) return;
    const tick = window.setInterval(() => {
      now = Date.now();
      if (now >= waitUntil) {
        waitUntil = 0;
        clearInterval(tick);
        if (pendingSearch) {
          pendingSearch = false;
          void search();
        }
      }
    }, 250);
    return () => clearInterval(tick);
  });
  /** A search error, or the wait it asks for. */
  function failed(e: unknown) {
    const wait = rateWait(String(e));
    if (wait !== null) {
      waitUntil = Date.now() + wait * 1000;
      now = Date.now();
      pendingSearch = true;
      error = null;
    } else {
      error = friendly(e);
    }
  }
  /** Each PoB mod row's advanced-copy mod, for its tier and roll range. */
  const advFor = $derived(info ? info.mods.map((mod) => findAdvanced(adv, mod.lines)) : []);
  /**
   * As in Awakened PoE Trade: changing a mod, a minimum or a filter does not search; the results
   * go stale and Search (or Enter) asks again. Choices that make it a different search altogether
   * (Base only, same base type, the listing kind, the currency) search at once, as a league does.
   */
  const whole = () => JSON.stringify([simple, baseOnly, baseType, listed, priceIn]);
  const choices = () =>
    JSON.stringify([
      whole(), useItemLevel, itemLevel, useQuality, quality, useLinks, links, corrupted, useEmptyPrefix, emptyPrefix,
      useEmptySuffix, emptySuffix, mods, defences, dps, pseudo, marks, rooms, useGemLevel, gemLevel, useMapTier, mapTier,
      useAreaLevel, areaLevel, maps, blighted, factions,
    ]);
  /** A number filter's value when its word is on. */
  const when = (on: boolean, value: string) => (on ? num(value) : null);
  let searchedWith = $state("");
  let searchedWhole = "";
  let again = false;
  $effect(() => {
    const now = whole();
    if (!(info || simple) || !league || !searchedWhole || now === searchedWhole) return;
    void search();
  });
  let searching = $state(false);
  /** The filters changed since the listings shown were found. */
  const outdated = $derived(!!result && !searching && choices() !== searchedWith);
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
    if (waiting) { pendingSearch = true; return; }
    if (searching) { again = true; return; }
    searching = true;
    searchedWith = choices();
    searchedWhole = whole();
    error = null;
    try {
      const query = simpleQuery(item, {
        rooms: rooms.filter((r) => r.checked && r.id).map((r) => r.id!),
        gemLevel: when(useGemLevel, gemLevel),
        quality: when(useQuality, quality),
        mapTier: when(useMapTier, mapTier),
        corrupted,
        mapStats: maps.filter((s) => s.checked).map((s) => ({ filter: s.filter, min: num(s.min) })),
        blight: blighted && item ? blight(item) : null,
        areaLevel: when(useAreaLevel, areaLevel),
        factions: factions.filter((f) => f.checked).map((f) => f.id),
        game: game.current,
      });
      result = await invoke<TradeResult>("price_trade", { game: game.current, league, query });
    } catch (e) {
      failed(e);
    } finally {
      searching = false;
      searchAgain();
    }
  }

  /** A change made while a search ran gets its own search once that one is back. */
  function searchAgain() {
    if (again && whole() !== searchedWhole) void search();
    again = false;
  }

  /** The temple's rooms, each with its trade filter; the rooms players pay for are ticked. */
  async function loadRooms() {
    if (!item) return;
    const stats = await invoke<{ id: string; text: string }[]>("price_stats", { game: game.current, group: "pseudo" }).catch(() => []);
    rooms = templeRooms(text).open.map((name) => ({ name, id: roomFilter(name, stats), checked: VALUABLE_ROOMS.some((re) => re.test(name)) }));
  }

  function startSimple() {
    if (!item) return;
    simple = true;
    corrupted = item.corrupted ? "yes" : "no";
    areaLevel = item.areaLevel !== null && !isMap(item) ? String(item.areaLevel) : "";
    useAreaLevel = areaLevel !== "";
    if (item.rarity === "Gem") {
      gemLevel = item.gemLevel !== null ? String(item.gemLevel) : "";
      useGemLevel = gemLevel !== "";
      quality = item.gemQuality ? String(item.gemQuality) : "";
    } else {
      quality = defaultQuality(item);
    }
    useQuality = quality !== "";
    if (isMap(item)) {
      if (item.mapTier !== null) mapTier = String(item.mapTier);
      useMapTier = mapTier !== "";
      maps = mapStats(game.current, text);
      blighted = game.isPoe1 && blight(item) !== null;
    }
    if (isLogbook(item) && game.isPoe1) {
      factions = logbookFactions(text).map((name) => ({ name, id: LOGBOOK_FACTIONS[name], checked: true }));
    }
  }

  async function search() {
    if (simple) return searchSimple();
    if (!info || !league) return;
    if (waiting) { pendingSearch = true; return; }
    if (searching) { again = true; return; }
    searching = true;
    searchedWith = choices();
    searchedWhole = whole();
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
        mods: mods.map((d, i) => ({ checked: (modsOn || baseMods.has(i)) && d.checked && !inTotal.has(i), min: num(d.min), max: num(d.max) })),
      });
      if (!query) throw new Error("this Path of Building version builds no trade query");
      const filtered = applyFilters(query, {
        pseudo: modsOn
          ? [
              ...pseudo.filter((p) => p.checked).map((p) => ({ id: PSEUDO[p.key].id, min: num(p.min), max: num(p.max) })),
              ...(when(useEmptyPrefix, emptyPrefix) !== null ? [{ id: "pseudo.pseudo_number_of_empty_prefix_mods", min: num(emptyPrefix) }] : []),
              ...(when(useEmptySuffix, emptySuffix) !== null ? [{ id: "pseudo.pseudo_number_of_empty_suffix_mods", min: num(emptySuffix) }] : []),
            ]
          : [],
        dps: modsOn ? dps.filter((d) => d.checked).map((d) => ({ key: d.key, min: num(d.min), max: num(d.max) })) : [],
        marks: marks.filter((mk) => mk.checked),
        game: game.current,
        itemLevel: when(useItemLevel, itemLevel),
        itemLevelMax: useItemLevel ? itemLevelMax : null,
        exclude: item ? exclusions(game.current, item, { base: baseOnly, league }) : [],
        rarity: item ? searchRarity(item) : null,
        quality: when(useQuality, quality),
        links: game.isPoe1 ? when(useLinks, links) : null,
        corrupted,
        priceIn,
      });
      result = await invoke<TradeResult>("price_trade", { game: game.current, league, query: filtered });
    } catch (e) {
      failed(e);
    } finally {
      searching = false;
      searchAgain();
    }
  }

  /** The base's own line in poe.ninja's BaseType table, once Buy Similar has named the base and item level. */
  async function lookBase() {
    const it = item;
    baseTrend = null;
    const base = info?.baseName ?? it?.baseType;
    if (!it || !league || !game.isPoe1 || !base || !isPricedBase(it)) return;
    try {
      const table = await invoke<NinjaTable>("price_ninja", { game: game.current, league, kind: "stash", category: "BaseType" });
      const line = matchBase(table, it, base, info?.itemLevel ?? null);
      baseTrend = line ? { line, table } : null;
    } catch {
      baseTrend = null;
    }
  }

  /** The league's currency table from poe.ninja: its rates turn any listing into one currency. */
  async function lookRates() {
    if (!league) return;
    try {
      const table = await invoke<NinjaTable>("price_ninja", { game: game.current, league, kind: "exchange", category: "Currency" });
      rates = { primary: table.primary, rates: table.rates };
    } catch {
      rates = null;
    }
  }

  /** poeprices.info's estimate, for rares only, and only when the user turned it on. */
  let predictedFor = "";
  // Settings can arrive after the card opened.
  $effect(() => {
    if (overlay.settings?.pricePrediction && league && predictedFor !== league) void lookPrediction();
  });
  async function lookPrediction() {
    const it = item;
    predictedFor = league;
    prediction = null;
    if (!overlay.settings?.pricePrediction || !it || !league || it.rarity !== "Rare" || it.unidentified || isMap(it) || isLogbook(it)) return;
    prediction = "loading";
    try {
      prediction = await invoke<Prediction>("price_predict", { league, text });
    } catch {
      prediction = null;
    }
  }

  function setLeague(id: string) {
    league = id;
    try { localStorage.setItem(leagueKey(), id); } catch {}
    result = null;
    void lookNinja();
    void lookBulk();
    void lookRates();
    void lookPrediction();
    if (info) void lookBase();
    if (info || simple) void search();
  }

  /** The mouse wheel nudges a number field: one step a notch, ten with Shift. */
  function onWheel(e: WheelEvent) {
    const input = e.target;
    if (!(input instanceof HTMLInputElement) || !input.classList.contains("mono") || input.disabled) return;
    e.preventDefault();
    input.value = wheelStep(input.value, e.deltaY < 0, e.shiftKey);
    input.dispatchEvent(new Event("input", { bubbles: true }));
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
      void lookRates();
      void lookPrediction();
      if (!item) return;
      // Unidentified uniques, logbooks and the like are searched by their base here.
      if (item.unidentified || isLogbook(item)) {
        startSimple();
        void search();
        return;
      }
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
          const rolls = info.mods.map((mod) => findAdvanced(adv, mod.lines)?.lines[0]?.rolls[0] ?? null);
          ({ mods, defences } = defaultChoices(info, range, rolls));
          dps = weaponChoices(info, range);
          quality = defaultQuality(item);
          useQuality = quality !== "";
          const offered = itemMarks(game.current, item);
          void lookBase();
          mutatedLines = info.mods.filter((mod) => mod.kinds?.includes("mutated")).flatMap((mod) => mod.lines.map(plain));
          if (mutatedLines.length) void lookNinja();
          // Crafted mods cost little to put back, so they are not searched at first. A Foulborn
          // unique's mutated mods are what it is priced by, so those are.
          const tagged = info.mods.map((mod) => modTags(text, mod));
          mods = mods.map((c, i) => {
            if (tagged[i].includes("crafted")) return { ...c, checked: false };
            if (tagged[i].includes("mutated") && info!.mods[i].searchable) return { ...c, checked: true };
            return c;
          });
          // Awakened's default: totals for resistances and life, the roll range under the item's.
          pseudo = info.unique ? [] : pseudoTotals(info).map((p) => ({ ...p, checked: true, min: String(rollMin(p.total, range)), max: "" }));
          // As in Awakened PoE Trade: a rare is searched across its category until the base is
          // clicked; a cluster jewel is its base (and item level) from the start.
          const cluster = /Cluster Jewel$/.test(info.baseName ?? "");
          baseType = cluster;
          if (info.itemLevel) {
            const level = itemLevelFilter(info.itemLevel, cluster);
            itemLevel = String(level.min);
            itemLevelMax = level.max;
          } else {
            itemLevel = "";
            itemLevelMax = null;
          }
          // Awakened's two presets: the usual one, and the base search for a base worth one.
          // Flasks and tinctures never search by item level.
          const levelMatters = !!info.itemLevel && !/Flasks?$|Tinctures?$/.test(item.itemClass);
          baseMods = new Set(tagged.flatMap((tags, i) => (tags.includes("fractured") ? [i] : [])));
          offerBase = !info.unique && (item.rarity === "Normal" || (hasCraftingValue(item, info.itemLevel ?? null) && !likelyFinished(item, tagged.some((t) => t.includes("crafted")))));
          const startBase = item.rarity === "Normal";
          marks = presetMarks(offered, startBase);
          otherMarks = presetMarks(offered, !startBase);
          useItemLevel = cluster || (startBase && levelMatters);
          otherLevel = cluster || (!startBase && levelMatters);
          mode = startBase ? "base" : "mods";
          baseOnly = startBase;
          listed = Math.max(1, info.listed.indexOf("Instant Buyout & In Person") + 1);
          corrupted = item.corrupted ? "yes" : "no";
          if (game.isPoe1 && item.links) {
            links = String(item.links);
            useLinks = item.links >= 5;
          }
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

<div class="price" onwheel={onWheel}>
  <div class="head">
    <div class="title">
      <strong class="rarity {item?.rarity.toLowerCase()}">{item?.name ?? m.price_title()}</strong>
      {#if info && !info.unique && info.baseName && info.category}
        <!-- As in Awakened PoE Trade: one button that is the search's base, the category until clicked. -->
        <button class="basebtn" class:exact={baseType || baseOnly} disabled={baseOnly} title={m.price_base_toggle_title()} onclick={() => (baseType = !baseType)}>
          {baseType || baseOnly ? info.baseName : info.category}
        </button>
      {:else if item?.baseType}<span class="dim">{item.baseType}</span>{/if}
      {#if item?.unidentified}<span class="dim small">{m.price_unidentified()}</span>{/if}
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

  {#if baseTrend}
    <!-- As in Awakened PoE Trade: the base's own market at the top, for normal, magic and rare gear. -->
    {@const line = baseTrend.line}
    {@const points = sparkPoints(line.trend, 84, 22)}
    <div class="ninja">
      <span class="label small">{m.price_base_item()} · {line.name} · {line.levelRequired === 86 ? "86+" : line.levelRequired}{line.variant ? ` · ${line.variant}` : ""}</span>
      <div class="ninja-row">
        <strong class="value">{formatValue(line.value, baseTrend.table)}</strong>
        {#if points}
          <svg class="spark" class:up={(line.change ?? 0) >= 0} viewBox="0 0 84 22" width="84" height="22" aria-hidden="true">
            <polyline points={points} fill="none" stroke="currentColor" stroke-width="1.5" stroke-linejoin="round" />
          </svg>
        {/if}
        {#if line.change !== null && points}
          <span class="small" class:good={line.change >= 0} class:bad={line.change < 0} title={m.price_change_title()}>
            {line.change >= 0 ? "+" : ""}{Math.round(line.change)}%
          </span>
        {/if}
      </div>
      {#if line.listings}<div class="ninja-meta dim small"><span>{m.price_ninja_listed({ count: line.listings })}</span></div>{/if}
    </div>
  {/if}

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

  {#if prediction === "loading"}
    <p class="dim small">{m.price_predicted_loading()}</p>
  {:else if prediction}
    {@const p = prediction}
    <div class="ninja">
      <span class="label small">{m.price_predicted()}</span>
      <div class="ninja-row">
        <strong class="value">{formatListing(p.min, p.currency)}{p.max > p.min ? ` – ${formatListing(p.max, p.currency)}` : ""}</strong>
        <span class="small" class:warn={p.confidence < 78} title={p.confidence < 78 ? m.price_confidence_low() : ""}>{m.price_confidence({ value: Math.round(p.confidence) })}</span>
      </div>
    </div>
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
      <!-- As in Awakened PoE Trade: the item's own filters are words, not checkboxes. -->
      <div class="chips">
        {#if game.isPoe1 && item?.links}<FilterChip numeric label={m.price_chip_links()} bind:on={useLinks} bind:value={links} />{/if}
        <FilterChip numeric label={m.price_chip_item_level()} bind:on={useItemLevel} bind:value={itemLevel} />
        <FilterChip numeric label={m.price_chip_quality()} bind:on={useQuality} bind:value={quality} />
        {#if !info.unique}
          <FilterChip numeric label={m.price_chip_empty_prefixes()} bind:on={useEmptyPrefix} bind:value={emptyPrefix} disabled={baseOnly} />
          <FilterChip numeric label={m.price_chip_empty_suffixes()} bind:on={useEmptySuffix} bind:value={emptySuffix} disabled={baseOnly} />
        {/if}
        {#each marks as mk, i (mk.line)}
          <FilterChip label={mk.line.replace(/ Item$/, "")} bind:on={marks[i].checked} />
        {/each}
        {@render corruptChip()}
        {#if offerBase}<FilterChip label={m.price_base_only()} title={m.price_base_only_title()} bind:on={baseOnly} />{/if}
      </div>
      <div class="mods">
        <div class="part" class:muted={baseOnly} inert={baseOnly}>
        {#each pseudo as p, i (p.key)}
          <label class="mod pseudo" class:off={!p.checked}>
            <input type="checkbox" bind:checked={pseudo[i].checked} />
            <span class="small">{PSEUDO[p.key].label.replace("#", String(p.total))}<span class="tags"><span class="tag pseudo">pseudo</span></span></span>
            <span class="minmax"><input class="input sm mono" bind:value={pseudo[i].min} aria-label={m.price_min()} /><input class="input sm mono" bind:value={pseudo[i].max} aria-label={m.price_max()} placeholder="max" /></span>
          </label>
        {/each}
        {#each dps as d, i (d.key)}
          <label class="mod" class:off={!d.checked}>
            <input type="checkbox" bind:checked={dps[i].checked} />
            <span class="small">{d.key === "pdps" ? m.price_pdps() : m.price_edps()} {d.value}{#if info.tradeQuality}<span class="tags"><span class="tag q20" title={m.price_q20_title()}>Q20</span></span>{/if}</span>
            <span class="minmax"><input class="input sm mono" bind:value={dps[i].min} aria-label={m.price_min()} /><input class="input sm mono" bind:value={dps[i].max} aria-label={m.price_max()} placeholder="max" /></span>
          </label>
        {/each}
        {#each info.defences as d, i (`d${i}`)}
          <label class="mod" class:off={!defences[i].checked}>
            <input type="checkbox" bind:checked={defences[i].checked} />
            <span class="small">{d.label} {d.value}{#if info.tradeQuality}<span class="tags"><span class="tag q20" title={m.price_q20_title()}>Q20</span></span>{/if}</span>
            <span class="minmax"><input class="input sm mono" bind:value={defences[i].min} aria-label={m.price_min()} /><input class="input sm mono" bind:value={defences[i].max} aria-label={m.price_max()} placeholder="max" /></span>
          </label>
        {/each}
        </div>
        {#each info.mods as mod, i (`m${i}`)}
          {@const counted = inTotal.has(i)}
          {@const parked = baseOnly && !baseMods.has(i)}
          <label class="mod" class:muted={parked} inert={parked} class:off={!mods[i].checked || counted} class:dead={!mod.searchable || counted} title={counted ? m.price_in_total() : ""}>
            <input type="checkbox" bind:checked={mods[i].checked} disabled={!mod.searchable || counted} />
            <span class="small">
              {mod.lines.map(plain).join(" / ")}{#if counted}<span class="dim">&nbsp;· {m.price_in_total()}</span>{/if}
              {#if modTags(text, mod).length || advFor[i]?.tier}<span class="tags">
                {#if advFor[i]?.tier}{@const tier = advFor[i]!.tier!}<span class="tier" class:t1={tier === 1} class:t2={tier === 2} title={m.price_tier_title({ tier })}>T{tier}</span>{/if}
                {#each modTags(text, mod) as tag (tag)}<span class="tag {tag}">{tag}</span>{/each}
              </span>{/if}
            </span>
            {#if mod.ranged && !counted}<span class="minmax"><input class="input sm mono" bind:value={mods[i].min} aria-label={m.price_min()} /><input class="input sm mono" bind:value={mods[i].max} aria-label={m.price_max()} placeholder="max" /></span>{/if}
          </label>
        {/each}
      </div>

      <div class="filters small">
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

      <div class="row">
        <span class="small" class:dim={!outdated && !waiting} class:warn={outdated || waiting} title={waiting ? m.price_wait_title() : ""}>{searching ? m.price_searching() : waiting ? m.price_wait({ time: clock((waitUntil - now) / 1000) }) : outdated ? m.price_outdated() : ""}</span>
        {#if outdated}
          <!-- As in Awakened PoE Trade: moving onto Search is enough, no click needed. -->
          <button class="btn primary searchbtn" onclick={() => void search()} onmouseenter={() => void search()}>{m.price_search()}</button>
        {:else}
          <button class="btn sm ghost" onclick={() => void search()} disabled={searching} title={m.price_search_again_title()}>{m.price_search_again()}</button>
        {/if}
      </div>
      {#if error}<p class="bad small">{error}</p>{/if}
      {#if result}
        <div class="results" class:stale={outdated}>
        {#if result.listings.length}
          <p class="dim small">{spread ? m.price_spread({ count: result.total, ...spread }) : m.price_total({ count: result.total })}</p>
          <div class="listings">
            {#each rows as l, i (i)}
              {@const conv = rates && l.currency !== rates.primary ? inPrimary(l.amount, l.currency, rates) : null}
              <span class="mono">{formatListing(l.amount, l.currency)}{#if conv !== null && rates}<span class="dim small conv">{m.price_in_primary({ price: formatListing(conv, rates.primary) })}</span>{/if}</span>
              <span class="dim small">{l.stack && l.stack > 1 ? `×${l.stack}` : l.times > 1 ? m.price_times({ count: l.times }) : ""}</span>
              <span class="dim small">{age(l.indexed)}</span>
            {/each}
          </div>
        {:else}
          <p class="dim small">{m.price_no_results()}</p>
        {/if}
        <button class="btn sm ghost" onclick={() => void openUrl(result!.url)}>{m.price_open_trade()}</button>
        </div>
      {/if}
    </div>
  {:else if simple && item}
    <div class="trade">
      <span class="label small">{m.price_trade()}</span>
      {#if factions.length}
        <div class="mods">
          {#each factions as f, i (f.name)}
            <label class="mod" class:off={!f.checked}>
              <input type="checkbox" bind:checked={factions[i].checked} />
              <span class="small">{f.name}<span class="tags"><span class="tag pseudo">{m.price_faction()}</span></span></span>
            </label>
          {/each}
        </div>
      {/if}
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
      <div class="chips">
        {#if item.rarity === "Gem"}<FilterChip numeric label={m.price_chip_gem_level()} bind:on={useGemLevel} bind:value={gemLevel} />{/if}
        {#if isMap(item)}<FilterChip numeric label={m.price_chip_map_tier()} bind:on={useMapTier} bind:value={mapTier} />{/if}
        {#if item.areaLevel !== null && !isMap(item)}<FilterChip numeric label={m.price_chip_area_level()} bind:on={useAreaLevel} bind:value={areaLevel} />{/if}
        {#if !isTemple(item) && !item.unidentified}<FilterChip numeric label={m.price_chip_quality()} bind:on={useQuality} bind:value={quality} />{/if}
        {#if blight(item) && game.isPoe1}<FilterChip label={blight(item) === "map_uberblighted" ? "Blight-ravaged" : "Blighted"} bind:on={blighted} />{/if}
        {@render corruptChip()}
      </div>
      {#if maps.length}
        <div class="mods">
          {#each maps as s, i (s.filter)}
            <label class="mod" class:off={!s.checked}>
              <input type="checkbox" bind:checked={maps[i].checked} />
              <span class="small">{s.label} {s.value}%</span>
              <input class="input sm mono" bind:value={maps[i].min} aria-label={m.price_min()} />
            </label>
          {/each}
        </div>
      {/if}
      <div class="row">
        <span class="small" class:dim={!outdated && !waiting} class:warn={outdated || waiting} title={waiting ? m.price_wait_title() : ""}>{searching ? m.price_searching() : waiting ? m.price_wait({ time: clock((waitUntil - now) / 1000) }) : outdated ? m.price_outdated() : ""}</span>
        {#if outdated}
          <!-- As in Awakened PoE Trade: moving onto Search is enough, no click needed. -->
          <button class="btn primary searchbtn" onclick={() => void search()} onmouseenter={() => void search()}>{m.price_search()}</button>
        {:else}
          <button class="btn sm ghost" onclick={() => void search()} disabled={searching} title={m.price_search_again_title()}>{m.price_search_again()}</button>
        {/if}
      </div>
      {#if error}<p class="bad small">{error}</p>{/if}
      {#if result}
        <div class="results" class:stale={outdated}>
        {#if result.listings.length}
          <p class="dim small">{spread ? m.price_spread({ count: result.total, ...spread }) : m.price_total({ count: result.total })}</p>
          <div class="listings">
            {#each rows as l, i (i)}
              {@const conv = rates && l.currency !== rates.primary ? inPrimary(l.amount, l.currency, rates) : null}
              <span class="mono">{formatListing(l.amount, l.currency)}{#if conv !== null && rates}<span class="dim small conv">{m.price_in_primary({ price: formatListing(conv, rates.primary) })}</span>{/if}</span>
              <span class="dim small">{l.stack && l.stack > 1 ? `×${l.stack}` : l.times > 1 ? m.price_times({ count: l.times }) : ""}</span>
              <span class="dim small">{age(l.indexed)}</span>
            {/each}
          </div>
        {:else}
          <p class="dim small">{m.price_no_results()}</p>
        {/if}
        <button class="btn sm ghost" onclick={() => void openUrl(result!.url)}>{m.price_open_trade()}</button>
        </div>
      {/if}
    </div>
  {/if}
  {#snippet corruptChip()}
    <!-- As in Awakened PoE Trade: Corrupted or Not Corrupted, always searched; a click flips it. -->
    <FilterChip
      label={corrupted === "yes" ? m.price_chip_corrupted() : m.price_chip_not_corrupted()}
      on={corrupted === "yes"}
      tone="bad"
      onclick={() => (corrupted = corrupted === "yes" ? "no" : "yes")}
    />
  {/snippet}
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
  .minmax { display: inline-flex; gap: 3px; }
  .minmax .input { width: 46px; }
  .basebtn { align-self: flex-start; margin-top: 1px; padding: 1px 7px; border: 1px dashed var(--line-2); border-radius: 4px;
    background: transparent; color: var(--fg-2); font: inherit; font-size: 12px; cursor: pointer; }
  .basebtn:hover:not(:disabled) { color: var(--fg-0); border-color: var(--fg-3); }
  .basebtn.exact { border-style: solid; color: var(--fg-0); border-color: var(--brand); }
  .basebtn:disabled { cursor: default; }
  /* Awakened PoE Trade's tier marks: T1 filled, T2 outlined, the rest grey. */
  .tier { font-size: 10px; line-height: 1; padding: 2px 4px; border-radius: 3px; font-weight: 600; color: #111; background: oklch(55% 0.01 80); }
  .tier.t1 { background: oklch(83% 0.16 90); color: #111; }
  .tier.t2 { background: transparent; color: oklch(83% 0.16 90); box-shadow: inset 0 0 0 1px oklch(83% 0.16 90); }
  .conv { margin-left: 6px; }
  .filters { display: grid; grid-template-columns: repeat(2, 1fr); gap: 4px 8px; }
  .filters label { display: flex; flex-direction: column; gap: 2px; min-width: 0; }
  .filters .select { width: 100%; box-sizing: border-box; }
  .tags { display: inline-flex; flex-wrap: wrap; gap: 3px; margin-left: 6px; vertical-align: 1px; }
  .tag { font-size: 10px; line-height: 1; padding: 2px 4px; border-radius: 3px; color: var(--fg-2);
    background: color-mix(in oklch, var(--fg-3) 18%, transparent); text-transform: lowercase; }
  .foul { color: oklch(68% 0.16 350); }
  .tag.mutated { color: oklch(72% 0.15 350); background: color-mix(in oklch, oklch(60% 0.16 350) 18%, transparent); }
  .part { display: flex; flex-direction: column; gap: 2px; }
  .part.muted, .mod.muted { opacity: 0.4; }
  .results { display: contents; }
  .results.stale { display: block; opacity: 0.4; }
  .warn { color: var(--warn); }
  .searchbtn { min-width: 112px; height: 30px; padding: 0 18px; font-size: var(--fs-md, 14px); font-weight: 600; }
  .tag.q20 { color: var(--fg-1); background: var(--bg-3); }
  .tag.pseudo { color: var(--brand); background: color-mix(in oklch, var(--brand) 16%, transparent); }
  .tag.eldritch { color: oklch(72% 0.14 300); background: color-mix(in oklch, oklch(60% 0.15 300) 20%, transparent); }
  .tag.crafted { color: oklch(78% 0.08 230); } .tag.fractured { color: oklch(80% 0.12 85); }
  .tag.implicit, .tag.enchant { color: oklch(80% 0.06 250); }
  .chips { display: flex; flex-wrap: wrap; gap: 4px; }
  .listings { display: grid; grid-template-columns: 1fr auto auto; gap: 2px 10px; font-variant-numeric: tabular-nums; }
  .dim { color: var(--fg-3); } .small { font-size: 12px; } .bad { color: var(--bad); } .good { color: var(--ok); }
  p { margin: 2px 0; }
  .rarity.unique { color: oklch(70% 0.12 60); } .rarity.rare { color: oklch(86% 0.13 95); } .rarity.magic { color: oklch(72% 0.12 260); }
  .rarity.gem { color: var(--c-gem); } .rarity.currency { color: var(--c-currency); }
</style>
