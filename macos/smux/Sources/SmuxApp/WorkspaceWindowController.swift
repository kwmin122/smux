import AppKit
import libghostty

/// Manages the main workspace: sidebar + timeline + terminal + inspector + controls.
class WorkspaceWindowController: NSWindowController {
    private var ghosttyApp: ghostty_app_t
    private var terminalViews: [GhosttyTerminalView] = []

    // UI components
    private var sidebar: SidebarView!
    private var timeline: StageTimeline!
    private var missionControl: MissionControlBar!
    private var inspector: InspectorDrawer!
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
        setupLayout()
    }

    required init?(coder: NSCoder) { fatalError() }

    private func setupLayout() {
        guard let window = window, let contentView = window.contentView else { return }
        contentView.wantsLayer = true

        // Main horizontal split: sidebar | center
        let mainSplit = NSSplitView()
        mainSplit.isVertical = true
        mainSplit.dividerStyle = .thin
        mainSplit.autoresizingMask = [.width, .height]
        mainSplit.frame = contentView.bounds

        // --- Left: Sidebar ---
        sidebar = SidebarView(frame: NSRect(x: 0, y: 0, width: 200, height: contentView.bounds.height))
        sidebar.onNewSession = { [weak self] in
            self?.startNewSession()
        }
        sidebar.onSelectSession = { [weak self] session in
            self?.selectSession(session)
        }
        mainSplit.addSubview(sidebar)

        // --- Center: timeline + terminal + mission control ---
        let centerStack = NSView()
        centerStack.wantsLayer = true

        // Timeline (top)
        timeline = StageTimeline(frame: NSRect(x: 0, y: 0, width: 800, height: 28))
        timeline.translatesAutoresizingMaskIntoConstraints = false
        centerStack.addSubview(timeline)

        // Terminal (middle)
        let termView = GhosttyTerminalView(frame: .zero, app: ghosttyApp)
        termView.translatesAutoresizingMaskIntoConstraints = false
        centerStack.addSubview(termView)
        terminalViews.append(termView)

        // Mission control bar (bottom)
        missionControl = MissionControlBar(frame: NSRect(x: 0, y: 0, width: 800, height: 32))
        missionControl.translatesAutoresizingMaskIntoConstraints = false
        missionControl.onApprove = { [weak self] in self?.missionState.approve() }
        missionControl.onPause = { [weak self] in
            guard let s = self else { return }
            if s.missionState.isAutoMode { s.missionState.pause() } else { s.missionState.resume() }
        }
        missionControl.onRetry = { [weak self] in
            NSLog("[smux] retry requested")
        }
        centerStack.addSubview(missionControl)

        NSLayoutConstraint.activate([
            timeline.topAnchor.constraint(equalTo: centerStack.topAnchor),
            timeline.leadingAnchor.constraint(equalTo: centerStack.leadingAnchor),
            timeline.trailingAnchor.constraint(equalTo: centerStack.trailingAnchor),
            timeline.heightAnchor.constraint(equalToConstant: 28),

            termView.topAnchor.constraint(equalTo: timeline.bottomAnchor),
            termView.leadingAnchor.constraint(equalTo: centerStack.leadingAnchor),
            termView.trailingAnchor.constraint(equalTo: centerStack.trailingAnchor),
            termView.bottomAnchor.constraint(equalTo: missionControl.topAnchor),

            missionControl.leadingAnchor.constraint(equalTo: centerStack.leadingAnchor),
            missionControl.trailingAnchor.constraint(equalTo: centerStack.trailingAnchor),
            missionControl.bottomAnchor.constraint(equalTo: centerStack.bottomAnchor),
            missionControl.heightAnchor.constraint(equalToConstant: 32),
        ])

        mainSplit.addSubview(centerStack)

        // --- Right: Inspector drawer (initially hidden) ---
        inspector = InspectorDrawer(frame: NSRect(x: 0, y: 0, width: 250, height: contentView.bounds.height))
        inspector.isHidden = true
        mainSplit.addSubview(inspector)

        contentView.addSubview(mainSplit)
        mainSplit.setPosition(200, ofDividerAt: 0)

        window.makeFirstResponder(termView)

        // Refresh session list
        refreshSessions()
    }

    // MARK: - Session management

    private func refreshSessions() {
        let ipc = SmuxIpcClient()
        missionState.refresh(client: ipc)
        sidebar.sessions = missionState.sessions
    }

    private func startNewSession() {
        NSLog("[smux] new session requested")
        // TODO: open session creation dialog
    }

    private func selectSession(_ session: SmuxSession) {
        NSLog("[smux] selected session: %@", session.id)
        window?.title = "smux — \(session.task.prefix(30))"
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

        let termView = GhosttyTerminalView(frame: newWindow.contentView!.bounds, app: ghosttyApp)
        termView.autoresizingMask = [.width, .height]
        newWindow.contentView = termView
        terminalViews.append(termView)

        currentWindow.addTabbedWindow(newWindow, ordered: .above)
        newWindow.makeKeyAndOrderFront(nil)
        newWindow.makeFirstResponder(termView)
    }

    // MARK: - Splits

    func splitVertical() {
        guard let window = window else { return }
        guard let termView = terminalViews.last else { return }
        let parent = termView.superview ?? window.contentView!

        let splitView = NSSplitView()
        splitView.isVertical = true
        splitView.dividerStyle = .thin
        splitView.frame = termView.frame
        splitView.autoresizingMask = termView.autoresizingMask
        splitView.translatesAutoresizingMaskIntoConstraints = termView.translatesAutoresizingMaskIntoConstraints

        // Copy constraints
        let constraints = parent.constraints.filter { $0.firstItem === termView || $0.secondItem === termView }

        termView.removeFromSuperview()
        splitView.addSubview(termView)

        let newTerm = GhosttyTerminalView(frame: .zero, app: ghosttyApp)
        splitView.addSubview(newTerm)
        terminalViews.append(newTerm)

        parent.addSubview(splitView)

        // Re-apply constraints to splitView
        for c in constraints {
            let first: AnyObject = (c.firstItem === termView) ? splitView : c.firstItem!
            let second: AnyObject? = (c.secondItem === termView) ? splitView : c.secondItem
            parent.addConstraint(NSLayoutConstraint(
                item: first, attribute: c.firstAttribute, relatedBy: c.relation,
                toItem: second, attribute: c.secondAttribute, multiplier: c.multiplier, constant: c.constant
            ))
        }

        splitView.setPosition(splitView.bounds.width / 2, ofDividerAt: 0)
        window.makeFirstResponder(newTerm)
    }

    func splitHorizontal() {
        guard let window = window else { return }
        guard let termView = terminalViews.last else { return }
        let parent = termView.superview ?? window.contentView!

        let splitView = NSSplitView()
        splitView.isVertical = false
        splitView.dividerStyle = .thin
        splitView.frame = termView.frame
        splitView.autoresizingMask = termView.autoresizingMask
        splitView.translatesAutoresizingMaskIntoConstraints = termView.translatesAutoresizingMaskIntoConstraints

        let constraints = parent.constraints.filter { $0.firstItem === termView || $0.secondItem === termView }
        termView.removeFromSuperview()
        splitView.addSubview(termView)

        let newTerm = GhosttyTerminalView(frame: .zero, app: ghosttyApp)
        splitView.addSubview(newTerm)
        terminalViews.append(newTerm)

        parent.addSubview(splitView)
        for c in constraints {
            let first: AnyObject = (c.firstItem === termView) ? splitView : c.firstItem!
            let second: AnyObject? = (c.secondItem === termView) ? splitView : c.secondItem
            parent.addConstraint(NSLayoutConstraint(
                item: first, attribute: c.firstAttribute, relatedBy: c.relation,
                toItem: second, attribute: c.secondAttribute, multiplier: c.multiplier, constant: c.constant
            ))
        }

        splitView.setPosition(splitView.bounds.height / 2, ofDividerAt: 0)
        window.makeFirstResponder(newTerm)
    }

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
            ("New Tab", { [weak self] in self?.newTab() }),
            ("Split Vertical", { [weak self] in self?.splitVertical() }),
            ("Split Horizontal", { [weak self] in self?.splitHorizontal() }),
            ("Find", { [weak self] in self?.toggleSearch() }),
            ("Toggle Inspector", { [weak self] in self?.toggleInspector() }),
            ("Refresh Sessions", { [weak self] in self?.refreshSessions() }),
        ], relativeTo: window)
    }

    // MARK: - Notifications

    func sendNotification(title: String, body: String) {
        let notification = NSUserNotification()
        notification.title = title
        notification.informativeText = body
        NSUserNotificationCenter.default.deliver(notification)
    }

    // MARK: - Save/Restore

    func saveState() {
        guard let window = window else { return }
        let f = window.frame
        let state = SessionRestore.WorkspaceState(
            windows: [SessionRestore.WindowState(
                tabs: [SessionRestore.TabState(
                    title: window.title,
                    workingDirectory: FileManager.default.currentDirectoryPath,
                    splits: []
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
    }
}
