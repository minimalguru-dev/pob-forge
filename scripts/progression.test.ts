import { expect, test } from "bun:test";
import { addSnapshot, assess, decodePlan, diffSnapshots, emptyPlan, MAX_BYTES, numberStat, priorities, removeSnapshot, savePlan, STORAGE_KEY, type Snapshot } from "../src/lib/progression/model";

function snapshot(patch: Partial<Snapshot> = {}): Snapshot {
  return {
    id: "one", at: "2026-11-15T12:00:00Z", xml: "<PathOfBuilding><Build/></PathOfBuilding>", label: "Test fixture", game: "poe1",
    pobVersion: "test", treeVersion: "test-tree", level: 80, className: "Duelist", ascendancy: "Slayer", mainSkill: "Shield Crush",
    nodes: [{ id: 1, name: "Start", mastery: null }], equipment: [], gems: [], gemErrors: [], config: {},
    stats: { Str: 100, Dex: 100, Int: 100, ReqStr: 50, ReqDex: 50, ReqInt: 50, CombinedDPS: 10, FireResist: 75, ColdResist: 75, LightningResist: 75, FireResistMax: 75, ColdResistMax: 75, LightningResistMax: 75 },
    loadout: { tree: "Default", items: "1", skills: "1" }, boss: { kind: "Pinnacle", stats: {}, overrides: {} }, ...patch,
  };
}
const item = (raw: string, slot = "Weapon 1") => ({ slot, name: "Same rare name", raw, variant: null, variant2: null });

test("equipment diff detects changed rolls with identical names and new or removed slots", () => {
  const diff = diffSnapshots(snapshot({ equipment: [item("10% attack speed"), item("shield", "Weapon 2")] }), snapshot({ equipment: [item("20% attack speed"), item("ring", "Ring 1")] }));
  expect(diff.equipment.map(i => i.slot)).toEqual(["Ring 1", "Weapon 1", "Weapon 2"]);
});
test("line endings normalize but variants remain significant", () => {
  expect(diffSnapshots(snapshot({ equipment: [item("a\r\nb")] }), snapshot({ equipment: [item("a\nb")] })).equipment).toHaveLength(0);
  expect(diffSnapshots(snapshot({ equipment: [item("a")] }), snapshot({ equipment: [{ ...item("a"), variant: 2 }] })).equipment).toHaveLength(1);
});
test("tree diff reports gained, removed and changed masteries separately", () => {
  const diff = diffSnapshots(snapshot({ nodes: [{ id: 1, name: "Mastery", mastery: 4 }, { id: 2, name: "Old", mastery: null }] }), snapshot({ nodes: [{ id: 1, name: "Mastery", mastery: 5 }, { id: 3, name: "New", mastery: null }] }));
  expect(diff.gained.map(n => n.id)).toEqual([3]);
  expect(diff.lost.map(n => n.id)).toEqual([2]);
  expect(diff.masteries.map(n => n.id)).toEqual([1]);
});
test("different tree versions do not produce misleading node diffs", () => {
  const diff = diffSnapshots(snapshot(), snapshot({ treeVersion: "other", nodes: [] }));
  expect(diff.compatibleTree).toBe(false);
  expect(diff.lost).toEqual([]);
});
test("class mismatch and unmet requirements block; matching gear never certifies readiness", () => {
  const checks = assess(snapshot(), snapshot({ className: "Witch", stats: { Int: 20, ReqInt: 149 } }));
  expect(checks.find(c => c.id === "class")?.status).toBe("blocked");
  expect(checks.find(c => c.id === "Target-Int")?.status).toBe("blocked");
  expect(assess(snapshot(), snapshot()).find(c => c.id === "manual")?.status).toBe("review");
});
test("missing attributes are unknown rather than zero or pass", () => {
  expect(assess(snapshot({ stats: {} }), snapshot()).find(c => c.id === "Current-Str")?.status).toBe("review");
  expect(numberStat({}, "CombinedDPS")).toBeNull();
  expect(numberStat({ CombinedDPS: 0 }, "CombinedDPS")).toBe(0);
  expect(numberStat({ CombinedDPS: Infinity }, "CombinedDPS")).toBeNull();
});
test("resource and gem errors outrank gear planning", () => {
  const a = snapshot({ stats: { ManaCostWarning: true }, gemErrors: ["Not enough intelligence"] });
  const b = snapshot({ equipment: [item("target")] });
  const list = priorities(a, b);
  expect(list[0].status).toBe("blocked");
  expect(list.findIndex(c => c.id === "Current-errors")).toBeLessThan(list.findIndex(c => c.id === "gear"));
});
test("configuration differences remain visible even with identical DPS", () => {
  const a = snapshot({ config: { useFrenzyCharges: true } });
  expect(diffSnapshots(a, snapshot()).config).toEqual(["useFrenzyCharges"]);
  expect(assess(a, snapshot()).find(c => c.id === "config")?.status).toBe("review");
});
test("disabled gems do not satisfy target preparation", () => {
  const gem = { group: 1, name: "Dual Strike of Ambidexterity", level: 1, quality: 0, enabled: false };
  expect(assess(snapshot({ gems: [gem] }), snapshot({ gems: [{ ...gem, enabled: true }] })).find(c => c.id === "gems")?.status).toBe("review");
});
test("history round trips with full XML and current / target selection", () => {
  const first = addSnapshot(emptyPlan(), snapshot(), "current");
  const plan = addSnapshot(first, snapshot({ id: "two" }), "target");
  expect(decodePlan(JSON.stringify(plan))).toEqual(plan);
  expect(first.snapshots).toHaveLength(1);
  expect(removeSnapshot(plan, "one").currentId).toBeNull();
  expect(removeSnapshot(plan, "one").targetId).toBe("two");
});
test("full history refuses capture instead of silently deleting snapshots", () => {
  let plan = emptyPlan();
  for (let i = 0; i < 20; i++) plan = addSnapshot(plan, snapshot({ id: String(i) }), "current");
  expect(() => addSnapshot(plan, snapshot(), "target")).toThrow("History is full");
});
test("invalid and future history cannot overwrite valid history", () => {
  for (const text of ["broken", "null", '{"schema":2,"snapshots":[]}', JSON.stringify({ ...emptyPlan(), currentId: "missing" }), JSON.stringify({ ...emptyPlan(), snapshots: [snapshot(), snapshot()] }), JSON.stringify({ ...emptyPlan(), snapshots: [{ ...snapshot(), equipment: [null] }] })]) {
    expect(() => decodePlan(text)).toThrow();
  }
  expect(() => decodePlan(" ".repeat(MAX_BYTES + 1))).toThrow("4 MiB");
});
test("quota failure propagates and caller retains its previous plan", () => {
  const plan = addSnapshot(emptyPlan(), snapshot(), "current");
  const storage = { setItem: () => { throw new Error("quota exceeded"); } };
  expect(() => savePlan(storage, plan)).toThrow("quota exceeded");
  expect(plan.currentId).toBe("one");
});
test("duplicate node IDs or slots and nested non-scalar values are rejected before rendering", () => {
  const base = snapshot();
  for (const malformed of [
    { ...base, nodes: [base.nodes[0], base.nodes[0]] },
    { ...base, equipment: [item("a"), item("b")] },
    { ...base, stats: { Life: { invalid: true } } },
    { ...base, config: { enemyIsBoss: ["Pinnacle"] } },
    { ...base, at: "not a date" },
    { ...base, loadout: {} },
  ]) expect(() => decodePlan(JSON.stringify({ ...emptyPlan(), snapshots: [malformed] }))).toThrow();
});
test("successful persistence uses its own versioned PoE1 key", () => {
  const writes: string[][] = [];
  savePlan({ setItem: (key, value) => { writes.push([key, value]); } }, emptyPlan());
  expect(writes[0][0]).toBe(STORAGE_KEY);
  expect(decodePlan(writes[0][1])).toEqual(emptyPlan());
});
