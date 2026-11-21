import { mount } from "svelte";
import "../../src/lib/state/locale.svelte";
import "@fontsource-variable/inter";
import "../../src/app.css";
import ImportView from "../../src/lib/views/ImportView.svelte";
import { game } from "../../src/lib/state/game.svelte";
import { build } from "../../src/lib/state/build.svelte";
game.current = "poe1";
// Keep the actual import dispatch; the rest of the build UI is outside this fixture.
build.sync = async () => {};
mount(ImportView, { target: document.getElementById("app")! });
