import { useMemo } from 'react'
import type { CommandRecord } from '../hooks/useShellIntegration'

interface StickyScrollProps {
  commands: CommandRecord[]
  viewportTopLine: number
  visible: boolean
}

function formatElapsed(startTime: number, endTime: number | null): string | null {
  if (endTime === null) return null
  const seconds = (endTime - startTime) / 1000
  return `${seconds.toFixed(1)}s`
}

export function StickyScroll({ commands, viewportTopLine, visible }: StickyScrollProps) {
  const activeCommand = useMemo(() => {
    if (!visible) return null

    for (let i = commands.length - 1; i >= 0; i--) {
      const cmd = commands[i]
      if (
        cmd.startLine <= viewportTopLine &&
        (cmd.endLine === null || cmd.endLine >= viewportTopLine)
      ) {
        // Only show if the command's prompt line is scrolled above the viewport
        if (viewportTopLine > cmd.startLine) {
          return cmd
        }
        return null
      }
    }
    return null
  }, [commands, viewportTopLine, visible])

  if (!activeCommand) return null

  const elapsed = formatElapsed(activeCommand.startTime, activeCommand.endTime)
  const exitCode = activeCommand.exitCode
  const hasExitCode = exitCode !== null

  return (
    <div
      className="absolute top-0 left-0 right-0 z-20 flex items-center justify-between px-3 border-b border-outline-variant/20 backdrop-blur-sm select-none"
      style={{
        height: 24,
        backgroundColor: 'color-mix(in srgb, var(--surface-container) 90%, transparent)',
      }}
    >
      {/* Left: command text */}
      <span className="font-mono text-[11px] text-on-surface-variant truncate min-w-0">
        <span className="text-primary/70 mr-1">$</span>
        {activeCommand.command}
      </span>

      {/* Right: exit code badge + elapsed time */}
      <div className="flex items-center gap-2 shrink-0 ml-3">
        {hasExitCode && (
          <span
            className={`font-mono text-[10px] font-medium px-1.5 rounded-sm ${
              exitCode === 0
                ? 'bg-green-500/15 text-green-400'
                : 'bg-red-500/15 text-red-400'
            }`}
          >
            {exitCode}
          </span>
        )}
        {elapsed && (
          <span className="font-mono text-[10px] text-outline">
            {elapsed}
          </span>
        )}
      </div>
    </div>
  )
}
