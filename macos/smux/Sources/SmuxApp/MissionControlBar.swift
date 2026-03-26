import AppKit

/// Bottom control bar with Approve/Pause/Retry/Resume buttons + status.
class MissionControlBar: NSView {
    private let pingPongButton = NSButton(title: "🔄 Ping-pong", target: nil, action: nil)
    private let approveButton = NSButton(title: "Approve", target: nil, action: nil)
    private let pauseButton = NSButton(title: "Pause", target: nil, action: nil)
    private let retryButton = NSButton(title: "Retry", target: nil, action: nil)
    private let statusLabel = NSTextField(labelWithString: "Ready")
    private let roundLabel = NSTextField(labelWithString: "")

    var onPingPong: (() -> Void)?
    var onApprove: (() -> Void)?
    var onPause: (() -> Void)?
    var onRetry: (() -> Void)?

    override init(frame: NSRect) {
        super.init(frame: NSRect(x: 0, y: 0, width: frame.width, height: 32))
        setupUI()
    }
    required init?(coder: NSCoder) { fatalError() }

    private func setupUI() {
        wantsLayer = true
        layer?.backgroundColor = NSColor(white: 0.1, alpha: 1).cgColor

        pingPongButton.bezelStyle = .inline
        pingPongButton.contentTintColor = .systemCyan
        pingPongButton.font = .monospacedSystemFont(ofSize: 10, weight: .bold)
        pingPongButton.target = self
        pingPongButton.action = #selector(pingPongTapped)

        approveButton.bezelStyle = .inline
        approveButton.contentTintColor = .systemGreen
        approveButton.font = .monospacedSystemFont(ofSize: 10, weight: .bold)
        approveButton.target = self
        approveButton.action = #selector(approveTapped)

        pauseButton.bezelStyle = .inline
        pauseButton.contentTintColor = .systemYellow
        pauseButton.font = .monospacedSystemFont(ofSize: 10, weight: .regular)
        pauseButton.target = self
        pauseButton.action = #selector(pauseTapped)

        retryButton.bezelStyle = .inline
        retryButton.contentTintColor = .systemOrange
        retryButton.font = .monospacedSystemFont(ofSize: 10, weight: .regular)
        retryButton.target = self
        retryButton.action = #selector(retryTapped)

        statusLabel.font = .monospacedSystemFont(ofSize: 10, weight: .regular)
        statusLabel.textColor = .secondaryLabelColor

        roundLabel.font = .monospacedSystemFont(ofSize: 9, weight: .regular)
        roundLabel.textColor = .tertiaryLabelColor

        let leftStack = NSStackView(views: [pingPongButton, approveButton, pauseButton, retryButton])
        leftStack.spacing = 4

        let rightStack = NSStackView(views: [statusLabel, roundLabel])
        rightStack.spacing = 8

        let mainStack = NSStackView(views: [leftStack, rightStack])
        mainStack.orientation = .horizontal
        mainStack.distribution = .equalSpacing
        mainStack.edgeInsets = NSEdgeInsets(top: 4, left: 12, bottom: 4, right: 12)
        mainStack.translatesAutoresizingMaskIntoConstraints = false
        addSubview(mainStack)

        NSLayoutConstraint.activate([
            mainStack.leadingAnchor.constraint(equalTo: leadingAnchor),
            mainStack.trailingAnchor.constraint(equalTo: trailingAnchor),
            mainStack.topAnchor.constraint(equalTo: topAnchor),
            mainStack.bottomAnchor.constraint(equalTo: bottomAnchor),
        ])
    }

    func update(status: String, round: Int, maxRounds: Int, isPaused: Bool) {
        statusLabel.stringValue = status
        roundLabel.stringValue = maxRounds > 0 ? "R\(round)/\(maxRounds)" : ""
        pauseButton.title = isPaused ? "Resume" : "Pause"
    }

    func setPingPongActive(_ active: Bool) {
        if active {
            pingPongButton.title = "🔴 Stop"
            pingPongButton.contentTintColor = .systemRed
        } else {
            pingPongButton.title = "🔄 Ping-pong"
            pingPongButton.contentTintColor = .systemCyan
        }
    }

    @objc private func pingPongTapped() { onPingPong?() }
    @objc private func approveTapped() { onApprove?() }
    @objc private func pauseTapped() { onPause?() }
    @objc private func retryTapped() { onRetry?() }
}
