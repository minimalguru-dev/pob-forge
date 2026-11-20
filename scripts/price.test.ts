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

import { applyFilters, poedbUrl, pseudoTotals, sparkPoints, wikiUrl } from "../src/lib/overlay/price";

const ring = {
  name: "Doom Loop", unique: false, rarity: "RARE", itemLevel: 84, corrupted: false, category: "Ring", baseName: "Two-Stone Ring", realms: ["PC"], listed: ["Any"],
  defences: [],
  mods: [
    { lines: ["^x8888FF+14% to Fire and Cold Resistances"], type: "implicit", searchable: true, value: 14, ranged: true },
    { lines: ["^x8888FF+78 to maximum Life"], type: "explicit", searchable: true, value: 78, ranged: true },
    { lines: ["^x8888FF+42% to Fire Resistance"], type: "explicit", searchable: true, value: 42, ranged: true },
    { lines: ["^x8888FF+10% to all Elemental Resistances"], type: "explicit", searchable: true, value: 10, ranged: true },
    { lines: ["^x8888FF+13% to Chaos Resistance"], type: "explicit", searchable: true, value: 13, ranged: true },
    { lines: ["^x8888FF+22 to Strength"], type: "explicit", searchable: true, value: 22, ranged: true },
  ],
};

test("totals resistances and life the way the trade site's pseudo stats do", () => {
  const totals = Object.fromEntries(pseudoTotals(ring).map((p) => [p.key, p]));
  expect(totals.ele.total).toBe(14 * 2 + 42 + 30);
  expect(totals.ele.mods).toEqual([0, 2, 3]);
  expect(totals.life).toEqual({ key: "life", total: 78, mods: [1] });
  expect(totals.chaos.total).toBe(13);
  expect(totals.es).toBeUndefined();
});

test("adds pseudo stats and item filters to Buy Similar's query", () => {
  const base = JSON.stringify({ status: { option: "securable" }, type: "Two-Stone Ring", stats: [{ type: "and", filters: [{ id: "explicit.stat_1", value: { min: 20 } }] }] });
  const q = JSON.parse(applyFilters(base, {
    pseudo: [{ id: "pseudo.pseudo_total_elemental_resistance", min: 90 }],
    itemLevel: 82, quality: null, links: 6, corrupted: "no", priceIn: "divine",
  }));
  expect(q.stats[0].filters.map((f: { id: string }) => f.id)).toEqual(["explicit.stat_1", "pseudo.pseudo_total_elemental_resistance"]);
  expect(q.filters.misc_filters.filters).toEqual({ ilvl: { min: 82 }, corrupted: { option: "false" } });
  expect(q.filters.socket_filters.filters.links).toEqual({ min: 6 });
  expect(q.filters.trade_filters.filters.price).toEqual({ option: "divine" });
  // A wrapped { query, sort } keeps its sort.
  const wrapped = JSON.parse(applyFilters(JSON.stringify({ query: { type: "X" }, sort: { price: "asc" } }), { pseudo: [], itemLevel: null, quality: 20, links: null, corrupted: "any", priceIn: "any" }));
  expect(wrapped.sort).toEqual({ price: "asc" });
  expect(wrapped.query.filters.misc_filters.filters.quality).toEqual({ min: 20 });
});

test("links to the wiki and poedb", () => {
  const tabula = parseCopied(TABULA)!;
  expect(wikiUrl("poe1", tabula)).toBe("https://www.poewiki.net/wiki/Tabula_Rasa");
  expect(wikiUrl("poe1", parseCopied(RARE)!)).toBeNull();
  expect(poedbUrl("poe2", parseCopied(RARE)!)).toBe("https://poe2db.tw/us/Iron_Ring");
});

test("draws a sparkline across gaps", () => {
  expect(sparkPoints([0, null, 10], 10, 10)).toBe("0.0,10.0 10.0,0.0");
  expect(sparkPoints([null, 5], 10, 10)).toBe("");
});

import { isMap, isTemple, modTags, roomFilter, simpleQuery, templeRooms } from "../src/lib/overlay/price";

const CHRONICLE = `Item Class: Misc Map Items
Rarity: Normal
Chronicle of Atzoatl
--------
Area Level: 83
--------
Open Rooms:
Apex of Atzoatl
Locus of Corruption (Tier 3)
Hall of Champions (Tier 1)
--------
Obstructed Rooms:
Doryani's Institute (Tier 3)`;

test("tags each mod with what the copy and PoB say about it", () => {
  const raw = "Item Class: Rings\nRarity: Rare\nX\nIron Ring\n--------\n+12% to Fire Resistance (implicit)\n--------\n+78 to maximum Life (crafted)\n+20% to Cold Resistance (fractured)";
  expect(modTags(raw, { type: "explicit", lines: ["^x8888FF+78 to maximum Life"] })).toEqual(["crafted"]);
  expect(modTags(raw, { type: "explicit", lines: ["+20% to Cold Resistance"] })).toEqual(["fractured"]);
  expect(modTags(raw, { type: "implicit", lines: ["+12% to Fire Resistance"] })).toEqual(["implicit"]);
  expect(modTags(raw, { type: "implicit", lines: ["5% increased Life if the Searing Exarch is dominant"] })).toEqual(["implicit", "eldritch"]);
  expect(modTags(raw, { type: "explicit", lines: ["+5 to Strength"] })).toEqual([]);
});

test("reads a temple's rooms and finds their trade filters", () => {
  const item = parseCopied(CHRONICLE)!;
  expect(isTemple(item)).toBe(true);
  expect(templeRooms(CHRONICLE)).toEqual({ open: ["Apex of Atzoatl", "Locus of Corruption (Tier 3)", "Hall of Champions (Tier 1)"], obstructed: ["Doryani's Institute (Tier 3)"] });
  const stats = [{ id: "pseudo.pseudo_temple_corruption_room_3", text: "Has Room: Locus of Corruption (Tier 3)" }];
  expect(roomFilter("Locus of Corruption (Tier 3)", stats)).toBe("pseudo.pseudo_temple_corruption_room_3");
  expect(roomFilter("Hall of Champions (Tier 1)", stats)).toBeNull();
  const q = JSON.parse(simpleQuery(item, { rooms: ["pseudo.pseudo_temple_corruption_room_3"], gemLevel: null, quality: null, mapTier: null, corrupted: "any" }));
  expect(q.query.type).toBe("Chronicle of Atzoatl");
  expect(q.query.stats[0].filters).toEqual([{ id: "pseudo.pseudo_temple_corruption_room_3" }]);
});

test("builds gem and map searches", () => {
  const gem = JSON.parse(simpleQuery(parseCopied(GEM)!, { rooms: [], gemLevel: 20, quality: 20, mapTier: null, corrupted: "yes" }));
  expect(gem.query.type).toBe("Vaal Rejuvenation Totem");
  expect(gem.query.filters.misc_filters.filters).toEqual({ gem_level: { min: 20 }, quality: { min: 20 }, corrupted: { option: "true" } });
  const map = parseCopied("Item Class: Maps\nRarity: Normal\nStrand Map\n--------\nMap Tier: 16")!;
  expect(isMap(map)).toBe(true);
  const q = JSON.parse(simpleQuery(map, { rooms: [], gemLevel: null, quality: null, mapTier: 16, corrupted: "any" }));
  expect(q.query.type).toBeUndefined();
  expect(q.query.filters.type_filters.filters.category).toEqual({ option: "map" });
  expect(q.query.filters.map_filters.filters.map_tier).toEqual({ min: 16, max: 16 });
});
