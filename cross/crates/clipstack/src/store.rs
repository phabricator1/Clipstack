//! History + stack. Mirrors the macOS app's `history.json` so both can share a file format.
//!
//! - `history`: every text copied, newest first, up to `max_history` (starred never evicted)
//! - `stack_ids`: ids not yet popped, oldest first (top of stack = last)
//!
//! Popping removes an id from the stack but leaves the item in history.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Item {
    pub id: Uuid,
    pub text: String,
    /// Unix seconds.
    pub copied_at: f64,
    pub pinned: bool,
    /// HTML flavour captured with the copy, written back on paste so formatting
    /// survives. The macOS Swift app additionally stores `rtf`; unknown fields are
    /// ignored here and dropped on save.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub html: Option<String>,
    /// Any other fields (e.g. the Swift app's `rtf`) are carried through untouched so the
    /// two apps can share one history file without losing each other's data.
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

impl Item {
    pub fn is_rich(&self) -> bool {
        self.html.is_some()
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Snapshot {
    history: Vec<Item>,
    #[serde(rename = "stackIDs")]
    stack_ids: Vec<Uuid>,
}

pub struct Store {
    pub history: Vec<Item>,
    pub stack_ids: Vec<Uuid>,
    pub max_stack: usize,
    pub max_history: usize,
    pub persist: bool,
    dirty: bool,
}

fn now() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0)
}

impl Store {
    pub fn file() -> PathBuf {
        crate::config::Config::dir().join("history.json")
    }

    pub fn new(max_stack: usize, max_history: usize, persist: bool) -> Self {
        let mut s = Store {
            history: Vec::new(),
            stack_ids: Vec::new(),
            max_stack,
            max_history,
            persist,
            dirty: false,
        };
        s.load();
        s
    }

    // ----- queries -------------------------------------------------------

    pub fn stack_count(&self) -> usize {
        self.stack_ids.len()
    }

    /// Items still on the stack, top (newest) first.
    pub fn stack_items(&self) -> Vec<&Item> {
        self.stack_ids
            .iter()
            .rev()
            .filter_map(|id| self.item(id))
            .collect()
    }

    pub fn starred(&self) -> Vec<&Item> {
        self.history.iter().filter(|i| i.pinned).collect()
    }

    pub fn unstarred_history(&self) -> Vec<&Item> {
        self.history.iter().filter(|i| !i.pinned).collect()
    }

    pub fn item(&self, id: &Uuid) -> Option<&Item> {
        self.history.iter().find(|i| &i.id == id)
    }

    pub fn is_on_stack(&self, id: &Uuid) -> bool {
        self.stack_ids.contains(id)
    }

    pub fn top_text(&self) -> Option<&str> {
        self.stack_ids
            .last()
            .and_then(|id| self.item(id))
            .map(|i| i.text.as_str())
    }

    // ----- mutations -----------------------------------------------------

    /// Records a new copy: adds to history (or moves an identical text to the top)
    /// and pushes it onto the stack.
    pub fn push(&mut self, text: &str, html: Option<String>) -> Uuid {
        let item = if let Some(pos) = self.history.iter().position(|i| i.text == text) {
            let mut existing = self.history.remove(pos);
            existing.copied_at = now();
            if html.is_some() {
                existing.html = html;
            }
            existing
        } else {
            Item {
                id: Uuid::new_v4(),
                text: text.to_string(),
                copied_at: now(),
                pinned: false,
                html,
                extra: Default::default(),
            }
        };
        let id = item.id;
        self.history.insert(0, item);
        self.stack_ids.retain(|x| x != &id);
        self.stack_ids.push(id);
        self.trim();
        self.mark_dirty();
        id
    }

    pub fn pop_newest(&mut self) -> Option<Item> {
        let id = self.stack_ids.pop()?;
        self.mark_dirty();
        self.item(&id).cloned()
    }

    pub fn pop_oldest(&mut self) -> Option<Item> {
        if self.stack_ids.is_empty() {
            return None;
        }
        let id = self.stack_ids.remove(0);
        self.mark_dirty();
        self.item(&id).cloned()
    }

    pub fn pop_id(&mut self, id: &Uuid) -> Option<Item> {
        let pos = self.stack_ids.iter().position(|x| x == id)?;
        self.stack_ids.remove(pos);
        self.mark_dirty();
        self.item(id).cloned()
    }

    pub fn toggle_pin(&mut self, id: &Uuid) {
        if let Some(i) = self.history.iter_mut().find(|i| &i.id == id) {
            i.pinned = !i.pinned;
            self.mark_dirty();
        }
    }

    pub fn remove(&mut self, id: &Uuid) {
        self.history.retain(|i| &i.id != id);
        self.stack_ids.retain(|x| x != id);
        self.mark_dirty();
    }

    pub fn clear_stack(&mut self) {
        self.stack_ids.clear();
        self.mark_dirty();
    }

    /// Drops all unstarred history (and therefore the whole stack).
    pub fn clear_history(&mut self) {
        self.history.retain(|i| i.pinned);
        self.stack_ids.clear();
        self.mark_dirty();
    }

    fn trim(&mut self) {
        if self.stack_ids.len() > self.max_stack {
            let excess = self.stack_ids.len() - self.max_stack;
            self.stack_ids.drain(0..excess);
        }
        let unstarred = self.history.iter().filter(|i| !i.pinned).count();
        let mut over = unstarred.saturating_sub(self.max_history);
        if over == 0 {
            return;
        }
        // Evict oldest unstarred items not on the stack first, then those on the stack.
        for pass in 0..2 {
            let mut idx = self.history.len();
            while idx > 0 && over > 0 {
                idx -= 1;
                let it = &self.history[idx];
                if it.pinned {
                    continue;
                }
                if pass == 0 && self.is_on_stack(&it.id) {
                    continue;
                }
                let id = it.id;
                self.history.remove(idx);
                self.stack_ids.retain(|x| x != &id);
                over -= 1;
            }
        }
    }

    // ----- persistence ---------------------------------------------------

    fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    /// Called from the main loop; writes only when something changed.
    pub fn save_if_dirty(&mut self) {
        if !self.dirty {
            return;
        }
        self.dirty = false;
        let snap = if self.persist {
            Snapshot {
                history: self.history.clone(),
                stack_ids: self.stack_ids.clone(),
            }
        } else {
            Snapshot {
                history: self.history.iter().filter(|i| i.pinned).cloned().collect(),
                stack_ids: Vec::new(),
            }
        };
        let path = Self::file();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        match serde_json::to_string_pretty(&snap) {
            Ok(s) => {
                let tmp = path.with_extension("json.tmp");
                if std::fs::write(&tmp, s).and_then(|_| std::fs::rename(&tmp, &path)).is_err() {
                    log::warn!("could not save {}", path.display());
                }
            }
            Err(e) => log::warn!("could not serialise history: {e}"),
        }
    }

    fn load(&mut self) {
        let Ok(s) = std::fs::read_to_string(Self::file()) else { return };
        let Ok(snap) = serde_json::from_str::<Snapshot>(&s) else { return };
        if self.persist {
            self.history = snap.history;
            self.stack_ids = snap
                .stack_ids
                .into_iter()
                .filter(|id| self.history.iter().any(|i| &i.id == id))
                .collect();
        } else {
            self.history = snap.history.into_iter().filter(|i| i.pinned).collect();
            self.stack_ids.clear();
        }
        self.trim();
    }
}
