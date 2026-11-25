// Look-only fixture for the Notes tab with a made-up guide; ?empty shows the empty state.
const guide = [
  "^xFFFF77=== Leveling ===",
  "^7Take ^xAF6025Tabula Rasa^7 early and link ^x8888FFLightning Arrow^7 by act 3.",
  "Video guide: https://youtu.be/example. Build: https://pobb.in/AbC123",
  "",
  "BANDITS",
  "Kill all of them for the two points.",
  "",
  "# Gear",
  "^xB97123Fire^7, ^x3F6DB3Cold^7 and ^xADAA47Lightning^7 resistances capped before maps.",
  "  - Boots with 30% movement speed",
  "  - Ring with life and two resists",
  "",
  "## Mapping",
  "Swap to ^xD02090Chaos^7 resistance once T14+.",
].join("\n");
let notes = new URLSearchParams(location.search).has("empty") ? "" : guide;

(window as unknown as { __TAURI_INTERNALS__: unknown }).__TAURI_INTERNALS__ = {
  metadata: { currentWindow: { label: "main" }, currentWebview: { windowLabel: "main", label: "main" } },
  transformCallback: () => 0,
  unregisterCallback: () => {},
  invoke: async (cmd: string, args: { method: string; params?: { text?: string } }) => {
    if (cmd === "engine_call" && args.method === "get_notes") return { result: { text: notes }, elapsed_ms: 1 };
    if (cmd === "engine_call" && args.method === "set_notes") {
      notes = args.params?.text ?? "";
      return { result: { ok: true }, elapsed_ms: 1 };
    }
    if (cmd === "open_link") console.log("open_link", (args as unknown as { url: string }).url);
    return null;
  },
};

const { mount } = await import("svelte");
await import("../../src/lib/state/locale.svelte");
await import("@fontsource-variable/inter");
await import("@fontsource-variable/jetbrains-mono");
await import("../../src/app.css");
const { build } = await import("../../src/lib/state/build.svelte");
const { default: NotesView } = await import("../../src/lib/views/NotesView.svelte");

document.documentElement.dataset.theme = new URLSearchParams(location.search).get("theme") ?? "dark";
build.info = { name: "Lightning Arrow Deadeye", file: "fixture.xml", generation: 1 } as never;
build.sync = async () => {};
const target = document.getElementById("app")!;
target.style.cssText = "height:100vh;display:flex;flex-direction:column";
mount(NotesView, { target });
