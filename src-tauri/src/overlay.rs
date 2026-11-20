//! The in-game overlay: a small always-on-top window that lets clicks through to the game until
//! the interact shortcut is pressed. It reads only what the user copies (Ctrl+C on an item) and,
//! through `gamelog`, the game's own Client.txt. It never reads game memory. The one input it
//! sends is the price check's Ctrl+C, right after the user presses the price shortcut in the game.

use std::sync::atomic::{AtomicBool, AtomicIsize, AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::Duration;

pub use crate::game::OverlaySettings;
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_clipboard_manager::ClipboardExt;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

pub const LABEL: &str = "overlay";
/// Short enough that a card follows Ctrl+C at once; a clipboard read is cheap.
const POLL: Duration = Duration::from_millis(100);
/// How often a card held open with Ctrl checks the key and the cursor.
const HOLD_POLL: Duration = Duration::from_millis(30);
/// A card shown after Ctrl was already let go closes once the cursor moves this far, as in Awakened.
const MOVE_AWAY: f64 = 120.0;
/// Larger than any item or map the game copies; anything bigger is not ours to read.
const MAX_ITEM_TEXT: usize = 16 * 1024;

/// How often the cursor is checked against the close button while clicks pass through.
const HOVER_POLL: Duration = Duration::from_millis(40);

/// A rectangle in the overlay webview's logical pixels.
#[derive(serde::Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

impl Rect {
    fn contains(&self, x: f64, y: f64) -> bool {
        x >= self.x && x < self.x + self.width && y >= self.y && y < self.y + self.height
    }
}

#[derive(Default)]
pub struct OverlayState {
    interact: AtomicBool,
    keys: Mutex<Option<(Shortcut, Shortcut)>>,
    /// The close button, the one spot that takes clicks while the rest lets them through to the game.
    close_button: Mutex<Option<Rect>>,
    /// Whether the window currently lets clicks through.
    passthrough: AtomicBool,
    /// The price shortcut. It is taken only while the game has focus, so other programs keep it.
    price_key: Mutex<Option<Shortcut>>,
    price_taken: AtomicBool,
    /// The game window the last price check came from, to give focus back to.
    game_window: AtomicIsize,
    /// The price check's copy, which the clipboard watcher must not also show as an item card.
    suppress: AtomicU64,
    /// Where the overlay stood before a card moved it beside the inventory.
    home: Mutex<Option<PhysicalPosition<i32>>>,
    /// Bumped for each card, so a newer card's hold watcher replaces the older one.
    card_generation: AtomicU64,
}

/// Whether the cursor is over the overlay window.
fn over_overlay(app: &AppHandle) -> bool {
    let Some(w) = app.get_webview_window(LABEL) else { return false };
    let (Ok(cursor), Ok(pos), Ok(size)) = (w.cursor_position(), w.outer_position(), w.outer_size()) else { return false };
    let (x, y) = (cursor.x - pos.x as f64, cursor.y - pos.y as f64);
    x >= 0.0 && y >= 0.0 && x < size.width as f64 && y < size.height as f64
}

fn cursor(app: &AppHandle) -> Option<(f64, f64)> {
    let p = app.get_webview_window(LABEL)?.cursor_position().ok()?;
    Some((p.x, p.y))
}

/// Hold to look, as in Awakened PoE Trade: once Ctrl has been seen down, letting it go closes the
/// card; a card that appears after Ctrl was let go closes when the cursor moves away. Moving the
/// cursor onto the overlay pins it open until it is closed by hand.
fn spawn_hold_watch(app: &AppHandle, kind: &'static str) {
    if crate::game_window::ctrl_held().is_none() {
        return;
    }
    let generation = app.state::<OverlayState>().card_generation.fetch_add(1, Ordering::Relaxed) + 1;
    let app = app.clone();
    let _ = std::thread::Builder::new().name("overlay-hold".into()).spawn(move || {
        let start = cursor(&app);
        let mut held = false;
        loop {
            std::thread::sleep(HOLD_POLL);
            if app.state::<OverlayState>().card_generation.load(Ordering::Relaxed) != generation {
                return;
            }
            if over_overlay(&app) {
                return;
            }
            match crate::game_window::ctrl_held() {
                Some(true) => held = true,
                Some(false) if held => {
                    let _ = app.emit_to(LABEL, "overlay:dismiss", kind);
                    return;
                }
                Some(false) => {
                    let moved = match (start, cursor(&app)) {
                        (Some((x0, y0)), Some((x, y))) => ((x - x0).powi(2) + (y - y0).powi(2)).sqrt() > MOVE_AWAY,
                        _ => false,
                    };
                    if moved {
                        let _ = app.emit_to(LABEL, "overlay:dismiss", kind);
                        return;
                    }
                }
                None => return,
            }
        }
    });
}

/// Moves the overlay just left of the game's inventory panel for an item or price card,
/// remembering where it stood so `undock` can put it back.
fn dock_beside_inventory(app: &AppHandle, game: crate::game_window::Handle) {
    let (Some(w), Some(rect)) = (app.get_webview_window(LABEL), crate::game_window::rect(game)) else { return };
    let (Ok(pos), Ok(size)) = (w.outer_position(), w.outer_size()) else { return };
    let state = app.state::<OverlayState>();
    let mut home = state.home.lock().unwrap_or_else(|e| e.into_inner());
    if home.is_none() {
        *home = Some(pos);
    }
    let (x, y) = crate::game_window::beside_inventory(rect, (size.width, size.height));
    let _ = w.set_position(PhysicalPosition::new(x, y));
}

/// Puts the overlay back where it stood before a card moved it.
fn undock(app: &AppHandle) {
    let home = app.state::<OverlayState>().home.lock().unwrap_or_else(|e| e.into_inner()).take();
    if let (Some(home), Some(w)) = (home, app.get_webview_window(LABEL)) {
        let _ = w.set_position(home);
    }
}

/// How often the focused window is checked, to take or release the price shortcut.
const FOCUS_POLL: Duration = Duration::from_millis(200);
/// How long the game gets to put the item on the clipboard after the price check's Ctrl+C.
const COPY_WAIT: Duration = Duration::from_millis(1000);

fn text_hash(text: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    text.hash(&mut h);
    h.finish()
}

fn set_passthrough(app: &AppHandle, w: &tauri::WebviewWindow, on: bool) {
    let state = app.state::<OverlayState>();
    if state.passthrough.swap(on, Ordering::Relaxed) != on {
        let _ = w.set_ignore_cursor_events(on);
    }
}

/// Lets clicks reach the close button while the cursor is over it, and only then.
fn spawn_hover_watch(app: AppHandle) {
    std::thread::Builder::new()
        .name("overlay-hover".into())
        .spawn(move || loop {
            std::thread::sleep(HOVER_POLL);
            let Some(w) = app.get_webview_window(LABEL) else { continue };
            let state = app.state::<OverlayState>();
            if state.interact.load(Ordering::Relaxed) || !w.is_visible().unwrap_or(false) {
                continue;
            }
            let Some(rect) = *state.close_button.lock().unwrap_or_else(|e| e.into_inner()) else { continue };
            let (Ok(cursor), Ok(origin), Ok(scale)) = (w.cursor_position(), w.inner_position(), w.scale_factor()) else { continue };
            let over = rect.contains((cursor.x - origin.x as f64) / scale, (cursor.y - origin.y as f64) / scale);
            set_passthrough(&app, &w, !over);
        })
        .expect("spawn overlay hover watcher");
}

async fn hide_window(app: &AppHandle) -> Result<(), String> {
    set_interact(app, Some(false));
    match app.get_webview_window(LABEL) {
        Some(w) => w.hide().map_err(|e| e.to_string()),
        None => Ok(()),
    }
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

/// Takes the shortcuts the settings name, releasing the previous ones first. The plugin runs each
/// (un)registration on the main thread, so no lock of ours is held across those calls: the
/// shortcut handler runs on that same thread and would wait for it.
fn register_keys(app: &AppHandle, s: &OverlaySettings) -> Result<(), String> {
    let state = app.state::<OverlayState>();
    let old = state.keys.lock().unwrap_or_else(|e| e.into_inner()).take();
    if let Some((a, b)) = old {
        let _ = app.global_shortcut().unregister(a);
        let _ = app.global_shortcut().unregister(b);
    }
    release_price_key(app);
    if !s.enabled {
        return Ok(());
    }
    let (toggle, interact) = (parse_key(&s.toggle_key)?, parse_key(&s.interact_key)?);
    if toggle == interact {
        return Err("The two overlay shortcuts must differ.".into());
    }
    let price = if s.price_check { Some(parse_key(&s.price_key)?) } else { None };
    if price.is_some_and(|p| p == toggle || p == interact) {
        return Err("The price check shortcut must differ from the other two.".into());
    }
    app.global_shortcut()
        .register(toggle)
        .map_err(|e| format!("could not take {toggle}: {e}. Another program may already use it."))?;
    if let Err(e) = app.global_shortcut().register(interact) {
        let _ = app.global_shortcut().unregister(toggle);
        return Err(format!("could not take {interact}: {e}. Another program may already use it."));
    }
    *state.keys.lock().unwrap_or_else(|e| e.into_inner()) = Some((toggle, interact));
    // The focus watcher takes this one while the game has focus.
    *state.price_key.lock().unwrap_or_else(|e| e.into_inner()) = price;
    log::info!("overlay: shortcuts {toggle} and {interact}; price check {}", price.map_or("off".into(), |p| p.to_string()));
    Ok(())
}

pub fn on_shortcut(app: &AppHandle, shortcut: &Shortcut, state: ShortcutState) {
    if state != ShortcutState::Pressed {
        return;
    }
    let price = *app.state::<OverlayState>().price_key.lock().unwrap_or_else(|e| e.into_inner());
    if price.as_ref() == Some(shortcut) {
        let app = app.clone();
        tauri::async_runtime::spawn(async move {
            if let Err(e) = price_check(&app).await {
                log::warn!("overlay price check: {e}");
                let _ = app.emit_to(LABEL, "overlay:price-error", e);
            }
        });
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

fn release_price_key(app: &AppHandle) {
    let state = app.state::<OverlayState>();
    let key = state.price_key.lock().unwrap_or_else(|e| e.into_inner()).take();
    if state.price_taken.swap(false, Ordering::Relaxed) {
        if let Some(key) = key {
            let _ = app.global_shortcut().unregister(key);
        }
    }
}

/// Takes the price shortcut while the game has focus and gives it back to Windows otherwise,
/// so Ctrl+D keeps working in every other program.
fn spawn_focus_watch(app: AppHandle) {
    std::thread::Builder::new()
        .name("overlay-focus".into())
        .spawn(move || {
            let mut failed: Option<Shortcut> = None;
            loop {
                std::thread::sleep(FOCUS_POLL);
                let state = app.state::<OverlayState>();
                let Some(key) = *state.price_key.lock().unwrap_or_else(|e| e.into_inner()) else { continue };
                let want = crate::game_window::foreground_game().is_some();
                if want == state.price_taken.load(Ordering::Relaxed) {
                    continue;
                }
                if want {
                    match app.global_shortcut().register(key) {
                        Ok(()) => {
                            state.price_taken.store(true, Ordering::Relaxed);
                            failed = None;
                            log::info!("overlay: the game has focus; {key} prices the item under the cursor");
                        }
                        // Say so once per key, not five times a second.
                        Err(e) if failed != Some(key) => {
                            log::warn!("overlay: could not take {key} for the price check: {e}");
                            failed = Some(key);
                        }
                        Err(_) => {}
                    }
                } else {
                    let _ = app.global_shortcut().unregister(key);
                    state.price_taken.store(false, Ordering::Relaxed);
                }
            }
        })
        .expect("spawn overlay focus watcher");
}

/// The price shortcut was pressed in the game: have the game copy the item under the cursor,
/// then show the price check, clickable, until it is closed.
async fn price_check(app: &AppHandle) -> Result<(), String> {
    let Some(game) = crate::game_window::foreground_game() else { return Ok(()) };
    let state = app.state::<OverlayState>();
    state.game_window.store(game, Ordering::Relaxed);
    let before = app.clipboard().read_text().ok();
    if !crate::game_window::send_copy() {
        return Err("Windows refused the Ctrl+C for the price check.".into());
    }
    log::info!("overlay: price check sent Ctrl+C to the game");
    let started = std::time::Instant::now();
    let mut copied = None;
    while started.elapsed() < COPY_WAIT {
        tokio::time::sleep(Duration::from_millis(40)).await;
        if let Ok(text) = app.clipboard().read_text() {
            if before.as_ref() != Some(&text) {
                copied = Some(text);
                break;
            }
        }
    }
    if copied.is_none() {
        log::info!("overlay: the clipboard did not change within {} ms of the Ctrl+C", COPY_WAIT.as_millis());
    }
    // Checking the same item twice leaves the clipboard as it was.
    let text = copied.or(before).filter(|t| t.len() <= MAX_ITEM_TEXT && classify(t).is_some());
    let Some(text) = text else {
        return Err("Hover over an item in the game, then press the price check shortcut.".into());
    };
    state.suppress.store(text_hash(&text), Ordering::Relaxed);
    open_window(app).await?;
    dock_beside_inventory(app, game);
    let _ = app.emit_to(LABEL, "overlay:price", serde_json::json!({ "raw": text }));
    set_interact(app, Some(true));
    spawn_hold_watch(app, "price");
    Ok(())
}

async fn open_window(app: &AppHandle) -> Result<(), String> {
    if let Some(w) = app.get_webview_window(LABEL) {
        let _ = w.show();
        return Ok(());
    }
    let s = settings(app);
    let builder = WebviewWindowBuilder::new(app, LABEL, WebviewUrl::default())
        .title("PoB Forge Overlay")
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
    let state = app.state::<OverlayState>();
    state.interact.store(false, Ordering::Relaxed);
    state.passthrough.store(true, Ordering::Relaxed);
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
        Some(w) if w.is_visible().unwrap_or(false) => hide_window(app).await,
        _ => open_window(app).await,
    }
}

/// Interact mode makes the overlay clickable and movable; leaving it saves where it stands.
fn set_interact(app: &AppHandle, on: Option<bool>) {
    let Some(w) = app.get_webview_window(LABEL) else { return };
    let state = app.state::<OverlayState>();
    let on = on.unwrap_or(!state.interact.load(Ordering::Relaxed));
    let was = state.interact.swap(on, Ordering::Relaxed);
    set_passthrough(app, &w, !on);
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
                let hash = text_hash(&text);
                // The first read after the overlay appears only sets the baseline.
                let first = last.is_none();
                if last.replace(hash) == Some(hash) || first || text.len() > MAX_ITEM_TEXT {
                    continue;
                }
                // The price check's own copy already has its card.
                if app.state::<OverlayState>().suppress.swap(0, Ordering::Relaxed) == hash {
                    continue;
                }
                if let Some(kind) = classify(&text) {
                    log::info!("overlay: {kind} copied from the game");
                    let s = settings(&app);
                    if (kind == "map" && s.map_check) || (kind == "item" && s.item_check) {
                        // Copied in the game: the card goes beside the inventory, not over it.
                        let game = crate::game_window::foreground_game();
                        if let Some(game) = game {
                            dock_beside_inventory(&app, game);
                        }
                        let _ = app.emit_to(LABEL, "overlay:item", serde_json::json!({ "raw": text, "kind": kind }));
                        if game.is_some() {
                            spawn_hold_watch(&app, "item");
                        }
                    }
                }
            }
        })
        .expect("spawn overlay clipboard watcher");
}

pub fn setup(app: &AppHandle) {
    app.manage(OverlayState::default());
    spawn_clipboard_watch(app.clone());
    spawn_hover_watch(app.clone());
    spawn_focus_watch(app.clone());
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
        if value.price_check {
            parse_key(&value.price_key)?;
        }
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

/// Hides the overlay (its close button). The show/hide shortcut brings it back.
#[tauri::command]
pub async fn overlay_hide(app: AppHandle) -> Result<(), String> {
    hide_window(&app).await
}

/// Closes the price check: clicks go back to the game, and so does focus.
#[tauri::command]
pub fn overlay_price_done(app: AppHandle) {
    // A hold watcher still running for this card has nothing left to close.
    app.state::<OverlayState>().card_generation.fetch_add(1, Ordering::Relaxed);
    // Back home first: leaving interact mode saves wherever the overlay stands.
    undock(&app);
    set_interact(&app, Some(false));
    let game = app.state::<OverlayState>().game_window.swap(0, Ordering::Relaxed);
    if game != 0 {
        crate::game_window::focus(game);
    }
}

/// The item card timed out: the overlay goes back where it stood, unless the user is moving it.
#[tauri::command]
pub fn overlay_card_closed(app: AppHandle) {
    if !app.state::<OverlayState>().interact.load(Ordering::Relaxed) {
        undock(&app);
    }
}

/// Where the overlay drew its close button, so clicks there are not passed to the game.
#[tauri::command]
pub fn overlay_close_button(app: AppHandle, rect: Option<Rect>) {
    *app.state::<OverlayState>().close_button.lock().unwrap_or_else(|e| e.into_inner()) = rect;
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
    fn close_button_hit_test() {
        let r = Rect { x: 350.0, y: 4.0, width: 24.0, height: 24.0 };
        assert!(r.contains(350.0, 4.0) && r.contains(373.9, 27.9));
        assert!(!r.contains(374.0, 10.0) && !r.contains(349.9, 10.0) && !r.contains(360.0, 28.0));
    }

    #[test]
    fn default_shortcuts_parse_and_differ() {
        let s = OverlaySettings::default();
        assert_ne!(parse_key(&s.toggle_key).unwrap(), parse_key(&s.interact_key).unwrap());
        let price = parse_key(&s.price_key).unwrap();
        assert!(price != parse_key(&s.toggle_key).unwrap() && price != parse_key(&s.interact_key).unwrap());
        assert!(parse_key("Ctrl+Nope+").is_err());
    }

    #[test]
    fn old_settings_files_get_defaults() {
        let s: OverlaySettings = serde_json::from_str(r#"{ "enabled": true }"#).unwrap();
        assert!(s.enabled && s.item_check);
        assert!(!s.leveling && !s.progression, "log panels are opt-in");
        assert_eq!(s.toggle_key, "Alt+Shift+O");
    }
}
