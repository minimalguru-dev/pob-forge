import { parsePobText, stripPobText, type Span } from "$lib/pobtext";

/** A piece of a notes line: coloured text, and a link when it is a web address. */
export interface Piece extends Span {
  href?: string;
}

export interface NoteLine {
  pieces: Piece[];
  /** Set when the line reads as a section title; the outline lists these. */
  heading?: string;
}

const URL = /https?:\/\/[^\s<>"'`]+/g;
// Closing punctuation after an address in prose belongs to the sentence, not the link.
const TRAILING = /[.,;:!?)\]}'"]+$/;

function linkify(span: Span): Piece[] {
  const out: Piece[] = [];
  let at = 0;
  for (const m of span.text.matchAll(URL)) {
    const href = m[0].replace(TRAILING, "");
    const start = m.index!;
    if (start > at) out.push({ text: span.text.slice(at, start), color: span.color });
    out.push({ text: href, color: span.color, href });
    at = start + href.length;
  }
  if (at < span.text.length) out.push({ text: span.text.slice(at), color: span.color });
  return out;
}

/**
 * The title a line stands for, if it is written as one: `# Title`, `== Title ==`,
 * `--- Title ---`, `**Title**`, or a short line in capitals.
 */
export function headingOf(line: string): string | undefined {
  const t = stripPobText(line).trim();
  if (!t || /^[-=*_~#.\s]+$/.test(t)) return undefined;
  const hash = /^#{1,4}\s+(.+?)\s*#*$/.exec(t);
  if (hash) return hash[1];
  const wrapped = /^([=\-*~#]{2,})\s*(.+?)\s*[=\-*~#]{2,}$/.exec(t);
  if (wrapped) return wrapped[2];
  const letters = t.replace(/[^\p{L}]/gu, "");
  if (t.length <= 48 && letters.length >= 3 && letters === letters.toUpperCase() && letters !== letters.toLowerCase()) {
    return t.replace(/[:\s]+$/, "");
  }
  return undefined;
}

export function noteLines(text: string): NoteLine[] {
  return text.split(/\r?\n/).map((line) => {
    const pieces = parsePobText(line).flatMap(linkify);
    const heading = headingOf(line);
    return heading ? { pieces, heading } : { pieces };
  });
}

/** Section titles with their line numbers; empty unless there are at least two to jump between. */
export function outline(lines: NoteLine[]): { line: number; title: string }[] {
  const out = lines.flatMap((l, line) => (l.heading ? [{ line, title: l.heading }] : []));
  return out.length >= 2 ? out : [];
}
