import AppKit
import UserNotifications
import WebKit
import libghostty

/// Manages the main workspace: sidebar + timeline + terminal + browser + inspector + controls.
class WorkspaceWindowController: NSWindowController, NSWindowDelegate {
    private var ghosttyApp: ghostty_app_t
    private var terminalViews: [GhosttyTerminalView] = []

    // UI components
    private var sidebar: SidebarView!
    private var timeline: StageTimeline!
    private var terminalContainer: NSView! // holds terminal (and browser split)
    private var missionControl: MissionControlBar!
    private var inspector: InspectorDrawer!
    private var browserPanel: BrowserPanelView?
    private var browserAutomation: BrowserAutomation?
    private var pingPongRouter: PingPongRouter?
    private var originalPauseHandler: (() -> Void)?
    private var searchBar: SearchBar?
    private var commandPalette: CommandPalette?

    let sessionRestore = SessionRestore()
    let missionState = MissionControlState()

    init(app: ghostty_app_t) {
        self.ghosttyApp = app

        let window = NSWindow(
            contentRect: NSRect(x: 100, y: 100, width: 1200, height: 800),
            styleMask: [.titled, .closable, .resizable, .miniaturizable],
            backing: .buffered,
            defer: false
        )
        window.title = "smux"
        window.backgroundColor = .black
        window.tabbingMode = .preferred
        window.minSize = NSSize(width: 600, height: 400)

        super.init(window: window)
        window.delegate = self
        setupLayout()
    }

    required init?(coder: NSCoder) { fatalError() }

    private func setupLayout() {
        guard let window = window, let contentView = window.contentView else { return }
        contentView.wantsLayer = true

        // --- Left: Sidebar (constraint-based, fixed width) ---
        sidebar = SidebarView(frame: .zero)
        sidebar.translatesAutoresizingMaskIntoConstraints = false
        sidebar.onNewSession = { [weak self] in self?.startNewSession() }
        sidebar.onSelectSession = { [weak self] session in self?.selectSession(session) }
        sidebar.onNewWorkspace = { [weak self] in self?.startNewSession() }
        sidebar.onSelectWorkspace = { [weak self] workspace in self?.selectWorkspace(workspace) }
        contentView.addSubview(sidebar)

        // --- Right: Inspector drawer (initially hidden) ---
        inspector = InspectorDrawer(frame: .zero)
        inspector.translatesAutoresizingMaskIntoConstraints = false
        inspector.isHidden = true
        contentView.addSubview(inspector)

        // --- Center: timeline + terminal + mission control ---
        let centerView = NSView()
        centerView.wantsLayer = true
        centerView.translatesAutoresizingMaskIntoConstraints = false
        contentView.addSubview(centerView)

        // Timeline (top of center)
        timeline = StageTimeline(frame: .zero)
        timeline.translatesAutoresizingMaskIntoConstraints = false
        centerView.addSubview(timeline)

        // Terminal container (holds terminal + optional browser split)
        terminalContainer = NSView()
        terminalContainer.wantsLayer = true
        terminalContainer.translatesAutoresizingMaskIntoConstraints = false
        centerView.addSubview(terminalContainer)

        // Terminal — constraint-based to fill container. HOST_MANAGED mode (smux owns PTY).
        let termView = GhosttyTerminalView(frame: NSRect(x: 0, y: 0, width: 800, height: 600), app: ghosttyApp, managed: true)
        termView.translatesAutoresizingMaskIntoConstraints = false
        terminalContainer.addSubview(termView)
        terminalViews.append(termView)

        NSLayoutConstraint.activate([
            termView.topAnchor.constraint(equalTo: terminalContainer.topAnchor),
            termView.leadingAnchor.constraint(equalTo: terminalContainer.leadingAnchor),
            termView.trailingAnchor.constraint(equalTo: terminalContainer.trailingAnchor),
            termView.bottomAnchor.constraint(equalTo: terminalContainer.bottomAnchor),
        ])

        // Mission control bar (bottom of center)
        missionControl = MissionControlBar(frame: .zero)
        missionControl.translatesAutoresizingMaskIntoConstraints = false
        missionControl.onApprove = { [weak self] in self?.missionState.approve() }
        missionControl.onPause = { [weak self] in
            guard let s = self else { return }
            if s.missionState.isAutoMode { s.missionState.pause() } else { s.missionState.resume() }
        }
        missionControl.onPingPong = { [weak self] in self?.togglePingPong() }
        missionControl.onRetry = { [weak self] in NSLog("[smux] retry requested") }
        centerView.addSubview(missionControl)

        // --- Constraints ---
        NSLayoutConstraint.activate([
            // Sidebar: left edge, full height, fixed 200px width
            sidebar.topAnchor.constraint(equalTo: contentView.topAnchor),
            sidebar.leadingAnchor.constraint(equalTo: contentView.leadingAnchor),
            sidebar.bottomAnchor.constraint(equalTo: contentView.bottomAnchor),
            sidebar.widthAnchor.constraint(equalToConstant: 200),

            // Center: right of sidebar, full height, flexible width
            centerView.topAnchor.constraint(equalTo: contentView.topAnchor),
            centerView.leadingAnchor.constraint(equalTo: sidebar.trailingAnchor),
            centerView.bottomAnchor.constraint(equalTo: contentView.bottomAnchor),
            centerView.trailingAnchor.constraint(equalTo: contentView.trailingAnchor),

            // Inspector: right edge (overlaps center when visible)
            inspector.topAnchor.constraint(equalTo: contentView.topAnchor),
            inspector.trailingAnchor.constraint(equalTo: contentView.trailingAnchor),
            inspector.bottomAnchor.constraint(equalTo: contentView.bottomAnchor),
            inspector.widthAnchor.constraint(equalToConstant: 250),

            // Timeline at top of center
            timeline.topAnchor.constraint(equalTo: centerView.topAnchor),
            timeline.leadingAnchor.constraint(equalTo: centerView.leadingAnchor),
            timeline.trailingAnchor.constraint(equalTo: centerView.trailingAnchor),
            timeline.heightAnchor.constraint(equalToConstant: 28),

            // Terminal container fills center between timeline and mission control
            terminalContainer.topAnchor.constraint(equalTo: timeline.bottomAnchor),
            terminalContainer.leadingAnchor.constraint(equalTo: centerView.leadingAnchor),
            terminalContainer.trailingAnchor.constraint(equalTo: centerView.trailingAnchor),
            terminalContainer.bottomAnchor.constraint(equalTo: missionControl.topAnchor),

            // Mission control at bottom of center
            missionControl.leadingAnchor.constraint(equalTo: centerView.leadingAnchor),
            missionControl.trailingAnchor.constraint(equalTo: centerView.trailingAnchor),
            missionControl.bottomAnchor.constraint(equalTo: centerView.bottomAnchor),
            missionControl.heightAnchor.constraint(equalToConstant: 32),
        ])

        window.makeFirstResponder(termView)

        // Refresh session list
        refreshSessions()
    }

    // MARK: - Session management

    private func refreshSessions() {
        DispatchQueue.global(qos: .utility).async { [weak self] in
            let ipc = SmuxIpcClient()
            self?.missionState.refresh(client: ipc)
            DispatchQueue.main.async {
                guard let self = self else { return }
                self.sidebar.sessions = self.missionState.sessions
                for session in self.missionState.sessions where session.status == .failed {
                    self.sendNotification(title: "Session Failed", body: session.task, source: session.planner)
                }
            }
        }
    }

    func startNewSession() {
        let dialog = NewSessionDialog()
        dialog.onStart = { [weak self] config in
            self?.launchRelaySession(config)
        }
        if let window = window {
            let wx = window.frame.midX - 240
            let wy = window.frame.midY - 260
            dialog.setFrameOrigin(NSPoint(x: wx, y: wy))
        }
        dialog.makeKeyAndOrderFront(nil)
    }

    /// Launch a relay session: send StartSession to daemon, update UI.
    private func launchRelaySession(_ config: NewSessionDialog.SessionConfig) {
        NSLog("[smux] starting relay: %@ → planner=%@, verifier=%@, rounds=%d",
              config.task, config.planner, config.verifier, config.maxRounds)

        // Update timeline to show we're in Ideate stage
        timeline.setCurrentStage(0)

        // Update mission control
        missionControl.update(
            status: "Relay: \(config.planner) ↔ \(config.verifier)",
            round: 1, maxRounds: config.maxRounds,
            isPaused: !config.autoApprove
        )

        // Update window title
        window?.title = "smux — \(config.task.prefix(40))"

        // Send to daemon via IPC
        let ipc = SmuxIpcClient()
        do {
            try ipc.connect()
            var agents: [[String: String]] = [
                ["name": config.planner, "role": "planner"],
                ["name": config.verifier, "role": "verifier"],
            ]
            for v in config.additionalVerifiers {
                agents.append(["name": v, "role": "verifier"])
            }
            try ipc.send([
                "StartSession": [
                    "task": config.task,
                    "planner": config.planner,
                    "verifier": config.verifier,
                    "max_rounds": config.maxRounds,
                    "auto_approve": config.autoApprove,
                    "agents": agents,
                ] as [String: Any]
            ])
            let response = try ipc.receive()
            ipc.disconnect()

            if let created = response["SessionCreated"] as? [String: Any],
               let sessionId = created["session_id"] as? String {
                NSLog("[smux] session created: %@", sessionId)
                if let delegate = NSApp.delegate as? AppDelegate {
                    delegate.sessionManager?.currentSessionId = sessionId
                    _ = delegate.sessionManager?.attach(sessionId: sessionId)
                }
                sendNotification(title: "세션 시작", body: config.task, source: config.planner)
            }
        } catch {
            NSLog("[smux] daemon not running, starting local relay display")
            // Even without daemon, show the relay state in UI
            sendNotification(
                title: "로컬 모드",
                body: "daemon 미실행 — 터미널에서 직접 에이전트를 실행하세요.\n예: claude \"\(config.task)\"",
                source: "smux"
            )
        }

        // Refresh sidebar
        refreshSessions()
    }

    private func selectSession(_ session: SmuxSession) {
        NSLog("[smux] selected session: %@", session.id)
        window?.title = "smux — \(session.task.prefix(30))"
        // Update current session for detach menu
        if let delegate = NSApp.delegate as? AppDelegate {
            delegate.sessionManager?.currentSessionId = session.id
        }
    }

    private func selectWorkspace(_ workspace: Workspace) {
        NSLog("[smux] selected workspace: %@", workspace.name)
        window?.title = "smux — \(workspace.name)"
        if !workspace.sessions.isEmpty {
            missionState.sessions = workspace.sessions
        }
    }

    // MARK: - Tabs

    func newTab() {
        guard let currentWindow = window else { return }
        let newWindow = NSWindow(
            contentRect: NSRect(x: 100, y: 100, width: 1200, height: 800),
            styleMask: [.titled, .closable, .resizable, .miniaturizable],
            backing: .buffered, defer: false
        )
        newWindow.title = "smux"
        newWindow.backgroundColor = .black

        let termView = GhosttyTerminalView(frame: NSRect(x: 0, y: 0, width: 800, height: 600), app: ghosttyApp, managed: true)
        termView.autoresizingMask = [.width, .height]
        newWindow.contentView = termView
        terminalViews.append(termView)

        currentWindow.addTabbedWindow(newWindow, ordered: .above)
        newWindow.makeKeyAndOrderFront(nil)
        newWindow.makeFirstResponder(termView)
    }

    // MARK: - Splits

    /// Split the last terminal view vertically (side by side).
    func splitVertical() {
        doSplit(vertical: true)
    }

    /// Split the last terminal view horizontally (top/bottom).
    func splitHorizontal() {
        doSplit(vertical: false)
    }

    /// Close the currently focused split pane (⌘⇧W).
    func closePane() {
        guard terminalViews.count > 1 else { return }
        guard let window = window else { return }
        guard let focused = window.firstResponder as? GhosttyTerminalView else { return }
        guard let split = focused.superview as? NSSplitView else { return }
        let parent = split.superview ?? terminalContainer!

        focused.removeFromSuperview()
        terminalViews.removeAll { $0 === focused }

        // If split has only 1 child left, unwrap it back into the parent with constraints
        if split.subviews.count == 1, let remaining = split.subviews.first {
            let splitConstraints = parent.constraints.filter {
                $0.firstItem === split || $0.secondItem === split
            }
            NSLayoutConstraint.deactivate(splitConstraints)

            remaining.removeFromSuperview()
            split.removeFromSuperview()

            remaining.translatesAutoresizingMaskIntoConstraints = false
            parent.addSubview(remaining)
            NSLayoutConstraint.activate([
                remaining.topAnchor.constraint(equalTo: parent.topAnchor),
                remaining.leadingAnchor.constraint(equalTo: parent.leadingAnchor),
                remaining.trailingAnchor.constraint(equalTo: parent.trailingAnchor),
                remaining.bottomAnchor.constraint(equalTo: parent.bottomAnchor),
            ])
        }

        if let last = terminalViews.last {
            window.makeFirstResponder(last)
        }
    }

    private func doSplit(vertical: Bool) {
        guard let window = window else { return }
        guard let termView = terminalViews.last else { return }
        let container = termView.superview ?? terminalContainer!

        guard container.bounds.width > 10, container.bounds.height > 10 else { return }

        // Deactivate old constraints on termView
        let oldConstraints = container.constraints.filter {
            $0.firstItem === termView || $0.secondItem === termView
        }
        NSLayoutConstraint.deactivate(oldConstraints)
        termView.removeFromSuperview()

        // NSSplitView: pinned to parent via constraints; children use autoresizingMask inside
        let splitView = NSSplitView()
        splitView.isVertical = vertical
        splitView.dividerStyle = .thin
        splitView.translatesAutoresizingMaskIntoConstraints = false

        // Children inside NSSplitView MUST use autoresizingMask (NSSplitView manages them)
        termView.translatesAutoresizingMaskIntoConstraints = true
        termView.autoresizingMask = [.width, .height]
        splitView.addSubview(termView)

        let newTerm = GhosttyTerminalView(frame: NSRect(x: 0, y: 0, width: 800, height: 600), app: ghosttyApp, managed: true)
        newTerm.translatesAutoresizingMaskIntoConstraints = true
        newTerm.autoresizingMask = [.width, .height]
        splitView.addSubview(newTerm)
        terminalViews.append(newTerm)

        // Pin splitView to parent via Auto Layout
        container.addSubview(splitView)
        NSLayoutConstraint.activate([
            splitView.topAnchor.constraint(equalTo: container.topAnchor),
            splitView.leadingAnchor.constraint(equalTo: container.leadingAnchor),
            splitView.trailingAnchor.constraint(equalTo: container.trailingAnchor),
            splitView.bottomAnchor.constraint(equalTo: container.bottomAnchor),
        ])

        // Position divider after layout resolves
        DispatchQueue.main.async {
            let pos = vertical ? splitView.bounds.width / 2 : splitView.bounds.height / 2
            if pos > 0 { splitView.setPosition(pos, ofDividerAt: 0) }
            window.makeFirstResponder(newTerm)
        }
    }

    // MARK: - Ping-Pong Mode

    /// Toggle ping-pong mode between the two visible terminal panes.
    /// Requires at least 2 terminal views (split first with ⌘D).
    func togglePingPong() {
        NSLog("[pingpong] togglePingPong — terminalViews.count=%d, router=%@", terminalViews.count, pingPongRouter != nil ? "exists" : "nil")
        if let router = pingPongRouter, router.isActive {
            // Stop ping-pong
            router.stop()
            pingPongRouter = nil
            missionControl.setPingPongActive(false)
            missionControl.update(status: "Ready", round: 0, maxRounds: 0, isPaused: false)
            missionControl.onPause = originalPauseHandler
        } else {
            // Need at least 2 terminal panes
            guard terminalViews.count >= 2 else {
                NSLog("[pingpong] need 2+ panes — split first with Cmd+D")
                return
            }

            let paneA = terminalViews[terminalViews.count - 2]
            let paneB = terminalViews[terminalViews.count - 1]

            let router = PingPongRouter(paneA: paneA, paneB: paneB, maxRounds: 5)
            router.paneALabel = "Left"
            router.paneBLabel = "Right"

            // Wire state updates to mission control
            router.onStateChanged = { [weak self] state, round in
                DispatchQueue.main.async {
                    self?.missionControl.update(
                        status: "🔄 \(state.rawValue)",
                        round: round + 1,
                        maxRounds: router.maxRounds,
                        isPaused: state == .paused
                    )
                    // Update stage timeline based on round progress
                    let progress = min(3, round / 5)
                    self?.timeline.setCurrentStage(progress)
                }
            }

            router.onTurnComplete = { [weak self] speaker, output in
                let target = (speaker == "Left") ? "Right" : "Left"
                let preview = String(output.prefix(200))
                self?.inspector.transcript += "\n[\(speaker) → \(target)] \(preview)"
            }

            router.onSessionComplete = { [weak self] totalRounds in
                self?.sendNotification(
                    title: "Ping-pong 완료",
                    body: "\(totalRounds) 라운드 완료",
                    source: "smux"
                )
                self?.missionControl.update(status: "✅ Complete", round: totalRounds, maxRounds: totalRounds, isPaused: false)
            }

            // Save original pause handler, wire ping-pong controls
            originalPauseHandler = missionControl.onPause
            missionControl.onPause = { [weak router] in
                if router?.state == .paused {
                    router?.resume()
                } else {
                    router?.pause()
                }
            }

            router.start()
            self.pingPongRouter = router

            missionControl.setPingPongActive(true)
            missionControl.update(status: "🔄 Ping-pong ON", round: 1, maxRounds: router.maxRounds, isPaused: false)
            NSLog("[pingpong] started — waiting for agent output")
        }
    }

    /// Check if ping-pong mode is active.
    var isPingPongActive: Bool { pingPongRouter?.isActive ?? false }

    // MARK: - Search

    func toggleSearch() {
        if let bar = searchBar {
            if bar.isHidden { bar.show() } else { bar.hide() }
        } else {
            guard let contentView = window?.contentView else { return }
            let bar = SearchBar(frame: contentView.bounds)
            bar.translatesAutoresizingMaskIntoConstraints = false
            contentView.addSubview(bar, positioned: .above, relativeTo: nil)
            NSLayoutConstraint.activate([
                bar.topAnchor.constraint(equalTo: contentView.topAnchor),
                bar.leadingAnchor.constraint(equalTo: contentView.leadingAnchor, constant: 200),
                bar.trailingAnchor.constraint(equalTo: contentView.trailingAnchor),
                bar.heightAnchor.constraint(equalToConstant: 32),
            ])
            bar.show()
            self.searchBar = bar
        }
    }

    // MARK: - Inspector

    func toggleInspector() {
        inspector.toggle()
    }

    // MARK: - Command Palette

    func showCommandPalette() {
        if commandPalette == nil {
            commandPalette = CommandPalette()
        }
        commandPalette?.show(commands: [
            ("Ping-pong Mode Toggle", { [weak self] in self?.togglePingPong() }),
            ("New Relay Session", { [weak self] in self?.startNewSession() }),
            ("New Tab", { [weak self] in self?.newTab() }),
            ("Split Vertical", { [weak self] in self?.splitVertical() }),
            ("Split Horizontal", { [weak self] in self?.splitHorizontal() }),
            ("Toggle Browser", { [weak self] in self?.toggleBrowser() }),
            ("Find", { [weak self] in self?.toggleSearch() }),
            ("Toggle Inspector", { [weak self] in self?.toggleInspector() }),
            ("Refresh Sessions", { [weak self] in self?.refreshSessions() }),
            ("Guide", { [weak self] in GuidePanel.toggle(relativeTo: self?.window) }),
        ], relativeTo: window)
    }

    // MARK: - Browser Panel

    /// Toggle the embedded browser panel (split right of terminal).
    func toggleBrowser() {
        if let browser = browserPanel, let split = browser.superview as? NSSplitView {
            // Browser OFF — restore terminal with constraints
            browser.removeFromSuperview()
            if let termView = terminalViews.first {
                termView.removeFromSuperview()
                let splitConstraints = terminalContainer.constraints.filter {
                    $0.firstItem === split || $0.secondItem === split
                }
                NSLayoutConstraint.deactivate(splitConstraints)
                split.removeFromSuperview()

                termView.translatesAutoresizingMaskIntoConstraints = false
                terminalContainer.addSubview(termView)
                NSLayoutConstraint.activate([
                    termView.topAnchor.constraint(equalTo: terminalContainer.topAnchor),
                    termView.leadingAnchor.constraint(equalTo: terminalContainer.leadingAnchor),
                    termView.trailingAnchor.constraint(equalTo: terminalContainer.trailingAnchor),
                    termView.bottomAnchor.constraint(equalTo: terminalContainer.bottomAnchor),
                ])
            }
            browserPanel = nil
            browserAutomation = nil
            return
        } else {
            // Browser ON — same pattern as doSplit
            guard let termView = terminalViews.first else { return }

            let oldConstraints = terminalContainer.constraints.filter {
                $0.firstItem === termView || $0.secondItem === termView
            }
            NSLayoutConstraint.deactivate(oldConstraints)
            termView.removeFromSuperview()

            let split = NSSplitView()
            split.isVertical = true
            split.dividerStyle = .thin
            split.translatesAutoresizingMaskIntoConstraints = false

            termView.translatesAutoresizingMaskIntoConstraints = true
            termView.autoresizingMask = [.width, .height]
            split.addSubview(termView)

            let browser = BrowserPanelView(frame: NSRect(x: 0, y: 0, width: 800, height: 600))
            browser.translatesAutoresizingMaskIntoConstraints = true
            browser.autoresizingMask = [.width, .height]
            browser.onPageLoaded = { [weak self] url in
                if let url = url {
                    self?.inspector.transcript += "\n[browser] loaded: \(url.absoluteString)"
                }
            }
            browser.onConsoleMessage = { [weak self] msg in
                self?.inspector.transcript += "\n[console] \(msg)"
            }
            split.addSubview(browser)

            terminalContainer.addSubview(split)
            NSLayoutConstraint.activate([
                split.topAnchor.constraint(equalTo: terminalContainer.topAnchor),
                split.leadingAnchor.constraint(equalTo: terminalContainer.leadingAnchor),
                split.trailingAnchor.constraint(equalTo: terminalContainer.trailingAnchor),
                split.bottomAnchor.constraint(equalTo: terminalContainer.bottomAnchor),
            ])

            DispatchQueue.main.async {
                split.setPosition(split.bounds.width * 0.5, ofDividerAt: 0)
            }
            browser.navigate(to: "http://localhost:3000")

            self.browserPanel = browser
            self.browserAutomation = BrowserAutomation(browserPanel: browser)
        }
    }

    /// Open a specific URL in the browser panel.
    func openInBrowser(url: String) {
        if browserPanel == nil { toggleBrowser() }
        browserPanel?.navigate(to: url)
    }

    /// Check if browser panel is visible.
    var isBrowserVisible: Bool {
        browserPanel != nil && !(browserPanel?.isHidden ?? true)
    }

    /// Access the browser automation engine (creates browser if needed).
    func automation() -> BrowserAutomation? {
        if browserAutomation == nil { toggleBrowser() }
        return browserAutomation
    }

    // MARK: - Notifications

    func sendNotification(title: String, body: String, source: String = "System") {
        // Add to sidebar notification list
        let notif = SmuxNotification(
            id: UUID().uuidString,
            title: title,
            body: body,
            source: source,
            timestamp: Date(),
            isRead: false
        )
        sidebar.notifications.append(notif)

        // Also send macOS system notification (only when running as a proper .app bundle)
        guard Bundle.main.bundleIdentifier != nil else { return }
        let content = UNMutableNotificationContent()
        content.title = title
        content.body = body
        content.sound = .default
        let request = UNNotificationRequest(identifier: notif.id, content: content, trigger: nil)
        UNUserNotificationCenter.current().add(request)
    }

    // MARK: - Save/Restore

    func saveState() {
        guard let window = window else { return }
        let f = window.frame

        // Serialize split directions by walking NSSplitView hierarchy
        let splits = collectSplitDirections(from: terminalContainer)

        let state = SessionRestore.WorkspaceState(
            windows: [SessionRestore.WindowState(
                tabs: [SessionRestore.TabState(
                    title: window.title,
                    workingDirectory: FileManager.default.currentDirectoryPath,
                    splits: splits
                )],
                activeTabIndex: 0,
                frame: SessionRestore.FrameRect(x: Double(f.origin.x), y: Double(f.origin.y), width: Double(f.width), height: Double(f.height))
            )],
            activeWindowIndex: 0
        )
        sessionRestore.save(state: state)
    }

    func restoreState() {
        guard let state = sessionRestore.load(), let ws = state.windows.first, let window = window else { return }
        let f = ws.frame
        window.setFrame(NSRect(x: f.x, y: f.y, width: f.width, height: f.height), display: true)

        // Restore split layout by replaying split directions with saved ratios
        guard let tab = ws.tabs.first, !tab.splits.isEmpty else { return }
        for split in tab.splits {
            let vertical = split.direction == "vertical"
            doSplit(vertical: vertical)
        }
        // Apply saved divider ratios after layout resolves
        DispatchQueue.main.async { [weak self] in
            self?.applySavedRatios(tab.splits)
        }
    }

    /// Walk view hierarchy to collect split directions (depth-first).
    private func collectSplitDirections(from view: NSView) -> [SessionRestore.SplitState] {
        var result: [SessionRestore.SplitState] = []
        for subview in view.subviews {
            if let split = subview as? NSSplitView {
                let ratio: Double
                if split.subviews.count >= 2 {
                    let total = split.isVertical ? split.bounds.width : split.bounds.height
                    let first = split.isVertical ? split.subviews[0].frame.width : split.subviews[0].frame.height
                    ratio = total > 0 ? Double(first / total) : 0.5
                } else {
                    ratio = 0.5
                }
                result.append(SessionRestore.SplitState(
                    direction: split.isVertical ? "vertical" : "horizontal",
                    ratio: ratio,
                    children: []
                ))
                // Recurse into nested splits
                result.append(contentsOf: collectSplitDirections(from: split))
            }
        }
        return result
    }

    /// Apply saved divider ratios to NSSplitViews in order.
    private func applySavedRatios(_ splits: [SessionRestore.SplitState]) {
        var splitViews: [NSSplitView] = []
        collectNSSplitViews(from: terminalContainer, into: &splitViews)
        for (i, sv) in splitViews.enumerated() where i < splits.count {
            let total = sv.isVertical ? sv.bounds.width : sv.bounds.height
            let pos = total * CGFloat(splits[i].ratio)
            if pos > 0 { sv.setPosition(pos, ofDividerAt: 0) }
        }
    }

    private func collectNSSplitViews(from view: NSView, into result: inout [NSSplitView]) {
        for subview in view.subviews {
            if let sv = subview as? NSSplitView {
                result.append(sv)
                collectNSSplitViews(from: sv, into: &result)
            }
        }
    }

    // MARK: - Window Delegate

    func windowWillClose(_ notification: Notification) {
        // Detach Metal layer hierarchy before window dealloc.
        // This prevents zombie CAMetalLayer when the window closes.
        destroyAllSurfaces()
    }

    /// Destroy all ghostty surfaces. Must call before ghostty_app_free.
    func destroyAllSurfaces() {
        pingPongRouter?.stop()
        pingPongRouter = nil
        // CRITICAL ORDER: Detach Metal CALayer from window hierarchy FIRST,
        // THEN free surfaces. Reversing this causes zombie Metal layers.
        window?.contentView = nil
        for tv in terminalViews { tv.destroySurface() }
        terminalViews.removeAll()
    }
}

