import Carbon.HIToolbox
import Cocoa

final class AppDelegate: NSObject, NSApplicationDelegate, NSMenuDelegate {
    private var config = Config.load()
    private lazy var store = ClipStore(maxStack: config.maxItems,
                                       maxHistory: config.maxHistory,
                                       persist: config.persistHistory)
    private var statusItem: NSStatusItem!
    private var pollTimer: Timer?

    private var lastChangeCount = NSPasteboard.general.changeCount
    /// Set right before we write to the clipboard ourselves, so the poller
    /// doesn't push the item we're pasting back onto the stack.
    private var ignoreNextChange = false

    /// Only nag for Accessibility once per launch, not on every paste.
    private var accessibilityPromptShown = false

    /// Password managers mark secrets with this type; never record those.
    private let concealedType = NSPasteboard.PasteboardType("org.nspasteboard.ConcealedType")

    private let maxStackRows = 15
    private let maxHistoryRows = 30
    /// Largest RTF/HTML payload (bytes) kept per item.
    private let richFlavourLimit = 2_000_000

    /// How hotkeys are delivered. The event tap is preferred (needs Accessibility);
    /// Carbon hotkeys are the fallback until permission is granted.
    private enum HotkeyMode { case eventTap, carbon }
    private var hotkeyMode: HotkeyMode = .carbon
    private var pollTicks = 0

    private var bindNewest: HotKeyParser.Parsed?
    private var bindOldest: HotKeyParser.Parsed?
    private var bindClear: HotKeyParser.Parsed?
    private var bindPeek: HotKeyParser.Parsed?

    // MARK: - Lifecycle

    func applicationDidFinishLaunching(_ notification: Notification) {
        statusItem = NSStatusBar.system.statusItem(withLength: NSStatusItem.variableLength)
        statusItem.menu = NSMenu()
        statusItem.menu?.delegate = self
        updateMenubarTitle()

        applyConfig()

        if !Paster.isTrusted {
            accessibilityPromptShown = true
            Paster.requestAccessibility()
        }
    }

    func applicationWillTerminate(_ notification: Notification) {
        store.saveNow()
        HotKeyCenter.shared.unregisterAll()
    }

    /// (Re)starts the clipboard poller and (re)registers hotkeys from `config`.
    private func applyConfig() {
        pollTimer?.invalidate()
        let timer = Timer(timeInterval: max(0.05, config.pollInterval), repeats: true) { [weak self] _ in
            self?.pollPasteboard()
        }
        // .common so it keeps ticking while a menu is open.
        RunLoop.main.add(timer, forMode: .common)
        pollTimer = timer

        bindNewest = HotKeyParser.parse(config.hotkeys.pasteNewest)
        bindOldest = HotKeyParser.parse(config.hotkeys.pasteOldest)
        bindClear  = HotKeyParser.parse(config.hotkeys.clear)
        bindPeek   = HotKeyParser.parse(config.hotkeys.peek)

        let center = HotKeyCenter.shared
        center.unregisterAll()

        if Paster.isTrusted && KeyTap.shared.start() {
            hotkeyMode = .eventTap
            KeyTap.shared.handler = { [weak self] key, mods, autorepeat in
                self?.tapDecision(keyCode: key, modifiers: mods, autorepeat: autorepeat) ?? .pass
            }
        } else {
            hotkeyMode = .carbon
            KeyTap.shared.stop()
            // A Carbon hotkey on plain ⌘V would swallow every paste on the system,
            // so it is only honoured once the event tap is available.
            if bindNewest?.isPlainCommandV == true {
                Log.info("pasteNewest is ⌘V; waiting for Accessibility before enabling it")
            } else {
                center.register(config.hotkeys.pasteNewest) { [weak self] in self?.pasteNewest() }
            }
            if bindOldest?.isPlainCommandV == true {
                Log.info("pasteOldest is ⌘V; waiting for Accessibility before enabling it")
            } else {
                center.register(config.hotkeys.pasteOldest) { [weak self] in self?.pasteOldest() }
            }
            center.register(config.hotkeys.clear) { [weak self] in self?.clearStack() }
            center.register(config.hotkeys.peek)  { [weak self] in self?.peek() }
        }

        Log.info("started. mode=\(hotkeyMode) hotkeys newest=\(bindNewest?.display ?? "INVALID") oldest=\(bindOldest?.display ?? "INVALID") clear=\(bindClear?.display ?? "INVALID") peek=\(bindPeek?.display ?? "INVALID") accessibility=\(Paster.isTrusted ? "granted" : "NOT granted") history=\(store.history.count) stack=\(store.stackCount)")

        store.maxStack = config.maxItems
        store.maxHistory = config.maxHistory
        store.persist = config.persistHistory
        updateMenubarTitle()
    }

    // MARK: - Clipboard watching

    private func pollPasteboard() {
        // Upgrade from Carbon hotkeys to the event tap once Accessibility is granted.
        pollTicks += 1
        if hotkeyMode == .carbon && pollTicks % 25 == 0 && Paster.isTrusted {
            applyConfig()
        }

        let pb = NSPasteboard.general
        guard pb.changeCount != lastChangeCount else { return }
        lastChangeCount = pb.changeCount

        if ignoreNextChange {
            ignoreNextChange = false
            return
        }
        if pb.types?.contains(concealedType) == true { return }
        guard let text = pb.string(forType: .string), !text.isEmpty else { return }
        if let top = store.stackItems.first, top.text == text { return }

        // Keep the rich flavours too (capped so a huge web page copy doesn't bloat history).
        let rtf = pb.data(forType: .rtf).flatMap { $0.count <= richFlavourLimit ? $0 : nil }
        let html = pb.string(forType: .html).flatMap { $0.utf8.count <= richFlavourLimit ? $0 : nil }

        store.push(text, rtf: rtf, html: html)
        Log.info("pushed \(text.count) chars\(rtf != nil ? " +rtf" : "")\(html != nil ? " +html" : ""), stack=\(store.stackCount) history=\(store.history.count)")
        updateMenubarTitle()
    }

    // MARK: - Event tap

    /// Runs inside the key tap for every key-down. For a paste binding we swap the
    /// clipboard *now* and let the same keystroke reach the app as a plain ⌘V, so the
    /// app itself performs the paste. With an empty stack the key acts as a normal paste.
    private func tapDecision(keyCode: CGKeyCode, modifiers: CGEventFlags, autorepeat: Bool) -> TapDecision {
        func matches(_ b: HotKeyParser.Parsed?) -> Bool {
            guard let b else { return false }
            return CGKeyCode(b.keyCode) == keyCode && b.cgModifiers == modifiers
        }

        let isNewest = matches(bindNewest)
        let isOldest = !isNewest && matches(bindOldest)
        if isNewest || isOldest {
            if autorepeat { return .swallow }   // holding the key must not drain the stack
            if let item = isNewest ? store.popNewest() : store.popOldest() {
                copyQuietly(item)
                updateMenubarTitle()
                Log.info("tap: \(isNewest ? "popped newest" : "popped oldest") \(item.text.count) chars\(item.isRich ? " (rich)" : "") -> ⌘V, stack=\(store.stackCount)")
            } else {
                Log.info("tap: stack empty -> normal paste")
            }
            return .rewrite(keyCode: CGKeyCode(kVK_ANSI_V), modifiers: .maskCommand)
        }
        if matches(bindClear) {
            if !autorepeat { clearStack() }
            return .swallow
        }
        if matches(bindPeek) {
            if !autorepeat { DispatchQueue.main.async { self.peek() } }
            return .swallow
        }
        return .pass
    }

    // MARK: - Stack actions (Carbon fallback path)

    private func pasteNewest() {
        Log.info("hotkey pasteNewest fired, stack=\(store.stackCount)")
        guard let item = store.popNewest() else { NSSound.beep(); return }
        paste(item)
    }

    private func pasteOldest() {
        Log.info("hotkey pasteOldest fired, stack=\(store.stackCount)")
        guard let item = store.popOldest() else { NSSound.beep(); return }
        paste(item)
    }

    /// Puts the item (all its flavours) on the clipboard without recording it as a new copy.
    private func copyQuietly(_ item: ClipItem) {
        ignoreNextChange = true
        Paster.setClipboard(item)
        lastChangeCount = NSPasteboard.general.changeCount
        ignoreNextChange = false
    }

    private func paste(_ item: ClipItem) {
        updateMenubarTitle()
        Log.info("pasting \(item.text.count) chars\(item.isRich ? " (rich)" : ""), accessibility=\(Paster.isTrusted ? "granted" : "NOT granted -> left on clipboard")")

        if Paster.isTrusted {
            // Clipboard is set right before ⌘V goes out, so queued pastes
            // each deliver their own item.
            Paster.enqueuePaste { [weak self] in self?.copyQuietly(item) }
        } else {
            // Text is on the clipboard anyway; user can ⌘V manually.
            copyQuietly(item)
            if !accessibilityPromptShown {
                accessibilityPromptShown = true
                Paster.requestAccessibility()
            }
        }
    }

    private func clearStack() {
        store.clearStack()
        updateMenubarTitle()
    }

    private func peek() {
        statusItem.button?.performClick(nil)
    }

    private func reloadConfig() {
        config = Config.load()
        applyConfig()
    }

    private func openConfig() {
        if !FileManager.default.fileExists(atPath: Config.fileURL.path) {
            config.save()
        }
        NSWorkspace.shared.open(Config.fileURL)
    }

    // MARK: - Menubar

    private func updateMenubarTitle() {
        guard let button = statusItem.button else { return }
        let count = store.stackCount
        if config.showCountInMenubar && count > 0 {
            button.title = "📋\(count)"
        } else {
            button.title = "📋"
        }
    }

    /// Rebuilds the menu each time it opens so it reflects the current state.
    ///
    /// Every row: click copies · ⌥-click pastes · ⇧-click stars/unstars · ⌘-click removes.
    /// On stack rows, ⌥-click also pops the item.
    func menuNeedsUpdate(_ menu: NSMenu) {
        menu.removeAllItems()

        let stackItems = store.stackItems
        let starred = store.pinnedItems
        let history = store.unpinnedHistory

        menu.addItem(header("click copies  ·  ☆ stars (keep always)  ·  ⌥-click pastes  ·  ⌘-click removes"))
        menu.addItem(.separator())

        // Stack
        if stackItems.isEmpty {
            menu.addItem(header("STACK — empty, copy something"))
        } else {
            menu.addItem(header("STACK (\(stackItems.count) of \(config.maxItems)) — ⌘⌃V pops the top"))
            for item in stackItems.prefix(maxStackRows) {
                addRows(for: item, to: menu, onStack: true)
            }
            if stackItems.count > maxStackRows {
                menu.addItem(header("… and \(stackItems.count - maxStackRows) more"))
            }
        }

        // Starred
        if !starred.isEmpty {
            menu.addItem(.separator())
            menu.addItem(header("★ STARRED"))
            for item in starred {
                addRows(for: item, to: menu, onStack: false)
            }
        }

        // History
        menu.addItem(.separator())
        if history.isEmpty {
            menu.addItem(header("HISTORY — empty"))
        } else {
            menu.addItem(header("HISTORY (\(history.count) of \(config.maxHistory))"))
            for item in history.prefix(maxHistoryRows) {
                addRows(for: item, to: menu, onStack: false)
            }
            if history.count > maxHistoryRows {
                menu.addItem(header("… and \(history.count - maxHistoryRows) more"))
            }
        }

        menu.addItem(.separator())

        let clearS = NSMenuItem(title: "Clear stack (history stays)", action: #selector(menuClearStack), keyEquivalent: "")
        clearS.target = self
        clearS.isEnabled = !stackItems.isEmpty
        menu.addItem(clearS)

        let clearH = NSMenuItem(title: "Clear history (starred stay)…", action: #selector(menuClearHistory), keyEquivalent: "")
        clearH.target = self
        clearH.isEnabled = !history.isEmpty
        menu.addItem(clearH)

        menu.addItem(.separator())
        menu.addItem(header(hotkeySummary()))

        let open = NSMenuItem(title: "Edit config…", action: #selector(menuOpenConfig), keyEquivalent: "")
        open.target = self
        menu.addItem(open)

        let reload = NSMenuItem(title: "Reload config", action: #selector(menuReloadConfig), keyEquivalent: "")
        reload.target = self
        menu.addItem(reload)

        if !Paster.isTrusted {
            let ax = NSMenuItem(title: "⚠️ Grant Accessibility to enable auto-paste…", action: #selector(menuRequestAX), keyEquivalent: "")
            ax.target = self
            menu.addItem(ax)
        }

        menu.addItem(.separator())
        let quit = NSMenuItem(title: "Quit ClipStack", action: #selector(NSApplication.terminate(_:)), keyEquivalent: "q")
        menu.addItem(quit)
    }

    /// Adds one row: text (click = copy, ⌥ = paste, ⌘ = remove) and a ☆/★ button.
    private func addRows(for item: ClipItem, to menu: NSMenu, onStack: Bool) {
        let view = ClipRowView(title: (item.isRich ? "𝐀 " : "") + preview(item.text), starred: item.pinned)
        view.toolTip = item.text
        let id = item.id

        view.onClick = { [weak self] flags in
            guard let self, let current = self.store.item(id: id) else { return }
            if flags.contains(.command) {
                self.store.remove(id: id)
                self.updateMenubarTitle()
            } else if flags.contains(.option) {
                if onStack { _ = self.store.pop(id: id) }
                self.afterMenuCloses { self.paste(current) }
            } else if flags.contains(.shift) {
                self.store.togglePin(id: id)
            } else {
                self.copyQuietly(current)
            }
        }
        view.onStar = { [weak self] in
            self?.store.togglePin(id: id)
        }

        let row = NSMenuItem()
        row.view = view
        row.toolTip = item.text
        menu.addItem(row)
    }

    private func header(_ title: String) -> NSMenuItem {
        let h = NSMenuItem(title: title, action: nil, keyEquivalent: "")
        h.isEnabled = false
        return h
    }

    private func preview(_ text: String) -> String {
        let oneLine = text.replacingOccurrences(of: "\n", with: " ⏎ ")
            .trimmingCharacters(in: .whitespaces)
        return oneLine.count > 60 ? String(oneLine.prefix(60)) + "…" : oneLine
    }

    private func hotkeySummary() -> String {
        let n = HotKeyParser.parse(config.hotkeys.pasteNewest)?.display ?? "—"
        let o = HotKeyParser.parse(config.hotkeys.pasteOldest)?.display ?? "—"
        return "Hotkeys: pop newest \(n) · pop oldest \(o)"
    }

    // The menu closes before the action runs, but the previously frontmost
    // app may need a moment to regain focus before it receives ⌘V.
    private func afterMenuCloses(_ work: @escaping () -> Void) {
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.15, execute: work)
    }

    // MARK: - Menu actions

    @objc private func menuClearStack() { clearStack() }
    @objc private func menuOpenConfig() { openConfig() }
    @objc private func menuReloadConfig() { reloadConfig() }
    @objc private func menuRequestAX() { Paster.requestAccessibility() }

    @objc private func menuClearHistory() {
        let alert = NSAlert()
        alert.messageText = "Clear clipboard history?"
        alert.informativeText = "Starred items are kept. This also empties the stack."
        alert.addButton(withTitle: "Clear")
        alert.addButton(withTitle: "Cancel")
        NSApp.activate(ignoringOtherApps: true)
        if alert.runModal() == .alertFirstButtonReturn {
            store.clearHistory()
            updateMenubarTitle()
        }
    }

}
