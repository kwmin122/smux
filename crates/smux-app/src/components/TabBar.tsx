import { useState, useRef, useCallback } from 'react'

export interface TabInfo {
  id: string
  name: string
  cwd: string
  color: TabColor
  icon: string
  isActive: boolean
  status: 'running' | 'idle' | 'exited'
}

export type TabColor = 'default' | 'red' | 'green' | 'blue' | 'yellow' | 'purple' | 'cyan' | 'orange'

const TAB_COLORS: Record<TabColor, string> = {
  default: 'border-primary',
  red: 'border-red-400',
  green: 'border-green-400',
  blue: 'border-blue-400',
  yellow: 'border-yellow-400',
  purple: 'border-purple-400',
  cyan: 'border-cyan-400',
  orange: 'border-orange-400',
}

const TAB_COLOR_OPTIONS: TabColor[] = ['default', 'red', 'green', 'blue', 'yellow', 'purple', 'cyan', 'orange']

interface TabBarProps {
  tabs: TabInfo[]
  onSelectTab: (id: string) => void
  onCloseTab: (id: string) => void
  onNewTab: () => void
  onRenameTab: (id: string, name: string) => void
  onChangeColor: (id: string, color: TabColor) => void
  onReorder: (fromId: string, toId: string) => void
}

export function TabBar({ tabs, onSelectTab, onCloseTab, onNewTab, onRenameTab, onChangeColor, onReorder }: TabBarProps) {
  const [contextMenu, setContextMenu] = useState<{ id: string; x: number; y: number } | null>(null)
  const [editingId, setEditingId] = useState<string | null>(null)
  const [editValue, setEditValue] = useState('')
  const dragRef = useRef<string | null>(null)

  const handleContextMenu = useCallback((e: React.MouseEvent, id: string) => {
    e.preventDefault()
    setContextMenu({ id, x: e.clientX, y: e.clientY })
  }, [])

  const handleRename = useCallback((id: string) => {
    const tab = tabs.find(t => t.id === id)
    if (tab) {
      setEditingId(id)
      setEditValue(tab.name)
    }
    setContextMenu(null)
  }, [tabs])

  const commitRename = useCallback(() => {
    if (editingId && editValue.trim()) {
      onRenameTab(editingId, editValue.trim())
    }
    setEditingId(null)
  }, [editingId, editValue, onRenameTab])

  const handleDragStart = useCallback((id: string) => {
    dragRef.current = id
  }, [])

  const lastDragTargetRef = useRef<string | null>(null)
  const handleDragOver = useCallback((e: React.DragEvent, targetId: string) => {
    e.preventDefault()
    if (dragRef.current && dragRef.current !== targetId && lastDragTargetRef.current !== targetId) {
      lastDragTargetRef.current = targetId
      onReorder(dragRef.current, targetId)
    }
  }, [onReorder])

  return (
    <>
      <div className="px-2 py-1.5 border-b border-outline-variant/20">
        <div className="flex items-center justify-between mb-1">
          <span className="text-[10px] font-mono uppercase tracking-widest text-outline">Terminals</span>
          <button
            onClick={onNewTab}
            className="material-symbols-outlined text-[14px] text-outline hover:text-primary transition-colors cursor-pointer"
            title="New Terminal (⌘T)"
            aria-label="New tab"
          >
            add
          </button>
        </div>
        <div className="space-y-0.5" role="tablist">
          {tabs.map(tab => (
            <div
              key={tab.id}
              role="tab"
              aria-selected={tab.isActive}
              tabIndex={tab.isActive ? 0 : -1}
              draggable
              onDragStart={() => handleDragStart(tab.id)}
              onDragOver={(e) => handleDragOver(e, tab.id)}
              onClick={() => onSelectTab(tab.id)}
              onContextMenu={(e) => handleContextMenu(e, tab.id)}
              className={`flex items-center gap-2 px-2 py-1.5 rounded-sm cursor-pointer group transition-colors border-l-2 ${
                tab.isActive
                  ? `bg-primary/10 ${TAB_COLORS[tab.color]}`
                  : 'border-transparent hover:bg-surface-container-high'
              }`}
            >
              <span className="material-symbols-outlined text-[14px] text-on-surface-variant">{tab.icon}</span>
              <div className="flex-1 min-w-0">
                {editingId === tab.id ? (
                  <input
                    autoFocus
                    value={editValue}
                    onChange={e => setEditValue(e.target.value)}
                    onBlur={commitRename}
                    onKeyDown={e => {
                      if (e.key === 'Enter') commitRename()
                      if (e.key === 'Escape') setEditingId(null)
                    }}
                    className="w-full bg-transparent font-mono text-[10px] text-on-surface outline-none border-b border-primary"
                  />
                ) : (
                  <div className="font-mono text-[10px] text-on-surface-variant truncate">{tab.name}</div>
                )}
                <div className="font-mono text-[8px] text-outline truncate">{tab.cwd || '~'}</div>
              </div>
              <div className="flex items-center gap-1">
                <span className={`w-1.5 h-1.5 rounded-full ${
                  tab.status === 'running' ? 'bg-secondary animate-pulse' :
                  tab.status === 'exited' ? 'bg-outline' : 'bg-secondary/50'
                }`} />
                <button
                  onClick={(e) => { e.stopPropagation(); onCloseTab(tab.id) }}
                  className="material-symbols-outlined text-[12px] text-outline hover:text-error opacity-0 group-hover:opacity-100 transition-opacity"
                  aria-label="Close tab"
                >
                  close
                </button>
              </div>
            </div>
          ))}
        </div>
      </div>

      {/* Context Menu */}
      {contextMenu && (
        <>
          <div className="fixed inset-0 z-[100]" onClick={() => setContextMenu(null)} />
          <div
            className="fixed z-[101] bg-surface-container border border-outline-variant/30 rounded shadow-lg py-1 min-w-[140px]"
            style={{ left: contextMenu.x, top: contextMenu.y }}
          >
            <button
              onClick={() => handleRename(contextMenu.id)}
              className="w-full text-left px-3 py-1 font-mono text-[11px] text-on-surface-variant hover:bg-surface-container-high"
            >
              Rename
            </button>
            <div className="px-3 py-1">
              <div className="font-mono text-[9px] text-outline mb-1">Color</div>
              <div className="flex gap-1">
                {TAB_COLOR_OPTIONS.map(c => (
                  <button
                    key={c}
                    onClick={() => { onChangeColor(contextMenu.id, c); setContextMenu(null) }}
                    className={`w-3 h-3 rounded-full border ${TAB_COLORS[c]} ${
                      c === 'default' ? 'bg-primary/50' :
                      c === 'red' ? 'bg-red-400' :
                      c === 'green' ? 'bg-green-400' :
                      c === 'blue' ? 'bg-blue-400' :
                      c === 'yellow' ? 'bg-yellow-400' :
                      c === 'purple' ? 'bg-purple-400' :
                      c === 'cyan' ? 'bg-cyan-400' :
                      'bg-orange-400'
                    }`}
                    aria-label={`Set tab color to ${c}`}
                  />
                ))}
              </div>
            </div>
            <div className="border-t border-outline-variant/20 my-1" />
            <button
              onClick={() => { onCloseTab(contextMenu.id); setContextMenu(null) }}
              className="w-full text-left px-3 py-1 font-mono text-[11px] text-error hover:bg-surface-container-high"
            >
              Close
            </button>
          </div>
        </>
      )}
    </>
  )
}
