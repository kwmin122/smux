import AppKit
import libghostty

/// Manages the main workspace window: terminal panes, tabs, splits.
class WorkspaceWindowController: NSWindowController {
    private var ghosttyApp: ghostty_app_t
    private var terminalViews: [GhosttyTerminalView] = []
    private var tabGroup: [NSWindow] = []

    init(app: ghostty_app_t) {
        self.ghosttyApp = app

        let window = NSWindow(
            contentRect: NSRect(x: 100, y: 100, width: 1000, height: 700),
            styleMask: [.titled, .closable, .resizable, .miniaturizable],
            backing: .buffered,
            defer: false
        )
        window.title = "smux"
        window.backgroundColor = .black
        window.tabbingMode = .preferred
        window.minSize = NSSize(width: 400, height: 300)

        super.init(window: window)

        // Create initial terminal pane
        let termView = GhosttyTerminalView(frame: window.contentView!.bounds, app: app)
        termView.autoresizingMask = [.width, .height]
        window.contentView = termView
        terminalViews.append(termView)

        window.makeFirstResponder(termView)
    }

    required init?(coder: NSCoder) { fatalError() }

    // MARK: - Tab management

    func newTab() {
        guard let currentWindow = window else { return }

        let newWindow = NSWindow(
            contentRect: NSRect(x: 100, y: 100, width: 1000, height: 700),
            styleMask: [.titled, .closable, .resizable, .miniaturizable],
            backing: .buffered,
            defer: false
        )
        newWindow.title = "smux"
        newWindow.backgroundColor = .black
        newWindow.tabbingMode = .preferred

        let termView = GhosttyTerminalView(frame: newWindow.contentView!.bounds, app: ghosttyApp)
        termView.autoresizingMask = [.width, .height]
        newWindow.contentView = termView
        terminalViews.append(termView)

        currentWindow.addTabbedWindow(newWindow, ordered: .above)
        newWindow.makeKeyAndOrderFront(nil)
        newWindow.makeFirstResponder(termView)
    }

    // MARK: - Split management

    func splitVertical() {
        guard let window = window, let currentContent = window.contentView else { return }

        let splitView = NSSplitView()
        splitView.isVertical = true
        splitView.dividerStyle = .thin
        splitView.autoresizingMask = [.width, .height]
        splitView.frame = currentContent.bounds

        // Keep existing terminal on left
        currentContent.removeFromSuperview()
        splitView.addSubview(currentContent)

        // New terminal on right
        let newTerm = GhosttyTerminalView(frame: .zero, app: ghosttyApp)
        splitView.addSubview(newTerm)
        terminalViews.append(newTerm)

        window.contentView = splitView
        splitView.setPosition(splitView.bounds.width / 2, ofDividerAt: 0)
        window.makeFirstResponder(newTerm)
    }

    func splitHorizontal() {
        guard let window = window, let currentContent = window.contentView else { return }

        let splitView = NSSplitView()
        splitView.isVertical = false
        splitView.dividerStyle = .thin
        splitView.autoresizingMask = [.width, .height]
        splitView.frame = currentContent.bounds

        currentContent.removeFromSuperview()
        splitView.addSubview(currentContent)

        let newTerm = GhosttyTerminalView(frame: .zero, app: ghosttyApp)
        splitView.addSubview(newTerm)
        terminalViews.append(newTerm)

        window.contentView = splitView
        splitView.setPosition(splitView.bounds.height / 2, ofDividerAt: 0)
        window.makeFirstResponder(newTerm)
    }
}
