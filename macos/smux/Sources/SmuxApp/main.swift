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
        // Only redraw the main window (not panels/dialogs)
        if let w = NSApplication.shared.mainWindow {
            w.contentView?.setNeedsDisplay(w.contentView?.bounds ?? .zero)
        }
    }
}
private let actionCb: @convention(c) (ghostty_app_t?, ghostty_target_s, ghostty_action_s) -> Bool = { _, target, action in
    if action.tag == GHOSTTY_ACTION_COMMAND_FINISHED {
        let payload = action.action.command_finished
        // Extract surface pointer as opaque identifier (UInt, NOT dereferenceable in async context)
        let surfaceKey: UInt
        if target.tag == GHOSTTY_TARGET_SURFACE {
            surfaceKey = UInt(bitPattern: target.target.surface)
        } else {
            surfaceKey = 0
        }
        let exitCode = Int(payload.exit_code)
        let duration = payload.duration

        NSLog("[ghostty-action] COMMAND_FINISHED exit=%d duration=%llu surface=0x%lx",
              exitCode, duration, surfaceKey)

        // Post to NotificationCenter — cannot capture Swift refs in @convention(c)
        DispatchQueue.main.async {
            NotificationCenter.default.post(
                name: .ghosttyCommandFinished,
                object: nil,
                userInfo: [
                    "exit_code": exitCode,
                    "surface_ptr": surfaceKey
                ]
            )
        }
    }

    return false
}
private let readCb: @convention(c) (UnsafeMutableRawPointer?, ghostty_clipboard_e, UnsafeMutableRawPointer?) -> Bool = { _, _, _ in false }
private let confirmCb: @convention(c) (UnsafeMutableRawPointer?, UnsafePointer<CChar>?, UnsafeMutableRawPointer?, ghostty_clipboard_request_e) -> Void = { _, _, _, _ in }
private let writeCb: @convention(c) (UnsafeMutableRawPointer?, ghostty_clipboard_e, UnsafePointer<ghostty_clipboard_content_s>?, Int, Bool) -> Void = { _, _, _, _, _ in }
private let closeCb: @convention(c) (UnsafeMutableRawPointer?, Bool) -> Void = { _, _ in
    // Do nothing — app closes via ⌘W menu action, not ghostty surface close
}

// MARK: - App Delegate

class AppDelegate: NSObject, NSApplicationDelegate {
    var ghosttyApp: ghostty_app_t?
    var workspaceController: WorkspaceWindowController?
    var appleScriptSupport: AppleScriptSupport?
    var sessionManager: SessionDetachReattach?
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
        workspaceController?.restoreState()
        workspaceController?.showWindow(nil)

        // Daemon status (async — never block main thread)
        SmuxIpcClient().checkDaemonAsync { [weak self] running in
            if running {
                self?.workspaceController?.window?.title = "smux — daemon ●"
            }
        }

        // Session detach/reattach — tmux-style
        sessionManager = SessionDetachReattach()
        let reattached = sessionManager?.reattachAll() ?? []
        if !reattached.isEmpty {
            NSLog("[smux] reattached %d sessions from previous run", reattached.count)
        }

        // AppleScript support
        if let wc = workspaceController {
            appleScriptSupport = AppleScriptSupport(controller: wc)
            appleScriptSupport?.registerHandlers()
        }

        // Setup menu bar
        setupMenuBar()

        // Tick timer
        tickTimer = Timer.scheduledTimer(withTimeInterval: 1.0/30.0, repeats: true) { [weak self] _ in
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
        fileMenu.addItem(NSMenuItem(title: "New Relay Session...", action: #selector(newSession), keyEquivalent: "n"))
        fileMenu.addItem(NSMenuItem(title: "New Tab", action: #selector(newTab), keyEquivalent: "t"))
        fileMenu.addItem(NSMenuItem(title: "Close", action: #selector(closeTab), keyEquivalent: "w"))

        let detachItem = NSMenuItem(title: "Detach Session", action: #selector(detachSession), keyEquivalent: "d")
        detachItem.keyEquivalentModifierMask = [.command, .shift, .control]
        fileMenu.addItem(detachItem)

        fileMenu.addItem(NSMenuItem(title: "Reattach Sessions", action: #selector(reattachSessions), keyEquivalent: ""))
        let fileItem = NSMenuItem()
        fileItem.submenu = fileMenu
        mainMenu.addItem(fileItem)

        // View menu
        let viewMenu = NSMenu(title: "View")
        viewMenu.addItem(NSMenuItem(title: "Split Vertical", action: #selector(splitV), keyEquivalent: "d"))

        let splitHItem = NSMenuItem(title: "Split Horizontal", action: #selector(splitH), keyEquivalent: "d")
        splitHItem.keyEquivalentModifierMask = [.command, .shift]
        viewMenu.addItem(splitHItem)

        let closePaneItem = NSMenuItem(title: "Close Pane", action: #selector(closePane), keyEquivalent: "w")
        closePaneItem.keyEquivalentModifierMask = [.command, .shift]
        viewMenu.addItem(closePaneItem)

        viewMenu.addItem(NSMenuItem(title: "Find", action: #selector(findInTerminal), keyEquivalent: "f"))
        viewMenu.addItem(NSMenuItem(title: "Toggle Inspector", action: #selector(toggleInspector), keyEquivalent: "i"))

        let browserItem = NSMenuItem(title: "Toggle Browser", action: #selector(toggleBrowser), keyEquivalent: "b")
        browserItem.keyEquivalentModifierMask = [.command, .shift]
        viewMenu.addItem(browserItem)

        let pingpongItem = NSMenuItem(title: "Ping-pong Mode", action: #selector(togglePingPong), keyEquivalent: "p")
        pingpongItem.keyEquivalentModifierMask = [.command, .shift]
        viewMenu.addItem(pingpongItem)

        viewMenu.addItem(NSMenuItem(title: "Command Palette", action: #selector(showPalette), keyEquivalent: "p"))

        let guideItem = NSMenuItem(title: "Guide", action: #selector(showGuide), keyEquivalent: "/")
        guideItem.keyEquivalentModifierMask = [.command]
        viewMenu.addItem(guideItem)

        let viewItem = NSMenuItem()
        viewItem.submenu = viewMenu
        mainMenu.addItem(viewItem)

        NSApp.mainMenu = mainMenu
    }

    @objc func findInTerminal() { workspaceController?.toggleSearch() }
    @objc func newSession() { workspaceController?.startNewSession() }
    @objc func newTab() { workspaceController?.newTab() }
    @objc func closeTab() {
        if let window = workspaceController?.window,
           let tabs = window.tabbedWindows, tabs.count > 1 {
            window.close()
        } else {
            performCleanShutdown()
        }
    }

    private func performCleanShutdown() {
        tickTimer?.invalidate()
        tickTimer = nil
        appleScriptSupport?.unregisterHandlers()
        workspaceController?.saveState()

        // destroyAllSurfaces detaches contentView (Metal) THEN frees surfaces async.
        // This order is critical — see Ghostty's BaseTerminalController.windowWillClose.
        workspaceController?.destroyAllSurfaces()
        workspaceController = nil
        if let a = ghosttyApp { ghostty_app_free(a); ghosttyApp = nil }
        NSApp.terminate(nil)
    }
    @objc func detachSession() {
        // Detach the current (focused) session — keeps running in daemon
        let targetId = sessionManager?.currentSessionId
            ?? sessionManager?.attachedSessions.sorted().first
        guard let id = targetId else {
            NSLog("[smux] no session to detach")
            return
        }
        _ = sessionManager?.detach(sessionId: id)
        NSLog("[smux] detached session: %@", id)
    }
    @objc func reattachSessions() {
        let ids = sessionManager?.reattachAll() ?? []
        NSLog("[smux] reattached %d sessions", ids.count)
    }
    @objc func closePane() { workspaceController?.closePane() }
    @objc func splitV() { workspaceController?.splitVertical() }
    @objc func splitH() { workspaceController?.splitHorizontal() }
    @objc func toggleInspector() { workspaceController?.toggleInspector() }
    @objc func toggleBrowser() { workspaceController?.toggleBrowser() }
    @objc func togglePingPong() {
        workspaceController?.togglePingPong()
    }
    @objc func showPalette() { workspaceController?.showCommandPalette() }
    @objc func showGuide() { GuidePanel.toggle(relativeTo: workspaceController?.window) }

    func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool { true }

    func applicationWillTerminate(_ notification: Notification) {
        // Safety net — performCleanShutdown or windowWillClose may have already cleaned up.
        tickTimer?.invalidate()
        tickTimer = nil
        appleScriptSupport?.unregisterHandlers()
        workspaceController?.destroyAllSurfaces()
        workspaceController = nil
        if let a = ghosttyApp { ghostty_app_free(a); ghosttyApp = nil }
    }
}

// MARK: - Notification Names

extension Notification.Name {
    /// Fired when ghostty's action_cb receives GHOSTTY_ACTION_COMMAND_FINISHED (OSC 133 D).
    /// userInfo keys: "exit_code" (Int), "surface_ptr" (UInt — opaque pointer as key, do NOT dereference)
    static let ghosttyCommandFinished = Notification.Name("smux.ghosttyCommandFinished")
}

// MARK: - Main

let nsApp = NSApplication.shared
let delegate = AppDelegate()
nsApp.delegate = delegate
nsApp.run()
