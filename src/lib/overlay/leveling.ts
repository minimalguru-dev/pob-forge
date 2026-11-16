import type { Game } from "$lib/state/game.svelte";

/** Mirrors `LogState` in src-tauri/src/gamelog.rs. */
export interface LogState {
  path: string | null;
  zone: string | null;
  areaId: string | null;
  areaLevel: number | null;
  act: number | null;
  endgame: boolean;
  character: string | null;
  className: string | null;
  level: number | null;
}

/**
 * PoE1's experience multiplier for a character level in an area level (poewiki "Experience").
 * Monster levels from 70 count as lower effective levels; levels 95+ carry an extra penalty.
 */
export function poe1XpMultiplier(charLevel: number, areaLevel: number): number {
  const effective = areaLevel >= 70 ? -0.03 * areaLevel ** 2 + 5.17 * areaLevel - 144.9 : areaLevel;
  const safe = 3 + Math.floor(charLevel / 16);
  const diff = Math.max(Math.abs(charLevel - effective) - safe, 0);
  let mult = ((charLevel + 5) / (charLevel + 5 + diff ** 2.5)) ** 1.5;
  const high = [1, 1, 1, 1, 1, 1.065, 1.115, 1.187, 1.239, 1.288][Math.max(0, charLevel - 90)] ?? 1;
  if (charLevel >= 95) mult *= 1 / high;
  return Math.max(mult, 0.01);
}

/** The elemental resistance penalty PoB's Config tab uses at this point of the game. */
export function resPenalty(game: Game, act: number | null, endgame: boolean): number | null {
  if (endgame) return -60;
  if (act === null) return null;
  if (game === "poe1") return act <= 5 ? 0 : act <= 10 ? -30 : -60;
  return act <= 6 ? (1 - act) * 10 : -60;
}

/** PoE1 quest passive points available on arriving in an act (PoB Build.lua `acts`), before the bandit choice. */
const POE1_QUEST_POINTS = [0, 0, 2, 4, 6, 7, 9, 12, 15, 18, 20, 23];
export function poe1QuestPoints(act: number | null, endgame: boolean): number | null {
  if (endgame) return POE1_QUEST_POINTS[11];
  return act === null ? null : POE1_QUEST_POINTS[Math.min(Math.max(act, 1), 11)];
}

/** poe-vault's quick-reference leveling guide has one page per PoE1 act. */
export function poeVaultUrl(game: Game, act: number | null): string | null {
  if (game !== "poe1" || act === null || act < 1 || act > 10) return null;
  return `https://www.poe-vault.com/guides/quick-reference-leveling-guide-act-${act}`;
}

export interface LevelGem {
  name: string;
  level: number | null;
  support: boolean;
  known: boolean;
}

/** Gems the character can use now that it could not at the previous level, and the next few to come. */
export function gemSchedule(gems: LevelGem[], level: number, ahead = 3) {
  const usable = gems.filter((g) => g.level !== null && g.level <= level);
  return {
    justUnlocked: usable.filter((g) => g.level === level),
    usable,
    upcoming: gems.filter((g) => g.level !== null && g.level > level).slice(0, ahead),
  };
}
