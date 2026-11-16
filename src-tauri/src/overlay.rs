//! The in-game overlay: a small always-on-top window that lets clicks through to the game until
//! the interact shortcut is pressed. It reads only what the user copies (Ctrl+C on an item) and,
//! through `gamelog`, the game's own Client.txt. It never reads game memory or sends input.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::Duration;

pub use crate::game::OverlaySettings;
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_clipboard_manager::ClipboardExt;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

pub const LABEL: &str = "overlay";
const POLL: Duration = Duration::from_millis(300);
/// Larger than any item or map the game copies; anything bigger is not ours to read.
const MAX_ITEM_TEXT: usize = 16 * 1024;

#[derive(Default)]
pub struct OverlayState {
    interact: AtomicBool,
    keys: Mutex<Option<(Shortcut, Shortcut)>>,
}

pub fn settings(app: &AppHandle) -> OverlaySettings {
    crate::game::load_settings(app).overlay
}

fn store(app: &AppHandle, overlay: OverlaySettings) -> Result<(), String> {
    let mut all = crate::game::load_settings(app);
    all.overlay = overlay;
    crate::game::save_settings(app, &all)
}

fn parse_key(text: &str) -> Result<Shortcut, String> {
    text.parse::<Shortcut>().map_err(|e| format!("not a shortcut: {text} ({e})"))
}

/// Takes the shortcuts the settings name, releasing the previous pair first.
fn register_keys(app: &AppHandle, s: &OverlaySettings) -> Result<(), String> {
    let state = app.state::<OverlayState>();
    let mut keys = state.keys.lock().unwrap_or_else(|e| e.into_inner());
    if let Some((a, b)) = keys.take() {
        let _ = app.global_shortcut().unregister(a);
        let _ = app.global_shortcut().unregister(b);
    }
    if !s.enabled {
        return Ok(());
    }
    let (toggle, interact) = (parse_key(&s.toggle_key)?, parse_key(&s.interact_key)?);
    if toggle == interact {
        return Err("The two overlay shortcuts must differ.".into());
    }
    for key in [toggle, interact] {
        app.global_shortcut()
            .register(key)
            .map_err(|e| format!("could not take {key}: {e}. Another program may already use it."))?;
    }
    *keys = Some((toggle, interact));
    Ok(())
}

pub fn on_shortcut(app: &AppHandle, shortcut: &Shortcut, state: ShortcutState) {
    if state != ShortcutState::Pressed {
        return;
    }
    let keys = *app.state::<OverlayState>().keys.lock().unwrap_or_else(|e| e.into_inner());
    let Some((toggle, interact)) = keys else { return };
    let app = app.clone();
    if *shortcut == toggle {
        tauri::async_runtime::spawn(async move {
            if let Err(e) = toggle_window(&app).await {
                log::warn!("overlay: {e}");
            }
        });
    } else if *shortcut == interact {
        tauri::async_runtime::spawn(async move { set_interact(&app, None) });
    }
}

async fn open_window(app: &AppHandle) -> Result<(), String> {
    if let Some(w) = app.get_webview_window(LABEL) {
        let _ = w.show();
        return Ok(());
    }
    let s = settings(app);
    let builder = WebviewWindowBuilder::new(app, LABEL, WebviewUrl::default())
        .title("PoB Redux Overlay")
        .decorations(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .focused(false)
        .shadow(false)
        .resizable(false)
        .inner_size(s.width, s.height);
    // Transparent windows need the private-API feature on macOS; there the overlay is opaque.
    #[cfg(not(target_os = "macos"))]
    let builder = builder.transparent(true);
    let window = builder.build().map_err(|e| e.to_string())?;
    let position = match (s.x, s.y) {
        (Some(x), Some(y)) => Some(PhysicalPosition::new(x, y)),
        _ => window.primary_monitor().ok().flatten().map(|m| {
            let scale = m.scale_factor();
            PhysicalPosition::new(m.position().x + m.size().width as i32 - ((s.width + 24.0) * scale) as i32, m.position().y + (96.0 * scale) as i32)
        }),
    };
    if let Some(p) = position {
        let _ = window.set_position(p);
    }
    app.state::<OverlayState>().interact.store(false, Ordering::Relaxed);
    let _ = window.set_ignore_cursor_events(true);
    Ok(())
}

pub fn close(app: &AppHandle) {
    if let Some(w) = app.get_webview_window(LABEL) {
        let _ = w.destroy();
    }
}

async fn toggle_window(app: &AppHandle) -> Result<(), String> {
    match app.get_webview_window(LABEL) {
        Some(w) if w.is_visible().unwrap_or(false) => {
            set_interact(app, Some(false));
            w.hide().map_err(|e| e.to_string())
        }
        _ => open_window(app).await,
    }
}

/// Interact mode makes the overlay clickable and movable; leaving it saves where it stands.
fn set_interact(app: &AppHandle, on: Option<bool>) {
    let Some(w) = app.get_webview_window(LABEL) else { return };
    let state = app.state::<OverlayState>();
    let on = on.unwrap_or(!state.interact.load(Ordering::Relaxed));
    let was = state.interact.swap(on, Ordering::Relaxed);
    let _ = w.set_ignore_cursor_events(!on);
    if on {
        let _ = w.set_focus();
    } else if was {
        if let Ok(p) = w.outer_position() {
            let mut s = settings(app);
            (s.x, s.y) = (Some(p.x), Some(p.y));
            let _ = store(app, s);
        }
    }
    let _ = app.emit_to(LABEL, "overlay:interact", on);
}

/// What kind of thing the game put on the clipboard, if it is one at all.
pub fn classify(text: &str) -> Option<&'static str> {
    let first = text.trim_start().lines().next()?.trim();
    let class = first.strip_prefix("Item Class:")?.trim();
    Some(match class {
        "Maps" | "Waystones" => "map",
        _ => "item",
    })
}

/// Watches the clipboard while the overlay is showing, for text the game wrote with Ctrl+C.
/// Other clipboard text is ignored and never logged or kept.
fn spawn_clipboard_watch(app: AppHandle) {
    std::thread::Builder::new()
        .name("overlay-clipboard".into())
        .spawn(move || {
            let mut last: Option<u64> = None;
            loop {
                std::thread::sleep(POLL);
                let showing = app.get_webview_window(LABEL).is_some_and(|w| w.is_visible().unwrap_or(false));
                if !showing {
                    last = None;
                    continue;
                }
                let Ok(text) = app.clipboard().read_text() else { continue };
                let hash = {
                    use std::hash::{Hash, Hasher};
                    let mut h = std::collections::hash_map::DefaultHasher::new();
                    text.hash(&mut h);
                    h.finish()
                };
                // The first read after the overlay appears only sets the baseline.
                let first = last.is_none();
                if last.replace(hash) == Some(hash) || first || text.len() > MAX_ITEM_TEXT {
                    continue;
                }
                if let Some(kind) = classify(&text) {
                    log::info!("overlay: {kind} copied from the game");
                    let s = settings(&app);
                    if (kind == "map" && s.map_check) || (kind == "item" && s.item_check) {
                        let _ = app.emit_to(LABEL, "overlay:item", serde_json::json!({ "raw": text, "kind": kind }));
                    }
                }
            }
        })
        .expect("spawn overlay clipboard watcher");
}

pub fn setup(app: &AppHandle) {
    app.manage(OverlayState::default());
    spawn_clipboard_watch(app.clone());
    let s = settings(app);
    if !s.enabled {
        return;
    }
    if let Err(e) = register_keys(app, &s) {
        log::warn!("overlay shortcuts: {e}");
    }
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        if let Err(e) = open_window(&app).await {
            log::warn!("overlay: {e}");
        }
    });
}

#[tauri::command]
pub fn overlay_settings_get(app: AppHandle) -> OverlaySettings {
    settings(&app)
}

/// Saves the settings and applies them: shortcuts, the window and every overlay's view of them.
#[tauri::command]
pub async fn overlay_settings_set(app: AppHandle, value: OverlaySettings) -> Result<OverlaySettings, String> {
    let mut value = value;
    value.width = value.width.clamp(260.0, 900.0);
    value.height = value.height.clamp(160.0, 1400.0);
    value.opacity = value.opacity.clamp(0.3, 1.0);
    value.card_seconds = value.card_seconds.clamp(3, 120);
    if value.enabled {
        parse_key(&value.toggle_key)?;
        parse_key(&value.interact_key)?;
    }
    store(&app, value.clone())?;
    register_keys(&app, &value)?;
    if value.enabled {
        open_window(&app).await?;
        if let Some(w) = app.get_webview_window(LABEL) {
            let _ = w.set_size(tauri::LogicalSize::new(value.width, value.height));
        }
    } else {
        close(&app);
    }
    let _ = app.emit("overlay:settings", &value);
    Ok(value)
}

#[tauri::command]
pub async fn overlay_toggle(app: AppHandle) -> Result<(), String> {
    toggle_window(&app).await
}

#[tauri::command]
pub fn overlay_interact(app: AppHandle, on: Option<bool>) {
    set_interact(&app, on);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_game_copies_are_read() {
        assert_eq!(classify("Item Class: Rings\nRarity: Rare\nOverlay Loop\nIron Ring"), Some("item"));
        assert_eq!(classify("\r\nItem Class: Maps\r\nRarity: Rare\r\nStrand Map"), Some("map"));
        assert_eq!(classify("Item Class: Waystones\nRarity: Magic"), Some("map"));
        assert_eq!(classify("Rarity: Rare\nno class line"), None);
        assert_eq!(classify("my password is Item Class: Rings"), None);
        assert_eq!(classify(""), None);
    }

    #[test]
    fn default_shortcuts_parse_and_differ() {
        let s = OverlaySettings::default();
        assert_ne!(parse_key(&s.toggle_key).unwrap(), parse_key(&s.interact_key).unwrap());
        assert!(parse_key("Ctrl+Nope+").is_err());
    }

    #[test]
    fn old_settings_files_get_defaults() {
        let s: OverlaySettings = serde_json::from_str(r#"{ "enabled": true }"#).unwrap();
        assert!(s.enabled && s.item_check);
        assert_eq!(s.toggle_key, "Alt+Shift+O");
    }
}
