import Foundation

/// User configuration, stored as JSON at ~/.config/clipstack/config.json.
/// Every key is optional; missing keys fall back to the defaults below.
struct Config: Codable {
    /// Maximum number of items kept on the stack. Oldest are dropped first.
    var maxItems: Int = 5
    /// History circulates at this size: popped items stay here until they are the
    /// oldest and get auto-removed. Starred items never count and are never removed.
    var maxHistory: Int = 15
    /// Save history to ~/.config/clipstack/history.json so it survives restarts.
    /// Pinned items are always saved regardless of this setting.
    var persistHistory: Bool = true
    /// How often (seconds) to check the clipboard for a new copy.
    var pollInterval: Double = 0.2
    /// Show the item count next to the menubar icon.
    var showCountInMenubar: Bool = true
    /// Which end of the stack the main paste hotkey takes: "lifo" (newest first, default)
    /// or "fifo" (oldest first — paste in the order you copied). The second paste hotkey
    /// always takes the other end. Also switchable from the menu.
    var pasteOrder: String = "lifo"
    /// Global hotkeys. Format: modifiers joined by "+", then the key.
    /// Modifiers: cmd, ctrl, alt (or option), shift.
    /// Keys: a-z, 0-9, f1-f12, space, return, tab, escape, backspace, delete,
    ///       up, down, left, right, and the punctuation - = [ ] ; ' , . / \ `
    /// Set a hotkey to "" to disable it.
    var hotkeys: Hotkeys = Hotkeys()

    struct Hotkeys: Codable {
        /// Pop the most recently copied item and paste it (LIFO).
        var pasteNewest: String = "cmd+ctrl+v"
        /// Pop the oldest item and paste it (FIFO: paste in the order you copied).
        var pasteOldest: String = "cmd+ctrl+shift+v"
        /// Empty the stack.
        var clear: String = "cmd+ctrl+backspace"
        /// Open the menubar menu to see what's on the stack.
        var peek: String = "cmd+ctrl+l"

        init() {}

        init(from decoder: Decoder) throws {
            let c = try decoder.container(keyedBy: CodingKeys.self)
            pasteNewest = try c.decodeIfPresent(String.self, forKey: .pasteNewest) ?? pasteNewest
            pasteOldest = try c.decodeIfPresent(String.self, forKey: .pasteOldest) ?? pasteOldest
            clear       = try c.decodeIfPresent(String.self, forKey: .clear)       ?? clear
            peek        = try c.decodeIfPresent(String.self, forKey: .peek)        ?? peek
        }
    }

    init() {}

    init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        maxItems           = try c.decodeIfPresent(Int.self,     forKey: .maxItems)           ?? maxItems
        maxHistory         = try c.decodeIfPresent(Int.self,     forKey: .maxHistory)         ?? maxHistory
        persistHistory     = try c.decodeIfPresent(Bool.self,    forKey: .persistHistory)     ?? persistHistory
        pollInterval       = try c.decodeIfPresent(Double.self,  forKey: .pollInterval)       ?? pollInterval
        showCountInMenubar = try c.decodeIfPresent(Bool.self,    forKey: .showCountInMenubar) ?? showCountInMenubar
        pasteOrder         = try c.decodeIfPresent(String.self,  forKey: .pasteOrder)         ?? pasteOrder
        hotkeys            = try c.decodeIfPresent(Hotkeys.self, forKey: .hotkeys)            ?? hotkeys
    }

    // MARK: - Load / save

    static var directory: URL {
        FileManager.default.homeDirectoryForCurrentUser
            .appendingPathComponent(".config/clipstack", isDirectory: true)
    }

    static var fileURL: URL {
        directory.appendingPathComponent("config.json")
    }

    /// Loads the config file, writing a default one first if none exists.
    /// On a parse error, logs it and falls back to defaults so the app still runs.
    static func load() -> Config {
        let fm = FileManager.default
        if !fm.fileExists(atPath: fileURL.path) {
            let defaults = Config()
            defaults.save()
            return defaults
        }
        do {
            let data = try Data(contentsOf: fileURL)
            return try JSONDecoder().decode(Config.self, from: data)
        } catch {
            NSLog("ClipStack: could not read config (%@), using defaults", "\(error)")
            return Config()
        }
    }

    func save() {
        do {
            try FileManager.default.createDirectory(at: Config.directory,
                                                    withIntermediateDirectories: true)
            let enc = JSONEncoder()
            enc.outputFormatting = [.prettyPrinted, .sortedKeys]
            try enc.encode(self).write(to: Config.fileURL)
        } catch {
            NSLog("ClipStack: could not write config: %@", "\(error)")
        }
    }
}
