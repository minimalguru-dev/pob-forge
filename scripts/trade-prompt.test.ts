import { expect, test } from "bun:test";
import { upgradePrompt } from "../src/lib/trade-prompt";
import type { TradeResult } from "../src/lib/engine.svelte";

const listing = (name: string, amount: number, currency: string, changes: [string, number][]): TradeResult => ({
  index: 0,
  name,
  rarity: "RARE",
  raw: "",
  amount,
  currency,
  priceType: "~price",
  whisper: null,
  trader: null,
  weight: 0,
  score: null,
  stats: changes.map(([label, change]) => ({ label, stat: label, change })),
});

test("lists the top listings per slot with PoB's changes and the price", () => {
  const text = upgradePrompt(
    [
      {
        label: "Helmet",
        current: "Doom Crown",
        results: [
          listing("Grim Visor", 45, "chaos", [["Full DPS", 3.24], ["Effective Hit Pool", -0.5]]),
          listing("Kraken Mask", 1, "divine", [["Full DPS", 1.0], ["Effective Hit Pool", 0.01]]),
          listing("Fourth", 5, "chaos", []),
          listing("Fifth", 5, "chaos", []),
        ],
      },
      { label: "Boots", current: null, results: [] },
    ],
    "Mercenaries",
  );
  expect(text).toContain("in Mercenaries");
  expect(text).toContain("Use only these numbers");
  expect(text).toContain("Helmet (now: Doom Crown)");
  expect(text).toContain("  1. Grim Visor, 45 chaos: Full DPS +3.2%, Effective Hit Pool -0.5%");
  expect(text).toContain("  2. Kraken Mask, 1 divine: Full DPS +1%");
  expect(text).not.toContain("Fifth");
  expect(text).not.toContain("Boots");
});
