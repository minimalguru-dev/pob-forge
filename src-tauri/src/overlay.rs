//! The in-game overlay: a small always-on-top window that lets clicks through to the game until
//! the interact shortcut is pressed. It reads only what the user copies (Ctrl+C on an item) and,
//! through `gamelog`, the game's own Client.txt. It never reads game memory.
//!
//! It sends input in two cases only, each on one key press of the user's while the game is the
//! foreground window: a macro types one chat command (Enter, the command, Enter), such as F5 for
//! /hideout; and the price check key (Ctrl+D) taps Ctrl+C once so the game copies the item under
//! the cursor. These keys are taken only while the game is in front.
//!
//! A card opens next to the cursor and closes as soon as the mouse moves, unless Ctrl is held.
//! With Ctrl held the card takes clicks, and a click keeps it open (pinned) until the user clicks
//! elsewhere or presses Esc. Ctrl is read with `GetAsyncKeyState`, which only reports whether the
//! key is down.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicIsize, AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::Serialize;

pub use crate::game::OverlaySettings;
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_clipboard_manager::ClipboardExt;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

pub const LABEL: &str = "overlay";
/// Short, so a card follows Ctrl+C at once; a clipboard read is cheap.
const POLL: Duration = Duration::from_millis(100);
/// How long the game gets to put the item on the clipboard after the price check's Ctrl+C.
const COPY_WAIT: Duration = Duration::from_millis(1000);
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
    /// Macro keys and their commands; taken only while the game is in front.
    macros: Mutex<Vec<(Shortcut, String)>>,
    macros_taken: AtomicBool,
    /// The price check key; taken with the macros while the game is in front.
    price_key: Mutex<Option<Shortcut>>,
    /// The game window the last price check came from, to give focus back to.
    game_window: AtomicIsize,
    /// The price check's own copy, which the clipboard watcher must not also show as an item card.
    suppress: AtomicU64,
}

fn text_hash(text: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    text.hash(&mut h);
    h.finish()
}

/// What a macro may type: one chat command, starting with a slash, on one line.
pub fn valid_command(command: &str) -> Result<String, String> {
    let c = command.trim();
    if !c.starts_with('/') || c.len() < 2 {
        return Err(format!("a macro types a chat command, which starts with /: {c}"));
    }
    if c.contains(['\n', '\r']) || c.chars().count() > 120 {
        return Err(format!("a macro command is one line of at most 120 characters: {c}"));
    }
    Ok(c.to_string())
}

/// The game window (by class, then title) is the foreground window.
fn game_in_front() -> bool {
    crate::game_window::foreground_game().is_some()
}

#[cfg(windows)]
fn modifiers_down() -> bool {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_CONTROL, VK_MENU, VK_SHIFT};
    // SAFETY: GetAsyncKeyState only reads key state.
    [VK_CONTROL, VK_SHIFT, VK_MENU].into_iter().any(|vk| (unsafe { GetAsyncKeyState(i32::from(vk)) } as u16 & 0x8000) != 0)
}

/// Types one chat command into the game: Enter opens the chat, the command, Enter sends it.
/// Held modifiers would turn the typing into shortcuts, so it waits briefly for them to be let go.
#[cfg(windows)]
fn type_command(command: &str) {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, KEYEVENTF_UNICODE, VK_RETURN};
    for _ in 0..20 {
        if !modifiers_down() {
            break;
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    if modifiers_down() || !game_in_front() {
        log::info!("overlay: macro skipped, a modifier is held or the game is not in front");
        return;
    }
    let key = |vk: u16, scan: u16, flags: u32| INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 { ki: KEYBDINPUT { wVk: vk, wScan: scan, dwFlags: flags, time: 0, dwExtraInfo: 0 } },
    };
    let send = |inputs: &[INPUT]| {
        // SAFETY: a plain array of keyboard inputs.
        unsafe { SendInput(inputs.len() as u32, inputs.as_ptr(), std::mem::size_of::<INPUT>() as i32) };
    };
    let enter = [key(VK_RETURN, 0, 0), key(VK_RETURN, 0, KEYEVENTF_KEYUP)];
    send(&enter);
    // The chat box needs a moment to open before it takes text.
    std::thread::sleep(Duration::from_millis(40));
    let text: Vec<INPUT> = command.encode_utf16().flat_map(|u| [key(0, u, KEYEVENTF_UNICODE), key(0, u, KEYEVENTF_UNICODE | KEYEVENTF_KEYUP)]).collect();
    send(&text);
    std::thread::sleep(Duration::from_millis(20));
    send(&enter);
}

#[cfg(not(windows))]
fn type_command(_: &str) {}

/// Takes the macro keys while the game is in front and gives them back otherwise, so F5 and the
/// like keep working in every other program.
fn spawn_macro_keys(app: AppHandle) {
    std::thread::Builder::new()
        .name("overlay-macros".into())
        .spawn(move || loop {
            std::thread::sleep(Duration::from_millis(200));
            let state = app.state::<OverlayState>();
            let mut keys: Vec<Shortcut> = state.macros.lock().unwrap_or_else(|e| e.into_inner()).iter().map(|(k, _)| *k).collect();
            keys.extend(*state.price_key.lock().unwrap_or_else(|e| e.into_inner()));
            if keys.is_empty() {
                continue;
            }
            let want = game_in_front();
            if want == state.macros_taken.load(Ordering::Relaxed) {
                continue;
            }
            let mut ok = true;
            for key in &keys {
                let done = if want { app.global_shortcut().register(*key) } else { app.global_shortcut().unregister(*key) };
                if let Err(e) = done {
                    ok = false;
                    log::warn!("overlay macro key {key}: {e}");
                }
            }
            if ok || !want {
                state.macros_taken.store(want, Ordering::Relaxed);
            }
        })
        .expect("spawn overlay macros");
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
    /// Ctrl was still held `HOLD_AFTER` after the card opened: letting go of it closes the card.
    held: bool,
    /// A price card: it takes clicks whenever the cursor is on it, Ctrl or not.
    trade: bool,
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
    /// The card is centred on `y` rather than hung from it: fixed beside the inventory.
    middle: bool,
}

/// How far, in logical pixels, the cursor may drift without Ctrl before the card closes. A hand
/// letting go of Ctrl+C moves the mouse a little; that must not close the card before it is read.
const DRIFT: f64 = 40.0;
/// After a card opens, the mouse may move this long before drift counts.
const GRACE: Duration = Duration::from_millis(800);
const FOLLOW_POLL: Duration = Duration::from_millis(25);
/// Ctrl still down this long after a card opens means the user holds it to look, as in Awakened
/// PoE Trade; a quick Ctrl+C has let go by then and gets the drift rule instead.
const HOLD_AFTER: Duration = Duration::from_millis(300);
/// How far the overlay draws a card from its anchor (OverlayWindow.svelte).
const CARD_GAP: f64 = 28.0;

fn ctrl_down() -> bool {
    crate::game_window::ctrl_held().unwrap_or(false)
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
    let mut anchor = Anchor { x: (c.x - p.x as f64) / scale, y: (c.y - p.y as f64) / scale, left: c.x > p.x as f64 + s.width as f64 / 2.0, middle: false };
    // Copied in the game: the card goes just left of the inventory panel, so it covers neither it
    // nor the stash. The overlay draws a left card CARD_GAP px left of its anchor.
    if let Some(rect) = crate::game_window::foreground_game().and_then(crate::game_window::rect) {
        let edge = crate::game_window::inventory_left(rect);
        // Fixed, as in Awakened PoE Trade: whatever item was copied, the card sits in the same place.
        let middle = (rect.1 + rect.3) as f64 / 2.0;
        anchor = Anchor { x: (edge as f64 - p.x as f64) / scale - 8.0 + CARD_GAP, y: (middle - p.y as f64) / scale, left: true, middle: true };
    }
    *follow(app) = Follow { origin: Some((c.x, c.y)), pinned: false, clickable: false, opened: Some(std::time::Instant::now()), held: false, trade: false, scale, card: (0.0, 0.0, 0.0, 0.0) };
    if !app.state::<OverlayState>().interact.load(Ordering::Relaxed) {
        let _ = w.set_ignore_cursor_events(true);
    }
    Some(anchor)
}

/// Closes the card: the window lets clicks through again and the overlay clears it.
fn dismiss(app: &AppHandle) {
    *follow(app) = Follow::default();
    let game = app.state::<OverlayState>().game_window.swap(0, Ordering::Relaxed);
    if let Some(w) = app.get_webview_window(LABEL) {
        if !app.state::<OverlayState>().interact.load(Ordering::Relaxed) {
            let _ = w.set_ignore_cursor_events(true);
            // A price card the user clicked into had focus; the game gets it back.
            if game != 0 && w.is_focused().unwrap_or(false) {
                crate::game_window::focus(game);
            }
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
            let clickable = (ctrl || state.trade) && inside;
            if clickable != state.clickable {
                let _ = w.set_ignore_cursor_events(!clickable);
                follow(&app).clickable = clickable;
            }
            let age = state.opened.map(|t| t.elapsed()).unwrap_or_default();
            if ctrl && age >= HOLD_AFTER && !state.held {
                follow(&app).held = true;
            }
            if state.held && !ctrl && !inside {
                // Ctrl let go away from the card: done looking.
                dismiss(&app);
            } else if ctrl || inside || age < GRACE {
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
    // Each lock is let go before the plugin is called: it registers on the main thread, where the
    // shortcut handler takes these same locks.
    let old = state.keys.lock().unwrap_or_else(|e| e.into_inner()).take();
    if let Some((a, b)) = old {
        let _ = app.global_shortcut().unregister(a);
        let _ = app.global_shortcut().unregister(b);
    }
    let mut taken: Vec<Shortcut> = std::mem::take(&mut *state.macros.lock().unwrap_or_else(|e| e.into_inner())).into_iter().map(|(k, _)| k).collect();
    taken.extend(state.price_key.lock().unwrap_or_else(|e| e.into_inner()).take());
    if state.macros_taken.swap(false, Ordering::Relaxed) {
        for key in taken {
            let _ = app.global_shortcut().unregister(key);
        }
    }
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
    let mut macros: Vec<(Shortcut, String)> = Vec::new();
    for m in &s.macros {
        if m.key.trim().is_empty() || m.command.trim().is_empty() {
            continue;
        }
        let key = parse_key(m.key.trim())?;
        if key == toggle || key == interact || Some(key) == price || macros.iter().any(|(k, _)| *k == key) {
            return Err(format!("the macro key {} is already used", m.key.trim()));
        }
        macros.push((key, valid_command(&m.command)?));
    }
    app.global_shortcut()
        .register(toggle)
        .map_err(|e| format!("could not take {toggle}: {e}. Another program may already use it."))?;
    if let Err(e) = app.global_shortcut().register(interact) {
        let _ = app.global_shortcut().unregister(toggle);
        return Err(format!("could not take {interact}: {e}. Another program may already use it."));
    }
    *state.keys.lock().unwrap_or_else(|e| e.into_inner()) = Some((toggle, interact));
    *state.macros.lock().unwrap_or_else(|e| e.into_inner()) = macros;
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
    let command = app.state::<OverlayState>().macros.lock().unwrap_or_else(|e| e.into_inner()).iter().find(|(k, _)| k == shortcut).map(|(_, c)| c.clone());
    if let Some(command) = command {
        // Off the shortcut handler: typing waits for modifiers and the chat box.
        std::thread::spawn(move || type_command(&command));
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

/// The price check key was pressed in the game: have the game copy the item under the cursor,
/// then open the price card beside the inventory.
async fn price_check(app: &AppHandle) -> Result<(), String> {
    let Some(game) = crate::game_window::foreground_game() else { return Ok(()) };
    let state = app.state::<OverlayState>();
    let before = app.clipboard().read_text().ok();
    if !crate::game_window::send_copy() {
        return Err("Windows refused the Ctrl+C for the price check.".into());
    }
    log::info!("overlay: price check sent Ctrl+C to the game");
    let started = std::time::Instant::now();
    let mut copied = None;
    while started.elapsed() < COPY_WAIT {
        tokio::time::sleep(Duration::from_millis(30)).await;
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
    let anchor = place_near_cursor(app);
    follow(app).trade = true;
    state.game_window.store(game, Ordering::Relaxed);
    let _ = app.emit_to(LABEL, "overlay:price", serde_json::json!({ "raw": text, "anchor": anchor }));
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
    spawn_macro_keys(app.clone());
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
        // A pinned card takes typing too: the price check's filters.
        let _ = w.set_focus();
    }
    let _ = app.emit_to(LABEL, "overlay:pinned", true);
}

#[tauri::command]
pub fn overlay_dismiss(app: AppHandle) {
    dismiss(&app);
}

/// The price card's own close button and Esc.
#[tauri::command]
pub fn overlay_price_done(app: AppHandle) {
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
    fn macros_type_one_chat_command() {
        assert_eq!(valid_command(" /hideout ").unwrap(), "/hideout");
        assert!(valid_command("hideout").is_err());
        assert!(valid_command("/").is_err());
        assert!(valid_command("/hideout\n/exit").is_err());
        let s = OverlaySettings::default();
        assert_eq!(s.macros.len(), 1);
        assert_eq!((s.macros[0].key.as_str(), s.macros[0].command.as_str()), ("F5", "/hideout"));
        assert_ne!(parse_key("F5").unwrap(), parse_key(&s.toggle_key).unwrap());
    }

    #[test]
    fn default_shortcuts_parse_and_differ() {
        let s = OverlaySettings::default();
        assert_ne!(parse_key(&s.toggle_key).unwrap(), parse_key(&s.interact_key).unwrap());
        let price = parse_key(&s.price_key).unwrap();
        assert!(price != parse_key(&s.toggle_key).unwrap() && price != parse_key(&s.interact_key).unwrap());
        assert!(s.macros.iter().all(|m| parse_key(&m.key).unwrap() != price));
        assert!(parse_key("Ctrl+Nope+").is_err());
    }

    #[test]
    fn old_settings_files_get_defaults() {
        let s: OverlaySettings = serde_json::from_str(r#"{ "enabled": true }"#).unwrap();
        assert!(s.enabled && s.item_check);
        assert_eq!(s.toggle_key, "Alt+Shift+O");
    }
}
