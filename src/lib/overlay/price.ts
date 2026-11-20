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
  corrupted: boolean;
  /** Largest linked socket group (PoE1). */
  links: number | null;
  mapTier: number | null;
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
    corrupted: all.includes("Corrupted"),
    links,
    mapTier: num(after("Map Tier:")),
  };
}

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

export interface Filters {
  pseudo: { id: string; min: number | null }[];
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
  const pseudo = f.pseudo.map((p) => (p.min === null ? { id: p.id } : { id: p.id, value: { min: p.min } }));
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
