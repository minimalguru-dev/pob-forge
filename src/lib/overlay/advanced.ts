/**
 * The price check's extras, kept apart from price.ts: the game's advanced copy (Ctrl+Alt+C, mod
 * tiers and roll ranges), starting minimums from a roll range setting, listings grouped by
 * seller, prices read in one currency, and the trade site's wait.
 */

/** One mod as the advanced copy writes it: its header and the lines under it. */
export interface AdvancedMod {
  /** "prefix", "suffix", "implicit", "crafted prefix"…, from the header. */
  kind: string;
  /** The affix name in quotes, such as "Virile". */
  name: string | null;
  tier: number | null;
  /** Crafted and some other mods have a rank instead of a tier. */
  rank: number | null;
  lines: { text: string; rolls: Roll[] }[];
}

/** A number in a mod line with the range it could have rolled in. */
export interface Roll {
  value: number;
  min: number;
  max: number;
}

const HEADER = /^\{\s*(.*?)\s*\}$/;
/** "+64(55-68)", "-12(-15--10)", "1.5(1.2-1.8)". */
const RANGED = /([+-]?\d+(?:\.\d+)?)\((-?\d+(?:\.\d+)?)-(-?\d+(?:\.\d+)?)\)/g;

/** The copied text with the advanced copy's extras taken out, as a plain Ctrl+C would give it. */
export function stripAdvanced(raw: string): string {
  return raw
    .replace(/\r/g, "")
    .split("\n")
    .filter((line) => !HEADER.test(line.trim()))
    .map((line) => line.replace(RANGED, "$1").replace(/\s+—\s+Unscalable Value$/, ""))
    .join("\n");
}

/** Whether the text came from Ctrl+Alt+C. */
export function isAdvanced(raw: string): boolean {
  return raw.replace(/\r/g, "").split("\n").some((line) => HEADER.test(line.trim()) && /Modifier/.test(line));
}

export function advancedMods(raw: string): AdvancedMod[] {
  const out: AdvancedMod[] = [];
  let current: AdvancedMod | null = null;
  for (const rawLine of raw.replace(/\r/g, "").split("\n")) {
    const line = rawLine.trim();
    const header = HEADER.exec(line);
    if (header && /Modifier/.test(header[1])) {
      const head = header[1].split("—")[0].trim();
      current = {
        kind: head
          .replace(/"[^"]*"/, "")
          .replace(/\((Tier|Rank): \d+\)/g, "")
          .replace(/Modifier/, "")
          .replace(/Master Crafted/, "crafted")
          .trim()
          .toLowerCase(),
        name: /"([^"]+)"/.exec(head)?.[1] ?? null,
        tier: Number(/\(Tier: (\d+)\)/.exec(head)?.[1]) || null,
        rank: Number(/\(Rank: (\d+)\)/.exec(head)?.[1]) || null,
        lines: [],
      };
      out.push(current);
      continue;
    }
    if (!line || /^-{4,}$/.test(line) || header) {
      if (/^-{4,}$/.test(line)) current = null;
      continue;
    }
    if (!current) continue;
    const rolls = [...line.matchAll(RANGED)].map((m) => ({ value: Number(m[1]), min: Number(m[2]), max: Number(m[3]) }));
    const text = line.replace(RANGED, "$1").replace(/\s+—\s+Unscalable Value$/, "").replace(/(\s*\([a-z ]+\))+$/i, "").trim();
    current.lines.push({ text, rolls });
  }
  return out;
}

const norm = (s: string) => s.replace(/\^x[0-9a-f]{6}|\^\d/gi, "").replace(/\s+/g, " ").trim().toLowerCase();

/** The advanced mod a PoB mod row came from, matched by its lines' text. */
export function findAdvanced(mods: AdvancedMod[], lines: string[]): AdvancedMod | null {
  const wanted = lines.map(norm);
  return mods.find((m) => wanted.every((w) => m.lines.some((l) => norm(l.text) === w))) ?? null;
}

/** Awakened PoE Trade's decimals: none from 10 up, two below 2.3, one in between. */
export function decimalPlaces(value: number): number {
  const v = Math.abs(value);
  if (v >= 10 || Number.isInteger(value)) return 0;
  return v < 2.3 ? 2 : 1;
}

function rounded(x: number, dp: number, how: (n: number) => number): number {
  const f = 10 ** dp;
  return how((x + (x >= 0 ? Number.EPSILON : -Number.EPSILON)) * f) / f;
}

/**
 * The starting minimum for a rolled stat, as Awakened PoE Trade sets it: `range` percent under
 * the roll. When the roll's range is known (a unique, or any advanced copy) the step is a share
 * of that range, and the result never leaves it.
 */
export function rollMin(value: number, range: number, roll?: { min: number; max: number } | null, unique = false): number {
  const dp = decimalPlaces(value);
  if (roll && unique) {
    const lo = Math.min(roll.min, roll.max);
    return Math.max(rounded(value - (Math.abs(roll.max - roll.min) * range) / 100, dp, Math.floor), Math.floor(lo));
  }
  const min = rounded(value - (Math.abs(value) * range) / 100, dp, Math.floor);
  return roll ? Math.max(min, Math.floor(Math.min(roll.min, roll.max))) : min;
}

/** One mouse-wheel notch on a filter value: a whole step, a tenth or a hundredth as the value has. */
export function wheelStep(value: string, up: boolean, big: boolean): string {
  const n = Number(value);
  if (value.trim() === "" || Number.isNaN(n)) return up ? "1" : "";
  const dp = value.includes(".") ? Math.min(2, value.split(".")[1].length) : 0;
  const step = (dp === 0 ? 1 : 10 ** -dp) * (big ? 10 : 1);
  const next = n + (up ? step : -step);
  return dp ? next.toFixed(dp) : String(Math.round(next));
}

export interface Listing {
  amount: number | null;
  currency: string | null;
  indexed: string | null;
  account: string | null;
  online: boolean;
  stack: number | null;
}

export interface ListingRow extends Listing {
  /** How many of the seller's listings this row stands for. */
  times: number;
}

/**
 * Listings as Awakened PoE Trade shows them: a seller's listings at one price fold into one row,
 * and so does a seller who comes back within the last two rows. Stacks add up.
 */
export function groupListings(listings: Listing[]): ListingRow[] {
  const out: ListingRow[] = [];
  for (const l of listings) {
    const idx = l.account
      ? out.findIndex((r, i) => r.account === l.account && ((r.amount === l.amount && r.currency === l.currency) || out.length - i <= 2))
      : -1;
    if (idx < 0) {
      out.push({ ...l, times: 1 });
      continue;
    }
    const row = out[idx];
    if (row.stack !== null || l.stack !== null) row.stack = (row.stack ?? 1) + (l.stack ?? 1);
    else row.times += 1;
  }
  return out;
}

/** A price in the table's primary currency (chaos in PoE1, exalted or divine in PoE2), when known. */
export function inPrimary(amount: number | null, currency: string | null, rates: { primary: string; rates: Record<string, number> } | null): number | null {
  if (amount === null || !currency || !rates) return null;
  if (currency === rates.primary) return amount;
  const rate = rates.rates[currency];
  return rate ? amount / rate : null;
}

/** The anonymous trade client's wait, sent as "rate-limited:<seconds>". */
export function rateWait(error: string): number | null {
  const m = /rate-limited:(\d+)/.exec(error);
  return m ? Number(m[1]) : null;
}

/** "0:24", "1:05". */
export function clock(seconds: number): string {
  const s = Math.max(0, Math.ceil(seconds));
  return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, "0")}`;
}
