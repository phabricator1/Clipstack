//! Windows: a low-level keyboard hook (WH_KEYBOARD_LL).
//!
//! The hook sees every key-down before the focused app does. For a binding that *is*
//! Ctrl+V we swap the clipboard and let the keystroke through, so the app pastes the
//! popped item itself. For any other chord we swallow the keystroke and queue a synthetic
//! Ctrl+V (a LL hook can block a key but cannot change it into a different one).

use std::sync::atomic::{AtomicIsize, AtomicU32, Ordering};

use global_hotkey::hotkey::Modifiers;
use windows_sys::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, VK_CONTROL, VK_LWIN, VK_MENU, VK_RWIN, VK_SHIFT,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, SetWindowsHookExW, HC_ACTION, KBDLLHOOKSTRUCT, LLKHF_INJECTED, WH_KEYBOARD_LL,
    WM_KEYDOWN, WM_KEYUP, WM_SYSKEYDOWN, WM_SYSKEYUP,
};

use crate::app::{with_shared, HookDecision};
use crate::hotkey::win_vk;

static HOOK: AtomicIsize = AtomicIsize::new(0);
/// Virtual key currently held (0 = none), to detect auto-repeat: the LL hook gives no
/// repeat flag, but a second key-down without a key-up for the same key is one.
static HELD_VK: AtomicU32 = AtomicU32::new(0);

pub fn install_hook() -> bool {
    if HOOK.load(Ordering::SeqCst) != 0 {
        return true;
    }
    // Must be called on a thread that pumps messages; the tao event loop does.
    let h = unsafe { SetWindowsHookExW(WH_KEYBOARD_LL, Some(hook_proc), GetModuleHandleW(std::ptr::null()), 0) };
    HOOK.store(h as isize, Ordering::SeqCst);
    if h.is_null() {
        log::warn!("could not install keyboard hook; falling back to global hotkeys");
        false
    } else {
        true
    }
}

/// A LL hook can only pass or block a keystroke, never change it.
pub fn hook_can_rewrite() -> bool {
    false
}

fn key_down(vk: u16) -> bool {
    unsafe { (GetAsyncKeyState(vk as i32) as u16 & 0x8000) != 0 }
}

fn current_modifiers() -> Modifiers {
    let mut m = Modifiers::empty();
    if key_down(VK_CONTROL) { m |= Modifiers::CONTROL; }
    if key_down(VK_MENU) { m |= Modifiers::ALT; }
    if key_down(VK_SHIFT) { m |= Modifiers::SHIFT; }
    if key_down(VK_LWIN) || key_down(VK_RWIN) { m |= Modifiers::META; }
    m
}

/// Modifiers that are not part of Ctrl+V and would corrupt a synthetic paste.
pub fn extra_modifiers_down() -> bool {
    key_down(VK_MENU) || key_down(VK_SHIFT) || key_down(VK_LWIN) || key_down(VK_RWIN)
}

pub fn input_permission_granted() -> bool {
    true
}

pub fn request_input_permission() {}

unsafe extern "system" fn hook_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code == HC_ACTION as i32 {
        let kb = &*(lparam as *const KBDLLHOOKSTRUCT);
        let msg = wparam as u32;
        let injected = kb.flags & LLKHF_INJECTED != 0;

        if msg == WM_KEYUP || msg == WM_SYSKEYUP {
            if HELD_VK.load(Ordering::Relaxed) == kb.vkCode {
                HELD_VK.store(0, Ordering::Relaxed);
            }
        } else if (msg == WM_KEYDOWN || msg == WM_SYSKEYDOWN) && !injected {
            let autorepeat = HELD_VK.swap(kb.vkCode, Ordering::Relaxed) == kb.vkCode;
            let mods = current_modifiers();
            let vk = kb.vkCode;

            let decision = with_shared(|s| {
                let action = [
                    crate::app::HotAction::PasteNewest,
                    crate::app::HotAction::PasteOldest,
                    crate::app::HotAction::Clear,
                    crate::app::HotAction::Peek,
                ]
                .into_iter()
                .find(|a| {
                    s.bindings
                        .get(*a)
                        .map_or(false, |b| b.mods == mods && win_vk(b.code) == Some(vk))
                });
                match action {
                    None => HookDecision::Pass,
                    Some(a) => {
                        // Only a literal Ctrl+V binding can be passed through as the paste.
                        let plain = s.bindings.get(a).map_or(false, |b| b.is_plain_paste());
                        match s.on_hot_action(a, autorepeat, plain) {
                            HookDecision::RewriteToPaste => HookDecision::Pass,
                            other => other,
                        }
                    }
                }
            })
            .unwrap_or(HookDecision::Pass);

            if decision == HookDecision::Swallow {
                return 1;
            }
        }
    }
    CallNextHookEx(std::ptr::null_mut(), code, wparam, lparam)
}
