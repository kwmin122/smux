import { useEffect, useRef, useImperativeHandle, forwardRef } from 'react'
import { Terminal } from '@xterm/xterm'
import { FitAddon } from '@xterm/addon-fit'
import '@xterm/xterm/css/xterm.css'

declare global {
  interface Window {
    __TAURI_INTERNALS__?: unknown
  }
}

const isTauri = !!window.__TAURI_INTERNALS__

export interface TerminalPanelHandle {
  write: (data: string) => void
  writeln: (data: string) => void
  clear: () => void
}

interface TerminalPanelProps {
  role: 'planner' | 'verifier' | 'terminal'
  /** When true, creates a real PTY shell and connects input/output */
  ptyMode?: boolean
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
  function TerminalPanel({ role, ptyMode = false }, ref) {
    const containerRef = useRef<HTMLDivElement>(null)
    const terminalRef = useRef<Terminal | null>(null)
    const fitAddonRef = useRef<FitAddon | null>(null)
    const ptyIdRef = useRef<string | null>(null)

    useImperativeHandle(ref, () => ({
      write(data: string) {
        terminalRef.current?.write(data)
      },
      writeln(data: string) {
        terminalRef.current?.writeln(data)
      },
      clear() {
        terminalRef.current?.clear()
      },
    }))

    useEffect(() => {
      if (!containerRef.current) return

      const terminal = new Terminal({
        fontFamily: '"JetBrains Mono", monospace',
        fontSize: 13,
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

      terminal.open(containerRef.current)
      fitAddon.fit()

      terminalRef.current = terminal
      fitAddonRef.current = fitAddon

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
            const tabId = await invoke<string>('create_pty', {
              rows: terminal.rows,
              cols: terminal.cols,
            })
            ptyIdRef.current = tabId

            // Listen for output BEFORE starting read loop (prevent race)
            const unlistenOutput = await listen<string>(`pty-output-${tabId}`, (event) => {
              terminal.write(event.payload)
            })

            const unlistenExit = await listen(`pty-exit-${tabId}`, () => {
              terminal.writeln('\x1b[90m\r\n[process exited]\x1b[0m')
            })

            // Start the PTY read loop
            await invoke('start_pty', { tabId })

            // Send keyboard input to PTY
            const onDataDisposable = terminal.onData((data) => {
              invoke('write_pty', { tabId, data }).catch(() => {})
            })

            cleanupPty = () => {
              onDataDisposable.dispose()
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
        cleanupPty?.()
        terminal.dispose()
      }
    }, [role, ptyMode])

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

    return <div ref={containerRef} className="w-full h-full" />
  }
)
