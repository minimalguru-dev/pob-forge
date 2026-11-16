import { expect, test } from "bun:test";
import { emptyPlan, LEGACY_STORAGE_KEY, loadPlan, numberStat } from "../src/lib/progression/model";

// Diffs, readiness checks, priorities and history validation live in src-tauri/src/progression.rs
// and are tested there with `cargo test -p pob-redux --lib progression`.

type Call = [string, Record<string, unknown> | undefined];
function fakeTauri(replies: Record<string, unknown>) {
  const calls: Call[] = [];
  (globalThis as any).window = globalThis;
  (globalThis as any).__TAURI_INTERNALS__ = {
    invoke: async (cmd: string, args?: Record<string, unknown>) => {
      calls.push([cmd, args]);
      const reply = replies[cmd];
      if (reply instanceof Error) throw reply;
      return reply;
    },
    transformCallback: () => 0,
  };
  return calls;
}
const storage = (value: string | null) => ({ getItem: (key: string) => (key === LEGACY_STORAGE_KEY ? value : null) });

test("missing attributes are unknown rather than zero", () => {
  expect(numberStat({}, "CombinedDPS")).toBeNull();
  expect(numberStat({ CombinedDPS: 0 }, "CombinedDPS")).toBe(0);
  expect(numberStat({ CombinedDPS: Infinity }, "CombinedDPS")).toBeNull();
  expect(numberStat({ CombinedDPS: true }, "CombinedDPS")).toBeNull();
});

test("a saved plan wins and local storage is not read", async () => {
  const saved = { ...emptyPlan(), currentId: null };
  const calls = fakeTauri({ progression_load: saved });
  expect(await loadPlan(storage("ignored"))).toEqual(saved);
  expect(calls.map(c => c[0])).toEqual(["progression_load"]);
});

test("old local-storage history is migrated once", async () => {
  const migrated = emptyPlan();
  const calls = fakeTauri({ progression_load: null, progression_migrate: migrated });
  expect(await loadPlan(storage('{"schema":1}'))).toEqual(migrated);
  expect(calls[1]).toEqual(["progression_migrate", { text: '{"schema":1}' }]);
});

test("no history anywhere starts empty without writing", async () => {
  const calls = fakeTauri({ progression_load: null });
  expect(await loadPlan(storage(null))).toEqual(emptyPlan());
  expect(calls.map(c => c[0])).toEqual(["progression_load"]);
});

test("a corrupt saved history is reported, never replaced", async () => {
  const calls = fakeTauri({ progression_load: new Error("Unsupported Progression history format.") });
  await expect(loadPlan(storage('{"schema":1}'))).rejects.toThrow("Unsupported");
  expect(calls.map(c => c[0])).toEqual(["progression_load"]);
});
