#!/usr/bin/env bun
/**
 * Publish an app update (the Windows installer) on the CDN, which is where installed copies look.
 *
 *   bun run publish:update --release v0.5.9   # files from a published GitHub release (what CI runs)
 *   bun run publish:update                    # build and sign here instead
 *
 *   --dry-run      check everything, upload nothing
 *   --skip-build   local: publish the installer already in target/
 *   --notes FILE   local: release notes (default: commits since the previous tag)
 *   --allow-dirty  local: build with uncommitted changes
 *   --force        publish a version the CDN already offers
 *
 * Environment:
 *   POB_FORGE_CDN_URL                     the CDN (infra/cdn)
 *   TAURI_UPDATER_PUBKEY                  public key (local default: the key file's .pub)
 *   R2_ACCOUNT_ID, R2_ACCESS_KEY_ID, R2_SECRET_ACCESS_KEY, R2_BUCKET   upload, as for `bun run downloads upload`
 *   GH_TOKEN, GITHUB_REPOSITORY           --release: `gh` reads the release (default repo minimalguru-dev/pob-forge)
 *   TAURI_SIGNING_PRIVATE_KEY(_PASSWORD)  local: the updater key, its contents or path (default ~/.tauri/pob-forge.key)
 *
 * On the CDN: update/<version>/<installer>, download/PoE-Forge-setup.exe (and the old
 * PoB-Forge-setup.exe name), and update/latest.json, written last.
 */
import { execFileSync } from "node:child_process";
import { existsSync, mkdtempSync, readFileSync, statSync } from "node:fs";
import { homedir, tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { BUNDLES, compareVersions, fromRelease, manifest, publishedVersion, signatureProblem, upload, type Bundle } from "./updates";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const args = process.argv.slice(2);
const flag = (name: string) => args.includes(name);
const option = (name: string) => {
  const i = args.indexOf(name);
  return i >= 0 ? args[i + 1] : undefined;
};
const release = option("--release");
const dryRun = flag("--dry-run");

function die(msg: string): never {
  console.error(`\x1b[31m✗ ${msg}\x1b[0m`);
  process.exit(1);
}
const ok = (msg: string) => console.log(`\x1b[32m✓ ${msg}\x1b[0m`);
const run = (cmd: string, argv: string[]) => execFileSync(cmd, argv, { cwd: root, encoding: "utf8" }).trim();

const cdn = (process.env.POB_FORGE_CDN_URL ?? "").trim().replace(/\/+$/, "");
if (!/^https:\/\//.test(cdn)) die("set POB_FORGE_CDN_URL to the CDN's https address (infra/cdn)");
const conf = JSON.parse(readFileSync(join(root, "src-tauri/tauri.conf.json"), "utf8")) as { version: string; productName: string };

const keyPath = join(homedir(), ".tauri", "pob-forge.key");
let privateKey = process.env.TAURI_SIGNING_PRIVATE_KEY ?? "";
if (!privateKey && !release && existsSync(keyPath)) privateKey = keyPath;
const keyFile = privateKey && existsSync(privateKey) ? privateKey : null;
const pubFile = [keyFile && `${keyFile}.pub`, `${keyPath}.pub`].find((p): p is string => !!p && existsSync(p));
const pubkey = (process.env.TAURI_UPDATER_PUBKEY || (pubFile ? readFileSync(pubFile, "utf8") : "")).trim();
if (!pubkey) die("set TAURI_UPDATER_PUBKEY, or keep the .pub file next to the key");

let version: string;
let notes = "";
const bundles: Bundle[] = [];

if (release) {
  // ── From a published GitHub release ────────────────────────────────────────
  const repo = process.env.GITHUB_REPOSITORY || "minimalguru-dev/pob-forge";
  try {
    const r = fromRelease(release, repo, conf.version, (argv) => run("gh", argv), mkdtempSync(join(tmpdir(), "publish-update-")));
    version = r.version;
    notes = r.notes;
    bundles.push(...r.bundles);
  } catch (e) {
    die(e instanceof Error ? e.message : String(e));
  }
} else {
  // ── Built here ────────────────────────────────────────────────────────────
  version = conf.version;
  if (!privateKey) die(`no updater key: set TAURI_SIGNING_PRIVATE_KEY or create ${keyPath} (bun tauri signer generate -w ${keyPath})`);
  if (run("git", ["status", "--porcelain"]) && !flag("--allow-dirty")) {
    die("the working tree has changes; commit them so the update matches a commit (or pass --allow-dirty)");
  }
  const notesFile = option("--notes");
  if (notesFile) notes = readFileSync(notesFile, "utf8");
  else if (run("git", ["tag", "--list", `v${version}`])) notes = run("bun", ["scripts/release-notes.mjs", `v${version}`]);
  else console.warn(`no tag v${version} and no --notes: the update has no release notes`);

  const installer = join(root, "target/release/bundle/nsis", `${conf.productName}_${version}_x64-setup.exe`);
  if (!flag("--skip-build")) {
    const env: Record<string, string | undefined> = { ...process.env, VITE_UPDATER: "true", TAURI_SIGNING_PRIVATE_KEY: privateKey };
    // Its presence alone stops LuaJIT's build from running the minilua it just built (mistakes.md).
    delete env.NoDefaultCurrentDirectoryInExePath;
    execFileSync("bun", ["scripts/downloads.ts", "check"], { cwd: root, stdio: "inherit", env });
    const config = { plugins: { updater: { pubkey, endpoints: [`${cdn}/update/latest.json`] } } };
    execFileSync("bun", ["tauri", "build", "--bundles", "nsis", "--config", JSON.stringify(config)], { cwd: root, stdio: "inherit", env });
  }
  const sigPath = `${installer}.sig`;
  if (!existsSync(installer)) die(`${installer} is missing`);
  if (!existsSync(sigPath)) die(`${sigPath} is missing: was the build made with the updater key?`);
  if (statSync(sigPath).mtimeMs < statSync(installer).mtimeMs) die(`${sigPath} is older than the installer`);
  bundles.push({ name: `${conf.productName}_${version}_x64-setup.exe`, body: readFileSync(installer), signature: readFileSync(sigPath, "utf8").trim(), platforms: BUNDLES[0]!.platforms });
}

// ── Checks, then upload ─────────────────────────────────────────────────────
const published = await publishedVersion(cdn);
if (published && compareVersions(version, published) <= 0 && !flag("--force")) {
  die(`the CDN already offers ${published}; publish a higher version (bun run release:tag patch)`);
}
console.log(`\x1b[36mPublishing ${conf.productName} ${version} to ${cdn}${published ? `, replacing ${published}` : ""}\x1b[0m`);

for (const b of bundles) {
  const problem = signatureProblem(b.body, b.signature, pubkey);
  if (problem) die(`${b.name}: ${problem}`);
  ok(`${b.name}: signature verifies (${(b.body.length / 1048576).toFixed(1)} MB)`);
}

if (dryRun) {
  console.log(JSON.stringify(manifest(cdn, version, notes, bundles), null, 2));
  ok("dry run: nothing uploaded");
  process.exit(0);
}
try {
  await upload(cdn, version, notes, bundles, ok);
} catch (e) {
  die(e instanceof Error ? e.message : String(e));
}
