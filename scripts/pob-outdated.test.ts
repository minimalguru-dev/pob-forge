import { expect, test } from "bun:test";
import {
  checkGame,
  formatLine,
  formatMarkdown,
  github,
  parsePins,
  placement,
  type Comparison,
  type Upstream,
} from "./pob-outdated";

const PIN = "f5ca0aeeeeba62d7d2c93b1fbd0704c74d54e7ea";

// Releases newest first; `vs` is how each ref compares with the pin.
function fake(vs: Record<string, Comparison>, tags = Object.keys(vs).filter((t) => t !== "dev")): Upstream & { calls: string[] } {
  const calls: string[] = [];
  return {
    calls,
    defaultBranch: async () => "dev",
    releases: async () => tags.map((tag, i) => ({ tag, published: `2026-11-${String(20 - i).padStart(2, "0")}T00:00:00Z` })),
    compare: async (_repo, base, head) => {
      expect(base).toBe(PIN);
      calls.push(head);
      return vs[head];
    },
  };
}

test("reads both pins from pob-sync.toml and rejects a short hash", () => {
  const toml = `[poe2]\nsource_path = "x"\ncommit = "${"a".repeat(40)}"\n\n[poe1]\ncommit = "${PIN}"\n`;
  expect(parsePins(toml)).toEqual({ poe1: PIN, poe2: "a".repeat(40) });
  expect(() => parsePins(`[poe2]\ncommit = "abc"\n[poe1]\ncommit = "${PIN}"`)).toThrow("[poe2]");
});

test("a pin inside the second-newest release: two newer releases, contained in the older one", async () => {
  const api = fake({
    dev: { status: "ahead", ahead: 54 },
    "v2.68.1": { status: "ahead", ahead: 26 },
    "v2.68.0": { status: "ahead", ahead: 12 },
    "v2.67.2": { status: "behind", ahead: 0 },
    "v2.67.1": { status: "behind", ahead: 0 },
  });
  const r = await checkGame(api, "poe1", PIN);
  expect(r.newer.map((n) => n.tag)).toEqual(["v2.68.1", "v2.68.0"]);
  expect(r.containedIn).toBe("v2.68.0");
  expect(r.after).toBe("v2.67.2");
  expect(r.releaseBehind).toBe(26);
  expect(r.branchBehind).toBe(54);
  // Stops at the first release the pin already has.
  expect(api.calls).toEqual(["dev", "v2.68.1", "v2.68.0", "v2.67.2"]);
  expect(formatLine(r)).toBe(
    "poe1  pinned f5ca0aee (in v2.68.0)  newest v2.68.1 (2026-11-20)  26 commits behind v2.68.1, 54 behind dev  UPDATE: 2 newer releases",
  );
});

test("a pin past the newest release is not outdated", async () => {
  const r = await checkGame(fake({ dev: { status: "ahead", ahead: 2 }, "v0.23.1": { status: "behind", ahead: 0 } }), "poe2", PIN);
  expect(r.newer).toEqual([]);
  expect(placement(r)).toBe("after v0.23.1, unreleased");
  expect(formatLine(r)).toEndWith("0 commits behind v0.23.1, 2 behind dev  no newer release");
});

test("a pin that is exactly a release", async () => {
  const r = await checkGame(fake({ dev: { status: "identical", ahead: 0 }, "v1.0.0": { status: "identical", ahead: 0 } }), "poe2", PIN);
  expect(placement(r)).toBe("= v1.0.0");
  expect(r.newer).toEqual([]);
});

test("a release cut on another branch counts as newer but does not contain the pin", async () => {
  const r = await checkGame(
    fake({ dev: { status: "ahead", ahead: 9 }, "v2.0.1": { status: "diverged", ahead: 1 }, "v2.0.0": { status: "behind", ahead: 0 } }),
    "poe1",
    PIN,
  );
  expect(r.newer.map((n) => n.tag)).toEqual(["v2.0.1"]);
  expect(r.containedIn).toBeUndefined();
  expect(placement(r)).toBe("after v2.0.0");
});

test("a very old pin stops after the newest 15 releases", async () => {
  const vs: Record<string, Comparison> = { dev: { status: "ahead", ahead: 999 } };
  for (let i = 20; i > 0; i--) vs[`v${i}`] = { status: "ahead", ahead: i * 10 };
  const api = fake(vs);
  const r = await checkGame(api, "poe1", PIN);
  expect(api.calls.length).toBe(16);
  expect(r.newer.length).toBe(15);
  expect(r.searchCut).toBe(true);
  expect(placement(r)).toBe("in v6");
});

test("the markdown report carries a marker of the newest releases and compare links", async () => {
  const poe1 = await checkGame(fake({ dev: { status: "ahead", ahead: 5 }, "v2.68.1": { status: "ahead", ahead: 3 }, "v2.68.0": { status: "behind", ahead: 0 } }), "poe1", PIN);
  const poe2 = await checkGame(fake({ dev: { status: "ahead", ahead: 2 }, "v0.23.1": { status: "behind", ahead: 0 } }), "poe2", PIN);
  const md = formatMarkdown([poe1, poe2], "2026-11-23");
  expect(md.split("\n")[0]).toBe("<!-- pob-outdated: poe1=v2.68.1 poe2=v0.23.1 -->");
  expect(md).toContain(`https://github.com/PathOfBuildingCommunity/PathOfBuilding/compare/${PIN}...v2.68.1`);
  expect(md).toContain("- poe1: v2.68.1");
  expect(md).not.toContain("- poe2:");
});

test("the GitHub client sends the token only when given, and names the rate limit", async () => {
  const seen: Headers[] = [];
  const limited = (async (_url: string, init: RequestInit) => {
    seen.push(new Headers(init.headers));
    return new Response("{}", { status: 403, statusText: "Forbidden", headers: { "x-ratelimit-remaining": "0", "x-ratelimit-reset": "1800000000" } });
  }) as unknown as typeof fetch;
  await expect(github(undefined, limited).defaultBranch("o/r")).rejects.toThrow("rate limit reached until 2027-01-15T08:00:00.000Z; set GH_TOKEN");
  expect(seen[0].has("authorization")).toBe(false);
  await expect(github("t0k", limited).defaultBranch("o/r")).rejects.toThrow();
  expect(seen[1].get("authorization")).toBe("Bearer t0k");
});

test("the GitHub client drops drafts and prereleases and sorts newest first", async () => {
  const body = [
    { tag_name: "v1", published_at: "2026-01-01T00:00:00Z", draft: false, prerelease: false },
    { tag_name: "v3-beta", published_at: "2026-03-01T00:00:00Z", draft: false, prerelease: true },
    { tag_name: "v2", published_at: "2026-02-01T00:00:00Z", draft: false, prerelease: false },
    { tag_name: "draft", published_at: null, draft: true, prerelease: false },
  ];
  const ok = (async () => Response.json(body)) as unknown as typeof fetch;
  expect((await github(undefined, ok).releases("o/r")).map((r) => r.tag)).toEqual(["v2", "v1"]);
});
