import AppKit
import libghostty

/// NSView hosting a ghostty terminal surface.
/// Supports EXEC mode (ghostty owns PTY) and HOST_MANAGED mode (smux owns PTY).
class GhosttyTerminalView: NSView {
    private var surface: ghostty_surface_t?
    private var ghosttyApp: ghostty_app_t?
    private var surfaceCreated = false

    /// HOST_MANAGED mode: smux owns the PTY.
    private(set) var isHostManaged: Bool = false
    private(set) var ptyManager: PTYManager?

    /// Called with raw PTY output bytes (only in HOST_MANAGED mode).
    /// Consumers (PingPongRouter) attach here to capture the stream.
    var onPTYOutput: ((Data) -> Void)?

    /// Accumulates text from insertText during a keyDown cycle.
    private var keyTextAccumulator: [String]? = nil

    /// Marked text for IME preedit (Korean composition etc.)
    private var markedText = NSMutableAttributedString()

    init(frame: NSRect, app: ghostty_app_t, managed: Bool = false) {
        self.ghosttyApp = app
        self.isHostManaged = managed
        let safeFrame = (frame.width > 1 && frame.height > 1)
            ? frame
            : NSRect(x: 0, y: 0, width: 800, height: 600)
        super.init(frame: safeFrame)
        wantsLayer = true
    }

    required init?(coder: NSCoder) { fatalError() }

    /// Explicitly free the ghostty surface. Must call before ghostty_app_free.
    func destroySurface() {
        // Stop PTY before freeing surface (HOST_MANAGED mode)
        ptyManager?.stop()
        ptyManager = nil

        guard let s = surface else { return }
        surface = nil
        // Free surface asynchronously on MainActor — Ghostty's pattern.
        // Metal is not thread-safe; synchronous free while CALayer is
        // still in the view hierarchy causes zombie Metal layers.
        Task.detached { @MainActor in
            ghostty_surface_free(s)
        }
    }

    deinit {
        destroySurface()
    }

    // MARK: - Metal layer

    override func makeBackingLayer() -> CALayer {
        let layer = CAMetalLayer()
        layer.isOpaque = true
        return layer
    }
    override var wantsUpdateLayer: Bool { true }
    override func updateLayer() {
        guard let s = surface else { return }
        ghostty_surface_draw(s)
    }

    // MARK: - Surface creation (when view enters window hierarchy)

    override func viewDidMoveToWindow() {
        super.viewDidMoveToWindow()
        guard window != nil, !surfaceCreated, let app = ghosttyApp else { return }
        surfaceCreated = true

        var cfg = ghostty_surface_config_new()
        cfg.platform_tag = GHOSTTY_PLATFORM_MACOS
        cfg.context = GHOSTTY_SURFACE_CONTEXT_WINDOW
        cfg.scale_factor = Double(window?.backingScaleFactor ?? 2.0)
        cfg.font_size = 14.0

        var plat = ghostty_platform_macos_s()
        plat.nsview = Unmanaged.passUnretained(self).toOpaque()
        cfg.platform.macos = plat

        // HOST_MANAGED mode: smux owns PTY, ghostty renders only
        if isHostManaged {
            cfg.backend = GHOSTTY_SURFACE_IO_BACKEND_HOST_MANAGED

            // Store self pointer for @convention(c) callbacks
            let selfPtr = Unmanaged.passUnretained(self).toOpaque()
            cfg.receive_userdata = selfPtr

            // receive_buffer: ghostty sends keyboard input bytes → we write to PTY master fd
            cfg.receive_buffer = { (userdata, bytes, len) in
                guard let userdata = userdata, let bytes = bytes else { return }
                let view = Unmanaged<GhosttyTerminalView>.fromOpaque(userdata).takeUnretainedValue()
                view.ptyManager?.writeToPTY(bytes, length: Int(len))
            }

            // receive_resize: ghostty notifies terminal resize → we resize PTY
            cfg.receive_resize = { (userdata, cols, rows, widthPx, heightPx) in
                guard let userdata = userdata else { return }
                let view = Unmanaged<GhosttyTerminalView>.fromOpaque(userdata).takeUnretainedValue()
                view.ptyManager?.resize(rows: rows, cols: cols)
            }
        }

        NSLog("[smux] creating surface (mode=%@)...", isHostManaged ? "HOST_MANAGED" : "EXEC")
        if let s = ghostty_surface_new(app, &cfg) {
            self.surface = s
            NSLog("[smux] surface created: %@", String(describing: s))
            let scale = window?.backingScaleFactor ?? 2.0
            ghostty_surface_set_content_scale(s, Double(scale), Double(scale))
            if bounds.width > 0 && bounds.height > 0 {
                ghostty_surface_set_size(s, UInt32(bounds.width * scale), UInt32(bounds.height * scale))
            }

            // HOST_MANAGED: start PTY and pipe output to ghostty
            if isHostManaged {
                startHostManagedPTY()
            }
        }
    }

    /// Start HOST_MANAGED PTY: fork child, read master fd, pipe to ghostty.
    private func startHostManagedPTY() {
        guard surface != nil else { return }

        let mgr = PTYManager()
        self.ptyManager = mgr

        // PTY output → ghostty render + capture callback
        mgr.onOutput = { [weak self] (bytes: UnsafePointer<UInt8>, len: Int) in
            guard let self = self, let surface = self.surface else { return }
            // Feed raw bytes to ghostty for rendering (must be main thread)
            ghostty_surface_write_buffer(surface, bytes, UInt(len))

            // Also deliver to capture consumers (PingPongRouter)
            let data = Data(bytes: bytes, count: len)
            self.onPTYOutput?(data)
        }

        mgr.onExit = { [weak self] status in
            NSLog("[pty] child exited with status %d", status)
            self?.ptyManager = nil
        }

        // Get terminal size in cells (approximate from pixel size)
        let scale = window?.backingScaleFactor ?? 2.0
        let charWidth: CGFloat = 8.0 * scale   // approximate
        let charHeight: CGFloat = 16.0 * scale  // approximate
        let cols = max(80, UInt16(bounds.width * scale / charWidth))
        let rows = max(24, UInt16(bounds.height * scale / charHeight))

        if !mgr.start(rows: rows, cols: cols) {
            NSLog("[pty] failed to start HOST_MANAGED PTY")
            ptyManager = nil
        }
    }

    // MARK: - Layout

    override func setFrameSize(_ newSize: NSSize) {
        super.setFrameSize(newSize)
        guard let s = surface, newSize.width > 0, newSize.height > 0 else { return }
        let scale = window?.backingScaleFactor ?? 2.0
        ghostty_surface_set_content_scale(s, Double(scale), Double(scale))
        ghostty_surface_set_size(s, UInt32(newSize.width * scale), UInt32(newSize.height * scale))
    }

    // MARK: - Focus

    override var acceptsFirstResponder: Bool { true }
    override func becomeFirstResponder() -> Bool { true }
    override func acceptsFirstMouse(for event: NSEvent?) -> Bool { true }

    // MARK: - Keyboard (Ghostty-grade: keyDown → interpretKeyEvents → accumulate → ghostty_surface_key)

    override func keyDown(with event: NSEvent) {
        guard surface != nil else {
            interpretKeyEvents([event])
            return
        }

        let action: ghostty_input_action_e = event.isARepeat ? GHOSTTY_ACTION_REPEAT : GHOSTTY_ACTION_PRESS
        let hadMarkedText = markedText.length > 0

        // Begin accumulating text from interpretKeyEvents → insertText
        keyTextAccumulator = []
        defer { keyTextAccumulator = nil }

        // Let AppKit process the event through IME
        interpretKeyEvents([event])

        // Sync preedit state
        syncPreedit(clearIfNeeded: hadMarkedText)

        // Send accumulated text or raw key to ghostty
        if let texts = keyTextAccumulator, !texts.isEmpty {
            for text in texts {
                sendKey(action, event: event, text: text, composing: false)
            }
            // If we had marked text (IME was composing) and the key that ended it
            // was Enter/Return, send the Enter as a separate key event too.
            // Without this, Enter during Korean composition only commits the text
            // but doesn't execute the command — user has to press Enter twice.
            if hadMarkedText && (event.keyCode == 36 || event.keyCode == 76) {
                sendKey(action, event: event, text: nil, composing: false)
            }
        } else {
            let composing = markedText.length > 0 || hadMarkedText
            let text = event.ghosttyCharacters
            sendKey(action, event: event, text: text, composing: composing)
        }
    }

    override func keyUp(with event: NSEvent) {
        sendKey(GHOSTTY_ACTION_RELEASE, event: event, text: nil, composing: false)
    }

    private var previousModifierFlags: NSEvent.ModifierFlags = []

    override func flagsChanged(with event: NSEvent) {
        guard let surface = surface else { return }
        let action: ghostty_input_action_e =
            event.modifierFlags.rawValue > previousModifierFlags.rawValue
                ? GHOSTTY_ACTION_PRESS : GHOSTTY_ACTION_RELEASE
        previousModifierFlags = event.modifierFlags
        var key = ghostty_input_key_s()
        key.action = action
        key.mods = Self.ghosttyMods(event.modifierFlags)
        key.keycode = UInt32(event.keyCode)
        key.text = nil
        key.composing = false
        ghostty_surface_key(surface, key)
    }

    /// Send a key event to the ghostty surface.
    private func sendKey(
        _ action: ghostty_input_action_e,
        event: NSEvent,
        text: String?,
        composing: Bool
    ) {
        guard let surface = surface else { return }

        var key = ghostty_input_key_s()
        key.action = action
        key.keycode = UInt32(event.keyCode)
        key.mods = Self.ghosttyMods(event.modifierFlags)
        key.consumed_mods = Self.ghosttyMods(
            event.modifierFlags.subtracting([.control, .command])
        )
        key.composing = composing
        key.unshifted_codepoint = 0

        // Compute unshifted codepoint
        if event.type == .keyDown || event.type == .keyUp {
            if let chars = event.characters(byApplyingModifiers: []),
               let cp = chars.unicodeScalars.first {
                key.unshifted_codepoint = cp.value
            }
        }

        if let text = text, !text.isEmpty,
           let first = text.utf8.first, first >= 0x20 {
            text.withCString { ptr in
                key.text = ptr
                ghostty_surface_key(surface, key)
            }
        } else {
            key.text = nil
            ghostty_surface_key(surface, key)
        }
    }

    /// Sync preedit state to ghostty.
    private func syncPreedit(clearIfNeeded: Bool) {
        guard let surface = surface else { return }
        if markedText.length > 0 {
            let str = markedText.string
            str.withCString { ptr in
                ghostty_surface_preedit(surface, ptr, UInt(str.utf8.count))
            }
        } else if clearIfNeeded {
            ghostty_surface_preedit(surface, nil, 0)
        }
    }

    // MARK: - Direct text input (for AppleScript / programmatic use)

    /// Send text directly to the terminal.
    /// In EXEC mode: uses ghostty_surface_text (ghostty writes to PTY).
    /// In HOST_MANAGED mode: writes directly to PTY master fd.
    func sendText(_ text: String) {
        if isHostManaged, let mgr = ptyManager {
            mgr.writeString(text)
        } else {
            guard let surface = surface else { return }
            text.withCString { ptr in
                ghostty_surface_text(surface, ptr, UInt(text.utf8.count))
            }
        }
    }

    // MARK: - Text Capture (EXEC mode polling)

    /// Read the full visible viewport text from the ghostty surface.
    /// MUST be called on the main thread (Metal thread safety).
    /// Returns nil if surface is not available or read fails.
    func captureViewportText() -> String? {
        guard let s = surface else { return nil }

        var sel = ghostty_selection_s()
        sel.rectangle = false
        sel.top_left = ghostty_point_s(
            tag: GHOSTTY_POINT_VIEWPORT,
            coord: GHOSTTY_POINT_COORD_TOP_LEFT,
            x: 0, y: 0
        )
        sel.bottom_right = ghostty_point_s(
            tag: GHOSTTY_POINT_VIEWPORT,
            coord: GHOSTTY_POINT_COORD_BOTTOM_RIGHT,
            x: 9999, y: 9999
        )

        var txt = ghostty_text_s()
        guard ghostty_surface_read_text(s, sel, &txt) else { return nil }
        defer { ghostty_surface_free_text(s, &txt) }

        guard let ptr = txt.text, txt.text_len > 0 else { return nil }
        return ptr.withMemoryRebound(to: UInt8.self, capacity: Int(txt.text_len)) { uint8Ptr in
            String(
                bytes: UnsafeBufferPointer(start: uint8Ptr, count: Int(txt.text_len)),
                encoding: .utf8
            )
        }
    }

    /// Timer-driven polling of terminal viewport text.
    private var captureTimer: Timer?
    private var lastCapturedText: String = ""

    /// Start polling the terminal viewport at ~4 Hz. Calls onChange with ANSI-stripped text
    /// whenever the visible content changes (delta > 4 chars to filter cursor blink noise).
    /// MUST be called on the main thread.
    func startCapturing(onChange: @escaping (String) -> Void) {
        stopCapturing()
        lastCapturedText = ""
        captureTimer = Timer.scheduledTimer(withTimeInterval: 0.25, repeats: true) { [weak self] _ in
            guard let self = self else { return }
            guard let raw = self.captureViewportText() else { return }
            let clean = ANSIStripper.strip(raw)
            // Filter noise: cursor blink / spinner causes tiny changes that are not meaningful
            if clean != self.lastCapturedText,
               abs(clean.count - self.lastCapturedText.count) > 4 || self.lastCapturedText.isEmpty {
                self.lastCapturedText = clean
                onChange(clean)
            }
        }
    }

    /// Stop the polling timer and clear captured state.
    func stopCapturing() {
        captureTimer?.invalidate()
        captureTimer = nil
        lastCapturedText = ""
    }

    // MARK: - Mouse

    override func mouseDown(with event: NSEvent) {
        window?.makeFirstResponder(self)
        guard let surface = surface else { return }
        let pt = convert(event.locationInWindow, from: nil)
        let scale = window?.backingScaleFactor ?? 2.0
        ghostty_surface_mouse_button(surface, GHOSTTY_MOUSE_PRESS, GHOSTTY_MOUSE_LEFT, Self.ghosttyMods(event.modifierFlags))
        ghostty_surface_mouse_pos(surface, Double(pt.x * scale), Double((bounds.height - pt.y) * scale), Self.ghosttyMods(event.modifierFlags))
    }

    override func mouseUp(with event: NSEvent) {
        guard let surface = surface else { return }
        ghostty_surface_mouse_button(surface, GHOSTTY_MOUSE_RELEASE, GHOSTTY_MOUSE_LEFT, Self.ghosttyMods(event.modifierFlags))
    }

    override func mouseMoved(with event: NSEvent) {
        guard let surface = surface else { return }
        let pt = convert(event.locationInWindow, from: nil)
        let scale = window?.backingScaleFactor ?? 2.0
        ghostty_surface_mouse_pos(surface, Double(pt.x * scale), Double((bounds.height - pt.y) * scale), Self.ghosttyMods(event.modifierFlags))
    }

    override func mouseDragged(with event: NSEvent) {
        mouseMoved(with: event)
    }

    override func scrollWheel(with event: NSEvent) {
        guard let surface = surface else { return }
        ghostty_surface_mouse_scroll(surface, Double(event.scrollingDeltaX), Double(event.scrollingDeltaY),
                                      Int32(Self.ghosttyMods(event.modifierFlags).rawValue))
    }

    override func updateTrackingAreas() {
        super.updateTrackingAreas()
        trackingAreas.forEach { removeTrackingArea($0) }
        addTrackingArea(NSTrackingArea(rect: bounds, options: [.mouseMoved, .activeInKeyWindow, .inVisibleRect], owner: self))
    }

    // MARK: - Modifier conversion (matches Ghostty exactly)

    static func ghosttyMods(_ flags: NSEvent.ModifierFlags) -> ghostty_input_mods_e {
        var mods: UInt32 = GHOSTTY_MODS_NONE.rawValue
        if flags.contains(.shift) { mods |= GHOSTTY_MODS_SHIFT.rawValue }
        if flags.contains(.control) { mods |= GHOSTTY_MODS_CTRL.rawValue }
        if flags.contains(.option) { mods |= GHOSTTY_MODS_ALT.rawValue }
        if flags.contains(.command) { mods |= GHOSTTY_MODS_SUPER.rawValue }
        if flags.contains(.capsLock) { mods |= GHOSTTY_MODS_CAPS.rawValue }
        return ghostty_input_mods_e(mods)
    }
}

// MARK: - NSEvent extension (matches Ghostty's NSEvent+Extension.swift)

extension NSEvent {
    var ghosttyCharacters: String? {
        guard let characters = characters else { return nil }
        if characters.count == 1, let scalar = characters.unicodeScalars.first {
            if scalar.value < 0x20 {
                return self.characters(byApplyingModifiers: modifierFlags.subtracting(.control))
            }
            if scalar.value >= 0xF700 && scalar.value <= 0xF8FF {
                return nil
            }
        }
        return characters
    }
}

// MARK: - NSTextInputClient (Korean/CJK IME — follows Ghostty's pattern exactly)

extension GhosttyTerminalView: NSTextInputClient {
    override func doCommand(by selector: Selector) {
        // Prevent NSBeep for unhandled selectors
    }

    func insertText(_ string: Any, replacementRange: NSRange) {
        guard NSApp.currentEvent != nil else { return }

        var chars = ""
        switch string {
        case let v as NSAttributedString: chars = v.string
        case let v as String: chars = v
        default: return
        }

        // Clear preedit on text insertion
        unmarkText()

        // If we're inside keyDown, accumulate text for the keyDown handler
        if var acc = keyTextAccumulator {
            acc.append(chars)
            keyTextAccumulator = acc
            return
        }

        // Direct text insertion (outside keyDown — e.g., paste)
        guard let surface = surface else { return }
        chars.withCString { ptr in
            ghostty_surface_text(surface, ptr, UInt(chars.utf8.count))
        }
    }

    func setMarkedText(_ string: Any, selectedRange: NSRange, replacementRange: NSRange) {
        switch string {
        case let v as NSAttributedString: markedText = NSMutableAttributedString(attributedString: v)
        case let v as String: markedText = NSMutableAttributedString(string: v)
        default: return
        }
    }

    func unmarkText() {
        markedText = NSMutableAttributedString()
    }

    func selectedRange() -> NSRange { NSRange(location: NSNotFound, length: 0) }
    func markedRange() -> NSRange {
        if markedText.length > 0 { return NSRange(location: 0, length: markedText.length) }
        return NSRange(location: NSNotFound, length: 0)
    }
    func hasMarkedText() -> Bool { markedText.length > 0 }
    func attributedSubstring(forProposedRange range: NSRange, actualRange: NSRangePointer?) -> NSAttributedString? { nil }
    func validAttributesForMarkedText() -> [NSAttributedString.Key] { [] }

    func firstRect(forCharacterRange range: NSRange, actualRange: NSRangePointer?) -> NSRect {
        guard let surface = surface, let window = window else { return .zero }
        var x: Double = 0, y: Double = 0, w: Double = 0, h: Double = 0
        ghostty_surface_ime_point(surface, &x, &y, &w, &h)
        let viewPoint = NSPoint(x: x, y: bounds.height - y - h)
        let windowPoint = convert(viewPoint, to: nil)
        let screenPoint = window.convertPoint(toScreen: windowPoint)
        return NSRect(x: screenPoint.x, y: screenPoint.y, width: w, height: h)
    }

    func characterIndex(for point: NSPoint) -> Int { 0 }
}
