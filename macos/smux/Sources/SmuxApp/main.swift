import AppKit
import libghostty

// ===================================================================
// smux native terminal — libghostty + AppKit + Korean IME
// ===================================================================

// MARK: - Initialize libghostty (MUST be first)

let initResult = ghostty_init(UInt(CommandLine.argc), CommandLine.unsafeArgv)
guard initResult == GHOSTTY_SUCCESS else {
    fatalError("ghostty_init failed: \(initResult)")
}

// MARK: - Runtime Callbacks

private let wakeupCb: @convention(c) (UnsafeMutableRawPointer?) -> Void = { _ in
    DispatchQueue.main.async {
        for w in NSApplication.shared.windows {
            w.contentView?.setNeedsDisplay(w.contentView?.bounds ?? .zero)
        }
    }
}

private let actionCb: @convention(c) (ghostty_app_t?, ghostty_target_s, ghostty_action_s) -> Bool = { _, _, _ in false }
private let readCb: @convention(c) (UnsafeMutableRawPointer?, ghostty_clipboard_e, UnsafeMutableRawPointer?) -> Bool = { _, _, _ in false }
private let confirmCb: @convention(c) (UnsafeMutableRawPointer?, UnsafePointer<CChar>?, UnsafeMutableRawPointer?, ghostty_clipboard_request_e) -> Void = { _, _, _, _ in }
private let writeCb: @convention(c) (UnsafeMutableRawPointer?, ghostty_clipboard_e, UnsafePointer<ghostty_clipboard_content_s>?, Int, Bool) -> Void = { _, _, _, _, _ in }
private let closeCb: @convention(c) (UnsafeMutableRawPointer?, Bool) -> Void = { _, _ in
    NSApplication.shared.terminate(nil)
}

// MARK: - App Delegate

class AppDelegate: NSObject, NSApplicationDelegate {
    var window: NSWindow!
    var ghosttyApp: ghostty_app_t?
    var terminalView: GhosttyTerminalView?

    func applicationDidFinishLaunching(_ notification: Notification) {
        window = NSWindow(
            contentRect: NSRect(x: 100, y: 100, width: 900, height: 600),
            styleMask: [.titled, .closable, .resizable, .miniaturizable],
            backing: .buffered,
            defer: false
        )
        window.title = "smux"
        window.backgroundColor = .black

        // Config
        guard let config = ghostty_config_new() else { fatalError("config failed") }
        ghostty_config_load_default_files(config)
        ghostty_config_finalize(config)

        // Runtime
        var rt = ghostty_runtime_config_s()
        rt.userdata = nil
        rt.supports_selection_clipboard = false
        rt.wakeup_cb = wakeupCb
        rt.action_cb = actionCb
        rt.read_clipboard_cb = readCb
        rt.confirm_read_clipboard_cb = confirmCb
        rt.write_clipboard_cb = writeCb
        rt.close_surface_cb = closeCb

        // App
        guard let gApp = ghostty_app_new(&rt, config) else {
            ghostty_config_free(config)
            fatalError("ghostty_app_new failed")
        }
        ghostty_config_free(config)
        self.ghosttyApp = gApp

        let info = ghostty_info()
        let ver = info.version != nil ? String(cString: info.version) : "?"
        print("✅ smux — ghostty \(ver)")

        // Check daemon connection
        let ipc = SmuxIpcClient()
        if ipc.isDaemonRunning {
            print("✅ daemon connected")
            window.title = "smux — daemon ●"
        } else {
            print("⚠️ daemon not running (smux daemon start)")
            window.title = "smux — daemon ○"
        }

        // Terminal view
        let termView = GhosttyTerminalView(frame: window.contentView!.bounds, app: gApp)
        termView.autoresizingMask = [.width, .height]
        window.contentView?.addSubview(termView)
        self.terminalView = termView

        // CRITICAL: activate the app so it can receive keyboard events.
        // Without this, CLI-launched apps stay in background and keyDown never fires.
        NSApp.setActivationPolicy(.regular)
        NSApp.activate(ignoringOtherApps: true)

        window.makeFirstResponder(termView)
        window.makeKeyAndOrderFront(nil)

        // Tick timer — only when needed, invalidated on quit
        tickTimer = Timer.scheduledTimer(withTimeInterval: 1.0/120.0, repeats: true) { [weak self] _ in
            guard let a = self?.ghosttyApp else { return }
            ghostty_app_tick(a)
        }
    }

    var tickTimer: Timer?

    func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool { true }

    func applicationWillTerminate(_ notification: Notification) {
        tickTimer?.invalidate()
        tickTimer = nil
        terminalView = nil
        if let a = ghosttyApp {
            ghostty_app_free(a)
            ghosttyApp = nil
        }
    }
}

// MARK: - Main

let nsApp = NSApplication.shared
let delegate = AppDelegate()
nsApp.delegate = delegate
nsApp.run()
