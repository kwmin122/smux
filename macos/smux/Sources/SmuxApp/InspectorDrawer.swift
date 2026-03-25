import AppKit

/// Right-side inspector drawer showing transcript, findings, diffs.
class InspectorDrawer: NSView {
    private let tabBar = NSSegmentedControl(labels: ["Transcript", "Findings", "Diffs", "Files"], trackingMode: .selectOne, target: nil, action: nil)
    private let contentView = NSScrollView()
    private let textView = NSTextView()

    var transcript: String = "" { didSet { updateContent() } }
    var findings: [(severity: String, message: String)] = [] { didSet { updateContent() } }

    override init(frame: NSRect) {
        super.init(frame: frame)
        setupUI()
    }
    required init?(coder: NSCoder) { fatalError() }

    private func setupUI() {
        wantsLayer = true
        layer?.backgroundColor = NSColor(white: 0.08, alpha: 1).cgColor

        tabBar.selectedSegment = 0
        tabBar.target = self
        tabBar.action = #selector(tabChanged)
        tabBar.font = .monospacedSystemFont(ofSize: 9, weight: .regular)
        tabBar.translatesAutoresizingMaskIntoConstraints = false
        addSubview(tabBar)

        textView.isEditable = false
        textView.backgroundColor = .clear
        textView.textColor = .labelColor
        textView.font = .monospacedSystemFont(ofSize: 11, weight: .regular)
        contentView.documentView = textView
        contentView.hasVerticalScroller = true
        contentView.drawsBackground = false
        contentView.translatesAutoresizingMaskIntoConstraints = false
        addSubview(contentView)

        NSLayoutConstraint.activate([
            tabBar.topAnchor.constraint(equalTo: topAnchor, constant: 4),
            tabBar.centerXAnchor.constraint(equalTo: centerXAnchor),
            contentView.topAnchor.constraint(equalTo: tabBar.bottomAnchor, constant: 4),
            contentView.leadingAnchor.constraint(equalTo: leadingAnchor, constant: 4),
            contentView.trailingAnchor.constraint(equalTo: trailingAnchor, constant: -4),
            contentView.bottomAnchor.constraint(equalTo: bottomAnchor),
        ])
    }

    private func updateContent() {
        switch tabBar.selectedSegment {
        case 0: textView.string = transcript.isEmpty ? "(no transcript)" : transcript
        case 1:
            let text = findings.isEmpty ? "(no findings)" :
                findings.map { "[\($0.severity)] \($0.message)" }.joined(separator: "\n")
            textView.string = text
        case 2: textView.string = "(diffs not yet connected)"
        case 3: textView.string = "(files not yet connected)"
        default: break
        }
    }

    @objc private func tabChanged() { updateContent() }

    func toggle() { isHidden = !isHidden }
}
