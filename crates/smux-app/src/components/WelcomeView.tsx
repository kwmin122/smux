import { useState, useEffect } from 'react'

type Lang = 'ko' | 'en'

interface RecentProject {
  path: string
  name: string
  lastOpened: number
}

interface WelcomeViewProps {
  onOpenFolder: (path: string) => void
  onNewSession: () => void
  daemonRunning: boolean
}

const text = {
  ko: {
    title: 'smux',
    subtitle: 'AI 에이전트가 서로 검증하는 코딩 워크플로우',
    openFolder: '폴더 열기',
    openFolderDesc: '프로젝트 폴더를 선택하여 시작',
    aiSession: 'AI 핑퐁 세션',
    aiSessionDesc: 'AI 에이전트가 코딩하고 검증',
    recentProjects: '최근 프로젝트',
    noRecent: '최근에 열었던 프로젝트가 없습니다',
    getStarted: '시작하기',
    step1: '프로젝트 폴더를 선택하세요',
    step2: '터미널에서 바로 작업하세요',
    step3: 'AI 모드를 켜면 에이전트가 자동으로 핑퐁합니다',
    shortcuts: '단축키',
  },
  en: {
    title: 'smux',
    subtitle: 'AI agents that verify each other\'s code',
    openFolder: 'Open Folder',
    openFolderDesc: 'Select a project folder to start',
    aiSession: 'AI Ping-Pong',
    aiSessionDesc: 'AI agents code and verify automatically',
    recentProjects: 'Recent Projects',
    noRecent: 'No recent projects',
    getStarted: 'Get Started',
    step1: 'Select a project folder',
    step2: 'Start working in the terminal',
    step3: 'Toggle AI mode for automatic agent ping-pong',
    shortcuts: 'Shortcuts',
  },
}

function getRecentProjects(): RecentProject[] {
  try {
    const saved = localStorage.getItem('smux-recent-projects')
    return saved ? JSON.parse(saved) : []
  } catch {
    return []
  }
}

export function addRecentProject(path: string) {
  try {
    const projects = getRecentProjects().filter(p => p.path !== path)
    const name = path.split('/').pop() || path
    projects.unshift({ path, name, lastOpened: Date.now() })
    localStorage.setItem('smux-recent-projects', JSON.stringify(projects.slice(0, 10)))
  } catch { /* ignore */ }
}

export function WelcomeView({ onOpenFolder, onNewSession, daemonRunning }: WelcomeViewProps) {
  const [lang, setLang] = useState<Lang>(() => {
    try {
      return (localStorage.getItem('smux-lang') as Lang) || 'ko'
    } catch {
      return 'ko'
    }
  })

  const [recentProjects, setRecentProjects] = useState<RecentProject[]>([])

  useEffect(() => {
    try { localStorage.setItem('smux-lang', lang) } catch { /* ignore */ }
  }, [lang])

  useEffect(() => {
    setRecentProjects(getRecentProjects())
  }, [])

  const t = text[lang]

  async function handleOpenFolder() {
    try {
      const { open } = await import('@tauri-apps/plugin-dialog')
      const selected = await open({ directory: true, multiple: false, title: t.openFolder })
      if (selected && typeof selected === 'string') {
        addRecentProject(selected)
        onOpenFolder(selected)
      }
    } catch {
      // Fallback for non-Tauri (browser dev mode)
      onOpenFolder('')
    }
  }

  return (
    <div className="flex-1 overflow-y-auto bg-surface-container-lowest">
      <div className="max-w-2xl mx-auto px-8 py-12">
        {/* Language Toggle */}
        <div className="flex justify-end mb-8">
          <div className="flex bg-surface-container-high rounded-sm overflow-hidden border border-outline-variant/20">
            <button
              onClick={() => setLang('ko')}
              className={`px-3 py-1 font-mono text-[11px] transition-colors ${
                lang === 'ko' ? 'bg-primary text-on-primary' : 'text-on-surface-variant hover:bg-surface-container-highest'
              }`}
            >한국어</button>
            <button
              onClick={() => setLang('en')}
              className={`px-3 py-1 font-mono text-[11px] transition-colors ${
                lang === 'en' ? 'bg-primary text-on-primary' : 'text-on-surface-variant hover:bg-surface-container-highest'
              }`}
            >EN</button>
          </div>
        </div>

        {/* Hero */}
        <div className="text-center mb-10">
          <h1 className="font-headline text-4xl font-bold text-on-surface tracking-tight mb-2">{t.title}</h1>
          <p className="text-on-surface-variant text-sm">{t.subtitle}</p>
        </div>

        {/* Action Buttons */}
        <div className="grid grid-cols-2 gap-4 mb-10">
          <button
            onClick={handleOpenFolder}
            className="flex flex-col items-center gap-2 px-6 py-6 bg-primary/10 border border-primary/30 rounded-xl hover:bg-primary/20 transition-colors group"
          >
            <span className="material-symbols-outlined text-3xl text-primary group-hover:scale-110 transition-transform">folder_open</span>
            <div className="text-center">
              <h3 className="font-mono text-[12px] font-bold text-on-surface">{t.openFolder}</h3>
              <p className="text-[10px] text-on-surface-variant mt-1">{t.openFolderDesc}</p>
            </div>
          </button>
          <button
            onClick={onNewSession}
            disabled={!daemonRunning}
            className={`flex flex-col items-center gap-2 px-6 py-6 rounded-xl border transition-colors group ${
              daemonRunning
                ? 'bg-secondary/10 border-secondary/30 hover:bg-secondary/20'
                : 'bg-surface-container-low border-outline-variant/20 opacity-40 cursor-not-allowed'
            }`}
          >
            <span className="material-symbols-outlined text-3xl text-secondary group-hover:scale-110 transition-transform">hub</span>
            <div className="text-center">
              <h3 className="font-mono text-[12px] font-bold text-on-surface">{t.aiSession}</h3>
              <p className="text-[10px] text-on-surface-variant mt-1">{t.aiSessionDesc}</p>
            </div>
          </button>
        </div>

        {/* Recent Projects */}
        <section className="mb-10">
          <h2 className="font-mono text-[11px] font-bold uppercase tracking-widest text-outline mb-3">{t.recentProjects}</h2>
          {recentProjects.length === 0 ? (
            <p className="text-[12px] text-on-surface-variant/50 italic">{t.noRecent}</p>
          ) : (
            <div className="space-y-1">
              {recentProjects.map(p => (
                <button
                  key={p.path}
                  onClick={() => { addRecentProject(p.path); onOpenFolder(p.path) }}
                  className="flex items-center gap-3 w-full px-3 py-2 rounded-lg hover:bg-surface-container-high transition-colors text-left group"
                >
                  <span className="material-symbols-outlined text-[16px] text-outline group-hover:text-primary">folder</span>
                  <div className="min-w-0 flex-1">
                    <div className="font-mono text-[12px] text-on-surface truncate">{p.name}</div>
                    <div className="font-mono text-[10px] text-outline truncate">{p.path}</div>
                  </div>
                </button>
              ))}
            </div>
          )}
        </section>

        {/* Getting Started Steps */}
        <section className="mb-8">
          <h2 className="font-mono text-[11px] font-bold uppercase tracking-widest text-outline mb-3">{t.getStarted}</h2>
          <div className="space-y-2">
            {[t.step1, t.step2, t.step3].map((step, i) => (
              <div key={i} className="flex items-center gap-3 px-3 py-2 bg-surface-container-low rounded-lg">
                <span className="w-5 h-5 rounded-full bg-primary/15 text-primary font-mono text-[10px] font-bold flex items-center justify-center shrink-0">{i + 1}</span>
                <span className="text-[11px] text-on-surface-variant">{step}</span>
              </div>
            ))}
          </div>
        </section>

        {/* Shortcuts */}
        <section>
          <h2 className="font-mono text-[11px] font-bold uppercase tracking-widest text-outline mb-3">{t.shortcuts}</h2>
          <div className="grid grid-cols-2 gap-x-6 gap-y-1.5">
            {[
              { key: 'Tab', desc: lang === 'ko' ? 'Focus ↔ Control' : 'Focus ↔ Control' },
              { key: '⌘1/2/3', desc: lang === 'ko' ? '레이아웃' : 'Layout' },
              { key: '⌘F', desc: lang === 'ko' ? '전체화면' : 'Fullscreen' },
              { key: '⌘B', desc: lang === 'ko' ? '브라우저' : 'Browser' },
            ].map(s => (
              <div key={s.key} className="flex items-center gap-2">
                <kbd className="px-1.5 py-0.5 bg-surface-container-high rounded text-[10px] font-mono text-primary border border-outline-variant/20 min-w-[40px] text-center">{s.key}</kbd>
                <span className="text-[11px] text-on-surface-variant">{s.desc}</span>
              </div>
            ))}
          </div>
        </section>
      </div>
    </div>
  )
}
