//! macOS: a session-level CGEvent tap, the same mechanism as the Swift app.
//!
//! The tap sees every key-down on its way to the frontmost app. For a paste binding we
//! swap the clipboard and rewrite the very same keystroke into a plain ⌘V, so the app
//! performs the paste itself. Needs Accessibility.

use std::sync::atomic::{AtomicPtr, Ordering};

use core_foundation::runloop::{kCFRunLoopCommonModes, CFRunLoop};
use core_graphics::event::{
    CGEvent, CGEventFlags, CGEventTap, CGEventTapLocation, CGEventTapOptions,
    CGEventTapPlacement, CGEventType, EventField,
};
use global_hotkey::hotkey::Modifiers;

use crate::app::{with_shared, HookDecision};
use crate::hotkey::mac_keycode;

const KEY_V: i64 = 9;

static TAP_PORT: AtomicPtr<std::ffi::c_void> = AtomicPtr::new(std::ptr::null_mut());

#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn AXIsProcessTrusted() -> bool;
    fn AXIsProcessTrustedWithOptions(options: core_foundation::dictionary::CFDictionaryRef) -> bool;
    static kAXTrustedCheckOptionPrompt: core_foundation::string::CFStringRef;
}

#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGEventSourceFlagsState(state_id: i32) -> u64;
    fn CGEventTapEnable(tap: *mut std::ffi::c_void, enable: bool);
}

const COMBINED_SESSION_STATE: i32 = 0;
const FLAG_SHIFT: u64 = 0x0002_0000;
const FLAG_CONTROL: u64 = 0x0004_0000;
const FLAG_ALTERNATE: u64 = 0x0008_0000;
const FLAG_COMMAND: u64 = 0x0010_0000;

pub fn input_permission_granted() -> bool {
    unsafe { AXIsProcessTrusted() }
}

pub fn request_input_permission() {
    use core_foundation::base::TCFType;
    use core_foundation::boolean::CFBoolean;
    use core_foundation::dictionary::CFDictionary;
    use core_foundation::string::CFString;
    unsafe {
        let key = CFString::wrap_under_get_rule(kAXTrustedCheckOptionPrompt);
        let dict = CFDictionary::from_CFType_pairs(&[(key, CFBoolean::true_value())]);
        let _ = AXIsProcessTrustedWithOptions(dict.as_concrete_TypeRef());
    }
}

/// Modifiers that are not part of ⌘V and would corrupt a synthetic paste.
pub fn extra_modifiers_down() -> bool {
    let f = unsafe { CGEventSourceFlagsState(COMBINED_SESSION_STATE) };
    f & (FLAG_CONTROL | FLAG_ALTERNATE | FLAG_SHIFT) != 0
}

pub fn hook_can_rewrite() -> bool {
    true
}

fn modifiers_of(flags: CGEventFlags) -> Modifiers {
    let bits = flags.bits();
    let mut m = Modifiers::empty();
    if bits & FLAG_COMMAND != 0 { m |= Modifiers::META; }
    if bits & FLAG_CONTROL != 0 { m |= Modifiers::CONTROL; }
    if bits & FLAG_ALTERNATE != 0 { m |= Modifiers::ALT; }
    if bits & FLAG_SHIFT != 0 { m |= Modifiers::SHIFT; }
    m
}

pub fn install_hook() -> bool {
    if !TAP_PORT.load(Ordering::SeqCst).is_null() {
        return true;
    }
    if !input_permission_granted() {
        return false;
    }

    let tap = CGEventTap::new(
        CGEventTapLocation::Session,
        CGEventTapPlacement::HeadInsertEventTap,
        CGEventTapOptions::Default,
        vec![CGEventType::KeyDown],
        |_proxy, ty, event: &CGEvent| -> Option<CGEvent> {
            match ty {
                CGEventType::TapDisabledByTimeout | CGEventType::TapDisabledByUserInput => {
                    let port = TAP_PORT.load(Ordering::SeqCst);
                    if !port.is_null() {
                        unsafe { CGEventTapEnable(port, true) };
                    }
                    return Some(event.clone());
                }
                CGEventType::KeyDown => {}
                _ => return Some(event.clone()),
            }

            let keycode = event.get_integer_value_field(EventField::KEYBOARD_EVENT_KEYCODE);
            let autorepeat = event.get_integer_value_field(EventField::KEYBOARD_EVENT_AUTOREPEAT) != 0;
            let mods = modifiers_of(event.get_flags());

            let decision = with_shared(|s| {
                let action = [
                    crate::app::HotAction::PasteNewest,
                    crate::app::HotAction::PasteOldest,
                    crate::app::HotAction::Clear,
                    crate::app::HotAction::Peek,
                ]
                .into_iter()
                .find(|a| {
                    s.bindings.get(*a).map_or(false, |b| {
                        b.mods == mods && mac_keycode(b.code).map(i64::from) == Some(keycode)
                    })
                });
                match action {
                    None => HookDecision::Pass,
                    Some(a) => s.on_hot_action(a, autorepeat, true),
                }
            })
            .unwrap_or(HookDecision::Pass);

            match decision {
                HookDecision::Pass => Some(event.clone()),
                HookDecision::Swallow => None,
                HookDecision::RewriteToPaste => {
                    event.set_integer_value_field(EventField::KEYBOARD_EVENT_KEYCODE, KEY_V);
                    let keep = event.get_flags().bits()
                        & !(FLAG_COMMAND | FLAG_CONTROL | FLAG_ALTERNATE | FLAG_SHIFT);
                    event.set_flags(CGEventFlags::from_bits_truncate(keep | FLAG_COMMAND));
                    Some(event.clone())
                }
            }
        },
    );

    match tap {
        Ok(tap) => {
            use core_foundation::base::TCFType;
            let source = match tap.mach_port.create_runloop_source(0) {
                Ok(s) => s,
                Err(_) => {
                    log::warn!("could not create run loop source for event tap");
                    return false;
                }
            };
            unsafe {
                CFRunLoop::get_current().add_source(&source, kCFRunLoopCommonModes);
            }
            tap.enable();
            TAP_PORT.store(tap.mach_port.as_concrete_TypeRef() as *mut _, Ordering::SeqCst);
            // The tap (and the closure it owns) must live for the rest of the process.
            std::mem::forget(source);
            std::mem::forget(tap);
            true
        }
        Err(_) => {
            log::warn!("could not create event tap (Accessibility not granted?)");
            false
        }
    }
}
