import AppKit

/// Left rail sidebar showing sessions, alerts, and workspace controls.
class SidebarView: NSView {
    private let sessionList = NSTableView()
    private let scrollView = NSScrollView()
    private let headerLabel = NSTextField(labelWithString: "SESSIONS")
    private let newSessionButton = NSButton(title: "+ New", target: nil, action: nil)
    private let alertBadge = NSTextField(labelWithString: "")

    var sessions: [SmuxSession] = [] {
        didSet { sessionList.reloadData() }
    }
    var onSelectSession: ((SmuxSession) -> Void)?
    var onNewSession: (() -> Void)?

    override init(frame: NSRect) {
        super.init(frame: frame)
        setupUI()
    }
    required init?(coder: NSCoder) { fatalError() }

    private func setupUI() {
        wantsLayer = true
        layer?.backgroundColor = NSColor(white: 0.1, alpha: 1).cgColor

        // Header
        headerLabel.font = .monospacedSystemFont(ofSize: 9, weight: .bold)
        headerLabel.textColor = .tertiaryLabelColor
        headerLabel.translatesAutoresizingMaskIntoConstraints = false
        addSubview(headerLabel)

        // New session button
        newSessionButton.bezelStyle = .inline
        newSessionButton.font = .monospacedSystemFont(ofSize: 10, weight: .regular)
        newSessionButton.target = self
        newSessionButton.action = #selector(newSessionTapped)
        newSessionButton.translatesAutoresizingMaskIntoConstraints = false
        addSubview(newSessionButton)

        // Session list
        let column = NSTableColumn(identifier: NSUserInterfaceItemIdentifier("session"))
        column.title = ""
        sessionList.addTableColumn(column)
        sessionList.headerView = nil
        sessionList.backgroundColor = .clear
        sessionList.dataSource = self
        sessionList.delegate = self
        sessionList.rowHeight = 36

        scrollView.documentView = sessionList
        scrollView.hasVerticalScroller = true
        scrollView.drawsBackground = false
        scrollView.translatesAutoresizingMaskIntoConstraints = false
        addSubview(scrollView)

        // Alert badge
        alertBadge.font = .monospacedSystemFont(ofSize: 9, weight: .bold)
        alertBadge.textColor = .systemOrange
        alertBadge.translatesAutoresizingMaskIntoConstraints = false
        addSubview(alertBadge)

        NSLayoutConstraint.activate([
            headerLabel.topAnchor.constraint(equalTo: topAnchor, constant: 12),
            headerLabel.leadingAnchor.constraint(equalTo: leadingAnchor, constant: 12),
            newSessionButton.centerYAnchor.constraint(equalTo: headerLabel.centerYAnchor),
            newSessionButton.trailingAnchor.constraint(equalTo: trailingAnchor, constant: -8),
            alertBadge.topAnchor.constraint(equalTo: headerLabel.bottomAnchor, constant: 4),
            alertBadge.leadingAnchor.constraint(equalTo: leadingAnchor, constant: 12),
            scrollView.topAnchor.constraint(equalTo: alertBadge.bottomAnchor, constant: 8),
            scrollView.leadingAnchor.constraint(equalTo: leadingAnchor),
            scrollView.trailingAnchor.constraint(equalTo: trailingAnchor),
            scrollView.bottomAnchor.constraint(equalTo: bottomAnchor),
        ])
    }

    func setAlerts(_ count: Int) {
        alertBadge.stringValue = count > 0 ? "⚠ \(count) alerts" : ""
    }

    @objc private func newSessionTapped() { onNewSession?() }
}

extension SidebarView: NSTableViewDataSource, NSTableViewDelegate {
    func numberOfRows(in tableView: NSTableView) -> Int { sessions.count }

    func tableView(_ tableView: NSTableView, viewFor tableColumn: NSTableColumn?, row: Int) -> NSView? {
        let session = sessions[row]
        let cell = NSTextField(labelWithString: "")
        cell.font = .monospacedSystemFont(ofSize: 10, weight: .regular)
        let statusIcon = session.status == .running ? "●" : "○"
        cell.stringValue = "\(statusIcon) \(session.task.prefix(20))  R\(session.currentRound)"
        cell.textColor = session.status == .running ? .systemGreen : .secondaryLabelColor
        return cell
    }

    func tableViewSelectionDidChange(_ notification: Notification) {
        let row = sessionList.selectedRow
        guard row >= 0, row < sessions.count else { return }
        onSelectSession?(sessions[row])
    }
}
