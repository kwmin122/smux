import AppKit

/// cmux-style vertical tab sidebar with workspace groups, git branches, ports, PR alerts,
/// notification bell with badge, and attention ring animation.
class SidebarView: NSView {

    // MARK: - Data

    var workspaces: [Workspace] = [] {
        didSet { tableView.reloadData() }
    }

    var notifications: [SmuxNotification] = [] {
        didSet { updateBadge() }
    }

    /// Legacy compat — auto-builds workspaces from flat session list.
    var sessions: [SmuxSession] = [] {
        didSet {
            workspaces = WorkspaceDetector.buildDefaultWorkspaces(from: sessions)
            // Populate git branches + ports asynchronously to avoid blocking UI
            if let first = workspaces.first {
                WorkspaceDetector.populateWorkspaceDetails(first) { [weak self] updated in
                    guard let self = self else { return }
                    if !self.workspaces.isEmpty {
                        self.workspaces[0] = updated
                    }
                }
            }
        }
    }

    var onSelectWorkspace: ((Workspace) -> Void)?
    var onNewWorkspace: (() -> Void)?
    var onSelectSession: ((SmuxSession) -> Void)?
    var onNewSession: (() -> Void)?

    private var selectedIndex: Int = 0

    // MARK: - UI Components

    private let bellButton = NSButton()
    private let badgeLabel = NSTextField(labelWithString: "")
    private let guideButton = NSButton()
    private let addButton = NSButton()
    private let tableView = NSTableView()
    private let scrollView = NSScrollView()
    private var notificationPanel: NotificationPanel?

    // MARK: - Init

    override init(frame: NSRect) {
        super.init(frame: frame)
        setup()
    }
    required init?(coder: NSCoder) { fatalError() }

    private func setup() {
        wantsLayer = true
        layer?.backgroundColor = NSColor(white: 0.08, alpha: 1).cgColor

        setupHeader()
        setupTable()
        layoutUI()
    }

    // MARK: - Header (bell + badge + add)

    private func setupHeader() {
        if let bellImage = NSImage(systemSymbolName: "bell.fill", accessibilityDescription: "Notifications") {
            bellButton.image = bellImage
        } else {
            bellButton.title = "🔔"
        }
        bellButton.bezelStyle = .inline
        bellButton.isBordered = false
        bellButton.imagePosition = .imageOnly
        bellButton.contentTintColor = .secondaryLabelColor
        bellButton.target = self
        bellButton.action = #selector(bellTapped)
        bellButton.translatesAutoresizingMaskIntoConstraints = false
        addSubview(bellButton)

        badgeLabel.font = .systemFont(ofSize: 8, weight: .bold)
        badgeLabel.textColor = .white
        badgeLabel.alignment = .center
        badgeLabel.wantsLayer = true
        badgeLabel.layer?.backgroundColor = NSColor.systemRed.cgColor
        badgeLabel.layer?.cornerRadius = 7
        badgeLabel.layer?.masksToBounds = true
        badgeLabel.isHidden = true
        badgeLabel.translatesAutoresizingMaskIntoConstraints = false
        addSubview(badgeLabel)

        guideButton.title = "?"
        guideButton.bezelStyle = .inline
        guideButton.isBordered = false
        guideButton.font = .systemFont(ofSize: 14, weight: .bold)
        guideButton.contentTintColor = .secondaryLabelColor
        guideButton.target = self
        guideButton.action = #selector(guideTapped)
        guideButton.translatesAutoresizingMaskIntoConstraints = false
        addSubview(guideButton)

        addButton.title = "+"
        addButton.bezelStyle = .inline
        addButton.isBordered = false
        addButton.font = .systemFont(ofSize: 18, weight: .regular)
        addButton.contentTintColor = .secondaryLabelColor
        addButton.target = self
        addButton.action = #selector(addTapped)
        addButton.translatesAutoresizingMaskIntoConstraints = false
        addSubview(addButton)
    }

    // MARK: - Table

    private func setupTable() {
        let column = NSTableColumn(identifier: NSUserInterfaceItemIdentifier("workspace"))
        column.title = ""
        tableView.addTableColumn(column)
        tableView.headerView = nil
        tableView.backgroundColor = .clear
        tableView.selectionHighlightStyle = .none
        tableView.dataSource = self
        tableView.delegate = self
        tableView.intercellSpacing = NSSize(width: 0, height: 2)
        tableView.usesAutomaticRowHeights = true

        scrollView.documentView = tableView
        scrollView.hasVerticalScroller = true
        scrollView.drawsBackground = false
        scrollView.translatesAutoresizingMaskIntoConstraints = false
        addSubview(scrollView)
    }

    // MARK: - Layout

    private func layoutUI() {
        NSLayoutConstraint.activate([
            bellButton.topAnchor.constraint(equalTo: topAnchor, constant: 8),
            bellButton.leadingAnchor.constraint(equalTo: leadingAnchor, constant: 10),
            bellButton.widthAnchor.constraint(equalToConstant: 24),
            bellButton.heightAnchor.constraint(equalToConstant: 24),

            badgeLabel.leadingAnchor.constraint(equalTo: bellButton.trailingAnchor, constant: -10),
            badgeLabel.topAnchor.constraint(equalTo: bellButton.topAnchor, constant: -4),
            badgeLabel.widthAnchor.constraint(greaterThanOrEqualToConstant: 14),
            badgeLabel.heightAnchor.constraint(equalToConstant: 14),

            guideButton.topAnchor.constraint(equalTo: topAnchor, constant: 8),
            guideButton.trailingAnchor.constraint(equalTo: addButton.leadingAnchor, constant: -2),
            guideButton.widthAnchor.constraint(equalToConstant: 24),
            guideButton.heightAnchor.constraint(equalToConstant: 24),

            addButton.topAnchor.constraint(equalTo: topAnchor, constant: 6),
            addButton.trailingAnchor.constraint(equalTo: trailingAnchor, constant: -8),
            addButton.widthAnchor.constraint(equalToConstant: 28),
            addButton.heightAnchor.constraint(equalToConstant: 28),

            scrollView.topAnchor.constraint(equalTo: bellButton.bottomAnchor, constant: 8),
            scrollView.leadingAnchor.constraint(equalTo: leadingAnchor),
            scrollView.trailingAnchor.constraint(equalTo: trailingAnchor),
            scrollView.bottomAnchor.constraint(equalTo: bottomAnchor),
        ])
    }

    // MARK: - Badge

    private func updateBadge() {
        let unread = notifications.filter { !$0.isRead }.count
        if unread > 0 {
            badgeLabel.stringValue = "\(unread)"
            badgeLabel.isHidden = false
            bellButton.contentTintColor = .systemYellow
        } else {
            badgeLabel.isHidden = true
            bellButton.contentTintColor = .secondaryLabelColor
        }
    }

    // MARK: - Legacy compat
    func setAlerts(_ count: Int) {
        // Map to notifications
        if count > 0 && notifications.isEmpty {
            notifications = [SmuxNotification(
                id: UUID().uuidString, title: "Alert",
                body: "\(count) items need attention",
                source: "System", timestamp: Date(), isRead: false
            )]
        }
    }

    // MARK: - Actions

    @objc private func bellTapped() {
        if let panel = notificationPanel, panel.isVisible {
            panel.close()
            notificationPanel = nil
        } else {
            let panel = NotificationPanel(notifications: notifications)
            panel.onDismissAll = { [weak self] in
                self?.notifications = self?.notifications.map {
                    var n = $0; n.isRead = true; return n
                } ?? []
            }
            // Position below bell button
            let buttonRect = bellButton.convert(bellButton.bounds, to: nil)
            let screenRect = window?.convertToScreen(buttonRect) ?? .zero
            panel.setFrameTopLeftPoint(NSPoint(x: screenRect.minX - 10, y: screenRect.minY - 4))
            panel.orderFront(nil)
            notificationPanel = panel
        }
    }

    @objc private func guideTapped() {
        GuidePanel.toggle(relativeTo: window)
    }

    @objc private func addTapped() {
        onNewWorkspace?()
        onNewSession?()
    }

    // MARK: - Attention ring

    func triggerAttentionRing(workspaceIndex: Int) {
        guard workspaceIndex < workspaces.count else { return }
        workspaces[workspaceIndex].needsAttention = true
        tableView.reloadData(forRowIndexes: IndexSet(integer: workspaceIndex),
                             columnIndexes: IndexSet(integer: 0))
    }
}

// MARK: - NSTableView DataSource/Delegate

extension SidebarView: NSTableViewDataSource, NSTableViewDelegate {
    func numberOfRows(in tableView: NSTableView) -> Int { workspaces.count }

    func tableView(_ tv: NSTableView, viewFor tableColumn: NSTableColumn?, row: Int) -> NSView? {
        let ws = workspaces[row]
        let isSelected = row == selectedIndex
        return WorkspaceCellView(workspace: ws, isSelected: isSelected)
    }

    func tableViewSelectionDidChange(_ notification: Notification) {
        let row = tableView.selectedRow
        guard row >= 0, row < workspaces.count else { return }
        selectedIndex = row
        tableView.reloadData()
        onSelectWorkspace?(workspaces[row])
        // Legacy compat
        if let first = workspaces[row].sessions.first {
            onSelectSession?(first)
        }
    }
}

// MARK: - Workspace Cell View (cmux-style card)

private class WorkspaceCellView: NSView {
    private let alertRingLayer = CAShapeLayer()

    init(workspace: Workspace, isSelected: Bool) {
        super.init(frame: .zero)
        wantsLayer = true

        // Background
        if isSelected {
            layer?.backgroundColor = NSColor.systemBlue.withAlphaComponent(0.25).cgColor
        }
        layer?.cornerRadius = 4

        // Alert ring (red border for attention)
        if workspace.needsAttention {
            alertRingLayer.strokeColor = NSColor.systemRed.cgColor
            alertRingLayer.fillColor = nil
            alertRingLayer.lineWidth = 2
            alertRingLayer.cornerRadius = 4
            layer?.addSublayer(alertRingLayer)
            animateAlertRing()
        }

        // Build content
        let stack = NSStackView()
        stack.orientation = .vertical
        stack.alignment = .leading
        stack.spacing = 2
        stack.edgeInsets = NSEdgeInsets(top: 8, left: 10, bottom: 8, right: 10)
        stack.translatesAutoresizingMaskIntoConstraints = false

        // Row 1: Icon + Name + Unread badge
        let nameRow = NSStackView()
        nameRow.orientation = .horizontal
        nameRow.spacing = 4

        let iconView = NSImageView()
        if let img = NSImage(systemSymbolName: workspace.icon, accessibilityDescription: nil) {
            iconView.image = img
            iconView.contentTintColor = workspace.color
        }
        iconView.translatesAutoresizingMaskIntoConstraints = false
        NSLayoutConstraint.activate([
            iconView.widthAnchor.constraint(equalToConstant: 14),
            iconView.heightAnchor.constraint(equalToConstant: 14),
        ])
        nameRow.addArrangedSubview(iconView)

        let nameLabel = NSTextField(labelWithString: workspace.name)
        nameLabel.font = .monospacedSystemFont(ofSize: 11, weight: .bold)
        nameLabel.textColor = isSelected ? .white : .labelColor
        nameRow.addArrangedSubview(nameLabel)

        if workspace.unreadCount > 0 {
            let badge = NSTextField(labelWithString: "\(workspace.unreadCount)")
            badge.font = .systemFont(ofSize: 8, weight: .bold)
            badge.textColor = .white
            badge.wantsLayer = true
            badge.layer?.backgroundColor = NSColor.systemBlue.cgColor
            badge.layer?.cornerRadius = 6
            badge.layer?.masksToBounds = true
            badge.alignment = .center
            badge.translatesAutoresizingMaskIntoConstraints = false
            NSLayoutConstraint.activate([
                badge.widthAnchor.constraint(greaterThanOrEqualToConstant: 14),
                badge.heightAnchor.constraint(equalToConstant: 14),
            ])
            nameRow.addArrangedSubview(badge)
        }

        stack.addArrangedSubview(nameRow)

        // Row 2: Status text (if any)
        if !workspace.statusText.isEmpty {
            let statusTextLabel = NSTextField(labelWithString: String(workspace.statusText.prefix(40)))
            statusTextLabel.font = .monospacedSystemFont(ofSize: 9, weight: .regular)
            statusTextLabel.textColor = .secondaryLabelColor
            statusTextLabel.lineBreakMode = .byTruncatingTail
            stack.addArrangedSubview(statusTextLabel)
        }

        // Row 3: Status indicator
        let statusRow = NSStackView()
        statusRow.orientation = .horizontal
        statusRow.spacing = 4

        let dot = NSView()
        dot.wantsLayer = true
        dot.layer?.cornerRadius = 3
        dot.layer?.backgroundColor = workspace.status.dotColor.cgColor
        dot.translatesAutoresizingMaskIntoConstraints = false
        NSLayoutConstraint.activate([
            dot.widthAnchor.constraint(equalToConstant: 6),
            dot.heightAnchor.constraint(equalToConstant: 6),
        ])
        statusRow.addArrangedSubview(dot)

        let statusLabel = NSTextField(labelWithString: workspace.status.label)
        statusLabel.font = .monospacedSystemFont(ofSize: 9, weight: .regular)
        statusLabel.textColor = .tertiaryLabelColor
        statusRow.addArrangedSubview(statusLabel)

        stack.addArrangedSubview(statusRow)

        // Row 4: Git branches
        for branch in workspace.gitBranches.prefix(3) {
            let branchLabel = NSTextField(labelWithString: branch.displayText)
            branchLabel.font = .monospacedSystemFont(ofSize: 9, weight: .regular)
            branchLabel.textColor = .secondaryLabelColor
            branchLabel.lineBreakMode = .byTruncatingTail
            stack.addArrangedSubview(branchLabel)
        }

        // Row 5: PR alerts
        for pr in workspace.prAlerts {
            let prLabel = NSTextField(labelWithString: pr.displayText)
            prLabel.font = .monospacedSystemFont(ofSize: 9, weight: .regular)
            prLabel.textColor = pr.isUnread ? .systemOrange : .tertiaryLabelColor
            stack.addArrangedSubview(prLabel)
        }

        // Row 6: Ports
        if !workspace.ports.isEmpty {
            let portsText = workspace.ports.map { ":\($0)" }.joined(separator: ", ")
            let portsLabel = NSTextField(labelWithString: portsText)
            portsLabel.font = .monospacedSystemFont(ofSize: 9, weight: .regular)
            portsLabel.textColor = .tertiaryLabelColor
            portsLabel.lineBreakMode = .byTruncatingTail
            stack.addArrangedSubview(portsLabel)
        }

        addSubview(stack)
        NSLayoutConstraint.activate([
            stack.topAnchor.constraint(equalTo: topAnchor),
            stack.leadingAnchor.constraint(equalTo: leadingAnchor),
            stack.trailingAnchor.constraint(equalTo: trailingAnchor),
            stack.bottomAnchor.constraint(equalTo: bottomAnchor),
        ])
    }

    required init?(coder: NSCoder) { fatalError() }

    override func layout() {
        super.layout()
        alertRingLayer.frame = bounds
        alertRingLayer.path = CGPath(roundedRect: bounds.insetBy(dx: 1, dy: 1),
                                      cornerWidth: 4, cornerHeight: 4, transform: nil)
    }

    private func animateAlertRing() {
        let anim = CABasicAnimation(keyPath: "opacity")
        anim.fromValue = 1.0
        anim.toValue = 0.3
        anim.duration = 0.8
        anim.autoreverses = true
        anim.repeatCount = .greatestFiniteMagnitude
        alertRingLayer.add(anim, forKey: "pulse")
    }
}

// MARK: - Notification Panel (dropdown from bell)

class NotificationPanel: NSPanel {
    var onDismissAll: (() -> Void)?

    init(notifications: [SmuxNotification]) {
        super.init(
            contentRect: NSRect(x: 0, y: 0, width: 300, height: min(400, max(120, notifications.count * 80 + 40))),
            styleMask: [.titled, .closable, .fullSizeContentView],
            backing: .buffered, defer: false
        )
        isFloatingPanel = true
        titleVisibility = .hidden
        titlebarAppearsTransparent = true
        backgroundColor = NSColor(white: 0.12, alpha: 1)
        isMovableByWindowBackground = true

        guard let content = contentView else { return }
        content.wantsLayer = true

        let stack = NSStackView()
        stack.orientation = .vertical
        stack.alignment = .leading
        stack.spacing = 8
        stack.translatesAutoresizingMaskIntoConstraints = false

        // Header
        let headerRow = NSStackView()
        headerRow.orientation = .horizontal
        headerRow.spacing = 8

        let titleLabel = NSTextField(labelWithString: "알림")
        titleLabel.font = .systemFont(ofSize: 13, weight: .bold)
        titleLabel.textColor = .labelColor
        headerRow.addArrangedSubview(titleLabel)

        let spacer = NSView()
        spacer.translatesAutoresizingMaskIntoConstraints = false
        spacer.setContentHuggingPriority(.defaultLow, for: .horizontal)
        headerRow.addArrangedSubview(spacer)

        let clearButton = NSButton(title: "모두 지우기", target: nil, action: nil)
        clearButton.bezelStyle = .inline
        clearButton.font = .systemFont(ofSize: 10, weight: .regular)
        clearButton.target = self
        clearButton.action = #selector(clearAll)
        headerRow.addArrangedSubview(clearButton)

        stack.addArrangedSubview(headerRow)
        headerRow.translatesAutoresizingMaskIntoConstraints = false
        NSLayoutConstraint.activate([
            headerRow.leadingAnchor.constraint(equalTo: stack.leadingAnchor),
            headerRow.trailingAnchor.constraint(equalTo: stack.trailingAnchor),
        ])

        // Notification cards
        if notifications.isEmpty {
            let emptyLabel = NSTextField(labelWithString: "알림 없음")
            emptyLabel.font = .systemFont(ofSize: 11, weight: .regular)
            emptyLabel.textColor = .tertiaryLabelColor
            stack.addArrangedSubview(emptyLabel)
        } else {
            for notif in notifications.suffix(10) {
                let card = NotificationCardView(notification: notif)
                card.translatesAutoresizingMaskIntoConstraints = false
                NSLayoutConstraint.activate([
                    card.leadingAnchor.constraint(equalTo: stack.leadingAnchor),
                    card.trailingAnchor.constraint(equalTo: stack.trailingAnchor),
                ])
                stack.addArrangedSubview(card)
            }
        }

        let scrollView = NSScrollView()
        scrollView.documentView = stack
        scrollView.hasVerticalScroller = true
        scrollView.drawsBackground = false
        scrollView.translatesAutoresizingMaskIntoConstraints = false
        content.addSubview(scrollView)

        NSLayoutConstraint.activate([
            scrollView.topAnchor.constraint(equalTo: content.topAnchor, constant: 28),
            scrollView.leadingAnchor.constraint(equalTo: content.leadingAnchor, constant: 12),
            scrollView.trailingAnchor.constraint(equalTo: content.trailingAnchor, constant: -12),
            scrollView.bottomAnchor.constraint(equalTo: content.bottomAnchor, constant: -8),
        ])
    }

    @objc private func clearAll() {
        onDismissAll?()
        close()
    }
}

// MARK: - Notification Card

private class NotificationCardView: NSView {
    init(notification: SmuxNotification) {
        super.init(frame: .zero)
        wantsLayer = true
        layer?.backgroundColor = NSColor(white: 0.15, alpha: 1).cgColor
        layer?.cornerRadius = 6

        // Red border ring for unread
        if !notification.isRead {
            layer?.borderColor = NSColor.systemRed.withAlphaComponent(0.6).cgColor
            layer?.borderWidth = 1.5
        }

        let stack = NSStackView()
        stack.orientation = .vertical
        stack.alignment = .leading
        stack.spacing = 2
        stack.edgeInsets = NSEdgeInsets(top: 8, left: 10, bottom: 8, right: 10)
        stack.translatesAutoresizingMaskIntoConstraints = false

        // Title row: dot + title + time
        let titleRow = NSStackView()
        titleRow.orientation = .horizontal
        titleRow.spacing = 6

        if !notification.isRead {
            let dot = NSView()
            dot.wantsLayer = true
            dot.layer?.cornerRadius = 3
            dot.layer?.backgroundColor = NSColor.systemBlue.cgColor
            dot.translatesAutoresizingMaskIntoConstraints = false
            NSLayoutConstraint.activate([
                dot.widthAnchor.constraint(equalToConstant: 6),
                dot.heightAnchor.constraint(equalToConstant: 6),
            ])
            titleRow.addArrangedSubview(dot)
        }

        let titleLabel = NSTextField(labelWithString: notification.title)
        titleLabel.font = .systemFont(ofSize: 11, weight: .bold)
        titleLabel.textColor = .labelColor
        titleRow.addArrangedSubview(titleLabel)

        let spacer = NSView()
        spacer.setContentHuggingPriority(.defaultLow, for: .horizontal)
        titleRow.addArrangedSubview(spacer)

        let formatter = DateFormatter()
        formatter.dateFormat = "a h:mm"
        formatter.locale = Locale(identifier: "ko_KR")
        let timeLabel = NSTextField(labelWithString: formatter.string(from: notification.timestamp))
        timeLabel.font = .systemFont(ofSize: 9, weight: .regular)
        timeLabel.textColor = .tertiaryLabelColor
        titleRow.addArrangedSubview(timeLabel)

        stack.addArrangedSubview(titleRow)
        titleRow.translatesAutoresizingMaskIntoConstraints = false

        // Body
        let bodyLabel = NSTextField(labelWithString: notification.body)
        bodyLabel.font = .systemFont(ofSize: 10, weight: .regular)
        bodyLabel.textColor = .secondaryLabelColor
        bodyLabel.lineBreakMode = .byTruncatingTail
        bodyLabel.maximumNumberOfLines = 2
        stack.addArrangedSubview(bodyLabel)

        // Source
        let sourceLabel = NSTextField(labelWithString: notification.source)
        sourceLabel.font = .systemFont(ofSize: 9, weight: .regular)
        sourceLabel.textColor = .tertiaryLabelColor
        stack.addArrangedSubview(sourceLabel)

        addSubview(stack)
        NSLayoutConstraint.activate([
            stack.topAnchor.constraint(equalTo: topAnchor),
            stack.leadingAnchor.constraint(equalTo: leadingAnchor),
            stack.trailingAnchor.constraint(equalTo: trailingAnchor),
            stack.bottomAnchor.constraint(equalTo: bottomAnchor),
            heightAnchor.constraint(greaterThanOrEqualToConstant: 60),
        ])
    }

    required init?(coder: NSCoder) { fatalError() }
}
