//! Parses "cmd+ctrl+v"-style strings into a key `Code` + modifiers, and maps codes to
//! the raw key numbers each platform's keyboard hook sees.

use global_hotkey::hotkey::{Code, HotKey, Modifiers};

#[derive(Debug, Clone)]
pub struct Binding {
    pub spec: String,
    pub code: Code,
    pub mods: Modifiers,
    pub display: String,
}

impl Binding {
    pub fn parse(spec: &str) -> Option<Binding> {
        let spec = spec.trim();
        if spec.is_empty() {
            return None;
        }
        let parts: Vec<String> = spec
            .split('+')
            .map(|p| p.trim().to_lowercase())
            .filter(|p| !p.is_empty())
            .collect();
        let key = parts.last()?;
        let code = code_for(key)?;

        let mut mods = Modifiers::empty();
        for m in &parts[..parts.len() - 1] {
            mods |= match m.as_str() {
                "cmd" | "command" | "meta" | "super" | "win" | "windows" => Modifiers::META,
                "ctrl" | "control" => Modifiers::CONTROL,
                "alt" | "option" | "opt" => Modifiers::ALT,
                "shift" => Modifiers::SHIFT,
                _ => return None,
            };
        }

        let display = display_for(mods, key);
        Some(Binding {
            spec: spec.to_string(),
            code,
            mods,
            display,
        })
    }

    pub fn to_hotkey(&self) -> HotKey {
        HotKey::new(Some(self.mods), self.code)
    }

    /// The platform's own paste chord: ⌘V on macOS, Ctrl+V elsewhere. Safe to bind only
    /// where the in-place keyboard hook is available; as a global hotkey it would
    /// swallow every normal paste.
    pub fn is_plain_paste(&self) -> bool {
        self.code == Code::KeyV && self.mods == paste_modifier()
    }

}

/// The modifier of the native paste chord.
pub fn paste_modifier() -> Modifiers {
    if cfg!(target_os = "macos") {
        Modifiers::META
    } else {
        Modifiers::CONTROL
    }
}

fn display_for(mods: Modifiers, key: &str) -> String {
    let key_label = if key.chars().count() == 1 {
        key.to_uppercase()
    } else {
        let mut c = key.chars();
        match c.next() {
            Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
            None => String::new(),
        }
    };
    if cfg!(target_os = "macos") {
        let mut s = String::new();
        if mods.contains(Modifiers::CONTROL) { s.push('⌃'); }
        if mods.contains(Modifiers::ALT) { s.push('⌥'); }
        if mods.contains(Modifiers::SHIFT) { s.push('⇧'); }
        if mods.contains(Modifiers::META) { s.push('⌘'); }
        s + &key_label
    } else {
        let mut parts = Vec::new();
        if mods.contains(Modifiers::CONTROL) { parts.push("Ctrl"); }
        if mods.contains(Modifiers::ALT) { parts.push("Alt"); }
        if mods.contains(Modifiers::SHIFT) { parts.push("Shift"); }
        if mods.contains(Modifiers::META) { parts.push(if cfg!(windows) { "Win" } else { "Super" }); }
        parts.push(&key_label);
        parts.join("+")
    }
}

pub fn code_for(key: &str) -> Option<Code> {
    use Code::*;
    Some(match key {
        "a" => KeyA, "b" => KeyB, "c" => KeyC, "d" => KeyD, "e" => KeyE, "f" => KeyF,
        "g" => KeyG, "h" => KeyH, "i" => KeyI, "j" => KeyJ, "k" => KeyK, "l" => KeyL,
        "m" => KeyM, "n" => KeyN, "o" => KeyO, "p" => KeyP, "q" => KeyQ, "r" => KeyR,
        "s" => KeyS, "t" => KeyT, "u" => KeyU, "v" => KeyV, "w" => KeyW, "x" => KeyX,
        "y" => KeyY, "z" => KeyZ,
        "0" => Digit0, "1" => Digit1, "2" => Digit2, "3" => Digit3, "4" => Digit4,
        "5" => Digit5, "6" => Digit6, "7" => Digit7, "8" => Digit8, "9" => Digit9,
        "f1" => F1, "f2" => F2, "f3" => F3, "f4" => F4, "f5" => F5, "f6" => F6,
        "f7" => F7, "f8" => F8, "f9" => F9, "f10" => F10, "f11" => F11, "f12" => F12,
        "space" => Space, "return" | "enter" => Enter, "tab" => Tab,
        "escape" | "esc" => Escape, "backspace" | "delete" => Backspace,
        "forwarddelete" => Delete,
        "up" => ArrowUp, "down" => ArrowDown, "left" => ArrowLeft, "right" => ArrowRight,
        "-" => Minus, "=" => Equal, "[" => BracketLeft, "]" => BracketRight,
        ";" => Semicolon, "'" => Quote, "," => Comma, "." => Period, "/" => Slash,
        "\\" => Backslash, "`" => Backquote,
        _ => return None,
    })
}

/// macOS virtual key code for a `Code` (what a CGEvent tap reports).
#[cfg(target_os = "macos")]
pub fn mac_keycode(code: Code) -> Option<u16> {
    use Code::*;
    Some(match code {
        KeyA => 0, KeyS => 1, KeyD => 2, KeyF => 3, KeyH => 4, KeyG => 5, KeyZ => 6, KeyX => 7,
        KeyC => 8, KeyV => 9, KeyB => 11, KeyQ => 12, KeyW => 13, KeyE => 14, KeyR => 15,
        KeyY => 16, KeyT => 17, Digit1 => 18, Digit2 => 19, Digit3 => 20, Digit4 => 21,
        Digit6 => 22, Digit5 => 23, Equal => 24, Digit9 => 25, Digit7 => 26, Minus => 27,
        Digit8 => 28, Digit0 => 29, BracketRight => 30, KeyO => 31, KeyU => 32,
        BracketLeft => 33, KeyI => 34, KeyP => 35, Enter => 36, KeyL => 37, KeyJ => 38,
        Quote => 39, KeyK => 40, Semicolon => 41, Backslash => 42, Comma => 43, Slash => 44,
        KeyN => 45, KeyM => 46, Period => 47, Tab => 48, Space => 49, Backquote => 50,
        Backspace => 51, Escape => 53, Delete => 117,
        F1 => 122, F2 => 120, F3 => 99, F4 => 118, F5 => 96, F6 => 97, F7 => 98, F8 => 100,
        F9 => 101, F10 => 109, F11 => 103, F12 => 111,
        ArrowLeft => 123, ArrowRight => 124, ArrowDown => 125, ArrowUp => 126,
        _ => return None,
    })
}

/// Windows virtual-key code for a `Code` (what a WH_KEYBOARD_LL hook reports).
#[cfg(windows)]
pub fn win_vk(code: Code) -> Option<u32> {
    use Code::*;
    Some(match code {
        KeyA => 0x41, KeyB => 0x42, KeyC => 0x43, KeyD => 0x44, KeyE => 0x45, KeyF => 0x46,
        KeyG => 0x47, KeyH => 0x48, KeyI => 0x49, KeyJ => 0x4A, KeyK => 0x4B, KeyL => 0x4C,
        KeyM => 0x4D, KeyN => 0x4E, KeyO => 0x4F, KeyP => 0x50, KeyQ => 0x51, KeyR => 0x52,
        KeyS => 0x53, KeyT => 0x54, KeyU => 0x55, KeyV => 0x56, KeyW => 0x57, KeyX => 0x58,
        KeyY => 0x59, KeyZ => 0x5A,
        Digit0 => 0x30, Digit1 => 0x31, Digit2 => 0x32, Digit3 => 0x33, Digit4 => 0x34,
        Digit5 => 0x35, Digit6 => 0x36, Digit7 => 0x37, Digit8 => 0x38, Digit9 => 0x39,
        F1 => 0x70, F2 => 0x71, F3 => 0x72, F4 => 0x73, F5 => 0x74, F6 => 0x75, F7 => 0x76,
        F8 => 0x77, F9 => 0x78, F10 => 0x79, F11 => 0x7A, F12 => 0x7B,
        Space => 0x20, Enter => 0x0D, Tab => 0x09, Escape => 0x1B, Backspace => 0x08,
        Delete => 0x2E, ArrowLeft => 0x25, ArrowUp => 0x26, ArrowRight => 0x27, ArrowDown => 0x28,
        Minus => 0xBD, Equal => 0xBB, BracketLeft => 0xDB, BracketRight => 0xDD,
        Semicolon => 0xBA, Quote => 0xDE, Comma => 0xBC, Period => 0xBE, Slash => 0xBF,
        Backslash => 0xDC, Backquote => 0xC0,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_specs() {
        let b = Binding::parse("cmd+ctrl+v").unwrap();
        assert_eq!(b.code, Code::KeyV);
        assert_eq!(b.mods, Modifiers::META | Modifiers::CONTROL);
        assert!(Binding::parse("ctrl+alt+shift+f5").is_some());
        assert!(Binding::parse("hyper+v").is_none());
        assert!(Binding::parse("").is_none());
        assert!(Binding::parse("cmd+bogus").is_none());
    }
}
