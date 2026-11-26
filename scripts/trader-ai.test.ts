import { describe, expect, test } from "bun:test";
import { askPrompt, matchMod, modKey, parsePlan, type AskContext } from "../src/lib/trader-ai";

const ctx: AskContext = {
  game: "poe1",
  rows: [
    { key: "Ring 1", label: "Ring 1", itemName: "Doom Loop" },
    { key: "Boots", label: "Boots", itemName: null },
  ],
  stats: [
    { stat: "FullDPS", label: "Full DPS" },
    { stat: "TotalEHP", label: "Effective Hit Pool" },
    { stat: "Life", label: "Life" },
  ],
  currencies: [
    { id: "chaos", name: "Chaos Orb" },
    { id: "divine", name: "Divine Orb" },
  ],
};

// As the engine's trade_mods lists them for Ring 1 (PoE1).
const ringMods = [
  { type: "Explicit", tradeId: "explicit.stat_983749596", text: "#% increased maximum Life" },
  { type: "Implicit", tradeId: "implicit.stat_3299347043", text: "+# to maximum Life" },
  { type: "Explicit", tradeId: "explicit.stat_3299347043", text: "+# to maximum Life" },
  { type: "Explicit", tradeId: "explicit.stat_3372524247", text: "+#% to Fire Resistance" },
  { type: "Explicit", tradeId: "explicit.stat_2901986750", text: "+#% to all Elemental Resistances" },
  { type: "Explicit", tradeId: "explicit.stat_174664100", text: "Minions have #% increased Movement Speed" },
  { type: "Pseudo", tradeId: "pseudo.pseudo_count_elemental_resistances", text: "# total Elemental Resistances" },
];

describe("parsePlan", () => {
  test("keeps what names a listed row, stat or currency", () => {
    const plan = parsePlan(
      'Here you go:\n```json\n{"rows":["Ring 1","Amulet"],"weights":[{"stat":"Life","weightMult":2},{"stat":"Made up","weightMult":1}],"maxPrice":50,"maxPriceType":"chaos","maxLevel":70,"corrupted":false,"required":{"Ring 1":[{"text":"+# to maximum Life","min":60}],"Amulet":[{"text":"x"}]},"blocked":{"Boots":[{"text":"y"}]},"note":"Amulet is not a slot here."}\n```',
      ctx,
    );
    expect(plan.rows).toEqual(["Ring 1"]);
    expect(plan.weights).toEqual([{ stat: "Life", weightMult: 2 }]);
    expect(plan.maxPrice).toBe(50);
    expect(plan.maxPriceType).toBe("chaos");
    expect(plan.maxLevel).toBe(70);
    expect(plan.corrupted).toBe(false);
    expect(plan.required).toEqual({ "Ring 1": [{ text: "+# to maximum Life", min: 60 }] });
    // Mods only for rows that are searched.
    expect(plan.blocked).toEqual({});
    expect(plan.note).toBe("Amulet is not a slot here.");
  });

  test("leaves unknown or nonsense values at their current setting", () => {
    const plan = parsePlan('{"rows":"Ring 1","weights":[],"maxPrice":-3,"maxPriceType":"exalted","maxLevel":400,"corrupted":"no","note":""}', ctx);
    expect(plan).toEqual({ rows: [], weights: null, maxPrice: null, maxPriceType: null, maxLevel: null, corrupted: null, required: {}, blocked: {}, note: null });
  });

  test("a currency without a price is dropped, and weights are clamped", () => {
    const plan = parsePlan('{"rows":["Boots"],"maxPriceType":"divine","weights":[{"stat":"FullDPS","weightMult":99},{"stat":"TotalEHP","weightMult":0}]}', ctx);
    expect(plan.maxPriceType).toBeNull();
    expect(plan.weights).toEqual([{ stat: "FullDPS", weightMult: 10 }]);
  });

  test("braces inside strings do not end the object", () => {
    expect(parsePlan('{"rows":["Boots"],"note":"a } inside"} trailing {', ctx).note).toBe("a } inside");
  });

  test("an answer without JSON is an error", () => {
    expect(() => parsePlan("I cannot help with that.", ctx)).toThrow("no JSON object");
  });
});

describe("matchMod", () => {
  test("the same wording, with numbers written in or not", () => {
    expect(matchMod("+# to maximum Life", ringMods)?.tradeId).toBe("explicit.stat_3299347043");
    expect(matchMod("+70 to Maximum Life", ringMods)?.tradeId).toBe("explicit.stat_3299347043");
    expect(matchMod("40% to fire resistance", ringMods)?.tradeId).toBe("explicit.stat_3372524247");
  });
  test("a prefix finds the longer wording", () => {
    expect(matchMod("+#% to all Elemental", ringMods)?.tradeId).toBe("explicit.stat_2901986750");
  });
  test("never the minions' version of a mod", () => {
    expect(matchMod("#% increased Movement Speed", ringMods)).toBeNull();
  });
  test("nothing for an empty or unknown mod", () => {
    expect(matchMod("", ringMods)).toBeNull();
    expect(matchMod("+# to Intelligence", ringMods)).toBeNull();
  });
  test("modKey evens out numbers, signs and case", () => {
    expect(modKey("+12.5% to Cold Resistance (Local)")).toBe("#% to cold resistance");
  });
});

describe("askPrompt", () => {
  test("lists rows, stats and currencies by id, and caps the request", () => {
    const p = askPrompt("x".repeat(5000), ctx);
    expect(p).toContain("Ring 1: Ring 1 (now Doom Loop)");
    expect(p).toContain("Boots: Boots (empty)");
    expect(p).toContain("TotalEHP: Effective Hit Pool");
    expect(p).toContain("divine: Divine Orb");
    expect(p).toContain("x".repeat(2000) + "\n</request>");
    expect(p).not.toContain("x".repeat(2001));
  });
});
