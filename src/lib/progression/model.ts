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
export const MAX_BYTES = 4 * 1024 * 1024;

export function numberStat(stats: Stats, key: string): number | null {
  const value = stats[key];
  return typeof value === "number" && Number.isFinite(value) ? value : null;
}

function itemSignature(item: Snapshot["equipment"][number] | undefined) {
  return item ? JSON.stringify([item.raw.replace(/\r\n/g, "\n").trim(), item.variant, item.variant2]) : null;
}

export function diffSnapshots(current: Snapshot, target: Snapshot) {
  const a = new Map(current.equipment.map((item) => [item.slot, item]));
  const b = new Map(target.equipment.map((item) => [item.slot, item]));
  const equipment = [...new Set([...a.keys(), ...b.keys()])].sort().flatMap((slot) =>
    itemSignature(a.get(slot)) === itemSignature(b.get(slot)) ? [] : [{ slot, current: a.get(slot), target: b.get(slot) }]);
  const compatibleTree = current.treeVersion === target.treeVersion && current.className === target.className;
  const mine = new Map(current.nodes.map((node) => [node.id, node]));
  const theirs = new Map(target.nodes.map((node) => [node.id, node]));
  return {
    equipment, compatibleTree,
    gained: compatibleTree ? target.nodes.filter((node) => !mine.has(node.id)) : [],
    lost: compatibleTree ? current.nodes.filter((node) => !theirs.has(node.id)) : [],
    masteries: compatibleTree ? target.nodes.filter((node) => mine.has(node.id) && mine.get(node.id)!.mastery !== node.mastery) : [],
    config: [...new Set([...Object.keys(current.config), ...Object.keys(target.config)])].sort()
      .filter((key) => current.config[key] !== target.config[key]),
  };
}

export interface Check {
  id: string;
  status: "blocked" | "review" | "pass";
  title: string;
  detail: string;
}

export function assess(current: Snapshot, target: Snapshot): Check[] {
  const diff = diffSnapshots(current, target);
  const checks: Check[] = [];
  const add = (id: string, status: Check["status"], title: string, detail: string) => checks.push({ id, status, title, detail });
  add("class", current.className === target.className ? "pass" : "blocked", "Character class",
    `${current.className} → ${target.className}. A different base class requires another character.`);
  add("ascendancy", current.ascendancy === target.ascendancy ? "pass" : "review", "Ascendancy", `${current.ascendancy} → ${target.ascendancy}`);
  add("versions", current.pobVersion === target.pobVersion && diff.compatibleTree ? "pass" : "review", "Engine and tree versions",
    "Re-capture both builds with the same PoB version before comparing. Tree node IDs from different trees are not compared.");
  add("level", current.level >= target.level ? "pass" : "review", "Target level", `Current ${current.level}; target ${target.level}. Select an appropriate earlier target loadout if needed.`);
  add("gear", diff.equipment.length ? "review" : "pass", "Target equipment", `${diff.equipment.length} slot changes. Different gear is not necessarily worse; ownership and budget are not known.`);
  const missing = [...new Set(target.gems.filter((gem) => gem.enabled && !current.gems.some((g) => g.enabled && g.name === gem.name)).map((g) => g.name))];
  add("gems", missing.length ? "review" : "pass", "Target gems", missing.length ? `Prepare: ${missing.join(", ")}. Verify links, colours and gem levels in Skills.` : "Enabled target gem names are present. Verify their levels, supports and socket links.");
  add("tree", !diff.compatibleTree || diff.gained.length || diff.lost.length || diff.masteries.length ? "review" : "pass", "Passive transition",
    diff.compatibleTree ? `Add ${diff.gained.length}, remove ${diff.lost.length}, change ${diff.masteries.length} masteries. Node counts are not a refund-cost estimate.` : "Different tree versions or classes: node diff unavailable.");
  for (const [side, snapshot] of [["Current", current], ["Target", target]] as const) {
    for (const attr of ["Str", "Dex", "Int"]) {
      const have = numberStat(snapshot.stats, attr), need = numberStat(snapshot.stats, `Req${attr}`);
      add(`${side}-${attr}`, have === null || need === null ? "review" : have < need ? "blocked" : "pass",
        `${side} ${attr} requirements`, `${have ?? "Unknown"} available / ${need ?? "Unknown"} required by PoB.`);
    }
    if (snapshot.stats.ManaCostWarning || snapshot.stats.LifeCostWarning) add(`${side}-cost`, "blocked", `${side} skill resource cost`, "PoB reports that the skill costs more than the unreserved pool. Resolve this in Config / Skills before switching.");
    if (snapshot.gemErrors.length) add(`${side}-errors`, "blocked", `${side} gem errors`, snapshot.gemErrors.join("; "));
    if (!snapshot.mainSkill || !(numberStat(snapshot.stats, "CombinedDPS") ?? numberStat(snapshot.stats, "TotalDPS")))
      add(`${side}-skill`, "review", `${side} main skill`, "No positive main-skill DPS reported. Select the intended damage skill and inspect Calcs.");
    for (const resist of ["Fire", "Cold", "Lightning"]) {
      const value = numberStat(snapshot.stats, `${resist}Resist`), cap = numberStat(snapshot.stats, `${resist}ResistMax`);
      if (value === null || cap === null || value < cap) add(`${side}-${resist}`, "review", `${side} ${resist.toLowerCase()} resistance`, `${value ?? "Unknown"}% / ${cap ?? "unknown"}% PoB cap. Check the boss profile without flasks too.`);
    }
  }
  if (diff.config.length) add("config", "review", "Different configuration", `${diff.config.length} options differ. Configured DPS is not a like-for-like upgrade estimate.`);
  add("manual", "review", "Validate the transition in play", "Confirm item ownership, links, refund budget, recovery, ailment protection, accuracy and sustainable charges. A target build is not a simulation of equipping its items on your current character.");
  return checks;
}

export function priorities(current: Snapshot, target: Snapshot): Check[] {
  const checks = assess(current, target).filter((check) => check.status !== "pass");
  const rank = (check: Check) => check.status === "blocked" ? 0 : /versions|config/.test(check.id) ? 1 : /gear|gems|tree/.test(check.id) ? 2 : 3;
  return checks.sort((a, b) => rank(a) - rank(b));
}

export function addSnapshot(plan: Plan, snapshot: Snapshot, role: "current" | "target"): Plan {
  if (plan.snapshots.length >= MAX_SNAPSHOTS) throw new Error("History is full (20 snapshots). Export a backup and remove a snapshot first.");
  return { ...plan, snapshots: [...plan.snapshots, snapshot], [role === "current" ? "currentId" : "targetId"]: snapshot.id };
}

export function removeSnapshot(plan: Plan, id: string): Plan {
  return { ...plan, snapshots: plan.snapshots.filter((s) => s.id !== id), currentId: plan.currentId === id ? null : plan.currentId, targetId: plan.targetId === id ? null : plan.targetId };
}

export const STORAGE_KEY = "pob-redux:progression:poe1:v1";

export function decodePlan(text: string): Plan {
  if (new TextEncoder().encode(text).length > MAX_BYTES) throw new Error("Progression data exceeds 4 MiB.");
  const plan = JSON.parse(text) as Plan;
  if (!plan || plan.schema !== 1 || !Array.isArray(plan.snapshots) || plan.snapshots.length > MAX_SNAPSHOTS) throw new Error("Unsupported Progression history format.");
  const ids = new Set<string>();
  const str = (v: unknown) => typeof v === "string";
  const obj = (v: unknown) => !!v && typeof v === "object" && !Array.isArray(v);
  for (const s of plan.snapshots) {
    if (!s || s.game !== "poe1" || !str(s.id) || ids.has(s.id) || !str(s.xml) || !str(s.label) || !str(s.at)
      || !str(s.pobVersion) || !str(s.treeVersion) || !str(s.className) || !str(s.ascendancy) || !Number.isFinite(s.level)
      || !(s.mainSkill === null || str(s.mainSkill)) || !obj(s.stats) || !obj(s.config) || !obj(s.loadout)
      || !s.boss || !["Boss", "Pinnacle", "Uber"].includes(s.boss.kind) || !obj(s.boss.stats) || !obj(s.boss.overrides)
      || !Array.isArray(s.nodes) || !s.nodes.every(n => Number.isInteger(n.id) && str(n.name) && (n.mastery === null || Number.isInteger(n.mastery)))
      || !Array.isArray(s.equipment) || !s.equipment.every(i => str(i.slot) && str(i.name) && str(i.raw))
      || !Array.isArray(s.gems) || !s.gems.every(g => str(g.name) && Number.isFinite(g.level) && Number.isFinite(g.quality) && typeof g.enabled === "boolean")
      || !Array.isArray(s.gemErrors) || !s.gemErrors.every(str)) throw new Error("Invalid Progression snapshot. Original history was not changed.");
    ids.add(s.id);
  }
  if ((plan.currentId !== null && !ids.has(plan.currentId)) || (plan.targetId !== null && !ids.has(plan.targetId))) throw new Error("History refers to a missing snapshot.");
  return plan;
}

export function savePlan(storage: Pick<Storage, "setItem">, plan: Plan) {
  const text = JSON.stringify(plan);
  decodePlan(text);
  storage.setItem(STORAGE_KEY, text);
}
