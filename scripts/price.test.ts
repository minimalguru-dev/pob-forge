import { expect, test } from "bun:test";
import { age, defaultChoices, formatListing, formatValue, matchNinja, ninjaSources, parseCopied, type NinjaTable } from "../src/lib/overlay/price";

const TABULA = `Item Class: Body Armours\r\nRarity: Unique\r\nTabula Rasa\r\nSimple Robe\r\n--------\r\nSockets: W-W-W-W-W-W \r\n--------\r\nItem Level: 80\r\n`;
const GEM = `Item Class: Skill Gems
Rarity: Gem
Vaal Rejuvenation Totem
--------
Totem, Vaal, Duration
Level: 20 (Max)
Quality: +20% (augmented)
--------
Corrupted`;
const DIVINE = `Item Class: Stackable Currency
Rarity: Currency
Divine Orb
--------
Stack Size: 1,234/20
`;
const RARE = `Item Class: Rings
Rarity: Rare
Doom Loop
Iron Ring
--------
Item Level: 84
--------
+250 to maximum Life`;

test("reads the parts of a copied item a price needs", () => {
  const t = parseCopied(TABULA)!;
  expect([t.itemClass, t.rarity, t.name, t.baseType, t.links, t.corrupted]).toEqual(["Body Armours", "Unique", "Tabula Rasa", "Simple Robe", 6, false]);
  const g = parseCopied(GEM)!;
  expect([g.gemLevel, g.gemQuality, g.corrupted, g.baseType]).toEqual([20, 20, true, null]);
  expect(parseCopied(DIVINE)!.stack).toBe(1234);
  expect(parseCopied(RARE)!.baseType).toBe("Iron Ring");
  expect(parseCopied("hello")).toBeNull();
});

test("picks poe.ninja tables per game and kind", () => {
  expect(ninjaSources("poe1", parseCopied(TABULA)!)[0]).toEqual({ kind: "stash", category: "UniqueArmour" });
  expect(ninjaSources("poe2", parseCopied(TABULA)!)[0]).toEqual({ kind: "stash", category: "UniqueArmours" });
  expect(ninjaSources("poe1", parseCopied(GEM)!)).toEqual([{ kind: "stash", category: "SkillGem" }]);
  expect(ninjaSources("poe1", parseCopied(DIVINE)!)[0]).toEqual({ kind: "exchange", category: "Currency" });
  expect(ninjaSources("poe1", parseCopied(RARE)!)).toEqual([]);
});

const table = (lines: Partial<NinjaTable["lines"][number]>[], primary = "chaos", rates: Record<string, number> = { divine: 0.005 }): NinjaTable => ({
  primary,
  rates,
  lines: lines.map((l) => ({ name: "", baseType: null, variant: null, gemLevel: null, gemQuality: null, corrupted: false, links: null, mapTier: null, value: 0, listings: null, ...l })),
});

test("matches six-link uniques and gem variants", () => {
  const uniques = table([
    { name: "Tabula Rasa", baseType: "Simple Robe", value: 10, listings: 500 },
    { name: "Tabula Rasa", baseType: "Simple Robe", links: 6, value: 12, listings: 50 },
  ]);
  expect(matchNinja(parseCopied(TABULA)!, uniques)?.value).toBe(12);
  const gems = table([
    { name: "Vaal Rejuvenation Totem", gemLevel: 20, gemQuality: 0, value: 1 },
    { name: "Vaal Rejuvenation Totem", gemLevel: 20, gemQuality: 20, corrupted: true, value: 5 },
    { name: "Vaal Rejuvenation Totem", gemLevel: 21, gemQuality: 20, corrupted: true, value: 50 },
  ]);
  expect(matchNinja(parseCopied(GEM)!, gems)?.value).toBe(5);
  expect(matchNinja(parseCopied(RARE)!, gems)).toBeNull();
});

test("quotes prices the way players do", () => {
  expect(formatValue(40, { primary: "chaos", rates: { divine: 0.005 } })).toBe("40 c");
  expect(formatValue(500, { primary: "chaos", rates: { divine: 0.005 } })).toBe("2.5 div");
  expect(formatValue(3.2, { primary: "divine", rates: { exalted: 500 } })).toBe("3.2 div");
  expect(formatValue(0.1, { primary: "divine", rates: { exalted: 500 } })).toBe("50 ex");
  expect(formatListing(1500, "chaos")).toBe("1,500 c");
  expect(formatListing(null, null)).toBe("—");
  expect(age("2026-11-19T10:00:00Z", Date.parse("2026-11-19T13:00:00Z"))).toBe("3h");
});

test("rares start with their searchable mods at 90%", () => {
  const info = {
    name: "Doom Loop", unique: false, rarity: "RARE", itemLevel: 84, corrupted: false, category: "Ring", baseName: "Iron Ring", realms: ["PC"], listed: ["Any"],
    defences: [],
    mods: [
      { lines: ["+250 to maximum Life"], type: "explicit", searchable: true, value: 250, ranged: true },
      { lines: ["Unsearchable"], type: "explicit", searchable: false, value: null, ranged: false },
    ],
  };
  expect(defaultChoices(info).mods).toEqual([{ checked: true, min: "225", max: "" }, { checked: false, min: "", max: "" }]);
  expect(defaultChoices({ ...info, unique: true }).mods[0].checked).toBe(false);
});
