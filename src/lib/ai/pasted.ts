/**
 * PoB codes pasted into the chat. A code is 5–30 KB of base64; the model would
 * pay for it on every turn and then have to copy it, character for character,
 * into a tool call. The app keeps each code (Rust, in memory) and the model
 * sees `[PoB code pasted:N]`, which analyze_reference_builds takes as `pasted:N`.
 */

/** zlib-deflated XML in base64 starts with eN (78 DA) or eJ (78 9C). */
const CODE = /(?<![A-Za-z0-9_+/=-])e[NJ][A-Za-z0-9_+/=-]{200,}/g;

export function findCodes(text: string): string[] {
  return [...new Set(text.match(CODE) ?? [])];
}

/** `text` with each pasted code replaced by its handle. A code the store refuses stays as it is. */
export async function withHandles(text: string, stash: (code: string) => Promise<number | null>): Promise<string> {
  let out = text;
  for (const code of findCodes(text)) {
    const n = await stash(code).catch(() => null);
    if (n) out = out.split(code).join(`[PoB code pasted:${n}]`);
  }
  return out;
}

/** Whether analyze_reference_builds will fetch anything; pasted codes are read locally. */
export function referenceLinks(args: unknown): string[] {
  const builds = (args as { builds?: unknown } | null)?.builds;
  const list = Array.isArray(builds) ? builds : typeof builds === "string" ? builds.split(",") : [];
  return list.map((b) => String(b).trim()).filter((b) => /^https?:\/\//.test(b));
}
