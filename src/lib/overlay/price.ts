/**
 * Price check helpers: read the item the game copied, pick the poe.ninja tables that could
 * price it, find its line, and write prices the way players say them.
 */
import type { BuySimilarInfo } from "$lib/engine.svelte";

export type Game = "poe1" | "poe2";

export interface CopiedItem {
  itemClass: string;
  rarity: string;
  name: string;
  baseType: string | null;
  stack: number | null;
  gemLevel: number | null;
  gemQuality: number | null;
  /** The item's quality (the plain "Quality:" line, not a catalyst's), null for none. */
  quality: number | null;
  corrupted: boolean;
  /** Largest linked socket group (PoE1). */
  links: number | null;
  mapTier: number | null;
  /** A Foulborn unique's mutated mod lines, as copied without the "(mutated)" mark. */
  mutated: string[];
  /** The marks the game writes on their own line: influences, Fractured Item, Mirrored… */
  marks: string[];
}

/** Mirrors `NinjaLine` / `NinjaTable` in src-tauri/src/price.rs. */
export interface NinjaLine {
  name: string;
  baseType: string | null;
  variant: string | null;
  gemLevel: number | null;
  gemQuality: number | null;
  corrupted: boolean;
  links: number | null;
  mapTier: number | null;
  /** A Foulborn line's mutated mods. */
  mutated?: string[];
  value: number;
  listings: number | null;
  /** Seven-day change in percent, and the daily points behind it (null: no data that day). */
  change: number | null;
  trend: (number | null)[];
}
export interface NinjaTable {
  primary: string;
  rates: Record<string, number>;
  lines: NinjaLine[];
}
export interface NinjaSource {
  kind: "exchange" | "stash";
  category: string;
}

const num = (s: string | undefined) => (s === undefined ? null : Number(s.replace(/[,.\s]/g, "")) || null);

export function parseCopied(text: string): CopiedItem | null {
  const lines = text.replace(/\r/g, "").split("\n").map((l) => l.trim());
  const sections: string[][] = [[]];
  for (const l of lines) {
    if (/^-{4,}$/.test(l)) sections.push([]);
    else if (l) sections[sections.length - 1].push(l);
  }
  const head = sections[0];
  const field = (prefix: string) => head.find((l) => l.startsWith(prefix))?.slice(prefix.length).trim();
  const itemClass = field("Item Class:");
  const rarity = field("Rarity:");
  if (!itemClass || !rarity) return null;
  const names = head.filter((l) => !l.startsWith("Item Class:") && !l.startsWith("Rarity:"));
  if (!names.length) return null;
  const all = sections.flat();
  const after = (prefix: string) => all.find((l) => l.startsWith(prefix))?.slice(prefix.length).trim();
  const twoNames = rarity === "Unique" || rarity === "Rare";
  const sockets = after("Sockets:");
  const links = sockets ? Math.max(...sockets.split(/\s+/).map((g) => g.split("-").length)) : null;
  const isGem = rarity === "Gem";
  return {
    itemClass,
    rarity,
    name: names[0],
    baseType: twoNames ? (names[1] ?? null) : null,
    stack: num(after("Stack Size:")?.split("/")[0]),
    gemLevel: isGem ? num(after("Level:")?.match(/^\d+/)?.[0]) : null,
    gemQuality: num(after("Quality:")?.match(/\d+/)?.[0]) ?? (isGem ? 0 : null),
    quality: num(after("Quality:")?.match(/\d+/)?.[0]),
    corrupted: all.includes("Corrupted"),
    links,
    mapTier: num(after("Map Tier:") ?? after("Waystone Tier:")),
    mutated: all.filter((l) => /\(mutated\)$/i.test(l)).map((l) => l.replace(/\s*\(mutated\)$/i, "")),
    marks: all.filter((l) => MARKS.has(l)),
  };
}

/** Lines the game writes alone on a copied item to say what it is. */
const MARKS = new Set([
  "Shaper Item", "Elder Item", "Crusader Item", "Redeemer Item", "Hunter Item", "Warlord Item",
  "Searing Exarch Item", "Eater of Worlds Item", "Fractured Item", "Synthesised Item", "Mirrored", "Split", "Sanctified",
]);

/** A mod line with its numbers and ranges taken out, to compare rolls of the same mod. */
const shape = (line: string) => line.replace(/\(?[+-]?\d+(\.\d+)?(-\d+(\.\d+)?)?\)?/g, "#").replace(/\s+/g, " ").trim().toLowerCase();

const ARMOUR = new Set(["Body Armours", "Helmets", "Gloves", "Boots", "Shields", "Quivers", "Bucklers", "Foci"]);
const ACCESSORY = new Set(["Rings", "Amulets", "Belts"]);
const JEWEL = new Set(["Jewels", "Abyss Jewels"]);

const POE1_UNIQUES = ["UniqueWeapon", "UniqueArmour", "UniqueAccessory", "UniqueFlask", "UniqueJewel"];
const POE2_UNIQUES = ["UniqueWeapons", "UniqueArmours", "UniqueAccessories", "UniqueFlasks", "UniqueJewels", "UniqueCharms"];
const POE1_EXCHANGE = ["Currency", "Fragment", "Scarab", "Essence", "Fossil", "Resonator", "Oil", "DeliriumOrb", "Incubator", "Omen", "Tattoo", "Artifact", "Invitation"];
const POE2_EXCHANGE = ["Currency", "Fragments", "Essences", "Runes", "SoulCores", "Ritual", "Expedition", "Delirium", "Breach", "Abyss", "Idols"];

function uniqueGuess(game: Game, itemClass: string): string {
  const [weapon, armour, accessory, flask, jewel, charm] = game === "poe1" ? POE1_UNIQUES : POE2_UNIQUES;
  if (ARMOUR.has(itemClass)) return armour;
  if (ACCESSORY.has(itemClass)) return accessory;
  if (itemClass.includes("Flask")) return flask;
  if (JEWEL.has(itemClass)) return jewel;
  if (itemClass === "Charms" && charm) return charm;
  return weapon;
}

/** poe.ninja tables that could hold the item, most likely first. Empty when poe.ninja does not price it. */
export function ninjaSources(game: Game, item: CopiedItem): NinjaSource[] {
  const exchange = (list: string[]) => list.map((category) => ({ kind: "exchange" as const, category }));
  const stash = (list: string[]) => list.map((category) => ({ kind: "stash" as const, category }));
  const { rarity, itemClass } = item;
  if (rarity === "Unique") {
    if (game === "poe1" && itemClass === "Maps") return stash(["UniqueMap"]);
    const guess = uniqueGuess(game, itemClass);
    const all = game === "poe1" ? POE1_UNIQUES : POE2_UNIQUES;
    return stash([guess, ...all.filter((c) => c !== guess)]);
  }
  if (rarity === "Gem") return game === "poe1" ? stash(["SkillGem"]) : exchange(["UncutGems", "LineageSupportGems"]);
  if (itemClass === "Divination Cards" || rarity === "Divination Card") return game === "poe1" ? exchange(["DivinationCard"]) : [];
  if (rarity === "Currency" || /Currency|Omen|Socketable|Fragment|Incubator|Splinter|Scarab|Map Items|Tattoo|Rune|Soul Core|Talisman|Idol/i.test(itemClass)) {
    return exchange(game === "poe1" ? POE1_EXCHANGE : POE2_EXCHANGE);
  }
  return [];
}

export interface NinjaGroup {
  /** Shown to the player. */
  label: string;
  sources: NinjaSource[];
  /** Ticked when a highlight regex is first built. */
  wanted: boolean;
}

/**
 * The poe.ninja tables a Ritual window (or a stash) can show, grouped as players think of them.
 * All of them are loaded for a highlight regex, ticked or not, so its pieces miss everything else.
 */
export function ninjaGroups(game: Game): NinjaGroup[] {
  const ex = (...c: string[]) => c.map((category) => ({ kind: "exchange" as const, category }));
  const st = (...c: string[]) => c.map((category) => ({ kind: "stash" as const, category }));
  if (game === "poe1") {
    return [
      { label: "Currency", sources: ex("Currency"), wanted: true },
      { label: "Divination cards", sources: ex("DivinationCard"), wanted: true },
      { label: "Fragments", sources: ex("Fragment"), wanted: true },
      { label: "Scarabs", sources: ex("Scarab"), wanted: true },
      { label: "Omens", sources: ex("Omen"), wanted: true },
      { label: "Essences", sources: ex("Essence"), wanted: false },
      { label: "Fossils & resonators", sources: ex("Fossil", "Resonator"), wanted: false },
      { label: "Oils", sources: ex("Oil"), wanted: false },
      { label: "Delirium orbs", sources: ex("DeliriumOrb"), wanted: false },
      { label: "Tattoos", sources: ex("Tattoo"), wanted: false },
      { label: "Uniques", sources: st(...POE1_UNIQUES), wanted: false },
      { label: "Gems", sources: st("SkillGem"), wanted: false },
    ];
  }
  return [
    { label: "Currency", sources: ex("Currency"), wanted: true },
    { label: "Omens & ritual", sources: ex("Ritual"), wanted: true },
    { label: "Fragments", sources: ex("Fragments"), wanted: true },
    { label: "Essences", sources: ex("Essences"), wanted: false },
    { label: "Runes & soul cores", sources: ex("Runes", "SoulCores", "Idols"), wanted: false },
    { label: "Expedition, Delirium, Breach, Abyss", sources: ex("Expedition", "Delirium", "Breach", "Abyss"), wanted: false },
    { label: "Uniques", sources: st(...POE2_UNIQUES), wanted: false },
    { label: "Gems", sources: ex("UncutGems", "LineageSupportGems"), wanted: false },
  ];
}

/** A line's value in chaos (PoE1) or exalted orbs (PoE2), the units prices are asked in. */
export function valueIn(game: Game, value: number, table: Pick<NinjaTable, "primary" | "rates">): number {
  const unit = game === "poe1" ? "chaos" : "exalted";
  if (table.primary === unit) return value;
  const rate = table.rates[unit];
  return rate ? value * rate : value;
}

/** One value per name: the line most listed, as a plain copy of the item would be. */
export function valuesByName(game: Game, table: NinjaTable): Map<string, number> {
  const best = new Map<string, NinjaLine>();
  for (const l of table.lines) {
    const had = best.get(l.name);
    if (!had || (l.listings ?? 0) > (had.listings ?? 0)) best.set(l.name, l);
  }
  return new Map([...best].map(([name, l]) => [name, valueIn(game, l.value, table)]));
}

/** The item's line in a table, or null. Gems match on level, quality and corruption; uniques on links. */
export function matchNinja(item: CopiedItem, table: NinjaTable): NinjaLine | null {
  let found = table.lines.filter((l) => l.name === item.name);
  if (!found.length) return null;
  if (item.baseType) {
    const sameBase = found.filter((l) => !l.baseType || l.baseType === item.baseType);
    if (sameBase.length) found = sameBase;
  }
  if (item.rarity === "Gem") {
    const cost = (l: NinjaLine) =>
      Math.abs((l.gemLevel ?? 1) - (item.gemLevel ?? 1)) * 10 + Math.abs((l.gemQuality ?? 0) - (item.gemQuality ?? 0)) + (l.corrupted !== item.corrupted ? 100 : 0);
    return [...found].sort((a, b) => cost(a) - cost(b))[0];
  }
  if (item.rarity === "Unique") {
    const linked = (item.links ?? 0) >= 5 ? item.links : null;
    const byLinks = found.filter((l) => (l.links ?? null) === linked || (!linked && !l.links));
    if (byLinks.length) found = byLinks;
    if (!item.mutated.length && /^Foulborn /.test(item.name) && found.some((l) => l.mutated?.length)) return null;
    if (item.mutated.length && found.some((l) => l.mutated?.length)) {
      // Foulborn: poe.ninja prices each set of mutated mods apart.
      const mine = new Set(item.mutated.map(shape));
      const same = found.filter((l) => l.mutated?.length && l.mutated.every((m) => mine.has(shape(m))));
      if (same.length) return [...same].sort((a, b) => (b.mutated!.length - a.mutated!.length) || (b.listings ?? 0) - (a.listings ?? 0))[0];
      return null;
    }
    const plain = found.filter((l) => !l.variant || l.links);
    if (plain.length) found = plain;
  }
  return [...found].sort((a, b) => (b.listings ?? 0) - (a.listings ?? 0))[0];
}

const SHORT: Record<string, string> = { chaos: "c", divine: "div", exalted: "ex" };
const round = (v: number) => (v >= 100 ? Math.round(v).toLocaleString("en") : v >= 10 ? v.toFixed(1).replace(/\.0$/, "") : v.toFixed(2).replace(/\.?0+$/, ""));

/** A value in the table's primary currency, in the unit players would quote it in. */
export function formatValue(value: number, table: Pick<NinjaTable, "primary" | "rates">): string {
  if (table.primary === "divine") {
    if (value >= 1) return `${round(value)} div`;
    const ex = table.rates.exalted;
    if (ex && value * ex >= 1) return `${round(value * ex)} ex`;
    const chaos = table.rates.chaos;
    return chaos ? `${round(value * chaos)} c` : `${round(value)} div`;
  }
  const div = table.rates.divine;
  if (div && value * div >= 1) return `${round(value * div)} div`;
  return `${round(value)} ${SHORT[table.primary] ?? table.primary}`;
}

export function formatListing(amount: number | null, currency: string | null): string {
  if (amount === null || !currency) return "—";
  return `${round(amount)} ${SHORT[currency] ?? currency}`;
}

/** How long ago a listing was indexed: "5m", "3h", "2d". */
export function age(indexed: string | null, now = Date.now()): string {
  if (!indexed) return "";
  const minutes = Math.max(0, Math.round((now - Date.parse(indexed)) / 60000));
  if (Number.isNaN(minutes)) return "";
  if (minutes < 60) return `${minutes}m`;
  if (minutes < 48 * 60) return `${Math.round(minutes / 60)}h`;
  return `${Math.round(minutes / 1440)}d`;
}

export interface Choice {
  checked: boolean;
  min: string;
  max: string;
}

/**
 * Starting filters, as a price check usually wants them: a unique is found by name alone; a rare
 * by its searchable mods at 90% of their rolls, so close rolls still match.
 */
export function defaultChoices(info: BuySimilarInfo): { mods: Choice[]; defences: Choice[] } {
  const mods = info.mods.map((m) => {
    const wanted = !info.unique && m.searchable && (m.type === "explicit" || m.type === "implicit" || m.type === "enchant");
    const min = m.ranged && typeof m.value === "number" && m.value !== 0 ? String(m.value > 0 ? Math.floor(m.value * 0.9) : m.value) : "";
    return { checked: wanted, min, max: "" };
  });
  const defences = info.defences.map((d) => ({ checked: false, min: String(Math.floor(d.value * 0.9)), max: "" }));
  return { mods, defences };
}

/** Items whose quality changes what they do beyond defences and damage: flasks, tinctures, charms. */
const QUALITY_MATTERS = /Flask|Tincture|Charm/i;

/**
 * The quality to search for, as Awakened PoE Trade sets it: on flasks, tinctures and charms the
 * quality itself; on anything else only quality past the usual 20%, which costs extra to reach.
 * Below that the trade site already compares defences and damage at 20%.
 */
export function defaultQuality(item: CopiedItem): string {
  const q = item.quality ?? 0;
  if (q <= 0 || item.rarity === "Gem") return "";
  if (QUALITY_MATTERS.test(item.itemClass) || q > 20) return String(q);
  return "";
}

export type DpsKey = "pdps" | "edps";
export interface DpsChoice extends Choice {
  key: DpsKey;
  value: number;
}

/** A weapon's physical and elemental DPS rows; on a rare the larger one is ticked, at 90%. */
export function weaponChoices(info: BuySimilarInfo): DpsChoice[] {
  const w = info.weapon;
  if (!w) return [];
  const rows = ([["pdps", w.pdps], ["edps", w.edps]] as const).filter(([, v]) => v > 0);
  const best: DpsKey = w.pdps >= w.edps ? "pdps" : "edps";
  return rows.map(([key, value]) => ({ key, value, checked: !info.unique && key === best, min: String(Math.floor(value * 0.9)), max: "" }));
}

/** The trade site's pseudo stats: one filter for a total that several mods add to. */
export const PSEUDO = {
  life: { id: "pseudo.pseudo_total_life", label: "+# total maximum Life" },
  es: { id: "pseudo.pseudo_total_energy_shield", label: "+# total maximum Energy Shield" },
  ele: { id: "pseudo.pseudo_total_elemental_resistance", label: "+#% total Elemental Resistance" },
  chaos: { id: "pseudo.pseudo_total_chaos_resistance", label: "+#% total to Chaos Resistance" },
} as const;
export type PseudoKey = keyof typeof PSEUDO;

export interface PseudoTotal {
  key: PseudoKey;
  total: number;
  /** Indexes into the Buy Similar mods that make up the total. */
  mods: number[];
}

const plainLine = (s: string) => s.replace(/\^x[0-9a-f]{6}|\^\d/gi, "").trim();
const ELEMENT = "(?:Fire|Cold|Lightning)";

/** What one mod line adds to a pseudo total, if anything. */
function contribution(line: string, armour: boolean): [PseudoKey, number] | null {
  const l = plainLine(line);
  let m: RegExpMatchArray | null;
  if ((m = l.match(new RegExp(`^([+-]?\\d+)% to ${ELEMENT} Resistance$`)))) return ["ele", +m[1]];
  if ((m = l.match(new RegExp(`^([+-]?\\d+)% to ${ELEMENT} and ${ELEMENT} Resistances$`)))) return ["ele", 2 * +m[1]];
  if ((m = l.match(/^([+-]?\d+)% to all Elemental Resistances$/))) return ["ele", 3 * +m[1]];
  if ((m = l.match(/^([+-]?\d+)% to Chaos Resistance$/))) return ["chaos", +m[1]];
  if ((m = l.match(new RegExp(`^([+-]?\\d+)% to ${ELEMENT} and Chaos Resistances$`)))) return ["chaos", +m[1]];
  if ((m = l.match(/^([+-]?\d+) to maximum Life$/))) return ["life", +m[1]];
  // On armour, flat energy shield is local and already in the item's own value.
  if (!armour && (m = l.match(/^([+-]?\d+) to maximum Energy Shield$/))) return ["es", +m[1]];
  return null;
}

/** Totals the way Awakened PoE Trade searches by default: resistances and life as one number each. */
export function pseudoTotals(info: BuySimilarInfo): PseudoTotal[] {
  const armour = info.defences.length > 0;
  const out = new Map<PseudoKey, PseudoTotal>();
  info.mods.forEach((mod, i) => {
    if (!mod.searchable) return;
    for (const line of mod.lines) {
      const c = contribution(line, armour);
      if (!c) continue;
      const [key, v] = c;
      const p = out.get(key) ?? { key, total: 0, mods: [] };
      p.total += v;
      if (!p.mods.includes(i)) p.mods.push(i);
      out.set(key, p);
    }
    // "+12% to Fire and Chaos Resistances" also adds to the elemental total.
    for (const line of mod.lines) {
      const m = plainLine(line).match(new RegExp(`^([+-]?\\d+)% to ${ELEMENT} and Chaos Resistances$`));
      if (!m) continue;
      const p = out.get("ele") ?? { key: "ele", total: 0, mods: [] };
      p.total += +m[1];
      if (!p.mods.includes(i)) p.mods.push(i);
      out.set("ele", p);
    }
  });
  return [...out.values()].filter((p) => p.total > 0);
}

export interface Mark {
  /** The line as the game wrote it. */
  line: string;
  checked: boolean;
  /** A trade misc filter set to true, or a pseudo stat to require. */
  misc?: string;
  pseudo?: string;
}

const INFLUENCE = ["Shaper", "Elder", "Crusader", "Redeemer", "Hunter", "Warlord"];

/** The marks the trade site can search on, each ticked: they are part of what the item is. */
export function itemMarks(game: Game, item: CopiedItem): Mark[] {
  const out: Mark[] = [];
  for (const line of item.marks) {
    const influence = INFLUENCE.find((i) => line === `${i} Item`);
    if (influence && game === "poe1") out.push({ line, checked: true, pseudo: `pseudo.pseudo_has_${influence.toLowerCase()}_influence` });
    const misc: Record<string, string> = game === "poe1"
      ? { "Searing Exarch Item": "searing_item", "Eater of Worlds Item": "tangled_item", "Fractured Item": "fractured_item", "Synthesised Item": "synthesised_item", Mirrored: "mirrored", Split: "split" }
      : { "Fractured Item": "fractured_item", Mirrored: "mirrored", Sanctified: "sanctified" };
    if (misc[line]) out.push({ line, checked: true, misc: misc[line] });
  }
  return out;
}

export interface Filters {
  pseudo: { id: string; min: number | null }[];
  /** Weapon DPS minimums (counted by the trade site at 20% quality). */
  dps?: { key: DpsKey; min: number | null }[];
  game?: Game;
  /** Ticked item marks. */
  marks?: Mark[];
  itemLevel: number | null;
  quality: number | null;
  links: number | null;
  /** "any", or whether the listing must be corrupted. */
  corrupted: "any" | "yes" | "no";
  /** Only listings priced in this currency; "any" for all. */
  priceIn: string;
}

/** Adds the price check's own filters to the query Buy Similar built. */
export function applyFilters(queryJson: string, f: Filters): string {
  const q = JSON.parse(queryJson);
  const query = q.query && typeof q.query === "object" ? q.query : q;
  query.filters ??= {};
  const group = (name: string) => ((query.filters[name] ??= { filters: {} }).filters ??= {});
  if (f.itemLevel !== null) group("misc_filters").ilvl = { min: f.itemLevel };
  if (f.quality !== null) group("misc_filters").quality = { min: f.quality };
  if (f.corrupted !== "any") group("misc_filters").corrupted = { option: f.corrupted === "yes" ? "true" : "false" };
  if (f.links !== null) group("socket_filters").links = { min: f.links };
  if (f.priceIn !== "any") group("trade_filters").price = { option: f.priceIn };
  // PoE2's site keeps weapon figures with the armour ones.
  const weapons = f.game === "poe2" ? "equipment_filters" : "weapon_filters";
  for (const d of f.dps ?? []) if (d.min !== null) group(weapons)[d.key] = { min: d.min };
  for (const mk of f.marks ?? []) if (mk.misc) group("misc_filters")[mk.misc] = { option: "true" };
  const pseudo = [
    ...f.pseudo.map((p) => (p.min === null ? { id: p.id } : { id: p.id, value: { min: p.min } })),
    ...(f.marks ?? []).filter((mk) => mk.pseudo).map((mk) => ({ id: mk.pseudo! })),
  ];
  if (pseudo.length) {
    query.stats ??= [];
    let and = query.stats.find((s: { type?: string }) => s.type === "and");
    if (!and) query.stats.push((and = { type: "and", filters: [] }));
    and.filters = [...(and.filters ?? []), ...pseudo];
  }
  return JSON.stringify(q);
}

/** The item's wiki page (uniques and gems) and its poedb page (bases and uniques). */
export function wikiUrl(game: Game, item: CopiedItem): string | null {
  if (item.rarity !== "Unique" && item.rarity !== "Gem" && item.rarity !== "Divination Card" && item.rarity !== "Currency") return null;
  const host = game === "poe1" ? "https://www.poewiki.net/wiki/" : "https://www.poe2wiki.net/wiki/";
  return host + encodeURIComponent(item.name.replace(/ /g, "_"));
}

export function poedbUrl(game: Game, item: CopiedItem): string {
  const host = game === "poe1" ? "https://poedb.tw/us/" : "https://poe2db.tw/us/";
  const name = item.rarity === "Unique" || !item.baseType ? item.name : item.baseType;
  return host + encodeURIComponent(name.replace(/ /g, "_"));
}

/** An SVG polyline for a sparkline, gaps skipped. */
export function sparkPoints(trend: (number | null)[], width: number, height: number): string {
  const known = trend.filter((v): v is number => v !== null);
  if (known.length < 2) return "";
  const lo = Math.min(...known), hi = Math.max(...known);
  const span = hi - lo || 1;
  const step = width / Math.max(1, trend.length - 1);
  return trend
    .map((v, i) => (v === null ? null : `${(i * step).toFixed(1)},${(height - ((v - lo) / span) * height).toFixed(1)}`))
    .filter(Boolean)
    .join(" ");
}

/**
 * Tags for one Buy Similar mod, as Awakened PoE Trade shows them: its kind (implicit, enchant…),
 * what the game wrote after it on the copied text ("(crafted)", "(fractured)"…), and eldritch for
 * the Searing Exarch and Eater of Worlds implicits. Explicit is the default and gets no tag.
 */
export function modTags(raw: string, mod: { type: string; lines: string[]; kinds?: string[] }): string[] {
  const marks = new Map<string, string[]>();
  for (const line of raw.replace(/\r/g, "").split("\n")) {
    const m = line.trim().match(/^(.*?)((?:\s*\([a-z ]+\))+)$/i);
    if (!m) continue;
    const tags = [...m[2].matchAll(/\(([a-z ]+)\)/gi)].map((x) => x[1].toLowerCase());
    marks.set(m[1].trim().toLowerCase(), tags);
  }
  const tags = new Set<string>();
  if (mod.type && mod.type !== "explicit") tags.add(mod.type);
  for (const kind of mod.kinds ?? []) tags.add(kind);
  for (const line of mod.lines) {
    const l = plainLine(line);
    for (const tag of marks.get(l.toLowerCase()) ?? []) tags.add(tag);
    if (/Searing Exarch|Eater of Worlds/.test(l)) tags.add("eldritch");
  }
  return [...tags];
}

/** A Chronicle of Atzoatl's rooms, as the game copies them. */
export function templeRooms(raw: string): { open: string[]; obstructed: string[] } {
  const out = { open: [] as string[], obstructed: [] as string[] };
  let into: string[] | null = null;
  for (const line of raw.replace(/\r/g, "").split("\n").map((l) => l.trim())) {
    if (/^Open Rooms:?$/i.test(line)) into = out.open;
    else if (/^Obstructed Rooms:?$/i.test(line)) into = out.obstructed;
    else if (/^-{4,}$/.test(line) || !line) into = line ? null : into;
    else if (into) into.push(line);
  }
  return out;
}

/** The temple rooms worth searching for first, as players price them. */
export const VALUABLE_ROOMS = [/Locus of Corruption/, /Doryani's Institute/, /Apex of Atzoatl/];

/** A room line ("Locus of Corruption (Tier 3)") to its trade filter, by the filter's text. */
export function roomFilter(room: string, stats: { id: string; text: string }[]): string | null {
  const want = `Has Room: ${room}`.toLowerCase();
  return stats.find((s) => s.text.toLowerCase() === want)?.id ?? null;
}

export function isTemple(item: CopiedItem): boolean {
  return item.name === "Chronicle of Atzoatl" || item.baseType === "Chronicle of Atzoatl";
}

export function isMap(item: CopiedItem): boolean {
  return item.itemClass === "Maps" || item.itemClass === "Waystones" || item.mapTier !== null;
}

export interface MapStat {
  label: string;
  value: number;
  filter: string;
  checked: boolean;
  min: string;
}

/** A map's or waystone's quantity, rarity and pack figures, each with its trade filter; unticked. */
export function mapStats(game: Game, raw: string): MapStat[] {
  const filters: [RegExp, string][] = game === "poe1"
    ? [[/^Item Quantity:/, "map_iiq"], [/^Item Rarity:/, "map_iir"], [/^Monster Pack Size:/, "map_packsize"]]
    : [[/^Item Rarity:/, "map_iir"], [/^(Monster )?Pack Size:/, "map_packsize"], [/^Magic Monsters:/, "map_magic_monsters"], [/^Rare Monsters:/, "map_rare_monsters"]];
  const out: MapStat[] = [];
  for (const line of raw.replace(/\r/g, "").split("\n").map((l) => l.trim())) {
    const hit = filters.find(([re]) => re.test(line));
    const value = Number(line.match(/[+-]?(\d+)%/)?.[1]);
    if (hit && value > 0) out.push({ label: line.split(":")[0], value, filter: hit[1], checked: false, min: String(value) });
  }
  return out;
}

/** Blighted and blight-ravaged maps are their own market (PoE1). */
export function blight(item: CopiedItem): "map_blighted" | "map_uberblighted" | null {
  const text = `${item.name} ${item.baseType ?? ""}`;
  return /Blight-ravaged/i.test(text) ? "map_uberblighted" : /\bBlighted\b/i.test(text) ? "map_blighted" : null;
}

export interface SimpleChoices {
  /** Temple room filters to require. */
  rooms: string[];
  gemLevel: number | null;
  quality: number | null;
  mapTier: number | null;
  corrupted: "any" | "yes" | "no";
  /** Ticked map figures. */
  mapStats?: { filter: string; min: number | null }[];
  /** Blighted or blight-ravaged, when ticked. */
  blight?: string | null;
}

/**
 * A trade search built here, for what PoB's Buy Similar does not read: temples, gems, maps and
 * waystones, and anything else by its name and base.
 */
export function simpleQuery(item: CopiedItem, c: SimpleChoices): string {
  const query: Record<string, any> = { status: { option: "available" } };
  const misc: Record<string, unknown> = {};
  if (isTemple(item)) {
    query.type = "Chronicle of Atzoatl";
    if (c.rooms.length) query.stats = [{ type: "and", filters: c.rooms.map((id) => ({ id })) }];
  } else if (item.rarity === "Gem") {
    query.type = item.name;
    if (c.gemLevel !== null) misc.gem_level = { min: c.gemLevel };
    if (c.quality !== null) misc.quality = { min: c.quality };
  } else if (isMap(item)) {
    // Maps are priced by tier: since 3.26 their names no longer tell them apart on the trade site.
    if (item.rarity === "Unique") query.name = item.name;
    query.filters = { type_filters: { filters: { category: { option: item.itemClass === "Waystones" ? "map.waystone" : "map" } } } };
    const mapf: Record<string, unknown> = {};
    if (c.mapTier !== null) mapf.map_tier = { min: c.mapTier, max: c.mapTier };
    for (const s of c.mapStats ?? []) if (s.min !== null) mapf[s.filter] = { min: s.min };
    if (c.blight) mapf[c.blight] = { option: "true" };
    if (Object.keys(mapf).length) query.filters.map_filters = { filters: mapf };
  } else {
    if (item.rarity === "Unique") query.name = item.name;
    query.type = item.baseType ?? item.name;
    if (c.quality !== null) misc.quality = { min: c.quality };
  }
  if (c.corrupted !== "any") misc.corrupted = { option: c.corrupted === "yes" ? "true" : "false" };
  if (Object.keys(misc).length) (query.filters ??= {}).misc_filters = { filters: misc };
  return JSON.stringify({ query, sort: { price: "asc" } });
}
