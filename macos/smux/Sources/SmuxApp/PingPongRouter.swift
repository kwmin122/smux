import AppKit

/// Ping-pong router — captures terminal output from EXEC mode ghostty panes,
/// detects turn-complete boundaries, and delivers cleaned text for relay injection.
class PingPongRouter {

    enum State: String {
        case idle = "Idle"
        case waitingForOutput = "Waiting..."
        case paneASpeaking = "A → B"
        case paneBSpeaking = "B → A"
        case paused = "Paused"
    }

    // MARK: - Public state (read by WorkspaceWindowController)

    private(set) var state: State = .idle
    private(set) var round: Int = 0
    private(set) var maxRounds: Int = 20
    private(set) var isActive: Bool = false

    private weak var paneA: GhosttyTerminalView?
    private weak var paneB: GhosttyTerminalView?

    var paneALabel: String = "A"
    var paneBLabel: String = "B"

    // MARK: - Callbacks (wired by WorkspaceWindowController.togglePingPong)

    var onStateChanged: ((State, Int) -> Void)?
    var onTurnComplete: ((String, String) -> Void)?  // (speakerLabel, cleanedOutput)
    var onSessionComplete: ((Int) -> Void)?

    // MARK: - Capture state

    /// Which pane is currently being captured (alternates A/B each turn)
    private var currentSpeaker: String = "A"

    /// Accumulated text from the current turn's polling deltas
    private var currentTurnText: String = ""

    /// Viewport snapshot taken at the start of a turn (used to compute delta output)
    private var baselineText: String = ""

    /// Silence timeout: if no text change for this duration, treat as turn-complete
    private let silenceThreshold: TimeInterval = 2.0

    /// Cancellable work item for silence timeout
    private var silenceWorkItem: DispatchWorkItem?

    /// NotificationCenter observer token for COMMAND_FINISHED
    private var commandFinishedObserver: NSObjectProtocol?

    // MARK: - Init

    init(paneA: GhosttyTerminalView, paneB: GhosttyTerminalView, maxRounds: Int = 20) {
        self.paneA = paneA
        self.paneB = paneB
        self.maxRounds = maxRounds
    }

    deinit {
        stop()
    }

    // MARK: - Lifecycle

    func start() {
        isActive = true
        round = 0
        currentSpeaker = "A"
        currentTurnText = ""
        updateState(.waitingForOutput)

        NSLog("[pingpong] started — polling paneA at 4 Hz, listening for COMMAND_FINISHED")

        // Subscribe to OSC 133 COMMAND_FINISHED (primary turn-complete signal)
        commandFinishedObserver = NotificationCenter.default.addObserver(
            forName: .ghosttyCommandFinished,
            object: nil,
            queue: .main
        ) { [weak self] notification in
            self?.handleCommandFinished(notification)
        }

        // Start polling the active pane
        startCapturingCurrentPane()
    }

    func stop() {
        isActive = false
        currentTurnText = ""

        // Stop polling
        paneA?.stopCapturing()
        paneB?.stopCapturing()

        // Cancel silence timeout
        silenceWorkItem?.cancel()
        silenceWorkItem = nil

        // Remove notification observer
        if let obs = commandFinishedObserver {
            NotificationCenter.default.removeObserver(obs)
            commandFinishedObserver = nil
        }

        updateState(.idle)
        NSLog("[pingpong] stopped — cleanup complete")
    }

    func pause() {
        guard isActive else { return }
        paneA?.stopCapturing()
        paneB?.stopCapturing()
        silenceWorkItem?.cancel()
        updateState(.paused)
        NSLog("[pingpong] paused")
    }

    func resume() {
        guard isActive, state == .paused else { return }
        updateState(.waitingForOutput)
        startCapturingCurrentPane()
        NSLog("[pingpong] resumed")
    }

    // MARK: - Capture

    /// Start polling the current speaker's pane.
    private func startCapturingCurrentPane() {
        let pane = (currentSpeaker == "A") ? paneA : paneB
        let speakerState: State = (currentSpeaker == "A") ? .paneASpeaking : .paneBSpeaking
        updateState(speakerState)

        // Snapshot baseline before polling starts — delta = final - baseline
        baselineText = pane?.captureViewportText().flatMap { ANSIStripper.strip($0) } ?? ""

        pane?.startCapturing { [weak self] newText in
            self?.handleNewOutput(newText)
        }
    }

    /// Called by the polling timer when new (ANSI-stripped) text arrives.
    private func handleNewOutput(_ newText: String) {
        guard isActive, state != .paused else { return }

        // Store the latest snapshot as current turn text
        currentTurnText = newText

        // Reset silence timer — text is still changing
        resetSilenceTimer()
    }

    // MARK: - Turn-Complete Detection

    /// OSC 133 COMMAND_FINISHED — primary (authoritative) turn-complete signal.
    private func handleCommandFinished(_ notification: Notification) {
        guard isActive, state != .paused else { return }

        let exitCode = notification.userInfo?["exit_code"] as? Int ?? -1
        NSLog("[pingpong] COMMAND_FINISHED received — exit=%d, processing turn-complete", exitCode)

        // Cancel silence timeout to prevent double-fire
        silenceWorkItem?.cancel()
        silenceWorkItem = nil

        processTurnComplete()
    }

    /// Silence timeout — fallback turn-complete signal when OSC 133 is unavailable.
    private func resetSilenceTimer() {
        silenceWorkItem?.cancel()
        let item = DispatchWorkItem { [weak self] in
            guard let self = self, self.isActive, self.state != .paused else { return }
            NSLog("[pingpong] silence timeout (%.1fs) — treating as turn-complete", self.silenceThreshold)
            DispatchQueue.main.async {
                self.processTurnComplete()
            }
        }
        silenceWorkItem = item
        DispatchQueue.global().asyncAfter(deadline: .now() + silenceThreshold, execute: item)
    }

    /// Process a completed turn: deliver output, inject into target pane, advance round, switch panes.
    private func processTurnComplete() {
        guard isActive else { return }

        let speaker = currentSpeaker
        let label = (speaker == "A") ? paneALabel : paneBLabel

        // Stop capturing the current pane
        if speaker == "A" {
            paneA?.stopCapturing()
        } else {
            paneB?.stopCapturing()
        }

        // Extract delta: new output only (strip baseline prefix)
        let delta = extractDelta(full: currentTurnText, baseline: baselineText)

        // Deliver the turn output + inject into the OTHER pane (relay)
        if !delta.isEmpty {
            onTurnComplete?(label, delta)

            // RELAY INJECTION: send captured output to the target pane's stdin
            let targetPane = (speaker == "A") ? paneB : paneA
            targetPane?.sendText(delta + "\n")
            NSLog("[pingpong] turn complete — speaker=%@ delta=%d chars, injected into %@",
                  label, delta.count, speaker == "A" ? paneBLabel : paneALabel)
        } else {
            NSLog("[pingpong] turn complete — speaker=%@ (empty delta, skipping relay)", label)
        }

        // Advance round
        round += 1

        // Check if session is complete
        if round >= maxRounds {
            isActive = false
            onSessionComplete?(round)
            updateState(.idle)
            NSLog("[pingpong] session complete — %d rounds", round)
            return
        }

        // Switch to the other pane
        currentSpeaker = (speaker == "A") ? "B" : "A"
        currentTurnText = ""

        // Start capturing the next pane
        startCapturingCurrentPane()
    }

    /// Extract delta between baseline and final viewport snapshot.
    /// If baseline is a prefix of full, returns only the new portion.
    /// Otherwise returns full text (conservative fallback).
    private func extractDelta(full: String, baseline: String) -> String {
        guard !baseline.isEmpty else { return full }
        if full.hasPrefix(baseline) {
            let delta = String(full.dropFirst(baseline.count))
            return delta.trimmingCharacters(in: .whitespacesAndNewlines)
        }
        // Baseline may have scrolled off — return full text trimmed
        return full.trimmingCharacters(in: .whitespacesAndNewlines)
    }

    // MARK: - State

    private func updateState(_ newState: State) {
        state = newState
        onStateChanged?(state, round)
    }
}
