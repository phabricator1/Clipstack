//! ClipStack — a clipboard stack in your tray. Cross-platform (macOS, Windows, Linux).
//!
//! Main loop: a `tao` event loop ticking every 50 ms. Each tick polls the clipboard,
//! drains hotkey / menu events, runs queued pastes and refreshes the tray menu.

#![cfg_attr(windows, windows_subsystem = "windows")]

mod app;
mod config;
mod hotkey;
mod paste;
mod platform;
mod store;
mod tray;

use std::collections::HashMap;
use std::time::{Duration, Instant};

use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};
use tao::event::{Event, StartCause};
use tao::event_loop::{ControlFlow, EventLoopBuilder};
use tray_icon::menu::MenuEvent;
use tray_icon::TrayIcon;

use app::{HotAction, Pending, Shared, SHARED};

const TICK: Duration = Duration::from_millis(50);
/// Largest HTML payload (bytes) kept per item.
const RICH_FLAVOUR_LIMIT: usize = 2_000_000;

/// Everything that must stay on the main thread and is not shared with hooks.
struct Ctx {
    tray: Option<TrayIcon>,
    hotkeys: Option<GlobalHotKeyManager>,
    hotkey_ids: HashMap<u32, HotAction>,
    paste: paste::PasteEngine,
    using_hook: bool,
    last_poll: Instant,
    permission_prompted: bool,
}

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    // `clipstack pop` / `clipstack pop-oldest` / `clipstack clear`: one-shot commands for
    // desktop shortcuts on Wayland, where global hotkeys are unavailable. They talk to the
    // running instance through the history file; the instance picks the change up.
    if let Some(cmd) = std::env::args().nth(1) {
        std::process::exit(cli::run(&cmd));
    }

    *SHARED.lock().unwrap() = Some(Shared::new());

    let event_loop = EventLoopBuilder::new().build();
    let mut ctx = Ctx {
        tray: None,
        hotkeys: None,
        hotkey_ids: HashMap::new(),
        paste: paste::PasteEngine::new(),
        using_hook: false,
        last_poll: Instant::now(),
        permission_prompted: false,
    };

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::WaitUntil(Instant::now() + TICK);
        match event {
            Event::NewEvents(StartCause::Init) => ctx.init(),
            Event::NewEvents(_) | Event::MainEventsCleared => ctx.tick(),
            _ => {}
        }
        if ctx.quit_requested() {
            *control_flow = ControlFlow::Exit;
        }
    });
}

impl Ctx {
    fn init(&mut self) {
        let mut guard = SHARED.lock().unwrap();
        let shared = guard.as_mut().unwrap();
        match tray::build_tray(shared) {
            Ok(t) => self.tray = Some(t),
            Err(e) => log::error!("could not create tray icon: {e:#}"),
        }
        shared.menu_dirty = false;
        drop(guard);

        self.setup_hotkeys();
        if !platform::input_permission_granted() && !self.permission_prompted && prompt_allowed() {
            self.permission_prompted = true;
            platform::request_input_permission();
        }
    }

    /// Prefer the in-place keyboard hook; fall back to global hotkeys.
    fn setup_hotkeys(&mut self) {
        let mut guard = SHARED.lock().unwrap();
        let shared = guard.as_mut().unwrap();

        self.hotkeys = None;
        self.hotkey_ids.clear();

        self.using_hook = platform::install_hook();

        if !self.using_hook {
            match GlobalHotKeyManager::new() {
                Ok(m) => {
                    for action in [HotAction::PasteNewest, HotAction::PasteOldest, HotAction::Clear, HotAction::Peek] {
                        let Some(b) = shared.bindings.get(action) else { continue };
                        if b.is_plain_paste() {
                            log::warn!("{:?} is the plain paste chord; only honoured with the in-place hook", action);
                            continue;
                        }
                        let hk = b.to_hotkey();
                        match m.register(hk) {
                            Ok(()) => { self.hotkey_ids.insert(hk.id(), action); }
                            Err(e) => log::warn!("could not register {}: {e}", b.spec),
                        }
                    }
                    self.hotkeys = Some(m);
                }
                Err(e) => log::warn!("global hotkeys unavailable: {e}"),
            }
        }

        let b = &shared.bindings;
        log::info!(
            "started. mode={} newest={} oldest={} clear={} peek={} permission={} history={} stack={}",
            if self.using_hook { platform::hook_name() } else { "global hotkeys" },
            b.newest.as_ref().map_or("INVALID", |b| &b.display),
            b.oldest.as_ref().map_or("INVALID", |b| &b.display),
            b.clear.as_ref().map_or("INVALID", |b| &b.display),
            b.peek.as_ref().map_or("INVALID", |b| &b.display),
            if platform::input_permission_granted() { "granted" } else { "NOT granted" },
            shared.store.history.len(),
            shared.store.stack_count(),
        );
    }

    fn tick(&mut self) {
        // Upgrade to the hook once permission arrives (macOS).
        if !self.using_hook && platform::input_permission_granted() && platform::hook_can_rewrite() {
            self.setup_hotkeys();
        }

        self.poll_clipboard();
        self.drain_hotkey_events();
        self.drain_menu_events();
        self.drain_pending();

        let mut guard = SHARED.lock().unwrap();
        let shared = guard.as_mut().unwrap();

        self.paste.tick(&mut |item| shared.set_clipboard_quietly(item));

        if shared.menu_dirty {
            if let Some(t) = &self.tray {
                tray::refresh(t, shared);
            }
            shared.menu_dirty = false;
        }
        shared.store.save_if_dirty();
    }

    fn poll_clipboard(&mut self) {
        let interval = {
            let guard = SHARED.lock().unwrap();
            Duration::from_secs_f64(guard.as_ref().map_or(0.2, |s| s.config.poll_interval.max(0.05)))
        };
        if self.last_poll.elapsed() < interval {
            return;
        }
        self.last_poll = Instant::now();

        let Ok(mut cb) = arboard::Clipboard::new() else { return };
        let Ok(text) = cb.get_text() else { return }; // not text, or empty
        if text.is_empty() {
            return;
        }

        let mut guard = SHARED.lock().unwrap();
        let shared = guard.as_mut().unwrap();
        if shared.last_seen.as_deref() == Some(text.as_str()) {
            return;
        }
        shared.last_seen = Some(text.clone());
        if shared.store.top_text() == Some(text.as_str()) {
            return;
        }

        // Keep the HTML flavour too (capped so a huge web-page copy doesn't bloat history).
        let html = cb.get().html().ok().filter(|h| !h.is_empty() && h.len() <= RICH_FLAVOUR_LIMIT);
        let rich = html.is_some();
        shared.store.push(&text, html);
        shared.menu_dirty = true;
        log::info!(
            "pushed {} chars{}, stack={} history={}",
            text.chars().count(),
            if rich { " +html" } else { "" },
            shared.store.stack_count(),
            shared.store.history.len()
        );
    }

    fn drain_hotkey_events(&mut self) {
        while let Ok(ev) = GlobalHotKeyEvent::receiver().try_recv() {
            if ev.state != HotKeyState::Pressed {
                continue;
            }
            let Some(&action) = self.hotkey_ids.get(&ev.id) else { continue };
            let mut guard = SHARED.lock().unwrap();
            let shared = guard.as_mut().unwrap();
            // Global hotkeys cannot pass the keystroke through, so always synthesize.
            let _ = shared.on_hot_action(action, false, false);
        }
    }

    fn drain_menu_events(&mut self) {
        while let Ok(ev) = MenuEvent::receiver().try_recv() {
            let id = ev.id.0.as_str();
            let mut guard = SHARED.lock().unwrap();
            let shared = guard.as_mut().unwrap();
            let (cmd, arg) = id.split_once(':').unwrap_or((id, ""));
            let uuid = uuid::Uuid::parse_str(arg).ok();
            match (cmd, uuid) {
                ("copy", Some(u)) => {
                    if let Some(it) = shared.store.item(&u).cloned() {
                        shared.set_clipboard_quietly(&it);
                    }
                }
                ("paste", Some(u)) => {
                    if let Some(it) = shared.store.item(&u).cloned() {
                        if shared.store.is_on_stack(&u) {
                            let _ = shared.store.pop_id(&u);
                        }
                        shared.pending.push_back(Pending::SyntheticPaste(Some(it)));
                        shared.menu_dirty = true;
                    }
                }
                ("star", Some(u)) => {
                    shared.store.toggle_pin(&u);
                    shared.menu_dirty = true;
                }
                ("remove", Some(u)) => {
                    shared.store.remove(&u);
                    shared.menu_dirty = true;
                }
                ("clear_stack", _) => {
                    shared.store.clear_stack();
                    shared.menu_dirty = true;
                }
                ("clear_history", _) => {
                    shared.store.clear_history();
                    shared.menu_dirty = true;
                }
                ("edit_config", _) => {
                    let path = config::Config::file();
                    if !path.exists() {
                        shared.config.save();
                    }
                    open_path(&path);
                }
                ("reload_config", _) => {
                    shared.reload_config();
                    drop(guard);
                    self.setup_hotkeys();
                    continue;
                }
                ("grant_permission", _) => platform::request_input_permission(),
                ("quit", _) => {
                    shared.store.save_if_dirty();
                    QUIT.store(true, std::sync::atomic::Ordering::SeqCst);
                }
                _ => {}
            }
        }
    }

    fn drain_pending(&mut self) {
        let mut guard = SHARED.lock().unwrap();
        let shared = guard.as_mut().unwrap();
        while let Some(p) = shared.pending.pop_front() {
            match p {
                Pending::SyntheticPaste(item) => {
                    if platform::input_permission_granted() {
                        self.paste.enqueue(item);
                    } else {
                        // Leave it on the clipboard; the user can paste by hand.
                        if let Some(it) = item {
                            shared.set_clipboard_quietly(&it);
                        }
                        if !self.permission_prompted && prompt_allowed() {
                            self.permission_prompted = true;
                            platform::request_input_permission();
                        }
                    }
                }
                Pending::Clear => {
                    shared.store.clear_stack();
                    shared.menu_dirty = true;
                }
                Pending::Peek => {
                    // Tray menus cannot be opened programmatically on every platform;
                    // the hotkey is kept for parity and does nothing harmful here.
                    log::info!("peek requested (open the tray menu to see the stack)");
                }
            }
        }
    }

    fn quit_requested(&self) -> bool {
        QUIT.load(std::sync::atomic::Ordering::SeqCst)
    }
}

static QUIT: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Set CLIPSTACK_NO_PROMPT=1 to suppress the OS permission dialog (useful for smoke tests).
fn prompt_allowed() -> bool {
    std::env::var_os("CLIPSTACK_NO_PROMPT").is_none()
}

fn open_path(path: &std::path::Path) {
    #[cfg(target_os = "macos")]
    let _ = std::process::Command::new("open").arg(path).spawn();
    #[cfg(windows)]
    let _ = std::process::Command::new("cmd").args(["/C", "start", "", &path.display().to_string()]).spawn();
    #[cfg(all(unix, not(target_os = "macos")))]
    let _ = std::process::Command::new("xdg-open").arg(path).spawn();
}

/// One-shot subcommands for desktop shortcuts (mainly Linux/Wayland).
mod cli {
    use crate::config::Config;
    use crate::store::Store;

    pub fn run(cmd: &str) -> i32 {
        match cmd {
            "pop" | "pop-oldest" => {
                let c = Config::load();
                let mut store = Store::new(c.max_items, c.max_history, c.persist_history);
                let item = if cmd == "pop" { store.pop_newest() } else { store.pop_oldest() };
                store.save_if_dirty();
                match item {
                    Some(it) => {
                        let r = arboard::Clipboard::new().and_then(|mut cb| match it.html {
                            Some(html) => cb.set_html(html, Some(it.text)),
                            None => cb.set_text(it.text),
                        });
                        match r {
                            Ok(()) => 0,
                            Err(e) => { eprintln!("clipboard: {e}"); 1 }
                        }
                    }
                    None => { eprintln!("stack empty"); 0 }
                }
            }
            "clear" => {
                let c = Config::load();
                let mut store = Store::new(c.max_items, c.max_history, c.persist_history);
                store.clear_stack();
                store.save_if_dirty();
                0
            }
            "config-path" => { println!("{}", Config::file().display()); 0 }
            "--version" | "-V" => { println!("clipstack {}", env!("CARGO_PKG_VERSION")); 0 }
            _ => {
                eprintln!("usage: clipstack [pop | pop-oldest | clear | config-path | --version]\n       (no arguments starts the tray app)");
                2
            }
        }
    }
}
