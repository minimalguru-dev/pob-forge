<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { call, engine, openUrl, type ItemEvaluation } from "$lib/engine.svelte";
  import { game } from "$lib/state/game.svelte";
  import { overlay } from "$lib/state/overlay.svelte";
  import { m } from "$lib/paraglide/messages";
  import { gemSchedule, poe1QuestPoints, poe1XpMultiplier, poeVaultUrl, resPenalty, type LevelGem, type LogState } from "$lib/overlay/leveling";
  import { CHANGED_EVENT, progression } from "$lib/progression/model";

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

  type Card = { kind: "item"; data: ItemEvaluation } | { kind: "error"; message: string };

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

  function rows(before: Record<string, number>, delta: Record<string, number>) {
    return ROWS.flatMap(([key, label, kind]) => {
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
    if (kind !== "item") return; // Maps arrive in a later step.
    busy = true;
    try {
      show({ kind: "item", data: await engine.itemEvaluate(raw) });
    } catch (e) {
      const text = String(e).replace(/^EngineError: (Error: )?/, "");
      show({ kind: "error", message: text.includes("no build is loaded") ? m.overlay_no_build() : text });
    } finally {
      busy = false;
    }
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
      listen<LogState>("gamelog:update", (e) => {
        const levelled = e.payload.level !== log?.level;
        log = e.payload;
        if (levelled) void refreshLeveling();
      }),
      listen(CHANGED_EVENT, () => void refreshLeveling()),
      listen("build:edited", () => void refreshLeveling()),
      listen("mcp:changed", () => void refreshLeveling()),
    ];
    void overlay.init();
    void (async () => {
      await game.init().catch(() => {});
      log = await invoke<LogState>("gamelog_state").catch(() => null);
      await refreshLeveling();
    })();
    return () => {
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

  {#if busy}
    <section class="panel dim">{m.overlay_calculating()}</section>
  {/if}

  {#if card?.kind === "error"}
    <section class="panel"><p class="bad">{card.message}</p></section>
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

  {#if interact && !card}
    <section class="panel dim small">{m.overlay_idle_hint({ toggle: s?.toggleKey ?? "", interact: s?.interactKey ?? "" })}</section>
  {/if}
</div>

<style>
  :global(html), :global(body) { background: transparent !important; overflow: hidden; }
  .overlay { display: flex; flex-direction: column; gap: 8px; padding: 6px; height: 100vh; box-sizing: border-box; font-size: 13px; user-select: none; }
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
  .rarity.unique { color: oklch(70% 0.12 60); } .rarity.rare { color: oklch(86% 0.13 95); } .rarity.magic { color: oklch(72% 0.12 260); }
</style>
