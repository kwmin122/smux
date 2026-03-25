import AppKit

/// Floating search bar for terminal text search.
/// Appears at top of terminal when ⌘F is pressed.
class SearchBar: NSView {
    private let searchField = NSTextField()
    private let matchLabel = NSTextField(labelWithString: "0/0")
    private let prevButton = NSButton(title: "▲", target: nil, action: nil)
    private let nextButton = NSButton(title: "▼", target: nil, action: nil)
    private let closeButton = NSButton(title: "✕", target: nil, action: nil)

    var onSearch: ((String) -> Void)?
    var onNext: (() -> Void)?
    var onPrev: (() -> Void)?
    var onClose: (() -> Void)?

    override init(frame: NSRect) {
        super.init(frame: NSRect(x: 0, y: 0, width: frame.width, height: 32))
        setupUI()
    }

    required init?(coder: NSCoder) { fatalError() }

    private func setupUI() {
        wantsLayer = true
        layer?.backgroundColor = NSColor(white: 0.15, alpha: 0.95).cgColor

        searchField.placeholderString = "Search..."
        searchField.font = .monospacedSystemFont(ofSize: 12, weight: .regular)
        searchField.focusRingType = .none
        searchField.target = self
        searchField.action = #selector(searchChanged)

        matchLabel.font = .monospacedSystemFont(ofSize: 10, weight: .regular)
        matchLabel.textColor = .secondaryLabelColor

        prevButton.target = self
        prevButton.action = #selector(prevMatch)
        prevButton.bezelStyle = .inline
        prevButton.font = .systemFont(ofSize: 10)

        nextButton.target = self
        nextButton.action = #selector(nextMatch)
        nextButton.bezelStyle = .inline
        nextButton.font = .systemFont(ofSize: 10)

        closeButton.target = self
        closeButton.action = #selector(closeSearch)
        closeButton.bezelStyle = .inline
        closeButton.font = .systemFont(ofSize: 10)

        let stack = NSStackView(views: [searchField, matchLabel, prevButton, nextButton, closeButton])
        stack.orientation = .horizontal
        stack.spacing = 4
        stack.edgeInsets = NSEdgeInsets(top: 4, left: 8, bottom: 4, right: 8)
        stack.translatesAutoresizingMaskIntoConstraints = false
        addSubview(stack)

        NSLayoutConstraint.activate([
            stack.leadingAnchor.constraint(equalTo: leadingAnchor),
            stack.trailingAnchor.constraint(equalTo: trailingAnchor),
            stack.topAnchor.constraint(equalTo: topAnchor),
            stack.bottomAnchor.constraint(equalTo: bottomAnchor),
            searchField.widthAnchor.constraint(greaterThanOrEqualToConstant: 200),
        ])
    }

    func show() {
        isHidden = false
        window?.makeFirstResponder(searchField)
    }

    func hide() {
        isHidden = true
        searchField.stringValue = ""
        onClose?()
    }

    func updateMatchCount(current: Int, total: Int) {
        matchLabel.stringValue = "\(current)/\(total)"
    }

    @objc private func searchChanged() {
        onSearch?(searchField.stringValue)
    }
    @objc private func prevMatch() { onPrev?() }
    @objc private func nextMatch() { onNext?() }
    @objc private func closeSearch() { hide() }
}
