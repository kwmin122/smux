import { useState, useCallback, useRef } from 'react'
import type { Terminal, IDisposable } from '@xterm/xterm'

export interface CommandRecord {
  id: string
  command: string
  cwd: string
  exitCode: number | null
  startLine: number
  endLine: number | null
  startTime: number
  endTime: number | null
  status: 'running' | 'success' | 'error' | 'unknown'
}

interface ShellIntegrationState {
  commands: CommandRecord[]
  currentCwd: string
  isIntegrated: boolean
}

/**
 * Hook that parses OSC 633 escape sequences from xterm.js terminal output
 * to track command boundaries, exit codes, and working directories.
 *
 * OSC 633 Protocol (VSCode standard):
 *   A = Prompt Start
 *   B = Prompt End (user typing begins)
 *   C = Pre-Execution (command about to run)
 *   D;exitcode = Execution Finished
 *   E;commandline = Command Line text
 *   P;Key=Value = Property (e.g., Cwd=/path)
 */
export function useShellIntegration() {
  const [state, setState] = useState<ShellIntegrationState>({
    commands: [],
    currentCwd: '',
    isIntegrated: false,
  })

  const pendingCommandRef = useRef<Partial<CommandRecord> | null>(null)
  const commandCounterRef = useRef(0)
  const disposablesRef = useRef<IDisposable[]>([])

  const attach = useCallback((terminal: Terminal) => {
    // Clean up previous listeners
    disposablesRef.current.forEach(d => d.dispose())
    disposablesRef.current = []

    // Listen for OSC 633 sequences via the terminal's custom OSC handler
    // xterm.js fires the registered handler when it encounters \e]633;...\a
    const oscHandler = terminal.parser.registerOscHandler(633, (data: string) => {
      const parts = data.split(';')
      const marker = parts[0]

      setState(prev => {
        const next = { ...prev, isIntegrated: true }

        switch (marker) {
          case 'A': {
            // Prompt Start — if there's a pending command that was running, mark its end
            if (pendingCommandRef.current && pendingCommandRef.current.status === 'running') {
              const cmd = pendingCommandRef.current as CommandRecord
              cmd.endLine = terminal.buffer.active.cursorY + terminal.buffer.active.baseY
              cmd.endTime = Date.now()
              if (cmd.exitCode === null) cmd.status = 'unknown'
              next.commands = [...prev.commands.filter(c => c.id !== cmd.id), cmd]
              pendingCommandRef.current = null
            }
            break
          }

          case 'B': {
            // Prompt End — user can start typing
            // We record the cursor position as potential command start
            pendingCommandRef.current = {
              id: `cmd-${++commandCounterRef.current}`,
              startLine: terminal.buffer.active.cursorY + terminal.buffer.active.baseY,
              startTime: Date.now(),
              cwd: prev.currentCwd,
              command: '',
              exitCode: null,
              endLine: null,
              endTime: null,
              status: 'unknown',
            }
            break
          }

          case 'C': {
            // Pre-Execution — command is about to run
            if (pendingCommandRef.current) {
              pendingCommandRef.current.status = 'running'
              pendingCommandRef.current.startTime = Date.now()
            }
            break
          }

          case 'D': {
            // Execution Finished — D;exitcode
            const exitCode = parts.length > 1 ? parseInt(parts[1], 10) : 0
            if (pendingCommandRef.current) {
              const cmd: CommandRecord = {
                id: pendingCommandRef.current.id || `cmd-${++commandCounterRef.current}`,
                command: pendingCommandRef.current.command || '',
                cwd: pendingCommandRef.current.cwd || prev.currentCwd,
                exitCode,
                startLine: pendingCommandRef.current.startLine ?? 0,
                endLine: terminal.buffer.active.cursorY + terminal.buffer.active.baseY,
                startTime: pendingCommandRef.current.startTime ?? Date.now(),
                endTime: Date.now(),
                status: exitCode === 0 ? 'success' : 'error',
              }
              next.commands = [...prev.commands, cmd]
              pendingCommandRef.current = null
            }
            break
          }

          case 'E': {
            // Command Line — E;commandtext
            const commandText = parts.slice(1).join(';')
            if (pendingCommandRef.current) {
              pendingCommandRef.current.command = commandText
            }
            break
          }

          case 'P': {
            // Property — P;Key=Value
            const propData = parts.slice(1).join(';')
            const eqIdx = propData.indexOf('=')
            if (eqIdx > 0) {
              const key = propData.substring(0, eqIdx)
              const value = propData.substring(eqIdx + 1)
              if (key === 'Cwd') {
                next.currentCwd = value
              }
            }
            break
          }
        }

        return next
      })

      return true // Mark as handled
    })

    disposablesRef.current.push(oscHandler)
  }, [])

  const detach = useCallback(() => {
    disposablesRef.current.forEach(d => d.dispose())
    disposablesRef.current = []
  }, [])

  const getCommandAtLine = useCallback((line: number): CommandRecord | null => {
    // Find the command whose output range includes this line
    for (let i = state.commands.length - 1; i >= 0; i--) {
      const cmd = state.commands[i]
      if (cmd.startLine <= line && (cmd.endLine === null || cmd.endLine >= line)) {
        return cmd
      }
    }
    return null
  }, [state.commands])

  const getRunningCommand = useCallback((): CommandRecord | null => {
    if (pendingCommandRef.current?.status === 'running') {
      return pendingCommandRef.current as CommandRecord
    }
    return null
  }, [])

  return {
    commands: state.commands,
    currentCwd: state.currentCwd,
    isIntegrated: state.isIntegrated,
    attach,
    detach,
    getCommandAtLine,
    getRunningCommand,
  }
}
