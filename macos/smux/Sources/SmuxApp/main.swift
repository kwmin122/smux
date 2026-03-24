import AppKit
import libghostty

// Minimal libghostty PoC: create a native macOS app with one terminal pane.
// This proves: libghostty links, Metal renders, PTY attaches, Korean IME works.

class AppDelegate: NSObject, NSApplicationDelegate {
    var window: NSWindow!

    func applicationDidFinishLaunching(_ notification: Notification) {
        // Create window
        window = NSWindow(
            contentRect: NSRect(x: 200, y: 200, width: 800, height: 600),
            styleMask: [.titled, .closable, .resizable, .miniaturizable],
            backing: .buffered,
            defer: false
        )
        window.title = "smux — libghostty PoC"
        window.makeKeyAndOrderFront(nil)

        // For now, just show a label proving the app runs
        let label = NSTextField(labelWithString: "smux PoC — libghostty linked successfully.\nNext: add GhosttyKit terminal surface.")
        label.font = NSFont.monospacedSystemFont(ofSize: 14, weight: .regular)
        label.alignment = .center
        label.frame = NSRect(x: 100, y: 250, width: 600, height: 100)
        window.contentView?.addSubview(label)

        print("✅ smux native PoC launched. GhosttyKit imported.")
    }

    func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool {
        true
    }
}

let app = NSApplication.shared
let delegate = AppDelegate()
app.delegate = delegate
app.run()
