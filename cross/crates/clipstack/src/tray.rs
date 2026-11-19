//! Tray icon and its menu. The menu is rebuilt from the store whenever state changes.

use tray_icon::menu::{Menu, MenuItem, PredefinedMenuItem, Submenu};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};

use crate::app::Shared;
use crate::store::Item;

const MAX_STACK_ROWS: usize = 15;
const MAX_HISTORY_ROWS: usize = 30;

pub fn build_tray(shared: &Shared) -> anyhow::Result<TrayIcon> {
    let menu = build_menu(shared);
    let mut builder = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_tooltip(tooltip(shared))
        .with_icon(icon());
    if cfg!(target_os = "macos") {
        builder = builder.with_title(title(shared));
    }
    Ok(builder.build()?)
}

pub fn refresh(tray: &TrayIcon, shared: &Shared) {
    tray.set_menu(Some(Box::new(build_menu(shared))));
    let _ = tray.set_tooltip(Some(tooltip(shared)));
    if cfg!(target_os = "macos") {
        tray.set_title(Some(title(shared)));
    }
}

fn title(shared: &Shared) -> String {
    let n = shared.store.stack_count();
    if shared.config.show_count_in_menubar && n > 0 {
        format!("📋{n}")
    } else {
        "📋".to_string()
    }
}

fn tooltip(shared: &Shared) -> String {
    let n = shared.store.stack_count();
    if n > 0 {
        format!("ClipStack — {n} on the stack")
    } else {
        "ClipStack — stack empty".to_string()
    }
}

/// Menu item ids encode the action: "copy:<uuid>", "star:<uuid>", "remove:<uuid>",
/// "paste:<uuid>", or a bare command name.
fn build_menu(shared: &Shared) -> Menu {
    let menu = Menu::new();
    let store = &shared.store;

    let stack = store.stack_items();
    let starred = store.starred();
    let history = store.unstarred_history();

    let newest = shared.bindings.newest.as_ref().map(|b| b.display.clone()).unwrap_or("—".into());
    let oldest = shared.bindings.oldest.as_ref().map(|b| b.display.clone()).unwrap_or("—".into());

    let fifo = shared.config.is_fifo();

    header(&menu, "click a row to copy it");
    let _ = menu.append(&MenuItem::with_id(
        "toggle_order",
        if fifo {
            "Paste order: oldest first (FIFO)  —  switch to newest first"
        } else {
            "Paste order: newest first (LIFO)  —  switch to oldest first"
        },
        true,
        None,
    ));
    sep(&menu);

    if stack.is_empty() {
        header(&menu, "STACK — empty, copy something");
    } else {
        header(&menu, &format!(
            "STACK ({} of {}) — {newest} pastes the {}",
            stack.len(),
            shared.config.max_items,
            if fifo { "oldest (bottom)" } else { "newest (top)" }
        ));
        for it in stack.iter().take(MAX_STACK_ROWS) {
            row(&menu, it);
        }
        if stack.len() > MAX_STACK_ROWS {
            header(&menu, &format!("… and {} more", stack.len() - MAX_STACK_ROWS));
        }
    }

    if !starred.is_empty() {
        sep(&menu);
        header(&menu, "★ STARRED");
        for it in &starred {
            row(&menu, it);
        }
    }

    sep(&menu);
    if history.is_empty() {
        header(&menu, "HISTORY — empty");
    } else {
        header(&menu, &format!("HISTORY ({} of {})", history.len(), shared.config.max_history));
        for it in history.iter().take(MAX_HISTORY_ROWS) {
            row(&menu, it);
        }
        if history.len() > MAX_HISTORY_ROWS {
            header(&menu, &format!("… and {} more", history.len() - MAX_HISTORY_ROWS));
        }
    }

    sep(&menu);

    // Actions that need a target item live in submenus (tray menus have no modifier-click).
    let all: Vec<&Item> = starred.iter().chain(history.iter()).copied().collect();

    let star_menu = Submenu::new("★ Star / unstar…", !all.is_empty());
    for it in &all {
        let label = format!("{} {}", if it.pinned { "★" } else { "☆" }, preview(&it.text));
        let _ = star_menu.append(&MenuItem::with_id(format!("star:{}", it.id), label, true, None));
    }
    let _ = menu.append(&star_menu);

    let paste_menu = Submenu::new("⤓ Paste now…", !all.is_empty());
    for it in &all {
        let _ = paste_menu.append(&MenuItem::with_id(format!("paste:{}", it.id), preview(&it.text), true, None));
    }
    let _ = menu.append(&paste_menu);

    let remove_menu = Submenu::new("✕ Remove…", !all.is_empty());
    for it in &all {
        let _ = remove_menu.append(&MenuItem::with_id(format!("remove:{}", it.id), preview(&it.text), true, None));
    }
    let _ = menu.append(&remove_menu);

    sep(&menu);
    let _ = menu.append(&MenuItem::with_id("clear_stack", "Clear stack (history stays)", !stack.is_empty(), None));
    let _ = menu.append(&MenuItem::with_id("clear_history", "Clear history (starred stay)", !history.is_empty(), None));

    sep(&menu);
    header(&menu, &if fifo {
        format!("Hotkeys: {newest} oldest first · {oldest} newest first")
    } else {
        format!("Hotkeys: {newest} newest first · {oldest} oldest first")
    });
    let _ = menu.append(&MenuItem::with_id("edit_config", "Edit config…", true, None));
    let _ = menu.append(&MenuItem::with_id("reload_config", "Reload config", true, None));
    if !crate::platform::input_permission_granted() {
        let _ = menu.append(&MenuItem::with_id("grant_permission", "⚠️ Grant Accessibility to enable pasting…", true, None));
    }

    sep(&menu);
    let _ = menu.append(&MenuItem::with_id("quit", "Quit ClipStack", true, None));
    menu
}

fn row(menu: &Menu, it: &Item) {
    let label = format!(
        "{}{}{}",
        if it.pinned { "★ " } else { "" },
        if it.is_rich() { "𝐀 " } else { "" },
        preview(&it.text)
    );
    let _ = menu.append(&MenuItem::with_id(format!("copy:{}", it.id), label, true, None));
}

fn header(menu: &Menu, text: &str) {
    let _ = menu.append(&MenuItem::new(text, false, None));
}

fn sep(menu: &Menu) {
    let _ = menu.append(&PredefinedMenuItem::separator());
}

pub fn preview(text: &str) -> String {
    let one_line: String = text.replace('\n', " ⏎ ").trim().to_string();
    let count = one_line.chars().count();
    if count > 60 {
        let cut: String = one_line.chars().take(60).collect();
        format!("{cut}…")
    } else {
        one_line
    }
}

/// A 32×32 clipboard glyph drawn in code, so no image assets are needed.
fn icon() -> Icon {
    const W: u32 = 32;
    const H: u32 = 32;
    let mut rgba = vec![0u8; (W * H * 4) as usize];
    let put = |rgba: &mut Vec<u8>, x: u32, y: u32, c: [u8; 4]| {
        let i = ((y * W + x) * 4) as usize;
        rgba[i..i + 4].copy_from_slice(&c);
    };
    let board = [60, 60, 60, 255];
    let paper = [245, 245, 245, 255];
    let clip = [200, 160, 60, 255];
    for y in 4..30 {
        for x in 5..27 {
            put(&mut rgba, x, y, board);
        }
    }
    for y in 8..27 {
        for x in 8..24 {
            put(&mut rgba, x, y, paper);
        }
    }
    for y in 2..8 {
        for x in 12..20 {
            put(&mut rgba, x, y, clip);
        }
    }
    for (i, y) in [12u32, 16, 20].iter().enumerate() {
        for x in 10..(22 - i as u32 * 2) {
            put(&mut rgba, x, *y, board);
        }
    }
    Icon::from_rgba(rgba, W, H).expect("static icon is valid")
}
