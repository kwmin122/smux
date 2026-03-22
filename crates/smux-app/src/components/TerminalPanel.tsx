import { useEffect, useRef, useImperativeHandle, forwardRef } from 'react'
import { Terminal } from '@xterm/xterm'
import { FitAddon } from '@xterm/addon-fit'
import '@xterm/xterm/css/xterm.css'

export interface TerminalPanelHandle {
  write: (data: string) => void
  writeln: (data: string) => void
  clear: () => void
}

interface TerminalPanelProps {
  role: 'planner' | 'verifier'
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
  function TerminalPanel({ role }, ref) {
    const containerRef = useRef<HTMLDivElement>(null)
    const terminalRef = useRef<Terminal | null>(null)

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
        convertEol: true,
        allowProposedApi: true,
        theme: getThemeColors(),
      })

      const fitAddon = new FitAddon()
      terminal.loadAddon(fitAddon)

      terminal.open(containerRef.current)
      fitAddon.fit()

      terminal.writeln(`\x1b[90m> smux v0.3.0 — ${role}\x1b[0m`)
      terminal.writeln('')

      terminalRef.current = terminal

      const observer = new ResizeObserver(() => {
        requestAnimationFrame(() => fitAddon.fit())
      })
      observer.observe(containerRef.current)

      return () => {
        observer.disconnect()
        terminal.dispose()
      }
    }, [role])

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
