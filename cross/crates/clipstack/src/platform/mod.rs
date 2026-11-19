//! Platform-specific pieces: the in-place keyboard hook (macOS, Windows) and a check for
//! held modifier keys used by the synthetic-paste fallback.

#[cfg(target_os = "macos")]
mod macos;
#[cfg(windows)]
mod windows;
#[cfg(all(unix, not(target_os = "macos")))]
mod linux;

#[cfg(target_os = "macos")]
pub use macos::*;
#[cfg(windows)]
pub use windows::*;
#[cfg(all(unix, not(target_os = "macos")))]
pub use linux::*;

/// Human-readable name of the hotkey mechanism in use, for logs and the menu.
pub fn hook_name() -> &'static str {
    if cfg!(target_os = "macos") {
        "event tap"
    } else if cfg!(windows) {
        "keyboard hook"
    } else {
        "global hotkeys"
    }
}
