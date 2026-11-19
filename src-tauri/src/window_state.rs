//! The main window's size, position and maximized state, kept across restarts.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, PhysicalPosition, PhysicalSize, WebviewWindow};

const FILE: &str = "window.json";
/// How much of the title bar must land on a monitor for the saved place to be used.
const MIN_VISIBLE: i64 = 160;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
struct Bounds {
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    #[serde(default)]
    maximized: bool,
}

/// A monitor's work area: x, y, width, height.
type Rect = (i32, i32, u32, u32);

fn path(app: &AppHandle) -> Option<PathBuf> {
    app.path().app_config_dir().ok().map(|d| d.join(FILE))
}

fn read(path: &Path) -> Option<Bounds> {
    serde_json::from_slice(&std::fs::read(path).ok()?).ok()
}

/// Puts the main window back where it was closed, unless that place is off every monitor now.
pub fn restore(app: &AppHandle) {
    let Some(window) = app.get_webview_window("main") else { return };
    let Some(bounds) = path(app).as_deref().and_then(read) else { return };
    let monitors: Vec<Rect> = window
        .available_monitors()
        .unwrap_or_default()
        .iter()
        .map(|m| (m.position().x, m.position().y, m.size().width, m.size().height))
        .collect();
    if !on_screen(&bounds, &monitors) {
        return;
    }
    let _ = window.set_size(PhysicalSize::new(bounds.width, bounds.height));
    let _ = window.set_position(PhysicalPosition::new(bounds.x, bounds.y));
    if bounds.maximized {
        let _ = window.maximize();
    }
}

/// Called as the main window closes. A maximized or minimized window keeps the
/// last normal size and place, so un-maximizing after a restart goes back there.
pub fn save(window: &WebviewWindow) {
    let Some(path) = path(window.app_handle()) else { return };
    let maximized = window.is_maximized().unwrap_or(false);
    let previous = read(&path);
    let bounds = match previous {
        Some(p) if window.is_minimized().unwrap_or(false) => p,
        Some(p) if maximized => Bounds { maximized: true, ..p },
        _ => {
            let (Ok(pos), Ok(size)) = (window.outer_position(), window.inner_size()) else { return };
            Bounds { x: pos.x, y: pos.y, width: size.width, height: size.height, maximized }
        }
    };
    if previous == Some(bounds) {
        return;
    }
    if let Ok(bytes) = serde_json::to_vec(&bounds) {
        let _ = crate::art::write_atomic(&path, &bytes);
    }
}

fn on_screen(b: &Bounds, monitors: &[Rect]) -> bool {
    if b.width == 0 || b.height == 0 {
        return false;
    }
    let (left, right, top) = (b.x as i64, b.x as i64 + b.width as i64, b.y as i64);
    monitors.iter().any(|&(mx, my, mw, mh)| {
        let (mx, my) = (mx as i64, my as i64);
        let overlap = right.min(mx + mw as i64) - left.max(mx);
        overlap >= MIN_VISIBLE && top >= my && top < my + mh as i64 - 40
    })
}

#[cfg(test)]
mod tests {
    use super::{on_screen, Bounds};

    const DESK: (i32, i32, u32, u32) = (0, 0, 1920, 1080);
    const RIGHT: (i32, i32, u32, u32) = (1920, 0, 2560, 1440);

    fn at(x: i32, y: i32) -> Bounds {
        Bounds { x, y, width: 1520, height: 960, maximized: false }
    }

    #[test]
    fn keeps_a_place_on_any_monitor() {
        assert!(on_screen(&at(100, 50), &[DESK]));
        assert!(on_screen(&at(2200, 100), &[DESK, RIGHT]));
        // Mostly off the left edge, but enough of the title bar still shows.
        assert!(on_screen(&at(-1300, 0), &[DESK]));
    }

    #[test]
    fn drops_a_place_that_left_every_monitor() {
        // The second monitor was unplugged.
        assert!(!on_screen(&at(2200, 100), &[DESK]));
        // Title bar above the top edge cannot be grabbed.
        assert!(!on_screen(&at(100, -200), &[DESK]));
        assert!(!on_screen(&at(-1400, 0), &[DESK]));
        assert!(!on_screen(&Bounds { width: 0, ..at(0, 0) }, &[DESK]));
    }
}
