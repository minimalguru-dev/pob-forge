import { expect, test } from "bun:test";
import { buildHighlight } from "../src/lib/regex/highlight";

const others = ["Chaos Orb", "Orb of Alteration", "Orb of Alchemy", "Orb of Fusing", "Jeweller's Orb", "Chromatic Orb", "Exalted Shard"];

test("highlights only the wanted names, by pieces found nowhere else", () => {
  const wanted = [
    { name: "Divine Orb", value: 200 },
    { name: "Exalted Orb", value: 15 },
    { name: "Mirror of Kalandra", value: 90000 },
  ];
  const h = buildHighlight(wanted, [...others, ...wanted.map((w) => w.name)], { limit: 50, minPiece: 3 });
  expect(h.covered.map((c) => c.name)).toEqual(["Mirror of Kalandra", "Divine Orb", "Exalted Orb"]);
  const re = new RegExp(h.regex.slice(1, -1), "i");
  for (const w of wanted) expect(re.test(w.name)).toBe(true);
  for (const o of others) expect(re.test(o)).toBe(false);
  expect(h.regex.length).toBeLessThanOrEqual(50);
});

test("keeps to the limit, most valuable first, and says what was left out", () => {
  const wanted = [
    { name: "Mirror of Kalandra", value: 90000 },
    { name: "Divine Orb", value: 200 },
    { name: "Exalted Orb", value: 15 },
  ];
  const h = buildHighlight(wanted, others, { limit: 7, minPiece: 3 });
  expect(h.regex.length).toBeLessThanOrEqual(7);
  expect(h.covered.map((c) => c.name)).toEqual(["Mirror of Kalandra"]);
  expect(h.overLimit.map((c) => c.name)).toEqual(["Divine Orb", "Exalted Orb"]);
});

test("reports names that cannot be told apart", () => {
  const h = buildHighlight([{ name: "Orb", value: 5 }], ["Chaos Orb"], { limit: 50, minPiece: 3 });
  expect(h.regex).toBe("");
  expect(h.noPiece.map((c) => c.name)).toEqual(["Orb"]);
});
