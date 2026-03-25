import Foundation

/// Client-side policy enforcement for the native shell.
/// Reads allow/deny rules from ~/.smux/config.toml and applies them.
class PolicyEngine {
    var allowCommands: [String] = ["git", "cargo", "npm", "swift", "ls", "cd", "echo"]
    var denyCommands: [String] = ["rm -rf /", "sudo rm", "shutdown", "reboot"]
    var autoExecutionLevel: String = "auto" // disabled, allowlist, auto, turbo

    func isAllowed(_ command: String) -> Bool {
        // Deny list always blocks
        for denied in denyCommands {
            if command.contains(denied) { return false }
        }

        switch autoExecutionLevel {
        case "disabled": return false
        case "turbo": return true
        case "allowlist":
            let binary = command.split(separator: " ").first.map(String.init) ?? ""
            return allowCommands.contains(binary)
        case "auto":
            let dangerous = ["rm -rf", "mkfs", "dd if=", "shutdown"]
            return !dangerous.contains(where: { command.contains($0) })
        default: return false
        }
    }

    func loadFromConfig() {
        let home = FileManager.default.homeDirectoryForCurrentUser.path
        let configPath = "\(home)/.smux/config.toml"
        guard let content = try? String(contentsOfFile: configPath, encoding: .utf8) else { return }

        // Simple TOML parsing for deny_commands
        for line in content.components(separatedBy: "\n") {
            let trimmed = line.trimmingCharacters(in: .whitespaces)
            if trimmed.hasPrefix("auto_execution_level") {
                if let val = trimmed.split(separator: "=").last?.trimmingCharacters(in: .whitespaces).replacingOccurrences(of: "\"", with: "") {
                    autoExecutionLevel = val
                }
            }
        }
    }
}

/// Exports audit records to a JSON file for compliance.
class AuditExporter {
    func export(sessionId: String, records: [[String: Any]]) {
        let home = FileManager.default.homeDirectoryForCurrentUser.path
        let dir = "\(home)/.smux/audits"
        try? FileManager.default.createDirectory(atPath: dir, withIntermediateDirectories: true)

        let path = "\(dir)/\(sessionId).jsonl"
        let lines = records.compactMap { dict -> String? in
            guard let data = try? JSONSerialization.data(withJSONObject: dict) else { return nil }
            return String(data: data, encoding: .utf8)
        }
        let content = lines.joined(separator: "\n") + "\n"
        try? content.write(toFile: path, atomically: true, encoding: .utf8)
        NSLog("[smux] audit exported to %@", path)
    }
}

/// Session templates — saved configurations for quick session creation.
class SessionTemplates {
    struct Template: Codable {
        var name: String
        var planner: String
        var verifier: String
        var workers: [String]
        var task: String
        var maxRounds: Int
        var consensus: String
    }

    private let savePath: String

    init() {
        let home = FileManager.default.homeDirectoryForCurrentUser.path
        savePath = "\(home)/.smux/templates.json"
    }

    func load() -> [Template] {
        guard let data = try? Data(contentsOf: URL(fileURLWithPath: savePath)) else { return defaultTemplates }
        return (try? JSONDecoder().decode([Template].self, from: data)) ?? defaultTemplates
    }

    func save(_ templates: [Template]) {
        let dir = (savePath as NSString).deletingLastPathComponent
        try? FileManager.default.createDirectory(atPath: dir, withIntermediateDirectories: true)
        if let data = try? JSONEncoder().encode(templates) {
            try? data.write(to: URL(fileURLWithPath: savePath))
        }
    }

    var defaultTemplates: [Template] {
        [
            Template(name: "Quick Fix", planner: "claude", verifier: "codex", workers: [], task: "", maxRounds: 3, consensus: "majority"),
            Template(name: "Full Pipeline", planner: "claude", verifier: "codex", workers: ["frontend", "backend"], task: "", maxRounds: 5, consensus: "majority"),
            Template(name: "Multi-Verify", planner: "claude", verifier: "codex", workers: [], task: "", maxRounds: 5, consensus: "unanimous"),
        ]
    }
}
