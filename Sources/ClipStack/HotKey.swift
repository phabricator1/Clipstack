import Carbon.HIToolbox
import Foundation

/// Parses strings like "cmd+ctrl+v" into a Carbon (keyCode, modifiers) pair.
enum HotKeyParser {
    struct Parsed {
        let keyCode: UInt32
        /// Carbon modifier mask (cmdKey, controlKey, …) for RegisterEventHotKey.
        let modifiers: UInt32
        let display: String

        /// The same modifiers as CGEventFlags, for matching inside an event tap.
        var cgModifiers: CGEventFlags {
            var f: CGEventFlags = []
            if modifiers & UInt32(cmdKey) != 0     { f.insert(.maskCommand) }
            if modifiers & UInt32(controlKey) != 0 { f.insert(.maskControl) }
            if modifiers & UInt32(optionKey) != 0  { f.insert(.maskAlternate) }
            if modifiers & UInt32(shiftKey) != 0   { f.insert(.maskShift) }
            return f
        }

        /// True for a binding that is exactly ⌘V — fine with the event tap, unsafe as a
        /// Carbon hotkey because it would swallow every normal paste on the system.
        var isPlainCommandV: Bool {
            keyCode == UInt32(kVK_ANSI_V) && modifiers == UInt32(cmdKey)
        }
    }

    private static let keyCodes: [String: Int] = [
        "a": kVK_ANSI_A, "b": kVK_ANSI_B, "c": kVK_ANSI_C, "d": kVK_ANSI_D,
        "e": kVK_ANSI_E, "f": kVK_ANSI_F, "g": kVK_ANSI_G, "h": kVK_ANSI_H,
        "i": kVK_ANSI_I, "j": kVK_ANSI_J, "k": kVK_ANSI_K, "l": kVK_ANSI_L,
        "m": kVK_ANSI_M, "n": kVK_ANSI_N, "o": kVK_ANSI_O, "p": kVK_ANSI_P,
        "q": kVK_ANSI_Q, "r": kVK_ANSI_R, "s": kVK_ANSI_S, "t": kVK_ANSI_T,
        "u": kVK_ANSI_U, "v": kVK_ANSI_V, "w": kVK_ANSI_W, "x": kVK_ANSI_X,
        "y": kVK_ANSI_Y, "z": kVK_ANSI_Z,
        "0": kVK_ANSI_0, "1": kVK_ANSI_1, "2": kVK_ANSI_2, "3": kVK_ANSI_3,
        "4": kVK_ANSI_4, "5": kVK_ANSI_5, "6": kVK_ANSI_6, "7": kVK_ANSI_7,
        "8": kVK_ANSI_8, "9": kVK_ANSI_9,
        "f1": kVK_F1, "f2": kVK_F2, "f3": kVK_F3, "f4": kVK_F4, "f5": kVK_F5,
        "f6": kVK_F6, "f7": kVK_F7, "f8": kVK_F8, "f9": kVK_F9, "f10": kVK_F10,
        "f11": kVK_F11, "f12": kVK_F12,
        "space": kVK_Space, "return": kVK_Return, "enter": kVK_Return,
        "tab": kVK_Tab, "escape": kVK_Escape, "esc": kVK_Escape,
        "backspace": kVK_Delete, "delete": kVK_Delete,
        "forwarddelete": kVK_ForwardDelete,
        "up": kVK_UpArrow, "down": kVK_DownArrow,
        "left": kVK_LeftArrow, "right": kVK_RightArrow,
        "-": kVK_ANSI_Minus, "=": kVK_ANSI_Equal,
        "[": kVK_ANSI_LeftBracket, "]": kVK_ANSI_RightBracket,
        ";": kVK_ANSI_Semicolon, "'": kVK_ANSI_Quote,
        ",": kVK_ANSI_Comma, ".": kVK_ANSI_Period,
        "/": kVK_ANSI_Slash, "\\": kVK_ANSI_Backslash, "`": kVK_ANSI_Grave,
    ]

    private static let modifierFlags: [String: Int] = [
        "cmd": cmdKey, "command": cmdKey, "meta": cmdKey,
        "ctrl": controlKey, "control": controlKey,
        "alt": optionKey, "option": optionKey, "opt": optionKey,
        "shift": shiftKey,
    ]

    private static let modifierSymbols: [Int: String] = [
        controlKey: "⌃", optionKey: "⌥", shiftKey: "⇧", cmdKey: "⌘",
    ]

    static func parse(_ spec: String) -> Parsed? {
        let parts = spec.lowercased()
            .split(separator: "+", omittingEmptySubsequences: true)
            .map { $0.trimmingCharacters(in: .whitespaces) }
        guard let keyName = parts.last, let code = keyCodes[keyName] else { return nil }

        var mods = 0
        for m in parts.dropLast() {
            guard let flag = modifierFlags[m] else { return nil }
            mods |= flag
        }

        // Display string in the usual macOS order: ⌃ ⌥ ⇧ ⌘ + key.
        var display = ""
        for flag in [controlKey, optionKey, shiftKey, cmdKey] where mods & flag != 0 {
            display += modifierSymbols[flag]!
        }
        display += keyName.count == 1 ? keyName.uppercased() : keyName.capitalized

        return Parsed(keyCode: UInt32(code), modifiers: UInt32(mods), display: display)
    }
}

/// Registers system-wide hotkeys through Carbon. Works regardless of which app is frontmost
/// and takes priority over that app's own shortcuts.
final class HotKeyCenter {
    static let shared = HotKeyCenter()

    fileprivate var handlers: [UInt32: () -> Void] = [:]
    private var refs: [EventHotKeyRef] = []
    private var nextID: UInt32 = 1
    private var handlerInstalled = false

    private init() {}

    /// Returns the human-readable form of the shortcut on success, nil if the spec is invalid
    /// or the OS refused it (usually because something else already owns that combination).
    @discardableResult
    func register(_ spec: String, _ handler: @escaping () -> Void) -> String? {
        let trimmed = spec.trimmingCharacters(in: .whitespaces)
        guard !trimmed.isEmpty, let parsed = HotKeyParser.parse(trimmed) else {
            if !trimmed.isEmpty { NSLog("ClipStack: cannot parse hotkey '%@'", spec) }
            return nil
        }
        installHandlerIfNeeded()

        let id = nextID
        nextID += 1
        var ref: EventHotKeyRef?
        let hotKeyID = EventHotKeyID(signature: fourCharCode("CLSP"), id: id)
        let status = RegisterEventHotKey(parsed.keyCode, parsed.modifiers, hotKeyID,
                                         GetApplicationEventTarget(), 0, &ref)
        guard status == noErr, let r = ref else {
            NSLog("ClipStack: could not register hotkey '%@' (status %d)", spec, status)
            return nil
        }
        refs.append(r)
        handlers[id] = handler
        return parsed.display
    }

    func unregisterAll() {
        refs.forEach { UnregisterEventHotKey($0) }
        refs.removeAll()
        handlers.removeAll()
    }

    private func installHandlerIfNeeded() {
        guard !handlerInstalled else { return }
        handlerInstalled = true
        var spec = EventTypeSpec(eventClass: OSType(kEventClassKeyboard),
                                 eventKind: UInt32(kEventHotKeyPressed))
        InstallEventHandler(GetApplicationEventTarget(), { _, event, _ -> OSStatus in
            var hotKeyID = EventHotKeyID()
            GetEventParameter(event, EventParamName(kEventParamDirectObject),
                              EventParamType(typeEventHotKeyID), nil,
                              MemoryLayout<EventHotKeyID>.size, nil, &hotKeyID)
            HotKeyCenter.shared.handlers[hotKeyID.id]?()
            return noErr
        }, 1, &spec, nil, nil)
    }
}

private func fourCharCode(_ s: String) -> OSType {
    s.utf8.reduce(0) { ($0 << 8) + OSType($1) }
}
