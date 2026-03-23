import { useState, useEffect, useCallback } from 'react'

interface FileNode {
  name: string
  path: string
  isDir: boolean
  children?: FileNode[]
  expanded?: boolean
}

interface FileExplorerProps {
  rootPath: string
  onFileSelect: (path: string) => void
  onNavigateBack: () => void
  onOpenFolder: () => void
}

declare global {
  interface Window {
    __TAURI_INTERNALS__?: unknown
  }
}

const isTauri = !!window.__TAURI_INTERNALS__

const FILE_ICONS: Record<string, string> = {
  rs: 'code',
  ts: 'javascript',
  tsx: 'javascript',
  js: 'javascript',
  json: 'data_object',
  toml: 'settings',
  md: 'description',
  css: 'palette',
  html: 'html',
  yml: 'settings',
  yaml: 'settings',
  sh: 'terminal',
  zsh: 'terminal',
  lock: 'lock',
  gitignore: 'visibility_off',
}

function getIcon(name: string, isDir: boolean): string {
  if (isDir) return 'folder'
  const ext = name.split('.').pop()?.toLowerCase() || ''
  return FILE_ICONS[ext] || 'draft'
}

export function FileExplorer({ rootPath, onFileSelect, onNavigateBack, onOpenFolder }: FileExplorerProps) {
  const [tree, setTree] = useState<FileNode[]>([])
  const [loading, setLoading] = useState(false)

  const loadDirectory = useCallback(async (dirPath: string): Promise<FileNode[]> => {
    if (!isTauri) return []
    try {
      const { invoke } = await import('@tauri-apps/api/core')
      const entries = await invoke<Array<{ name: string; path: string; is_dir: boolean }>>('list_directory', { path: dirPath })
      return entries
        .filter(e => !e.name.startsWith('.') && e.name !== 'node_modules' && e.name !== 'target' && e.name !== '__pycache__')
        .sort((a, b) => {
          if (a.is_dir !== b.is_dir) return a.is_dir ? -1 : 1
          return a.name.localeCompare(b.name)
        })
        .map(e => ({
          name: e.name,
          path: e.path,
          isDir: e.is_dir,
          expanded: false,
        }))
    } catch {
      return []
    }
  }, [])

  // Load root on mount or path change
  useEffect(() => {
    if (!rootPath) return
    setLoading(true)
    loadDirectory(rootPath).then(nodes => {
      setTree(nodes)
      setLoading(false)
    })
  }, [rootPath, loadDirectory])

  const toggleDir = useCallback(async (node: FileNode) => {
    if (!node.isDir) {
      onFileSelect(node.path)
      return
    }

    if (node.expanded && node.children) {
      // Collapse
      setTree(prev => updateNode(prev, node.path, { ...node, expanded: false }))
    } else {
      // Expand — load children
      const children = await loadDirectory(node.path)
      setTree(prev => updateNode(prev, node.path, { ...node, expanded: true, children }))
    }
  }, [loadDirectory, onFileSelect])

  const projectName = rootPath.split('/').pop() || rootPath

  return (
    <div className="flex flex-col h-full bg-surface-container-low">
      {/* Header with back + open folder */}
      <div className="px-2 py-1.5 border-b border-outline-variant/20 flex items-center justify-between">
        <div className="flex items-center gap-1">
          <button
            onClick={onNavigateBack}
            className="material-symbols-outlined text-[14px] text-outline hover:text-primary transition-colors cursor-pointer"
            aria-label="Back to home"
          >
            arrow_back
          </button>
          <span className="font-mono text-[10px] font-bold uppercase tracking-widest text-outline truncate max-w-[120px]" title={rootPath}>
            {projectName}
          </span>
        </div>
        <button
          onClick={onOpenFolder}
          className="material-symbols-outlined text-[14px] text-outline hover:text-primary transition-colors cursor-pointer"
          aria-label="Open different folder"
        >
          folder_open
        </button>
      </div>

      {/* File tree */}
      <div className="flex-1 overflow-y-auto py-1">
        {loading ? (
          <div className="px-3 py-4 font-mono text-[10px] text-outline text-center">Loading...</div>
        ) : tree.length === 0 ? (
          <div className="px-3 py-4 font-mono text-[10px] text-outline text-center">Empty directory</div>
        ) : (
          tree.map(node => (
            <FileTreeNode key={node.path} node={node} depth={0} onToggle={toggleDir} />
          ))
        )}
      </div>
    </div>
  )
}

function FileTreeNode({ node, depth, onToggle }: { node: FileNode; depth: number; onToggle: (n: FileNode) => void }) {
  return (
    <>
      <button
        onClick={() => onToggle(node)}
        className="flex items-center gap-1 w-full text-left px-2 py-0.5 hover:bg-surface-container-high transition-colors group"
        style={{ paddingLeft: `${8 + depth * 12}px` }}
      >
        {node.isDir && (
          <span className="material-symbols-outlined text-[12px] text-outline transition-transform" style={{ transform: node.expanded ? 'rotate(90deg)' : 'rotate(0)' }}>
            chevron_right
          </span>
        )}
        {!node.isDir && <span className="w-3" />}
        <span className={`material-symbols-outlined text-[14px] ${node.isDir ? 'text-primary/70' : 'text-outline'}`}>
          {getIcon(node.name, node.isDir)}
        </span>
        <span className="font-mono text-[11px] text-on-surface-variant truncate group-hover:text-on-surface">
          {node.name}
        </span>
      </button>
      {node.expanded && node.children?.map(child => (
        <FileTreeNode key={child.path} node={child} depth={depth + 1} onToggle={onToggle} />
      ))}
    </>
  )
}

function updateNode(nodes: FileNode[], path: string, updated: FileNode): FileNode[] {
  return nodes.map(n => {
    if (n.path === path) return updated
    if (n.children) return { ...n, children: updateNode(n.children, path, updated) }
    return n
  })
}
