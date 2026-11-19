//! Linux: no in-place hook. X11 global hotkeys (via `global-hotkey`) plus a synthetic
//! Ctrl+V. On Wayland neither global hotkeys nor synthetic input are available to
//! ordinary apps; the tray menu and history still work, and the user can bind a desktop
//! shortcut to `clipstack pop` instead.

/// Try to install an in-place keyboard hook. Not available on Linux.
pub fn install_hook() -> bool {
    false
}

/// Whether the hook can rewrite arbitrary chords into a plain paste. N/A on Linux.
pub fn hook_can_rewrite() -> bool {
    false
}

/// We have no cheap way to read live modifier state on X11/Wayland from here, so the
/// paste engine just relies on its short delay.
pub fn extra_modifiers_down() -> bool {
    false
}

/// Permission needed before hotkeys/paste work. Nothing to prompt for on Linux.
pub fn input_permission_granted() -> bool {
    true
}

pub fn request_input_permission() {}
