import { useState, useEffect } from 'react'

type Lang = 'ko' | 'en'

interface WelcomeViewProps {
  onNewSession: () => void
  onOpenTerminal: () => void
  daemonRunning: boolean
}

const content = {
  ko: {
    title: 'smux에 오신 걸 환영합니다',
    subtitle: 'AI 에이전트가 서로 검증하는 코딩 워크플로우',
    whatIs: 'smux란?',
    whatIsDesc:
      '여러 AI 에이전트(Claude, Codex, Gemini)가 코드를 작성하고, 다른 에이전트가 독립적으로 검증합니다. 틀린 부분이 있으면 서로 토론하며 합의에 도달할 때까지 반복합니다.',
    howItWorks: '어떻게 작동하나요?',
    steps: [
      {
        icon: '1',
        title: 'Daemon 시작',
        desc: '터미널에서 smux-daemon을 실행하세요',
        code: 'smux-daemon',
      },
      {
        icon: '2',
        title: '세션 생성',
        desc: '좌측 사이드바에서 "New Session"을 클릭하세요',
        code: null,
      },
      {
        icon: '3',
        title: '관찰 & 개입',
        desc: 'Planner가 코딩하고 Verifier가 검증하는 과정을 실시간으로 봅니다',
        code: null,
      },
    ],
    daemonStatus: 'Daemon 상태',
    daemonRunning: '연결됨 — 세션을 시작할 수 있습니다',
    daemonStopped: '연결 안 됨 — 터미널에서 smux-daemon을 먼저 실행하세요',
    quickStart: '빠른 시작',
    startSession: 'AI 세션',
    openTerminal: '터미널 열기',
    features: '주요 기능',
    featureList: [
      { icon: '🔄', title: 'Cross-Verify', desc: '여러 AI가 서로의 코드를 검증' },
      { icon: '🎯', title: 'Focus & Control', desc: '두 가지 모드로 워크플로우 관리' },
      { icon: '⌨️', title: 'Keyboard-First', desc: 'Tab, ⌘1/2/3으로 빠른 조작' },
      { icon: '🔀', title: 'Consensus', desc: '다수결, 가중치, 만장일치 등 합의 전략' },
    ],
    shortcuts: '단축키',
    shortcutList: [
      { key: 'Tab', desc: 'Focus ↔ Control 모드 전환' },
      { key: '⌘1/2/3', desc: '레이아웃 변경' },
      { key: '⌘F', desc: '전체화면' },
      { key: '⌘B', desc: '브라우저 패널' },
      { key: '⌘S', desc: '레이아웃 저장' },
      { key: '⌘⇧S', desc: '패널 위치 교체' },
    ],
  },
  en: {
    title: 'Welcome to smux',
    subtitle: 'AI agents that verify each other\'s code',
    whatIs: 'What is smux?',
    whatIsDesc:
      'Multiple AI agents (Claude, Codex, Gemini) write code while other agents independently verify it. When something is wrong, they debate until consensus is reached.',
    howItWorks: 'How does it work?',
    steps: [
      {
        icon: '1',
        title: 'Start Daemon',
        desc: 'Run smux-daemon in your terminal',
        code: 'smux-daemon',
      },
      {
        icon: '2',
        title: 'Create Session',
        desc: 'Click "New Session" in the sidebar',
        code: null,
      },
      {
        icon: '3',
        title: 'Watch & Intervene',
        desc: 'See the Planner code and the Verifier review in real-time',
        code: null,
      },
    ],
    daemonStatus: 'Daemon Status',
    daemonRunning: 'Connected — you can start a session',
    daemonStopped: 'Not connected — run smux-daemon in your terminal first',
    quickStart: 'Quick Start',
    startSession: 'AI Session',
    openTerminal: 'Open Terminal',
    features: 'Key Features',
    featureList: [
      { icon: '🔄', title: 'Cross-Verify', desc: 'Multiple AIs verify each other\'s code' },
      { icon: '🎯', title: 'Focus & Control', desc: 'Two modes for workflow management' },
      { icon: '⌨️', title: 'Keyboard-First', desc: 'Tab, ⌘1/2/3 for fast navigation' },
      { icon: '🔀', title: 'Consensus', desc: 'Majority, weighted, unanimous strategies' },
    ],
    shortcuts: 'Shortcuts',
    shortcutList: [
      { key: 'Tab', desc: 'Toggle Focus ↔ Control' },
      { key: '⌘1/2/3', desc: 'Change layout' },
      { key: '⌘F', desc: 'Fullscreen' },
      { key: '⌘B', desc: 'Browser panel' },
      { key: '⌘S', desc: 'Save layout' },
      { key: '⌘⇧S', desc: 'Swap panels' },
    ],
  },
}

export function WelcomeView({ onNewSession, onOpenTerminal, daemonRunning }: WelcomeViewProps) {
  const [lang, setLang] = useState<Lang>(() => {
    try {
      return (localStorage.getItem('smux-lang') as Lang) || 'ko'
    } catch {
      return 'ko'
    }
  })

  useEffect(() => {
    try {
      localStorage.setItem('smux-lang', lang)
    } catch { /* ignore */ }
  }, [lang])

  const t = content[lang]

  return (
    <div className="flex-1 overflow-y-auto bg-surface-container-lowest">
      <div className="max-w-3xl mx-auto px-8 py-10">
        {/* Language Toggle */}
        <div className="flex justify-end mb-6">
          <div className="flex bg-surface-container-high rounded-sm overflow-hidden border border-outline-variant/20">
            <button
              onClick={() => setLang('ko')}
              className={`px-3 py-1 font-mono text-[11px] transition-colors ${
                lang === 'ko'
                  ? 'bg-primary text-on-primary'
                  : 'text-on-surface-variant hover:bg-surface-container-highest'
              }`}
            >
              한국어
            </button>
            <button
              onClick={() => setLang('en')}
              className={`px-3 py-1 font-mono text-[11px] transition-colors ${
                lang === 'en'
                  ? 'bg-primary text-on-primary'
                  : 'text-on-surface-variant hover:bg-surface-container-highest'
              }`}
            >
              EN
            </button>
          </div>
        </div>

        {/* Hero */}
        <div className="text-center mb-10">
          <h1 className="font-headline text-3xl font-bold text-on-surface tracking-tight mb-2">
            {t.title}
          </h1>
          <p className="text-on-surface-variant text-sm">{t.subtitle}</p>
        </div>

        {/* Quick Actions */}
        <div className="grid grid-cols-2 gap-3 mb-8">
          <button
            onClick={onOpenTerminal}
            className="flex items-center gap-3 px-5 py-4 bg-primary/10 border border-primary/30 rounded-lg hover:bg-primary/20 transition-colors text-left"
          >
            <span className="text-2xl">{'>'}_</span>
            <div>
              <h3 className="font-mono text-[12px] font-bold text-on-surface">{t.openTerminal}</h3>
              <p className="text-[11px] text-on-surface-variant mt-0.5">
                {lang === 'ko' ? '일반 터미널 셸 열기' : 'Open a regular shell terminal'}
              </p>
            </div>
          </button>
          <button
            onClick={onNewSession}
            disabled={!daemonRunning}
            className={`flex items-center gap-3 px-5 py-4 rounded-lg border text-left transition-colors ${
              daemonRunning
                ? 'bg-secondary/10 border-secondary/30 hover:bg-secondary/20'
                : 'bg-surface-container-low border-outline-variant/20 opacity-50 cursor-not-allowed'
            }`}
          >
            <span className="text-2xl">AI</span>
            <div>
              <h3 className="font-mono text-[12px] font-bold text-on-surface">{t.startSession}</h3>
              <p className="text-[11px] text-on-surface-variant mt-0.5">
                {lang === 'ko' ? 'AI 에이전트 핑퐁 세션' : 'AI agent cross-verify session'}
              </p>
            </div>
          </button>
        </div>

        {/* Daemon Status */}
        <div
          className={`flex items-center gap-3 px-4 py-3 rounded-lg border mb-8 ${
            daemonRunning
              ? 'bg-secondary/5 border-secondary/30'
              : 'bg-error/5 border-error/30'
          }`}
        >
          <span
            className={`w-2.5 h-2.5 rounded-full shrink-0 ${
              daemonRunning ? 'bg-secondary animate-pulse' : 'bg-error'
            }`}
          />
          <div>
            <span className="font-mono text-[11px] font-bold uppercase tracking-wider text-on-surface-variant">
              {t.daemonStatus}
            </span>
            <p className="text-[12px] text-on-surface-variant mt-0.5">
              {daemonRunning ? t.daemonRunning : t.daemonStopped}
            </p>
          </div>
          {/* Status only — action buttons are above */}
        </div>

        {/* What is smux */}
        <section className="mb-8">
          <h2 className="font-headline text-lg font-bold text-on-surface mb-2">{t.whatIs}</h2>
          <p className="text-[13px] text-on-surface-variant leading-relaxed">{t.whatIsDesc}</p>
        </section>

        {/* How it works */}
        <section className="mb-8">
          <h2 className="font-headline text-lg font-bold text-on-surface mb-4">{t.howItWorks}</h2>
          <div className="space-y-3">
            {t.steps.map((step) => (
              <div
                key={step.icon}
                className="flex items-start gap-4 px-4 py-3 bg-surface-container-low rounded-lg border border-outline-variant/10"
              >
                <span className="w-7 h-7 rounded-full bg-primary/15 text-primary font-mono text-[13px] font-bold flex items-center justify-center shrink-0 mt-0.5">
                  {step.icon}
                </span>
                <div className="min-w-0">
                  <h3 className="font-mono text-[12px] font-bold text-on-surface">{step.title}</h3>
                  <p className="text-[12px] text-on-surface-variant mt-0.5">{step.desc}</p>
                  {step.code && (
                    <code className="inline-block mt-1.5 px-2 py-0.5 bg-surface-container-highest rounded text-[11px] font-mono text-secondary">
                      $ {step.code}
                    </code>
                  )}
                </div>
              </div>
            ))}
          </div>
        </section>

        {/* Features */}
        <section className="mb-8">
          <h2 className="font-headline text-lg font-bold text-on-surface mb-4">{t.features}</h2>
          <div className="grid grid-cols-2 gap-3">
            {t.featureList.map((f) => (
              <div
                key={f.title}
                className="px-4 py-3 bg-surface-container-low rounded-lg border border-outline-variant/10"
              >
                <div className="flex items-center gap-2 mb-1">
                  <span className="text-base">{f.icon}</span>
                  <h3 className="font-mono text-[11px] font-bold text-on-surface">{f.title}</h3>
                </div>
                <p className="text-[11px] text-on-surface-variant">{f.desc}</p>
              </div>
            ))}
          </div>
        </section>

        {/* Shortcuts */}
        <section className="mb-8">
          <h2 className="font-headline text-lg font-bold text-on-surface mb-4">{t.shortcuts}</h2>
          <div className="grid grid-cols-2 gap-x-6 gap-y-1.5">
            {t.shortcutList.map((s) => (
              <div key={s.key} className="flex items-center gap-2">
                <kbd className="px-1.5 py-0.5 bg-surface-container-high rounded text-[10px] font-mono text-primary border border-outline-variant/20 min-w-[48px] text-center">
                  {s.key}
                </kbd>
                <span className="text-[11px] text-on-surface-variant">{s.desc}</span>
              </div>
            ))}
          </div>
        </section>
      </div>
    </div>
  )
}
