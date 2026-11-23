// Says whether upstream Path of Building has moved past the commits pinned in pob-sync.toml.
//
//   bun run pob:outdated              one line per game
//   bun run pob:outdated --markdown   a report for the GitHub issue (pob-outdated.yml)
//
// Exit 0: no newer release. Exit 1: a newer release exists for at least one game. Exit 2: the
// check itself failed (network, rate limit, a pin upstream does not have).
//
// Reads only public GitHub API endpoints and sends nothing but the pinned commit hashes. Set
// GH_TOKEN to lift the unauthenticated limit of 60 requests an hour. It never changes the pins:
// they are bumped on purpose, see CONTRIBUTING.md.

import { readFileSync } from "node:fs";

export const REPOS = {
  poe1: "PathOfBuildingCommunity/PathOfBuilding",
  poe2: "PathOfBuildingCommunity/PathOfBuilding-PoE2",
} as const;
export type Game = keyof typeof REPOS;

/** At most this many releases are compared with a pin, newest first. */
const MAX_RELEASES = 15;

export interface Release {
  tag: string;
  published: string;
}

export interface Comparison {
  /** GitHub's word for `head` relative to `base`: ahead, behind, identical or diverged. */
  status: string;
  /** Commits `head` has that `base` lacks. */
  ahead: number;
}

export interface Upstream {
  defaultBranch(repo: string): Promise<string>;
  /** Published, non-prerelease releases, newest first. */
  releases(repo: string): Promise<Release[]>;
  compare(repo: string, base: string, head: string): Promise<Comparison>;
}

export interface Report {
  game: Game;
  repo: string;
  pin: string;
  branch: string;
  /** Commits on the default branch the pin lacks. */
  branchBehind: number;
  newest?: Release;
  /** Releases with commits the pin lacks, newest first. */
  newer: Release[];
  /** Commits on the newest release the pin lacks. */
  releaseBehind: number;
  /** The pin is exactly this release. */
  exact?: string;
  /** Oldest release that contains the pin. */
  containedIn?: string;
  /** Newest release the pin already contains. */
  after?: string;
  /** No release older than the ones compared was reached. */
  searchCut: boolean;
}

export function parsePins(toml: string): Record<Game, string> {
  const parsed = Bun.TOML.parse(toml) as Record<string, { commit?: unknown }>;
  const pins = {} as Record<Game, string>;
  for (const game of Object.keys(REPOS) as Game[]) {
    const commit = parsed[game]?.commit;
    if (typeof commit !== "string" || !/^[0-9a-f]{40}$/.test(commit)) {
      throw new Error(`pob-sync.toml: [${game}] commit is not a full 40-character hash`);
    }
    pins[game] = commit;
  }
  return pins;
}

export async function checkGame(api: Upstream, game: Game, pin: string): Promise<Report> {
  const repo = REPOS[game];
  const [branch, releases] = await Promise.all([api.defaultBranch(repo), api.releases(repo)]);
  const head = await api.compare(repo, pin, branch);
  const report: Report = {
    game,
    repo,
    pin,
    branch,
    branchBehind: head.ahead,
    newest: releases[0],
    newer: [],
    releaseBehind: 0,
    searchCut: false,
  };
  const compared = releases.slice(0, MAX_RELEASES);
  for (const [i, release] of compared.entries()) {
    const c = await api.compare(repo, pin, release.tag);
    if (i === 0) report.releaseBehind = c.ahead;
    if (c.ahead === 0) {
      // Every release from here on is in the pin already.
      if (c.status === "identical") report.exact = release.tag;
      else report.after = release.tag;
      return report;
    }
    report.newer.push(release);
    // "diverged" (a release cut on another branch) has commits the pin lacks but does not contain it.
    if (c.status === "ahead") report.containedIn = release.tag;
  }
  report.searchCut = compared.length < releases.length;
  return report;
}

const short = (sha: string) => sha.slice(0, 8);
const day = (iso: string) => iso.slice(0, 10);

/** Where the pin sits among upstream releases. */
export function placement(r: Report): string {
  if (r.exact) return `= ${r.exact}`;
  if (r.containedIn) return `in ${r.containedIn}`;
  if (r.after && r.newer.length === 0) return `after ${r.after}, unreleased`;
  if (r.after) return `after ${r.after}`;
  if (!r.newest) return "no releases";
  return r.searchCut ? `older than the last ${r.newer.length} releases` : "before every release";
}

export function formatLine(r: Report): string {
  const newest = r.newest ? `${r.newest.tag} (${day(r.newest.published)})` : "none";
  const behind = r.newest
    ? `${r.releaseBehind} commits behind ${r.newest.tag}, ${r.branchBehind} behind ${r.branch}`
    : `${r.branchBehind} commits behind ${r.branch}`;
  const verdict = r.newer.length
    ? `UPDATE: ${r.newer.length} newer release${r.newer.length === 1 ? "" : "s"}`
    : "no newer release";
  return `${r.game}  pinned ${short(r.pin)} (${placement(r)})  newest ${newest}  ${behind}  ${verdict}`;
}

/** Hidden marker the workflow compares to decide whether the issue needs a new comment. */
export function marker(reports: Report[]): string {
  return `<!-- pob-outdated: ${reports.map((r) => `${r.game}=${r.newest?.tag ?? "none"}`).join(" ")} -->`;
}

export function formatMarkdown(reports: Report[], checked: string): string {
  const url = (r: Report) => `https://github.com/${r.repo}`;
  const lines = [
    marker(reports),
    reports.some((r) => r.newer.length)
      ? "Upstream Path of Building has a release the pins in `pob-sync.toml` do not include."
      : "The pins in `pob-sync.toml` include every upstream Path of Building release.",
    "",
    "| Game | Pinned | Newest release | Behind the release | Behind `dev` |",
    "|---|---|---|---|---|",
  ];
  for (const r of reports) {
    const pinned = `[\`${short(r.pin)}\`](${url(r)}/commit/${r.pin}) (${placement(r)})`;
    const newest = r.newest
      ? `[${r.newest.tag}](${url(r)}/releases/tag/${r.newest.tag}), ${day(r.newest.published)}`
      : "none";
    const rel = r.newest
      ? `[${r.releaseBehind} commits](${url(r)}/compare/${r.pin}...${r.newest.tag})`
      : "";
    const dev = `[${r.branchBehind} commits](${url(r)}/compare/${r.pin}...${r.branch})`;
    lines.push(`| ${r.game} | ${pinned} | ${newest} | ${rel} | ${dev} |`);
  }
  const newer = reports.filter((r) => r.newer.length);
  if (newer.length) {
    lines.push("", "Newer releases:");
    for (const r of newer) lines.push(`- ${r.game}: ${r.newer.map((n) => n.tag).join(", ")}`);
  }
  lines.push(
    "",
    'To move a pin, follow "Update the bundled PoB data" in CONTRIBUTING.md. Pins are not bumped automatically.',
    "",
    `Checked ${checked} by \`bun run pob:outdated\`. This issue is updated weekly and closed once no newer release is left.`,
  );
  return lines.join("\n");
}

class GitHubError extends Error {}

export function github(token = process.env.GH_TOKEN, fetchImpl: typeof fetch = fetch): Upstream {
  const headers: Record<string, string> = {
    Accept: "application/vnd.github+json",
    "X-GitHub-Api-Version": "2022-11-28",
    "User-Agent": "pob-outdated",
  };
  if (token) headers.Authorization = `Bearer ${token}`;
  const get = async (path: string) => {
    const res = await fetchImpl(`https://api.github.com/repos/${path}`, { headers });
    if (res.ok) return res.json();
    let why = `${res.status} ${res.statusText}`;
    if (res.headers.get("x-ratelimit-remaining") === "0") {
      const reset = Number(res.headers.get("x-ratelimit-reset")) * 1000;
      why += `: rate limit reached until ${new Date(reset).toISOString()}; set GH_TOKEN`;
    }
    throw new GitHubError(`GitHub ${path}: ${why}`);
  };
  return {
    async defaultBranch(repo) {
      return (await get(repo)).default_branch;
    },
    async releases(repo) {
      const list = (await get(`${repo}/releases?per_page=50`)) as {
        tag_name: string;
        published_at: string | null;
        draft: boolean;
        prerelease: boolean;
      }[];
      return list
        .filter((r) => !r.draft && !r.prerelease && r.published_at)
        .map((r) => ({ tag: r.tag_name, published: r.published_at! }))
        .sort((a, b) => b.published.localeCompare(a.published));
    },
    async compare(repo, base, head) {
      try {
        const c = await get(`${repo}/compare/${base}...${encodeURIComponent(head)}?per_page=1`);
        return { status: c.status, ahead: c.ahead_by };
      } catch (e) {
        if (e instanceof GitHubError && e.message.includes(": 404 ")) {
          throw new GitHubError(`${repo} has no commit ${base} (or no ${head}); is the pin from a fork?`);
        }
        throw e;
      }
    },
  };
}

async function main() {
  const markdown = process.argv.includes("--markdown");
  const pins = parsePins(readFileSync(new URL("../pob-sync.toml", import.meta.url), "utf8"));
  const api = github();
  let reports: Report[];
  try {
    reports = await Promise.all((Object.keys(REPOS) as Game[]).map((g) => checkGame(api, g, pins[g])));
  } catch (e) {
    console.error(e instanceof Error ? e.message : e);
    process.exit(2);
  }
  console.log(markdown ? formatMarkdown(reports, new Date().toISOString().slice(0, 10)) : reports.map(formatLine).join("\n"));
  process.exit(reports.some((r) => r.newer.length) ? 1 : 0);
}

if (import.meta.main) await main();
