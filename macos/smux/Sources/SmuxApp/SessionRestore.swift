import Foundation

/// Saves and restores workspace state across app launches.
class SessionRestore {
    private let savePath: String

    struct WorkspaceState: Codable {
        var windows: [WindowState]
        var activeWindowIndex: Int
    }

    struct WindowState: Codable {
        var tabs: [TabState]
        var activeTabIndex: Int
        var frame: FrameRect
    }

    struct TabState: Codable {
        var title: String
        var workingDirectory: String
        var splits: [SplitState]
    }

    struct SplitState: Codable {
        var direction: String // "vertical" or "horizontal"
        var ratio: Double
        var children: [SplitChild]
    }

    struct SplitChild: Codable {
        var type: String // "terminal" or "split"
        var workingDirectory: String?
        var splitIndex: Int? // index into splits array if type == "split"
    }

    struct FrameRect: Codable {
        var x: Double, y: Double, width: Double, height: Double
    }

    init() {
        let home = FileManager.default.homeDirectoryForCurrentUser.path
        savePath = "\(home)/.smux/workspace-state.json"
    }

    func save(state: WorkspaceState) {
        do {
            let dir = (savePath as NSString).deletingLastPathComponent
            try FileManager.default.createDirectory(atPath: dir, withIntermediateDirectories: true)
            let data = try JSONEncoder().encode(state)
            try data.write(to: URL(fileURLWithPath: savePath))
        } catch {
            NSLog("[smux] failed to save workspace state: %@", error.localizedDescription)
        }
    }

    func load() -> WorkspaceState? {
        guard let data = try? Data(contentsOf: URL(fileURLWithPath: savePath)) else { return nil }
        return try? JSONDecoder().decode(WorkspaceState.self, from: data)
    }

    func clear() {
        try? FileManager.default.removeItem(atPath: savePath)
    }
}
