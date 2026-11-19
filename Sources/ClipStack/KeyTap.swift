import Cocoa

/// What to do with a key-down the tap has seen.
enum TapDecision {
    /// Let it through untouched.
    case pass
    /// Drop it; the app never sees it.
    case swallow
    /// Deliver it to the app as a different key: used to turn ⌘⌃V into a plain ⌘V
    /// *after* the clipboard has been swapped, so the app pastes the popped item.
    case rewrite(keyCode: CGKeyCode, modifiers: CGEventFlags)
}

/// A session-level keyboard event tap. Needs Accessibility; without it `start()`
/// returns false and the caller should fall back to Carbon hotkeys.
///
/// Compared with RegisterEventHotKey + synthetic ⌘V this has three advantages:
/// the user's own keystroke does the pasting (nothing synthetic, nothing swallowed),
/// held modifiers can't leak into the paste because we rewrite the flags, and a
/// binding of plain ⌘V is safe — with an empty stack it passes through as a normal paste.
final class KeyTap {
    static let shared = KeyTap()

    /// Called on the main thread for every key-down. `autorepeat` is true while a key is held.
    var handler: ((_ keyCode: CGKeyCode, _ modifiers: CGEventFlags, _ autorepeat: Bool) -> TapDecision)?

    private var tap: CFMachPort?
    private var source: CFRunLoopSource?

    private static let modifierMask: CGEventFlags = [.maskCommand, .maskControl, .maskAlternate, .maskShift]

    private init() {}

    var isRunning: Bool { tap != nil }

    @discardableResult
    func start() -> Bool {
        if tap != nil { return true }
        let mask = CGEventMask(1 << CGEventType.keyDown.rawValue)
        let refcon = Unmanaged.passUnretained(self).toOpaque()
        guard let port = CGEvent.tapCreate(tap: .cgSessionEventTap,
                                           place: .headInsertEventTap,
                                           options: .defaultTap,
                                           eventsOfInterest: mask,
                                           callback: { _, type, event, refcon in
                                               let me = Unmanaged<KeyTap>.fromOpaque(refcon!).takeUnretainedValue()
                                               return me.handle(type: type, event: event)
                                           },
                                           userInfo: refcon)
        else { return false }
        tap = port
        let src = CFMachPortCreateRunLoopSource(kCFAllocatorDefault, port, 0)
        source = src
        CFRunLoopAddSource(CFRunLoopGetMain(), src, .commonModes)
        CGEvent.tapEnable(tap: port, enable: true)
        return true
    }

    func stop() {
        if let src = source { CFRunLoopRemoveSource(CFRunLoopGetMain(), src, .commonModes) }
        if let port = tap { CGEvent.tapEnable(tap: port, enable: false) }
        source = nil
        tap = nil
    }

    private func handle(type: CGEventType, event: CGEvent) -> Unmanaged<CGEvent>? {
        // macOS disables a tap that is slow or when the user asks it to; just turn it back on.
        if type == .tapDisabledByTimeout || type == .tapDisabledByUserInput {
            if let port = tap { CGEvent.tapEnable(tap: port, enable: true) }
            return Unmanaged.passUnretained(event)
        }
        guard type == .keyDown, let handler else { return Unmanaged.passUnretained(event) }

        let keyCode = CGKeyCode(event.getIntegerValueField(.keyboardEventKeycode))
        let mods = event.flags.intersection(KeyTap.modifierMask)
        let autorepeat = event.getIntegerValueField(.keyboardEventAutorepeat) != 0

        switch handler(keyCode, mods, autorepeat) {
        case .pass:
            return Unmanaged.passUnretained(event)
        case .swallow:
            return nil
        case .rewrite(let newKey, let newMods):
            event.setIntegerValueField(.keyboardEventKeycode, value: Int64(newKey))
            event.flags = event.flags.subtracting(KeyTap.modifierMask).union(newMods)
            return Unmanaged.passUnretained(event)
        }
    }
}
