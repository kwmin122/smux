import AppKit

/// AppleScript support — enables external automation via `osascript` or Automator.
/// Registers Apple Event handlers for smux commands.
///
/// Usage from osascript:
///   tell application "SmuxApp"
///     do script "ls -la"
///     split vertical
///     open browser "http://localhost:3000"
///     take screenshot "/tmp/smux.png"
///   end tell
///
/// Or via command-line:
///   osascript -e 'tell application "SmuxApp" to do script "npm start"'
class AppleScriptSupport {
    private weak var controller: WorkspaceWindowController?

    init(controller: WorkspaceWindowController) {
        self.controller = controller
    }

    /// Register all Apple Event handlers.
    func registerHandlers() {
        let em = NSAppleEventManager.shared()

        // 'dosc' — do script (run command in terminal)
        em.setEventHandler(self,
                           andSelector: #selector(handleDoScript(_:withReply:)),
                           forEventClass: AEEventClass(kAECoreSuite),
                           andEventID: AEEventID(kAEDoScript))

        // Custom 'smux' event class for extended commands
        let smuxClass = AEEventClass(fourCharCode("smux"))

        // 'splt' — split terminal
        em.setEventHandler(self,
                           andSelector: #selector(handleSplit(_:withReply:)),
                           forEventClass: smuxClass,
                           andEventID: AEEventID(fourCharCode("splt")))

        // 'brws' — open browser
        em.setEventHandler(self,
                           andSelector: #selector(handleOpenBrowser(_:withReply:)),
                           forEventClass: smuxClass,
                           andEventID: AEEventID(fourCharCode("brws")))

        // 'ssht' — take screenshot
        em.setEventHandler(self,
                           andSelector: #selector(handleScreenshot(_:withReply:)),
                           forEventClass: smuxClass,
                           andEventID: AEEventID(fourCharCode("ssht")))

        // 'ntab' — new tab
        em.setEventHandler(self,
                           andSelector: #selector(handleNewTab(_:withReply:)),
                           forEventClass: smuxClass,
                           andEventID: AEEventID(fourCharCode("ntab")))

        // 'sess' — list sessions
        em.setEventHandler(self,
                           andSelector: #selector(handleListSessions(_:withReply:)),
                           forEventClass: smuxClass,
                           andEventID: AEEventID(fourCharCode("sess")))

        // 'ntfy' — send notification
        em.setEventHandler(self,
                           andSelector: #selector(handleNotify(_:withReply:)),
                           forEventClass: smuxClass,
                           andEventID: AEEventID(fourCharCode("ntfy")))

        NSLog("[smux] AppleScript handlers registered")
    }

    /// Unregister all handlers.
    func unregisterHandlers() {
        let em = NSAppleEventManager.shared()
        em.removeEventHandler(forEventClass: AEEventClass(kAECoreSuite),
                              andEventID: AEEventID(kAEDoScript))
        let smuxClass = AEEventClass(fourCharCode("smux"))
        for code in ["splt", "brws", "ssht", "ntab", "sess", "ntfy"] {
            em.removeEventHandler(forEventClass: smuxClass,
                                  andEventID: AEEventID(fourCharCode(code)))
        }
    }

    // MARK: - Handlers

    @objc private func handleDoScript(_ event: NSAppleEventDescriptor,
                                       withReply reply: NSAppleEventDescriptor) {
        guard let command = event.paramDescriptor(forKeyword: keyDirectObject)?.stringValue else { return }
        NSLog("[smux-applescript] do script: %@", command)
        DispatchQueue.main.async { [weak self] in
            // Type command into the active terminal
            self?.typeIntoTerminal(command + "\n")
        }
    }

    @objc private func handleSplit(_ event: NSAppleEventDescriptor,
                                    withReply reply: NSAppleEventDescriptor) {
        let direction = event.paramDescriptor(forKeyword: keyDirectObject)?.stringValue ?? "vertical"
        NSLog("[smux-applescript] split: %@", direction)
        DispatchQueue.main.async { [weak self] in
            if direction == "horizontal" {
                self?.controller?.splitHorizontal()
            } else {
                self?.controller?.splitVertical()
            }
        }
    }

    @objc private func handleOpenBrowser(_ event: NSAppleEventDescriptor,
                                          withReply reply: NSAppleEventDescriptor) {
        let url = event.paramDescriptor(forKeyword: keyDirectObject)?.stringValue ?? "http://localhost:3000"
        NSLog("[smux-applescript] open browser: %@", url)
        DispatchQueue.main.async { [weak self] in
            self?.controller?.openInBrowser(url: url)
        }
    }

    @objc private func handleScreenshot(_ event: NSAppleEventDescriptor,
                                         withReply reply: NSAppleEventDescriptor) {
        let path = event.paramDescriptor(forKeyword: keyDirectObject)?.stringValue ?? "/tmp/smux-screenshot.png"
        NSLog("[smux-applescript] screenshot: %@", path)
        DispatchQueue.main.async { [weak self] in
            self?.controller?.automation()?.screenshotToFile(path: path) { result in
                switch result {
                case .success(let p):
                    NSLog("[smux-applescript] screenshot saved: %@", p)
                case .failure(let err):
                    NSLog("[smux-applescript] screenshot error: %@", err.localizedDescription)
                }
            }
        }
    }

    @objc private func handleNewTab(_ event: NSAppleEventDescriptor,
                                     withReply reply: NSAppleEventDescriptor) {
        NSLog("[smux-applescript] new tab")
        DispatchQueue.main.async { [weak self] in
            self?.controller?.newTab()
        }
    }

    @objc private func handleListSessions(_ event: NSAppleEventDescriptor,
                                            withReply reply: NSAppleEventDescriptor) {
        NSLog("[smux-applescript] list sessions")
        let sessions = controller?.missionState.sessions ?? []
        let list = sessions.map { "\($0.id): \($0.task) [\($0.status.rawValue)]" }.joined(separator: "\n")
        reply.setDescriptor(NSAppleEventDescriptor(string: list.isEmpty ? "(no sessions)" : list),
                            forKeyword: keyDirectObject)
    }

    @objc private func handleNotify(_ event: NSAppleEventDescriptor,
                                     withReply reply: NSAppleEventDescriptor) {
        let message = event.paramDescriptor(forKeyword: keyDirectObject)?.stringValue ?? ""
        NSLog("[smux-applescript] notify: %@", message)
        DispatchQueue.main.async { [weak self] in
            self?.controller?.sendNotification(title: "AppleScript", body: message, source: "External")
        }
    }

    // MARK: - Helpers

    private func typeIntoTerminal(_ text: String) {
        // Send text directly to the ghostty surface via ghostty_surface_text.
        // Cannot use insertText() here because AppleScript handlers have no
        // active NSEvent, and insertText guards on NSApp.currentEvent != nil.
        guard let window = controller?.window,
              let termView = window.firstResponder as? GhosttyTerminalView else {
            // Fallback: find any terminal view in the window
            if let termView = controller?.window?.contentView?.findSubview(ofType: GhosttyTerminalView.self) {
                termView.sendText(text)
            }
            return
        }
        termView.sendText(text)
    }
}

// MARK: - FourCharCode helper

private func fourCharCode(_ string: String) -> UInt32 {
    var result: UInt32 = 0
    for (i, char) in string.utf8.enumerated() where i < 4 {
        result = (result << 8) | UInt32(char)
    }
    return result
}
