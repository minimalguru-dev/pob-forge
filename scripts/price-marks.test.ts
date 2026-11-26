import { expect, test } from "bun:test";
import { applyFilters, blight, exclusions, searchRarity, hasCraftingValue, itemLevelFilter, itemMarks, likelyFinished, mapStats, matchNinja, modTags, parseCopied, presetMarks, simpleQuery, type NinjaTable } from "../src/lib/overlay/price";

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

const RARE = (marks: string, extra = "") =>
  parseCopied(`Item Class: Helmets\nRarity: Rare\nDoom Crown\nHubris Circlet\n--------\nItem Level: 87\n--------\n+90 to maximum Life${extra}\n--------\n${marks}`)!;

test("the usual search starts the marks off, the base search turns on what makes the base", () => {
  const item = RARE("Shaper Item\nElder Item\nFractured Item\nSynthesised Item\n");
  const marks = itemMarks("poe1", item);
  expect(marks.map((m) => [m.kind, m.checked])).toEqual([["influence", false], ["influence", false], ["fractured", false], ["synthesised", false]]);
  expect(presetMarks(marks, true).every((m) => m.checked)).toBe(true);
  expect(presetMarks(marks, false).some((m) => m.checked)).toBe(false);
  // Mirrored and Split are never required by a preset.
  expect(presetMarks(itemMarks("poe1", RARE("Mirrored\nSplit\n")), true).map((m) => m.checked)).toEqual([false, false]);
  // Like Awakened: more than two influences, no influence filters.
  expect(itemMarks("poe1", RARE("Shaper Item\nElder Item\nCrusader Item\n")).filter((m) => m.kind === "influence")).toEqual([]);
});

test("item level as Awakened sets it: at most 86, a cluster jewel's bracket", () => {
  expect(itemLevelFilter(87, false)).toEqual({ min: 86, max: null });
  expect(itemLevelFilter(84, false)).toEqual({ min: 84, max: null });
  expect(itemLevelFilter(84, true)).toEqual({ min: 84, max: 100 });
  expect(itemLevelFilter(80, true)).toEqual({ min: 75, max: 100 });
  expect(itemLevelFilter(70, true)).toEqual({ min: 68, max: 74 });
  expect(itemLevelFilter(55, true)).toEqual({ min: 50, max: 67 });
});

test("offers the base search for a base worth one that is not finished", () => {
  expect(hasCraftingValue(RARE("Shaper Item\n"), 70)).toBe(true);
  expect(hasCraftingValue(RARE(""), 82)).toBe(true);
  expect(hasCraftingValue(RARE(""), 81)).toBe(false);
  expect(likelyFinished(RARE(""), false)).toBe(false);
  expect(likelyFinished(RARE(""), true)).toBe(true);
  expect(likelyFinished(RARE("Mirrored\n"), false)).toBe(true);
  expect(likelyFinished({ ...RARE(""), corrupted: true }, false)).toBe(true);
});

test("leaves out fractured, mirrored and split listings, as Awakened does", () => {
  expect(exclusions("poe1", RARE(""), { base: false, league: "Mercenaries" })).toEqual(["fractured_item", "mirrored", "split"]);
  // Standard keeps split items unless it is the base search.
  expect(exclusions("poe1", RARE(""), { base: false, league: "Standard" })).toEqual(["fractured_item", "mirrored"]);
  expect(exclusions("poe1", RARE(""), { base: true, league: "Standard" })).toEqual(["fractured_item", "mirrored", "split"]);
  // An influenced or fractured item may be split; a fractured one is not left out for being fractured.
  expect(exclusions("poe1", RARE("Shaper Item\n"), { base: false, league: "Mercenaries" })).toEqual(["fractured_item", "mirrored"]);
  expect(exclusions("poe1", RARE("Fractured Item\n"), { base: false, league: "Mercenaries" })).toEqual(["mirrored"]);
  // Corrupted, mirrored and unique items exclude nothing.
  expect(exclusions("poe1", { ...RARE(""), corrupted: true }, { base: false, league: "x" })).toEqual([]);
  expect(exclusions("poe1", RARE("Mirrored\n"), { base: false, league: "x" })).toEqual([]);
  expect(exclusions("poe1", { ...RARE(""), rarity: "Unique" }, { base: false, league: "x" })).toEqual([]);
  expect(exclusions("poe2", RARE(""), { base: false, league: "x" })).toEqual(["fractured_item", "mirrored"]);
});

test("a base search for a Shaper Hubris Circlet: base, item level 86, influence, no mods", () => {
  const item = RARE("Shaper Item\n");
  const marks = presetMarks(itemMarks("poe1", item), true).filter((m) => m.checked);
  const q = JSON.parse(applyFilters(JSON.stringify({ query: { type: "Hubris Circlet", stats: [{ type: "and", filters: [] }] } }), {
    pseudo: [], itemLevel: itemLevelFilter(87, false).min, quality: null, links: null, corrupted: "no", priceIn: "any", marks,
    exclude: exclusions("poe1", item, { base: true, league: "Mercenaries" }),
    rarity: searchRarity(item),
  }));
  expect(q.query.type).toBe("Hubris Circlet");
  expect(q.query.filters.type_filters.filters.rarity).toEqual({ option: "nonunique" });
  expect(q.query.filters.misc_filters.filters).toEqual({ ilvl: { min: 86 }, corrupted: { option: "false" }, fractured_item: { option: "false" }, mirrored: { option: "false" } });
  expect(q.query.stats[0].filters).toEqual([{ id: "pseudo.pseudo_has_shaper_influence" }]);
});

test("rarity as Awakened sets it", () => {
  expect(searchRarity(RARE(""))).toBe("nonunique");
  expect(searchRarity({ ...RARE(""), rarity: "Normal" })).toBe("nonunique");
  expect(searchRarity({ ...RARE(""), rarity: "Unique" })).toBeNull();
  expect(searchRarity({ ...RARE(""), rarity: "Magic", itemClass: "Jewels" })).toBe("magic");
});

test("a ticked mark wins over the exclusion of the same filter", () => {
  const item = RARE("");
  const q = JSON.parse(applyFilters(JSON.stringify({ query: {} }), { pseudo: [], itemLevel: null, quality: null, links: null, corrupted: "any", priceIn: "any", exclude: ["mirrored"], marks: [{ line: "Mirrored", checked: true, kind: "mirrored", misc: "mirrored" }] }));
  expect(q.query.filters.misc_filters.filters.mirrored).toEqual({ option: "true" });
  expect(item.rarity).toBe("Rare");
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
