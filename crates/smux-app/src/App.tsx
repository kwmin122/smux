import { useState, useEffect } from 'react'

declare global {
  interface Window {
    __TAURI_INTERNALS__?: unknown
  }
}

const isTauri = !!window.__TAURI_INTERNALS__

async function tauriPing(): Promise<string> {
  if (!isTauri) return 'pong (browser mode)'
  const { invoke } = await import('@tauri-apps/api/core')
  return invoke<string>('ping')
}

function App() {
  const [pingResult, setPingResult] = useState<string | null>(null)
  const [theme, setTheme] = useState<string>('deep-navy')

  useEffect(() => {
    tauriPing().then(setPingResult)
  }, [])

  function cycleTheme() {
    const themes = ['deep-navy', 'amber', 'forest-green']
    const next = themes[(themes.indexOf(theme) + 1) % themes.length]
    setTheme(next)
    document.documentElement.setAttribute('data-theme', next)
  }

  return (
    <div className="flex flex-col h-screen">
      {/* Top Bar */}
      <header className="h-12 bg-background flex items-center justify-between px-4 border-b border-outline-variant/20 z-50 shrink-0">
        <div className="flex items-center gap-4">
          <span className="font-headline font-bold text-xl tracking-[-0.02em] text-on-surface">
            SMUX
          </span>
          <span className="text-xs uppercase tracking-widest font-mono text-primary border-b-2 border-primary pb-1">
            Dashboard
          </span>
        </div>
        <div className="flex items-center gap-3">
          <button
            onClick={cycleTheme}
            className="text-xs font-mono uppercase tracking-wider text-outline hover:text-primary transition-colors px-2 py-1"
          >
            {theme}
          </button>
          <span className="material-symbols-outlined text-[18px] text-outline hover:text-primary transition-colors cursor-pointer">
            settings
          </span>
        </div>
      </header>

      {/* Main Content */}
      <div className="flex flex-1 overflow-hidden">
        {/* Sidebar */}
        <nav className="w-64 bg-surface-container-low flex flex-col shrink-0 border-r border-outline-variant/20 z-40">
          <div className="px-4 py-3 border-b border-outline-variant/20">
            <div className="text-xs font-mono uppercase tracking-widest text-outline">
              Sessions
            </div>
          </div>
          <div className="flex-1 overflow-y-auto py-2">
            {['Sessions', 'Clusters', 'Network', 'Logs', 'Vault'].map((item, i) => (
              <div
                key={item}
                className={`flex items-center gap-3 px-4 py-3 text-sm cursor-pointer transition-all ${
                  i === 0
                    ? 'bg-primary/10 text-primary border-l-2 border-primary'
                    : 'text-on-surface-variant hover:bg-surface-container-high border-l-2 border-transparent'
                }`}
              >
                <span className="material-symbols-outlined text-[18px]">
                  {['terminal', 'hub', 'lan', 'event_note', 'lock'][i]}
                </span>
                {item}
              </div>
            ))}
          </div>
        </nav>

        {/* Terminal Panels */}
        <main className="flex-1 flex gap-1 p-1 overflow-hidden">
          {/* Planner Panel */}
          <section className="flex-1 flex flex-col bg-surface-container-lowest border border-outline-variant/20 rounded-[var(--radius-default)]">
            <div className="h-8 bg-surface-container-high px-3 flex items-center border-b border-outline-variant/20">
              <span className="font-mono text-[10px] font-bold uppercase tracking-widest text-on-surface-variant">
                Planner
              </span>
              <span className="ml-2 w-2 h-2 rounded-full bg-secondary animate-pulse" />
            </div>
            <div className="flex-1 p-4 font-mono text-[13px] leading-relaxed overflow-y-auto text-on-surface-variant">
              <div className="text-outline text-[11px]">{'>'} smux v0.3.0 — Tauri UI</div>
              <div className="mt-2 text-primary">IPC bridge: {pingResult ?? 'connecting...'}</div>
              <div className="mt-1 text-secondary">Theme: {theme}</div>
              <div className="mt-4">
                <span className="inline-block w-2 h-[1.2em] bg-primary shadow-[0_0_4px_var(--primary)] animate-[blink_1s_step-end_infinite]" />
              </div>
            </div>
          </section>

          {/* Verifier Panel */}
          <section className="flex-1 flex flex-col bg-surface-container-lowest border border-outline-variant/20 rounded-[var(--radius-default)]">
            <div className="h-8 bg-surface-container-high px-3 flex items-center border-b border-outline-variant/20">
              <span className="font-mono text-[10px] font-bold uppercase tracking-widest text-on-surface-variant">
                Verifier
              </span>
              <span className="ml-2 w-2 h-2 rounded-full bg-tertiary" />
            </div>
            <div className="flex-1 p-4 font-mono text-[13px] leading-relaxed overflow-y-auto text-on-surface-variant">
              <div className="text-outline text-[11px]">{'>'} verification-gate idle</div>
              <div className="mt-2 text-on-surface-variant">Awaiting session...</div>
            </div>
          </section>
        </main>
      </div>

      {/* Status Bar */}
      <footer className="h-8 bg-surface-container-high flex items-center justify-end gap-6 px-4 border-t border-outline-variant/20 z-50 shrink-0">
        <span className="font-mono text-[10px] uppercase text-outline flex items-center gap-1">
          <span className="w-2 h-2 rounded-full bg-secondary" /> CPU: 12%
        </span>
        <span className="font-mono text-[10px] uppercase text-outline">
          Latency: 14ms
        </span>
        <span className="font-mono text-[10px] uppercase text-outline">
          Git: main
        </span>
        <span className="font-mono text-[10px] uppercase text-outline flex items-center gap-1">
          <span className="w-2 h-2 rounded-full bg-primary" /> Uptime: 42m
        </span>
      </footer>
    </div>
  )
}

export default App
