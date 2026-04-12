import AppKit

/// Dialog for starting a new multi-agent relay session.
/// This is THE core product feature — planner/verifier ping-pong over real PTYs.
///
/// Flow:
/// 1. User enters a task description
/// 2. Picks planner + verifier agents
/// 3. Configures rounds, approval mode
/// 4. smux starts the relay: planner → verifier → planner → ... until consensus
///
/// The dialog sends StartSession IPC to the daemon, which manages the pipeline.
class NewSessionDialog: NSPanel {

    var onStart: ((SessionConfig) -> Void)?

    struct SessionConfig {
        let task: String
        let planner: String
        let verifier: String
        let maxRounds: Int
        let autoApprove: Bool
        let additionalVerifiers: [String]
        let workers: [String]
    }

    private let taskField = NSTextField()
    private let plannerPopup = NSPopUpButton()
    private let verifierPopup = NSPopUpButton()
    private let roundsStepper = NSStepper()
    private let roundsLabel = NSTextField(labelWithString: "5")
    private let autoApproveCheck = NSButton(checkboxWithTitle: "자동 승인 (Full Auto)", target: nil, action: nil)
    private let addVerifierCheck = NSButton(checkboxWithTitle: "추가 검증자 (Multi-Verify)", target: nil, action: nil)
    private let startButton = NSButton(title: "세션 시작", target: nil, action: nil)
    private let cancelButton = NSButton(title: "취소", target: nil, action: nil)

    private static let agents = [
        ("claude", "Claude Code (Anthropic)"),
        ("codex", "Codex CLI (OpenAI)"),
        ("gemini", "Gemini CLI (Google)"),
        ("aider", "Aider"),
        ("custom", "커스텀..."),
    ]

    private static let templates = [
        ("quick", "Quick Fix — 빠른 버그 수정", "claude", "codex", 3, true),
        ("full", "Full Pipeline — 전체 개발 파이프라인", "claude", "codex", 10, false),
        ("multi", "Multi-Verify — 다중 검증", "claude", "gemini", 5, false),
    ]

    init() {
        super.init(
            contentRect: NSRect(x: 0, y: 0, width: 480, height: 520),
            styleMask: [.titled, .closable, .fullSizeContentView],
            backing: .buffered, defer: false
        )
        title = "새 릴레이 세션"
        titlebarAppearsTransparent = true
        isFloatingPanel = true
        isMovableByWindowBackground = true
        backgroundColor = NSColor(white: 0.10, alpha: 1)
        isReleasedWhenClosed = false
        setupUI()
    }

    private func setupUI() {
        guard let content = contentView else { return }
        content.wantsLayer = true

        let stack = NSStackView()
        stack.orientation = .vertical
        stack.alignment = .leading
        stack.spacing = 12
        stack.edgeInsets = NSEdgeInsets(top: 40, left: 24, bottom: 16, right: 24)
        stack.translatesAutoresizingMaskIntoConstraints = false
        content.addSubview(stack)

        NSLayoutConstraint.activate([
            stack.topAnchor.constraint(equalTo: content.topAnchor),
            stack.leadingAnchor.constraint(equalTo: content.leadingAnchor),
            stack.trailingAnchor.constraint(equalTo: content.trailingAnchor),
            stack.bottomAnchor.constraint(equalTo: content.bottomAnchor),
        ])

        // --- Explanation ---
        let explainLabel = NSTextField(wrappingLabelWithString:
            "smux 릴레이: 하나의 에이전트가 계획하고, 다른 에이전트가 검증합니다. " +
            "합의에 도달하거나 최대 라운드까지 핑퐁합니다.")
        explainLabel.font = .systemFont(ofSize: 11, weight: .regular)
        explainLabel.textColor = .secondaryLabelColor
        stack.addArrangedSubview(explainLabel)
        explainLabel.translatesAutoresizingMaskIntoConstraints = false
        NSLayoutConstraint.activate([
            explainLabel.widthAnchor.constraint(equalToConstant: 430),
        ])

        // --- Templates ---
        let templateLabel = makeLabel("프리셋")
        stack.addArrangedSubview(templateLabel)

        let templateStack = NSStackView()
        templateStack.orientation = .horizontal
        templateStack.spacing = 8
        for (i, tmpl) in Self.templates.enumerated() {
            let btn = NSButton(title: tmpl.1, target: self, action: #selector(templateSelected(_:)))
            btn.bezelStyle = .inline
            btn.font = .systemFont(ofSize: 10, weight: .medium)
            btn.tag = i
            templateStack.addArrangedSubview(btn)
        }
        stack.addArrangedSubview(templateStack)

        // --- Task ---
        let taskLabel = makeLabel("작업 설명")
        stack.addArrangedSubview(taskLabel)

        taskField.placeholderString = "예: 블로그 자동화 도구 만들어줘"
        taskField.font = .systemFont(ofSize: 13, weight: .regular)
        taskField.textColor = .labelColor
        taskField.backgroundColor = NSColor(white: 0.15, alpha: 1)
        taskField.drawsBackground = true
        taskField.isBordered = true
        taskField.isBezeled = true
        taskField.bezelStyle = .roundedBezel
        taskField.translatesAutoresizingMaskIntoConstraints = false
        stack.addArrangedSubview(taskField)
        NSLayoutConstraint.activate([
            taskField.widthAnchor.constraint(equalToConstant: 430),
            taskField.heightAnchor.constraint(equalToConstant: 28),
        ])

        // --- Planner ---
        let plannerLabel = makeLabel("플래너 (계획 에이전트)")
        stack.addArrangedSubview(plannerLabel)

        for agent in Self.agents {
            plannerPopup.addItem(withTitle: agent.1)
            plannerPopup.lastItem?.representedObject = agent.0
        }
        plannerPopup.selectItem(at: 0) // claude
        plannerPopup.translatesAutoresizingMaskIntoConstraints = false
        stack.addArrangedSubview(plannerPopup)
        NSLayoutConstraint.activate([plannerPopup.widthAnchor.constraint(equalToConstant: 430)])

        // --- Verifier ---
        let verifierLabel = makeLabel("검증자 (리뷰 에이전트)")
        stack.addArrangedSubview(verifierLabel)

        for agent in Self.agents {
            verifierPopup.addItem(withTitle: agent.1)
            verifierPopup.lastItem?.representedObject = agent.0
        }
        verifierPopup.selectItem(at: 1) // codex
        verifierPopup.translatesAutoresizingMaskIntoConstraints = false
        stack.addArrangedSubview(verifierPopup)
        NSLayoutConstraint.activate([verifierPopup.widthAnchor.constraint(equalToConstant: 430)])

        // --- Rounds ---
        let roundsRow = NSStackView()
        roundsRow.orientation = .horizontal
        roundsRow.spacing = 8

        let roundsTitle = makeLabel("최대 라운드")
        roundsRow.addArrangedSubview(roundsTitle)

        roundsStepper.minValue = 1
        roundsStepper.maxValue = 20
        roundsStepper.intValue = 5
        roundsStepper.target = self
        roundsStepper.action = #selector(roundsChanged)
        roundsRow.addArrangedSubview(roundsStepper)

        roundsLabel.font = .monospacedSystemFont(ofSize: 12, weight: .bold)
        roundsLabel.textColor = .labelColor
        roundsRow.addArrangedSubview(roundsLabel)

        stack.addArrangedSubview(roundsRow)

        // --- Options ---
        autoApproveCheck.state = .on
        autoApproveCheck.font = .systemFont(ofSize: 11)
        stack.addArrangedSubview(autoApproveCheck)

        addVerifierCheck.state = .off
        addVerifierCheck.font = .systemFont(ofSize: 11)
        stack.addArrangedSubview(addVerifierCheck)

        // --- Relay diagram ---
        let diagramLabel = NSTextField(wrappingLabelWithString:
            "┌─────────┐    ┌──────────┐\n" +
            "│ Planner  │───▶│ Verifier │\n" +
            "│ (claude) │◀───│ (codex)  │\n" +
            "└─────────┘    └──────────┘\n" +
            "    Round 1 → 2 → ... → N")
        diagramLabel.font = .monospacedSystemFont(ofSize: 10, weight: .regular)
        diagramLabel.textColor = .tertiaryLabelColor
        diagramLabel.alignment = .center
        diagramLabel.translatesAutoresizingMaskIntoConstraints = false
        stack.addArrangedSubview(diagramLabel)
        NSLayoutConstraint.activate([diagramLabel.widthAnchor.constraint(equalToConstant: 430)])

        // --- Buttons ---
        let buttonRow = NSStackView()
        buttonRow.orientation = .horizontal
        buttonRow.spacing = 12

        cancelButton.bezelStyle = .rounded
        cancelButton.target = self
        cancelButton.action = #selector(cancelTapped)
        buttonRow.addArrangedSubview(cancelButton)

        startButton.bezelStyle = .rounded
        startButton.keyEquivalent = "\r"
        startButton.contentTintColor = .systemGreen
        startButton.font = .systemFont(ofSize: 13, weight: .bold)
        startButton.target = self
        startButton.action = #selector(startTapped)
        buttonRow.addArrangedSubview(startButton)

        stack.addArrangedSubview(buttonRow)
    }

    // MARK: - Actions

    @objc private func templateSelected(_ sender: NSButton) {
        let idx = sender.tag
        guard idx < Self.templates.count else { return }
        let tmpl = Self.templates[idx]
        // Set planner
        for (i, agent) in Self.agents.enumerated() {
            if agent.0 == tmpl.2 { plannerPopup.selectItem(at: i) }
            if agent.0 == tmpl.3 { verifierPopup.selectItem(at: i) }
        }
        roundsStepper.intValue = Int32(tmpl.4)
        roundsLabel.stringValue = "\(tmpl.4)"
        autoApproveCheck.state = tmpl.5 ? .on : .off
    }

    @objc private func roundsChanged() {
        roundsLabel.stringValue = "\(roundsStepper.intValue)"
    }

    @objc private func cancelTapped() {
        close()
    }

    @objc private func startTapped() {
        let task = taskField.stringValue.trimmingCharacters(in: .whitespaces)
        guard !task.isEmpty else {
            taskField.placeholderString = "⚠ 작업을 입력하세요"
            return
        }

        let planner = plannerPopup.selectedItem?.representedObject as? String ?? "claude"
        let verifier = verifierPopup.selectedItem?.representedObject as? String ?? "codex"

        let config = SessionConfig(
            task: task,
            planner: planner,
            verifier: verifier,
            maxRounds: Int(roundsStepper.intValue),
            autoApprove: autoApproveCheck.state == .on,
            additionalVerifiers: addVerifierCheck.state == .on ? ["gemini"] : [],
            workers: []
        )

        onStart?(config)
        close()
    }

    // MARK: - Helpers

    private func makeLabel(_ text: String) -> NSTextField {
        let label = NSTextField(labelWithString: text)
        label.font = .systemFont(ofSize: 11, weight: .bold)
        label.textColor = .secondaryLabelColor
        return label
    }

    override func keyDown(with event: NSEvent) {
        if event.keyCode == 53 { close() }
        else { super.keyDown(with: event) }
    }
}
