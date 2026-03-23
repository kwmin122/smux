import { useState, useEffect } from 'react'

interface FileViewerProps {
  filePath: string | null
  onClose: () => void
}

declare global {
  interface Window {
    __TAURI_INTERNALS__?: unknown
  }
}

const isTauri = !!window.__TAURI_INTERNALS__

const LANG_MAP: Record<string, string> = {
  rs: 'rust',
  ts: 'typescript',
  tsx: 'tsx',
  js: 'javascript',
  jsx: 'jsx',
  json: 'json',
  toml: 'toml',
  yaml: 'yaml',
  yml: 'yaml',
  md: 'markdown',
  css: 'css',
  html: 'html',
  sh: 'bash',
  zsh: 'bash',
  py: 'python',
  go: 'go',
}

export function FileViewer({ filePath, onClose }: FileViewerProps) {
  const [content, setContent] = useState<string>('')
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    if (!filePath) return
    setLoading(true)
    setError(null)

    if (isTauri) {
      import('@tauri-apps/api/core').then(({ invoke }) => {
        invoke<string>('read_file', { path: filePath })
          .then(text => {
            setContent(text)
            setLoading(false)
          })
          .catch(e => {
            setError(String(e))
            setLoading(false)
          })
      })
    }
  }, [filePath])

  if (!filePath) return null

  const fileName = filePath.split('/').pop() || filePath
  const ext = fileName.split('.').pop()?.toLowerCase() || ''
  const lang = LANG_MAP[ext] || 'text'
  const lines = content.split('\n')

  return (
    <div className="flex flex-col h-full bg-surface-container-lowest">
      {/* Header */}
      <div className="h-6 bg-surface-container-high px-3 flex items-center justify-between border-b border-outline-variant/20 shrink-0">
        <div className="flex items-center gap-2">
          <span className="material-symbols-outlined text-[14px] text-outline">draft</span>
          <span className="font-mono text-[10px] font-bold text-on-surface-variant truncate max-w-[200px]" title={filePath}>
            {fileName}
          </span>
          <span className="font-mono text-[8px] text-outline px-1 py-0.5 rounded bg-surface-container">
            {lang}
          </span>
          <span className="font-mono text-[8px] text-outline">
            {lines.length} lines
          </span>
        </div>
        <button
          onClick={onClose}
          className="material-symbols-outlined text-[14px] text-outline hover:text-on-surface cursor-pointer"
          aria-label="Close file"
        >
          close
        </button>
      </div>

      {/* Content */}
      <div className="flex-1 overflow-auto">
        {loading ? (
          <div className="p-4 font-mono text-[11px] text-outline">Loading...</div>
        ) : error ? (
          <div className="p-4 font-mono text-[11px] text-error">{error}</div>
        ) : (
          <div className="flex">
            {/* Line numbers */}
            <div className="shrink-0 py-2 pr-2 text-right select-none border-r border-outline-variant/10 bg-surface-container-low">
              {lines.map((_, i) => (
                <div key={i} className="font-mono text-[11px] leading-[1.4] text-outline/50 px-2">
                  {i + 1}
                </div>
              ))}
            </div>
            {/* Code */}
            <pre className="flex-1 py-2 px-3 overflow-x-auto">
              <code className="font-mono text-[11px] leading-[1.4] text-on-surface-variant whitespace-pre">
                {content}
              </code>
            </pre>
          </div>
        )}
      </div>
    </div>
  )
}
