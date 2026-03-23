// TODO: Integrate into AI session setup UI
export interface PanelPreset {
  id: string
  name: string
  description: string
  panels: { role: string; agent: string }[]
  icon: string
}

export const PRESETS: PanelPreset[] = [
  {
    id: 'dual',
    name: 'Dual (Default)',
    description: 'Planner + Verifier',
    panels: [
      { role: 'Planner', agent: 'claude' },
      { role: 'Verifier', agent: 'codex' },
    ],
    icon: 'view_column_2',
  },
  {
    id: 'code-review',
    name: 'Code Review',
    description: 'Planner + Reviewer + Tester',
    panels: [
      { role: 'Planner', agent: 'claude' },
      { role: 'Reviewer', agent: 'codex' },
      { role: 'Tester', agent: 'gemini' },
    ],
    icon: 'view_week',
  },
  {
    id: 'full-pipeline',
    name: 'Full Pipeline',
    description: 'Planner + Reviewer + Tester + Terminal',
    panels: [
      { role: 'Planner', agent: 'claude' },
      { role: 'Reviewer', agent: 'codex' },
      { role: 'Tester', agent: 'gemini' },
      { role: 'Terminal', agent: '' },
    ],
    icon: 'grid_view',
  },
]

interface PanelPresetsProps {
  onSelect: (preset: PanelPreset) => void
  currentPresetId?: string
}

export function PanelPresets({ onSelect, currentPresetId }: PanelPresetsProps) {
  return (
    <div className="flex gap-3">
      {PRESETS.map(preset => {
        const isSelected = preset.id === currentPresetId
        return (
          <button
            key={preset.id}
            onClick={() => onSelect(preset)}
            className={`relative flex flex-col items-center gap-2 px-5 py-4 rounded-xl border transition-all cursor-pointer group ${
              isSelected
                ? 'border-primary bg-primary/10 shadow-[0_0_12px_rgba(var(--primary-rgb),0.15)]'
                : 'border-outline-variant/30 bg-surface-container-low hover:border-primary/50 hover:bg-surface-container'
            }`}
          >
            {/* Icon */}
            <span
              className={`material-symbols-outlined text-2xl transition-transform group-hover:scale-110 ${
                isSelected ? 'text-primary' : 'text-outline'
              }`}
            >
              {preset.icon}
            </span>

            {/* Name */}
            <span
              className={`font-mono text-[12px] font-bold ${
                isSelected ? 'text-on-surface' : 'text-on-surface-variant'
              }`}
            >
              {preset.name}
            </span>

            {/* Description */}
            <span className="font-mono text-[10px] text-outline leading-tight text-center">
              {preset.description}
            </span>

            {/* Panel count badge */}
            <span
              className={`font-mono text-[9px] font-bold px-2 py-0.5 rounded-full ${
                isSelected
                  ? 'bg-primary/20 text-primary'
                  : 'bg-surface-container-high text-outline'
              }`}
            >
              {preset.panels.length} panels
            </span>
          </button>
        )
      })}
    </div>
  )
}
