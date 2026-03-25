import AppKit

/// ⌘P command palette — quick action search overlay.
class CommandPalette: NSPanel {
    private let searchField = NSTextField()
    private let resultsList = NSTableView()
    private var allCommands: [(name: String, action: () -> Void)] = []
    private var filtered: [(name: String, action: () -> Void)] = []

    init() {
        super.init(
            contentRect: NSRect(x: 0, y: 0, width: 400, height: 300),
            styleMask: [.titled, .fullSizeContentView],
            backing: .buffered,
            defer: false
        )
        isFloatingPanel = true
        titlebarAppearsTransparent = true
        titleVisibility = .hidden
        backgroundColor = NSColor(white: 0.12, alpha: 0.98)
        level = .floating
        setupUI()
    }

    private func setupUI() {
        searchField.placeholderString = "Type a command..."
        searchField.font = .monospacedSystemFont(ofSize: 14, weight: .regular)
        searchField.focusRingType = .none
        searchField.target = self
        searchField.action = #selector(filterChanged)
        searchField.translatesAutoresizingMaskIntoConstraints = false
        contentView?.addSubview(searchField)

        let column = NSTableColumn(identifier: NSUserInterfaceItemIdentifier("cmd"))
        resultsList.addTableColumn(column)
        resultsList.headerView = nil
        resultsList.backgroundColor = .clear
        resultsList.dataSource = self
        resultsList.delegate = self
        resultsList.rowHeight = 28
        resultsList.target = self
        resultsList.doubleAction = #selector(executeSelected)

        let scroll = NSScrollView()
        scroll.documentView = resultsList
        scroll.hasVerticalScroller = true
        scroll.drawsBackground = false
        scroll.translatesAutoresizingMaskIntoConstraints = false
        contentView?.addSubview(scroll)

        NSLayoutConstraint.activate([
            searchField.topAnchor.constraint(equalTo: contentView!.topAnchor, constant: 8),
            searchField.leadingAnchor.constraint(equalTo: contentView!.leadingAnchor, constant: 12),
            searchField.trailingAnchor.constraint(equalTo: contentView!.trailingAnchor, constant: -12),
            scroll.topAnchor.constraint(equalTo: searchField.bottomAnchor, constant: 8),
            scroll.leadingAnchor.constraint(equalTo: contentView!.leadingAnchor),
            scroll.trailingAnchor.constraint(equalTo: contentView!.trailingAnchor),
            scroll.bottomAnchor.constraint(equalTo: contentView!.bottomAnchor),
        ])
    }

    func show(commands: [(name: String, action: () -> Void)], relativeTo window: NSWindow?) {
        self.allCommands = commands
        self.filtered = commands
        searchField.stringValue = ""
        resultsList.reloadData()

        if let w = window {
            let wFrame = w.frame
            let x = wFrame.midX - 200
            let y = wFrame.midY
            setFrame(NSRect(x: x, y: y, width: 400, height: 300), display: true)
        }

        makeKeyAndOrderFront(nil)
        makeFirstResponder(searchField)
    }

    @objc private func filterChanged() {
        let query = searchField.stringValue.lowercased()
        filtered = query.isEmpty ? allCommands : allCommands.filter { $0.name.lowercased().contains(query) }
        resultsList.reloadData()
    }

    @objc private func executeSelected() {
        let row = resultsList.selectedRow
        guard row >= 0, row < filtered.count else { return }
        orderOut(nil)
        filtered[row].action()
    }

    override func cancelOperation(_ sender: Any?) {
        orderOut(nil)
    }
}

extension CommandPalette: NSTableViewDataSource, NSTableViewDelegate {
    func numberOfRows(in tableView: NSTableView) -> Int { filtered.count }

    func tableView(_ tableView: NSTableView, viewFor tableColumn: NSTableColumn?, row: Int) -> NSView? {
        let label = NSTextField(labelWithString: filtered[row].name)
        label.font = .monospacedSystemFont(ofSize: 12, weight: .regular)
        label.textColor = .labelColor
        return label
    }
}
