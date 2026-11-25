import { expect, test } from "bun:test";
import { headingOf, noteLines, outline } from "../src/lib/notes";

test("reads the usual guide section styles as headings", () => {
  expect(headingOf("# Leveling")).toBe("Leveling");
  expect(headingOf("## Gear ##")).toBe("Gear");
  expect(headingOf("=== Bandits ===")).toBe("Bandits");
  expect(headingOf("--- Pantheon ---")).toBe("Pantheon");
  expect(headingOf("**Flasks**")).toBe("Flasks");
  expect(headingOf("^xFFFF77ACT 1-3:")).toBe("ACT 1-3");
  expect(headingOf("ÉLITE MAPPING")).toBe("ÉLITE MAPPING");
});

test("leaves ordinary lines, separators and short codes alone", () => {
  expect(headingOf("Take Oak in act 2, kill the others.")).toBeUndefined();
  expect(headingOf("==========")).toBeUndefined();
  expect(headingOf("-----")).toBeUndefined();
  expect(headingOf("")).toBeUndefined();
  expect(headingOf("EE")).toBeUndefined();
  expect(headingOf("1-2-3")).toBeUndefined();
  expect(headingOf("USE THIS SETUP UNTIL YOU HAVE ENOUGH LIFE ON GEAR AND RESISTS CAPPED FOR MAPS")).toBeUndefined();
});

test("turns web addresses into links and keeps the sentence's punctuation out", () => {
  const [line] = noteLines("Guide: https://pobb.in/abc123. Video (https://youtu.be/x_y).");
  const links = line.pieces.filter((p) => p.href).map((p) => p.href);
  expect(links).toEqual(["https://pobb.in/abc123", "https://youtu.be/x_y"]);
  expect(line.pieces.map((p) => p.text).join("")).toBe("Guide: https://pobb.in/abc123. Video (https://youtu.be/x_y).");
});

test("links keep the colour of the text around them", () => {
  const [line] = noteLines("^xAF6025see https://poe.ninja/builds");
  const link = line.pieces.find((p) => p.href)!;
  expect(link.color).toBe("var(--c-unique)");
});

test("an outline needs at least two sections", () => {
  expect(outline(noteLines("# Only one\ntext"))).toEqual([]);
  expect(outline(noteLines("# Leveling\nact 1\n\n# Mapping\nT16"))).toEqual([
    { line: 0, title: "Leveling" },
    { line: 3, title: "Mapping" },
  ]);
});
