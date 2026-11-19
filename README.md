# ClipStack

**A clipboard stack for your menubar / tray.** Copy, copy, copy — then one hotkey pastes
them back one at a time. Newest first (LIFO) or in the order you copied (FIFO), your call.

```
copy A · copy B · copy C          →  stack: [A, B, C]      menubar: 📋3
⌘⌃V  ⌘⌃V  ⌘⌃V                      →  pastes C, B, A       (or A, B, C in FIFO mode)
```

- **Copy and paste work exactly as before.** ClipStack only adds a stack on the side.
- **One hotkey pops and pastes** — macOS ⌘⌃V, Windows/Linux Ctrl+Alt+V. Bindable to plain ⌘V / Ctrl+V.
- **History** keeps what you've popped (15 by default, circulates). **★ Star** anything to keep it forever.
- **Formatting survives**: rich text (fonts, bold, colours, links) is captured and pasted back as-is.
- **Private**: text only, stays on your machine, password-manager copies are never recorded.
- Native, tiny (≈1.5 MB), no Electron, no account.

Two implementations live here:

| | Where | Status |
|---|---|---|
| **macOS native app** (Swift, AppKit) | repo root | daily-driven, signed, custom menu rows with ☆ buttons |
| **Cross-platform app** (Rust) — macOS, Windows, Linux | [`cross/`](cross/README.md) | builds on all three; Windows/Linux awaiting first real-machine test |

Both read the same config and history files, so on a Mac either one can be used.

---

## Install

### Windows, Linux, macOS — via npm

```bash
npx clipstack install
```

Copies the binary for your platform to a stable location, registers it to start at login,
launches it. `npx clipstack uninstall` reverses it. See [`cross/README.md`](cross/README.md)
for platform details (Wayland users: read the note there).

> Not published yet — see [PUBLISHING.md](PUBLISHING.md).

### macOS native app — from source

Needs Xcode Command Line Tools (`xcode-select --install`).

```bash
./make-app.sh --install
```

Builds `build/ClipStack.app`, copies it to `/Applications` and launches it. First launch asks
for **Accessibility** (System Settings → Privacy & Security → Accessibility) — that's what
lets it paste for you. Add to Login Items to start at boot.

The script signs with your Developer ID / Apple Development certificate if you have one,
which keeps the Accessibility grant across rebuilds. Unsigned builds get re-asked after
every rebuild.

---

## Using it

| Key (macOS / Win+Linux) | Does |
|---|---|
| ⌘C / Ctrl+C | normal copy — *also* pushes onto the stack |
| ⌘V / Ctrl+V | normal paste, unchanged |
| **⌘⌃V** / **Ctrl+Alt+V** | pop and paste (newest first, or oldest first in FIFO mode) |
| ⌘⌃⇧V / Ctrl+Alt+Shift+V | pop from the *other* end |
| ⌘⌃⌫ / Ctrl+Alt+Backspace | clear the stack |
| ⌘⌃L / Ctrl+Alt+L | open the menu |

With an empty stack the paste hotkey behaves as a normal paste.

### The menu (📋)

- **Paste order** — one click switches LIFO ↔ FIFO (saved to config).
- **Stack** — not yet pasted. **★ Starred** — kept forever. **History** — circulates.
- macOS app: click a row to copy it · **☆** button stars it · ⌥-click pastes · ⌘-click removes.
- Cross-platform app: click a row to copy it; Paste / Star / Remove submenus.
- *Clear stack* keeps history; *Clear history* keeps starred items.

### Config

`~/.config/clipstack/config.json` (macOS/Linux) · `%APPDATA%\clipstack\config.json` (Windows).
Created on first run; **Edit config…** opens it, **Reload config** applies it.

```json
{
  "maxItems": 5,
  "maxHistory": 15,
  "pasteOrder": "lifo",
  "persistHistory": true,
  "pollInterval": 0.2,
  "showCountInMenubar": true,
  "hotkeys": {
    "pasteNewest": "cmd+ctrl+v",
    "pasteOldest": "cmd+ctrl+shift+v",
    "clear": "cmd+ctrl+backspace",
    "peek": "cmd+ctrl+l"
  }
}
```

| Key | Meaning |
|---|---|
| `maxItems` | stack size; oldest dropped when exceeded |
| `maxHistory` | history size; starred items don't count |
| `pasteOrder` | `"lifo"` newest first · `"fifo"` oldest first (for the main hotkey) |
| `persistHistory` | save history across restarts (starred always saved) |
| `hotkeys.*` | modifiers joined with `+`, then the key: `cmd`/`win`/`super`, `ctrl`, `alt`/`option`, `shift`; keys `a`–`z`, `0`–`9`, `f1`–`f12`, `space`, `return`, `tab`, `escape`, `backspace`, arrows, punctuation. `""` disables. |

You can bind `pasteNewest` to plain `cmd+v` / `ctrl+v`: it then pops when the stack has
items and pastes normally when it's empty (macOS and Windows; not on Linux).

---

## How the paste actually happens

The interesting part. A naïve clipboard tool grabs your hotkey and *sends a fake ⌘V*. That
breaks in two ways: a still-held Control key corrupts the fake keystroke, and binding plain
⌘V is impossible because the tool swallows it. ClipStack instead watches the keystroke on
its way to the app, swaps the clipboard, and lets **your own keystroke** continue as a
plain paste — the app in front does the pasting, so it works everywhere ⌘V works.

| Platform | Mechanism |
|---|---|
| macOS | CGEvent tap — rewrites ⌘⌃V into ⌘V in place |
| Windows | low-level keyboard hook — Ctrl+V passes through after the swap; other chords use a synthetic Ctrl+V sent once Alt/Shift are released |
| Linux X11 | global hotkey + synthetic Ctrl+V |
| Linux Wayland | no global hotkeys for apps; bind a desktop shortcut to `clipstack pop` |

---

## Layout

```
Sources/ClipStack/     macOS Swift app (AppDelegate, Store, KeyTap, Paster, RowView, Config)
make-app.sh            builds + signs + installs the macOS .app
cross/                 Rust app for macOS/Windows/Linux + npm wrapper + CI
.github/workflows/     release pipeline: binaries for 5 targets, npm publish on tag
PUBLISHING.md          how to release
```

## Licence

MIT
