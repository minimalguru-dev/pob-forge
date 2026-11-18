import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

/** Mirrors `OverlaySettings` in src-tauri/src/game.rs. */
export interface OverlaySettings {
  enabled: boolean;
  toggleKey: string;
  interactKey: string;
  logPath: string | null;
  x: number | null;
  y: number | null;
  width: number;
  height: number;
  opacity: number;
  itemCheck: boolean;
  mapCheck: boolean;
  prices: boolean;
  priceLeague: string | null;
  cardSeconds: number;
  macros: { key: string; command: string }[];
}

/** The in-game overlay's settings, shared by the Settings page and the overlay window. */
class OverlayStore {
  settings = $state<OverlaySettings | null>(null);
  error = $state<string | null>(null);
  saving = $state(false);
  private listening = false;

  async init() {
    try {
      this.settings = await invoke<OverlaySettings>("overlay_settings_get");
    } catch (e) {
      this.error = String(e);
    }
    if (!this.listening) {
      this.listening = true;
      void listen<OverlaySettings>("overlay:settings", (e) => (this.settings = e.payload));
    }
  }

  async update(patch: Partial<OverlaySettings>) {
    if (!this.settings) return;
    this.saving = true;
    this.error = null;
    try {
      this.settings = await invoke<OverlaySettings>("overlay_settings_set", { value: { ...this.settings, ...patch } });
    } catch (e) {
      this.error = String(e);
    } finally {
      this.saving = false;
    }
  }

  toggle() {
    return invoke("overlay_toggle").catch((e) => (this.error = String(e)));
  }
}

export const overlay = new OverlayStore();
