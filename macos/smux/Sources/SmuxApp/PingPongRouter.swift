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

    /// Raw PTY output accumulated during the current turn
    private var outputBuffer = Data()

    /// Silence timeout: if no PTY output for this duration, treat as turn-complete
    private let silenceThreshold: TimeInterval = 3.0

    /// Cancellable silence timeout
    private var silenceWorkItem: DispatchWorkItem?

    /// Flag to ignore output briefly after injection (prevent echo noise)
    private var ignoreOutputUntil: Date = .distantPast

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

        // Filter: only count printable content as "activity"
        let hasPrintable = data.contains(where: { $0 >= 0x20 && $0 < 0x7F || $0 >= 0x80 })
        guard hasPrintable else { return }

        outputBuffer.append(data)

        // Reset silence timer — output is still flowing
        resetSilenceTimer()
    }

    // MARK: - Turn-Complete Detection

    private func resetSilenceTimer() {
        silenceWorkItem?.cancel()
        let item = DispatchWorkItem { [weak self] in
            guard let self = self, self.isActive, self.state != .paused else { return }
            DispatchQueue.main.async {
                self.processTurnComplete()
            }
        }
        silenceWorkItem = item
        DispatchQueue.global().asyncAfter(deadline: .now() + silenceThreshold, execute: item)
    }

    /// Turn complete: extract text from buffer, relay to other pane.
    private func processTurnComplete() {
        guard isActive else { return }

        let speaker = currentSpeaker
        let label = (speaker == "A") ? paneALabel : paneBLabel

        // Detach stream listener from current pane
        let currentPane = (speaker == "A") ? paneA : paneB
        currentPane?.onPTYOutput = nil

        // Convert raw bytes to string, strip ANSI
        let rawText = String(data: outputBuffer, encoding: .utf8) ?? ""
        let cleanText = ANSIStripper.strip(rawText).trimmingCharacters(in: .whitespacesAndNewlines)

        if !cleanText.isEmpty {
            onTurnComplete?(label, cleanText)

            // RELAY: inject into OTHER pane's PTY stdin
            let targetPane = (speaker == "A") ? paneB : paneA
            // Brief ignore window to skip echo of our injection
            ignoreOutputUntil = Date().addingTimeInterval(0.5)
            targetPane?.sendText(cleanText + "\n")

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

    // MARK: - State

    private func updateState(_ newState: State) {
        state = newState
        onStateChanged?(state, round)
    }
}
