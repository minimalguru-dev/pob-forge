//! The game's window on Windows: whether it has focus, handing focus back to it, and the one key
//! press the app ever sends it — Ctrl+C for a price check, which makes the game copy the item
//! under the cursor. Elsewhere these find no game and send nothing.

/// A window handle, kept as a number so it can sit in an atomic.
pub type Handle = isize;

/// The game's window, when it is the one with focus.
pub fn foreground_game() -> Option<Handle> {
    imp::foreground_game()
}

/// Gives focus back to the game, as the price check closes.
pub fn focus(window: Handle) {
    imp::focus(window)
}

/// The game window's screen rectangle: left, top, right, bottom in physical pixels.
pub fn rect(window: Handle) -> Option<(i32, i32, i32, i32)> {
    imp::rect(window)
}

/// The game's inventory (and stash) panel is 986/1600 of the window height wide, at any resolution.
const PANEL_PER_HEIGHT: f64 = 986.0 / 1600.0;
const GAP: i32 = 8;

/// Where a window of `size` sits just left of the inventory panel, vertically centred,
/// kept inside the game window.
pub fn beside_inventory((left, top, right, bottom): (i32, i32, i32, i32), (width, height): (u32, u32)) -> (i32, i32) {
    let panel = ((bottom - top) as f64 * PANEL_PER_HEIGHT).round() as i32;
    let x = (right - panel - GAP - width as i32).max(left);
    let y = top + ((bottom - top - height as i32) / 2).max(0);
    (x, y)
}

/// Whether Ctrl is held right now, wherever focus is. `None` where this cannot be told.
pub fn ctrl_held() -> Option<bool> {
    imp::ctrl_held()
}

/// Taps Ctrl+C. Only called right after the price shortcut, while the game has focus.
pub fn send_copy() -> bool {
    imp::send_copy()
}

/// The game names its window class this way in both Path of Exile 1 and 2; the title is the fallback.
#[cfg_attr(not(windows), allow(dead_code))]
pub(crate) fn is_game(class: &str, title: &str) -> bool {
    class == "POEWindowClass" || title == "Path of Exile" || title == "Path of Exile 2"
}

#[cfg(windows)]
mod imp {
    use super::Handle;
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, MapVirtualKeyW, SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, MAPVK_VK_TO_VSC, VK_CONTROL};
    use windows_sys::Win32::Foundation::RECT;
    use windows_sys::Win32::UI::WindowsAndMessaging::{GetClassNameW, GetForegroundWindow, GetWindowRect, GetWindowTextW, SetForegroundWindow};

    pub fn ctrl_held() -> Option<bool> {
        // SAFETY: a plain state query; the high bit means the key is down.
        Some(unsafe { GetAsyncKeyState(VK_CONTROL as i32) } as u16 & 0x8000 != 0)
    }

    pub fn rect(window: Handle) -> Option<(i32, i32, i32, i32)> {
        let mut r = RECT { left: 0, top: 0, right: 0, bottom: 0 };
        // SAFETY: `r` outlives the call; a stale handle only makes it fail.
        let ok = unsafe { GetWindowRect(window as _, &mut r) } != 0;
        (ok && r.right > r.left && r.bottom > r.top).then_some((r.left, r.top, r.right, r.bottom))
    }

    const VK_C: u16 = 0x43;

    pub fn foreground_game() -> Option<Handle> {
        // SAFETY: plain Win32 queries; the buffers outlive the calls and their lengths are passed.
        unsafe {
            let hwnd = GetForegroundWindow();
            if hwnd.is_null() {
                return None;
            }
            let mut class = [0u16; 64];
            let n = GetClassNameW(hwnd, class.as_mut_ptr(), class.len() as i32).max(0) as usize;
            let mut title = [0u16; 64];
            let t = GetWindowTextW(hwnd, title.as_mut_ptr(), title.len() as i32).max(0) as usize;
            super::is_game(&String::from_utf16_lossy(&class[..n]), &String::from_utf16_lossy(&title[..t])).then_some(hwnd as Handle)
        }
    }

    pub fn focus(window: Handle) {
        // SAFETY: a stale handle only makes the call fail.
        unsafe {
            SetForegroundWindow(window as _);
        }
    }

    /// The game reads keys by their hardware scan code, so each event carries one as well as the virtual key.
    fn key(vk: u16, up: bool) -> INPUT {
        // SAFETY: a pure table lookup.
        let scan = unsafe { MapVirtualKeyW(vk as u32, MAPVK_VK_TO_VSC) } as u16;
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 { ki: KEYBDINPUT { wVk: vk, wScan: scan, dwFlags: if up { KEYEVENTF_KEYUP } else { 0 }, time: 0, dwExtraInfo: 0 } },
        }
    }

    fn send(inputs: &[INPUT]) -> bool {
        // SAFETY: the slice outlives the call and its length and element size are passed.
        let sent = unsafe { SendInput(inputs.len() as u32, inputs.as_ptr(), std::mem::size_of::<INPUT>() as i32) };
        sent as usize == inputs.len()
    }

    /// Down, a short hold, then up: a game that reads keys once a frame still sees the press.
    /// When the user still holds Ctrl (from Ctrl+D) only C is sent: a sent Ctrl-up would make
    /// Windows report Ctrl as released while the finger is still on it, and the card holds on Ctrl.
    pub fn send_copy() -> bool {
        let ctrl = !ctrl_held().unwrap_or(false);
        let mut down = vec![key(VK_C, false)];
        let mut up = vec![key(VK_C, true)];
        if ctrl {
            down.insert(0, key(VK_CONTROL, false));
            up.push(key(VK_CONTROL, true));
        }
        let pressed = send(&down);
        std::thread::sleep(std::time::Duration::from_millis(30));
        pressed && send(&up)
    }
}

#[cfg(not(windows))]
mod imp {
    use super::Handle;

    pub fn foreground_game() -> Option<Handle> {
        None
    }

    pub fn focus(_: Handle) {}

    pub fn rect(_: Handle) -> Option<(i32, i32, i32, i32)> {
        None
    }

    pub fn ctrl_held() -> Option<bool> {
        None
    }

    pub fn send_copy() -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn recognises_the_game_window() {
        assert!(super::is_game("POEWindowClass", ""));
        assert!(super::is_game("", "Path of Exile 2"));
        assert!(!super::is_game("Chrome_WidgetWin_1", "Path of Exile wiki - Google Chrome"));
    }

    #[test]
    fn cards_sit_left_of_the_inventory() {
        // 2560x1440: the inventory panel is 887 px wide, so the card ends 8 px short of it.
        assert_eq!(super::beside_inventory((0, 0, 2560, 1440), (380, 620)), (1285, 410));
        // 1920x1080 (a 666 px panel) on a monitor to the left, with an overlay taller than the window.
        assert_eq!(super::beside_inventory((-1920, 0, 0, 1080), (380, 1200)), (-1054, 0));
        // A window too narrow for both stays inside the game.
        assert_eq!(super::beside_inventory((0, 0, 800, 1080), (380, 620)), (0, 230));
    }
}
