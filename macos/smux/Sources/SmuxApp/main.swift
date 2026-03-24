import AppKit
import libghostty

// Minimal libghostty PoC — Phase 2: test runtime API calls.
// Proves: config creation, app initialization, surface config.

class AppDelegate: NSObject, NSApplicationDelegate {
    var window: NSWindow!

    func applicationDidFinishLaunching(_ notification: Notification) {
        window = NSWindow(
            contentRect: NSRect(x: 200, y: 200, width: 900, height: 600),
            styleMask: [.titled, .closable, .resizable, .miniaturizable],
            backing: .buffered,
            defer: false
        )
        window.title = "smux — libghostty PoC"

        // Test 1: Create config
        let config = ghostty_config_new()
        let configOk = config != nil
        print("✅ ghostty_config_new: \(configOk ? "OK" : "FAIL")")

        if let config = config {
            // Load default config
            ghostty_config_load_default_files(config)
            ghostty_config_finalize(config)
            print("✅ ghostty_config_finalize: OK")

            // Test 2: Create surface config
            let surfaceConfig = ghostty_surface_config_new()
            print("✅ ghostty_surface_config_new: OK (backend: \(surfaceConfig.backend))")

            // Test 3: Get ghostty info
            let info = ghostty_info()
            let version = info.version != nil ? String(cString: info.version) : "unknown"
            print("✅ ghostty_info version: \(version)")

            // Show results in window
            let results = """
            smux — libghostty Runtime PoC

            ✅ ghostty_config_new: \(configOk ? "success" : "failed")
            ✅ ghostty_config_finalize: success
            ✅ ghostty_surface_config_new: success
            ✅ ghostty version: \(version)

            Next: create ghostty_app_t with runtime callbacks,
            then ghostty_surface_t with Metal rendering.
            """

            let label = NSTextField(wrappingLabelWithString: results)
            label.font = NSFont.monospacedSystemFont(ofSize: 13, weight: .regular)
            label.frame = NSRect(x: 40, y: 200, width: 820, height: 300)
            label.isEditable = false
            label.isBordered = false
            label.backgroundColor = .clear
            window.contentView?.addSubview(label)

            ghostty_config_free(config)
        }

        window.makeKeyAndOrderFront(nil)
    }

    func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool {
        true
    }
}

let app = NSApplication.shared
let delegate = AppDelegate()
app.delegate = delegate
app.run()
