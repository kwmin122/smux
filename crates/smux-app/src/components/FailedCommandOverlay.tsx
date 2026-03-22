import { useState } from 'react'
import type { CommandRecord } from '../hooks/useShellIntegration'

interface FailedCommandOverlayProps {
  command: CommandRecord | null
  onFixWithAi: (command: CommandRecord) => void
  onDismiss: () => void
}

/**
 * Overlay shown when a command fails (non-zero exit code).
 * Offers "Fix with AI" button to send the failed command + output to AI for analysis.
 */
export function FailedCommandOverlay({ command, onFixWithAi, onDismiss }: FailedCommandOverlayProps) {
  const [isAnalyzing, setIsAnalyzing] = useState(false)

  if (!command || command.status !== 'error') return null

  return (
    <div className="absolute bottom-2 right-2 z-30 bg-surface-container border border-error/30 rounded-lg shadow-lg max-w-[320px] overflow-hidden">
      <div className="px-3 py-2 bg-error/10 border-b border-error/20 flex items-center gap-2">
        <span className="material-symbols-outlined text-[14px] text-error">error</span>
        <span className="font-mono text-[10px] font-bold text-error uppercase tracking-wider">
          Command Failed (exit {command.exitCode})
        </span>
        <button
          onClick={onDismiss}
          className="ml-auto material-symbols-outlined text-[14px] text-outline hover:text-on-surface cursor-pointer"
        >
          close
        </button>
      </div>
      <div className="px-3 py-2">
        <div className="font-mono text-[11px] text-on-surface-variant truncate mb-2">
          $ {command.command}
        </div>
        <div className="flex gap-2">
          <button
            onClick={() => {
              setIsAnalyzing(true)
              onFixWithAi(command)
              setTimeout(() => setIsAnalyzing(false), 3000)
            }}
            disabled={isAnalyzing}
            className="flex-1 h-7 bg-primary text-on-primary font-mono text-[10px] rounded hover:opacity-90 disabled:opacity-50 flex items-center justify-center gap-1"
          >
            {isAnalyzing ? (
              <>
                <span className="w-3 h-3 border-2 border-on-primary/30 border-t-on-primary rounded-full animate-spin" />
                Analyzing...
              </>
            ) : (
              <>
                <span className="material-symbols-outlined text-[14px]">auto_fix_high</span>
                Fix with AI
              </>
            )}
          </button>
          <button
            onClick={onDismiss}
            className="h-7 px-3 border border-outline-variant/30 font-mono text-[10px] text-outline rounded hover:text-on-surface"
          >
            Dismiss
          </button>
        </div>
      </div>
    </div>
  )
}
