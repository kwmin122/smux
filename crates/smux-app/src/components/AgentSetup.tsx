import { useState, useEffect } from 'react'

interface AgentInfo {
  name: string
  installed: boolean
  path: string | null
}

const INSTALL_GUIDES: Record<string, { cmd: string; note: string }> = {
  claude: {
    cmd: 'curl -fsSL https://claude.ai/install.sh | bash',
    note: 'Claude Pro ($20/월) 이상이면 API 키 없이 브라우저 로그인으로 사용. 설치 후 claude 실행하면 자동 로그인.',
  },
  codex: {
    cmd: 'npm install -g @openai/codex',
    note: 'ChatGPT Plus ($20/월) 이상이면 브라우저 로그인으로 사용. Pro ($200/월)면 $50 크레딧 포함.',
  },
  gemini: {
    cmd: 'npm install -g @google/gemini-cli',
    note: '무료! Google 계정만 있으면 됨. 브라우저 로그인으로 사용.',
  },
}

declare global {
  interface Window {
    __TAURI_INTERNALS__?: unknown
  }
}

const isTauri = !!window.__TAURI_INTERNALS__

interface AgentSetupProps {
  onReady: (planner: string, verifier: string) => void
  onSkip: () => void
}

/**
 * Shown before AI ping-pong starts.
 * Detects which agents are installed and guides the user through setup.
 * If only one agent is available, uses it for both planner and verifier.
 */
export function AgentSetup({ onReady, onSkip }: AgentSetupProps) {
  const [agents, setAgents] = useState<AgentInfo[]>([])
  const [loading, setLoading] = useState(true)
  const [planner, setPlanner] = useState('claude')
  const [verifier, setVerifier] = useState('codex')

  useEffect(() => {
    if (!isTauri) {
      setLoading(false)
      return
    }
    import('@tauri-apps/api/core').then(({ invoke }) => {
      invoke<AgentInfo[]>('detect_agents')
        .then(result => {
          setAgents(result)
          // Auto-select based on what's installed
          const installed = result.filter(a => a.installed).map(a => a.name)
          if (installed.length >= 2) {
            setPlanner(installed[0])
            setVerifier(installed[1])
          } else if (installed.length === 1) {
            // Use same agent for both roles
            setPlanner(installed[0])
            setVerifier(installed[0])
          }
          setLoading(false)
        })
        .catch(() => setLoading(false))
    })
  }, [])

  const installed = agents.filter(a => a.installed)
  const notInstalled = agents.filter(a => !a.installed)
  const canStart = installed.length >= 1

  if (loading) {
    return (
      <div className="p-4 font-mono text-[11px] text-outline text-center">
        Detecting installed AI agents...
      </div>
    )
  }

  return (
    <div className="p-4 space-y-4 max-w-md">
      <h3 className="font-mono text-[12px] font-bold text-on-surface">AI Agent Setup</h3>

      {/* Installed agents */}
      {installed.length > 0 && (
        <div className="space-y-1">
          <div className="font-mono text-[9px] uppercase tracking-widest text-secondary">Installed</div>
          {installed.map(a => (
            <div key={a.name} className="flex items-center gap-2 px-2 py-1 bg-secondary/5 rounded">
              <span className="w-1.5 h-1.5 rounded-full bg-secondary" />
              <span className="font-mono text-[11px] text-on-surface">{a.name}</span>
              <span className="font-mono text-[8px] text-outline truncate">{a.path}</span>
            </div>
          ))}
        </div>
      )}

      {/* Not installed */}
      {notInstalled.length > 0 && (
        <div className="space-y-1">
          <div className="font-mono text-[9px] uppercase tracking-widest text-outline">Not Installed</div>
          {notInstalled.map(a => {
            const guide = INSTALL_GUIDES[a.name]
            return (
              <div key={a.name} className="px-2 py-2 bg-surface-container rounded space-y-1">
                <div className="flex items-center gap-2">
                  <span className="w-1.5 h-1.5 rounded-full bg-outline" />
                  <span className="font-mono text-[11px] text-on-surface-variant">{a.name}</span>
                </div>
                {guide && (
                  <>
                    <code className="block font-mono text-[10px] text-primary bg-surface-container-lowest px-2 py-1 rounded select-all">
                      {guide.cmd}
                    </code>
                    <div className="font-mono text-[8px] text-outline">{guide.note}</div>
                  </>
                )}
              </div>
            )
          })}
        </div>
      )}

      {/* Role selection */}
      {canStart && (
        <div className="space-y-2 pt-2 border-t border-outline-variant/20">
          <div className="flex gap-2">
            <div className="flex-1">
              <label className="font-mono text-[9px] uppercase tracking-widest text-outline block mb-1">Planner</label>
              <select
                value={planner}
                onChange={e => setPlanner(e.target.value)}
                className="w-full h-7 bg-surface-container-lowest border border-outline-variant/30 rounded px-2 font-mono text-[11px] text-on-surface-variant outline-none"
              >
                {installed.map(a => <option key={a.name} value={a.name}>{a.name}</option>)}
              </select>
            </div>
            <div className="flex-1">
              <label className="font-mono text-[9px] uppercase tracking-widest text-outline block mb-1">Verifier</label>
              <select
                value={verifier}
                onChange={e => setVerifier(e.target.value)}
                className="w-full h-7 bg-surface-container-lowest border border-outline-variant/30 rounded px-2 font-mono text-[11px] text-on-surface-variant outline-none"
              >
                {installed.map(a => <option key={a.name} value={a.name}>{a.name}</option>)}
              </select>
            </div>
          </div>
          {planner === verifier && (
            <div className="font-mono text-[9px] text-tertiary">
              Same agent for both roles — will still verify but no cross-model benefit.
            </div>
          )}
          <button
            onClick={() => onReady(planner, verifier)}
            className="w-full h-8 bg-secondary text-on-primary font-mono text-[11px] font-bold rounded hover:opacity-90"
          >
            Start AI Ping-Pong
          </button>
        </div>
      )}

      {/* No agents installed */}
      {!canStart && (
        <div className="p-3 bg-error/10 border border-error/20 rounded">
          <div className="font-mono text-[11px] text-error font-bold mb-1">No AI agents found</div>
          <div className="font-mono text-[10px] text-on-surface-variant">
            Install at least one agent (claude recommended) to use AI Ping-Pong.
          </div>
        </div>
      )}

      <button
        onClick={onSkip}
        className="w-full h-7 border border-outline-variant/30 font-mono text-[10px] text-outline rounded hover:text-on-surface"
      >
        Skip — use terminals without AI
      </button>
    </div>
  )
}
