// Look-only fixture for the Instrument shell: header, stat sheet and change ledger
// with made-up sidebar figures. It is not engine validation: no PoB numbers here.
type Call = { method: string };
(window as unknown as { __TAURI_INTERNALS__: unknown }).__TAURI_INTERNALS__ = {
  metadata: { currentWindow: { label: "main" }, currentWebview: { windowLabel: "main", label: "main" } },
  transformCallback: () => 0,
  unregisterCallback: () => {},
  invoke: async (cmd: string, args: Call) => {
    if (cmd === "engine_call") {
      const result = args.method === "get_loadouts" ? { loadouts: [], active: null } : null;
      return { result, elapsed_ms: 1 };
    }
    if (cmd.startsWith("plugin:window|is_maximized")) return false;
    return null;
  },
};

const { mount } = await import("svelte");
await import("../../src/lib/state/locale.svelte");
await import("@fontsource-variable/inter");
await import("@fontsource-variable/jetbrains-mono");
await import("@fontsource/cormorant-sc/600.css");
await import("@fontsource/cormorant-sc/700.css");
await import("../../src/app.css");
const { build } = await import("../../src/lib/state/build.svelte");
const { ledger, statMap } = await import("../../src/lib/state/ledger.svelte");
const { game } = await import("../../src/lib/state/game.svelte");
const { default: Shell } = await import("./InstrumentShell.svelte");

game.current = "poe1";
document.documentElement.dataset.theme = new URLSearchParams(location.search).get("theme") ?? "dark";

type Row = [stat: string | null, lhs: string | null, rhs: string | null];
const rows = (dps: string, hit: string, ehp: string, maxHit: string): Row[] => [
  ["CombinedDPS", "^7Combined DPS:", dps],
  [null, null, null],
  ["TotalDPS", "^7Hit DPS:", hit],
  ["Speed", "^7Attack Rate:", "7.90"],
  ["CritChance", "^7Crit Chance:", "71.00%"],
  [null, null, null],
  ["Life", "^xE05030Life:", "4,812"],
  ["Mana", "^x7070FFMana:", "1,104"],
  ["ManaCost", "^7Mana Cost:", "18"],
  [null, null, null],
  ["TotalEHP", "^7Effective Hit Pool:", ehp],
  ["PhysicalMaximumHitTaken", "^7Phys Max Hit:", maxHit],
  ["Evasion", "^x33FF77Evasion rating:", "21,450"],
  [null, null, null],
  ["FireResist", "^xB97123Fire Resistance:", "75%"],
  ["ColdResist", "^x3F6DB3Cold Resistance:", "75%"],
  ["LightningResist", "^xADAA47Lightning Resistance:", "76%"],
  ["ChaosResist", "^xD02090Chaos Resistance:", "-12%"],
];
const sidebar = (r: Row[]) => ({
  rev: 1,
  warnings: [],
  rows: r.map(([stat, lhs, rhs]) => ({ h: 16, lhs, rhs, breakdown: null, hasBreakdown: false, align: null, stat, actor: stat ? ("player" as const) : null })),
});

const info = {
  generation: 1, name: "Lightning Arrow Deadeye", file: null, level: 94, levelAuto: false, classId: 2, className: "Ranger",
  ascendClassId: 1, ascendClassName: "Deadeye", secondaryAscendClassId: 0, secondaryAscendClassName: null, mainSocketGroup: 1,
  treeVersion: "3_26", rev: 1, unsaved: true, title: "", targetVersion: "3_0",
  points: { used: 118, max: 123, ascUsed: 8, ascMax: 8, weaponSet1Used: 0, weaponSet2Used: 0, weaponSetMax: 0, socketsUsed: 0 },
};

const saved = sidebar(rows("4,017,700", "3,445,000", "38,214", "10,080"));
const afterTree = sidebar(rows("4,266,800", "3,662,000", "38,214", "10,080"));
const now = sidebar(rows("4,210,550", "3,620,000", "37,900", "9,960"));

build.info = info as never;
build.sidebar = saved as never;
ledger.follow(1, saved as never);
ledger.record("Tree", statMap(saved as never), statMap(afterTree as never));
ledger.record("Items", statMap(afterTree as never), statMap(now as never));
build.sidebar = now as never;
build.view = (new URLSearchParams(location.search).get("view") as never) ?? "tree";

mount(Shell, { target: document.getElementById("app")! });
