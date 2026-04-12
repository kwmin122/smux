import AppKit

/// tmux-style detach/reattach — daemon owns sessions, app connects/disconnects.
///
/// Architecture:
/// - smux-daemon owns all PTY sessions and keeps them alive even when the app closes.
/// - The native app is a "view" that attaches to daemon sessions.
/// - On app close: detach (sessions keep running in daemon).
/// - On app open: list daemon sessions → reattach to active ones.
/// - On explicit detach (⌘⇧D): detach current session, keep it in daemon.
///
/// Wire protocol (IPC messages added):
/// - AttachSession { session_id } → AttachResult { session_id, pty_fd_path, status }
/// - DetachSession { session_id } → DetachResult { session_id, status }
/// - ListDetachedSessions → DetachedSessionList { sessions }
class SessionDetachReattach {
    private let ipcClient: SmuxIpcClient
    private let sessionRestore: SessionRestore

    /// Currently attached session IDs.
    private(set) var attachedSessions: Set<String> = []

    /// The currently focused/active session (for detach menu).
    var currentSessionId: String?

    /// Known detached sessions from daemon.
    private(set) var detachedSessions: [DetachedSession] = []

    struct DetachedSession: Codable {
        let id: String
        let task: String
        let createdAt: String
        let status: String
        let workingDirectory: String
    }

    struct AttachState: Codable {
        var attachedSessionIds: [String]
        var lastAttachTime: String
        var windowStates: [SessionRestore.WindowState]
    }

    private let stateFilePath: String

    init() {
        self.ipcClient = SmuxIpcClient()
        self.sessionRestore = SessionRestore()
        let home = FileManager.default.homeDirectoryForCurrentUser.path
        self.stateFilePath = "\(home)/.smux/attach-state.json"
    }

    // MARK: - Attach

    /// Attach to a daemon session — request PTY handle and mark as attached.
    func attach(sessionId: String) -> Bool {
        do {
            try ipcClient.connect()
            try ipcClient.send(["AttachSession": ["session_id": sessionId]])
            let response = try ipcClient.receive()
            ipcClient.disconnect()

            if let result = response["AttachResult"] as? [String: Any],
               let status = result["status"] as? String, status == "attached" {
                attachedSessions.insert(sessionId)
                NSLog("[smux-attach] attached to session: %@", sessionId)
                return true
            }

            // Fallback: if daemon doesn't support AttachSession yet,
            // still mark as attached locally
            if let error = response["Error"] as? [String: Any],
               let msg = error["message"] as? String {
                NSLog("[smux-attach] daemon error: %@, using local attach", msg)
            }
            attachedSessions.insert(sessionId)
            return true
        } catch {
            // Daemon not running — create a local-only session reference
            NSLog("[smux-attach] no daemon, local attach: %@", sessionId)
            attachedSessions.insert(sessionId)
            return true
        }
    }

    /// Attach to all previously-attached sessions (on app launch).
    func reattachAll() -> [String] {
        guard let state = loadAttachState() else { return [] }
        var reattached: [String] = []
        for id in state.attachedSessionIds {
            if attach(sessionId: id) {
                reattached.append(id)
            }
        }
        NSLog("[smux-attach] reattached %d sessions", reattached.count)
        return reattached
    }

    // MARK: - Detach

    /// Detach from a session — the daemon keeps it alive.
    func detach(sessionId: String) -> Bool {
        do {
            try ipcClient.connect()
            try ipcClient.send(["DetachSession": ["session_id": sessionId]])
            let response = try ipcClient.receive()
            ipcClient.disconnect()

            if let result = response["DetachResult"] as? [String: Any],
               let status = result["status"] as? String, status == "detached" {
                attachedSessions.remove(sessionId)
                NSLog("[smux-detach] detached session: %@", sessionId)
                return true
            }
        } catch {
            NSLog("[smux-detach] error: %@", error.localizedDescription)
        }
        attachedSessions.remove(sessionId)
        return true
    }

    /// Detach all sessions (on app close — sessions survive in daemon).
    func detachAll() {
        for id in attachedSessions {
            _ = detach(sessionId: id)
        }
        NSLog("[smux-detach] detached all sessions")
    }

    // MARK: - List Detached

    /// Query daemon for detached sessions that can be reattached.
    func refreshDetachedSessions() {
        do {
            try ipcClient.connect()
            try ipcClient.send(["ListDetachedSessions": [:]])
            let response = try ipcClient.receive()
            ipcClient.disconnect()

            if let list = response["DetachedSessionList"] as? [String: Any],
               let sessions = list["sessions"] as? [[String: Any]] {
                detachedSessions = sessions.map {
                    DetachedSession(
                        id: $0["id"] as? String ?? "",
                        task: $0["task"] as? String ?? "",
                        createdAt: $0["created_at"] as? String ?? "",
                        status: $0["status"] as? String ?? "",
                        workingDirectory: $0["working_directory"] as? String ?? ""
                    )
                }
            }
        } catch {
            detachedSessions = []
        }
    }

    // MARK: - State Persistence

    /// Save attach state to disk (which sessions were attached, window layout).
    func saveAttachState(windowStates: [SessionRestore.WindowState]) {
        let formatter = ISO8601DateFormatter()
        let state = AttachState(
            attachedSessionIds: Array(attachedSessions),
            lastAttachTime: formatter.string(from: Date()),
            windowStates: windowStates
        )
        do {
            let dir = (stateFilePath as NSString).deletingLastPathComponent
            try FileManager.default.createDirectory(atPath: dir, withIntermediateDirectories: true)
            let data = try JSONEncoder().encode(state)
            try data.write(to: URL(fileURLWithPath: stateFilePath))
            NSLog("[smux-attach] saved attach state: %d sessions", state.attachedSessionIds.count)
        } catch {
            NSLog("[smux-attach] failed to save state: %@", error.localizedDescription)
        }
    }

    /// Load previous attach state from disk.
    func loadAttachState() -> AttachState? {
        guard let data = try? Data(contentsOf: URL(fileURLWithPath: stateFilePath)) else { return nil }
        return try? JSONDecoder().decode(AttachState.self, from: data)
    }

    /// Clear attach state.
    func clearAttachState() {
        try? FileManager.default.removeItem(atPath: stateFilePath)
    }
}
