import { useState } from 'react'

// --- Types ---

export interface LaunchPane {
  command: string
  name: string
  color: string  // 'default' | 'red' | 'green' | 'blue' | 'yellow'
}

export interface LaunchConfig {
  id: string
  name: string
  layout: 'single' | '2-split-vertical' | '2-split-horizontal' | '3-split' | '4-grid'
  panes: LaunchPane[]
}

interface LaunchConfigsProps {
  configs: LaunchConfig[]
  onLaunch: (config: LaunchConfig) => void
  onSave: (config: LaunchConfig) => void
  onDelete: (id: string) => void
}

// --- Built-in Defaults ---

export const DEFAULT_CONFIGS: LaunchConfig[] = [
  {
    id: 'dev',
    name: 'Dev Environment',
    layout: '3-split',
    panes: [
      { command: 'npm run dev', name: 'Dev Server', color: 'green' },
      { command: 'npm run test:watch', name: 'Tests', color: 'yellow' },
      { command: '', name: 'Terminal', color: 'default' },
    ],
  },
  {
    id: 'fullstack',
    name: 'Full-Stack',
    layout: '4-grid',
    panes: [
      { command: 'npm run dev', name: 'Frontend', color: 'blue' },
      { command: 'cargo watch -x run', name: 'Backend', color: 'green' },
      { command: 'npm run test:watch', name: 'Tests', color: 'yellow' },
      { command: '', name: 'Terminal', color: 'default' },
    ],
  },
]

const DEFAULT_IDS = new Set(DEFAULT_CONFIGS.map(c => c.id))

// --- Layout Icons (SVG) ---

function LayoutIcon({ layout }: { layout: LaunchConfig['layout'] }) {
  const cls = 'w-8 h-6 rounded border border-outline-variant/30 overflow-hidden flex'

  switch (layout) {
    case 'single':
      return (
        <div className={cls}>
          <div className="flex-1 bg-primary/20" />
        </div>
      )
    case '2-split-vertical':
      return (
        <div className={cls}>
          <div className="flex-1 bg-primary/20 border-r border-outline-variant/30" />
          <div className="flex-1 bg-primary/10" />
        </div>
      )
    case '2-split-horizontal':
      return (
        <div className={`${cls} flex-col`}>
          <div className="flex-1 bg-primary/20 border-b border-outline-variant/30" />
          <div className="flex-1 bg-primary/10" />
        </div>
      )
    case '3-split':
      return (
        <div className={cls}>
          <div className="flex-1 bg-primary/20 border-r border-outline-variant/30" />
          <div className="flex-1 flex flex-col">
            <div className="flex-1 bg-primary/15 border-b border-outline-variant/30" />
            <div className="flex-1 bg-primary/10" />
          </div>
        </div>
      )
    case '4-grid':
      return (
        <div className={`${cls} flex-wrap`}>
          <div className="w-1/2 h-1/2 bg-primary/20 border-r border-b border-outline-variant/30" />
          <div className="w-1/2 h-1/2 bg-primary/15 border-b border-outline-variant/30" />
          <div className="w-1/2 h-1/2 bg-primary/10 border-r border-outline-variant/30" />
          <div className="w-1/2 h-1/2 bg-primary/5" />
        </div>
      )
  }
}

// --- Pane Color Dot ---

const PANE_COLOR_MAP: Record<string, string> = {
  default: 'bg-outline/50',
  red: 'bg-error',
  green: 'bg-secondary',
  blue: 'bg-primary',
  yellow: 'bg-tertiary',
}

function PaneColorDot({ color }: { color: string }) {
  return <span className={`w-1.5 h-1.5 rounded-full shrink-0 ${PANE_COLOR_MAP[color] || PANE_COLOR_MAP.default}`} />
}

// --- Main Component ---

export function LaunchConfigs({ configs, onLaunch, onSave, onDelete }: LaunchConfigsProps) {
  const [editing, setEditing] = useState<LaunchConfig | null>(null)

  function handleNewConfig() {
    setEditing({
      id: `custom-${Date.now()}`,
      name: '',
      layout: 'single',
      panes: [{ command: '', name: 'Terminal', color: 'default' }],
    })
  }

  function handleSaveEdit() {
    if (!editing || !editing.name.trim()) return
    onSave(editing)
    setEditing(null)
  }

  function handleCancelEdit() {
    setEditing(null)
  }

  return (
    <section>
      <div className="flex items-center justify-between mb-3">
        <h2 className="font-mono text-[11px] font-bold uppercase tracking-widest text-outline">Launch Configs</h2>
        <button
          onClick={handleNewConfig}
          className="flex items-center gap-1 px-2 py-1 rounded text-[10px] font-mono text-primary hover:bg-primary/10 transition-colors"
        >
          <span className="material-symbols-outlined text-[14px]">add</span>
          New
        </button>
      </div>

      {/* Config List */}
      <div className="space-y-1.5">
        {configs.map(config => (
          <div
            key={config.id}
            className="flex items-center gap-3 px-3 py-2.5 rounded-lg bg-surface-container-low border border-outline-variant/10 hover:border-outline-variant/30 transition-colors group"
          >
            {/* Layout Icon */}
            <LayoutIcon layout={config.layout} />

            {/* Info */}
            <div className="flex-1 min-w-0">
              <div className="font-mono text-[12px] text-on-surface font-medium truncate">{config.name}</div>
              <div className="flex items-center gap-2 mt-0.5">
                <span className="font-mono text-[10px] text-outline">{config.layout}</span>
                <span className="text-outline/30">·</span>
                <div className="flex items-center gap-1">
                  {config.panes.map((pane, i) => (
                    <span key={i} className="flex items-center gap-0.5" title={`${pane.name}${pane.command ? `: ${pane.command}` : ''}`}>
                      <PaneColorDot color={pane.color} />
                    </span>
                  ))}
                  <span className="font-mono text-[10px] text-outline ml-0.5">{config.panes.length} pane{config.panes.length !== 1 ? 's' : ''}</span>
                </div>
              </div>
            </div>

            {/* Actions */}
            <div className="flex items-center gap-1 shrink-0">
              <button
                onClick={() => onLaunch(config)}
                className="flex items-center gap-1 px-2.5 py-1 rounded bg-primary/10 text-primary font-mono text-[10px] font-bold hover:bg-primary/20 transition-colors"
              >
                <span className="material-symbols-outlined text-[14px]">play_arrow</span>
                Launch
              </button>
              {!DEFAULT_IDS.has(config.id) && (
                <button
                  onClick={() => onDelete(config.id)}
                  className="flex items-center px-1.5 py-1 rounded text-outline hover:text-error hover:bg-error/10 transition-colors opacity-0 group-hover:opacity-100"
                  title="Delete config"
                >
                  <span className="material-symbols-outlined text-[14px]">delete</span>
                </button>
              )}
            </div>
          </div>
        ))}
      </div>

      {configs.length === 0 && (
        <p className="text-[12px] text-on-surface-variant/50 italic py-2">No launch configs. Click "New" to create one.</p>
      )}

      {/* Inline Editor */}
      {editing && (
        <div className="mt-3 p-4 rounded-lg bg-surface-container-high border border-outline-variant/30">
          <h3 className="font-mono text-[11px] font-bold text-on-surface mb-3">New Launch Config</h3>

          {/* Name */}
          <div className="mb-3">
            <label className="font-mono text-[10px] text-outline block mb-1">Name</label>
            <input
              type="text"
              value={editing.name}
              onChange={e => setEditing({ ...editing, name: e.target.value })}
              placeholder="My Workspace"
              className="w-full h-7 bg-surface-container-lowest border border-outline-variant/30 rounded px-2 font-mono text-[12px] text-on-surface outline-none focus:border-primary placeholder:text-outline/40"
              autoFocus
            />
          </div>

          {/* Layout */}
          <div className="mb-3">
            <label className="font-mono text-[10px] text-outline block mb-1">Layout</label>
            <div className="flex gap-2">
              {(['single', '2-split-vertical', '2-split-horizontal', '3-split', '4-grid'] as const).map(layout => (
                <button
                  key={layout}
                  onClick={() => {
                    const paneCount = layout === 'single' ? 1
                      : layout === '3-split' ? 3
                      : layout === '4-grid' ? 4
                      : 2
                    const panes: LaunchPane[] = Array.from({ length: paneCount }, (_, i) =>
                      editing.panes[i] || { command: '', name: `Pane ${i + 1}`, color: 'default' }
                    )
                    setEditing({ ...editing, layout, panes })
                  }}
                  className={`p-1.5 rounded border transition-colors ${
                    editing.layout === layout
                      ? 'border-primary bg-primary/10'
                      : 'border-outline-variant/20 hover:border-outline-variant/40'
                  }`}
                  title={layout}
                >
                  <LayoutIcon layout={layout} />
                </button>
              ))}
            </div>
          </div>

          {/* Panes */}
          <div className="mb-4">
            <label className="font-mono text-[10px] text-outline block mb-1">Panes</label>
            <div className="space-y-1.5">
              {editing.panes.map((pane, i) => (
                <div key={i} className="flex items-center gap-2">
                  <select
                    value={pane.color}
                    onChange={e => {
                      const panes = [...editing.panes]
                      panes[i] = { ...pane, color: e.target.value }
                      setEditing({ ...editing, panes })
                    }}
                    className="w-20 h-7 bg-surface-container-lowest border border-outline-variant/30 rounded px-1 font-mono text-[10px] text-on-surface-variant outline-none focus:border-primary"
                  >
                    {['default', 'red', 'green', 'blue', 'yellow'].map(c => (
                      <option key={c} value={c}>{c}</option>
                    ))}
                  </select>
                  <input
                    type="text"
                    value={pane.name}
                    onChange={e => {
                      const panes = [...editing.panes]
                      panes[i] = { ...pane, name: e.target.value }
                      setEditing({ ...editing, panes })
                    }}
                    placeholder="Name"
                    className="w-28 h-7 bg-surface-container-lowest border border-outline-variant/30 rounded px-2 font-mono text-[11px] text-on-surface outline-none focus:border-primary placeholder:text-outline/40"
                  />
                  <input
                    type="text"
                    value={pane.command}
                    onChange={e => {
                      const panes = [...editing.panes]
                      panes[i] = { ...pane, command: e.target.value }
                      setEditing({ ...editing, panes })
                    }}
                    placeholder="command (empty = shell)"
                    className="flex-1 h-7 bg-surface-container-lowest border border-outline-variant/30 rounded px-2 font-mono text-[11px] text-on-surface outline-none focus:border-primary placeholder:text-outline/40"
                  />
                </div>
              ))}
            </div>
          </div>

          {/* Actions */}
          <div className="flex items-center justify-end gap-2">
            <button
              onClick={handleCancelEdit}
              className="px-3 py-1.5 rounded font-mono text-[11px] text-on-surface-variant hover:bg-surface-container-highest transition-colors"
            >
              Cancel
            </button>
            <button
              onClick={handleSaveEdit}
              disabled={!editing.name.trim()}
              className={`px-3 py-1.5 rounded font-mono text-[11px] font-bold transition-colors ${
                editing.name.trim()
                  ? 'bg-primary text-on-primary hover:bg-primary/90'
                  : 'bg-outline/20 text-outline cursor-not-allowed'
              }`}
            >
              Save
            </button>
          </div>
        </div>
      )}
    </section>
  )
}
