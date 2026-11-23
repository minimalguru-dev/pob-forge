// Look-only fixture for the app update banner and status bar button: a fake updater backend finds
// 0.5.9, streams its download, and records installs. Needs VITE_UPDATER=true in the dev server.
// Not proof that a real signed update installs.
const callbacks = new Map<number, (m: unknown) => void>();
let nextId = 1;
const installs: unknown[] = [];
const notes = "## Features\n\n- Price check reads Foulborn mods\n- Overlay remembers its position\n\n## Fixes\n\n- Tree search keeps focus";
(window as unknown as { __TAURI_INTERNALS__: unknown }).__TAURI_INTERNALS__ = {
  metadata: { currentWindow: { label: "main" }, currentWebview: { windowLabel: "main", label: "main" } },
  transformCallback: (cb: (m: unknown) => void) => {
    const id = nextId++;
    callbacks.set(id, cb);
    return id;
  },
  unregisterCallback: (id: number) => callbacks.delete(id),
  invoke: async (cmd: string, args: Record<string, unknown>) => {
    if (cmd === "plugin:updater|check") return { rid: 1, currentVersion: "0.5.8", version: "0.5.9", body: notes, rawJson: {} };
    if (cmd === "plugin:updater|download") {
      const send = callbacks.get((args.onEvent as { id: number }).id)!;
      let index = 0;
      const emit = (message: unknown) => send({ message, index: index++ });
      emit({ event: "Started", data: { contentLength: 1000 } });
      for (let i = 0; i < 10; i++) {
        await new Promise((r) => setTimeout(r, 120));
        emit({ event: "Progress", data: { chunkLength: 100 } });
      }
      emit({ event: "Finished" });
      return 2;
    }
    if (cmd === "plugin:updater|install") {
      installs.push(args);
      return null;
    }
    if (cmd === "plugin:event|listen") return nextId++;
    return null;
  },
};

const { mount } = await import("svelte");
await import("../../src/lib/state/locale.svelte");
await import("@fontsource-variable/inter");
await import("@fontsource-variable/jetbrains-mono");
await import("../../src/app.css");
const { appUpdate } = await import("../../src/lib/state/update.svelte");
const { default: UpdateBanner } = await import("../../src/lib/components/UpdateBanner.svelte");
const { default: StatusBar } = await import("../../src/lib/components/StatusBar.svelte");
document.documentElement.dataset.theme = "dark";
const box = document.getElementById("app")!;
box.style.cssText = "display:flex; flex-direction:column; height:100vh; background: var(--bg-0);";
mount(UpdateBanner, { target: box });
const grow = document.createElement("div");
grow.style.flex = "1";
box.appendChild(grow);
mount(StatusBar, { target: box, props: { status: null, paths: null } });
Object.assign(window, { appUpdate, installs, check: () => appUpdate.check(false) });
