//! Shared application state and the logic behind every hotkey, reachable from the main
//! loop *and* from platform keyboard hooks (which are plain C callbacks and therefore go
//! through the global `SHARED`).

use std::collections::VecDeque;
use std::sync::Mutex;

use crate::config::Config;
use crate::hotkey::Binding;
use crate::store::{Item, Store};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HotAction {
    PasteNewest,
    PasteOldest,
    Clear,
    Peek,
}

/// Work the hook could not do itself, handed to the main loop.
pub enum Pending {
    /// Press the paste chord (after putting `Some(item)` on the clipboard).
    SyntheticPaste(Option<Item>),
    Clear,
    Peek,
}

/// What a keyboard hook should do with the keystroke it just saw.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)] // `Pass` is only produced by the macOS / Windows hooks
pub enum HookDecision {
    /// Let it through untouched.
    Pass,
    /// Drop it.
    Swallow,
    /// macOS only: deliver it to the app as a plain ⌘V (clipboard already swapped).
    RewriteToPaste,
}

pub struct Bindings {
    pub newest: Option<Binding>,
    pub oldest: Option<Binding>,
    pub clear: Option<Binding>,
    pub peek: Option<Binding>,
}

impl Bindings {
    pub fn from_config(c: &Config) -> Self {
        Self {
            newest: Binding::parse(&c.hotkeys.paste_newest),
            oldest: Binding::parse(&c.hotkeys.paste_oldest),
            clear: Binding::parse(&c.hotkeys.clear),
            peek: Binding::parse(&c.hotkeys.peek),
        }
    }

    pub fn get(&self, a: HotAction) -> Option<&Binding> {
        match a {
            HotAction::PasteNewest => self.newest.as_ref(),
            HotAction::PasteOldest => self.oldest.as_ref(),
            HotAction::Clear => self.clear.as_ref(),
            HotAction::Peek => self.peek.as_ref(),
        }
    }
}

pub struct Shared {
    pub config: Config,
    pub store: Store,
    pub bindings: Bindings,
    pub pending: VecDeque<Pending>,
    /// Last clipboard text we know about (either seen by the poller or written by us),
    /// so our own writes are not recorded as new copies.
    pub last_seen: Option<String>,
    /// The tray menu must be rebuilt.
    pub menu_dirty: bool,
}

pub static SHARED: Mutex<Option<Shared>> = Mutex::new(None);

impl Shared {
    pub fn new() -> Self {
        let config = Config::load();
        let store = Store::new(config.max_items, config.max_history, config.persist_history);
        let bindings = Bindings::from_config(&config);
        Shared {
            config,
            store,
            bindings,
            pending: VecDeque::new(),
            last_seen: None,
            menu_dirty: true,
        }
    }

    pub fn reload_config(&mut self) {
        self.config = Config::load();
        self.bindings = Bindings::from_config(&self.config);
        self.store.max_stack = self.config.max_items;
        self.store.max_history = self.config.max_history;
        self.store.persist = self.config.persist_history;
        self.menu_dirty = true;
    }

    /// Writes an item (text plus its HTML flavour, if any) to the system clipboard
    /// without it being recorded as a new copy.
    pub fn set_clipboard_quietly(&mut self, item: &Item) {
        let result = arboard::Clipboard::new().and_then(|mut cb| match &item.html {
            Some(html) => cb.set_html(html.clone(), Some(item.text.clone())),
            None => cb.set_text(item.text.clone()),
        });
        match result {
            Ok(()) => self.last_seen = Some(item.text.clone()),
            Err(e) => log::warn!("could not write clipboard: {e}"),
        }
    }

    /// A hotkey fired (from a hook or a global hotkey). `can_rewrite` is true when the
    /// caller can hand the same keystroke to the app as a plain paste — then the app
    /// itself pastes and nothing synthetic is needed.
    pub fn on_hot_action(&mut self, action: HotAction, autorepeat: bool, can_rewrite: bool) -> HookDecision {
        match action {
            HotAction::PasteNewest | HotAction::PasteOldest => {
                if autorepeat {
                    return HookDecision::Swallow; // holding the key must not drain the stack
                }
                let popped = if action == HotAction::PasteNewest {
                    self.store.pop_newest()
                } else {
                    self.store.pop_oldest()
                };
                self.menu_dirty = true;
                match popped {
                    Some(item) => {
                        log::info!(
                            "popped {} chars{}, stack={}",
                            item.text.chars().count(),
                            if item.is_rich() { " (rich)" } else { "" },
                            self.store.stack_count()
                        );
                        if can_rewrite {
                            self.set_clipboard_quietly(&item);
                            HookDecision::RewriteToPaste
                        } else {
                            self.pending.push_back(Pending::SyntheticPaste(Some(item)));
                            HookDecision::Swallow
                        }
                    }
                    None => {
                        log::info!("stack empty -> normal paste");
                        if can_rewrite {
                            HookDecision::RewriteToPaste
                        } else {
                            self.pending.push_back(Pending::SyntheticPaste(None));
                            HookDecision::Swallow
                        }
                    }
                }
            }
            HotAction::Clear => {
                if !autorepeat {
                    self.pending.push_back(Pending::Clear);
                }
                HookDecision::Swallow
            }
            HotAction::Peek => {
                if !autorepeat {
                    self.pending.push_back(Pending::Peek);
                }
                HookDecision::Swallow
            }
        }
    }
}

/// Helper for hooks: look up the shared state without blocking (a hook must never wait).
#[allow(dead_code)] // no keyboard hook on Linux
pub fn with_shared<R>(f: impl FnOnce(&mut Shared) -> R) -> Option<R> {
    let mut guard = SHARED.try_lock().ok()?;
    guard.as_mut().map(f)
}
