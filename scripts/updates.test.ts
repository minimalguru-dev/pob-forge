import { describe, expect, test } from "bun:test";
import { createHash, generateKeyPairSync, sign } from "node:crypto";
import { mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { BUNDLES, cdnKey, cdnVersion, compareVersions, fromRelease, manifest, signatureProblem } from "./updates";

// Minisign files the way `tauri signer` writes them, from a throwaway key.
function keypair(id: number) {
  const { publicKey, privateKey } = generateKeyPairSync("ed25519");
  const raw = Buffer.from(publicKey.export({ format: "jwk" }).x!, "base64url");
  const keyId = Buffer.alloc(8);
  keyId.writeUInt32LE(id);
  const pubText = `untrusted comment: minisign public key: ${id}\n${Buffer.concat([Buffer.from("Ed"), keyId, raw]).toString("base64")}\n`;
  const signFile = (body: Uint8Array) => {
    const s = sign(null, createHash("blake2b512").update(body).digest(), privateKey);
    const text = `untrusted comment: signature from tauri secret key\n${Buffer.concat([Buffer.from("ED"), keyId, s]).toString("base64")}\ntrusted comment: test\nAAAA\n`;
    return Buffer.from(text).toString("base64");
  };
  return { pubkey: Buffer.from(pubText).toString("base64"), signFile };
}

describe("signatureProblem", () => {
  const ours = keypair(1);
  const body = new TextEncoder().encode("installer bytes");

  test("accepts a file signed with the key", () => {
    expect(signatureProblem(body, ours.signFile(body), ours.pubkey)).toBeNull();
  });
  test("rejects a changed file", () => {
    const changed = new TextEncoder().encode("installer bytes!");
    expect(signatureProblem(changed, ours.signFile(body), ours.pubkey)).toContain("does not verify");
  });
  test("rejects another key", () => {
    const theirs = keypair(2);
    expect(signatureProblem(body, theirs.signFile(body), ours.pubkey)).toContain("different key");
  });
  test("rejects a key with the same id but other bytes", () => {
    const impostor = keypair(1);
    expect(signatureProblem(body, impostor.signFile(body), ours.pubkey)).toContain("does not verify");
  });
  test("rejects text that is not minisign", () => {
    expect(signatureProblem(body, "bm90IGEgc2lnbmF0dXJl", ours.pubkey)).toContain("minisign format");
  });
});

describe("fromRelease", () => {
  // A stand-in for the gh CLI: `view` answers the release, `download` writes the named assets into -D.
  function fakeGh(release: { tagName: string; isDraft: boolean; body: string | null; assets: string[] }) {
    const downloads: string[][] = [];
    const gh = (argv: string[]) => {
      if (argv[1] === "view") return JSON.stringify({ ...release, assets: release.assets.map((name) => ({ name })) });
      downloads.push(argv);
      const dir = argv[argv.indexOf("-D") + 1]!;
      argv.forEach((x, i) => {
        if (x === "-p") writeFileSync(join(dir, argv[i + 1]!), argv[i + 1]!.endsWith(".sig") ? "c2ln\n" : `bytes of ${argv[i + 1]}`);
      });
      return "";
    };
    return { gh, downloads };
  }
  const win = "PoE.Forge_0.5.9_x64-setup.exe";
  const all = [win, `${win}.sig`, "latest.json"];

  test("downloads the signed installer and keeps the release notes", () => {
    const { gh, downloads } = fakeGh({ tagName: "v0.5.9", isDraft: false, body: "notes", assets: all });
    const r = fromRelease("v0.5.9", "o/r", gh, mkdtempSync(join(tmpdir(), "updates-test-")));
    expect(r.version).toBe("0.5.9");
    expect(r.notes).toBe("notes");
    expect(r.bundles.map((b) => [b.name, b.signature, b.platforms])).toEqual([[win, "c2ln", ["windows-x86_64", "windows-x86_64-nsis"]]]);
    expect(new TextDecoder().decode(r.bundles[0]!.body)).toBe(`bytes of ${win}`);
    expect(downloads).toHaveLength(1);
  });
  test("an empty release body gives empty notes", () => {
    const { gh } = fakeGh({ tagName: "v0.5.9", isDraft: false, body: null, assets: all });
    expect(fromRelease("v0.5.9", "o/r", gh, mkdtempSync(join(tmpdir(), "updates-test-"))).notes).toBe("");
  });
  test("refuses drafts, a release without a signed installer, and a missing release", () => {
    const dir = mkdtempSync(join(tmpdir(), "updates-test-"));
    const draft = fakeGh({ tagName: "v0.5.9", isDraft: true, body: "", assets: all }).gh;
    expect(() => fromRelease("v0.5.9", "o/r", draft, dir)).toThrow("draft");
    const unsigned = fakeGh({ tagName: "v0.5.9", isDraft: false, body: "", assets: [win] }).gh;
    expect(() => fromRelease("v0.5.9", "o/r", unsigned, dir)).toThrow("no signed Windows installer");
    const missing = () => {
      throw new Error("release not found");
    };
    expect(() => fromRelease("v0.5.9", "o/r", missing, dir)).toThrow("no release v0.5.9 on o/r");
  });
});

describe("cdnVersion", () => {
  const cdn = "https://cdn.example";
  test("counts only a manifest whose installer is on the CDN", () => {
    const ours = manifest(cdn, "0.5.10", "", [{ name: "PoE Forge_0.5.10_x64-setup.exe", body: new Uint8Array(), signature: "s", platforms: BUNDLES[0]!.platforms }]);
    expect(cdnVersion(ours, cdn)).toBe("0.5.10");
    // The Worker's fallback: the latest GitHub release's own latest.json, with GitHub download links.
    const github = { version: "0.5.10", platforms: { "windows-x86_64": { signature: "s", url: "https://api.github.com/repos/o/r/releases/assets/1" } } };
    expect(cdnVersion(github, cdn)).toBeNull();
    // A host that merely starts with the CDN's name is not the CDN.
    expect(cdnVersion({ version: "1.0.0", platforms: { "windows-x86_64": { url: "https://cdn.example.evil/x.exe" } } }, cdn)).toBeNull();
    expect(cdnVersion(null, cdn)).toBeNull();
    expect(cdnVersion({ version: 3 }, cdn)).toBeNull();
  });
});

describe("compareVersions", () => {
  test("orders releases and pre-releases", () => {
    expect(compareVersions("0.5.9", "0.5.8")).toBe(1);
    expect(compareVersions("0.5.8", "0.5.8")).toBe(0);
    expect(compareVersions("v0.5.10", "0.5.9")).toBe(1);
    expect(compareVersions("0.6.0-alpha.1", "0.6.0")).toBe(-1);
    expect(compareVersions("0.6.0", "0.6.0-alpha.1")).toBe(1);
    expect(compareVersions("1.0.0", "0.99.99")).toBe(1);
  });
});

describe("manifest", () => {
  test("points both Windows keys at the installer on the CDN", () => {
    const bundles = [{ name: "PoE Forge_0.5.9_x64-setup.exe", body: new Uint8Array(), signature: "win", platforms: BUNDLES[0]!.platforms }];
    const m = manifest("https://cdn.example", "0.5.9", "## Fixes\r\n\r\n- one\r\n", bundles, new Date("2026-11-23T12:00:00.123Z"));
    expect(m.version).toBe("0.5.9");
    expect(m.notes).toBe("## Fixes\n\n- one");
    expect(m.pub_date).toBe("2026-11-23T12:00:00Z");
    expect(m.platforms["windows-x86_64"]).toEqual({ signature: "win", url: "https://cdn.example/update/0.5.9/PoE.Forge_0.5.9_x64-setup.exe" });
    expect(m.platforms["windows-x86_64-nsis"]).toEqual(m.platforms["windows-x86_64"]!);
    expect(Object.keys(m.platforms)).toEqual(["windows-x86_64", "windows-x86_64-nsis"]);
  });
  test("keys have no spaces", () => {
    expect(cdnKey("0.5.9", "PoE Forge_0.5.9_x64-setup.exe")).toBe("update/0.5.9/PoE.Forge_0.5.9_x64-setup.exe");
  });
});
