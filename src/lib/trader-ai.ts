// The Trader's request field: the assistant turns a sentence into Trader settings, and the
// Trader runs its own search with them. Path of Building scores every listing; the model only
// picks slots, weights and filters from the lists given to it, and anything else is dropped.

export interface AskContext {
  rows: { key: string; label: string; itemName: string | null }[];
  /** Path of Building's weighting stats: the id the Trader stores, and its label. */
  stats: { stat: string; label: string }[];
  currencies: { id: string; name: string }[];
  game: "poe1" | "poe2";
}

export interface ModRequest {
  text: string;
  min: number | null;
}

export interface AskPlan {
  rows: string[];
  weights: { stat: string; weightMult: number }[] | null;
  maxPrice: number | null;
  maxPriceType: string | null;
  maxLevel: number | null;
  corrupted: boolean | null;
  required: Record<string, ModRequest[]>;
  blocked: Record<string, ModRequest[]>;
  /** What the model said it could not do, shown to the user. */
  note: string | null;
}

export const ASK_INSTRUCTIONS = `You set up Path of Building's Trader, a trade-site search that Path of Building scores.
Answer with one JSON object and nothing else:
{"rows": [row keys], "weights": [{"stat": stat id, "weightMult": number}] or null, "maxPrice": number or null,
 "maxPriceType": currency id or null, "maxLevel": number or null, "corrupted": true, false or null,
 "required": {row key: [{"text": mod, "min": number or null}]}, "blocked": {row key: [{"text": mod}]}, "note": string or null}
Use only the row keys, stat ids and currency ids listed. "rows" are the slots to search; keep it to what the request names.
"weights" are what the listings are scored by (Path of Building computes the numbers); null keeps the current ones.
Write a mod as the trade site writes it, with # for the number, for example "+# to maximum Life" or "#% increased Movement Speed";
"min" is the least roll. "corrupted": false leaves corrupted items out. Put anything you cannot set in "note", in one short sentence.
Never estimate damage, defences or prices yourself.`;

export function askPrompt(request: string, ctx: AskContext): string {
  const rows = ctx.rows.map((r) => `${r.key}: ${r.label}${r.itemName ? ` (now ${r.itemName})` : " (empty)"}`).join("\n");
  const stats = ctx.stats.map((s) => `${s.stat}: ${s.label}`).join("\n");
  const currencies = ctx.currencies.map((c) => `${c.id}: ${c.name}`).join("\n");
  return `Game: ${ctx.game === "poe2" ? "Path of Exile 2" : "Path of Exile 1"}\n\n<rows>\n${rows}\n</rows>\n\n<stats>\n${stats}\n</stats>\n\n<currencies>\n${currencies}\n</currencies>\n\n<request>\n${request.slice(0, 2000)}\n</request>`;
}

/** The first balanced JSON object in the answer: models wrap it in prose or code fences. */
function firstObject(raw: string): unknown {
  const start = raw.indexOf("{");
  if (start < 0) throw new Error("no JSON object in the answer");
  let depth = 0;
  let inString = false;
  for (let i = start; i < raw.length; i++) {
    const c = raw[i];
    if (inString) {
      if (c === "\\") i++;
      else if (c === '"') inString = false;
    } else if (c === '"') inString = true;
    else if (c === "{") depth++;
    else if (c === "}" && --depth === 0) return JSON.parse(raw.slice(start, i + 1));
  }
  throw new Error("the JSON object in the answer is not closed");
}

const num = (v: unknown) => (typeof v === "number" && Number.isFinite(v) ? v : null);

function mods(v: unknown, rows: Set<string>): Record<string, ModRequest[]> {
  const out: Record<string, ModRequest[]> = {};
  if (!v || typeof v !== "object") return out;
  for (const [key, list] of Object.entries(v as Record<string, unknown>)) {
    if (!rows.has(key) || !Array.isArray(list)) continue;
    const kept = list
      .map((x) => (typeof x === "string" ? { text: x } : x))
      .filter((x): x is { text: string; min?: unknown } => !!x && typeof (x as { text?: unknown }).text === "string" && (x as { text: string }).text.trim() !== "")
      .slice(0, 5)
      .map((x) => ({ text: x.text.trim(), min: num(x.min) }));
    if (kept.length) out[key] = kept;
  }
  return out;
}

/** Reads the model's answer, keeping only what names a listed row, stat or currency. */
export function parsePlan(raw: string, ctx: AskContext): AskPlan {
  const o = firstObject(raw) as Record<string, unknown>;
  if (!o || typeof o !== "object" || Array.isArray(o)) throw new Error("the answer is not a JSON object");
  const rowKeys = new Set(ctx.rows.map((r) => r.key));
  const statIds = new Set(ctx.stats.map((s) => s.stat));
  const rows = Array.isArray(o.rows) ? [...new Set(o.rows.filter((k): k is string => typeof k === "string" && rowKeys.has(k)))] : [];
  let weights: AskPlan["weights"] = null;
  if (Array.isArray(o.weights)) {
    const kept = o.weights
      .filter((w): w is { stat: string; weightMult: unknown } => !!w && typeof w === "object" && statIds.has((w as { stat?: string }).stat ?? ""))
      .map((w) => ({ stat: w.stat, weightMult: Math.max(0, Math.min(10, num(w.weightMult) ?? 1)) }))
      .filter((w) => w.weightMult > 0);
    weights = kept.length ? [...new Map(kept.map((w) => [w.stat, w])).values()] : null;
  }
  const maxPrice = num(o.maxPrice);
  const type = typeof o.maxPriceType === "string" && ctx.currencies.some((c) => c.id === o.maxPriceType) ? o.maxPriceType : null;
  const level = num(o.maxLevel);
  return {
    rows,
    weights,
    maxPrice: maxPrice !== null && maxPrice > 0 ? maxPrice : null,
    maxPriceType: maxPrice !== null && maxPrice > 0 ? type : null,
    maxLevel: level !== null && level >= 1 && level <= 100 ? Math.round(level) : null,
    corrupted: typeof o.corrupted === "boolean" ? o.corrupted : null,
    required: mods(o.required, new Set(rows)),
    blocked: mods(o.blocked, new Set(rows)),
    note: typeof o.note === "string" && o.note.trim() ? o.note.trim().slice(0, 300) : null,
  };
}

/** Mod text with every number as #, signs and spacing evened out, for comparing trade-site wording. */
export function modKey(text: string): string {
  return text
    .toLowerCase()
    .replace(/\(\w+\)\s*$/, "")
    .replace(/[+-]?\d+(\.\d+)?/g, "#")
    .replace(/[+]?#/g, "#")
    .replace(/\s+/g, " ")
    .trim();
}

/**
 * The trade stat a requested mod names: the same wording, else the shortest that starts with it.
 * Only a prefix counts, so "#% increased Movement Speed" never becomes the minions' version.
 */
export function matchMod(text: string, list: { tradeId: string; text: string; type: string }[]): { tradeId: string; text: string } | null {
  const want = modKey(text);
  if (!want) return null;
  const keyed = list.map((m) => ({ m, key: modKey(m.text) }));
  const exact = keyed.filter((k) => k.key === want);
  const pick = (xs: typeof keyed) => xs.sort((a, b) => (a.m.type === "Explicit" ? 0 : 1) - (b.m.type === "Explicit" ? 0 : 1) || a.key.length - b.key.length)[0];
  const hit = exact.length ? pick(exact) : pick(keyed.filter((k) => k.key.startsWith(want)));
  return hit ? { tradeId: hit.m.tradeId, text: hit.m.text } : null;
}
