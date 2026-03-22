import { useMemo, useCallback } from 'react'
import type { CommandRecord } from '../hooks/useShellIntegration'

interface CommandGutterProps {
  commands: CommandRecord[]
  terminalElement: HTMLElement | null
  lineHeight: number
  baseY: number
}

const GUTTER_WIDTH = 16
const DOT_SIZE = 6

function statusClasses(status: CommandRecord['status']): string {
  switch (status) {
    case 'success':
      return 'bg-green-400'
    case 'error':
      return 'bg-red-400'
    case 'running':
      return 'bg-yellow-400 animate-pulse'
    case 'unknown':
    default:
      return 'bg-outline/50'
  }
}

export function CommandGutter({ commands, terminalElement, lineHeight, baseY }: CommandGutterProps) {
  const containerHeight = terminalElement?.clientHeight ?? 0

  const visibleCommands = useMemo(() => {
    if (!containerHeight || lineHeight <= 0) return []

    const maxVisibleLines = Math.ceil(containerHeight / lineHeight)
    const viewTop = baseY
    const viewBottom = baseY + maxVisibleLines

    return commands.filter(cmd => {
      return cmd.startLine >= viewTop && cmd.startLine < viewBottom
    })
  }, [commands, containerHeight, lineHeight, baseY])

  const handleDotClick = useCallback((cmd: CommandRecord) => {
    console.log('[CommandGutter] clicked command:', cmd.id, cmd.command, `exit=${cmd.exitCode}`)
  }, [])

  if (!terminalElement || commands.length === 0) return null

  return (
    <div
      className="absolute top-0 left-0 pointer-events-none overflow-hidden"
      style={{ width: GUTTER_WIDTH, height: containerHeight }}
    >
      {visibleCommands.map(cmd => {
        const y = (cmd.startLine - baseY) * lineHeight
        const isError = cmd.status === 'error'

        return (
          <div key={cmd.id} className="absolute left-0" style={{ top: y, width: GUTTER_WIDTH, height: lineHeight }}>
            {/* Faint red row highlight for errors */}
            {isError && (
              <div
                className="absolute inset-0 bg-red-400/10 rounded-sm"
                style={{ width: '100vw' }}
              />
            )}

            {/* Status dot */}
            <button
              type="button"
              className={[
                'absolute pointer-events-auto cursor-pointer rounded-full border-0 p-0',
                statusClasses(cmd.status),
              ].join(' ')}
              style={{
                width: DOT_SIZE,
                height: DOT_SIZE,
                top: (lineHeight - DOT_SIZE) / 2,
                left: (GUTTER_WIDTH - DOT_SIZE) / 2,
              }}
              title={`${cmd.command || '(empty)'} — exit ${cmd.exitCode ?? '?'}`}
              onClick={() => handleDotClick(cmd)}
            />
          </div>
        )
      })}
    </div>
  )
}
