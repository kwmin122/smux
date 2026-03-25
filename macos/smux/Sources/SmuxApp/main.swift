import AppKit
import libghostty

// ===================================================================
// smux native terminal — libghostty + AppKit
// ===================================================================

// Initialize libghostty
let initResult = ghostty_init(UInt(CommandLine.argc), CommandLine.unsafeArgv)
guard initResult == GHOSTTY_SUCCESS else { fatalError("ghostty_init: \(initResult)") }

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
    var ghosttyApp: ghostty_app_t?
    var workspaceController: WorkspaceWindowController?
    var tickTimer: Timer?

    func applicationDidFinishLaunching(_ notification: Notification) {
        // Config
        guard let config = ghostty_config_new() else { fatalError("config") }
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

        guard let gApp = ghostty_app_new(&rt, config) else {
            ghostty_config_free(config)
            fatalError("ghostty_app_new")
        }
        ghostty_config_free(config)
        self.ghosttyApp = gApp

        // Activate app (CRITICAL for keyboard input from CLI launch)
        NSApp.setActivationPolicy(.regular)
        NSApp.activate(ignoringOtherApps: true)

        // Create workspace
        workspaceController = WorkspaceWindowController(app: gApp)
        workspaceController?.showWindow(nil)

        // Daemon status
        let ipc = SmuxIpcClient()
        if ipc.isDaemonRunning {
            workspaceController?.window?.title = "smux — daemon ●"
        }

        // Setup menu bar
        setupMenuBar()

        // Tick timer
        tickTimer = Timer.scheduledTimer(withTimeInterval: 1.0/120.0, repeats: true) { [weak self] _ in
            guard let a = self?.ghosttyApp else { return }
            ghostty_app_tick(a)
        }
    }

    func setupMenuBar() {
        let mainMenu = NSMenu()

        // App menu
        let appMenu = NSMenu()
        appMenu.addItem(NSMenuItem(title: "Quit smux", action: #selector(NSApplication.terminate(_:)), keyEquivalent: "q"))
        let appItem = NSMenuItem()
        appItem.submenu = appMenu
        mainMenu.addItem(appItem)

        // File menu
        let fileMenu = NSMenu(title: "File")
        fileMenu.addItem(NSMenuItem(title: "New Tab", action: #selector(newTab), keyEquivalent: "t"))
        fileMenu.addItem(NSMenuItem(title: "Close", action: #selector(closeTab), keyEquivalent: "w"))
        let fileItem = NSMenuItem()
        fileItem.submenu = fileMenu
        mainMenu.addItem(fileItem)

        // View menu
        let viewMenu = NSMenu(title: "View")
        viewMenu.addItem(NSMenuItem(title: "Split Vertical", action: #selector(splitV), keyEquivalent: "d"))

        let splitHItem = NSMenuItem(title: "Split Horizontal", action: #selector(splitH), keyEquivalent: "d")
        splitHItem.keyEquivalentModifierMask = [.command, .shift]
        viewMenu.addItem(splitHItem)

        let viewItem = NSMenuItem()
        viewItem.submenu = viewMenu
        mainMenu.addItem(viewItem)

        NSApp.mainMenu = mainMenu
    }

    @objc func newTab() { workspaceController?.newTab() }
    @objc func closeTab() { workspaceController?.window?.close() }
    @objc func splitV() { workspaceController?.splitVertical() }
    @objc func splitH() { workspaceController?.splitHorizontal() }

    func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool { true }

    func applicationWillTerminate(_ notification: Notification) {
        tickTimer?.invalidate()
        tickTimer = nil
        workspaceController = nil
        if let a = ghosttyApp { ghostty_app_free(a); ghosttyApp = nil }
    }
}

// MARK: - Main

let nsApp = NSApplication.shared
let delegate = AppDelegate()
nsApp.delegate = delegate
nsApp.run()
