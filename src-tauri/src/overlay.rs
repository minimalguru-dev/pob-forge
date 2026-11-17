//! The in-game overlay: a small always-on-top window that lets clicks through to the game until
//! the interact shortcut is pressed. It reads only what the user copies (Ctrl+C on an item) and,
//! through `gamelog`, the game's own Client.txt. It never reads game memory or sends input.
//!
//! A card opens next to the cursor and closes as soon as the mouse moves, unless Ctrl is held.
//! With Ctrl held the card takes clicks, and a click keeps it open (pinned) until the user clicks
//! elsewhere or presses Esc. Ctrl is read with `GetAsyncKeyState`, which only reports whether the
//! key is down.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::Serialize;

pub use crate::game::OverlaySettings;
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_clipboard_manager::ClipboardExt;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

pub const LABEL: &str = "overlay";
const POLL: Duration = Duration::from_millis(150);
/// Larger than any item or map the game copies; anything bigger is not ours to read.
const MAX_ITEM_TEXT: usize = 16 * 1024;
/// How many copied items the assistant can look back on (`get_game_context`).
const RECENT: usize = 5;

#[derive(Default)]
pub struct OverlayState {
    interact: AtomicBool,
    keys: Mutex<Option<(Shortcut, Shortcut)>>,
    /// The last items and maps copied while the overlay showed, newest first. Memory only.
    recent: Mutex<VecDeque<Copied>>,
    follow: Mutex<Follow>,
    /// The monitor size the window covers, which a repaint nudge returns to.
    full: Mutex<Option<tauri::PhysicalSize<u32>>>,
}

/// The open card's link to the cursor.
#[derive(Default, Clone, Copy)]
struct Follow {
    /// Where the cursor stood when the card opened, or last stood with Ctrl held. None: no card.
    origin: Option<(f64, f64)>,
    /// Ctrl+clicked: stays until a click elsewhere or Esc.
    pinned: bool,
    /// The window takes clicks now (Ctrl held over it).
    clickable: bool,
    /// When the card opened: the mouse may move freely for `GRACE` after it.
    opened: Option<std::time::Instant>,
    scale: f64,
    /// The card's rectangle in the window, logical pixels (x, y, width, height); only it takes clicks.
    card: (f64, f64, f64, f64),
}

/// Where the card goes: the cursor in window coordinates (logical pixels), and which side has room.
#[derive(Serialize, Clone, Copy)]
struct Anchor {
    x: f64,
    y: f64,
    left: bool,
}

/// How far, in logical pixels, the cursor may drift without Ctrl before the card closes. A hand
/// letting go of Ctrl+C moves the mouse a little; that must not close the card before it is read.
const DRIFT: f64 = 40.0;
/// After a card opens, the mouse may move this long before drift counts.
const GRACE: Duration = Duration::from_millis(800);
const FOLLOW_POLL: Duration = Duration::from_millis(25);

#[cfg(windows)]
fn ctrl_down() -> bool {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_CONTROL};
    // SAFETY: GetAsyncKeyState only reads the key's state.
    (unsafe { GetAsyncKeyState(VK_CONTROL as i32) } as u16 & 0x8000) != 0
}

#[cfg(not(windows))]
fn ctrl_down() -> bool {
    false
}

fn follow(app: &AppHandle) -> std::sync::MutexGuard<'_, Follow> {
    app.state::<OverlayState>().inner().follow.lock().unwrap_or_else(|e| e.into_inner())
}

/// The monitor a point is on.
fn monitor_at(w: &tauri::WebviewWindow, x: f64, y: f64) -> Option<tauri::Monitor> {
    w.available_monitors()
        .ok()
        .and_then(|ms| {
            ms.into_iter().find(|m| {
                let (p, s) = (m.position(), m.size());
                x >= p.x as f64 && x < (p.x + s.width as i32) as f64 && y >= p.y as f64 && y < (p.y + s.height as i32) as f64
            })
        })
        .or_else(|| w.primary_monitor().ok().flatten())
}

/// The window covers one whole monitor, transparent and click-through, like Awakened PoE Trade's.
/// It moves only to follow the cursor to another monitor: WebView2 stops painting an unfocused
/// window that has moved, so cards are placed inside it instead of moving it.
fn cover(app: &AppHandle, w: &tauri::WebviewWindow, m: &tauri::Monitor) {
    // macOS draws the overlay opaque, so there it stays a small window.
    if cfg!(target_os = "macos") {
        return;
    }
    // One pixel short of the monitor: Windows treats an exactly monitor-sized window as fullscreen,
    // and then the overlay stops drawing over a game in front of it.
    let (p, s) = (*m.position(), *m.size());
    let s = tauri::PhysicalSize::new(s.width, s.height.saturating_sub(1));
    *app.state::<OverlayState>().full.lock().unwrap_or_else(|e| e.into_inner()) = Some(s);
    if w.outer_position().ok() != Some(p) {
        let _ = w.set_position(p);
    }
    if w.inner_size().ok() != Some(s) {
        let _ = w.set_size(s);
    }
}

/// Anchors the card at the cursor, on the side of the screen with more room, and starts following it.
fn place_near_cursor(app: &AppHandle) -> Option<Anchor> {
    let w = app.get_webview_window(LABEL)?;
    let c = app.cursor_position().ok()?;
    let m = monitor_at(&w, c.x, c.y)?;
    cover(app, &w, &m);
    let (p, s, scale) = (m.position(), m.size(), m.scale_factor());
    let anchor = Anchor { x: (c.x - p.x as f64) / scale, y: (c.y - p.y as f64) / scale, left: c.x > p.x as f64 + s.width as f64 / 2.0 };
    *follow(app) = Follow { origin: Some((c.x, c.y)), pinned: false, clickable: false, opened: Some(std::time::Instant::now()), scale, card: (0.0, 0.0, 0.0, 0.0) };
    if !app.state::<OverlayState>().interact.load(Ordering::Relaxed) {
        let _ = w.set_ignore_cursor_events(true);
    }
    Some(anchor)
}

/// Closes the card: the window lets clicks through again and the overlay clears it.
fn dismiss(app: &AppHandle) {
    *follow(app) = Follow::default();
    if let Some(w) = app.get_webview_window(LABEL) {
        if !app.state::<OverlayState>().interact.load(Ordering::Relaxed) {
            let _ = w.set_ignore_cursor_events(true);
        }
    }
    let _ = app.emit_to(LABEL, "overlay:dismiss", ());
}

/// Watches the cursor and Ctrl while a card is open (the Awakened PoE Trade way).
fn spawn_follow(app: AppHandle) {
    std::thread::Builder::new()
        .name("overlay-follow".into())
        .spawn(move || loop {
            std::thread::sleep(FOLLOW_POLL);
            let state = *follow(&app);
            let Some(origin) = state.origin else { continue };
            if state.pinned || app.state::<OverlayState>().interact.load(Ordering::Relaxed) {
                continue;
            }
            let Some(w) = app.get_webview_window(LABEL) else { continue };
            let Ok(c) = app.cursor_position() else { continue };
            let ctrl = ctrl_down();
            let inside = match w.outer_position() {
                Ok(p) => {
                    let (x, y, cw, ch) = state.card;
                    let (left, top) = (p.x as f64 + x * state.scale, p.y as f64 + y * state.scale);
                    cw > 0.0 && c.x >= left && c.x < left + cw * state.scale && c.y >= top && c.y < top + ch * state.scale
                }
                _ => false,
            };
            let clickable = ctrl && inside;
            if clickable != state.clickable {
                let _ = w.set_ignore_cursor_events(!clickable);
                follow(&app).clickable = clickable;
            }
            let settling = state.opened.is_some_and(|t| t.elapsed() < GRACE);
            if ctrl || settling {
                follow(&app).origin = Some((c.x, c.y));
            } else if (c.x - origin.0).hypot(c.y - origin.1) > DRIFT * state.scale {
                dismiss(&app);
            }
        })
        .expect("spawn overlay follow");
}

/// An item or map the game or the trade site put on the clipboard while the overlay showed.
#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Copied {
    pub kind: &'static str,
    pub raw: String,
    /// Seconds since the Unix epoch.
    pub at: u64,
}

pub fn recent(app: &AppHandle) -> Vec<Copied> {
    app.try_state::<OverlayState>()
        .map(|s| s.recent.lock().unwrap_or_else(|e| e.into_inner()).iter().cloned().collect())
        .unwrap_or_default()
}

fn remember(app: &AppHandle, kind: &'static str, raw: &str) {
    let at = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let state = app.state::<OverlayState>();
    let mut recent = state.recent.lock().unwrap_or_else(|e| e.into_inner());
    recent.retain(|c| c.raw != raw);
    recent.push_front(Copied { kind, raw: raw.to_string(), at });
    recent.truncate(RECENT);
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
    let monitor = app.cursor_position().ok().and_then(|c| monitor_at(&window, c.x, c.y)).or_else(|| window.primary_monitor().ok().flatten());
    if let Some(m) = monitor {
        cover(app, &window, &m);
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

/// Interact mode makes the whole overlay clickable until the user is done with the card.
fn set_interact(app: &AppHandle, on: Option<bool>) {
    let Some(w) = app.get_webview_window(LABEL) else { return };
    let state = app.state::<OverlayState>();
    let on = on.unwrap_or(!state.interact.load(Ordering::Relaxed));
    state.interact.store(on, Ordering::Relaxed);
    let _ = w.set_ignore_cursor_events(!on);
    if on {
        let _ = w.set_focus();
    }
    let _ = app.emit_to(LABEL, "overlay:interact", on);
}

/// What kind of thing the game, or the trade site's Copy button, put on the clipboard, if it is one
/// at all. The game starts with `Item Class:`; text without it must start with a `Rarity:` line and
/// have the game's `--------` section breaks.
pub fn classify(text: &str) -> Option<&'static str> {
    let mut lines = text.trim_start().lines().map(str::trim);
    let first = lines.next()?;
    if let Some(class) = first.strip_prefix("Item Class:") {
        return Some(match class.trim() {
            "Maps" | "Waystones" => "map",
            _ => "item",
        });
    }
    const RARITIES: [&str; 8] = ["Normal", "Magic", "Rare", "Unique", "Gem", "Currency", "Divination Card", "Relic"];
    let rarity = first.strip_prefix("Rarity:")?.trim();
    if !RARITIES.contains(&rarity) {
        return None;
    }
    let rest: Vec<&str> = lines.collect();
    if !rest.contains(&"--------") {
        return None;
    }
    let map = rest.iter().any(|l| l.starts_with("Map Tier:") || l.starts_with("Waystone Tier:"));
    Some(if map { "map" } else { "item" })
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
                        remember(&app, kind, &text);
                        let anchor = place_near_cursor(&app);
                        let _ = app.emit_to(LABEL, "overlay:item", serde_json::json!({ "raw": text, "kind": kind, "anchor": anchor }));
                    }
                }
            }
        })
        .expect("spawn overlay clipboard watcher");
}

pub fn setup(app: &AppHandle) {
    app.manage(OverlayState::default());
    spawn_clipboard_watch(app.clone());
    spawn_follow(app.clone());
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

/// Ctrl+click on a card: it stays open, and its buttons work, until a click elsewhere or Esc.
#[tauri::command]
pub fn overlay_pin(app: AppHandle) {
    {
        let mut f = follow(&app);
        if f.origin.is_none() {
            return;
        }
        f.pinned = true;
        f.clickable = true;
    }
    if let Some(w) = app.get_webview_window(LABEL) {
        let _ = w.set_ignore_cursor_events(false);
    }
    let _ = app.emit_to(LABEL, "overlay:pinned", true);
}

#[tauri::command]
pub fn overlay_dismiss(app: AppHandle) {
    dismiss(&app);
}

/// The overlay shows something new: the card's rectangle in the window (logical pixels, zero size
/// for none) is the only part that takes Ctrl clicks, and the window is nudged so WebView2 paints
/// it without focus.
#[tauri::command]
pub fn overlay_fit(app: AppHandle, x: f64, y: f64, width: f64, height: f64) {
    follow(&app).card = (x, y, width.max(0.0), height.max(0.0));
    overlay_repaint(app);
}

/// WebView2 stops painting a click-through window that does not have focus, so a new card is
/// computed but stays invisible until the user presses the interact key. A 1 px resize and back
/// makes it draw a frame without taking focus from the game.
#[tauri::command]
pub fn overlay_repaint(app: AppHandle) {
    let Some(w) = app.get_webview_window(LABEL) else { return };
    // Back to the covered size, not the current one: nudges can overlap.
    let full = *app.state::<OverlayState>().full.lock().unwrap_or_else(|e| e.into_inner());
    let Some(size) = full.or_else(|| w.inner_size().ok()) else { return };
    let _ = w.set_size(tauri::PhysicalSize::new(size.width, size.height + 1));
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(40));
        let _ = w.set_size(size);
    });
}

/// "Ask the assistant" on a card: the chat's message box gets the text and its window comes forward.
/// Nothing is sent until the user presses Send.
#[tauri::command]
pub fn overlay_ask(app: AppHandle, text: String) {
    set_interact(&app, Some(false));
    dismiss(&app);
    let _ = app.emit("overlay:ask", text);
    let window = app.get_webview_window("assistant").or_else(|| app.get_webview_window("main"));
    if let Some(w) = window {
        let _ = w.unminimize();
        let _ = w.show();
        let _ = w.set_focus();
    }
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
        // The trade site's Copy button, which may leave out the class line.
        assert_eq!(classify("Rarity: Rare\nOverlay Loop\nIron Ring\n--------\nItem Level: 80\n--------\n+70 to maximum Life"), Some("item"));
        assert_eq!(classify("Rarity: Magic\nStrand Map of Fun\n--------\nMap Tier: 5\n--------\nArea is a large Maze"), Some("map"));
        assert_eq!(classify("Rarity: Mythic\nnot an item\n--------"), None);
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
