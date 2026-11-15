import { mount } from "svelte";
import "../../src/lib/state/locale.svelte";
import "@fontsource-variable/geist";
import "../../src/app.css";
import ProgressionView from "../../src/lib/views/ProgressionView.svelte";
import { game } from "../../src/lib/state/game.svelte";
game.current = "poe1";
mount(ProgressionView, { target: document.getElementById("app")! });
