import { invoke } from "@tauri-apps/api/core";

export type Stats = Record<string, number | boolean | null>;
export interface Snapshot {
  id: string;
  at: string;
  xml: string;
  label: string;
  game: "poe1";
  pobVersion: string;
  treeVersion: string;
  level: number;
  className: string;
  ascendancy: string;
  mainSkill: string | null;
  nodes: { id: number; name: string; mastery: number | null }[];
  equipment: { slot: string; name: string; raw: string; variant: number | null; variant2: number | null }[];
  gems: { group: number; name: string; level: number; quality: number; enabled: boolean }[];
  gemErrors: string[];
  config: Record<string, string | number | boolean>;
  stats: Stats;
  loadout: { tree: string; items: string; skills: string };
  boss: { kind: Boss; stats: Stats; overrides: Record<string, string | number | boolean> };
}
export type Boss = "Boss" | "Pinnacle" | "Uber";
export interface Plan {
  schema: 1;
  snapshots: Snapshot[];
  currentId: string | null;
  targetId: string | null;
}
export const emptyPlan = (): Plan => ({ schema: 1, snapshots: [], currentId: null, targetId: null });
export const MAX_SNAPSHOTS = 20;

export interface Check {
  id: string;
  status: "blocked" | "review" | "pass";
  title: string;
  detail: string;
}
type Equipment = Snapshot["equipment"][number];
type Node = Snapshot["nodes"][number];
export interface Diff {
  equipment: { slot: string; current: Equipment | null; target: Equipment | null }[];
  compatibleTree: boolean;
  gained: Node[];
  lost: Node[];
  masteries: Node[];
  config: string[];
}
export interface Report {
  diff: Diff;
  checks: Check[];
  priorities: Check[];
}

export function numberStat(stats: Stats, key: string): number | null {
  const value = stats[key];
  return typeof value === "number" && Number.isFinite(value) ? value : null;
}

/** Where the view kept history before it moved to the app's data folder; read once to migrate. */
export const LEGACY_STORAGE_KEY = "pob-forge:progression:poe1:v1";
export const CHANGED_EVENT = "progression:changed";

/** History is stored by the app, so the Progression tab, the overlay and the assistant see the same plan. */
export const progression = {
  load: () => invoke<Plan | null>("progression_load"),
  migrate: (text: string) => invoke<Plan>("progression_migrate", { text }),
  decode: (text: string) => invoke<Plan>("progression_decode", { text }),
  add: (captured: Omit<Snapshot, "id" | "at">, role: "current" | "target") => invoke<Plan>("progression_add", { captured, role }),
  remove: (id: string) => invoke<Plan>("progression_remove", { id }),
  select: (currentId: string | null, targetId: string | null) => invoke<Plan>("progression_select", { currentId, targetId }),
  importBackup: (text: string) => invoke<Plan>("progression_import", { text }),
  report: (current: Snapshot, target: Snapshot) => invoke<Report>("progression_report", { current, target }),
};

/** The saved plan, moving the old local-storage copy in first when the app has none yet. */
export async function loadPlan(storage: Pick<Storage, "getItem"> | null = globalThis.localStorage ?? null): Promise<Plan> {
  const saved = await progression.load();
  if (saved) return saved;
  const legacy = storage?.getItem(LEGACY_STORAGE_KEY);
  return legacy ? progression.migrate(legacy) : emptyPlan();
}
