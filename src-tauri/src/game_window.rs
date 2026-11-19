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
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, VK_CONTROL};
    use windows_sys::Win32::UI::WindowsAndMessaging::{GetClassNameW, GetForegroundWindow, GetWindowTextW, SetForegroundWindow};

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

    fn key(vk: u16, up: bool) -> INPUT {
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 { ki: KEYBDINPUT { wVk: vk, wScan: 0, dwFlags: if up { KEYEVENTF_KEYUP } else { 0 }, time: 0, dwExtraInfo: 0 } },
        }
    }

    pub fn send_copy() -> bool {
        let inputs = [key(VK_CONTROL, false), key(VK_C, false), key(VK_C, true), key(VK_CONTROL, true)];
        // SAFETY: the array outlives the call and its length and element size are passed.
        let sent = unsafe { SendInput(inputs.len() as u32, inputs.as_ptr(), std::mem::size_of::<INPUT>() as i32) };
        sent as usize == inputs.len()
    }
}

#[cfg(not(windows))]
mod imp {
    use super::Handle;

    pub fn foreground_game() -> Option<Handle> {
        None
    }

    pub fn focus(_: Handle) {}

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
}
