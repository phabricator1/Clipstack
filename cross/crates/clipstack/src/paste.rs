//! Synthetic paste: puts text on the clipboard and presses the platform's paste chord.
//!
//! Used when the keystroke could not be rewritten in place (Linux always; Windows and
//! macOS for menu-triggered pastes, or before the hook is available). Pastes are queued
//! and only sent once the user has released every modifier that is *not* part of the
//! paste chord, otherwise the app would see e.g. Ctrl+Alt+V instead of Ctrl+V.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use enigo::{Direction, Enigo, Key, Keyboard, Settings};

use crate::store::Item;

const MODIFIER_WAIT_LIMIT: Duration = Duration::from_secs(2);
const GAP_BETWEEN_PASTES: Duration = Duration::from_millis(80);

/// `None` means "paste whatever is on the clipboard" (used when the stack is empty).
pub struct Pending(pub Option<Item>);

pub struct PasteEngine {
    queue: VecDeque<Pending>,
    waiting_since: Option<Instant>,
    last_sent: Option<Instant>,
    enigo: Option<Enigo>,
}

impl PasteEngine {
    pub fn new() -> Self {
        Self {
            queue: VecDeque::new(),
            waiting_since: None,
            last_sent: None,
            enigo: None,
        }
    }

    pub fn enqueue(&mut self, item: Option<Item>) {
        self.queue.push_back(Pending(item));
        if self.waiting_since.is_none() {
            self.waiting_since = Some(Instant::now());
        }
    }

    /// Call frequently (every ~20–50 ms). `set_clipboard` is invoked right before the
    /// chord is sent so queued pastes each deliver their own item.
    pub fn tick(&mut self, set_clipboard: &mut dyn FnMut(&Item)) {
        if self.queue.is_empty() {
            self.waiting_since = None;
            return;
        }
        if let Some(last) = self.last_sent {
            if last.elapsed() < GAP_BETWEEN_PASTES {
                return;
            }
        }
        let started = *self.waiting_since.get_or_insert_with(Instant::now);
        if crate::platform::extra_modifiers_down() && started.elapsed() < MODIFIER_WAIT_LIMIT {
            return;
        }

        let Pending(item) = self.queue.pop_front().unwrap();
        if let Some(it) = &item {
            set_clipboard(it);
        }
        log::info!(
            "sending paste chord (waited {:.0} ms, extra modifiers down: {})",
            started.elapsed().as_millis(),
            crate::platform::extra_modifiers_down()
        );
        self.send_chord();
        self.last_sent = Some(Instant::now());
        self.waiting_since = if self.queue.is_empty() { None } else { Some(Instant::now()) };
    }

    fn send_chord(&mut self) {
        if self.enigo.is_none() {
            match Enigo::new(&Settings::default()) {
                Ok(e) => self.enigo = Some(e),
                Err(e) => {
                    log::warn!("cannot create input source: {e}");
                    return;
                }
            }
        }
        let Some(enigo) = self.enigo.as_mut() else { return };

        #[cfg(target_os = "macos")]
        let (modifier, v) = (Key::Meta, Key::Other(9)); // kVK_ANSI_V
        #[cfg(windows)]
        let (modifier, v) = (Key::Control, Key::Other(0x56)); // VK_V
        #[cfg(all(unix, not(target_os = "macos")))]
        let (modifier, v) = (Key::Control, Key::Unicode('v'));

        let _ = enigo.key(modifier, Direction::Press);
        let _ = enigo.key(v, Direction::Click);
        let _ = enigo.key(modifier, Direction::Release);
    }
}
