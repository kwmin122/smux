import { useState, useEffect } from 'react'
import { AiExecutionLevel, type ExecutionLevel } from './AiExecutionLevel'
import { useKeybindings, type KeyPreset } from '../hooks/useKeybindings'

declare global {
  interface Window {
    __TAURI_INTERNALS__?: unknown
  }
}

const isTauri = !!window.__TAURI_INTERNALS__

interface AppConfig {
  general: { shell: string; scrollback: number }
  appearance: { font_family: string; font_size: number; theme: string; cursor_style: string; cursor_blink: boolean; minimum_contrast_ratio: number }
  ai: { auto_execution_level: string; allow_commands: string[]; deny_commands: string[]; max_rounds: number; default_planner: string; default_verifier: string }
}

type SettingsCategory = 'general' | 'appearance' | 'terminal' | 'ai' | 'keybindings'

const CATEGORIES: { id: SettingsCategory; label: string; icon: string }[] = [
  { id: 'general', label: 'General', icon: 'tune' },
  { id: 'appearance', label: 'Appearance', icon: 'palette' },
  { id: 'terminal', label: 'Terminal', icon: 'terminal' },
  { id: 'ai', label: 'AI', icon: 'psychology' },
  { id: 'keybindings', label: 'Keybindings', icon: 'keyboard' },
]

const THEMES = ['deep-navy', 'amber', 'forest-green']
const FONTS = ['JetBrains Mono', 'SF Mono', 'Menlo', 'Fira Code', 'Cascadia Code', 'Monaco']
const CURSOR_STYLES = ['block', 'underline', 'bar']
const AGENTS = ['claude', 'codex', 'gemini']

interface SettingsViewProps {
  onClose: () => void
  theme: string
  onThemeChange: (theme: string) => void
  executionLevel: ExecutionLevel
  onExecutionLevelChange: (level: ExecutionLevel) => void
}

export function SettingsView({ onClose, theme, onThemeChange, executionLevel, onExecutionLevelChange }: SettingsViewProps) {
  const [category, setCategory] = useState<SettingsCategory>('general')
  const [config, setConfig] = useState<AppConfig | null>(null)
  const { bindings, preset, resetToPreset } = useKeybindings()

  // Load config on mount
  useEffect(() => {
    if (isTauri) {
      import('@tauri-apps/api/core').then(({ invoke }) => {
        invoke<AppConfig>('load_app_config').then(setConfig).catch(() => {})
      })
    }
  }, [])

  async function saveField(path: string, value: unknown) {
    if (!config || !isTauri) return
    // Deep update config
    const parts = path.split('.')
    const updated = JSON.parse(JSON.stringify(config))
    let obj = updated as Record<string, unknown>
    for (let i = 0; i < parts.length - 1; i++) {
      obj = obj[parts[i]] as Record<string, unknown>
    }
    obj[parts[parts.length - 1]] = value
    setConfig(updated)
    try {
      const { invoke } = await import('@tauri-apps/api/core')
      await invoke('save_app_config', { config: updated })
    } catch { /* ignore */ }
  }

  // ESC to close
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose()
    }
    window.addEventListener('keydown', handler)
    return () => window.removeEventListener('keydown', handler)
  }, [onClose])

  return (
    <div className="fixed inset-0 z-[100] flex bg-surface-container-lowest">
      {/* Left: Category List */}
      <nav className="w-48 bg-surface-container-low border-r border-outline-variant/20 py-4">
        <div className="px-4 mb-4 flex items-center justify-between">
          <h2 className="font-headline text-sm font-bold text-on-surface">Settings</h2>
          <button onClick={onClose} className="material-symbols-outlined text-[16px] text-outline hover:text-on-surface cursor-pointer" aria-label="Close settings">close</button>
        </div>
        <div className="space-y-0.5 px-2">
          {CATEGORIES.map(c => (
            <button
              key={c.id}
              onClick={() => setCategory(c.id)}
              className={`w-full flex items-center gap-2 px-3 py-2 rounded text-left transition-colors ${
                category === c.id ? 'bg-primary/10 text-primary' : 'text-on-surface-variant hover:bg-surface-container-high'
              }`}
            >
              <span className="material-symbols-outlined text-[16px]">{c.icon}</span>
              <span className="font-mono text-[11px]">{c.label}</span>
            </button>
          ))}
        </div>
      </nav>

      {/* Right: Settings Content */}
      <main className="flex-1 overflow-y-auto py-6 px-8 max-w-2xl">
        {category === 'general' && (
          <div className="space-y-6">
            <h3 className="font-headline text-lg font-bold text-on-surface">General</h3>
            <SettingRow label="Shell" description="Default shell for new terminal tabs">
              <div className="font-mono text-[12px] text-on-surface-variant">{config?.general.shell || '/bin/zsh'}</div>
            </SettingRow>
            <SettingRow label="Scrollback" description="Number of lines to keep in terminal history">
              <input
                type="number"
                value={config?.general.scrollback ?? 10000}
                onChange={e => saveField('general.scrollback', Number(e.target.value))}
                className="w-24 h-7 bg-surface-container-lowest border border-outline-variant/30 rounded px-2 font-mono text-[12px] text-on-surface text-right outline-none focus:border-primary"
              />
            </SettingRow>
          </div>
        )}

        {category === 'appearance' && (
          <div className="space-y-6">
            <h3 className="font-headline text-lg font-bold text-on-surface">Appearance</h3>
            <SettingRow label="Theme" description="Color scheme for the application">
              <div className="flex gap-2">
                {THEMES.map(t => (
                  <button
                    key={t}
                    onClick={() => { onThemeChange(t); saveField('appearance.theme', t) }}
                    className={`px-3 py-1.5 rounded font-mono text-[11px] border transition-colors ${
                      theme === t ? 'bg-primary text-on-primary border-primary' : 'border-outline-variant/30 text-on-surface-variant hover:border-primary'
                    }`}
                  >{t}</button>
                ))}
              </div>
            </SettingRow>
            <SettingRow label="Font Family" description="Monospace font for terminal text">
              <select
                value={config?.appearance.font_family ?? 'JetBrains Mono'}
                onChange={e => saveField('appearance.font_family', e.target.value)}
                className="w-48 h-7 bg-surface-container-lowest border border-outline-variant/30 rounded px-2 font-mono text-[12px] text-on-surface-variant outline-none focus:border-primary"
              >
                {FONTS.map(f => <option key={f} value={f}>{f}</option>)}
              </select>
            </SettingRow>
            <SettingRow label="Font Size" description={`${config?.appearance.font_size ?? 14}px`}>
              <input
                type="range" min={10} max={24}
                value={config?.appearance.font_size ?? 14}
                onChange={e => saveField('appearance.font_size', Number(e.target.value))}
                className="w-48 accent-primary"
              />
            </SettingRow>
            <SettingRow label="Cursor Style" description="Terminal cursor appearance">
              <div className="flex gap-2">
                {CURSOR_STYLES.map(s => (
                  <button
                    key={s}
                    onClick={() => saveField('appearance.cursor_style', s)}
                    className={`px-3 py-1 rounded font-mono text-[11px] border transition-colors ${
                      (config?.appearance.cursor_style ?? 'block') === s
                        ? 'bg-primary text-on-primary border-primary'
                        : 'border-outline-variant/30 text-on-surface-variant hover:border-primary'
                    }`}
                  >{s}</button>
                ))}
              </div>
            </SettingRow>
            <SettingRow label="Cursor Blink" description="Animate the terminal cursor">
              <ToggleSwitch
                checked={config?.appearance.cursor_blink ?? true}
                onChange={v => saveField('appearance.cursor_blink', v)}
              />
            </SettingRow>
          </div>
        )}

        {category === 'terminal' && (
          <div className="space-y-6">
            <h3 className="font-headline text-lg font-bold text-on-surface">Terminal</h3>
            <SettingRow label="Minimum Contrast Ratio" description="WCAG accessibility contrast (1.0 - 21.0)">
              <input
                type="number" min={1} max={21} step={0.5}
                value={config?.appearance.minimum_contrast_ratio ?? 4.5}
                onChange={e => saveField('appearance.minimum_contrast_ratio', Number(e.target.value))}
                className="w-20 h-7 bg-surface-container-lowest border border-outline-variant/30 rounded px-2 font-mono text-[12px] text-on-surface text-right outline-none focus:border-primary"
              />
            </SettingRow>
          </div>
        )}

        {category === 'ai' && (
          <div className="space-y-6">
            <h3 className="font-headline text-lg font-bold text-on-surface">AI Configuration</h3>
            <SettingRow label="Execution Level" description="How much autonomy AI agents have">
              <AiExecutionLevel level={executionLevel} onChange={onExecutionLevelChange} />
            </SettingRow>
            <SettingRow label="Default Planner" description="AI agent for code generation">
              <select
                value={config?.ai.default_planner ?? 'claude'}
                onChange={e => saveField('ai.default_planner', e.target.value)}
                className="w-32 h-7 bg-surface-container-lowest border border-outline-variant/30 rounded px-2 font-mono text-[12px] text-on-surface-variant outline-none focus:border-primary"
              >
                {AGENTS.map(a => <option key={a} value={a}>{a}</option>)}
              </select>
            </SettingRow>
            <SettingRow label="Default Verifier" description="AI agent for code review">
              <select
                value={config?.ai.default_verifier ?? 'codex'}
                onChange={e => saveField('ai.default_verifier', e.target.value)}
                className="w-32 h-7 bg-surface-container-lowest border border-outline-variant/30 rounded px-2 font-mono text-[12px] text-on-surface-variant outline-none focus:border-primary"
              >
                {AGENTS.map(a => <option key={a} value={a}>{a}</option>)}
              </select>
            </SettingRow>
            <SettingRow label="Max Rounds" description="Maximum ping-pong iterations">
              <input
                type="number" min={1} max={50}
                value={config?.ai.max_rounds ?? 5}
                onChange={e => saveField('ai.max_rounds', Number(e.target.value))}
                className="w-20 h-7 bg-surface-container-lowest border border-outline-variant/30 rounded px-2 font-mono text-[12px] text-on-surface text-right outline-none focus:border-primary"
              />
            </SettingRow>
          </div>
        )}

        {category === 'keybindings' && (
          <div className="space-y-6">
            <h3 className="font-headline text-lg font-bold text-on-surface">Keybindings</h3>

            {/* Preset selector */}
            <SettingRow label="Preset" description="Switch between keybinding presets">
              <div className="flex gap-2">
                {(['default', 'tmux', 'vim'] as KeyPreset[]).map(p => (
                  <button
                    key={p}
                    onClick={() => resetToPreset(p)}
                    className={`px-3 py-1.5 rounded font-mono text-[11px] border transition-colors ${
                      preset === p ? 'bg-primary text-on-primary border-primary' : 'border-outline-variant/30 text-on-surface-variant hover:border-primary'
                    }`}
                  >{p}</button>
                ))}
              </div>
            </SettingRow>

            {/* Binding rows */}
            <div className="space-y-2">
              {bindings.map(binding => (
                <div key={binding.id} className="flex items-center justify-between py-1.5 border-b border-outline-variant/10">
                  <span className="font-mono text-[12px] text-on-surface-variant">{binding.action}</span>
                  <div className="flex items-center gap-2">
                    <kbd className="font-mono text-[11px] px-2 py-0.5 rounded bg-surface-container-high border border-outline-variant/20 text-on-surface">
                      {binding.customKey || binding.defaultKey}
                    </kbd>
                    <span className="material-symbols-outlined text-[14px] text-outline/40 hover:text-primary cursor-pointer" title="Edit binding">edit</span>
                  </div>
                </div>
              ))}
            </div>
          </div>
        )}
      </main>
    </div>
  )
}

// --- Helper Components ---

function SettingRow({ label, description, children }: { label: string; description: string; children: React.ReactNode }) {
  return (
    <div className="flex items-start justify-between gap-4 py-2 border-b border-outline-variant/10">
      <div>
        <div className="font-mono text-[12px] text-on-surface">{label}</div>
        <div className="font-mono text-[10px] text-outline mt-0.5">{description}</div>
      </div>
      <div className="shrink-0">{children}</div>
    </div>
  )
}

function ToggleSwitch({ checked, onChange }: { checked: boolean; onChange: (v: boolean) => void }) {
  return (
    <button
      onClick={() => onChange(!checked)}
      className={`w-10 h-5 rounded-full relative transition-colors ${
        checked ? 'bg-primary' : 'bg-outline/30'
      }`}
      role="switch"
      aria-checked={checked}
      aria-label="Toggle"
    >
      <div className={`w-4 h-4 rounded-full bg-white shadow absolute top-0.5 transition-transform ${
        checked ? 'translate-x-5' : 'translate-x-0.5'
      }`} />
    </button>
  )
}
