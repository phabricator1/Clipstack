import Foundation

struct ClipItem: Codable, Identifiable {
    let id: UUID
    var text: String
    var copiedAt: Date
    var pinned: Bool
    /// Rich flavours captured alongside the text, written back on paste so fonts,
    /// colours and links survive. Nil when the copy was plain text.
    var rtf: Data?
    var html: String?

    var isRich: Bool { rtf != nil || html != nil }
}

/// Holds everything ClipStack knows about:
///  - `history`: every text copied, newest first, up to `maxHistory` (pinned items never evicted)
///  - `stackIDs`: the ids not yet popped, oldest first (top of stack = last)
/// Popping removes an id from the stack but leaves the item in history.
final class ClipStore {
    private(set) var history: [ClipItem] = []
    private(set) var stackIDs: [UUID] = []

    var maxStack: Int
    var maxHistory: Int
    var persist: Bool

    private var saveWork: DispatchWorkItem?

    static var fileURL: URL {
        Config.directory.appendingPathComponent("history.json")
    }

    init(maxStack: Int, maxHistory: Int, persist: Bool) {
        self.maxStack = maxStack
        self.maxHistory = maxHistory
        self.persist = persist
        load()
    }

    // MARK: - Queries

    var stackCount: Int { stackIDs.count }

    /// Items still on the stack, top (newest) first.
    var stackItems: [ClipItem] {
        stackIDs.reversed().compactMap { item(id: $0) }
    }

    var pinnedItems: [ClipItem] { history.filter { $0.pinned } }
    var unpinnedHistory: [ClipItem] { history.filter { !$0.pinned } }

    func item(id: UUID) -> ClipItem? {
        history.first { $0.id == id }
    }

    func isOnStack(_ id: UUID) -> Bool { stackIDs.contains(id) }

    // MARK: - Mutations

    /// Records a new copy: adds to history (or moves an identical text to the top)
    /// and pushes it onto the stack.
    @discardableResult
    func push(_ text: String, rtf: Data? = nil, html: String? = nil) -> ClipItem {
        let item: ClipItem
        if let idx = history.firstIndex(where: { $0.text == text }) {
            var existing = history.remove(at: idx)
            existing.copiedAt = Date()
            // A fresh copy of the same text may carry (different) formatting.
            if rtf != nil || html != nil {
                existing.rtf = rtf
                existing.html = html
            }
            item = existing
        } else {
            item = ClipItem(id: UUID(), text: text, copiedAt: Date(), pinned: false, rtf: rtf, html: html)
        }
        history.insert(item, at: 0)

        stackIDs.removeAll { $0 == item.id }
        stackIDs.append(item.id)

        trim()
        scheduleSave()
        return item
    }

    func popNewest() -> ClipItem? {
        guard let id = stackIDs.popLast() else { return nil }
        scheduleSave()
        return item(id: id)
    }

    func popOldest() -> ClipItem? {
        guard !stackIDs.isEmpty else { return nil }
        let id = stackIDs.removeFirst()
        scheduleSave()
        return item(id: id)
    }

    func pop(id: UUID) -> ClipItem? {
        guard let idx = stackIDs.firstIndex(of: id) else { return nil }
        stackIDs.remove(at: idx)
        scheduleSave()
        return item(id: id)
    }

    func togglePin(id: UUID) {
        guard let idx = history.firstIndex(where: { $0.id == id }) else { return }
        history[idx].pinned.toggle()
        scheduleSave()
    }

    func remove(id: UUID) {
        history.removeAll { $0.id == id }
        stackIDs.removeAll { $0 == id }
        scheduleSave()
    }

    func clearStack() {
        stackIDs.removeAll()
        scheduleSave()
    }

    /// Drops all unpinned history (and therefore the whole stack).
    func clearHistory() {
        history.removeAll { !$0.pinned }
        stackIDs.removeAll()
        scheduleSave()
    }

    private func trim() {
        if stackIDs.count > maxStack {
            stackIDs.removeFirst(stackIDs.count - maxStack)
        }
        // Evict oldest unpinned items that are not on the stack first,
        // then oldest unpinned items on the stack if still over the limit.
        var over = history.count - maxHistory
        guard over > 0 else { return }
        for pass in 0..<2 where over > 0 {
            for idx in stride(from: history.count - 1, through: 0, by: -1) where over > 0 {
                let it = history[idx]
                if it.pinned { continue }
                if pass == 0 && isOnStack(it.id) { continue }
                history.remove(at: idx)
                stackIDs.removeAll { $0 == it.id }
                over -= 1
            }
        }
    }

    // MARK: - Persistence

    private struct Snapshot: Codable {
        var history: [ClipItem]
        var stackIDs: [UUID]
    }

    private func load() {
        guard let data = try? Data(contentsOf: ClipStore.fileURL),
              let snap = try? JSONDecoder().decode(Snapshot.self, from: data)
        else { return }
        if persist {
            history = snap.history
            stackIDs = snap.stackIDs.filter { id in history.contains { $0.id == id } }
        } else {
            history = snap.history.filter { $0.pinned }
            stackIDs = []
        }
        trim()
    }

    private func scheduleSave() {
        saveWork?.cancel()
        let work = DispatchWorkItem { [weak self] in self?.saveNow() }
        saveWork = work
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.5, execute: work)
    }

    func saveNow() {
        saveWork?.cancel()
        saveWork = nil
        let snap = persist
            ? Snapshot(history: history, stackIDs: stackIDs)
            : Snapshot(history: pinnedItems, stackIDs: [])
        do {
            try FileManager.default.createDirectory(at: Config.directory, withIntermediateDirectories: true)
            let enc = JSONEncoder()
            enc.outputFormatting = [.prettyPrinted]
            try enc.encode(snap).write(to: ClipStore.fileURL, options: .atomic)
        } catch {
            Log.info("could not save history: \(error)")
        }
    }
}
