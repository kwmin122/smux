import { useState, useEffect, useRef, useCallback } from 'react'
import { TerminalPanel, type TerminalPanelHandle } from './components/TerminalPanel'
import { MissionControl, type RoundEntry, type EventLogEntry, type CrossVerifyState } from './components/MissionControl'
import { BrowserPanel } from './components/BrowserPanel'
import { WelcomeView } from './components/WelcomeView'

declare global {
  interface Window {
    __TAURI_INTERNALS__?: unknown
  }
}

const isTauri = !!window.__TAURI_INTERNALS__

type AppMode = 'focus' | 'control'
type LayoutPreset = 'center' | 'right' | 'bottom'
type FullscreenPanel = 'planner' | 'verifier' | null

interface DaemonEvent {
  kind: string
  role?: string
  content?: string
  round?: number
  verdict_summary?: string
  summary?: string
  message?: string
  // CrossVerifyResult fields
  individual?: Array<{ verifier: string; verdict: string; confidence: number; reason: string }>
  final_verdict?: string
  strategy?: string
  agreement_ratio?: number
}

function App() {
  const [theme, setTheme] = useState<string>('deep-navy')
  const [mode, setMode] = useState<AppMode>('focus')
  const [activeSession, setActiveSession] = useState<string | null>(null)
  const [sessionTask, setSessionTask] = useState<string>('')
  const [connected, setConnected] = useState(false)
  const [currentRound, setCurrentRound] = useState(0)
  const [maxRounds, setMaxRounds] = useState(10)
  const [dividerPos, setDividerPos] = useState(50)
  const [isDragging, setIsDragging] = useState(false)
  const [rounds, setRounds] = useState<RoundEntry[]>([])
  const [eventLog, setEventLog] = useState<EventLogEntry[]>([])
  const [health, setHealth] = useState({ planner: 100, verifier: 100 })
  const [safetyOk, setSafetyOk] = useState(true)
  const [showBrowser, setShowBrowser] = useState(false)
  const [crossVerify, setCrossVerify] = useState<CrossVerifyState | null>(null)
  const [layout, setLayout] = useState<LayoutPreset>('center')
  const [panelOrder, setPanelOrder] = useState<['planner' | 'verifier', 'planner' | 'verifier']>(['planner', 'verifier'])
  const [gitBranch, setGitBranch] = useState('—')
  const [gitFilesChanged, setGitFilesChanged] = useState(0)
  const [showNewSession, setShowNewSession] = useState(false)
  const [inputTask, setInputTask] = useState('')
  const [inputPlanner, setInputPlanner] = useState('claude')
  const [inputVerifier, setInputVerifier] = useState('codex')
  const [inputVerifiers, setInputVerifiers] = useState('')
  const [inputConsensus, setInputConsensus] = useState('majority')
  const [inputMaxRounds, setInputMaxRounds] = useState(10)
  const [fullscreen, setFullscreen] = useState<FullscreenPanel>(null)
  const [daemonRunning, setDaemonRunning] = useState(false)
  const [terminalMode, setTerminalMode] = useState<'idle' | 'terminal' | 'ai-session'>('terminal')

  const plannerRef = useRef<TerminalPanelHandle>(null)
  const verifierRef = useRef<TerminalPanelHandle>(null)
  const mainRef = useRef<HTMLElement>(null)

  // Load saved layout on mount
  useEffect(() => {
    try {
      const saved = localStorage.getItem('smux-layout')
      if (saved) {
        const parsed = JSON.parse(saved)
        if (parsed.layout) setLayout(parsed.layout)
        if (typeof parsed.dividerPos === 'number') setDividerPos(parsed.dividerPos)
        if (typeof parsed.showBrowser === 'boolean') setShowBrowser(parsed.showBrowser)
        if (Array.isArray(parsed.panelOrder) && parsed.panelOrder.length === 2) setPanelOrder(parsed.panelOrder)
      }
    } catch { /* ignore */ }
    // Request notification permission on first load
    if ('Notification' in window && Notification.permission === 'default') {
      Notification.requestPermission()
    }
    // Fetch git info and check daemon
    if (isTauri) {
      fetchGitInfo()
      checkDaemon()
      const gitInterval = setInterval(fetchGitInfo, 15000)
      const daemonInterval = setInterval(checkDaemon, 5000)
      return () => {
        clearInterval(gitInterval)
        clearInterval(daemonInterval)
      }
    }
  }, [])

  function notify(title: string, body: string) {
    if ('Notification' in window && Notification.permission === 'granted') {
      new Notification(title, { body, icon: '/smux-icon.png' })
    } else if ('Notification' in window && Notification.permission !== 'denied') {
      Notification.requestPermission()
    }
  }

  async function fetchGitInfo() {
    try {
      const { invoke } = await import('@tauri-apps/api/core')
      const info = await invoke<{ branch: string; files_changed: number }>('get_git_info')
      setGitBranch(info.branch)
      setGitFilesChanged(info.files_changed)
    } catch { /* ignore */ }
  }

  async function checkDaemon() {
    try {
      const { invoke } = await import('@tauri-apps/api/core')
      const sessions = await invoke<unknown[]>('list_sessions')
      setDaemonRunning(true)
      void sessions // used for connection check
    } catch {
      setDaemonRunning(false)
    }
  }

  function addLogEntry(kind: string, message: string) {
    const ts = new Date().toLocaleTimeString('en-US', { hour12: false, hour: '2-digit', minute: '2-digit', second: '2-digit' })
    setEventLog(prev => [...prev.slice(-49), { timestamp: ts, kind, message }])
  }

  // Listen for daemon events
  useEffect(() => {
    if (!isTauri) return

    let unlisten: (() => void) | null = null

    ;(async () => {
      const { listen } = await import('@tauri-apps/api/event')
      unlisten = await listen<DaemonEvent>('daemon-event', (event) => {
        const data = event.payload
        switch (data.kind) {
          case 'agent_output':
            if (data.role === 'planner') {
              plannerRef.current?.write(data.content ?? '')
            } else if (data.role === 'verifier') {
              verifierRef.current?.write(data.content ?? '')
            } else if (data.role?.startsWith('health:')) {
              const agent = data.role.split(':')[1]
              const terminal = agent === 'planner' ? plannerRef : verifierRef
              terminal.current?.writeln(
                `\x1b[33m[health] ${data.content}\x1b[0m`
              )
              // Parse health percentage if present (e.g. "healthy:85")
              const match = data.content?.match(/(\d+)/)
              if (match) {
                const pct = parseInt(match[1], 10)
                setHealth(prev => ({
                  ...prev,
                  [agent]: Math.min(100, Math.max(0, pct)),
                }))
              }
              addLogEntry('health', `${agent}: ${data.content}`)
            } else if (data.role?.startsWith('safety:')) {
              const severity = data.role.split(':')[1]
              const line = `\x1b[31m[safety:${severity}] ${data.content}\x1b[0m`
              plannerRef.current?.writeln(line)
              verifierRef.current?.writeln(line)
              if (severity === 'high' || severity === 'critical') {
                setSafetyOk(false)
              }
              addLogEntry('safety', `[${severity}] ${data.content}`)
            }
            break
          case 'round_complete': {
            const round = data.round ?? 0
            setCurrentRound(round)
            const verdictLower = (data.verdict_summary ?? '').toLowerCase()
            const verdict: RoundEntry['verdict'] = verdictLower.includes('approved')
              ? 'approved'
              : verdictLower.includes('rejected')
              ? 'rejected'
              : verdictLower.includes('needs')
              ? 'needs_info'
              : 'pending'
            setRounds(prev => [
              ...prev.filter(r => r.round !== round),
              { round, verdict, summary: data.verdict_summary },
            ])
            plannerRef.current?.writeln(
              `\x1b[36m\n━━━ Round ${round} complete: ${data.verdict_summary} ━━━\x1b[0m\n`
            )
            verifierRef.current?.writeln(
              `\x1b[36m\n━━━ Round ${round} complete: ${data.verdict_summary} ━━━\x1b[0m\n`
            )
            addLogEntry('round', `Round ${round}: ${data.verdict_summary}`)
            if (document.hidden) {
              notify(`Round ${round} complete`, data.verdict_summary ?? '')
            }
            break
          }
          case 'session_complete':
            plannerRef.current?.writeln(
              `\x1b[32m\n✓ Session complete: ${data.summary}\x1b[0m`
            )
            verifierRef.current?.writeln(
              `\x1b[32m\n✓ Session complete: ${data.summary}\x1b[0m`
            )
            setConnected(false)
            addLogEntry('session', `Complete: ${data.summary}`)
            notify('Session complete', data.summary ?? '')
            break
          case 'cross_verify_result':
            if (data.individual && data.round != null) {
              setCrossVerify({
                round: data.round,
                individual: data.individual.map(v => ({
                  verifier: v.verifier,
                  verdict: v.verdict.toLowerCase().includes('approved')
                    ? 'approved' as const
                    : v.verdict.toLowerCase().includes('rejected')
                    ? 'rejected' as const
                    : 'needs_info' as const,
                  confidence: v.confidence,
                  reason: v.reason,
                })),
                finalVerdict: data.final_verdict ?? 'unknown',
                strategy: data.strategy ?? 'majority',
                agreementRatio: data.agreement_ratio ?? 0,
              })
              addLogEntry('cross-verify', `R${data.round}: ${data.final_verdict} (${data.strategy}, ${Math.round((data.agreement_ratio ?? 0) * 100)}%)`)
            }
            break
          case 'error':
            plannerRef.current?.writeln(
              `\x1b[31m[error] ${data.message}\x1b[0m`
            )
            addLogEntry('error', data.message ?? 'unknown error')
            break
        }
      })
    })()

    return () => {
      unlisten?.()
    }
  }, [])

  // Keyboard shortcuts
  useEffect(() => {
    function handleKeyDown(e: KeyboardEvent) {
      if (e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement) return

      if (e.key === 'Tab') {
        e.preventDefault()
        setMode(m => m === 'focus' ? 'control' : 'focus')
      }
      if (e.key === 'b' && (e.metaKey || e.ctrlKey)) {
        e.preventDefault()
        setShowBrowser(prev => !prev)
      }
      if ((e.metaKey || e.ctrlKey) && e.key === '1') {
        e.preventDefault()
        setLayout('center')
        setFullscreen(null)
      }
      if ((e.metaKey || e.ctrlKey) && e.key === '2') {
        e.preventDefault()
        setLayout('right')
        setFullscreen(null)
      }
      if ((e.metaKey || e.ctrlKey) && e.key === '3') {
        e.preventDefault()
        setLayout('bottom')
        setFullscreen(null)
      }
      if (e.key === 'f' && (e.metaKey || e.ctrlKey)) {
        e.preventDefault()
        setFullscreen(prev => prev ? null : 'planner')
      }
      if (e.key === 's' && (e.metaKey || e.ctrlKey) && !e.shiftKey) {
        e.preventDefault()
        localStorage.setItem('smux-layout', JSON.stringify({ layout, dividerPos, showBrowser, panelOrder }))
        plannerRef.current?.writeln('\x1b[90m[layout saved]\x1b[0m')
      }
      // Cmd+Shift+S: swap panel positions
      if (e.key === 'S' && (e.metaKey || e.ctrlKey) && e.shiftKey) {
        e.preventDefault()
        setPanelOrder(prev => [prev[1], prev[0]])
      }
      // Non-modifier shortcuts (only when no Cmd/Ctrl)
      if (!e.metaKey && !e.ctrlKey && !e.altKey) {
        if (e.key === 'q') {
          if (isTauri) {
            import('@tauri-apps/api/core').then(({ invoke }) => invoke('ping')) // keep alive
          }
        }
      }
    }
    window.addEventListener('keydown', handleKeyDown)
    return () => window.removeEventListener('keydown', handleKeyDown)
  }, [layout, dividerPos, showBrowser, panelOrder])

  // Divider drag handling
  useEffect(() => {
    if (!isDragging) return

    function handleMouseMove(e: MouseEvent) {
      if (!mainRef.current) return
      const rect = mainRef.current.getBoundingClientRect()
      const pos = layout === 'bottom'
        ? ((e.clientY - rect.top) / rect.height) * 100
        : ((e.clientX - rect.left) / rect.width) * 100
      setDividerPos(Math.max(20, Math.min(80, pos)))
    }

    function handleMouseUp() {
      setIsDragging(false)
    }

    document.addEventListener('mousemove', handleMouseMove)
    document.addEventListener('mouseup', handleMouseUp)
    document.body.style.cursor = layout === 'bottom' ? 'row-resize' : 'col-resize'
    document.body.style.userSelect = 'none'

    return () => {
      document.removeEventListener('mousemove', handleMouseMove)
      document.removeEventListener('mouseup', handleMouseUp)
      document.body.style.cursor = ''
      document.body.style.userSelect = ''
    }
  }, [isDragging, layout])

  const handleStartSession = useCallback(async () => {
    if (!isTauri) {
      plannerRef.current?.writeln('\x1b[33m[browser mode] daemon connection not available\x1b[0m')
      return
    }
    if (!inputTask.trim()) {
      plannerRef.current?.writeln('\x1b[33m[error] task description required\x1b[0m')
      return
    }

    try {
      const { invoke } = await import('@tauri-apps/api/core')
      plannerRef.current?.writeln('\x1b[90mConnecting to daemon...\x1b[0m')
      const verifiersList = inputVerifiers.trim()
        ? inputVerifiers.split(',').map(s => s.trim()).filter(Boolean)
        : []
      const sessionId = await invoke<string>('start_session', {
        args: {
          planner: inputPlanner,
          verifier: inputVerifier,
          task: inputTask,
          maxRounds: inputMaxRounds,
          verifiers: verifiersList,
          consensus: inputConsensus,
        },
      })
      setActiveSession(sessionId)
      setSessionTask(inputTask)
      setMaxRounds(inputMaxRounds)
      setConnected(true)
      setCurrentRound(1)
      setRounds([])
      setEventLog([])
      setHealth({ planner: 100, verifier: 100 })
      setSafetyOk(true)
      setCrossVerify(null)
      setShowNewSession(false)
      plannerRef.current?.writeln(`\x1b[32mSession created: ${sessionId}\x1b[0m`)
      plannerRef.current?.writeln(`\x1b[90mplanner=${inputPlanner} verifier=${inputVerifier}${verifiersList.length ? ` verifiers=${verifiersList.join(',')}` : ''} consensus=${inputConsensus}\x1b[0m\n`)
      verifierRef.current?.writeln(`\x1b[32mAttached to session: ${sessionId}\x1b[0m\n`)
      addLogEntry('session', `Created: ${sessionId}`)
    } catch (e) {
      plannerRef.current?.writeln(`\x1b[31mFailed: ${e}\x1b[0m`)
    }
  }, [inputTask, inputPlanner, inputVerifier, inputVerifiers, inputConsensus, inputMaxRounds])

  function cycleTheme() {
    const themes = ['deep-navy', 'amber', 'forest-green']
    const next = themes[(themes.indexOf(theme) + 1) % themes.length]
    setTheme(next)
    document.documentElement.setAttribute('data-theme', next)
  }

  const statusBarBg = mode === 'focus'
    ? 'bg-primary/20 border-primary/30'
    : 'bg-tertiary/20 border-tertiary/30'

  // Layout sizing
  const isBottom = layout === 'bottom' && mode === 'focus'
  const plannerPct = mode === 'control' ? 35
    : layout === 'right' ? 65
    : dividerPos
  const verifierPct = mode === 'control' ? 35
    : layout === 'right' ? 35
    : 100 - dividerPos

  return (
    <div className="flex flex-col h-screen">
      {/* Top Status Bar */}
      <header className={`h-10 flex items-center justify-between px-4 border-b z-50 shrink-0 ${statusBarBg}`}>
        <div className="flex items-center gap-3">
          <span className="font-headline font-bold text-sm tracking-[-0.02em] text-on-surface">
            smux
          </span>
          <span className={`font-mono text-[10px] font-bold uppercase tracking-widest px-2 py-0.5 rounded-sm ${
            mode === 'focus' ? 'bg-primary/20 text-primary' : 'bg-tertiary/20 text-tertiary'
          }`}>
            {mode}
          </span>
          {activeSession && (
            <>
              <span className="text-outline">|</span>
              <span className="font-mono text-[11px] text-on-surface-variant truncate max-w-[300px]">
                {sessionTask}
              </span>
              <span className="font-mono text-[11px] text-secondary">
                R{currentRound}/{maxRounds}
              </span>
            </>
          )}
        </div>
        <div className="flex items-center gap-3">
          {connected && (
            <span className="font-mono text-[10px] text-secondary flex items-center gap-1">
              <span className="w-1.5 h-1.5 rounded-full bg-secondary animate-pulse" />
              live
            </span>
          )}
          <button
            onClick={cycleTheme}
            className="text-[10px] font-mono uppercase tracking-wider text-outline hover:text-primary transition-colors px-1.5 py-0.5"
          >
            {theme}
          </button>
          <span className="material-symbols-outlined text-[16px] text-outline hover:text-primary transition-colors cursor-pointer">
            settings
          </span>
        </div>
      </header>

      {/* Main Content */}
      <div className="flex flex-1 overflow-hidden">
        {/* Sidebar */}
        <nav className="w-56 bg-surface-container-low flex flex-col shrink-0 border-r border-outline-variant/20 z-40">
          <div className="px-3 py-2 border-b border-outline-variant/20">
            <div className="text-[10px] font-mono uppercase tracking-widest text-outline">
              Sessions
            </div>
          </div>
          <div className="flex-1 overflow-y-auto py-1">
            {activeSession && (
              <div className="flex items-center gap-2 px-3 py-2 text-sm bg-primary/10 text-primary border-l-2 border-primary">
                <span className="material-symbols-outlined text-[16px]">terminal</span>
                <div className="min-w-0">
                  <div className="font-mono text-[10px] truncate">{activeSession}</div>
                  <div className="text-[9px] text-on-surface-variant mt-0.5">
                    R{currentRound}/{maxRounds} {connected ? '● live' : '○ done'}
                  </div>
                </div>
              </div>
            )}
            {!activeSession && !showNewSession && (
              <button
                onClick={() => setShowNewSession(true)}
                className="flex items-center gap-2 px-3 py-2 text-sm text-on-surface-variant hover:bg-surface-container-high w-full text-left border-l-2 border-transparent"
              >
                <span className="material-symbols-outlined text-[16px]">add_circle</span>
                New Session
              </button>
            )}
            {showNewSession && !activeSession && (
              <div className="px-3 py-2 space-y-2">
                <input
                  type="text"
                  placeholder="Task description"
                  value={inputTask}
                  onChange={e => setInputTask(e.target.value)}
                  className="w-full h-7 bg-surface-container-lowest border border-outline-variant/30 rounded-sm px-2 font-mono text-[11px] text-on-surface-variant outline-none focus:border-primary"
                />
                <div className="flex gap-1">
                  <select
                    value={inputPlanner}
                    onChange={e => setInputPlanner(e.target.value)}
                    className="flex-1 h-6 bg-surface-container-lowest border border-outline-variant/30 rounded-sm px-1 font-mono text-[10px] text-on-surface-variant outline-none"
                  >
                    <option value="claude">claude</option>
                    <option value="codex">codex</option>
                    <option value="gemini">gemini</option>
                  </select>
                  <select
                    value={inputVerifier}
                    onChange={e => setInputVerifier(e.target.value)}
                    className="flex-1 h-6 bg-surface-container-lowest border border-outline-variant/30 rounded-sm px-1 font-mono text-[10px] text-on-surface-variant outline-none"
                  >
                    <option value="codex">codex</option>
                    <option value="claude">claude</option>
                    <option value="gemini">gemini</option>
                  </select>
                </div>
                <input
                  type="text"
                  placeholder="Extra verifiers (e.g. claude,gemini)"
                  value={inputVerifiers}
                  onChange={e => setInputVerifiers(e.target.value)}
                  className="w-full h-6 bg-surface-container-lowest border border-outline-variant/30 rounded-sm px-2 font-mono text-[10px] text-on-surface-variant outline-none focus:border-primary"
                />
                <div className="flex gap-1">
                  <select
                    value={inputConsensus}
                    onChange={e => setInputConsensus(e.target.value)}
                    className="flex-1 h-6 bg-surface-container-lowest border border-outline-variant/30 rounded-sm px-1 font-mono text-[10px] text-on-surface-variant outline-none"
                  >
                    <option value="majority">majority</option>
                    <option value="weighted">weighted</option>
                    <option value="unanimous">unanimous</option>
                    <option value="leader">leader</option>
                  </select>
                  <input
                    type="number"
                    min={1}
                    max={50}
                    value={inputMaxRounds}
                    onChange={e => setInputMaxRounds(Number(e.target.value))}
                    className="w-12 h-6 bg-surface-container-lowest border border-outline-variant/30 rounded-sm px-1 font-mono text-[10px] text-on-surface-variant outline-none text-center"
                    title="Max rounds"
                  />
                </div>
                <div className="flex gap-1">
                  <button
                    onClick={handleStartSession}
                    className="flex-1 h-6 bg-primary text-on-primary font-mono text-[10px] rounded-sm hover:opacity-90"
                  >
                    Start
                  </button>
                  <button
                    onClick={() => setShowNewSession(false)}
                    className="h-6 px-2 border border-outline-variant/30 font-mono text-[10px] text-outline rounded-sm hover:text-on-surface"
                  >
                    Cancel
                  </button>
                </div>
              </div>
            )}
          </div>
        </nav>

        {/* Terminal Panels with optional Mission Control + Browser */}
        <main ref={mainRef} className={`flex-1 flex p-1 overflow-hidden ${isBottom ? 'flex-col' : 'flex-row'}`}>
          {/* Terminal Mode: full-screen PTY shell */}
          {terminalMode === 'terminal' ? (
            <section className="flex-1 flex flex-col bg-surface-container-lowest border border-outline-variant/20 rounded-[var(--radius-default)] overflow-hidden">
              <div className="h-7 bg-surface-container-high px-3 flex items-center justify-between border-b border-outline-variant/20 shrink-0">
                <div className="flex items-center">
                  <span className="font-mono text-[10px] font-bold uppercase tracking-widest text-on-surface-variant">
                    Terminal
                  </span>
                  <span className="ml-2 w-1.5 h-1.5 rounded-full bg-secondary animate-pulse" />
                </div>
                <button
                  onClick={() => setTerminalMode('idle')}
                  className="font-mono text-[9px] text-outline hover:text-primary transition-colors"
                >
                  CLOSE
                </button>
              </div>
              <div className="flex-1 overflow-hidden">
                <TerminalPanel ref={plannerRef} role="terminal" ptyMode={true} />
              </div>
            </section>
          ) : !activeSession && !showNewSession ? (
            <WelcomeView
              onNewSession={() => setShowNewSession(true)}
              onOpenTerminal={() => setTerminalMode('terminal')}
              daemonRunning={daemonRunning}
            />
          ) : fullscreen ? (
            <section className="flex-1 flex flex-col bg-surface-container-lowest border border-outline-variant/20 rounded-[var(--radius-default)] overflow-hidden">
              <div className="h-7 bg-surface-container-high px-3 flex items-center justify-between border-b border-outline-variant/20 shrink-0">
                <div className="flex items-center">
                  <span className="font-mono text-[10px] font-bold uppercase tracking-widest text-on-surface-variant">
                    {fullscreen}
                  </span>
                  <span className={`ml-2 w-1.5 h-1.5 rounded-full ${connected ? (fullscreen === 'planner' ? 'bg-secondary' : 'bg-tertiary') + ' animate-pulse' : 'bg-outline'}`} />
                </div>
                <button
                  onClick={() => setFullscreen(null)}
                  className="font-mono text-[9px] text-outline hover:text-primary transition-colors"
                >
                  EXIT FULLSCREEN
                </button>
              </div>
              <div className="flex-1 overflow-hidden">
                <TerminalPanel ref={fullscreen === 'planner' ? plannerRef : verifierRef} role={fullscreen} />
              </div>
            </section>
          ) : (
            <>
              {/* Planner Panel */}
              <section
                className="flex flex-col bg-surface-container-lowest border border-outline-variant/20 rounded-[var(--radius-default)] overflow-hidden"
                style={isBottom ? { height: `${plannerPct}%` } : { width: `${plannerPct}%` }}
              >
                <div className="h-7 bg-surface-container-high px-3 flex items-center justify-between border-b border-outline-variant/20 shrink-0">
                  <div className="flex items-center">
                    <span className="font-mono text-[10px] font-bold uppercase tracking-widest text-on-surface-variant">
                      Planner
                    </span>
                    <span className={`ml-2 w-1.5 h-1.5 rounded-full ${connected ? 'bg-secondary animate-pulse' : 'bg-outline'}`} />
                  </div>
                  <button
                    onClick={() => setFullscreen('planner')}
                    className="material-symbols-outlined text-[14px] text-outline hover:text-primary transition-colors"
                    title="Fullscreen (Cmd+F)"
                  >
                    fullscreen
                  </button>
                </div>
                <div className="flex-1 overflow-hidden">
                  <TerminalPanel ref={plannerRef} role="planner" />
                </div>
              </section>

              {/* Resizable Divider (Focus mode only) */}
              {mode === 'focus' && (
                <div
                  className={`shrink-0 group flex items-center justify-center transition-colors ${
                    isBottom
                      ? 'h-1 cursor-row-resize hover:bg-primary/20'
                      : 'w-1 cursor-col-resize hover:bg-primary/20'
                  }`}
                  onMouseDown={() => setIsDragging(true)}
                >
                  <div className={`rounded-full transition-colors ${
                    isBottom ? 'w-8 h-0.5' : 'w-0.5 h-8'
                  } ${isDragging ? 'bg-primary' : 'bg-outline-variant/40 group-hover:bg-primary/60'}`} />
                </div>
              )}

              {/* Mission Control (Control mode only) */}
              {mode === 'control' && (
                <>
                  <div className="w-1 shrink-0" />
                  <div className="shrink-0" style={{ width: '30%' }}>
                    <MissionControl
                      currentRound={currentRound}
                      maxRounds={maxRounds}
                      rounds={rounds}
                      health={health}
                      safetyOk={safetyOk}
                      gitBranch={gitBranch}
                      gitFilesChanged={gitFilesChanged}
                      eventLog={eventLog}
                      crossVerify={crossVerify}
                    />
                  </div>
                  <div className="w-1 shrink-0" />
                </>
              )}

              {/* Verifier Panel */}
              <section
                className="flex flex-col bg-surface-container-lowest border border-outline-variant/20 rounded-[var(--radius-default)] overflow-hidden"
                style={isBottom ? { height: `${verifierPct}%` } : { width: `${verifierPct}%` }}
              >
                <div className="h-7 bg-surface-container-high px-3 flex items-center justify-between border-b border-outline-variant/20 shrink-0">
                  <div className="flex items-center">
                    <span className="font-mono text-[10px] font-bold uppercase tracking-widest text-on-surface-variant">
                      Verifier
                    </span>
                    <span className={`ml-2 w-1.5 h-1.5 rounded-full ${connected ? 'bg-tertiary animate-pulse' : 'bg-outline'}`} />
                  </div>
                  <button
                    onClick={() => setFullscreen('verifier')}
                    className="material-symbols-outlined text-[14px] text-outline hover:text-primary transition-colors"
                    title="Fullscreen (Cmd+F)"
                  >
                    fullscreen
                  </button>
                </div>
                <div className="flex-1 overflow-hidden">
                  <TerminalPanel ref={verifierRef} role="verifier" />
                </div>
              </section>

              {/* Browser Panel */}
              {showBrowser && (
                <>
                  <div className={isBottom ? 'h-1 shrink-0' : 'w-1 shrink-0'} />
                  <div className="shrink-0" style={isBottom ? { height: '40%' } : { width: '40%' }}>
                    <BrowserPanel onClose={() => setShowBrowser(false)} />
                  </div>
                </>
              )}
            </>
          )}
        </main>
      </div>

      {/* Bottom Shortcut Bar */}
      <footer className={`h-7 flex items-center justify-between px-4 border-t z-50 shrink-0 ${statusBarBg}`}>
        <div className="flex items-center gap-3">
          <span className="font-mono text-[10px] text-on-surface-variant">
            <span className="text-primary">[Tab]</span> {mode === 'focus' ? 'Control' : 'Focus'}
          </span>
          <span className="font-mono text-[10px] text-on-surface-variant">
            <span className="text-primary">[i]</span> Intervene
          </span>
          <span className="font-mono text-[10px] text-on-surface-variant">
            <span className="text-primary">[r]</span> Rewind
          </span>
          <span className="font-mono text-[10px] text-on-surface-variant">
            <span className="text-primary">[d]</span> Diff
          </span>
          <span className="font-mono text-[10px] text-on-surface-variant">
            <span className="text-primary">[q]</span> Quit
          </span>
          <span className="font-mono text-[10px] text-on-surface-variant">
            <span className="text-primary">[⌘B]</span> Browser
          </span>
          <span className="font-mono text-[10px] text-on-surface-variant">
            <span className="text-primary">[⌘1/2/3]</span> Layout
          </span>
          <span className="font-mono text-[10px] text-on-surface-variant">
            <span className="text-primary">[⌘F]</span> Fullscreen
          </span>
        </div>
        <div className="flex items-center gap-4">
          <span className="font-mono text-[10px] text-outline flex items-center gap-1">
            <span className={`w-1.5 h-1.5 rounded-full ${connected ? 'bg-secondary' : 'bg-outline'}`} />
            {connected ? 'Connected' : 'Idle'}
          </span>
          <span className="font-mono text-[10px] text-outline">
            Git: {gitBranch}{gitFilesChanged > 0 ? ` (+${gitFilesChanged})` : ''}
          </span>
        </div>
      </footer>
    </div>
  )
}

export default App
