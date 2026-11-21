<script lang="ts">
  import { build } from "$lib/state/build.svelte";
  import { ledger, formatPct, headlineKey, statMap, pctChange, EHP_STAT } from "$lib/state/ledger.svelte";
  import { m } from "$lib/paraglide/messages";

  /** The last few edits fit on the line; the count and totals cover the rest. */
  const SHOWN = 4;
  const shown = $derived(ledger.entries.slice(-SHOWN).reverse());
  const now = $derived(statMap(build.sidebar));
  const dpsKey = $derived(headlineKey(now));
  const dpsTotal = $derived(dpsKey ? formatPct(pctChange(ledger.baseline.get(dpsKey), now.get(dpsKey))) : "");
  const ehpTotal = $derived(formatPct(pctChange(ledger.baseline.get(`player:${EHP_STAT}`), now.get(`player:${EHP_STAT}`))));
  const tone = (text: string) => (text.startsWith("+") ? "gain" : text.startsWith("−") ? "loss" : "");
  const canUndo = $derived(build.view === "tree" && build.busy === 0);
</script>

<footer class="ledger" aria-label={m.ledger_title()}>
  <span class="fg-tag">{m.ledger_title()}</span>
  <div class="list">
    {#if shown.length === 0}
      <span class="none">{m.ledger_none()}</span>
    {:else}
      {#each shown as e (e.id)}
        {@const dps = formatPct(e.dps)}
        {@const ehp = formatPct(e.ehp)}
        <span class="entry">
          {e.label}
          {#if dps}<span class={tone(dps)}>{dps} DPS</span>{/if}
          {#if ehp}<span class={tone(ehp)}>{ehp} EHP</span>{/if}
          {#if !dps && !ehp}<span class="flat">±0</span>{/if}
        </span>
      {/each}
    {/if}
  </div>
  {#if ledger.entries.length}
    <span class="sum">
      {m.ledger_count({ count: ledger.entries.length })}
      · {#if dpsTotal}<span class={tone(dpsTotal)}>{dpsTotal} DPS</span>{:else}{m.ledger_same({ stat: "DPS" })}{/if}
      · {#if ehpTotal}<span class={tone(ehpTotal)}>{ehpTotal} EHP</span>{:else}{m.ledger_same({ stat: "EHP" })}{/if}
    </span>
  {/if}
  {#if build.view === "tree"}
    <button class="btn sm" disabled={!canUndo} title={m.ledger_undo_title()} onclick={() => void build.undo()}>{m.ledger_undo()}</button>
  {/if}
</footer>

<style>
  .ledger {
    flex: none;
    display: flex;
    align-items: center;
    gap: 16px;
    min-height: 38px;
    padding: 5px 16px;
    border-top: 1px solid var(--plate-edge);
    background: var(--bar-bg);
    box-shadow: var(--bar-shadow);
    font-family: var(--font-mono);
    font-size: var(--fs-sm);
    color: var(--fg-2);
    font-variant-numeric: tabular-nums;
  }
  .list {
    flex: 1;
    min-width: 0;
    display: flex;
    gap: 4px 22px;
    overflow: hidden;
    white-space: nowrap;
  }
  .entry {
    display: flex;
    gap: 6px;
  }
  .none,
  .flat {
    color: var(--fg-4);
  }
  .sum {
    flex: none;
    color: var(--fg-3);
    white-space: nowrap;
  }
  .gain {
    color: var(--gain);
  }
  .loss {
    color: var(--loss);
  }
</style>
