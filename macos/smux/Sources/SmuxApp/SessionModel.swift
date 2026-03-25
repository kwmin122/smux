import Foundation

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
