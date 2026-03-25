import AppKit
import libghostty

/// A minimal NSView that hosts a ghostty terminal surface.
/// Provides Metal rendering + keyboard/mouse forwarding + IME support.
class GhosttyTerminalView: NSView {
    private var surface: ghostty_surface_t?
    private var ghosttyApp: ghostty_app_t? // not weak — raw pointer

    // MARK: - Init

    init(frame: NSRect, app: ghostty_app_t) {
        self.ghosttyApp = app
        super.init(frame: frame)

        wantsLayer = true
        layer?.isOpaque = true

        // Create surface config
        var surfaceConfig = ghostty_surface_config_new()
        surfaceConfig.platform_tag = GHOSTTY_PLATFORM_MACOS

        // Pass this NSView to libghostty
        var macPlatform = ghostty_platform_macos_s()
        let viewPtr = Unmanaged.passUnretained(self).toOpaque()
        macPlatform.nsview = viewPtr
        surfaceConfig.platform.macos = macPlatform

        surfaceConfig.context = GHOSTTY_SURFACE_CONTEXT_WINDOW
        surfaceConfig.scale_factor = Double(NSScreen.main?.backingScaleFactor ?? 2.0)
        surfaceConfig.font_size = 14.0

        // Create the terminal surface
        let newSurface = ghostty_surface_new(app, &surfaceConfig)
        if let newSurface = newSurface {
            self.surface = newSurface
            print("✅ ghostty_surface_new: SUCCESS — terminal surface created")
        } else {
            print("❌ ghostty_surface_new: FAILED")
        }
    }

    required init?(coder: NSCoder) {
        fatalError("init(coder:) not supported")
    }

    deinit {
        if let surface = surface {
            ghostty_surface_free(surface)
        }
    }

    // MARK: - Layer

    override func makeBackingLayer() -> CALayer {
        // libghostty expects a CAMetalLayer for GPU rendering
        let metalLayer = CAMetalLayer()
        metalLayer.isOpaque = true
        metalLayer.contentsScale = window?.backingScaleFactor ?? 2.0
        return metalLayer
    }

    override var wantsUpdateLayer: Bool { true }

    override func updateLayer() {
        guard let surface = surface else { return }
        ghostty_surface_draw(surface)
    }

    // MARK: - Layout

    override func setFrameSize(_ newSize: NSSize) {
        super.setFrameSize(newSize)
        guard let surface = surface else { return }
        let scale = window?.backingScaleFactor ?? 2.0
        ghostty_surface_set_content_scale(surface, scale, scale)
        ghostty_surface_set_size(
            surface,
            UInt32(newSize.width * scale),
            UInt32(newSize.height * scale)
        )
    }

    // MARK: - Focus

    override var acceptsFirstResponder: Bool { true }

    override func becomeFirstResponder() -> Bool {
        true
    }

    // MARK: - Keyboard (basic — IME handled via NSTextInputClient extension)

    override func keyDown(with event: NSEvent) {
        // Forward to input context for IME handling
        inputContext?.handleEvent(event)
    }

    override func keyUp(with event: NSEvent) {
        // Key up events don't need IME processing
    }

    override func flagsChanged(with event: NSEvent) {
        // Modifier key changes
    }
}

// MARK: - NSTextInputClient (Korean/CJK IME support)

extension GhosttyTerminalView: NSTextInputClient {
    override func doCommand(by selector: Selector) {
    }

    func insertText(_ string: Any, replacementRange: NSRange) {
        guard let surface = surface else { return }
        let text: String
        if let str = string as? String {
            text = str
        } else if let attrStr = string as? NSAttributedString {
            text = attrStr.string
        } else {
            return
        }

        // Send committed text to the terminal
        text.withCString { cstr in
            ghostty_surface_text(surface, cstr, UInt(text.utf8.count))
        }
    }

    func setMarkedText(_ string: Any, selectedRange: NSRange, replacementRange: NSRange) {
        guard let surface = surface else { return }
        let text: String
        if let str = string as? String {
            text = str
        } else if let attrStr = string as? NSAttributedString {
            text = attrStr.string
        } else {
            return
        }

        // Send preedit (composing) text — this is what makes Korean IME work!
        text.withCString { cstr in
            ghostty_surface_preedit(surface, cstr, UInt(text.utf8.count))
        }
    }

    func unmarkText() {
        guard let surface = surface else { return }
        // Clear preedit
        ghostty_surface_preedit(surface, nil, 0)
    }

    func selectedRange() -> NSRange {
        NSRange(location: NSNotFound, length: 0)
    }

    func markedRange() -> NSRange {
        NSRange(location: NSNotFound, length: 0)
    }

    func hasMarkedText() -> Bool {
        false
    }

    func attributedSubstring(forProposedRange range: NSRange, actualRange: NSRangePointer?) -> NSAttributedString? {
        nil
    }

    func validAttributedString(for text: NSAttributedString) -> NSAttributedString {
        text
    }

    func firstRect(forCharacterRange range: NSRange, actualRange: NSRangePointer?) -> NSRect {
        guard let surface = surface, let window = window else {
            return .zero
        }

        // Get IME candidate window position from libghostty
        var x: Double = 0, y: Double = 0, w: Double = 0, h: Double = 0
        ghostty_surface_ime_point(surface, &x, &y, &w, &h)

        let viewPoint = NSPoint(x: x, y: bounds.height - y - h)
        let windowPoint = convert(viewPoint, to: nil)
        let screenPoint = window.convertPoint(toScreen: windowPoint)
        return NSRect(x: screenPoint.x, y: screenPoint.y, width: w, height: h)
    }

    func characterIndex(for point: NSPoint) -> Int {
        0
    }

    func validAttributesForMarkedText() -> [NSAttributedString.Key] {
        []
    }
}
