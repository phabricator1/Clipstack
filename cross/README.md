# ClipStack (cross-platform)

A clipboard **stack** in your menubar / tray, for macOS, Windows and Linux.

- **Copy** works exactly as before. Every copy is also pushed onto a stack.
- **One hotkey pops the newest item and pastes it** — press it again for the next one.
  macOS default **⌘⌃V**, Windows/Linux default **Ctrl+Alt+V**. Bindable to plain ⌘V / Ctrl+V.
- Popped items stay in a **history** (15 by default, circulates). **★ Star** anything to keep it forever.
- Click a row in the tray menu to copy it. Submenus for Paste now / Star / Remove.

This is the Rust port of the Swift menubar app in the parent folder; same config and
history file format, so on macOS either can be used.

## Install

```bash
npx clipstack install
```

That copies the binary for your platform to a stable location, registers it to start at
login, and launches it. `npx clipstack uninstall` reverses it; `npx clipstack status` shows state.

| Platform | Binary goes to | Autostart | Notes |
|---|---|---|---|
| macOS | `~/.local/bin/clipstack` | LaunchAgent | Asks for **Accessibility** once |
| Windows | `%LOCALAPPDATA%\ClipStack\clipstack.exe` | `HKCU\…\Run` | No admin needed |
| Linux | `~/.local/bin/clipstack` | `~/.config/autostart` | X11: full. Wayland: see below |

## How the hotkey works on each platform

| | Mechanism | Plain ⌘V / Ctrl+V binding | Other chords |
|---|---|---|---|
| macOS | CGEvent tap | ✅ rewritten in place | ✅ rewritten in place |
| Windows | Low-level keyboard hook | ✅ passed through after clipboard swap | swallowed + synthetic Ctrl+V |
| Linux X11 | Global hotkey | ❌ (would block all pasting) | synthetic Ctrl+V |
| Linux Wayland | — | ❌ | bind a desktop shortcut to `clipstack pop` |

"Rewritten in place" means your own keystroke does the pasting: the app in front receives
a normal paste, nothing synthetic is sent. The synthetic path waits until you release any
extra modifier keys (Alt/Shift) before pressing Ctrl+V.

With an empty stack the paste hotkey behaves as a **normal paste**.

## Config

`~/.config/clipstack/config.json` (macOS/Linux) or `%APPDATA%\clipstack\config.json` (Windows).
Created on first run; **Edit config…** in the menu opens it, **Reload config** applies changes.

```json
{
  "maxItems": 5,
  "maxHistory": 15,
  "persistHistory": true,
  "pollInterval": 0.2,
  "showCountInMenubar": true,
  "hotkeys": {
    "pasteNewest": "ctrl+alt+v",
    "pasteOldest": "ctrl+alt+shift+v",
    "clear": "ctrl+alt+backspace",
    "peek": "ctrl+alt+l"
  }
}
```

Hotkey format: modifiers joined with `+`, then the key. Modifiers: `cmd`/`win`/`super`,
`ctrl`, `alt`/`option`, `shift`. Keys: `a`–`z`, `0`–`9`, `f1`–`f12`, `space`, `return`,
`tab`, `escape`, `backspace`, arrows, and `- = [ ] ; ' , . / \ `` ` ``. Empty string disables.

## CLI (for desktop shortcuts)

```
clipstack              start the tray app
clipstack pop          pop the newest item onto the clipboard
clipstack pop-oldest   pop the oldest item onto the clipboard
clipstack clear        empty the stack
clipstack config-path
```

`pop`, `pop-oldest` and `clear` are handed to the running tray app through a command file in
`<config dir>/commands/`, so the tray stays the single owner of the clipboard and the history
file. Without a running tray they fall back to acting on the history file directly.

## Build from source

Needs Rust (https://rustup.rs). On Linux also:
`sudo apt install libgtk-3-dev libayatana-appindicator3-dev libxdo-dev libxkbcommon-dev libxi-dev libxtst-dev`.

```bash
cd cross
cargo build --release
./target/release/clipstack
```

## Releasing

Tag `vX.Y.Z` and push. `.github/workflows/release.yml` builds macOS (arm64, x64),
Windows (x64) and Linux (x64, arm64), attaches archives to a GitHub release and publishes
`clipstack` plus the `@clipstack/<os>-<arch>` packages to npm (needs an `NPM_TOKEN` secret).

## Layout

```
cross/
  crates/clipstack/      Rust app
    src/main.rs          event loop, clipboard poll, menu/hotkey dispatch
    src/app.rs           shared state + hotkey logic (used by hooks too)
    src/store.rs         history / stack / stars, JSON persistence
    src/config.rs        config file
    src/hotkey.rs        "cmd+ctrl+v" parsing, key tables
    src/paste.rs         synthetic paste with modifier wait
    src/tray.rs          tray icon + menu
    src/platform/        macos.rs (event tap) · windows.rs (LL hook) · linux.rs
  npm/clipstack/         the `npx clipstack` wrapper
  npm/scripts/           builds @clipstack/<os>-<arch> packages in CI
```
