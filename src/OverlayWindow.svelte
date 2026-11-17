<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { call, engine, type ItemEvaluation } from "$lib/engine.svelte";
  import { game } from "$lib/state/game.svelte";
  import { overlay } from "$lib/state/overlay.svelte";
  import { m } from "$lib/paraglide/messages";

  interface MapEvaluation {
    tier: number | null;
    mods: { name: string; kind: "prefix" | "suffix" | null; line: string; calculated: boolean }[];
    unmatched: string[];
    warnings: { level: "danger" | "caution"; text: string; line: string }[];
    calculated: boolean;
    before: Record<string, number> | null;
    delta: Record<string, number> | null;
  }
  /** Mirrors `Price` in src-tauri/src/prices.rs. */
  interface Price {
    name: string;
    league: string;
    chaos: number;
    chaosMax: number | null;
    divine: number | null;
    stack: number | null;
    listings: number | null;
  }
  type Card =
    | { kind: "item"; data: ItemEvaluation; raw: string }
    | { kind: "map"; data: MapEvaluation; raw: string }
    | { kind: "price"; raw: string }
    | { kind: "error"; message: string };

  let price = $state<Price | null>(null);
  let asked = $state(false);

  /** Chaos below a divine, divines above; a stack also gets its total. */
  function priceText(p: Price) {
    const one = (chaos: number) => {
      const div = p.divine && p.chaos > 0 ? (chaos * p.divine) / p.chaos : null;
      return div && div >= 1 ? `${div.toFixed(div >= 10 ? 0 : 1)} div` : `${chaos >= 10 ? Math.round(chaos) : chaos.toFixed(1)}c`;
    };
    const each = p.chaosMax ? `${one(p.chaos)}–${one(p.chaosMax)}` : one(p.chaos);
    return p.stack ? m.overlay_price_stack({ each, total: one(p.chaos * p.stack), count: p.stack }) : each;
  }

  /** Puts the copied text in the assistant's message box, in whichever window shows the chat. */
  async function ask(raw: string) {
    await invoke("overlay_ask", { text: `${m.overlay_ask_prompt()}\n\n${raw}` }).catch(() => {});
    asked = true;
  }

  const MAP_ROWS: [string, string, "pct" | "abs"][] = [
    ["CombinedDPS", "DPS", "pct"],
    ["TotalEHP", "EHP", "pct"],
    ["PhysicalMaximumHitTaken", "Max phys hit", "pct"],
    ["FireMaximumHitTaken", "Max fire hit", "pct"],
    ["ColdMaximumHitTaken", "Max cold hit", "pct"],
    ["LightningMaximumHitTaken", "Max lightning hit", "pct"],
    ["ChaosMaximumHitTaken", "Max chaos hit", "pct"],
    ["FireResistMax", "Max fire res", "abs"],
    ["ColdResistMax", "Max cold res", "abs"],
    ["LightningResistMax", "Max lightning res", "abs"],
    ["ChaosResistMax", "Max chaos res", "abs"],
  ];

  let interact = $state(false);
  /** Ctrl+clicked: the card stays and its buttons work until a click elsewhere or Esc. */
  let pinned = $state(false);
  let root: HTMLDivElement | undefined = $state();
  let card = $state<Card | null>(null);
  let busy = $state(false);
  let timer: ReturnType<typeof setTimeout> | undefined;
  const s = $derived(overlay.settings);

  // Stats worth a glance in the middle of a fight, in reading order.
  const ROWS: [string, string, "pct" | "abs"][] = [
    ["CombinedDPS", "DPS", "pct"],
    ["MinionDPS", "Minion DPS", "pct"],
    ["Life", "Life", "abs"],
    ["EnergyShield", "Energy shield", "abs"],
    ["TotalEHP", "EHP", "pct"],
    ["FireResist", "Fire res", "abs"],
    ["ColdResist", "Cold res", "abs"],
    ["LightningResist", "Lightning res", "abs"],
    ["ChaosResist", "Chaos res", "abs"],
    ["MovementSpeedMod", "Move speed", "abs"],
  ];

  function rows(before: Record<string, number>, delta: Record<string, number>, table = ROWS) {
    return table.flatMap(([key, label, kind]) => {
      const d = delta[key];
      if (!d) return [];
      const base = before[key] ?? 0;
      const text = kind === "pct" && base > 0
        ? `${d > 0 ? "+" : ""}${((d / base) * 100).toFixed(1)}%`
        : `${d > 0 ? "+" : ""}${key === "MovementSpeedMod" ? (d * 100).toFixed(0) + "%" : Math.round(d).toLocaleString()}`;
      return [{ key, label, text, good: d > 0 }];
    });
  }

  /** DPS and EHP change as one number, to put the best slot first. */
  function score(e: ItemEvaluation, slot: ItemEvaluation["slots"][number]) {
    if (!slot.delta) return -Infinity;
    const rel = (k: string) => ((e.before[k] ?? 0) > 0 ? (slot.delta![k] ?? 0) / e.before[k] : 0);
    return rel("CombinedDPS") + rel("MinionDPS") + rel("TotalEHP");
  }
  const evaluation = $derived(card?.kind === "item" ? card.data : null);
  const slots = $derived(evaluation ? [...evaluation.slots].sort((a, b) => score(evaluation, b) - score(evaluation, a)) : []);

  // The window is as tall as what it shows; the resize also makes an unfocused overlay paint.
  $effect(() => {
    void card;
    void price;
    void busy;
    void pinned;
    void interact;
    void asked;
    const height = root ? Math.ceil(root.offsetHeight) : 1;
    void invoke("overlay_fit", { height: card || busy || interact ? height : 1 }).catch(() => {});
  });

  function show(next: Card) {
    card = next;
    clearTimeout(timer);
    timer = setTimeout(() => { if (!interact && !pinned) void invoke("overlay_dismiss"); }, (s?.cardSeconds ?? 15) * 1000);
  }

  function clear() {
    clearTimeout(timer);
    card = null;
    price = null;
    pinned = false;
    busy = false;
  }

  /** With Ctrl held the card takes clicks; the first one pins it. */
  function onPointerDown() {
    if (card && !pinned && !interact) void invoke("overlay_pin");
  }

  async function evaluate(raw: string, kind: string) {
    busy = true;
    price = null;
    asked = false;
    // poe.ninja lists uniques, currency and cards; the price comes in beside PoB's numbers.
    const priced = s?.prices && kind === "item" ? invoke<Price | null>("price_check", { raw }).catch(() => null) : Promise.resolve(null);
    try {
      if (kind === "map") show({ kind: "map", data: await call<MapEvaluation>("map_evaluate", { raw }), raw });
      else show({ kind: "item", data: await engine.itemEvaluate(raw), raw });
    } catch (e) {
      const text = String(e).replace(/^EngineError: (Error: )?/, "");
      // Currency and cards fit no slot; their price is still worth showing.
      const p = await priced;
      if (p) show({ kind: "price", raw });
      else show({ kind: "error", message: text.includes("no build is loaded") ? m.overlay_no_build() : text });
    } finally {
      busy = false;
    }
    const p = await priced;
    if (card && "raw" in card && card.raw === raw) price = p;
  }

  onMount(() => {
    // The overlay sits over the game: nothing behind the panel is painted.
    document.documentElement.style.background = "transparent";
    document.body.style.background = "transparent";
    const offs: Promise<() => void>[] = [
      listen<{ raw: string; kind: string }>("overlay:item", (e) => void evaluate(e.payload.raw, e.payload.kind)),
      listen<boolean>("overlay:interact", (e) => {
        interact = e.payload;
        if (!interact && card) show(card);
      }),
      listen("overlay:pinned", () => {
        pinned = true;
        clearTimeout(timer);
      }),
      listen("overlay:dismiss", () => clear()),
    ];
    // A pinned card closes on a click back in the game or on Esc.
    const onBlur = () => { if (pinned) void invoke("overlay_dismiss"); };
    const onKey = (e: KeyboardEvent) => { if (e.key === "Escape" && (pinned || card)) void invoke("overlay_dismiss"); };
    window.addEventListener("blur", onBlur);
    window.addEventListener("keydown", onKey);
    void overlay.init();
    void game.init().catch(() => {});
    return () => {
      clearTimeout(timer);
      window.removeEventListener("blur", onBlur);
      window.removeEventListener("keydown", onKey);
      for (const off of offs) void off.then((f) => f());
    };
  });
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="overlay" class:interact class:pinned bind:this={root} onpointerdown={onPointerDown} style:--panel-alpha={s?.opacity ?? 0.92}>
  {#if interact}
    <header data-tauri-drag-region>
      <span data-tauri-drag-region>{m.overlay_title()} · {m.overlay_drag_hint()}</span>
      <button class="btn sm" onclick={() => invoke("overlay_interact", { on: false })}>{m.overlay_done()}</button>
    </header>
  {/if}

  {#if busy}
    <section class="panel dim">{m.overlay_calculating()}</section>
  {/if}

  {#if card?.kind === "error"}
    <section class="panel"><p class="bad">{card.message}</p></section>
  {:else if card?.kind === "map"}
    {@const e = card.data}
    <section class="panel map">
      <div class="slothead">
        <strong>{m.overlay_map_title()}</strong>
        {#if e.tier}<span class="dim small">{m.overlay_map_tier({ tier: e.tier })}</span>{/if}
      </div>
      {#each e.warnings as w (w.text)}
        <p class="small" class:bad={w.level === "danger"} class:warn={w.level === "caution"}>{w.level === "danger" ? "⚠ " : ""}{w.text}</p>
      {/each}
      {#if e.calculated && e.before && e.delta}
        {@const list = rows(e.before, e.delta, MAP_ROWS)}
        <p class="dim small">{m.overlay_map_calculated()}</p>
        <div class="grid">
          {#each list as r (r.key)}<span>{r.label}</span><span class:good={r.good} class:bad={!r.good}>{r.text}</span>{/each}
        </div>
        {#if !list.length}<p class="dim small">{m.overlay_no_change()}</p>{/if}
      {:else if !e.warnings.length}
        <p class="dim small">{m.overlay_map_nothing()}</p>
      {/if}
      {#if e.unmatched.length}
        <p class="dim small">{m.overlay_map_unmatched({ count: e.unmatched.length })}</p>
      {/if}
      {@render askButton(card.raw)}
    </section>
  {:else if card?.kind === "price"}
    <section class="panel item">
      <div class="title"><strong>{price?.name ?? ""}</strong></div>
      {@render priceLine()}
      {@render askButton(card.raw)}
    </section>
  {:else if card?.kind === "item"}
    {@const e = card.data}
    <section class="panel item">
      <div class="title">
        <strong class="rarity {e.item.rarity?.toLowerCase()}">{e.item.name}</strong>
        {#if e.item.baseName && e.item.baseName !== e.item.name}<span class="dim">{e.item.baseName}</span>{/if}
      </div>
      {@render priceLine()}
      {#if e.tooLowLevel}<p class="warn">{m.overlay_level_req({ need: e.item.levelRequired, have: e.characterLevel })}</p>{/if}
      {#each slots as slot (slot.slot)}
        <div class="slot">
          <div class="slothead">
            <span>{slot.label}</span>
            <span class="dim small">{slot.replacing ? m.overlay_replacing({ name: slot.replacing }) : m.overlay_empty_slot()}</span>
          </div>
          {#if slot.skipped}
            <p class="dim small">{slot.skipped}</p>
          {:else}
            {@const list = rows(e.before, slot.delta ?? {})}
            {#if !list.length}<p class="dim small">{m.overlay_no_change()}</p>{/if}
            <div class="grid">
              {#each list as r (r.key)}<span>{r.label}</span><span class:good={r.good} class:bad={!r.good}>{r.text}</span>{/each}
            </div>
            {#each slot.attributeShortfall ?? [] as a (a.attribute)}
              <p class="bad small">{m.overlay_attr_short({ attribute: a.attribute, missing: Math.ceil(a.missing) })}</p>
            {/each}
          {/if}
        </div>
      {/each}
      {#if !slots.length}<p class="dim small">{m.overlay_no_slot()}</p>{/if}
      {@render askButton(card.raw)}
    </section>
  {/if}

  {#snippet priceLine()}
    {#if price}
      <p class="small price" title={m.overlay_price_source({ league: price.league })}>
        {m.overlay_price({ price: priceText(price) })}{#if price.listings}<span class="dim"> · {m.overlay_price_listings({ count: price.listings })}</span>{/if}
      </p>
    {/if}
  {/snippet}

  {#snippet askButton(raw: string)}
    {#if interact || pinned}
      <div class="actions">
        <button class="btn sm" disabled={asked} onclick={() => void ask(raw)}>{asked ? m.overlay_asked() : m.overlay_ask()}</button>
      </div>
    {:else}
      <p class="dim small hint">{m.overlay_pin_hint()}</p>
    {/if}
  {/snippet}

  {#if interact && !card}
    <section class="panel dim small">{m.overlay_idle_hint({ toggle: s?.toggleKey ?? "", interact: s?.interactKey ?? "" })}</section>
  {/if}
</div>

<style>
  :global(html), :global(body) { background: transparent !important; overflow: hidden; }
  .overlay { display: flex; flex-direction: column; gap: 8px; padding: 6px; box-sizing: border-box; font-size: 13px; user-select: none; }
  .overlay.pinned .panel { outline: 1px solid var(--line-2); }
  .hint { margin-top: 6px; }
  .overlay.interact { outline: 1px dashed var(--line-2); outline-offset: -1px; }
  header { display: flex; justify-content: space-between; align-items: center; gap: 8px; padding: 6px 8px; border-radius: 8px; cursor: move;
    background: color-mix(in oklch, var(--bg-1) calc(var(--panel-alpha) * 100%), transparent); }
  .panel { padding: 10px 12px; border-radius: 10px; border: 1px solid var(--line-1);
    background: color-mix(in oklch, var(--bg-0) calc(var(--panel-alpha) * 100%), transparent); }
  .title { display: flex; flex-direction: column; margin-bottom: 6px; }
  .slot { padding-top: 8px; margin-top: 8px; border-top: 1px solid var(--line-1); }
  .slot:first-of-type { border-top: 0; margin-top: 0; padding-top: 0; }
  .slothead { display: flex; justify-content: space-between; gap: 8px; margin-bottom: 4px; }
  .grid { display: grid; grid-template-columns: 1fr auto; gap: 2px 12px; font-variant-numeric: tabular-nums; }
  .good { color: var(--ok); } .bad { color: var(--bad); } .warn { color: var(--warn); }
  .dim { color: var(--fg-3); } .small { font-size: 12px; } p { margin: 4px 0; }
  .price { color: oklch(86% 0.13 95); }
  .actions { display: flex; justify-content: flex-end; margin-top: 6px; }
  .rarity.unique { color: oklch(70% 0.12 60); } .rarity.rare { color: oklch(86% 0.13 95); } .rarity.magic { color: oklch(72% 0.12 260); }
</style>
