import AppKit

/// Ping-pong router — captures raw PTY output stream from HOST_MANAGED panes,
/// detects turn-complete via silence timeout, and relays cleaned text between panes.
///
/// KEY DIFFERENCE from v0.8: no viewport polling. Uses onPTYOutput callback
/// which receives raw bytes from the PTY master fd output. Injection writes
/// to PTY stdin (master fd write) — completely separate path, NO feedback loop.
class PingPongRouter {

    enum State: String {
        case idle = "Idle"
        case waitingForOutput = "Waiting..."
        case paneASpeaking = "A → B"
        case paneBSpeaking = "B → A"
        case paused = "Paused"
    }

    // MARK: - Public state

    private(set) var state: State = .idle
    private(set) var round: Int = 0
    private(set) var maxRounds: Int = 20
    private(set) var isActive: Bool = false

    private weak var paneA: GhosttyTerminalView?
    private weak var paneB: GhosttyTerminalView?

    var paneALabel: String = "A"
    var paneBLabel: String = "B"

    // MARK: - Callbacks

    var onStateChanged: ((State, Int) -> Void)?
    var onTurnComplete: ((String, String) -> Void)?  // (speakerLabel, cleanedOutput)
    var onSessionComplete: ((Int) -> Void)?

    // MARK: - Stream capture state

    private var currentSpeaker: String = "A"

    /// Raw PTY output accumulated during the current turn (for activity detection only)
    private var outputBuffer = Data()

    /// Viewport snapshot at turn start (for delta extraction at turn end)
    private var baselineText: String = ""

    /// Silence timeout: if no PTY output for this duration, treat as turn-complete
    private let silenceThreshold: TimeInterval = 3.0

    /// Cancellable silence timeout
    private var silenceWorkItem: DispatchWorkItem?

    /// Flag to ignore output briefly after injection (prevent echo noise)
    private var ignoreOutputUntil: Date = .distantPast

    /// Wait for a burst of output (agent responding) before starting silence timer.
    /// User typing produces 1 char at a time. Agent response produces bursts of many chars.
    /// We require at least this many meaningful chars before arming the silence timer.
    private let burstThreshold: Int = 50

    /// Debug counter for bytes this turn
    private var bytesThisTurn: Int = 0

    // MARK: - Init

    init(paneA: GhosttyTerminalView, paneB: GhosttyTerminalView, maxRounds: Int = 20) {
        self.paneA = paneA
        self.paneB = paneB
        self.maxRounds = maxRounds
    }

    deinit { stop() }

    // MARK: - Lifecycle

    func start() {
        isActive = true
        round = 0
        currentSpeaker = "A"
        outputBuffer = Data()
        updateState(.waitingForOutput)

        NSLog("[pingpong] started — listening to PTY output stream (silence=%.0fs)", silenceThreshold)

        // Subscribe to PTY output from the active speaker pane
        startListeningToCurrentPane()
    }

    func stop() {
        isActive = false
        outputBuffer = Data()

        // Detach stream listeners
        paneA?.onPTYOutput = nil
        paneB?.onPTYOutput = nil

        silenceWorkItem?.cancel()
        silenceWorkItem = nil

        updateState(.idle)
        NSLog("[pingpong] stopped")
    }

    func pause() {
        guard isActive else { return }
        paneA?.onPTYOutput = nil
        paneB?.onPTYOutput = nil
        silenceWorkItem?.cancel()
        updateState(.paused)
        NSLog("[pingpong] paused")
    }

    func resume() {
        guard isActive, state == .paused else { return }
        updateState(.waitingForOutput)
        startListeningToCurrentPane()
        NSLog("[pingpong] resumed")
    }

    // MARK: - PTY Stream Capture

    /// Attach onPTYOutput callback to the current speaker's pane.
    private func startListeningToCurrentPane() {
        let pane = (currentSpeaker == "A") ? paneA : paneB
        let speakerState: State = (currentSpeaker == "A") ? .paneASpeaking : .paneBSpeaking
        updateState(speakerState)
        outputBuffer = Data()
        bytesThisTurn = 0

        // Snapshot viewport baseline for delta extraction at turn-complete
        baselineText = ANSIStripper.strip(pane?.captureViewportText() ?? "")

        // Detach previous listeners
        paneA?.onPTYOutput = nil
        paneB?.onPTYOutput = nil

        // Attach to current speaker's PTY output stream
        pane?.onPTYOutput = { [weak self] data in
            self?.handlePTYOutput(data)
        }
    }

    /// Called when raw bytes arrive from the speaker's PTY output.
    /// This is the stdout/stderr of the child process — NOT our injected text.
    private func handlePTYOutput(_ data: Data) {
        guard isActive, state != .paused else { return }

        // Ignore brief echo after injection
        if Date() < ignoreOutputUntil { return }

        // Strip ANSI escape sequences first, then check for real content.
        // TUI apps send constant cursor movement, status bar updates, etc.
        // that are 100% ANSI sequences with no meaningful text — filter those out.
        let raw = String(data: data, encoding: .utf8) ?? ""
        let stripped = ANSIStripper.strip(raw)
        let meaningful = stripped.trimmingCharacters(in: .whitespacesAndNewlines)

        // Only count as activity if there's real printable content after ANSI strip
        guard !meaningful.isEmpty else { return }

        outputBuffer.append(data)
        bytesThisTurn += meaningful.count

        // Only arm silence timer after burst threshold — distinguishes agent response
        // (many chars in rapid succession) from user typing (1 char at a time).
        if bytesThisTurn >= burstThreshold {
            resetSilenceTimer()
        }

        NSLog("[pingpong] activity: +%d chars (total %d, armed=%@)",
              meaningful.count, bytesThisTurn,
              bytesThisTurn >= burstThreshold ? "YES" : "no")
    }

    // MARK: - Turn-Complete Detection

    private func resetSilenceTimer() {
        silenceWorkItem?.cancel()
        let item = DispatchWorkItem { [weak self] in
            guard let self = self, self.isActive, self.state != .paused else { return }
            NSLog("[pingpong] silence timeout (%.0fs) — turn complete", self.silenceThreshold)
            DispatchQueue.main.async {
                self.processTurnComplete()
            }
        }
        silenceWorkItem = item
        DispatchQueue.global().asyncAfter(deadline: .now() + silenceThreshold, execute: item)
    }

    /// Turn complete: read rendered viewport for clean text, relay to other pane.
    /// HYBRID APPROACH: PTY stream for turn DETECTION, viewport for text EXTRACTION.
    /// This gives clean rendered text without TUI artifacts (spinner, cursor repositioning).
    private func processTurnComplete() {
        guard isActive else { return }

        let speaker = currentSpeaker
        let label = (speaker == "A") ? paneALabel : paneBLabel

        // Detach stream listener from current pane
        let currentPane = (speaker == "A") ? paneA : paneB
        currentPane?.onPTYOutput = nil

        // Read the RENDERED viewport (clean, no TUI artifacts) instead of raw PTY bytes.
        // This is a single read at turn-complete — NOT polling. No feedback loop because
        // turn detection is PTY-stream-based, not viewport-change-based.
        let viewportText = currentPane?.captureViewportText() ?? ""
        let cleanText = ANSIStripper.strip(viewportText).trimmingCharacters(in: .whitespacesAndNewlines)

        // Extract delta from baseline
        let delta = extractDelta(full: cleanText, baseline: baselineText)

        if !delta.isEmpty {
            onTurnComplete?(label, delta)

            // RELAY: inject into OTHER pane's PTY stdin
            let targetPane = (speaker == "A") ? paneB : paneA
            // Brief ignore window to skip echo of our injection
            ignoreOutputUntil = Date().addingTimeInterval(1.0)

            // Send text first, then CR separately after delay.
            // TUI apps treat multi-line paste differently from typed input —
            // they show "[Pasted text +N lines]" and wait for Enter.
            // Sending CR separately after the paste is processed submits it.
            targetPane?.sendText(delta)
            DispatchQueue.main.asyncAfter(deadline: .now() + 0.3) {
                targetPane?.sendText("\r")
            }

            NSLog("[pingpong] turn complete — speaker=%@ output=%d chars → %@",
                  label, cleanText.count, speaker == "A" ? paneBLabel : paneALabel)
        } else {
            NSLog("[pingpong] turn complete — speaker=%@ (empty, skipping relay)", label)
        }

        // Advance round
        round += 1
        if round >= maxRounds {
            isActive = false
            onSessionComplete?(round)
            updateState(.idle)
            NSLog("[pingpong] session complete — %d rounds", round)
            return
        }

        // Switch speaker
        currentSpeaker = (speaker == "A") ? "B" : "A"
        outputBuffer = Data()

        // Start listening to the other pane
        startListeningToCurrentPane()
    }

    // MARK: - Text Extraction

    /// Extract new content by comparing viewport at turn-end vs turn-start.
    private func extractDelta(full: String, baseline: String) -> String {
        guard !baseline.isEmpty else { return full }
        // For TUI apps, the viewport is fully redrawn each time.
        // Find the longest common prefix and return what's new.
        if full.hasPrefix(baseline) {
            let delta = String(full.dropFirst(baseline.count))
            let trimmed = delta.trimmingCharacters(in: .whitespacesAndNewlines)
            return trimmed.isEmpty ? full.trimmingCharacters(in: .whitespacesAndNewlines) : trimmed
        }
        // TUI redrew entirely — return full viewport
        return full.trimmingCharacters(in: .whitespacesAndNewlines)
    }

    // MARK: - State

    private func updateState(_ newState: State) {
        state = newState
        onStateChanged?(state, round)
    }
}
