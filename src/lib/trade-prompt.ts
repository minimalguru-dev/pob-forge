import type { TradeResult } from "$lib/engine.svelte";

export interface ScoredSlot {
  label: string;
  current: string | null;
  results: TradeResult[];
}

const pct = (v: number) => `${v > 0 ? "+" : ""}${Math.round(v * 10) / 10}%`;

/**
 * A question for the assistant about the Trader's listings. Every figure in it is Path of
 * Building's: the price as listed and PoB's change to each weighed stat. The assistant is told
 * to use those and not to work out its own.
 */
export function upgradePrompt(slots: ScoredSlot[], league: string, perSlot = 3): string {
  const lines = [
    `These are trade listings for my build in ${league}. Path of Building scored each one against my current gear; the percentages are its changes to my stats.`,
    "Which would you buy first, and why? My budget: (fill in). Use only these numbers; if something cannot be judged from them, say so.",
    "",
  ];
  for (const s of slots) {
    const top = s.results.slice(0, perSlot);
    if (!top.length) continue;
    lines.push(`${s.label}${s.current ? ` (now: ${s.current})` : ""}`);
    top.forEach((r, i) => {
      const price = r.amount > 0 ? `${r.amount} ${r.currency}` : "no price";
      const stats = r.stats.filter((x) => Math.abs(x.change) >= 0.05).map((x) => `${x.label} ${pct(x.change)}`);
      lines.push(`  ${i + 1}. ${r.name}, ${price}${stats.length ? `: ${stats.join(", ")}` : ""}`);
    });
  }
  return lines.join("\n").trimEnd();
}
