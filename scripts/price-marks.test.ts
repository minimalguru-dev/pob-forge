import { expect, test } from "bun:test";
import { applyFilters, blight, itemMarks, mapStats, matchNinja, modTags, parseCopied, simpleQuery, type NinjaTable } from "../src/lib/overlay/price";

const FOULBORN = `Item Class: Belts
Rarity: Unique
Foulborn Headhunter
Leather Belt
--------
Item Level: 84
--------
+40 to Strength
+50 to Dexterity
Culling Strike
`;

const ninja = (lines: Partial<NinjaTable["lines"][number]>[]): NinjaTable => ({
  primary: "chaos",
  rates: {},
  lines: lines.map((l) => ({ name: "", baseType: null, variant: null, gemLevel: null, gemQuality: null, corrupted: false, links: null, mapTier: null, value: 0, listings: null, change: null, trend: [], ...l })),
});

test("prices a Foulborn unique by its mutated mods", () => {
  const table = ninja([
    { name: "Foulborn Headhunter", mutated: ["Culling Strike"], value: 900, listings: 3 },
    { name: "Foulborn Headhunter", mutated: ["(20-30)% increased Damage with Hits against Rare monsters"], value: 400, listings: 9 },
    { name: "Headhunter", value: 2000, listings: 50 },
  ]);
  const item = parseCopied(FOULBORN)!;
  // The game's copy does not say which mods are mutated: no line until PoB has read them.
  expect(matchNinja(item, table)).toBeNull();
  expect(matchNinja({ ...item, mutated: ["Culling Strike"] }, table)?.value).toBe(900);
  expect(matchNinja({ ...item, mutated: ["25% increased Damage with Hits against Rare monsters"] }, table)?.value).toBe(400);
  expect(matchNinja({ ...item, name: "Headhunter" }, table)?.value).toBe(2000);
});

test("tags mods with what PoB's colours say they are", () => {
  expect(modTags(FOULBORN, { type: "explicit", lines: ["^xCD2285Culling Strike"], kinds: ["mutated"] })).toEqual(["mutated"]);
});

test("turns influence and item marks into trade filters", () => {
  const raw = `Item Class: Helmets\nRarity: Rare\nDoom Crown\nHubris Circlet\n--------\nItem Level: 86\n--------\n+90 to maximum Life\n--------\nShaper Item\nFractured Item\nMirrored\n`;
  const item = parseCopied(raw)!;
  const marks = itemMarks("poe1", item);
  expect(marks.map((m) => m.misc ?? m.pseudo)).toEqual(["pseudo.pseudo_has_shaper_influence", "fractured_item", "mirrored"]);
  const q = JSON.parse(applyFilters(JSON.stringify({ query: { stats: [{ type: "and", filters: [] }] } }), { pseudo: [], itemLevel: null, quality: null, links: null, corrupted: "any", priceIn: "any", marks }));
  expect(q.query.filters.misc_filters.filters).toEqual({ fractured_item: { option: "true" }, mirrored: { option: "true" } });
  expect(q.query.stats[0].filters).toEqual([{ id: "pseudo.pseudo_has_shaper_influence" }]);
  // PoE2 has no influences.
  expect(itemMarks("poe2", item).map((m) => m.misc)).toEqual(["fractured_item", "mirrored"]);
});

test("searches a Forbidden Tome by its area level", () => {
  const raw = "Item Class: Sanctum Research\nRarity: Normal\nForbidden Tome\n--------\nArea Level: 83\n--------\nItem Level: 83\n";
  const item = parseCopied(raw)!;
  expect(item.areaLevel).toBe(83);
  const c = { rooms: [], gemLevel: null, quality: null, mapTier: null, corrupted: "any" as const, areaLevel: 83 };
  const poe1 = JSON.parse(simpleQuery(item, { ...c, game: "poe1" }));
  expect(poe1.query.type).toBe("Forbidden Tome");
  expect(poe1.query.filters.map_filters.filters.area_level).toEqual({ min: 83 });
  expect(JSON.parse(simpleQuery(item, { ...c, game: "poe2" })).query.filters.misc_filters.filters.area_level).toEqual({ min: 83 });
});

test("reads a map's figures and blight", () => {
  const raw = `Item Class: Maps\nRarity: Rare\nGrim Hollow\nBlighted Cemetery Map\n--------\nMap Tier: 16\nItem Quantity: +85% (augmented)\nItem Rarity: +50% (augmented)\nMonster Pack Size: +30% (augmented)\n--------\nItem Level: 83\n`;
  const item = parseCopied(raw)!;
  expect(mapStats("poe1", raw).map((s) => [s.filter, s.value, s.checked])).toEqual([["map_iiq", 85, false], ["map_iir", 50, false], ["map_packsize", 30, false]]);
  expect(blight(item)).toBe("map_blighted");
  const q = JSON.parse(simpleQuery(item, { rooms: [], gemLevel: null, quality: null, mapTier: 16, corrupted: "any", mapStats: [{ filter: "map_iiq", min: 80 }], blight: "map_blighted" }));
  expect(q.query.filters.map_filters.filters).toEqual({ map_tier: { min: 16, max: 16 }, map_iiq: { min: 80 }, map_blighted: { option: "true" } });
});
