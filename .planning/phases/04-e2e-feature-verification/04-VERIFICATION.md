---
phase: 04-e2e-feature-verification
verified: 2026-03-26T15:30:00Z
status: human_needed
score: 4/4 must-haves verified (code-level)
must_haves:
  truths:
    - "User presses Cmd+Shift+B and a browser panel opens alongside the terminal pane and renders a localhost URL correctly"
    - "Browser automation DOM snapshot call returns actual page content (non-empty, structurally valid HTML/text)"
    - "User detaches a session, relaunches the app, reattaches, and sees the same pane layout restored"
    - "An external AppleScript targeting smux executes successfully and receives a confirmed response"
  artifacts:
    - path: "macos/smux/Sources/SmuxApp/BrowserPanelView.swift"
      provides: "WKWebView wrapper with toolbar, navigation, URL bar"
    - path: "macos/smux/Sources/SmuxApp/BrowserAutomation.swift"
      provides: "domSnapshot, domTree, executeJS, click, type, screenshot"
    - path: "macos/smux/Sources/SmuxApp/SessionDetachReattach.swift"
      provides: "attach/detach/reattachAll via daemon IPC + state persistence"
    - path: "macos/smux/Sources/SmuxApp/SessionRestore.swift"
      provides: "WorkspaceState/SplitState codable save/load"
    - path: "macos/smux/Sources/SmuxApp/WorkspaceWindowController.swift"
      provides: "toggleBrowser, saveState/restoreState with split serialization, destroyAllSurfaces"
    - path: "macos/smux/Sources/SmuxApp/AppleScriptSupport.swift"
      provides: "7 Apple Event handlers registered via NSAppleEventManager"
    - path: "macos/smux/Sources/SmuxApp/main.swift"
      provides: "Menu bindings, delegate wiring, lifecycle hooks"
  key_links:
    - from: "main.swift"
      to: "WorkspaceWindowController"
      via: "toggleBrowser menu item (Cmd+Shift+B)"
    - from: "WorkspaceWindowController"
      to: "BrowserPanelView"
      via: "toggleBrowser() creates NSSplitView + BrowserPanelView"
    - from: "WorkspaceWindowController"
      to: "BrowserAutomation"
      via: "automation() lazy-creates BrowserAutomation(browserPanel:)"
    - from: "BrowserAutomation"
      to: "BrowserPanelView"
      via: "evaluateJavaScript delegation"
    - from: "main.swift"
      to: "SessionDetachReattach"
      via: "sessionManager created in applicationDidFinishLaunching"
    - from: "main.swift"
      to: "WorkspaceWindowController.saveState/restoreState"
      via: "restoreState on launch (line 99), saveState on shutdown (line 209)"
    - from: "WorkspaceWindowController.saveState"
      to: "SessionRestore"
      via: "collectSplitDirections -> sessionRestore.save()"
    - from: "main.swift"
      to: "AppleScriptSupport"
      via: "appleScriptSupport created and registerHandlers() called"
human_verification:
  - test: "Press Cmd+Shift+B and verify browser panel opens with localhost:3000"
    expected: "NSSplitView shows terminal left, WKWebView right, URL bar reads http://localhost:3000"
    why_human: "Requires running app with visual UI -- WKWebView rendering cannot be verified via static analysis"
  - test: "Call domSnapshot() and verify non-empty HTML returned"
    expected: "Returns string starting with <html containing page structure"
    why_human: "Requires live WKWebView with a loaded page -- JavaScript execution is runtime-only"
  - test: "Create splits, quit app, relaunch, verify splits restored"
    expected: "Same number and direction of splits appear on relaunch"
    why_human: "Requires full app lifecycle test with visual pane inspection"
  - test: "Run osascript -e 'tell application SmuxApp to do script ls' and verify execution"
    expected: "ls command executes in terminal, no AppleScript error returned"
    why_human: "Requires running app as proper .app bundle with Apple Event registration"
---

# Phase 4: E2E Feature Verification -- Verification Report

**Phase Goal:** Browser panel, browser automation, session detach/reattach, and AppleScript hooks all work end-to-end
**Verified:** 2026-03-26T15:30:00Z
**Status:** human_needed
**Re-verification:** No -- initial verification

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | User presses Cmd+Shift+B and a browser panel opens alongside the terminal pane and renders a localhost URL correctly | ? NEEDS HUMAN | Code wiring complete: menu item (main.swift:172-173) -> toggleBrowser() -> NSSplitView + BrowserPanelView + navigate("http://localhost:3000"). All code paths substantive. |
| 2 | Browser automation DOM snapshot call returns actual page content (non-empty, structurally valid HTML/text) | ? NEEDS HUMAN | domSnapshot() executes `document.documentElement.outerHTML` via real evaluateJavaScript, returns Result<String>. No stubs. Requires live WKWebView. |
| 3 | User detaches a session, relaunches the app, reattaches, and sees the same pane layout restored | ? NEEDS HUMAN | saveState() walks NSSplitView via collectSplitDirections(), saves direction+ratio to JSON. restoreState() replays splits via doSplit(). Wired in lifecycle. Note: ratio saved but NOT applied on restore (always 50/50). |
| 4 | An external AppleScript targeting smux executes successfully and receives a confirmed response | ? NEEDS HUMAN | 7 handlers registered via NSAppleEventManager with correct fourCharCode. handleListSessions sets reply descriptor. Requires .app bundle for Apple Events. |

**Score:** 4/4 truths verified at code level -- all require human runtime verification

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `BrowserPanelView.swift` | WKWebView with toolbar, URL bar, navigation | VERIFIED | 287 lines, full WKWebView implementation with toolbar (back/forward/reload/URL bar/focus toggle), WKNavigationDelegate, WKScriptMessageHandler for console.log bridge, navigate(), evaluateJavaScript(), takeScreenshot() |
| `BrowserAutomation.swift` | DOM snapshot, interaction, screenshot | VERIFIED | 397 lines, domSnapshot() via outerHTML, domTree() with depth-limited walk, click/doubleClick/type/fillForm, getText/getAttribute/elementExists/waitForElement, screenshot, scrollTo, executeRawJS. Full AutomationError enum. String.escapedForJS extension. |
| `SessionDetachReattach.swift` | Attach/detach/reattach via IPC | VERIFIED | 187 lines, attach() sends AttachSession IPC with graceful fallback when daemon unavailable, detach() sends DetachSession, reattachAll() loads from disk, saveAttachState/loadAttachState with JSON persistence at ~/.smux/attach-state.json |
| `SessionRestore.swift` | Workspace state codable save/load | VERIFIED | 64 lines, WorkspaceState/WindowState/TabState/SplitState/SplitChild/FrameRect all Codable, save() creates directory + writes JSON, load() reads JSON |
| `WorkspaceWindowController.swift` | toggleBrowser, saveState/restoreState, split serialization | VERIFIED | 708 lines, toggleBrowser() creates NSSplitView with terminal+BrowserPanelView, saveState() calls collectSplitDirections() to walk NSSplitView hierarchy, restoreState() replays splits. Browser navigates to localhost:3000 on open. |
| `AppleScriptSupport.swift` | 7 Apple Event handlers | VERIFIED | 189 lines, 7 handlers (dosc, splt, brws, ssht, ntab, sess, ntfy) registered with correct event classes. handleListSessions returns reply. handleDoScript sends text to terminal. fourCharCode helper. |
| `main.swift` | Menu bindings, delegate wiring | VERIFIED | 271 lines. Browser toggle: Cmd+Shift+B (line 172-173). SessionDetachReattach created at launch (line 110), reattachAll() called (line 111). AppleScriptSupport created and registerHandlers() called (line 118-119). saveState() in performCleanShutdown (line 209). restoreState() at launch (line 99). |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| main.swift menu | WorkspaceWindowController.toggleBrowser | NSMenuItem "b" + [.command, .shift] -> @objc toggleBrowser() | WIRED | Line 172-173 creates menu item, line 237 dispatches to controller |
| toggleBrowser() | BrowserPanelView | NSSplitView creation + BrowserPanelView init | WIRED | Line 555 creates BrowserPanelView, line 566 adds to split, line 579 navigates to localhost:3000 |
| toggleBrowser() | BrowserAutomation | BrowserAutomation(browserPanel: browser) | WIRED | Line 582 creates automation instance |
| BrowserAutomation.domSnapshot | BrowserPanelView.evaluateJavaScript | executeJS -> panel.evaluateJavaScript | WIRED | Line 344-356 delegates to browserPanel, line 18 calls executeJS("document.documentElement.outerHTML") |
| main.swift | SessionDetachReattach | sessionManager = SessionDetachReattach() | WIRED | Line 110 creates, line 111 calls reattachAll() |
| main.swift shutdown | saveState() | performCleanShutdown -> workspaceController?.saveState() | WIRED | Line 209 |
| main.swift launch | restoreState() | applicationDidFinishLaunching -> workspaceController?.restoreState() | WIRED | Line 99 |
| saveState() | SessionRestore.save() | collectSplitDirections -> sessionRestore.save(state:) | WIRED | Lines 634-648 serialize splits and save |
| restoreState() | doSplit() | Loop over tab.splits -> doSplit(vertical:) | WIRED | Lines 657-661 replay splits |
| main.swift | AppleScriptSupport.registerHandlers() | appleScriptSupport = AppleScriptSupport(controller: wc); registerHandlers() | WIRED | Lines 117-119 |
| AppleScript handlers | WorkspaceWindowController | controller?.splitVertical/Horizontal, openInBrowser, newTab | WIRED | All 7 handlers dispatch to controller methods |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|----------|--------------|--------|--------------------|--------|
| BrowserPanelView | webView URL/content | WKWebView.load(URLRequest) | Yes -- loads real URL via HTTP | FLOWING |
| BrowserAutomation.domSnapshot | html string | evaluateJavaScript("document.documentElement.outerHTML") | Yes -- real JS execution on live page | FLOWING (runtime) |
| SessionRestore | WorkspaceState | collectSplitDirections walks NSSplitView hierarchy | Yes -- reads actual view tree | FLOWING |
| AppleScriptSupport.handleListSessions | sessions | controller?.missionState.sessions | Depends on daemon -- returns "(no sessions)" if empty | FLOWING (may be empty) |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Swift build succeeds | `cd macos/smux && swift build` | Build complete! (0.11s) | PASS |
| BrowserPanelView compiles | swift build | Included in successful build | PASS |
| BrowserAutomation compiles | swift build | Included in successful build | PASS |
| SessionDetachReattach compiles | swift build | Included in successful build | PASS |
| AppleScriptSupport compiles | swift build | Included in successful build | PASS |

Step 7b runtime behavioral checks: SKIPPED -- macOS GUI app requires launching with display server, cannot test headlessly.

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|-----------|-------------|--------|----------|
| E2E-01 | 04-01-PLAN.md | Browser panel (Cmd+Shift+B) opens alongside terminal pane and renders localhost URL | NEEDS HUMAN | toggleBrowser() creates NSSplitView with BrowserPanelView, navigates to localhost:3000. Menu binding correct. All code paths substantive. |
| E2E-02 | 04-01-PLAN.md | Browser automation DOM snapshot returns actual page content (not empty) | NEEDS HUMAN | domSnapshot() calls evaluateJavaScript("document.documentElement.outerHTML"), returns Result<String>. No stubs or empty returns. |
| E2E-03 | 04-01-PLAN.md | Session detach saves state; reattach restores session with same pane layout | NEEDS HUMAN | saveState/restoreState wired in lifecycle. collectSplitDirections() walks real NSSplitView hierarchy. restoreState() replays splits. Minor gap: ratio not applied on restore. |
| E2E-04 | 04-01-PLAN.md | AppleScript hook executes a test script targeting smux and confirms response | NEEDS HUMAN | 7 handlers registered via NSAppleEventManager. handleListSessions returns reply. handleDoScript types into terminal. Requires .app bundle. |

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| WorkspaceWindowController.swift | 651-661 | restoreState() replays split directions but ignores saved ratio (always 50/50) | Warning | Split proportions not preserved across restarts -- layout structure correct, sizing approximate |
| SessionDetachReattach.swift | 55-83 | attach() always returns true (fallback when daemon unavailable) | Info | Graceful degradation -- local-only attach when daemon not running, which is acceptable for standalone use |
| AppleScriptSupport.swift | 89-97 | handleDoScript does not set reply descriptor | Info | Consistent with terminal "do script" convention -- fire-and-forget is expected behavior |

### Human Verification Required

### 1. Browser Panel Toggle (E2E-01)

**Test:** Launch smux, press Cmd+Shift+B
**Expected:** A browser panel appears on the right side of the terminal (NSSplitView), URL bar shows "http://localhost:3000", WKWebView attempts to load the page. Press Cmd+Shift+B again to close it.
**Why human:** Requires running macOS GUI app with display server. WKWebView rendering is runtime-only.

### 2. DOM Snapshot (E2E-02)

**Test:** Open browser panel, navigate to a page (e.g., a simple HTML file served locally), then invoke domSnapshot via debug console or automation
**Expected:** Returns a non-empty string starting with `<html` containing the page's DOM structure
**Why human:** Requires live WKWebView with a loaded page. JavaScript execution and result capture are runtime behaviors.

### 3. Session Layout Persistence (E2E-03)

**Test:** Create 2 vertical splits (Cmd+D twice), quit app, relaunch
**Expected:** App relaunches with 2 vertical splits restored (note: proportions may not be preserved -- they default to 50/50)
**Why human:** Requires full app lifecycle (launch -> split -> quit -> relaunch) with visual inspection of pane layout.

### 4. AppleScript Execution (E2E-04)

**Test:** Launch smux as a proper .app bundle, then run: `osascript -e 'tell application "SmuxApp" to do script "echo hello"'`
**Expected:** "echo hello" appears in the active terminal pane. No AppleScript error returned.
**Why human:** Apple Events require the app to be registered with the system as a proper .app bundle. Cannot test from CLI-launched swift binary.

### 5. AppleScript Reply (E2E-04 extended)

**Test:** Run: `osascript -e 'tell application "SmuxApp" to list sessions'`
**Expected:** Returns session list string (or "(no sessions)" if no daemon)
**Why human:** Verifies reply descriptor is correctly returned through Apple Event mechanism.

### Gaps Summary

No code-level gaps found. All artifacts exist, are substantive (not stubs), and are properly wired through the application lifecycle. The build succeeds cleanly.

One minor quality note: `restoreState()` saves split ratios but does not apply them during restoration -- splits always restore at 50/50. This is a polish issue, not a blocker for E2E-03 (the pane layout structure IS restored).

All four E2E requirements require human runtime verification because they involve macOS GUI rendering (WKWebView), OS-level event handling (Apple Events), and full application lifecycle testing (quit + relaunch). Static code analysis confirms the implementation is complete and correctly wired, but final confirmation requires running the application.

---

_Verified: 2026-03-26T15:30:00Z_
_Verifier: Claude (gsd-verifier)_
