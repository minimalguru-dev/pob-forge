import { mount } from "svelte";
import "../../src/lib/state/locale.svelte";
import "@fontsource-variable/geist";
import "../../src/app.css";
import ProgressionView from "../../src/lib/views/ProgressionView.svelte";
import { game } from "../../src/lib/state/game.svelte";
import { build } from "../../src/lib/state/build.svelte";
game.current = "poe1";
if (location.search === "?view=import") {
  // Keep the actual import dispatch; the rest of the build UI is outside this fixture.
  build.sync = async () => {};
  const { default: ImportView } = await import("../../src/lib/views/ImportView.svelte");
  mount(ImportView, { target: document.getElementById("app")! });
} else {
  mount(ProgressionView, { target: document.getElementById("app")! });
}
