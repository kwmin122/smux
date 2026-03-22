import { useState, useEffect, useRef, useCallback } from 'react'
import { TerminalPanel, type TerminalPanelHandle } from './components/TerminalPanel'
import { MissionControl, type RoundEntry, type EventLogEntry, type CrossVerifyState } from './components/MissionControl'
import { BrowserPanel } from './components/BrowserPanel'
import { WelcomeView } from './components/WelcomeView'
import { TabBar, type TabInfo, type TabColor } from './components/TabBar'
import { SplitContainer, type SplitNode, createLeaf, splitLeaf, removeLeaf } from './components/SplitContainer'
import { AiExecutionLevel, type ExecutionLevel } from './components/AiExecutionLevel'
import { FailedCommandOverlay } from './components/FailedCommandOverlay'
import { useAiPingPong, type AiSessionStatus } from './hooks/useAiPingPong'
import type { CommandRecord } from './hooks/useShellIntegration'

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
  const [showSettings, setShowSettings] = useState(false)
  const [aiTask, setAiTask] = useState('')
  const [showAiPrompt, setShowAiPrompt] = useState(false)
  // Tab management
  const [tabs, setTabs] = useState<TabInfo[]>([])
  const [activeTabId, setActiveTabId] = useState<string | null>(null)
  const tabRefsMap = useRef<Map<string, TerminalPanelHandle>>(new Map())
  // Split pane management
  const [splitRoot, setSplitRoot] = useState<SplitNode | null>(null)
  const [activeLeafId, setActiveLeafId] = useState<string | null>(null)
  // AI session state
  const [executionLevel, setExecutionLevel] = useState<ExecutionLevel>('auto')
  const [failedCommand, setFailedCommand] = useState<CommandRecord | null>(null)
  const [aiSessionStatus, setAiSessionStatus] = useState<AiSessionStatus>('idle')
  const pingPong = useAiPingPong()
  const [terminalMode, setTerminalMode] = useState<'idle' | 'terminal' | 'ai-session'>(() => {
    // Auto-resume last project if available
    try {
      const last = localStorage.getItem('smux-last-project')
      return last ? 'terminal' : 'idle'
    } catch { return 'idle' }
  })
  const [projectDir, setProjectDir] = useState<string>(() => {
    try { return localStorage.getItem('smux-last-project') || '' } catch { return '' }
  })

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
      if (e.key === 't' && (e.metaKey || e.ctrlKey)) {
        e.preventDefault()
        if (terminalMode === 'terminal') createTab()
      }
      if (e.key === 'w' && (e.metaKey || e.ctrlKey) && !e.shiftKey) {
        e.preventDefault()
        if (terminalMode === 'terminal' && activeLeafId && splitRoot?.type === 'split') {
          handleClosePane(activeLeafId)
        } else if (terminalMode === 'terminal' && activeTabId && tabs.length > 1) {
          closeTab(activeTabId)
        }
      }
      if (e.key === 'd' && (e.metaKey || e.ctrlKey) && !e.shiftKey) {
        e.preventDefault()
        if (terminalMode === 'terminal') handleSplit('vertical')
      }
      if (e.key === 'D' && (e.metaKey || e.ctrlKey) && e.shiftKey) {
        e.preventDefault()
        if (terminalMode === 'terminal') handleSplit('horizontal')
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

  // --- Tab management ---
  const createTab = useCallback(() => {
    const id = `tab-${Date.now()}-${Math.random().toString(36).slice(2, 6)}`
    const dirName = projectDir ? projectDir.split('/').pop() || '~' : '~'
    const newTab: TabInfo = {
      id,
      name: dirName,
      cwd: projectDir || '',
      color: 'default' as TabColor,
      icon: 'terminal',
      isActive: true,
      status: 'running',
    }
    setTabs(prev => [
      ...prev.map(t => ({ ...t, isActive: false })),
      newTab,
    ])
    setActiveTabId(id)
    return id
  }, [projectDir])

  const selectTab = useCallback((id: string) => {
    setTabs(prev => prev.map(t => ({ ...t, isActive: t.id === id })))
    setActiveTabId(id)
  }, [])

  const closeTab = useCallback((id: string) => {
    setTabs(prev => {
      const filtered = prev.filter(t => t.id !== id)
      if (filtered.length === 0) return filtered
      // If we're closing the active tab, activate the last one
      if (id === activeTabId) {
        const last = filtered[filtered.length - 1]
        last.isActive = true
        setActiveTabId(last.id)
      }
      return filtered
    })
    tabRefsMap.current.delete(id)
  }, [activeTabId])

  const renameTab = useCallback((id: string, name: string) => {
    setTabs(prev => prev.map(t => t.id === id ? { ...t, name } : t))
  }, [])

  const changeTabColor = useCallback((id: string, color: TabColor) => {
    setTabs(prev => prev.map(t => t.id === id ? { ...t, color } : t))
  }, [])

  const reorderTabs = useCallback((fromId: string, toId: string) => {
    setTabs(prev => {
      const arr = [...prev]
      const fromIdx = arr.findIndex(t => t.id === fromId)
      const toIdx = arr.findIndex(t => t.id === toId)
      if (fromIdx < 0 || toIdx < 0) return prev
      const [moved] = arr.splice(fromIdx, 1)
      arr.splice(toIdx, 0, moved)
      return arr
    })
  }, [])

  // Auto-create first tab when entering terminal mode
  useEffect(() => {
    if (terminalMode === 'terminal' && tabs.length === 0) {
      createTab()
    }
  }, [terminalMode, tabs.length, createTab])

  // Initialize split root when first tab is created
  useEffect(() => {
    if (tabs.length > 0 && !splitRoot) {
      const leaf = createLeaf(tabs[0].id)
      setSplitRoot(leaf)
      setActiveLeafId(leaf.id)
    }
  }, [tabs, splitRoot])

  // Split the active pane
  const handleSplit = useCallback((direction: 'horizontal' | 'vertical') => {
    if (!splitRoot || !activeLeafId) return
    const newTabId = createTab()
    setSplitRoot(prev => prev ? splitLeaf(prev, activeLeafId, direction, newTabId) : prev)
  }, [splitRoot, activeLeafId, createTab])

  const handleSplitResize = useCallback((splitId: string, ratio: number) => {
    setSplitRoot(prev => {
      if (!prev) return prev
      const update = (node: SplitNode): SplitNode => {
        if (node.id === splitId && node.type === 'split') {
          return { ...node, ratio }
        }
        if (node.type === 'split' && node.children) {
          return { ...node, children: [update(node.children[0]), update(node.children[1])] }
        }
        return node
      }
      return update(prev)
    })
  }, [])

  const handleClosePane = useCallback((leafId: string) => {
    if (!splitRoot) return
    // Find the tab associated with this leaf
    const findTabId = (node: SplitNode): string | null => {
      if (node.type === 'leaf' && node.id === leafId) return node.tabId || null
      if (node.children) {
        return findTabId(node.children[0]) || findTabId(node.children[1])
      }
      return null
    }
    const tabId = findTabId(splitRoot)

    const newRoot = removeLeaf(splitRoot, leafId)
    if (newRoot) {
      setSplitRoot(newRoot)
      // Set active to first remaining leaf
      const findFirst = (n: SplitNode): string => n.type === 'leaf' ? n.id : findFirst(n.children![0])
      setActiveLeafId(findFirst(newRoot))
    } else {
      setSplitRoot(null)
      setActiveLeafId(null)
    }
    if (tabId) closeTab(tabId)
  }, [splitRoot, closeTab])

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
          <button
            onClick={() => setShowSettings(true)}
            className="material-symbols-outlined text-[16px] text-outline hover:text-primary transition-colors cursor-pointer"
          >
            settings
          </button>
        </div>
      </header>

      {/* Main Content */}
      <div className="flex flex-1 overflow-hidden">
        {/* Sidebar */}
        <nav className="w-56 bg-surface-container-low flex flex-col shrink-0 border-r border-outline-variant/20 z-40">
          {/* Terminal tabs section */}
          {terminalMode === 'terminal' && (
            <TabBar
              tabs={tabs}
              onSelectTab={selectTab}
              onCloseTab={closeTab}
              onNewTab={createTab}
              onRenameTab={renameTab}
              onChangeColor={changeTabColor}
              onReorder={reorderTabs}
            />
          )}

          {/* AI Sessions section */}
          {terminalMode !== 'terminal' && (
          <div className="px-3 py-2 border-b border-outline-variant/20">
            <div className="text-[10px] font-mono uppercase tracking-widest text-outline">
              Sessions
            </div>
          </div>
          )}
          <div className="flex-1 overflow-y-auto py-1">
            {terminalMode !== 'terminal' && activeSession && (
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
            {terminalMode !== 'terminal' && !activeSession && !showNewSession && (
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
          {/* Terminal Mode: multi-tab PTY shells */}
          {terminalMode === 'terminal' ? (
            <section className="flex-1 flex flex-col bg-surface-container-lowest border border-outline-variant/20 rounded-[var(--radius-default)] overflow-hidden">
              <div className="h-7 bg-surface-container-high px-3 flex items-center justify-between border-b border-outline-variant/20 shrink-0">
                <div className="flex items-center">
                  <span className="font-mono text-[10px] font-bold uppercase tracking-widest text-on-surface-variant">
                    {tabs.find(t => t.id === activeTabId)?.name || 'Terminal'}
                  </span>
                  <span className="ml-2 w-1.5 h-1.5 rounded-full bg-secondary animate-pulse" />
                </div>
                <div className="flex items-center gap-2">
                  <button
                    onClick={() => setShowAiPrompt(true)}
                    className="font-mono text-[9px] px-2 py-0.5 rounded bg-secondary/10 text-secondary border border-secondary/20 hover:bg-secondary/20 transition-colors"
                  >
                    AI PING-PONG
                  </button>
                  <button
                    onClick={() => { setTerminalMode('idle'); setActiveSession(null); setTabs([]); setActiveTabId(null); try { localStorage.removeItem('smux-last-project') } catch {} }}
                    className="font-mono text-[9px] text-outline hover:text-primary transition-colors"
                  >
                    HOME
                  </button>
                </div>
              </div>
              <div className="flex-1 overflow-hidden relative">
                {splitRoot && splitRoot.type === 'split' ? (
                  <SplitContainer
                    root={splitRoot}
                    activeLeafId={activeLeafId}
                    onActivateLeaf={setActiveLeafId}
                    onResizeRatio={handleSplitResize}
                    renderLeaf={(node) => {
                      const tabId = node.tabId || activeTabId || ''
                      const tab = tabs.find(t => t.id === tabId)
                      return (
                        <TerminalPanel
                          ref={(handle) => {
                            if (handle) tabRefsMap.current.set(tabId, handle)
                          }}
                          role="terminal"
                          ptyMode={true}
                          cwd={tab?.cwd || projectDir || undefined}
                          onCwdChange={(cwd) => {
                            setTabs(prev => prev.map(t => t.id === tabId ? { ...t, cwd, name: cwd.split('/').pop() || t.name } : t))
                          }}
                          onCommandComplete={(cmd) => {
                            if (cmd.status === 'error') setFailedCommand(cmd)
                          }}
                        />
                      )
                    }}
                  />
                ) : (
                  tabs.map(tab => (
                    <div
                      key={tab.id}
                      className="absolute inset-0"
                      style={{ display: tab.id === activeTabId ? 'block' : 'none' }}
                    >
                      <TerminalPanel
                        ref={(handle) => {
                          if (handle) tabRefsMap.current.set(tab.id, handle)
                          if (handle && tab.id === tabs[0]?.id) {
                            (plannerRef as React.MutableRefObject<TerminalPanelHandle | null>).current = handle
                          }
                        }}
                        role="terminal"
                        ptyMode={true}
                        cwd={tab.cwd || projectDir || undefined}
                        onCwdChange={(cwd) => {
                          setTabs(prev => prev.map(t => t.id === tab.id ? { ...t, cwd, name: cwd.split('/').pop() || t.name } : t))
                        }}
                        onCommandComplete={(cmd) => {
                          if (cmd.status === 'error') setFailedCommand(cmd)
                        }}
                      />
                    </div>
                  ))
                )}
                {/* Failed Command Overlay */}
                {failedCommand && (
                  <FailedCommandOverlay
                    command={failedCommand}
                    onFixWithAi={(cmd) => {
                      // Send failed command to AI for analysis
                      const activeRef = tabRefsMap.current.get(activeTabId || '')
                      if (activeRef) {
                        activeRef.write(`\n# AI Analysis: The command '${cmd.command}' failed with exit code ${cmd.exitCode}\n`)
                        activeRef.write(`claude -p 'The following command failed with exit code ${cmd.exitCode}: ${cmd.command}. Please analyze why and suggest a fix.'\n`)
                      }
                      setFailedCommand(null)
                    }}
                    onDismiss={() => setFailedCommand(null)}
                  />
                )}
              </div>
            </section>
          ) : terminalMode === 'ai-session' && aiTask ? (
            <>
              {/* Planner PTY Panel — runs claude */}
              <section className="flex flex-col bg-surface-container-lowest border border-outline-variant/20 rounded-[var(--radius-default)] overflow-hidden" style={{ width: '50%' }}>
                <div className="h-7 bg-surface-container-high px-3 flex items-center justify-between border-b border-outline-variant/20 shrink-0">
                  <div className="flex items-center gap-2">
                    <span className="font-mono text-[10px] font-bold uppercase tracking-widest text-secondary">Planner (Claude)</span>
                    <span className={`w-1.5 h-1.5 rounded-full ${pingPong.status === 'planner-running' ? 'bg-secondary animate-pulse' : 'bg-outline'}`} />
                    {pingPong.currentRound > 0 && (
                      <span className="font-mono text-[9px] text-on-surface-variant">R{pingPong.currentRound}</span>
                    )}
                  </div>
                  <div className="flex items-center gap-2">
                    <AiExecutionLevel level={executionLevel} onChange={setExecutionLevel} compact />
                    {pingPong.status === 'idle' && (
                      <button
                        onClick={() => {
                          if (plannerRef.current && verifierRef.current) {
                            setAiSessionStatus('planner-running')
                            pingPong.start(
                              {
                                task: aiTask,
                                planner: 'claude',
                                verifier: 'codex',
                                maxRounds: 5,
                                cwd: projectDir || undefined,
                                onStatusChange: setAiSessionStatus,
                                onRoundComplete: (round) => {
                                  addLogEntry('round', `R${round.round}: ${round.verdict}`)
                                  if (document.hidden) {
                                    notify(`Round ${round.round}`, round.verdict.toUpperCase())
                                  }
                                },
                                onSessionComplete: () => {
                                  notify('AI Session Complete', `Finished after ${pingPong.currentRound} rounds`)
                                },
                              },
                              plannerRef.current,
                              verifierRef.current
                            )
                          }
                        }}
                        className="font-mono text-[9px] px-2 py-0.5 rounded bg-secondary text-on-primary hover:opacity-90"
                      >
                        START
                      </button>
                    )}
                    {(pingPong.status === 'planner-running' || pingPong.status === 'verifier-running') && (
                      <>
                        <button
                          onClick={pingPong.pause}
                          className="font-mono text-[9px] px-2 py-0.5 rounded bg-yellow-500/20 text-yellow-400 border border-yellow-500/30 hover:opacity-90"
                        >
                          PAUSE
                        </button>
                        <button
                          onClick={pingPong.abort}
                          className="font-mono text-[9px] px-2 py-0.5 rounded bg-error/20 text-error border border-error/30 hover:opacity-90"
                        >
                          STOP
                        </button>
                      </>
                    )}
                    {pingPong.status === 'paused' && (
                      <button
                        onClick={pingPong.resume}
                        className="font-mono text-[9px] px-2 py-0.5 rounded bg-secondary/20 text-secondary border border-secondary/30 hover:opacity-90"
                      >
                        RESUME
                      </button>
                    )}
                  </div>
                </div>
                <div className="flex-1 overflow-hidden">
                  <TerminalPanel ref={plannerRef} role="planner" ptyMode={true} cwd={projectDir || undefined} />
                </div>
              </section>

              {/* Divider */}
              <div className="w-1 shrink-0 flex items-center justify-center">
                <div className="w-0.5 h-8 rounded-full bg-outline-variant/40" />
              </div>

              {/* Verifier PTY Panel — runs codex */}
              <section className="flex flex-col bg-surface-container-lowest border border-outline-variant/20 rounded-[var(--radius-default)] overflow-hidden" style={{ width: '50%' }}>
                <div className="h-7 bg-surface-container-high px-3 flex items-center justify-between border-b border-outline-variant/20 shrink-0">
                  <div className="flex items-center gap-2">
                    <span className="font-mono text-[10px] font-bold uppercase tracking-widest text-tertiary">Verifier (Codex)</span>
                    <span className={`w-1.5 h-1.5 rounded-full ${pingPong.status === 'verifier-running' ? 'bg-tertiary animate-pulse' : 'bg-outline'}`} />
                    <span className={`font-mono text-[9px] px-1.5 py-0.5 rounded ${
                      aiSessionStatus === 'completed' ? 'bg-secondary/20 text-secondary' :
                      aiSessionStatus === 'error' ? 'bg-error/20 text-error' :
                      'bg-outline/10 text-outline'
                    }`}>
                      {aiSessionStatus === 'idle' ? 'READY' : aiSessionStatus.toUpperCase().replace('-', ' ')}
                    </span>
                  </div>
                  <button
                    onClick={() => { pingPong.abort(); setTerminalMode('terminal'); setAiTask(''); setAiSessionStatus('idle') }}
                    className="font-mono text-[9px] text-outline hover:text-primary transition-colors"
                  >
                    EXIT AI
                  </button>
                </div>
                <div className="flex-1 overflow-hidden">
                  <TerminalPanel ref={verifierRef} role="verifier" ptyMode={true} cwd={projectDir || undefined} />
                </div>
              </section>
            </>
          ) : !activeSession && !showNewSession && terminalMode === 'idle' ? (
            <WelcomeView
              onOpenFolder={(path: string) => {
                if (path) {
                  try { localStorage.setItem('smux-last-project', path) } catch { /* */ }
                }
                setProjectDir(path)
                setTerminalMode('terminal')
              }}
              onNewSession={() => setShowNewSession(true)}
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

      {/* Bottom Status Bar */}
      <footer className={`h-7 flex items-center justify-between px-4 border-t z-50 shrink-0 ${statusBarBg}`}>
        <div className="flex items-center gap-3">
          <span className="font-mono text-[10px] text-on-surface-variant">
            <span className="text-primary">[Tab]</span> {mode === 'focus' ? 'Control' : 'Focus'}
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
          {projectDir && (
            <span className="font-mono text-[10px] text-outline truncate max-w-[200px]">
              {projectDir.split('/').pop()}
            </span>
          )}
        </div>
      </footer>

      {/* Settings Modal */}
      {showSettings && (
        <div className="fixed inset-0 z-[100] flex items-center justify-center bg-black/50" onClick={() => setShowSettings(false)}>
          <div className="bg-surface-container-high rounded-xl border border-outline-variant/20 w-[400px] shadow-2xl" onClick={e => e.stopPropagation()}>
            <div className="px-5 py-4 border-b border-outline-variant/20 flex items-center justify-between">
              <h2 className="font-headline text-sm font-bold text-on-surface">Settings</h2>
              <button onClick={() => setShowSettings(false)} className="material-symbols-outlined text-[18px] text-outline hover:text-on-surface">close</button>
            </div>
            <div className="px-5 py-4 space-y-4 max-h-[60vh] overflow-y-auto">
              {/* Appearance */}
              <div>
                <label className="font-mono text-[10px] uppercase tracking-widest text-outline block mb-1.5">Theme</label>
                <div className="flex gap-2">
                  {['deep-navy', 'amber', 'forest-green'].map(t => (
                    <button
                      key={t}
                      onClick={() => { setTheme(t); document.documentElement.setAttribute('data-theme', t) }}
                      className={`px-3 py-1.5 rounded font-mono text-[11px] border transition-colors ${
                        theme === t ? 'bg-primary text-on-primary border-primary' : 'border-outline-variant/30 text-on-surface-variant hover:border-primary'
                      }`}
                    >{t}</button>
                  ))}
                </div>
              </div>
              {/* Font Family */}
              <div>
                <label className="font-mono text-[10px] uppercase tracking-widest text-outline block mb-1.5">Font Family</label>
                <select
                  defaultValue="JetBrains Mono"
                  onChange={async (e) => {
                    const font = e.target.value
                    document.documentElement.style.setProperty('--terminal-font', `"${font}", monospace`)
                    if (isTauri) {
                      try {
                        const { invoke } = await import('@tauri-apps/api/core')
                        const config = await invoke<Record<string, unknown>>('load_app_config') as { appearance?: { font_family?: string } }
                        await invoke('save_app_config', { config: { ...config, appearance: { ...config.appearance, font_family: font } } })
                      } catch { /* ignore */ }
                    }
                  }}
                  className="w-full h-8 bg-surface-container-lowest border border-outline-variant/30 rounded px-2 font-mono text-[12px] text-on-surface-variant outline-none focus:border-primary"
                >
                  {['JetBrains Mono', 'SF Mono', 'Menlo', 'Fira Code', 'Cascadia Code', 'Monaco', 'Consolas'].map(f => (
                    <option key={f} value={f}>{f}</option>
                  ))}
                </select>
              </div>
              {/* Font Size */}
              <div>
                <label className="font-mono text-[10px] uppercase tracking-widest text-outline block mb-1.5">Font Size: <span id="font-size-val">14</span>px</label>
                <input
                  type="range"
                  min={10}
                  max={24}
                  defaultValue={14}
                  onChange={(e) => {
                    const size = e.target.value
                    document.documentElement.style.setProperty('--terminal-font-size', `${size}px`)
                    const label = document.getElementById('font-size-val')
                    if (label) label.textContent = size
                  }}
                  className="w-full accent-primary"
                />
              </div>
              {/* Cursor */}
              <div>
                <label className="font-mono text-[10px] uppercase tracking-widest text-outline block mb-1.5">Cursor Style</label>
                <div className="flex gap-2">
                  {['block', 'underline', 'bar'].map(s => (
                    <button
                      key={s}
                      className="px-3 py-1.5 rounded font-mono text-[11px] border border-outline-variant/30 text-on-surface-variant hover:border-primary transition-colors"
                    >{s}</button>
                  ))}
                </div>
              </div>
              {/* Shell */}
              <div>
                <label className="font-mono text-[10px] uppercase tracking-widest text-outline block mb-1.5">Shell</label>
                <div className="font-mono text-[12px] text-on-surface-variant bg-surface-container-lowest px-3 py-2 rounded border border-outline-variant/20">
                  {typeof window !== 'undefined' ? '/bin/zsh' : 'default'}
                </div>
              </div>
              {/* Project */}
              <div>
                <label className="font-mono text-[10px] uppercase tracking-widest text-outline block mb-1.5">Project</label>
                <div className="font-mono text-[12px] text-on-surface-variant bg-surface-container-lowest px-3 py-2 rounded border border-outline-variant/20 truncate">
                  {projectDir || 'No project open'}
                </div>
              </div>
            </div>
          </div>
        </div>
      )}

      {/* AI Task Prompt Modal */}
      {showAiPrompt && (
        <div className="fixed inset-0 z-[100] flex items-center justify-center bg-black/50" onClick={() => setShowAiPrompt(false)}>
          <div className="bg-surface-container-high rounded-xl border border-outline-variant/20 w-[500px] shadow-2xl" onClick={e => e.stopPropagation()}>
            <div className="px-5 py-4 border-b border-outline-variant/20">
              <h2 className="font-headline text-sm font-bold text-on-surface">AI Ping-Pong Session</h2>
              <p className="text-[11px] text-on-surface-variant mt-1">Planner (Claude) will code, Verifier (Codex) will review</p>
            </div>
            <div className="px-5 py-4">
              <label className="font-mono text-[10px] uppercase tracking-widest text-outline block mb-1.5">Task</label>
              <textarea
                value={aiTask}
                onChange={e => setAiTask(e.target.value)}
                placeholder="What should the AI agents work on?"
                className="w-full h-20 bg-surface-container-lowest border border-outline-variant/30 rounded px-3 py-2 font-mono text-[12px] text-on-surface resize-none outline-none focus:border-primary"
                autoFocus
              />
            </div>
            <div className="px-5 py-3 border-t border-outline-variant/20 flex justify-end gap-2">
              <button onClick={() => setShowAiPrompt(false)} className="px-4 py-1.5 font-mono text-[11px] text-outline hover:text-on-surface border border-outline-variant/30 rounded">Cancel</button>
              <button
                onClick={() => {
                  if (aiTask.trim()) {
                    setShowAiPrompt(false)
                    setTerminalMode('ai-session')
                  }
                }}
                disabled={!aiTask.trim()}
                className="px-4 py-1.5 font-mono text-[11px] bg-primary text-on-primary rounded hover:opacity-90 disabled:opacity-40"
              >Start</button>
            </div>
          </div>
        </div>
      )}
    </div>
  )
}

export default App
