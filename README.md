# ClipStack

A tiny macOS menubar app that turns the clipboard into a stack.

- **⌘C** works exactly as before. Every copy is also pushed onto the stack.
- **⌘V** works exactly as before (pastes the last thing you copied).
- **⌘⌃V** pops the newest item off the stack and pastes it (LIFO).
- **⌘⌃⇧V** pops the oldest item and pastes it (FIFO — paste in the order you copied).
- **⌘⌃⌫** clears the stack. **⌘⌃L** opens the menu to see what's on it.

Popping an item takes it off the stack but **keeps it in history**. Click the 📋 icon to see
three sections — **Stack** (not yet pasted), **★ Starred** (kept always), **History** (circulates,
15 by default, oldest auto-removed).

Every row works the same way:

| Action | Does |
|---|---|
| click the text | copy to clipboard |
| click ☆ / ★ at the right | star / unstar |
| ⌥-click | paste now (on a stack row: paste **and pop**) |
| ⌘-click | remove |

The stack holds 5 items by default (`maxItems`); the oldest is dropped when a sixth is copied.

Starred items are never auto-removed and never cleared by "Clear history".
History is saved to `~/.config/clipstack/history.json` and restored on launch
(`persistHistory` in the config; starred items are always saved).

## Build & install

Needs Xcode Command Line Tools (`xcode-select --install`). The script compiles with
`swiftc` directly, so it doesn't depend on SwiftPM working (which it doesn't on some
Command Line Tools-only installs). `Package.swift` is there for opening the project in Xcode.

```bash
./make-app.sh --install
```

That builds `build/ClipStack.app`, copies it to `/Applications`, and launches it.
Run `./make-app.sh` without the flag to just build.

First launch: macOS asks for **Accessibility** access. Allow it in
System Settings → Privacy & Security → Accessibility. Without it the app still records
copies and puts the popped item on the clipboard, but it can't press ⌘V for you.

To start it at login: System Settings → General → Login Items → **+** → ClipStack.

## Customise

The config lives at `~/.config/clipstack/config.json` (created on first run; also reachable
from the menu via **Edit config…**). After editing, choose **Reload config** from the menu.

```json
{
  "maxItems": 5,
  "maxHistory": 15,
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

You can bind the paste key to plain `"cmd+v"` if you like: ⌘V then pops from the stack
when it has items and behaves as a normal paste when it's empty. (ClipStack watches the
keystroke on its way to the app and swaps the clipboard first — the app does the pasting,
so this works everywhere ⌘V works.)

Hotkey format: modifiers joined with `+`, then the key.
Modifiers: `cmd`, `ctrl`, `alt` (or `option`), `shift`.
Keys: `a`–`z`, `0`–`9`, `f1`–`f12`, `space`, `return`, `tab`, `escape`, `backspace`,
`up`, `down`, `left`, `right`, and `- = [ ] ; ' , . / \ `` ` ``.
Set a hotkey to `""` to disable it.

## Notes

- Text only. Images and files are ignored.
- Items marked as concealed by password managers are never recorded.
- Global hotkeys registered this way take priority over an app's own shortcut with the same keys.
- Not sandboxed (Accessibility + synthetic keystrokes aren't allowed in the App Sandbox), so
  this can't be distributed through the Mac App Store as-is.
