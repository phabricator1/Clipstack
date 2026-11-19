import Foundation

/// Lightweight logger: writes to the unified log (Console.app) and appends to
/// ~/Library/Logs/ClipStack.log so problems can be inspected with `tail -f`.
enum Log {
    static let fileURL: URL = FileManager.default.homeDirectoryForCurrentUser
        .appendingPathComponent("Library/Logs/ClipStack.log")

    private static let formatter: DateFormatter = {
        let f = DateFormatter()
        f.dateFormat = "yyyy-MM-dd HH:mm:ss.SSS"
        return f
    }()

    private static let queue = DispatchQueue(label: "clipstack.log")

    static func info(_ message: String) {
        NSLog("ClipStack: %@", message)
        let line = "\(formatter.string(from: Date())) \(message)\n"
        queue.async {
            guard let data = line.data(using: .utf8) else { return }
            if let handle = try? FileHandle(forWritingTo: fileURL) {
                defer { try? handle.close() }
                _ = try? handle.seekToEnd()
                try? handle.write(contentsOf: data)
            } else {
                try? data.write(to: fileURL)
            }
        }
    }
}
