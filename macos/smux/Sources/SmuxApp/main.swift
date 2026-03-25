import AppKit
import libghostty

// ===================================================================
// smux libghostty PoC — Phase 3: full terminal surface with IME
// ===================================================================

// MARK: - Runtime Callbacks

private let wakeupCallback: @convention(c) (UnsafeMutableRawPointer?) -> Void = { _ in
    DispatchQueue.main.async {
        NSApplication.shared.windows.first?.contentView?.setNeedsDisplay(
            NSApplication.shared.windows.first?.contentView?.bounds ?? .zero
        )
    }
}

private let actionCallback: @convention(c) (ghostty_app_t?, ghostty_target_s, ghostty_action_s) -> Bool = { _, _, action in
    print("[smux] action: tag=\(action.tag)")
    return false
}

private let readClipboardCallback: @convention(c) (UnsafeMutableRawPointer?, ghostty_clipboard_e, UnsafeMutableRawPointer?) -> Bool = { _, _, _ in
    false
}

private let confirmReadClipboardCallback: @convention(c) (UnsafeMutableRawPointer?, UnsafePointer<CChar>?, UnsafeMutableRawPointer?, ghostty_clipboard_request_e) -> Void = { _, _, _, _ in
}

private let writeClipboardCallback: @convention(c) (UnsafeMutableRawPointer?, ghostty_clipboard_e, UnsafePointer<ghostty_clipboard_content_s>?, Int, Bool) -> Void = { _, _, _, _, _ in
}

private let closeSurfaceCallback: @convention(c) (UnsafeMutableRawPointer?, Bool) -> Void = { _, _ in
    NSApplication.shared.terminate(nil)
}

// MARK: - App Delegate

class AppDelegate: NSObject, NSApplicationDelegate {
    var window: NSWindow!
    var ghosttyApp: ghostty_app_t?
    var terminalView: GhosttyTerminalView?

    func applicationDidFinishLaunching(_ notification: Notification) {
        // Create window
        window = NSWindow(
            contentRect: NSRect(x: 100, y: 100, width: 900, height: 600),
            styleMask: [.titled, .closable, .resizable, .miniaturizable],
            backing: .buffered,
            defer: false
        )
        window.title = "smux"
        window.backgroundColor = .black

        // Create config
        guard let config = ghostty_config_new() else {
            fatalError("ghostty_config_new failed")
        }
        ghostty_config_load_default_files(config)
        ghostty_config_finalize(config)

        // Create runtime
        var runtimeConfig = ghostty_runtime_config_s()
        runtimeConfig.userdata = nil
        runtimeConfig.supports_selection_clipboard = false
        runtimeConfig.wakeup_cb = wakeupCallback
        runtimeConfig.action_cb = actionCallback
        runtimeConfig.read_clipboard_cb = readClipboardCallback
        runtimeConfig.confirm_read_clipboard_cb = confirmReadClipboardCallback
        runtimeConfig.write_clipboard_cb = writeClipboardCallback
        runtimeConfig.close_surface_cb = closeSurfaceCallback

        // Create ghostty app
        guard let app = ghostty_app_new(&runtimeConfig, config) else {
            ghostty_config_free(config)
            fatalError("ghostty_app_new failed")
        }
        ghostty_config_free(config)
        self.ghosttyApp = app

        let info = ghostty_info()
        let version = info.version != nil ? String(cString: info.version) : "unknown"
        print("✅ ghostty app created (version: \(version))")

        // Create terminal view filling the window
        let termView = GhosttyTerminalView(
            frame: window.contentView!.bounds,
            app: app
        )
        termView.autoresizingMask = [.width, .height]
        window.contentView?.addSubview(termView)
        self.terminalView = termView

        // Make terminal first responder for keyboard input
        window.makeFirstResponder(termView)
        window.makeKeyAndOrderFront(nil)

        // Start the render/tick timer
        Timer.scheduledTimer(withTimeInterval: 1.0 / 60.0, repeats: true) { [weak self] _ in
            guard let gApp = self?.ghosttyApp else { return }
            ghostty_app_tick(gApp)
            DispatchQueue.main.async {
                self?.terminalView?.setNeedsDisplay(self?.terminalView?.bounds ?? .zero)
            }
        }

        print("✅ terminal surface created — type to test Korean IME!")
    }

    func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool {
        true
    }

    func applicationWillTerminate(_ notification: Notification) {
        terminalView = nil
        if let app = ghosttyApp {
            ghostty_app_free(app)
        }
    }
}

// MARK: - Main

let app = NSApplication.shared
let delegate = AppDelegate()
app.delegate = delegate
app.run()
