import { expect, test } from "bun:test";
import {
  advancedMods, clock, decimalPlaces, findAdvanced, groupListings, inPrimary, isAdvanced, rateWait, rollMin, stripAdvanced, wheelStep,
} from "../src/lib/overlay/advanced";
import { applyFilters, logbookFactions, parseCopied, simpleQuery } from "../src/lib/overlay/price";

const ring = `Item Class: Rings
Rarity: Rare
Doom Loop
Two-Stone Ring
--------
Item Level: 84
--------
{ Implicit Modifier — Elemental, Fire, Cold, Resistance }
+14(12-16)% to Fire and Cold Resistances (implicit)
--------
{ Prefix Modifier "Virile" (Tier: 4) — Life }
+62(55-69) to maximum Life
{ Suffix Modifier "of the Volcano" (Tier: 1) — Elemental, Fire, Resistance }
+46(46-48)% to Fire Resistance
{ Master Crafted Suffix Modifier "of Craft" (Rank: 2) — Attribute }
+22(19-22) to Strength (crafted)
`;

test("the advanced copy reads back as the plain copy", () => {
  const plain = stripAdvanced(ring);
  expect(plain).not.toContain("{");
  expect(plain).toContain("+62 to maximum Life");
  expect(plain).toContain("+14% to Fire and Cold Resistances (implicit)");
  expect(isAdvanced(ring)).toBe(true);
  expect(isAdvanced(plain)).toBe(false);
  expect(parseCopied(plain)?.baseType).toBe("Two-Stone Ring");
});

test("mod tiers, ranks and rolls come from the headers", () => {
  const mods = advancedMods(ring);
  expect(mods.map((m) => [m.kind, m.name, m.tier, m.rank])).toEqual([
    ["implicit", null, null, null],
    ["prefix", "Virile", 4, null],
    ["suffix", "of the Volcano", 1, null],
    ["crafted suffix", "of Craft", null, 2],
  ]);
  expect(mods[1].lines[0]).toEqual({ text: "+62 to maximum Life", rolls: [{ value: 62, min: 55, max: 69 }] });
  // PoB's lines carry colour codes; they still find their header.
  expect(findAdvanced(mods, ["^x8888FF+46% to Fire Resistance"])?.tier).toBe(1);
  expect(findAdvanced(mods, ["+10% to Cold Resistance"])).toBeNull();
});

test("starting minimums sit the roll range under the roll, as Awakened's", () => {
  expect(rollMin(62, 10)).toBe(55);
  expect(rollMin(46, 10, { min: 46, max: 48 })).toBe(46); // never under the range's floor
  expect(rollMin(1.5, 10)).toBe(1.35);
  expect(decimalPlaces(5)).toBe(0);
  expect(decimalPlaces(3.4)).toBe(1);
  // A unique steps by a share of its range: 80 in 60-100 at 10% is 76.
  expect(rollMin(80, 10, { min: 60, max: 100 }, true)).toBe(76);
  expect(rollMin(62, 0)).toBe(62);
});

test("the mouse wheel steps by the value's own precision", () => {
  expect(wheelStep("55", true, false)).toBe("56");
  expect(wheelStep("55", false, true)).toBe("45");
  expect(wheelStep("1.35", true, false)).toBe("1.36");
  expect(wheelStep("", true, false)).toBe("1");
});

test("a seller's listings fold into one row", () => {
  const l = (account: string, amount: number, currency = "chaos") => ({ amount, currency, indexed: null, account, online: true, stack: null });
  const rows = groupListings([l("a", 5), l("a", 5), l("b", 6), l("a", 7), l("c", 8), l("d", 9), l("a", 9)]);
  expect(rows.map((r) => [r.account, r.amount, r.times])).toEqual([
    ["a", 5, 3], // same price twice, then back within the last two rows
    ["b", 6, 1],
    ["c", 8, 1],
    ["d", 9, 1],
    ["a", 9, 1],
  ]);
});

test("prices read in the table's currency", () => {
  const rates = { primary: "chaos", rates: { divine: 0.005 } };
  expect(inPrimary(2, "divine", rates)).toBe(400);
  expect(inPrimary(30, "chaos", rates)).toBe(30);
  expect(inPrimary(1, "mirror", rates)).toBeNull();
});

test("the trade site's wait reads as a countdown", () => {
  expect(rateWait("rate-limited:24")).toBe(24);
  expect(rateWait("pathofexile.com returned HTTP 500")).toBeNull();
  expect(clock(24)).toBe("0:24");
  expect(clock(65.2)).toBe("1:06");
});

test("an unidentified unique is searched by its base", () => {
  const item = parseCopied(`Item Class: Body Armours
Rarity: Unique
Vaal Regalia
--------
Item Level: 85
--------
Unidentified
`)!;
  expect([item.unidentified, item.name, item.baseType]).toEqual([true, "Vaal Regalia", null]);
  const q = JSON.parse(simpleQuery(item, { rooms: [], gemLevel: null, quality: null, mapTier: null, corrupted: "any" })).query;
  expect(q.type).toBe("Vaal Regalia");
  expect(q.name).toBeUndefined();
  expect(q.filters.type_filters.filters.rarity.option).toBe("unique");
  expect(q.filters.misc_filters.filters.identified.option).toBe("false");
});

test("a logbook's factions become filters", () => {
  const raw = `Item Class: Expedition Logbooks
Rarity: Magic
Expedition Logbook
--------
Area Level: 83
--------
Battleground Graves
Knights of the Sun
--------
Dried Riverbed
Druids of the Broken Circle
`;
  expect(logbookFactions(raw)).toEqual(["Knights of the Sun", "Druids of the Broken Circle"]);
  const item = parseCopied(raw)!;
  const q = JSON.parse(simpleQuery(item, { rooms: [], gemLevel: null, quality: null, mapTier: null, corrupted: "any", factions: ["pseudo.pseudo_logbook_faction_knights"], areaLevel: 83, game: "poe1" })).query;
  expect(q.stats[0].filters).toEqual([{ id: "pseudo.pseudo_logbook_faction_knights" }]);
  expect(q.filters.map_filters.filters.area_level.min).toBe(83);
});

test("maximums go into the query beside minimums", () => {
  const q = JSON.parse(applyFilters(JSON.stringify({ query: { status: { option: "online" } } }), {
    pseudo: [{ id: "pseudo.pseudo_total_life", min: 60, max: 80 }],
    dps: [{ key: "pdps", min: null, max: 500 }],
    itemLevel: null, quality: null, links: null, corrupted: "any", priceIn: "any",
  })).query;
  expect(q.stats[0].filters[0]).toEqual({ id: "pseudo.pseudo_total_life", value: { min: 60, max: 80 } });
  expect(q.filters.weapon_filters.filters.pdps).toEqual({ max: 500 });
});

test("a base is priced by its item level, influences and rarity, as Awakened's", async () => {
  const { matchBase } = await import("../src/lib/overlay/price");
  const line = (variant: string | null, levelRequired: number, frame: number, value: number, listings = 5) => ({
    name: "Stygian Vise", baseType: null, variant, gemLevel: null, gemQuality: null, corrupted: false, links: null, mapTier: null,
    levelRequired, frame, value, listings, change: null, trend: [],
  });
  const table = { primary: "chaos", rates: {}, lines: [line(null, 84, 2, 40), line(null, 84, 0, 30, 9), line("Shaper/Hunter", 86, 2, 900), line(null, 86, 2, 120)] };
  const rare = parseCopied("Item Class: Belts\nRarity: Rare\nDoom Cord\nStygian Vise\n--------\nItem Level: 84\n")!;
  expect(matchBase(table, rare, "Stygian Vise", 84)?.value).toBe(40); // its own rarity first
  expect(matchBase(table, { ...rare, rarity: "Magic" }, "Stygian Vise", 84)?.value).toBe(30); // else most listed
  expect(matchBase(table, rare, "Stygian Vise", 88)?.value).toBe(120); // 86 stands for 86 and over
  expect(matchBase(table, { ...rare, marks: ["Hunter Item", "Shaper Item"] }, "Stygian Vise", 86)?.value).toBe(900);
  expect(matchBase(table, rare, "Stygian Vise", 75)).toBeNull(); // poe.ninja starts at 82
});
