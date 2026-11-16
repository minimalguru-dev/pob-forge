import { expect, test } from "bun:test";
import { gemSchedule, poe1QuestPoints, poe1XpMultiplier, poeVaultUrl, resPenalty } from "../src/lib/overlay/leveling";

test("no penalty inside the safe range", () => {
  // Level 16: safe range is 3 + 1 = 4 levels either way.
  expect(poe1XpMultiplier(16, 20)).toBe(1);
  expect(poe1XpMultiplier(16, 12)).toBe(1);
  expect(poe1XpMultiplier(16, 21)).toBeLessThan(1);
});

test("over-levelled zones cost experience, never below 1%", () => {
  expect(poe1XpMultiplier(40, 20)).toBeLessThan(0.2);
  expect(poe1XpMultiplier(90, 1)).toBe(0.01);
});

test("resistance penalty by act", () => {
  expect(resPenalty("poe1", 5, false)).toBe(0);
  expect(resPenalty("poe1", 6, false)).toBe(-30);
  expect(resPenalty("poe1", 10, true)).toBe(-60);
  expect(resPenalty("poe2", 1, false)).toBe(0);
  expect(resPenalty("poe2", 3, false)).toBe(-20);
  expect(resPenalty("poe1", null, false)).toBeNull();
});

test("quest points follow PoB's act table", () => {
  expect(poe1QuestPoints(1, false)).toBe(0);
  expect(poe1QuestPoints(6, false)).toBe(9);
  expect(poe1QuestPoints(null, true)).toBe(23);
});

test("poe-vault pages exist for PoE1 acts 1 to 10 only", () => {
  expect(poeVaultUrl("poe1", 3)).toBe("https://www.poe-vault.com/guides/quick-reference-leveling-guide-act-3");
  expect(poeVaultUrl("poe1", 11)).toBeNull();
  expect(poeVaultUrl("poe2", 3)).toBeNull();
});

test("gem schedule splits unlocked, usable and upcoming", () => {
  const gems = [
    { name: "A", level: 1, support: false, known: true },
    { name: "B", level: 12, support: true, known: true },
    { name: "C", level: 18, support: false, known: true },
    { name: "D", level: null, support: false, known: false },
  ];
  const s = gemSchedule(gems, 12);
  expect(s.justUnlocked.map((g) => g.name)).toEqual(["B"]);
  expect(s.usable.map((g) => g.name)).toEqual(["A", "B"]);
  expect(s.upcoming.map((g) => g.name)).toEqual(["C"]);
});
