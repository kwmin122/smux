import AppKit

/// Represents a session from the daemon for display in the native shell.
struct SmuxSession {
    let id: String
    let task: String
    let planner: String
    let verifier: String
    let currentRound: Int
    let status: SessionStatus

    enum SessionStatus: String {
        case running = "Running"
        case completed = "Completed"
        case failed = "Failed"
        case paused = "Paused"
    }
}

/// Represents the current pipeline stage.
struct PipelineState {
    let currentStage: String
    let stageIndex: Int
    let totalStages: Int
    let approval: String

    var progress: Double {
        guard totalStages > 0 else { return 0 }
        return Double(stageIndex) / Double(totalStages)
    }

    var displayText: String {
        "\(currentStage) (\(stageIndex + 1)/\(totalStages))"
    }
}

/// Mission control state — tracks what's happening across sessions.
class MissionControlState {
    var sessions: [SmuxSession] = []
    var pipelineState: PipelineState?
    var isAutoMode: Bool = true
    var pendingApproval: Bool = false
    var findingsCount: Int = 0

    /// Fetch sessions from daemon via IPC.
    func refresh(client: SmuxIpcClient) {
        do {
            try client.connect()
            let rawSessions = try client.listSessions()
            sessions = rawSessions.map { dict in
                SmuxSession(
                    id: dict["id"] as? String ?? "",
                    task: dict["task"] as? String ?? "",
                    planner: dict["planner"] as? String ?? "",
                    verifier: dict["verifier"] as? String ?? "",
                    currentRound: dict["current_round"] as? Int ?? 0,
                    status: SmuxSession.SessionStatus(rawValue: dict["status"] as? String ?? "") ?? .running
                )
            }
            client.disconnect()
        } catch {
            // Daemon not available — clear sessions
            sessions = []
        }
    }

    /// Actions
    func approve() { pendingApproval = false }
    func pause() { isAutoMode = false }
    func resume() { isAutoMode = true }
}

// MARK: - Workspace Model (cmux-style vertical tabs)

struct Workspace {
    let id: String
    var name: String
    var icon: String
    var color: NSColor
    var status: WorkspaceStatus
    var statusText: String
    var gitBranches: [GitBranchInfo]
    var ports: [Int]
    var prAlerts: [PRAlert]
    var sessions: [SmuxSession]
    var needsAttention: Bool
    var unreadCount: Int

    enum WorkspaceStatus {
        case running, idle, error

        var dotColor: NSColor {
            switch self {
            case .running: return .systemGreen
            case .idle: return .secondaryLabelColor
            case .error: return .systemRed
            }
        }
        var label: String {
            switch self {
            case .running: return "Running"
            case .idle: return "Idle"
            case .error: return "Error"
            }
        }
    }
}

struct GitBranchInfo {
    let name: String
    let path: String
    let hasChanges: Bool

    var displayText: String {
        let suffix = hasChanges ? "*" : ""
        let shortPath = (path as NSString).abbreviatingWithTildeInPath
        return "\(name)\(suffix) · \(shortPath)"
    }
}

struct PRAlert {
    let number: Int
    let title: String
    var isUnread: Bool

    var displayText: String {
        "📎 PR #\(number) 알림"
    }
}

struct SmuxNotification {
    let id: String
    let title: String
    let body: String
    let source: String
    let timestamp: Date
    var isRead: Bool
}

// MARK: - Workspace Detection Helpers

enum WorkspaceDetector {
    static func detectGitBranches(at directory: String = FileManager.default.currentDirectoryPath) -> [GitBranchInfo] {
        let task = Process()
        task.executableURL = URL(fileURLWithPath: "/usr/bin/git")
        task.arguments = ["branch", "--format=%(refname:short)"]
        task.currentDirectoryURL = URL(fileURLWithPath: directory)
        let pipe = Pipe()
        task.standardOutput = pipe
        task.standardError = Pipe()
        do {
            try task.run()
            task.waitUntilExit()
            let data = pipe.fileHandleForReading.readDataToEndOfFile()
            if let output = String(data: data, encoding: .utf8) {
                return output.split(separator: "\n").prefix(5).map {
                    GitBranchInfo(name: String($0), path: directory, hasChanges: false)
                }
            }
        } catch {}
        return []
    }

    static func detectCurrentBranch(at directory: String = FileManager.default.currentDirectoryPath) -> String? {
        let task = Process()
        task.executableURL = URL(fileURLWithPath: "/usr/bin/git")
        task.arguments = ["rev-parse", "--abbrev-ref", "HEAD"]
        task.currentDirectoryURL = URL(fileURLWithPath: directory)
        let pipe = Pipe()
        task.standardOutput = pipe
        task.standardError = Pipe()
        do {
            try task.run()
            task.waitUntilExit()
            let data = pipe.fileHandleForReading.readDataToEndOfFile()
            return String(data: data, encoding: .utf8)?.trimmingCharacters(in: .whitespacesAndNewlines)
        } catch { return nil }
    }

    static func detectListeningPorts() -> [Int] {
        var found: [Int] = []
        for port: UInt16 in [3000, 3001, 3004, 3005, 5173, 6379, 8000, 8080, 8001] {
            let fd = socket(AF_INET, SOCK_STREAM, 0)
            guard fd >= 0 else { continue }
            defer { Darwin.close(fd) }
            var addr = sockaddr_in()
            addr.sin_family = sa_family_t(AF_INET)
            addr.sin_port = port.bigEndian
            addr.sin_addr.s_addr = inet_addr("127.0.0.1")
            let result = withUnsafePointer(to: &addr) { ptr in
                ptr.withMemoryRebound(to: sockaddr.self, capacity: 1) {
                    Darwin.connect(fd, $0, socklen_t(MemoryLayout<sockaddr_in>.size))
                }
            }
            if result == 0 { found.append(Int(port)) }
        }
        return found
    }

    /// Build default workspaces synchronously (fast path — used for initial display).
    static func buildDefaultWorkspaces(from sessions: [SmuxSession]) -> [Workspace] {
        let cwd = FileManager.default.currentDirectoryPath
        let hasRunning = sessions.contains { $0.status == .running }
        let hasFailed = sessions.contains { $0.status == .failed }

        // Return immediately with empty branches/ports — populate async
        return [Workspace(
            id: "main",
            name: (cwd as NSString).lastPathComponent,
            icon: "folder.fill",
            color: .systemBlue,
            status: hasRunning ? .running : .idle,
            statusText: sessions.first?.task ?? "",
            gitBranches: [],
            ports: [],
            prAlerts: [],
            sessions: sessions,
            needsAttention: hasFailed,
            unreadCount: 0
        )]
    }

    /// Populate git branches and ports asynchronously, then call the update handler on main thread.
    static func populateWorkspaceDetails(_ workspace: Workspace,
                                          completion: @escaping (Workspace) -> Void) {
        DispatchQueue.global(qos: .userInitiated).async {
            let cwd = FileManager.default.currentDirectoryPath
            let branches = detectGitBranches(at: cwd)
            let ports = detectListeningPorts()
            var updated = workspace
            updated.gitBranches = branches
            updated.ports = ports
            DispatchQueue.main.async {
                completion(updated)
            }
        }
    }
}
