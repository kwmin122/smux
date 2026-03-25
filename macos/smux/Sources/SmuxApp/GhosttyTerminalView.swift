import AppKit
import libghostty

/// NSView hosting a ghostty terminal surface — tmux-grade keyboard/mouse/IME.
/// Follows the exact same keyDown → interpretKeyEvents → insertText → ghostty_surface_key
/// pattern that Ghostty's own SurfaceView_AppKit.swift uses.
class GhosttyTerminalView: NSView {
    private var surface: ghostty_surface_t?
    private var ghosttyApp: ghostty_app_t?
    private var surfaceCreated = false

    /// Accumulates text from insertText during a keyDown cycle.
    /// Non-nil means we are inside a keyDown → interpretKeyEvents call.
    private var keyTextAccumulator: [String]? = nil

    /// Marked text for IME preedit (Korean composition etc.)
    private var markedText = NSMutableAttributedString()

    init(frame: NSRect, app: ghostty_app_t) {
        self.ghosttyApp = app
        super.init(frame: frame)
        wantsLayer = true
    }

    required init?(coder: NSCoder) { fatalError() }

    deinit {
        if let s = surface { ghostty_surface_free(s) }
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

    // MARK: - Surface (created after window attachment)

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

        NSLog("[smux] creating surface...")
        if let s = ghostty_surface_new(app, &cfg) {
            self.surface = s
            NSLog("[smux] ✅ surface created: %@", String(describing: s))
            let scale = window?.backingScaleFactor ?? 2.0
            ghostty_surface_set_content_scale(s, Double(scale), Double(scale))
            ghostty_surface_set_size(s, UInt32(bounds.width * scale), UInt32(bounds.height * scale))
        }
    }

    // MARK: - Layout

    override func setFrameSize(_ newSize: NSSize) {
        super.setFrameSize(newSize)
        guard let s = surface else { return }
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
        NSLog("[smux] keyDown: keyCode=%d chars='%@' surface=%@",
              event.keyCode,
              event.characters ?? "<nil>",
              surface != nil ? "yes" : "NO")

        guard let surface = surface else {
            NSLog("[smux] no surface — falling back to interpretKeyEvents")
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
        } else {
            let composing = markedText.length > 0 || hadMarkedText
            let text = event.ghosttyCharacters
            sendKey(action, event: event, text: text, composing: composing)
        }
    }

    override func keyUp(with event: NSEvent) {
        sendKey(GHOSTTY_ACTION_RELEASE, event: event, text: nil, composing: false)
    }

    override func flagsChanged(with event: NSEvent) {
        guard let surface = surface else { return }
        var key = ghostty_input_key_s()
        key.action = GHOSTTY_ACTION_PRESS
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
        ghostty_surface_mouse_scroll(surface, Double(event.scrollingDeltaX), Double(event.scrollingDeltaY), 0)
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
        NSLog("[smux] insertText: '%@'", String(describing: string))
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
