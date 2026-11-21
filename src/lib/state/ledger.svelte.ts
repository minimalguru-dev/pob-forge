import type { Sidebar } from "$lib/engine.svelte";
import { stripPobText } from "$lib/pobtext";

/** The first of these PoB lists is the build's headline damage figure. */
export const DPS_STATS = ["FullDPS", "CombinedDPS", "TotalDPS", "TotalDotDPS", "AverageDamage", "AverageHit"];
export const EHP_STAT = "TotalEHP";

/** Figures where a smaller number is the better one. */
const LOWER_IS_BETTER = /Cost|Reserved|Reservation|DamageTaken|Duration$|Cooldown/;

export type StatMap = Map<string, number>;

const statKey = (actor: string | null, stat: string) => `${actor ?? "player"}:${stat}`;

/** The number a PoB sidebar value starts with: "4,210,550.3" → 4210550.3, "75% (+40%)" → 75. */
export function statValue(rhs: string | null): number | null {
  if (!rhs) return null;
  const text = stripPobText(rhs).replace(/,/g, "");
  const found = /[-+−]?\d+(?:\.\d+)?/.exec(text);
  if (!found) return null;
  const n = Number(found[0].replace("−", "-"));
  return Number.isFinite(n) ? n : null;
}

export function statMap(sidebar: Sidebar | null): StatMap {
  const map: StatMap = new Map();
  for (const r of sidebar?.rows ?? []) {
    if (!r.stat) continue;
    const v = statValue(r.rhs);
    if (v !== null) map.set(statKey(r.actor, r.stat), v);
  }
  return map;
}

/** Relative change in percent; null when there is nothing to compare against. */
export function pctChange(before: number | undefined, after: number | undefined): number | null {
  if (before === undefined || after === undefined) return null;
  if (before === after) return 0;
  if (before === 0) return null;
  return ((after - before) / Math.abs(before)) * 100;
}

/** "+4.8%", "−0.4%"; empty under a twentieth of a percent. */
export function formatPct(pct: number | null): string {
  if (pct === null || Math.abs(pct) < 0.05) return "";
  const digits = Math.abs(pct) >= 100 ? 0 : 1;
  return `${pct > 0 ? "+" : "−"}${Math.abs(pct).toFixed(digits)}%`;
}

export function headlineKey(map: StatMap): string | null {
  for (const s of DPS_STATS) if (map.has(statKey(null, s))) return statKey(null, s);
  return null;
}

export interface LedgerEntry {
  id: number;
  label: string;
  dps: number | null;
  ehp: number | null;
}

/**
 * What changed since the build was opened or last saved: the figures it was
 * saved with, and each edit the user made since with its effect on damage and
 * effective HP. Every number is PoB's own sidebar value before and after.
 */
class Ledger {
  baseline = $state<StatMap>(new Map());
  entries = $state<LedgerEntry[]>([]);
  private generation = -1;
  private nextId = 1;

  /** A different build is open: start over from its figures. */
  follow(generation: number, sidebar: Sidebar | null) {
    if (generation === this.generation) return;
    this.generation = generation;
    this.reset(sidebar);
  }

  /** Saved: the saved figures are the new baseline. */
  reset(sidebar: Sidebar | null) {
    this.baseline = statMap(sidebar);
    this.entries = [];
  }

  record(label: string, before: StatMap, after: StatMap) {
    const dpsKey = headlineKey(after) ?? headlineKey(before);
    const ehpKey = statKey(null, EHP_STAT);
    const entry: LedgerEntry = {
      id: this.nextId++,
      label,
      dps: dpsKey ? pctChange(before.get(dpsKey), after.get(dpsKey)) : null,
      ehp: pctChange(before.get(ehpKey), after.get(ehpKey)),
    };
    this.entries = [...this.entries.slice(-49), entry];
  }

  /** Change of one sidebar figure since the baseline, in percent. */
  sinceSave(actor: string | null, stat: string, value: number | null): number | null {
    if (value === null) return null;
    return pctChange(this.baseline.get(statKey(actor, stat)), value);
  }

  /** Whether a rise in this figure is good for the build. */
  higherIsBetter(stat: string) {
    return !LOWER_IS_BETTER.test(stat);
  }
}

export const ledger = new Ledger();
