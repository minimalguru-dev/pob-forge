import { getCurrentWindow } from "@tauri-apps/api/window";
import { relaunch } from "@tauri-apps/plugin-process";
import { check, type Update } from "@tauri-apps/plugin-updater";

const DISMISS_KEY = "pob-forge:update";
const AUTO_KEY = "pob-forge:update:auto";
/** Only builds signed with this app's own updater key set this (scripts/publish-update.ts). */
const ENABLED = import.meta.env.VITE_UPDATER === "true";
/** The app stays open through play sessions, so it looks again now and then, not only at start. */
const RECHECK_MS = 6 * 60 * 60 * 1000;

export type UpdatePhase = "idle" | "checking" | "downloading" | "ready" | "error";

/**
 * App auto-update, the way Path of Building does it: look for a new version
 * after start and every few hours, download it in the background, then offer
 * a restart. An update nobody restarted for is installed when the app closes.
 *
 * The download is verified against the public key baked into the build before
 * anything is installed, so a compromised update host cannot ship a payload.
 */
class UpdateStore {
  readonly enabled = ENABLED;
  phase = $state<UpdatePhase>("idle");
  version = $state<string | null>(null);
  notes = $state<string | null>(null);
  error = $state<string | null>(null);
  /** 0-100 while downloading, null when the server sends no length. */
  progress = $state<number | null>(null);
  /** Set when the user closes the banner, so it stays closed for that version. */
  dismissed = $state<string | null>(null);
  /** Look for updates by itself; off leaves only the Settings button. */
  auto = $state(true);
  installing = $state(false);
  /** Set after a manual check that found nothing. */
  upToDate = $state(false);

  private update: Update | null = null;
  private downloaded = 0;
  private total = 0;
  private offClose: (() => void) | null = null;

  get showBanner() {
    return this.phase === "ready" && this.version !== this.dismissed;
  }

  init() {
    if (!ENABLED) return;
    try {
      this.dismissed = localStorage.getItem(DISMISS_KEY);
      this.auto = localStorage.getItem(AUTO_KEY) !== "off";
    } catch {}
    // Let the engine finish booting before touching the network.
    setTimeout(() => void this.check(false), 4000);
    setInterval(() => void this.check(false), RECHECK_MS);
  }

  setAuto(on: boolean) {
    this.auto = on;
    try {
      localStorage.setItem(AUTO_KEY, on ? "on" : "off");
    } catch {}
    if (on) void this.check(false);
  }

  dismiss() {
    this.dismissed = this.version;
    try {
      if (this.version) localStorage.setItem(DISMISS_KEY, this.version);
    } catch {}
  }

  /** `manual` reports "up to date" and errors; the automatic checks stay quiet. */
  async check(manual = true) {
    if (!ENABLED) {
      this.error = "This build does not update itself. Download the installer again to get a new version.";
      this.phase = "error";
      return;
    }
    if (!manual && !this.auto) return;
    // A downloaded update waits for its restart; a newer one is picked up after that.
    if (this.phase === "checking" || this.phase === "downloading" || this.phase === "ready") return;
    this.phase = "checking";
    this.error = null;
    this.upToDate = false;
    try {
      const found = await check();
      if (!found) {
        this.phase = "idle";
        this.version = null;
        this.upToDate = manual;
        return;
      }
      await this.update?.close().catch(() => {});
      this.update = found;
      this.version = found.version;
      this.notes = found.body?.trim() || null;
    } catch (e) {
      this.fail(e, manual);
      return;
    }
    await this.download(manual);
  }

  private async download(manual: boolean) {
    if (!this.update) return;
    this.phase = "downloading";
    this.progress = null;
    this.downloaded = 0;
    this.total = 0;
    try {
      await this.update.download((event) => {
        if (event.event === "Started") {
          this.total = event.data.contentLength ?? 0;
        } else if (event.event === "Progress") {
          this.downloaded += event.data.chunkLength;
          this.progress = this.total ? Math.min(100, Math.round((this.downloaded / this.total) * 100)) : null;
        } else if (event.event === "Finished") {
          this.progress = 100;
        }
      });
      this.phase = "ready";
      await this.installOnClose();
    } catch (e) {
      // The next automatic check downloads it again.
      this.fail(e, true);
      if (!manual) console.warn("update download failed:", e);
    }
  }

  private fail(e: unknown, show: boolean) {
    this.phase = show ? "error" : "idle";
    this.error = show ? String(e) : null;
    if (!show) console.warn("update check failed:", e);
  }

  /** On Windows the installer closes the app and starts the new version; on macOS we relaunch. */
  async restart() {
    if (!this.update || this.installing) return;
    this.installing = true;
    try {
      await this.update.install();
      await relaunch();
    } catch (e) {
      this.installing = false;
      this.fail(e, true);
    }
  }

  /**
   * Closing the main window with a downloaded update installs it without
   * starting the app again. Listening only once there is one keeps an
   * ordinary close exactly as it was.
   */
  private async installOnClose() {
    if (this.offClose) return;
    const win = getCurrentWindow();
    if (win.label !== "main") return;
    this.offClose = await win.onCloseRequested(async (e) => {
      if (this.phase !== "ready" || !this.update || this.installing) return;
      e.preventDefault();
      this.installing = true;
      try {
        await this.update.install({ restartAfterInstall: false });
      } catch (err) {
        console.warn("update install on close failed:", err);
      }
      await win.destroy();
    });
  }
}

export const appUpdate = new UpdateStore();
