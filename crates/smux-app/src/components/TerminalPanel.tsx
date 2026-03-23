import { useState, useEffect, useRef, useImperativeHandle, forwardRef } from 'react'
import { Terminal } from '@xterm/xterm'
import { FitAddon } from '@xterm/addon-fit'
import { Unicode11Addon } from '@xterm/addon-unicode11'
import { WebglAddon } from '@xterm/addon-webgl'
import { useShellIntegration, type CommandRecord } from '../hooks/useShellIntegration'
import { useTerminalLinks } from '../hooks/useTerminalLinks'
import { SearchOverlay } from './SearchOverlay'
import { CommandGutter } from './CommandGutter'
import { StickyScroll } from './StickyScroll'
import { redactSecrets } from './GitInfo'
import '@xterm/xterm/css/xterm.css'

declare global {
  interface Window {
    __TAURI_INTERNALS__?: unknown
  }
}

const isTauri = !!window.__TAURI_INTERNALS__

export interface TerminalPanelHandle {
  /** Write to xterm display (visual only, does NOT execute in shell) */
  write: (data: string) => void
  writeln: (data: string) => void
  /** Write to PTY stdin (actually executes in the shell) */
  writeToPty: (data: string) => void
  clear: () => void
  getCommands: () => CommandRecord[]
  getCurrentCwd: () => string
  isShellIntegrated: () => boolean
}

interface TerminalPanelProps {
  role: 'planner' | 'verifier' | 'terminal'
  /** When true, creates a real PTY shell and connects input/output */
  ptyMode?: boolean
  /** Working directory for the PTY shell */
  cwd?: string
  /** Custom shell command (e.g., 'claude -p "task"') */
  shellCmd?: string
  /** Callback when shell integration detects a command completion */
  onCommandComplete?: (cmd: CommandRecord) => void
  /** Callback when CWD changes via shell integration */
  onCwdChange?: (cwd: string) => void
  /** Font family override (from config) */
  fontFamily?: string
  /** Font size override (from config) */
  fontSize?: number
  /** Callback for every PTY output chunk (used by orchestrator to capture output) */
  onPtyOutput?: (data: string) => void
}

function getThemeColors() {
  const style = getComputedStyle(document.documentElement)
  return {
    background: style.getPropertyValue('--surface-container-lowest').trim(),
    foreground: style.getPropertyValue('--on-surface-variant').trim(),
    cursor: style.getPropertyValue('--primary').trim(),
    cursorAccent: style.getPropertyValue('--surface-container-lowest').trim(),
    selectionBackground: style.getPropertyValue('--primary').trim() + '40',
    black: style.getPropertyValue('--surface-container-lowest').trim(),
    brightBlack: style.getPropertyValue('--outline').trim(),
    white: style.getPropertyValue('--on-surface').trim(),
    brightWhite: '#ffffff',
    green: style.getPropertyValue('--secondary').trim(),
    brightGreen: style.getPropertyValue('--secondary').trim(),
    cyan: style.getPropertyValue('--primary').trim(),
    brightCyan: style.getPropertyValue('--primary').trim(),
    red: style.getPropertyValue('--error').trim(),
    brightRed: style.getPropertyValue('--error').trim(),
    yellow: style.getPropertyValue('--tertiary').trim(),
    brightYellow: style.getPropertyValue('--tertiary').trim(),
    blue: style.getPropertyValue('--primary').trim(),
    brightBlue: style.getPropertyValue('--primary').trim(),
    magenta: style.getPropertyValue('--tertiary').trim(),
    brightMagenta: style.getPropertyValue('--tertiary').trim(),
  }
}

export const TerminalPanel = forwardRef<TerminalPanelHandle, TerminalPanelProps>(
  function TerminalPanel({ role, ptyMode = false, cwd, shellCmd, onCommandComplete, onCwdChange, onPtyOutput, fontFamily, fontSize }, ref) {
    const containerRef = useRef<HTMLDivElement>(null)
    const terminalRef = useRef<Terminal | null>(null)
    const fitAddonRef = useRef<FitAddon | null>(null)
    const ptyIdRef = useRef<string | null>(null)
    const shellIntegration = useShellIntegration()
    const terminalLinks = useTerminalLinks()
    const [showSearch, setShowSearch] = useState(false)
    const [viewportTopLine, setViewportTopLine] = useState(0)
    const [baseY, setBaseY] = useState(0)
    // Ref to always hold latest onPtyOutput callback (avoids stale closure in PTY listener)
    const onPtyOutputRef = useRef(onPtyOutput)
    useEffect(() => { onPtyOutputRef.current = onPtyOutput })

    useImperativeHandle(ref, () => ({
      write(data: string) {
        // Write to xterm display (visual only)
        terminalRef.current?.write(data)
      },
      writeln(data: string) {
        terminalRef.current?.writeln(data)
      },
      /** Write to PTY stdin (actually executes in the shell) */
      writeToPty(data: string) {
        if (ptyIdRef.current && isTauri) {
          import('@tauri-apps/api/core').then(({ invoke }) => {
            invoke('write_pty', { tabId: ptyIdRef.current, data }).catch(() => {})
          })
        }
      },
      clear() {
        terminalRef.current?.clear()
      },
      getCommands() {
        return shellIntegration.commands
      },
      getCurrentCwd() {
        return shellIntegration.currentCwd
      },
      isShellIntegrated() {
        return shellIntegration.isIntegrated
      },
    }))

    useEffect(() => {
      if (!containerRef.current) return

      const terminal = new Terminal({
        fontFamily: fontFamily ? `"${fontFamily}", monospace` : '"JetBrains Mono", monospace',
        fontSize: fontSize || 14,
        lineHeight: 1.4,
        cursorBlink: true,
        cursorStyle: 'block',
        scrollback: 10000,
        convertEol: !ptyMode, // PTY handles EOL itself
        allowProposedApi: true,
        theme: getThemeColors(),
      })

      const fitAddon = new FitAddon()
      terminal.loadAddon(fitAddon)

      // Load Unicode11 addon for correct CJK/Korean character widths
      const unicode11 = new Unicode11Addon()
      terminal.loadAddon(unicode11)
      terminal.unicode.activeVersion = '11'

      // Load WebGL addon for GPU-accelerated rendering (fallback to canvas if unavailable)
      try {
        const webgl = new WebglAddon()
        webgl.onContextLoss(() => webgl.dispose())
        terminal.loadAddon(webgl)
      } catch {
        // WebGL not available, use default canvas renderer
      }

      terminal.open(containerRef.current)
      fitAddon.fit()

      terminalRef.current = terminal
      fitAddonRef.current = fitAddon

      // Attach shell integration OSC 633 parser and clickable links
      if (ptyMode) {
        shellIntegration.attach(terminal)
        terminalLinks.attach(terminal)
      }

      // Track scroll position (throttled to ~20fps to avoid excessive React re-renders)
      let scrollRaf = 0
      const scrollDisposable = terminal.onScroll(() => {
        if (scrollRaf) return
        scrollRaf = requestAnimationFrame(() => {
          scrollRaf = 0
          setViewportTopLine(terminal.buffer.active.viewportY)
          setBaseY(terminal.buffer.active.baseY)
        })
      })

      if (!ptyMode) {
        terminal.writeln(`\x1b[90m> smux v0.3.0 — ${role}\x1b[0m`)
        terminal.writeln('')
      }

      // PTY mode: create a real shell
      let cleanupPty: (() => void) | null = null
      if (ptyMode && isTauri) {
        ;(async () => {
          try {
            const { invoke } = await import('@tauri-apps/api/core')
            const { listen } = await import('@tauri-apps/api/event')

            // Create PTY with current terminal dimensions
            const ptyArgs: Record<string, unknown> = {
              rows: terminal.rows,
              cols: terminal.cols,
            }
            if (cwd) ptyArgs.cwd = cwd
            if (shellCmd) ptyArgs.shellCmd = shellCmd
            const tabId = await invoke<string>('create_pty', ptyArgs)
            ptyIdRef.current = tabId

            // Listen for output BEFORE starting read loop (prevent race)
            const unlistenOutput = await listen<string>(`pty-output-${tabId}`, (event) => {
              // Apply secret redaction to terminal output before rendering
              const output = redactSecrets(event.payload)
              terminal.write(output)
              // Feed output to orchestrator via ref (avoids stale closure)
              onPtyOutputRef.current?.(output)
            })

            const unlistenExit = await listen(`pty-exit-${tabId}`, () => {
              terminal.writeln('\x1b[90m\r\n[process exited]\x1b[0m')
            })

            // Start the PTY read loop
            await invoke('start_pty', { tabId })

            // --- Korean / CJK IME composition guard ---
            // In WKWebView (Tauri v2 macOS), xterm.js's internal CompositionHelper
            // may not reliably suppress onData during IME composition, causing
            // individual jamo (ㅈ ㅓ ㅇ) to be sent instead of composed syllables (정).
            // We track composition state on the textarea and gate PTY writes.
            let isComposing = false
            let compositionEndData: string | null = null
            const textarea = terminal.textarea
            const onCompositionStart = () => { isComposing = true }
            const onCompositionEnd = (e: CompositionEvent) => {
              isComposing = false
              // Store the composed text; the next onData call should deliver it.
              // If onData fires with the same text, we let it through (normal path).
              // If onData doesn't fire (WKWebView bug), we send it after a microtask.
              compositionEndData = e.data || null
              if (compositionEndData) {
                Promise.resolve().then(() => {
                  // If onData already sent this, compositionEndData was cleared.
                  // Otherwise, WKWebView swallowed it — send directly.
                  if (compositionEndData) {
                    invoke('write_pty', { tabId, data: compositionEndData }).catch(() => {})
                    compositionEndData = null
                  }
                })
              }
            }
            textarea?.addEventListener('compositionstart', onCompositionStart)
            textarea?.addEventListener('compositionend', onCompositionEnd)

            // Send keyboard input to PTY (skip during IME composition)
            const onDataDisposable = terminal.onData((data) => {
              if (!isComposing) {
                // Clear compositionEndData if onData delivers the composed result,
                // preventing the microtask fallback from sending a duplicate.
                if (compositionEndData && data === compositionEndData) {
                  compositionEndData = null
                }
                invoke('write_pty', { tabId, data }).catch(() => {})
              }
            })

            cleanupPty = () => {
              onDataDisposable.dispose()
              textarea?.removeEventListener('compositionstart', onCompositionStart)
              textarea?.removeEventListener('compositionend', onCompositionEnd)
              unlistenOutput()
              unlistenExit()
              invoke('close_pty', { tabId }).catch(() => {})
            }
          } catch (e) {
            terminal.writeln(`\x1b[31m[PTY error] ${e}\x1b[0m`)
          }
        })()
      }

      // Resize handling
      const resizeObserver = new ResizeObserver(() => {
        requestAnimationFrame(() => {
          fitAddon.fit()
          // Sync resize to PTY if in pty mode
          if (ptyMode && ptyIdRef.current && isTauri) {
            import('@tauri-apps/api/core').then(({ invoke }) => {
              invoke('resize_pty', {
                tabId: ptyIdRef.current,
                rows: terminal.rows,
                cols: terminal.cols,
              }).catch(() => {})
            })
          }
        })
      })
      resizeObserver.observe(containerRef.current)

      return () => {
        resizeObserver.disconnect()
        scrollDisposable.dispose()
        shellIntegration.detach()
        terminalLinks.detach()
        cleanupPty?.()
        terminal.dispose()
      }
    }, [role, ptyMode, cwd, shellCmd])

    // Fire callbacks when shell integration detects changes
    useEffect(() => {
      if (shellIntegration.commands.length > 0 && onCommandComplete) {
        const latest = shellIntegration.commands[shellIntegration.commands.length - 1]
        if (latest.endTime !== null) {
          onCommandComplete(latest)
        }
      }
    }, [shellIntegration.commands, onCommandComplete])

    useEffect(() => {
      if (shellIntegration.currentCwd && onCwdChange) {
        onCwdChange(shellIntegration.currentCwd)
      }
    }, [shellIntegration.currentCwd, onCwdChange])

    // ⌘F to open search
    useEffect(() => {
      const handler = (e: KeyboardEvent) => {
        if ((e.metaKey || e.ctrlKey) && e.key === 'f') {
          e.preventDefault()
          setShowSearch(true)
        }
      }
      window.addEventListener('keydown', handler)
      return () => window.removeEventListener('keydown', handler)
    }, [])

    // Re-apply theme colors when the data-theme attribute changes
    useEffect(() => {
      const html = document.documentElement
      const observer = new MutationObserver(() => {
        if (terminalRef.current) {
          terminalRef.current.options.theme = getThemeColors()
        }
      })
      observer.observe(html, { attributes: true, attributeFilter: ['data-theme'] })
      return () => observer.disconnect()
    }, [])

    return (
      <div className="relative w-full h-full">
        {/* Sticky scroll header */}
        {ptyMode && (
          <StickyScroll
            commands={shellIntegration.commands}
            viewportTopLine={viewportTopLine}
            visible={shellIntegration.isIntegrated}
          />
        )}
        {/* Command exit code gutter */}
        {ptyMode && shellIntegration.isIntegrated && (
          <CommandGutter
            commands={shellIntegration.commands}
            terminalElement={containerRef.current}
            lineHeight={terminalRef.current ? Math.round((terminalRef.current.options.lineHeight ?? 1.4) * (terminalRef.current.options.fontSize ?? 13)) : 18}
            baseY={baseY}
          />
        )}
        <div ref={containerRef} className="w-full h-full" />
        {showSearch && terminalRef.current && (
          <SearchOverlay
            terminal={terminalRef.current}
            onClose={() => setShowSearch(false)}
          />
        )}
      </div>
    )
  }
)
