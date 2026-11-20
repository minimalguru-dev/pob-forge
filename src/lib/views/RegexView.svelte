<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { writeText } from "@tauri-apps/plugin-clipboard-manager";
  import { engine } from "$lib/engine.svelte";
  import { game } from "$lib/state/game.svelte";
  import { m } from "$lib/paraglide/messages";
  import { ninjaGroups, valuesByName, type NinjaTable } from "$lib/overlay/price";
  import { highlightFrom, indexNames, loadSaved, storeSaved, type SavedRegex } from "$lib/regex/highlight";

  const g = game.current;
  const unit = g === "poe1" ? "chaos" : "exalted";
  const settingsKey = `pob-forge:regex-settings:${g}`;
  const leagueKey = `pob-forge:price-league:${g}`;

  interface Settings { min: number; limit: number; minPiece: number; wanted: boolean[] }
  const groups = ninjaGroups(g);
  function loadSettings(): Settings {
    // The game's search box: PoE1's takes 50 characters; PoE2's is longer.
    const base: Settings = { min: g === "poe1" ? 20 : 10, limit: g === "poe1" ? 50 : 250, minPiece: 3, wanted: groups.map((x) => x.wanted) };
    try {
      const s = JSON.parse(localStorage.getItem(settingsKey) ?? "null");
      if (s && typeof s === "object") return { ...base, ...s, wanted: groups.map((x, i) => (Array.isArray(s.wanted) && typeof s.wanted[i] === "boolean" ? s.wanted[i] : x.wanted)) };
    } catch {}
    return base;
  }
  const settings = $state<Settings>(loadSettings());
  /** The settings the regex was last built from: typing waits for a pause before it rebuilds. */
  let applied = $state.raw<Settings>(JSON.parse(JSON.stringify(settings)));
  $effect(() => {
    const snapshot = JSON.stringify(settings);
    const timer = setTimeout(() => {
      applied = JSON.parse(snapshot);
      try { localStorage.setItem(settingsKey, snapshot); } catch {}
    }, 150);
    return () => clearTimeout(timer);
  });

  let leagues = $state<{ id: string; text: string }[]>([]);
  let league = $state("");
  /** Per group: every name poe.ninja prices in it, and what it is worth. */
  let prices = $state.raw<Map<string, number>[] | null>(null);
  let loading = $state(false);
  let error = $state<string | null>(null);
  let copied = $state<string | null>(null);
  let saved = $state<SavedRegex[]>(loadSaved());
  let saveName = $state("");
  let ownName = $state("");
  let ownText = $state("");

  async function load() {
    if (!league) return;
    loading = true;
    error = null;
    try {
      const out: Map<string, number>[] = [];
      let reached = false;
      for (const group of groups) {
        const merged = new Map<string, number>();
        for (const source of group.sources) {
          try {
            const table = await invoke<NinjaTable>("price_ninja", { game: g, league, kind: source.kind, category: source.category });
            reached = true;
            for (const [name, value] of valuesByName(g, table)) merged.set(name, Math.max(value, merged.get(name) ?? 0));
          } catch {
            // A table poe.ninja lacks for this league leaves the group a little shorter.
          }
        }
        out.push(merged);
      }
      if (!reached) throw new Error("no table answered");
      prices = out;
    } catch (e) {
      error = m.regex_failed({ error: String(e) });
      prices = null;
    } finally {
      loading = false;
    }
  }

  /** Every name broken into pieces: only redone when the prices or the shortest piece change. */
  const minPiece = $derived(Math.min(8, Math.max(2, Math.round(applied.minPiece) || 3)));
  const index = $derived.by(() => (prices ? indexNames(prices.flatMap((map) => [...map.keys()]), minPiece) : null));

  const result = $derived.by(() => {
    if (!prices || !index) return null;
    const wanted: { name: string; value: number }[] = [];
    prices.forEach((map, i) => {
      if (!applied.wanted[i]) return;
      for (const [name, value] of map) if (value >= applied.min) wanted.push({ name, value });
    });
    return highlightFrom(index, wanted, applied.limit);
  });

  async function copy(text: string) {
    await writeText(text).catch(() => navigator.clipboard?.writeText(text));
    copied = text;
    setTimeout(() => { if (copied === text) copied = null; }, 1500);
  }

  function save(name: string, regex: string) {
    if (!regex.trim()) return;
    saved = [...saved, { id: crypto.randomUUID(), name: name.trim() || regex.slice(0, 24), game: g, regex }];
    storeSaved(saved);
  }

  function remove(id: string) {
    saved = saved.filter((s) => s.id !== id);
    storeSaved(saved);
  }

  const fmt = (v: number) => (v >= 100 ? Math.round(v).toLocaleString("en") : v >= 10 ? v.toFixed(0) : v.toFixed(1));

  onMount(() => {
    void (async () => {
      try {
        leagues = (await engine.tradeLeagues()).leagues;
      } catch {
        leagues = [{ id: "Standard", text: "Standard" }];
      }
      let stored = "";
      try { stored = localStorage.getItem(leagueKey) ?? ""; } catch {}
      league = leagues.some((l) => l.id === stored) ? stored : (leagues[0]?.id ?? "Standard");
      void load();
    })();
  });

  function setLeague(id: string) {
    league = id;
    try { localStorage.setItem(leagueKey, id); } catch {}
    void load();
  }
</script>

<div class="page">
  <div class="body">
    <h1>{m.regex_title()}</h1>
    <p class="dim small intro">{m.regex_intro()}</p>

    <section>
      <h2>{m.regex_highlight()}</h2>
      <p class="dim small">{m.regex_highlight_hint()}</p>
      <div class="controls small">
        <label>
          <span class="dim">{m.regex_league()}</span>
          <select class="select sm" value={league} onchange={(e) => setLeague((e.target as HTMLSelectElement).value)}>
            {#each leagues as l (l.id)}<option value={l.id}>{l.text}</option>{/each}
          </select>
        </label>
        <label>
          <span class="dim">{m.regex_min_price()}</span>
          <input class="input sm mono num" type="number" min="0" step="any" bind:value={settings.min} />
          <span class="dim">{unit}</span>
        </label>
        <label title={m.regex_limit_title()}>
          <span class="dim">{m.regex_limit()}</span>
          <input class="input sm mono num" type="number" min="5" max="500" bind:value={settings.limit} />
        </label>
        <label>
          <span class="dim">{m.regex_min_piece()}</span>
          <input class="input sm mono num" type="number" min="2" max="8" bind:value={settings.minPiece} />
        </label>
      </div>
      <div class="groups small">
        {#each groups as group, i (group.label)}
          <label class="group" class:on={settings.wanted[i]}>
            <input type="checkbox" bind:checked={settings.wanted[i]} />
            {group.label}{#if prices}<span class="dim"> {prices[i].size}</span>{/if}
          </label>
        {/each}
      </div>

      {#if loading}
        <p class="dim small">{m.regex_loading()}</p>
      {:else if error}
        <p class="bad small">{error}</p>
      {:else if result}
        {#if result.regex}
          <div class="out">
            <code class="mono selectable">{result.regex}</code>
            <span class="dim small" class:bad={result.regex.length > applied.limit}>{m.regex_length({ length: result.regex.length, limit: applied.limit })}</span>
            <button class="btn sm primary" onclick={() => void copy(result.regex)}>{copied === result.regex ? m.regex_copied() : m.regex_copy()}</button>
          </div>
          <div class="saveRow small">
            <input class="input sm" placeholder={m.regex_save_name()} bind:value={saveName} />
            <button class="btn sm" onclick={() => { save(saveName || `${settings.min}+ ${unit}`, result.regex); saveName = ""; }}>{m.regex_save()}</button>
          </div>
        {:else}
          <p class="dim small">{m.regex_none()}</p>
        {/if}
        {#if result.covered.length}
          <details open>
            <summary class="small">{m.regex_covered({ count: result.covered.length })}</summary>
            <div class="names small">
              {#each result.covered as c (c.name)}<span>{c.name} <span class="dim mono">{fmt(c.value)}</span></span>{/each}
            </div>
          </details>
        {/if}
        {#if result.overLimit.length}
          <details>
            <summary class="small">{m.regex_over_limit({ count: result.overLimit.length })}</summary>
            <div class="names small">
              {#each result.overLimit as c (c.name)}<span>{c.name} <span class="dim mono">{fmt(c.value)}</span></span>{/each}
            </div>
          </details>
        {/if}
        {#if result.noPiece.length}
          <details>
            <summary class="small">{m.regex_no_piece({ count: result.noPiece.length })}</summary>
            <div class="names small">
              {#each result.noPiece as c (c.name)}<span>{c.name} <span class="dim mono">{fmt(c.value)}</span></span>{/each}
            </div>
          </details>
        {/if}
      {/if}
    </section>

    <section>
      <h2>{m.regex_saved()}</h2>
      {#if saved.filter((s) => s.game === g).length}
        <div class="saved">
          {#each saved.filter((s) => s.game === g) as s (s.id)}
            <span class="small">{s.name}</span>
            <code class="mono small selectable">{s.regex}</code>
            <button class="btn sm" onclick={() => void copy(s.regex)}>{copied === s.regex ? m.regex_copied() : m.regex_copy()}</button>
            <button class="btn sm ghost" onclick={() => remove(s.id)} title={m.regex_delete()} aria-label={m.regex_delete()}>✕</button>
          {/each}
        </div>
      {:else}
        <p class="dim small">{m.regex_saved_empty()}</p>
      {/if}
      <div class="own small">
        <span class="dim">{m.regex_add_own()}</span>
        <input class="input sm" placeholder={m.regex_save_name()} bind:value={ownName} />
        <input class="input sm mono wide" placeholder={m.regex_own_text()} bind:value={ownText} />
        <button class="btn sm" disabled={!ownText.trim()} onclick={() => { save(ownName, ownText.trim()); ownName = ""; ownText = ""; }}>{m.regex_add()}</button>
      </div>
    </section>
  </div>
</div>

<style>
  .page { flex: 1; min-height: 0; overflow: auto; }
  .body { max-width: 820px; margin: 0 auto; padding: 18px 20px 40px; display: flex; flex-direction: column; gap: 18px; }
  h1 { margin: 0; font-family: var(--font-display); font-size: 20px; font-weight: 400; color: var(--fg-0); }
  h2 { margin: 0 0 4px; font-family: var(--font-display); font-size: 15px; font-weight: 400; color: var(--brand); }
  .intro { margin: -10px 0 0; }
  section { display: flex; flex-direction: column; gap: 8px; padding-top: 12px; border-top: 1px solid var(--line-0); }
  section p { margin: 0; }
  .controls, .own, .saveRow { display: flex; flex-wrap: wrap; align-items: center; gap: 8px 16px; }
  .controls label, .own { display: flex; align-items: center; gap: 6px; }
  .num { width: 64px; }
  .wide { flex: 1; min-width: 200px; }
  .groups { display: flex; flex-wrap: wrap; gap: 4px 14px; }
  .group { display: flex; align-items: center; gap: 5px; color: var(--fg-2); cursor: var(--cursor-pointer, pointer); }
  .group.on { color: var(--fg-0); }
  .out { display: flex; align-items: center; gap: 10px; padding: 8px 10px; border: 1px solid color-mix(in oklch, var(--brand) 45%, var(--line-1)); border-radius: var(--r-1); background: var(--bg-1); }
  .out code { flex: 1; min-width: 0; overflow-wrap: anywhere; color: var(--fg-0); font-size: var(--fs-md, 14px); }
  .saveRow input { width: 220px; }
  details summary { cursor: var(--cursor-pointer, pointer); color: var(--fg-1); }
  .names { display: flex; flex-wrap: wrap; gap: 3px 14px; padding: 6px 0 0 14px; color: var(--fg-1); }
  .saved { display: grid; grid-template-columns: minmax(80px, max-content) 1fr auto auto; align-items: center; gap: 6px 10px; }
  .saved code { overflow-wrap: anywhere; color: var(--fg-1); }
</style>
