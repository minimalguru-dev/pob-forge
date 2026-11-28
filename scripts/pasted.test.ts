import { describe, expect, test } from "bun:test";
import { findCodes, referenceLinks, withHandles } from "../src/lib/ai/pasted";

const code = (tail: string) => `eNrtXW1z2ja${"A".repeat(300)}${tail}`;

describe("pasted PoB codes", () => {
  test("finds codes among other text, once each", () => {
    const a = code("x1");
    const b = code("y2_-");
    const text = `compare these:\n${a}\nand ${b}, also ${a}`;
    expect(findCodes(text)).toEqual([a, b]);
  });

  test("leaves links, short base64 and words alone", () => {
    expect(findCodes("https://pobb.in/eNabc pasted:1 eNough said")).toEqual([]);
    expect(findCodes(`https://example.com/${code("z")}`)).toEqual([]);
  });

  test("replaces each code with its handle", async () => {
    const a = code("a");
    const b = code("b");
    const seen: string[] = [];
    const out = await withHandles(`one ${a} two ${b} again ${a}`, async (c) => {
      seen.push(c);
      return seen.length;
    });
    expect(out).toBe("one [PoB code pasted:1] two [PoB code pasted:2] again [PoB code pasted:1]");
  });

  test("keeps a code the store refuses or fails on", async () => {
    const a = code("a");
    expect(await withHandles(`x ${a}`, async () => null)).toBe(`x ${a}`);
    expect(await withHandles(`x ${a}`, () => Promise.reject(new Error("no")))).toBe(`x ${a}`);
  });

  test("only links make a lookup", () => {
    expect(referenceLinks({ builds: ["pasted:1", "https://pobb.in/abc", " http://pastebin.com/x"] })).toEqual([
      "https://pobb.in/abc",
      "http://pastebin.com/x",
    ]);
    expect(referenceLinks({ builds: "pasted:1, pasted:2" })).toEqual([]);
    expect(referenceLinks(null)).toEqual([]);
  });
});
