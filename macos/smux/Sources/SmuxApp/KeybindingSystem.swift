import AppKit

/// Native keybinding system — configurable shortcuts.
class KeybindingSystem {
    struct Binding {
        let action: String
        let key: String
        let modifiers: NSEvent.ModifierFlags
        let handler: () -> Void
    }

    private var bindings: [Binding] = []

    func register(_ action: String, key: String, modifiers: NSEvent.ModifierFlags, handler: @escaping () -> Void) {
        bindings.append(Binding(action: action, key: key, modifiers: modifiers, handler: handler))
    }

    func handle(event: NSEvent) -> Bool {
        guard let chars = event.charactersIgnoringModifiers else { return false }
        for binding in bindings {
            if chars == binding.key && event.modifierFlags.intersection([.command, .control, .option, .shift]) == binding.modifiers {
                binding.handler()
                return true
            }
        }
        return false
    }

    func setupDefaults(controller: WorkspaceWindowController) {
        register("New Tab", key: "t", modifiers: .command) { controller.newTab() }
        register("Split Vertical", key: "d", modifiers: .command) { controller.splitVertical() }
        register("Split Horizontal", key: "d", modifiers: [.command, .shift]) { controller.splitHorizontal() }
        register("Find", key: "f", modifiers: .command) { controller.toggleSearch() }
        register("Inspector", key: "i", modifiers: .command) { controller.toggleInspector() }
        register("Command Palette", key: "p", modifiers: .command) { controller.showCommandPalette() }
    }

    var allBindings: [(action: String, shortcut: String)] {
        bindings.map { b in
            var parts: [String] = []
            if b.modifiers.contains(.command) { parts.append("⌘") }
            if b.modifiers.contains(.shift) { parts.append("⇧") }
            if b.modifiers.contains(.control) { parts.append("⌃") }
            if b.modifiers.contains(.option) { parts.append("⌥") }
            parts.append(b.key.uppercased())
            return (b.action, parts.joined())
        }
    }
}

/// Launch configurations — saved workspace setups.
class LaunchConfigurations {
    struct Config: Codable {
        var name: String
        var panes: [PaneConfig]
        var layout: String // "single", "split-v", "split-h", "grid"
    }

    struct PaneConfig: Codable {
        var command: String
        var workingDirectory: String
        var name: String
    }

    private let savePath: String

    init() {
        let home = FileManager.default.homeDirectoryForCurrentUser.path
        savePath = "\(home)/.smux/launch-configs.json"
    }

    func load() -> [Config] {
        guard let data = try? Data(contentsOf: URL(fileURLWithPath: savePath)) else { return defaultConfigs }
        return (try? JSONDecoder().decode([Config].self, from: data)) ?? defaultConfigs
    }

    func save(_ configs: [Config]) {
        let dir = (savePath as NSString).deletingLastPathComponent
        try? FileManager.default.createDirectory(atPath: dir, withIntermediateDirectories: true)
        if let data = try? JSONEncoder().encode(configs) {
            try? data.write(to: URL(fileURLWithPath: savePath))
        }
    }

    var defaultConfigs: [Config] {
        [
            Config(name: "Dev Server", panes: [
                PaneConfig(command: "npm run dev", workingDirectory: ".", name: "Frontend"),
                PaneConfig(command: "", workingDirectory: ".", name: "Terminal"),
            ], layout: "split-v"),
            Config(name: "AI Ping-Pong", panes: [
                PaneConfig(command: "claude", workingDirectory: ".", name: "Planner"),
                PaneConfig(command: "codex", workingDirectory: ".", name: "Verifier"),
            ], layout: "split-v"),
        ]
    }
}
