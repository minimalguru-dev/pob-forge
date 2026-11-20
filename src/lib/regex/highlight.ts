/**
 * Search-box regexes for the game: the stash, vendor and Ritual windows highlight what matches
 * the text typed into their search box, and that text is a regex. The app only builds the text;
 * the player pastes it into the game themselves.
 */

export interface Named {
  name: string;
  /** What the item is worth, in the unit the threshold uses. */
  value: number;
}

export interface Piece {
  piece: string;
  /** The wanted names it highlights. */
  names: string[];
}

export interface Highlight {
  /** What to paste, quotes included; empty when nothing could be matched. */
  regex: string;
  pieces: Piece[];
  /** Wanted names the regex highlights, most valuable first. */
  covered: Named[];
  /** Wanted names left out because the character limit was reached. */
  overLimit: Named[];
  /** Wanted names that share every piece of their name with something unwanted. */
  noPiece: Named[];
}

export interface HighlightOptions {
  /** The search box's character limit, quotes included. */
  limit: number;
  /** The shortest piece of a name to use; one- and two-letter pieces match too much of a tooltip. */
  minPiece: number;
  /** The longest piece of a name tried. */
  maxPiece?: number;
}

const norm = (s: string) => s.toLowerCase();
/**
 * Letters, digits, apostrophes and spaces: the whole regex goes in quotes, so a piece may cross
 * words ("d orb" tells Exalted Orb from Exalted Shard). Other characters would need escaping.
 */
const PLAIN = /^[a-z0-9' ]+$/;

function pieces(name: string, min: number, max: number, into: (p: string) => void) {
  const n = norm(name);
  for (let len = min; len <= max; len++) {
    for (let i = 0; i + len <= n.length; i++) {
      const p = n.slice(i, i + len);
      if (PLAIN.test(p)) into(p);
    }
  }
}

/**
 * Every name the window could show, broken into pieces once: each piece with the names it is in.
 * Built once per price table, so changing the price or the groups only redoes the quick part.
 */
export interface PieceIndex {
  min: number;
  max: number;
  /** Lower-cased names, without repeats. */
  names: string[];
  ids: Map<string, number>;
  byPiece: Map<string, number[]>;
}

export function indexNames(names: Iterable<string>, min: number, max = 10): PieceIndex {
  const ids = new Map<string, number>();
  const list: string[] = [];
  for (const name of names) {
    const n = norm(name);
    if (!ids.has(n)) {
      ids.set(n, list.length);
      list.push(n);
    }
  }
  const byPiece = new Map<string, number[]>();
  list.forEach((n, id) =>
    pieces(n, min, max, (p) => {
      const at = byPiece.get(p);
      if (!at) byPiece.set(p, [id]);
      else if (at[at.length - 1] !== id) at.push(id);
    }),
  );
  return { min, max, names: list, ids, byPiece };
}

/**
 * The shortest regex that highlights the wanted names and none of the others, as far as the
 * limit allows: pieces of the wanted names found in no other name, chosen for the most value
 * per character. The others should be every name the window could show (the whole price table),
 * since the search matches any of them.
 */
export function buildHighlight(wanted: Named[], others: string[], o: HighlightOptions): Highlight {
  return highlightFrom(indexNames([...others, ...wanted.map((w) => w.name)], o.minPiece, o.maxPiece ?? 10), wanted, o.limit);
}

/** buildHighlight against an index made by indexNames, which must hold every wanted name too. */
export function highlightFrom(index: PieceIndex, wanted: Named[], limit: number): Highlight {
  // Wanted names by their id in the index; a piece is usable when every name it is in is wanted.
  const wantedAt = new Map<number, number>();
  wanted.forEach((w, i) => {
    const id = index.ids.get(norm(w.name));
    if (id !== undefined) wantedAt.set(id, i);
  });
  const covers = new Map<string, number[]>();
  for (const w of wanted) {
    pieces(w.name, index.min, index.max, (p) => {
      if (covers.has(p)) return;
      const ids = index.byPiece.get(p);
      if (!ids || !ids.every((id) => wantedAt.has(id))) return;
      covers.set(p, ids.map((id) => wantedAt.get(id)!));
    });
  }
  const reachable = new Set<number>();
  for (const set of covers.values()) for (const i of set) reachable.add(i);

  const chosen: string[] = [];
  const covered = new Set<number>();
  // Two quotes around it, a bar between pieces.
  let used = 2;
  const candidates = new Map(covers);
  for (;;) {
    const room = limit - used - (chosen.length ? 1 : 0);
    let best: string | null = null;
    let bestScore = 0;
    for (const [p, set] of candidates) {
      let gain = 0;
      for (const i of set) if (!covered.has(i)) gain += Math.max(wanted[i].value, 1e-6);
      if (gain === 0) {
        candidates.delete(p);
        continue;
      }
      // Only pieces that still fit are weighed, so a full regex ends the search at once.
      if (p.length > room) continue;
      const score = gain / (p.length + (chosen.length ? 1 : 0));
      if (score > bestScore || (score === bestScore && best !== null && p.length < best.length)) {
        best = p;
        bestScore = score;
      }
    }
    if (best === null) break;
    used += best.length + (chosen.length ? 1 : 0);
    chosen.push(best);
    candidates.delete(best);
    for (const i of covers.get(best)!) covered.add(i);
  }

  const byValue = (a: Named, b: Named) => b.value - a.value;
  const piecesOut = chosen.map((p) => ({ piece: p, names: covers.get(p)!.map((i) => wanted[i].name) }));
  return {
    regex: chosen.length ? `"${chosen.join("|")}"` : "",
    pieces: piecesOut,
    covered: wanted.filter((_, i) => covered.has(i)).sort(byValue),
    overLimit: wanted.filter((_, i) => !covered.has(i) && reachable.has(i)).sort(byValue),
    noPiece: wanted.filter((_, i) => !reachable.has(i)).sort(byValue),
  };
}

/** Saved regexes: the player's own and the ones built here. */
export interface SavedRegex {
  id: string;
  name: string;
  game: "poe1" | "poe2";
  regex: string;
}

const SAVED_KEY = "pob-forge:regexes";

export function loadSaved(): SavedRegex[] {
  try {
    const list = JSON.parse(localStorage.getItem(SAVED_KEY) ?? "[]");
    return Array.isArray(list) ? list.filter((r) => r && typeof r.regex === "string" && typeof r.name === "string") : [];
  } catch {
    return [];
  }
}

export function storeSaved(list: SavedRegex[]) {
  try {
    localStorage.setItem(SAVED_KEY, JSON.stringify(list));
  } catch {
    // Storage off: the list lasts until the app closes.
  }
}
