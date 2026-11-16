<script lang="ts">
  import { onMount, tick } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { call, engine, openUrl, type ItemEvaluation } from "$lib/engine.svelte";
  import { game } from "$lib/state/game.svelte";
  import { overlay } from "$lib/state/overlay.svelte";
  import { m } from "$lib/paraglide/messages";
  import { gemSchedule, poe1QuestPoints, poe1XpMultiplier, poeVaultUrl, resPenalty, type LevelGem, type LogState } from "$lib/overlay/leveling";
  import { CHANGED_EVENT, progression, type Check } from "$lib/progression/model";

  let plan = $state<{ current: string; target: string; next: Check[]; blocked: number } | null>(null);

  /** The next few Progression priorities, from the same rules the Progression tab uses. */
  async function refreshProgression() {
    if (!game.isPoe1) { plan = null; return; }
    try {
      const saved = await progression.load();
      const current = saved?.snapshots.find((x) => x.id === saved.currentId);
      const target = saved?.snapshots.find((x) => x.id === saved.targetId);
      if (!current || !target) { plan = null; return; }
      const report = await progression.report(current, target);
      plan = {
        current: current.label,
        target: target.label,
        next: report.priorities.filter((c) => c.id !== "manual").slice(0, 4),
        blocked: report.checks.filter((c) => c.status === "blocked").length,
      };
    } catch {
      plan = null;
    }
  }

  let log = $state<LogState | null>(null);
  let gems = $state<LevelGem[]>([]);
  let gemSource = $state<"target" | "build">("build");
  let pointsSpent = $state<number | null>(null);
  const level = $derived(log?.level ?? null);
  // Towns and hideouts have no monsters, so their area level says nothing about experience.
  const safeArea = $derived(!!log?.areaId && (log.areaId.includes("Hideout") || /_town$/i.test(log.areaId)));
  const xp = $derived(game.isPoe1 && level && log?.areaLevel && !safeArea ? poe1XpMultiplier(level, log.areaLevel) : null);
  const penalty = $derived(log ? resPenalty(game.current, log.act, log.endgame) : null);
  const quest = $derived(game.isPoe1 && log ? poe1QuestPoints(log.act, log.endgame) : null);
  const schedule = $derived(level ? gemSchedule(gems, level) : null);
  const guide = $derived(log && !log.endgame ? poeVaultUrl(game.current, log.act) : null);

  /** Gems come from the Progression target when there is one, else from the open build. */
  async function refreshLeveling() {
    try {
      let names: string[] | undefined;
      if (game.isPoe1) {
        const plan = await progression.load().catch(() => null);
        const target = plan?.snapshots.find((s) => s.id === plan.targetId);
        if (target) names = [...new Set(target.gems.filter((g) => g.enabled).map((g) => g.name))];
      }
      gemSource = names ? "target" : "build";
      const info = await call<{ gems: LevelGem[]; passivePointsSpent: number | null }>("leveling_info", names ? { names } : {});
      gems = info.gems;
      pointsSpent = info.passivePointsSpent;
    } catch {
      gems = [];
      pointsSpent = null;
    }
  }

  interface MapEvaluation {
    tier: number | null;
    mods: { name: string; kind: "prefix" | "suffix" | null; line: string; calculated: boolean }[];
    unmatched: string[];
    warnings: { level: "danger" | "caution"; text: string; line: string }[];
    calculated: boolean;
    before: Record<string, number> | null;
    delta: Record<string, number> | null;
  }
  type Card = { kind: "item"; data: ItemEvaluation } | { kind: "map"; data: MapEvaluation } | { kind: "error"; message: string };

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

  function show(next: Card) {
    card = next;
    clearTimeout(timer);
    timer = setTimeout(() => { if (!interact) card = null; }, (s?.cardSeconds ?? 15) * 1000);
  }

  async function evaluate(raw: string, kind: string) {
    busy = true;
    try {
      if (kind === "map") show({ kind: "map", data: await call<MapEvaluation>("map_evaluate", { raw }) });
      else show({ kind: "item", data: await engine.itemEvaluate(raw) });
    } catch (e) {
      const text = String(e).replace(/^EngineError: (Error: )?/, "");
      show({ kind: "error", message: text.includes("no build is loaded") ? m.overlay_no_build() : text });
    } finally {
      busy = false;
    }
  }

  let closeButton = $state<HTMLButtonElement | null>(null);
  let stack = $state<HTMLDivElement | null>(null);
  const anyPanel = $derived(busy || !!card || (!!s?.leveling && (!!log?.zone || interact)) || (!!s?.progression && !!plan) || interact);

  /** Rust lets clicks reach this one button while the rest of the overlay passes them to the game. */
  function reportCloseButton() {
    const r = closeButton?.getBoundingClientRect();
    void invoke("overlay_close_button", { rect: r ? { x: r.x, y: r.y, width: r.width, height: r.height } : null }).catch(() => {});
  }

  // The button moves with the panels: report it again whenever they change.
  $effect(() => {
    void [closeButton, interact, anyPanel];
    void tick().then(reportCloseButton);
  });
  $effect(() => {
    if (!stack) return;
    const ro = new ResizeObserver(reportCloseButton);
    ro.observe(stack);
    return () => ro.disconnect();
  });

  onMount(() => {
    reportCloseButton();
    window.addEventListener("resize", reportCloseButton);
    // The overlay sits over the game: nothing behind the panel is painted.
    document.documentElement.style.background = "transparent";
    document.body.style.background = "transparent";
    const offs: Promise<() => void>[] = [
      listen<{ raw: string; kind: string }>("overlay:item", (e) => void evaluate(e.payload.raw, e.payload.kind)),
      listen<boolean>("overlay:interact", (e) => {
        interact = e.payload;
        if (!interact && card) show(card);
      }),
      listen<LogState>("gamelog:update", (e) => {
        const levelled = e.payload.level !== log?.level;
        log = e.payload;
        if (levelled) void refreshLeveling();
      }),
      listen(CHANGED_EVENT, () => { void refreshLeveling(); void refreshProgression(); }),
      listen("build:edited", () => void refreshLeveling()),
      listen("mcp:changed", () => void refreshLeveling()),
    ];
    void overlay.init();
    void (async () => {
      await game.init().catch(() => {});
      log = await invoke<LogState>("gamelog_state").catch(() => null);
      await Promise.all([refreshLeveling(), refreshProgression()]);
    })();
    return () => {
      window.removeEventListener("resize", reportCloseButton);
      clearTimeout(timer);
      for (const off of offs) void off.then((f) => f());
    };
  });
</script>

<div class="overlay" class:interact style:--panel-alpha={s?.opacity ?? 0.92}>
  {#if interact}
    <header data-tauri-drag-region>
      <span data-tauri-drag-region>{m.overlay_title()} · {m.overlay_drag_hint()}</span>
      <button class="btn sm" onclick={() => invoke("overlay_interact", { on: false })}>{m.overlay_done()}</button>
    </header>
  {/if}

  <div class="stack" bind:this={stack}>
  {#if anyPanel}
    <button
      class="close"
      bind:this={closeButton}
      title={m.overlay_close_hint({ key: s?.toggleKey ?? "" })}
      aria-label={m.overlay_close()}
      onclick={() => void invoke("overlay_hide")}
    >✕</button>
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
    </section>
  {:else if card?.kind === "item"}
    {@const e = card.data}
    <section class="panel item">
      <div class="title">
        <strong class="rarity {e.item.rarity?.toLowerCase()}">{e.item.name}</strong>
        {#if e.item.baseName && e.item.baseName !== e.item.name}<span class="dim">{e.item.baseName}</span>{/if}
      </div>
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
    </section>
  {/if}

  {#if s?.leveling && log?.zone}
    <section class="panel leveling">
      <div class="slothead">
        <strong>{log.zone}</strong>
        <span class="dim small">
          {#if log.endgame}{m.overlay_endgame()}{:else if log.act}{m.overlay_act({ act: log.act })}{/if}
          {#if log.areaLevel} · {m.overlay_area_level({ level: log.areaLevel })}{/if}
        </span>
      </div>
      <div class="grid">
        {#if level}<span>{m.overlay_char_level()}</span><span>{level}</span>{/if}
        {#if xp !== null}
          <span>{m.overlay_xp()}</span><span class:good={xp >= 0.95} class:warn={xp < 0.95 && xp >= 0.5} class:bad={xp < 0.5}>{Math.round(xp * 100)}%</span>
        {:else if game.isPoe2 && level && log.areaLevel && !safeArea}
          <span>{m.overlay_level_gap()}</span><span>{log.areaLevel - level > 0 ? "+" : ""}{log.areaLevel - level}</span>
        {/if}
        {#if penalty !== null}<span>{m.overlay_res_penalty()}</span><span class:warn={penalty < 0}>{penalty}%</span>{/if}
        {#if level && quest !== null}
          <span>{m.overlay_points()}</span>
          <span>{m.overlay_points_value({ min: level - 1 + quest, max: level - 1 + quest + ((log.act ?? 11) > 2 ? 1 : 0), spent: pointsSpent ?? "?" })}</span>
        {/if}
      </div>
      {#if schedule}
        {#if schedule.justUnlocked.length}
          <p class="good small">{m.overlay_gems_now({ gems: schedule.justUnlocked.map((g) => g.name).join(", ") })}</p>
        {/if}
        {#if schedule.upcoming.length}
          <p class="dim small">{m.overlay_gems_next({ gems: schedule.upcoming.map((g) => `${g.name} (${g.level})`).join(", ") })}</p>
        {/if}
        <p class="dim small">{gemSource === "target" ? m.overlay_gems_from_target() : m.overlay_gems_from_build()}</p>
      {/if}
      {#if guide}
        <button class="btn sm" disabled={!interact} title={interact ? "" : m.overlay_click_hint({ key: s.interactKey })} onclick={() => void openUrl(guide)}>
          {m.overlay_poevault({ act: log.act ?? 0 })}
        </button>
      {/if}
    </section>
  {:else if s?.leveling && interact}
    <section class="panel dim small">{log?.path ? m.overlay_log_waiting() : m.overlay_log_missing()}</section>
  {/if}

  {#if s?.progression && plan}
    <section class="panel progression">
      <div class="slothead">
        <strong>{m.overlay_progression_title()}</strong>
        <span class="dim small">{plan.current} → {plan.target}</span>
      </div>
      {#if plan.blocked}<p class="bad small">{m.overlay_progression_blocked({ count: plan.blocked })}</p>{/if}
      <ol>
        {#each plan.next as c (c.id)}<li class="small" class:bad={c.status === "blocked"}>{c.title}</li>{/each}
      </ol>
      {#if !plan.next.length}<p class="dim small">{m.overlay_progression_clear()}</p>{/if}
    </section>
  {/if}

  {#if interact && !card}
    <section class="panel dim small">{m.overlay_idle_hint({ toggle: s?.toggleKey ?? "", interact: s?.interactKey ?? "" })}</section>
  {/if}
  </div>
</div>

<style>
  :global(html), :global(body) { background: transparent !important; overflow: hidden; }
  .overlay { display: flex; flex-direction: column; gap: 8px; padding: 6px; height: 100vh; box-sizing: border-box; font-size: 13px; user-select: none; }
  .stack { position: relative; display: flex; flex-direction: column; gap: 8px; }
  /* Sits in the top-right corner of the first panel. */
  .close { position: absolute; top: 7px; right: 7px; width: 20px; height: 20px; padding: 0; border: 0; border-radius: 5px; cursor: pointer;
    display: grid; place-items: center; font-size: 11px; line-height: 1; color: var(--fg-3); background: transparent; z-index: 1; }
  .close:hover { color: var(--fg-0); background: color-mix(in oklch, var(--bad) 45%, transparent); }
  .stack > section:first-of-type { padding-right: 34px; }
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
  ol { margin: 4px 0 0; padding-left: 18px; } li { padding: 1px 0; }
  .rarity.unique { color: oklch(70% 0.12 60); } .rarity.rare { color: oklch(86% 0.13 95); } .rarity.magic { color: oklch(72% 0.12 260); }
</style>
