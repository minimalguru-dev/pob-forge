import { expect, test } from "bun:test";
import { EmptyClipboardError, readImportClipboard } from "../src/lib/import/clipboard";

test("empty or non-text clipboard gives an actionable failure", async () => {
  for (const text of [null, "", " \n"]) {
    await expect(readImportClipboard(async () => text)).rejects.toBeInstanceOf(EmptyClipboardError);
  }
});

test("a transient clipboard lock is retried and the original import is preserved", async () => {
  let reads = 0;
  const text = await readImportClipboard(async () => {
    if (++reads < 3) throw new Error("Clipboard locked");
    return "  <PathOfBuilding><Build/></PathOfBuilding>  ";
  }, async () => {});
  expect(reads).toBe(3);
  expect(text).toBe("<PathOfBuilding><Build/></PathOfBuilding>");
});

test("persistent clipboard denial stops after three attempts and preserves the reason", async () => {
  let reads = 0;
  await expect(readImportClipboard(async () => {
    reads++;
    throw new Error("Permission denied");
  }, async () => {})).rejects.toThrow("Permission denied");
  expect(reads).toBe(3);
});
