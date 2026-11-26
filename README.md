<p align="center"><img src="assets/icon-256.png" width="128" alt="ClipStack icon"></p>

# ClipStack

**A clipboard stack for your menubar / tray — macOS, Windows, Linux.**
Copy several things, then one hotkey pastes them back one at a time: newest first (LIFO)
or in the order you copied (FIFO).

```
copy A · copy B · copy C        →   stack [A, B, C]        tray shows 📋3
hotkey · hotkey · hotkey        →   pastes C, B, A         (A, B, C in FIFO mode)
```

- Copy and paste keep working exactly as before; ClipStack only adds a stack beside them.
- Popped items stay in a **history** (15, circulates). **★ Star** anything to keep it forever.
- **Formatting survives** — fonts, bold, colours, links are pasted back as they were copied.
- Text only, stays on your machine, password-manager copies are never recorded.
- Native and small (≈1.5 MB). No Electron, no account, no network.

---

## Table of contents

1. [Install](#install) — [macOS](#macos) · [Windows](#windows) · [Linux](#linux)
2. [Commands](#commands)
3. [Using it](#using-it)
4. [Configuration](#configuration) — [file location](#where-the-config-lives) · [all options](#all-options) · [hotkey syntax](#hotkey-syntax) · [recipes](#recipes)
5. [Troubleshooting](#troubleshooting)
6. [Build from source](#build-from-source)

---

## Install

The same command on every platform (needs [Node.js](https://nodejs.org) 18+, which you
probably already have):

```bash
npx clipstack install
```

It downloads nothing extra: the binary for your OS/CPU ships inside the npm package. The
command copies it to a stable place, registers it to start at login, and launches it.

| | Binary installed to | Starts at login via |
|---|---|---|
| macOS | `~/.local/bin/clipstack` | LaunchAgent `~/Library/LaunchAgents/com.clipstack.agent.plist` |
| Windows | `%LOCALAPPDATA%\ClipStack\clipstack.exe` | `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` |
| Linux | `~/.local/bin/clipstack` | `~/.config/autostart/clipstack.desktop` |

No administrator / sudo rights are needed anywhere.

> **Status:** the npm package is being published for the first time; until it appears on
> npmjs.com use [Build from source](#build-from-source).

### macOS

```bash
npx clipstack install
```

Then grant **Accessibility** when macOS asks (System Settings → Privacy & Security →
Accessibility → ClipStack **on**). That is what lets it paste for you. Default hotkey **⌘⌃V**.

Prefer a native `.app` in `/Applications`? This repo also contains a Swift menubar app with
the same features and a nicer menu (☆ buttons on every row):

```bash
git clone https://github.com/phabricator1/Clipstack.git && cd Clipstack
./make-app.sh --install
```

Needs only the Xcode Command Line Tools (`xcode-select --install`). It signs with your
Developer ID / Apple Development certificate if you have one, so the Accessibility grant
survives updates. Add it to *Login Items* to start at boot. Uninstall: quit it from the 📋
menu, delete `/Applications/ClipStack.app`.

### Windows

PowerShell or Command Prompt:

```powershell
npx clipstack install
```

Nothing else to grant. Default hotkey **Ctrl+Alt+V**. Works in every window except ones
running *as Administrator* (Windows blocks non-elevated apps from pasting into those).

If `npx` isn't recognised, install Node.js from https://nodejs.org (LTS) and open a new
terminal.

### Linux

```bash
npx clipstack install
```

Runtime libraries (usually already present on a desktop system):

```bash
# Debian / Ubuntu
sudo apt install libgtk-3-0 libayatana-appindicator3-1 libxdo3
# Fedora
sudo dnf install gtk3 libappindicator-gtk3 xdotool
# Arch
sudo pacman -S gtk3 libappindicator-gtk3 xdotool
```

GNOME needs the [AppIndicator extension](https://extensions.gnome.org/extension/615/appindicator-support/)
to show tray icons at all (KDE, XFCE, Cinnamon, MATE show them natively).

Default hotkey **Ctrl+Alt+V**.

**X11** — everything works out of the box.

**Wayland** (default on recent Ubuntu/Fedora) — Wayland does not let ordinary apps register
global hotkeys or type into other windows, so the hotkey can't work the usual way. The tray
menu, history and stars still work. For the hotkey, bind a desktop shortcut to the CLI:

```bash
# GNOME: Settings → Keyboard → Custom Shortcuts → "+"
#   Command:  /home/<you>/.local/bin/clipstack pop      Shortcut: Ctrl+Alt+V
# or from a terminal:
gsettings set org.gnome.settings-daemon.plugins.media-keys custom-keybindings \
  "['/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/clipstack/']"
gsettings set org.gnome.settings-daemon.plugins.media-keys.custom-keybinding:/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/clipstack/ name 'ClipStack pop'
gsettings set org.gnome.settings-daemon.plugins.media-keys.custom-keybinding:/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/clipstack/ command "$HOME/.local/bin/clipstack pop"
gsettings set org.gnome.settings-daemon.plugins.media-keys.custom-keybinding:/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/clipstack/ binding '<Ctrl><Alt>v'
```

KDE: System Settings → Shortcuts → Custom Shortcuts → New → Command: `~/.local/bin/clipstack pop`.

`clipstack pop` hands the request to the running tray app, which puts the next item on the
clipboard (about 50 ms); you then press Ctrl+V as usual. If the tray app is not running, the
command works on the saved history directly instead.

---

## Commands

```
npx clipstack install        install, register autostart, launch
npx clipstack uninstall      stop, remove autostart and the binary (config & history are kept)
npx clipstack start          launch it
npx clipstack stop           quit it
npx clipstack status         installed? running? autostart? config path

npx clipstack pop            pop the newest item onto the clipboard   (desktop-shortcut friendly)
npx clipstack pop-oldest     pop the oldest item onto the clipboard
npx clipstack clear          empty the stack
npx clipstack config-path    print where the config file is
```

After `install`, the binary is also directly on disk (see table above), so for shortcuts
you can call it without `npx`: `~/.local/bin/clipstack pop`.

---

## Using it

| macOS | Windows / Linux | Does |
|---|---|---|
| ⌘C | Ctrl+C | normal copy — *also* pushes onto the stack |
| ⌘V | Ctrl+V | normal paste, unchanged |
| **⌘⌃V** | **Ctrl+Alt+V** | **pop & paste** — newest first (LIFO), or oldest first in FIFO mode |
| ⌘⌃⇧V | Ctrl+Alt+Shift+V | pop & paste from the *other* end |
| ⌘⌃⌫ | Ctrl+Alt+Backspace | clear the stack |
| ⌘⌃L | Ctrl+Alt+L | open the menu |

- With an **empty stack** the pop hotkey behaves as a normal paste.
- Holding the hotkey down does **not** drain the stack (key-repeat is ignored).
- The tray/menubar count (📋3) is the number of items waiting on the stack.

### The menu

Click the 📋 icon:

| Section | What's in it |
|---|---|
| **Paste order** | one click switches LIFO ↔ FIFO, saved to the config |
| **STACK** | copied, not yet pasted — the hotkey takes from here |
| **★ STARRED** | items you've starred; never removed automatically |
| **HISTORY** | everything else you've copied, newest first, 15 kept |

Click any row to copy it back to the clipboard.
macOS native app: **☆** button on each row stars it · ⌥-click pastes · ⌘-click removes.
npm/Rust app: *Paste now…*, *Star / unstar…*, *Remove…* submenus.
*Clear stack* keeps history; *Clear history* keeps starred items.

---

## Configuration

Everything is in one JSON file. Change it, then choose **Reload config** from the menu
(or restart the app). Keys you leave out keep their defaults; a broken file is ignored with
a log message rather than crashing the app.

### Where the config lives

| | Config | History (your copied items) |
|---|---|---|
| macOS | `~/.config/clipstack/config.json` | `~/.config/clipstack/history.json` |
| Linux | `~/.config/clipstack/config.json` | `~/.config/clipstack/history.json` |
| Windows | `%APPDATA%\clipstack\config.json` | `%APPDATA%\clipstack\history.json` |

`npx clipstack config-path` prints it; **Edit config…** in the menu opens it in your editor.

### Default config

macOS:

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

Windows and Linux — identical except the hotkeys:

```json
  "hotkeys": {
    "pasteNewest": "ctrl+alt+v",
    "pasteOldest": "ctrl+alt+shift+v",
    "clear": "ctrl+alt+backspace",
    "peek": "ctrl+alt+l"
  }
```

### All options

| Key | Type | Default | Meaning |
|---|---|---|---|
| `maxItems` | number | `5` | Stack size. Copying a 6th item drops the oldest from the stack (it stays in history). |
| `maxHistory` | number | `15` | How many unstarred items history keeps. Oldest are removed automatically. Starred items don't count. |
| `pasteOrder` | `"lifo"` \| `"fifo"` | `"lifo"` | Which end the **main** hotkey pastes from. `lifo` = newest first. `fifo` = the order you copied. The second hotkey always takes the other end. Also switchable from the menu. |
| `persistHistory` | boolean | `true` | Save history to disk so it survives restarts. With `false`, only starred items are saved. |
| `pollInterval` | seconds | `0.2` | How often the clipboard is checked. Lower = catches very fast successive copies; 0.1–0.5 is sensible. |
| `showCountInMenubar` | boolean | `true` | Show the stack count next to the icon (macOS title; Windows/Linux tooltip). |
| `hotkeys.pasteNewest` | string | see above | Main pop-and-paste key. |
| `hotkeys.pasteOldest` | string | see above | Pop from the other end. |
| `hotkeys.clear` | string | see above | Empty the stack. |
| `hotkeys.peek` | string | see above | Open the menu (macOS; no-op where the OS can't open tray menus programmatically). |

Any hotkey can be disabled with an empty string: `"peek": ""`.

### Hotkey syntax

Modifiers joined with `+`, the key last. Case doesn't matter.

| Modifiers | Keys |
|---|---|
| `cmd` (= `command`, `meta`, `super`, `win`) · `ctrl` · `alt` (= `option`) · `shift` | `a`–`z`, `0`–`9`, `f1`–`f12`, `space`, `return`/`enter`, `tab`, `escape`, `backspace`, `delete`, `up`, `down`, `left`, `right`, and `- = [ ] ; ' , . / \ `` ` `` |

Examples: `"cmd+ctrl+v"`, `"ctrl+alt+shift+f5"`, `"win+shift+v"`, `"ctrl+space"`.

### Recipes

**Paste in the order I copied (FIFO) by default**
```json
"pasteOrder": "fifo"
```

**Make plain ⌘V / Ctrl+V the stack key** — pops when the stack has items, pastes normally when empty:
```json
"hotkeys": { "pasteNewest": "cmd+v" }          // macOS
"hotkeys": { "pasteNewest": "ctrl+v" }         // Windows
```
(Not available on Linux — there it would block normal pasting. Remember ⌘V ⌘V then pastes two *different* items.)

**Bigger stack, longer history**
```json
"maxItems": 20,
"maxHistory": 100
```

**Nothing on disk except starred items**
```json
"persistHistory": false
```

**Only the stack hotkey, nothing else**
```json
"hotkeys": { "pasteOldest": "", "clear": "", "peek": "" }
```

**Avoid a clash** (e.g. an app already uses Ctrl+Alt+V):
```json
"hotkeys": { "pasteNewest": "ctrl+shift+alt+v" }
```

---

## Troubleshooting

**Hotkey does nothing / count doesn't drop**
- Another app owns that key. Pick a different one in the config, *Reload config*.
- Linux Wayland: hotkeys can't work from inside the app — use the desktop-shortcut setup above.

**Count drops but nothing is pasted (macOS)**
- Accessibility isn't granted: System Settings → Privacy & Security → Accessibility → ClipStack on, then quit and reopen it.
- If the switch is already on but it still fails, the grant belongs to an older build. Remove the entry (−), add it again (+), restart the app. From a terminal: `tccutil reset Accessibility local.clipstack`, then relaunch.

**Pasting into an admin window fails (Windows)** — expected; Windows blocks it. Run that app non-elevated or copy manually.

**Copies from my password manager appear** — they shouldn't: ClipStack honours the `ConcealedType` marker all major managers set. Report the manager if you see one.

**Where are the logs?**
- macOS native app: `~/Library/Logs/ClipStack.log`
- npm/Rust app: run it from a terminal with `RUST_LOG=info clipstack` to see what it's doing.

**Reset everything** — quit the app and delete the config and history files listed above; they're recreated with defaults on the next start.

---

## Build from source

### Cross-platform app (Rust)

```bash
git clone https://github.com/phabricator1/Clipstack.git
cd Clipstack/cross
cargo build --release          # needs Rust: https://rustup.rs
./target/release/clipstack     # run it
```

Linux build deps: `sudo apt install libgtk-3-dev libayatana-appindicator3-dev libxdo-dev libxkbcommon-dev libxi-dev libxtst-dev`.

### macOS native app (Swift)

```bash
git clone https://github.com/phabricator1/Clipstack.git
cd Clipstack
./make-app.sh --install        # needs Xcode Command Line Tools
```

### Releasing (maintainers)

Tag `vX.Y.Z` and push the tag; GitHub Actions builds macOS (arm64, x64), Windows (x64) and
Linux (x64, arm64), attaches archives to a GitHub Release and publishes the npm packages.
Details in [PUBLISHING.md](PUBLISHING.md).

---

## How the paste actually happens

Most clipboard tools grab your hotkey and *send a fake ⌘V*. That breaks when Control is
still held (the app sees ⌃⌘V) and makes binding plain ⌘V impossible. ClipStack instead
watches the keystroke on its way to the app, swaps the clipboard, and lets **your own
keystroke** continue as a plain paste — so the app in front pastes exactly as if you'd
pressed ⌘V yourself.

| Platform | Mechanism |
|---|---|
| macOS | CGEvent tap — rewrites ⌘⌃V into ⌘V in place |
| Windows | low-level keyboard hook — Ctrl+V passes through after the swap; other chords use a synthetic Ctrl+V sent once Alt/Shift are released |
| Linux X11 | global hotkey + synthetic Ctrl+V |
| Linux Wayland | `clipstack pop` bound to a desktop shortcut |

## Repository layout

```
Sources/ClipStack/     macOS Swift app
make-app.sh            build + sign + install the macOS .app
cross/                 Rust app (macOS/Windows/Linux), npm wrapper, platform packaging
.github/workflows/     release pipeline
PUBLISHING.md          how to cut a release
```

## Licence

MIT
