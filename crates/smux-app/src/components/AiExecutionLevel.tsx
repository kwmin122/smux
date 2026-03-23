import { useState } from 'react'

export type ExecutionLevel = 'disabled' | 'allowlist' | 'auto' | 'turbo'

const LEVEL_INFO: Record<ExecutionLevel, { label: string; color: string; description: string }> = {
  disabled: { label: 'OFF', color: 'text-outline', description: 'AI suggests only, no auto-execution' },
  allowlist: { label: 'SAFE', color: 'text-tertiary', description: 'Only allowed commands auto-execute' },
  auto: { label: 'AUTO', color: 'text-secondary', description: 'AI judges safety before executing' },
  turbo: { label: 'TURBO', color: 'text-error', description: 'All commands auto-execute (YOLO)' },
}

const LEVELS: ExecutionLevel[] = ['disabled', 'allowlist', 'auto', 'turbo']

interface AiExecutionLevelProps {
  level: ExecutionLevel
  onChange: (level: ExecutionLevel) => void
  compact?: boolean
}

/**
 * Displays and allows switching between AI auto-execution levels.
 * Windsurf-inspired 4-tier model.
 */
export function AiExecutionLevel({ level, onChange, compact = false }: AiExecutionLevelProps) {
  const [showPicker, setShowPicker] = useState(false)
  const info = LEVEL_INFO[level]

  if (compact) {
    return (
      <div className="relative">
        <button
          onClick={() => setShowPicker(!showPicker)}
          className={`font-mono text-[9px] font-bold px-2 py-0.5 rounded border border-outline-variant/30 ${info.color} hover:opacity-80 transition-colors cursor-pointer`}
          aria-label={`AI execution level: ${info.label}`}
        >
          {info.label}
        </button>
        {showPicker && (
          <>
            <div className="fixed inset-0 z-[90]" onClick={() => setShowPicker(false)} />
            <div className="absolute top-full right-0 mt-1 z-[91] bg-surface-container border border-outline-variant/30 rounded shadow-lg py-1 min-w-[180px]">
              {LEVELS.map(l => {
                const li = LEVEL_INFO[l]
                return (
                  <button
                    key={l}
                    onClick={() => { onChange(l); setShowPicker(false) }}
                    className={`w-full text-left px-3 py-1.5 flex items-center gap-2 hover:bg-surface-container-high transition-colors ${
                      l === level ? 'bg-primary/10' : ''
                    }`}
                  >
                    <span className={`font-mono text-[10px] font-bold ${li.color}`}>{li.label}</span>
                    <span className="font-mono text-[9px] text-outline">{li.description}</span>
                  </button>
                )
              })}
            </div>
          </>
        )}
      </div>
    )
  }

  return (
    <div className="space-y-1">
      <label className="font-mono text-[10px] uppercase tracking-widest text-outline block">
        AI Execution Level
      </label>
      <div className="flex gap-1">
        {LEVELS.map(l => {
          const li = LEVEL_INFO[l]
          return (
            <button
              key={l}
              onClick={() => onChange(l)}
              className={`flex-1 py-1.5 rounded font-mono text-[10px] border transition-colors ${
                l === level
                  ? `${li.color} border-current bg-current/10`
                  : 'border-outline-variant/30 text-outline hover:border-primary'
              }`}
            >
              {li.label}
            </button>
          )
        })}
      </div>
      <div className="font-mono text-[9px] text-outline">{info.description}</div>
    </div>
  )
}

/**
 * Check if a command is allowed to auto-execute based on the current level.
 */
// TODO: Wire this into the orchestrator's command execution path
export function isCommandAllowed(
  command: string,
  level: ExecutionLevel,
  allowList: string[],
  denyList: string[]
): boolean {
  // Deny list always blocks
  for (const denied of denyList) {
    if (command.includes(denied)) return false
  }

  switch (level) {
    case 'disabled':
      return false
    case 'turbo':
      return true
    case 'allowlist': {
      // Extract the binary name (first token) for precise matching
      const binary = command.trim().split(/\s+/)[0]
      const binaryName = binary.split('/').pop() || binary
      return allowList.some(allowed => binaryName === allowed)
    }
    case 'auto':
      // In auto mode, block obviously dangerous commands
      const dangerous = ['rm -rf /', 'sudo rm', 'mkfs', 'dd if=', ':(){:|:&};:', 'shutdown', 'reboot']
      return !dangerous.some(d => command.includes(d))
    default:
      return false
  }
}
