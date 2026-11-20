<script lang="ts">
  import { engine } from "$lib/engine.svelte";
  import { build } from "$lib/state/build.svelte";
  import { m } from "$lib/paraglide/messages";

  /** The figures a tree change moves, in one line under the tree; the full sheet is a click away. */
  const FIELDS = ["FullDPS", "CombinedDPS", "TotalEHP", "Life", "EnergyShield", "FireResist", "ColdResist", "LightningResist", "ChaosResist", "MovementSpeedMod"];

  let { sheet, ontoggle }: { sheet: boolean; ontoggle: () => void } = $props();

  let stats = $state<Record<string, number | string | boolean>>({});

  $effect(() => {
    void build.rev;
    if (!build.loaded) return;
    engine.getStats(FIELDS).then((r) => (stats = r.stats)).catch(() => {});
  });

  const num = (k: string) => (typeof stats[k] === "number" ? (stats[k] as number) : null);
  const short = (v: number | null) =>
    v === null ? "—" : v >= 1e6 ? `${(v / 1e6).toFixed(2)}M` : v >= 1e4 ? `${(v / 1e3).toFixed(1)}k` : Math.round(v).toLocaleString("en");
  const dps = $derived(num("FullDPS") || num("CombinedDPS"));
  const move = $derived(num("MovementSpeedMod"));
</script>

<footer class="strip" aria-label={m.strip_label()}>
  <span><span class="k">DPS</span> <strong class="dps">{short(dps)}</strong></span>
  <span><span class="k">EHP</span> {short(num("TotalEHP"))}</span>
  <span><span class="k">{m.strip_life()}</span> <span class="life">{short(num("Life"))}</span></span>
  {#if (num("EnergyShield") ?? 0) > 0}<span><span class="k">ES</span> <span class="es">{short(num("EnergyShield"))}</span></span>{/if}
  <span>
    <span class="k">{m.strip_res()}</span>
    <span class="fire">{num("FireResist") ?? "—"}</span>
    <span class="cold">{num("ColdResist") ?? "—"}</span>
    <span class="light">{num("LightningResist") ?? "—"}</span>
    <span class="chaos">{num("ChaosResist") ?? "—"}</span>
  </span>
  {#if move !== null}<span><span class="k">{m.strip_move()}</span> {move >= 1 ? "+" : ""}{Math.round((move - 1) * 100)}%</span>{/if}
  <span class="grow"></span>
  <button class="all" onclick={ontoggle}>{sheet ? m.strip_hide() : m.strip_all()}</button>
</footer>

<style>
  .strip {
    flex: none;
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: 4px 22px;
    padding: 7px 16px;
    border-top: 1px solid var(--line-0);
    background: var(--bg-1);
    font-size: var(--fs-sm);
    font-variant-numeric: tabular-nums;
    color: var(--fg-1);
  }
  .k { color: var(--fg-3); margin-right: 2px; }
  .dps { color: var(--brand); font-weight: 600; }
  .life { color: var(--c-life); }
  .es { color: var(--c-es); }
  .fire { color: var(--c-fire); }
  .cold { color: var(--c-cold); }
  .light { color: var(--c-lightning); }
  .chaos { color: var(--c-chaos); }
  .grow { flex: 1; }
  .all {
    appearance: none;
    border: 0;
    background: none;
    padding: 0;
    color: var(--brand);
    font: inherit;
    font-size: var(--fs-xs);
    cursor: var(--cursor-pointer, pointer);
  }
  .all:hover { text-decoration: underline; }
</style>
