import ApplicationServices
import Carbon.HIToolbox
import Cocoa

/// Writes text to the clipboard and synthesizes ⌘V into the frontmost app.
///
/// Pastes are queued and only sent once the user has released every modifier other
/// than ⌘. The hotkey that triggers a paste is usually ⌘⌃V, and if we post ⌘V while
/// Control is still physically held, the target app sees ⌃⌘V and ignores it.
enum Paster {
    /// True when the app has been granted Accessibility. Without it CGEvent posting
    /// is silently ignored by the system.
    static var isTrusted: Bool { AXIsProcessTrusted() }

    /// Asks macOS to show the "allow Accessibility" prompt for this app.
    static func requestAccessibility() {
        let key = kAXTrustedCheckOptionPrompt.takeUnretainedValue() as String
        _ = AXIsProcessTrustedWithOptions([key: true] as CFDictionary)
    }

    static func setClipboard(_ text: String) {
        let pb = NSPasteboard.general
        pb.clearContents()
        pb.setString(text, forType: .string)
    }

    /// Writes every flavour the item has. Declared richest-first so apps that can
    /// take RTF/HTML do, while plain-text fields still get the text.
    static func setClipboard(_ item: ClipItem) {
        let pb = NSPasteboard.general
        pb.clearContents()
        var types: [NSPasteboard.PasteboardType] = []
        if item.rtf != nil { types.append(.rtf) }
        if item.html != nil { types.append(.html) }
        types.append(.string)
        pb.declareTypes(types, owner: nil)
        if let rtf = item.rtf { pb.setData(rtf, forType: .rtf) }
        if let html = item.html { pb.setString(html, forType: .html) }
        pb.setString(item.text, forType: .string)
    }

    // MARK: - Queued paste

    /// Longest we wait for modifiers to be released before pasting anyway.
    private static let modifierWaitLimit: TimeInterval = 2.0
    /// Gap between consecutive queued pastes so the target app can keep up.
    private static let gapBetweenPastes: TimeInterval = 0.08

    private static var queue: [() -> Void] = []
    private static var drainTimer: Timer?

    /// Schedules a paste. `prepare` runs right before ⌘V is sent and should put the
    /// text on the clipboard, so queued pastes each paste their own item.
    static func enqueuePaste(prepare: @escaping () -> Void) {
        queue.append(prepare)
        startDrainIfNeeded()
    }

    private static var extraModifiersDown: Bool {
        let flags = CGEventSource.flagsState(.combinedSessionState)
        return !flags.intersection([.maskControl, .maskAlternate, .maskShift]).isEmpty
    }

    private static func startDrainIfNeeded() {
        guard drainTimer == nil else { return }
        let started = Date()
        let timer = Timer(timeInterval: 0.01, repeats: true) { t in
            let waitedTooLong = Date().timeIntervalSince(started) > modifierWaitLimit
            if extraModifiersDown && !waitedTooLong { return }

            t.invalidate()
            drainTimer = nil
            guard !queue.isEmpty else { return }

            let prepare = queue.removeFirst()
            prepare()
            Log.info("sending ⌘V (hw flags=\(describe(CGEventSource.flagsState(.combinedSessionState))) secureInput=\(IsSecureEventInputEnabled()) waited=\(String(format: "%.2f", Date().timeIntervalSince(started)))s)")
            sendCommandV()

            if !queue.isEmpty {
                DispatchQueue.main.asyncAfter(deadline: .now() + gapBetweenPastes) {
                    startDrainIfNeeded()
                }
            }
        }
        RunLoop.main.add(timer, forMode: .common)
        drainTimer = timer
    }

    private static func sendCommandV() {
        let src = CGEventSource(stateID: .combinedSessionState)
        // Briefly suppress real local keyboard events so a still-held key can't
        // get merged into the synthetic ⌘V.
        src?.setLocalEventsFilterDuringSuppressionState(
            [.permitLocalMouseEvents, .permitSystemDefinedEvents],
            state: .eventSuppressionStateSuppressionInterval)

        let v = CGKeyCode(kVK_ANSI_V)
        guard let down = CGEvent(keyboardEventSource: src, virtualKey: v, keyDown: true),
              let up = CGEvent(keyboardEventSource: src, virtualKey: v, keyDown: false)
        else { return }
        down.flags = .maskCommand
        up.flags = .maskCommand
        down.post(tap: .cghidEventTap)
        up.post(tap: .cghidEventTap)
    }

    private static func describe(_ flags: CGEventFlags) -> String {
        var parts: [String] = []
        if flags.contains(.maskControl)   { parts.append("ctrl") }
        if flags.contains(.maskAlternate) { parts.append("alt") }
        if flags.contains(.maskShift)     { parts.append("shift") }
        if flags.contains(.maskCommand)   { parts.append("cmd") }
        return parts.isEmpty ? "none" : parts.joined(separator: "+")
    }
}
