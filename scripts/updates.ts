// Shared by scripts/publish-update.ts (local builds and GitHub releases): checking an update's
// signature the way the app will, and putting the update on the CDN.
import { S3Client } from "bun";
import { createHash, createPublicKey, verify } from "node:crypto";
import { readFileSync } from "node:fs";
import { join } from "node:path";

/**
 * Tauri's .sig and public key are base64 of minisign's text files. Their second
 * line is base64 of: 2 bytes algorithm, 8 bytes key id, then the signature
 * (64 bytes) or the Ed25519 key (32 bytes). "ED" signs the BLAKE2b-512 hash of
 * the file, "Ed" the file itself.
 */
function minisign(b64: string) {
  const lines = Buffer.from(b64.trim(), "base64").toString("utf8").split(/\r?\n/);
  return Buffer.from(lines[1] ?? "", "base64");
}

/** Why `signature` does not hold for `body` under `pubkey`, or null when it does. */
export function signatureProblem(body: Uint8Array, signature: string, pubkey: string): string | null {
  const sig = minisign(signature);
  const pub = minisign(pubkey);
  if (sig.length !== 74 || pub.length !== 42) return "the signature or public key is not in minisign format";
  if (!sig.subarray(2, 10).equals(pub.subarray(2, 10))) return "it was signed with a different key than the public key";
  const message = sig.subarray(0, 2).toString("latin1") === "ED" ? createHash("blake2b512").update(body).digest() : Buffer.from(body);
  const key = createPublicKey({ key: Buffer.concat([Buffer.from("302a300506032b6570032100", "hex"), pub.subarray(10)]), format: "der", type: "spki" });
  return verify(null, message, key, sig.subarray(10)) ? null : "the signature does not verify against the public key";
}

/** -1, 0 or 1. A pre-release sorts before its release, as in semver. */
export function compareVersions(a: string, b: string) {
  const parse = (v: string) => {
    const [core, pre] = v.replace(/^v/, "").split("-", 2);
    return { nums: core!.split(".").map(Number), pre: pre ?? null };
  };
  const x = parse(a);
  const y = parse(b);
  for (let i = 0; i < 3; i++) if ((x.nums[i] ?? 0) !== (y.nums[i] ?? 0)) return (x.nums[i] ?? 0) < (y.nums[i] ?? 0) ? -1 : 1;
  if (x.pre === y.pre) return 0;
  if (x.pre === null) return 1;
  if (y.pre === null) return -1;
  return x.pre < y.pre ? -1 : 1;
}

/** An update file on its way to the CDN: the bytes, its updater signature, and the platform keys it serves. */
export type Bundle = { name: string; body: Uint8Array; signature: string; platforms: string[] };

/** Asset suffix -> updater platform keys, as `scripts/updater-manifest.mjs` writes them. */
export const BUNDLES = [
  { suffix: "_x64-setup.exe", platforms: ["windows-x86_64", "windows-x86_64-nsis"] },
  { suffix: "_aarch64.app.tar.gz", platforms: ["darwin-aarch64", "darwin-aarch64-app"] },
];

/** Keys and URLs on the CDN. Names have no spaces (GitHub's dotted asset names), so URLs need no escaping. */
export function cdnKey(version: string, name: string) {
  return `update/${version}/${name.replace(/ /g, ".")}`;
}

export function manifest(cdn: string, version: string, notes: string, bundles: Bundle[], now = new Date()) {
  const platforms: Record<string, { signature: string; url: string }> = {};
  for (const b of bundles) for (const p of b.platforms) platforms[p] = { signature: b.signature, url: `${cdn}/${cdnKey(version, b.name)}` };
  return { version, notes: notes.replace(/\r\n/g, "\n").trim(), pub_date: now.toISOString().replace(/\.\d{3}Z$/, "Z"), platforms };
}

/**
 * The bundles of a published GitHub release, downloaded into `dir` by `gh` (which runs the gh CLI
 * with these arguments and returns its output). Throws when the release is a draft, is not this
 * checkout's version, or has no signed Windows installer; other platforms are optional.
 */
export function fromRelease(tag: string, repo: string, expected: string, gh: (argv: string[]) => string, dir: string, warn: (msg: string) => void) {
  let view: string;
  try {
    view = gh(["release", "view", tag, "-R", repo, "--json", "assets,body,isDraft,tagName"]);
  } catch {
    throw new Error(`no release ${tag} on ${repo} (or gh is not signed in)`);
  }
  const info = JSON.parse(view) as { assets: { name: string }[]; body: string | null; isDraft: boolean; tagName: string };
  if (info.isDraft) throw new Error(`${tag} is still a draft; publish it on GitHub first`);
  const version = info.tagName.replace(/^v/, "");
  if (version !== expected) throw new Error(`${tag} is version ${version}, but this checkout's tauri.conf.json says ${expected}; check out the tag`);
  const bundles: Bundle[] = [];
  for (const { suffix, platforms } of BUNDLES) {
    const asset = info.assets.find((a) => a.name.endsWith(suffix));
    if (!asset || !info.assets.some((a) => a.name === `${asset.name}.sig`)) {
      if (suffix.endsWith(".exe")) throw new Error(`${tag} has no signed Windows installer (*${suffix} and its .sig)`);
      warn(`${tag} has no signed *${suffix}; that platform is left out`);
      continue;
    }
    gh(["release", "download", tag, "-R", repo, "-p", asset.name, "-p", `${asset.name}.sig`, "-D", dir, "--clobber"]);
    bundles.push({
      name: asset.name,
      body: readFileSync(join(dir, asset.name)),
      signature: readFileSync(join(dir, `${asset.name}.sig`), "utf8").trim(),
      platforms,
    });
  }
  return { version, notes: info.body ?? "", bundles };
}

export async function publishedVersion(cdn: string) {
  try {
    const r = await fetch(`${cdn}/update/latest.json`, { headers: { "Cache-Control": "no-cache" } });
    return r.ok ? (((await r.json()) as { version?: string }).version ?? null) : null;
  } catch {
    return null;
  }
}

/** The bundles first, then the download links, then the manifest, so no client sees a manifest without its files. */
export async function upload(cdn: string, version: string, notes: string, bundles: Bundle[], log: (msg: string) => void) {
  const need = (n: string) => process.env[n] || (() => { throw new Error(`${n} is not set`); })();
  const s3 = new S3Client({
    accessKeyId: need("R2_ACCESS_KEY_ID"),
    secretAccessKey: need("R2_SECRET_ACCESS_KEY"),
    bucket: need("R2_BUCKET"),
    endpoint: `https://${need("R2_ACCOUNT_ID")}.r2.cloudflarestorage.com`,
  });
  const type = (name: string) => (name.endsWith(".exe") ? "application/vnd.microsoft.portable-executable" : "application/gzip");
  for (const b of bundles) {
    const key = cdnKey(version, b.name);
    await s3.write(key, b.body, { type: type(b.name) });
    const head = await fetch(`${cdn}/${key}`, { method: "HEAD" });
    const size = Number(head.headers.get("Content-Length"));
    if (!head.ok || size !== b.body.length) throw new Error(`${cdn}/${key} answers ${head.status} with ${size} bytes, expected ${b.body.length}; latest.json not changed`);
    log(`uploaded ${key}`);
  }
  const windows = bundles.find((b) => b.name.endsWith(".exe"));
  if (windows) {
    for (const link of ["download/PoE-Forge-setup.exe", "download/PoB-Forge-setup.exe"]) await s3.write(link, windows.body, { type: type(windows.name) });
    log("updated the download links");
  }
  await s3.write("update/latest.json", JSON.stringify(manifest(cdn, version, notes, bundles), null, 2), { type: "application/json" });
  const live = await publishedVersion(cdn);
  if (live !== version) throw new Error(`${cdn}/update/latest.json says ${live}, expected ${version}`);
  log(`${cdn}/update/latest.json now offers ${version}`);
}
