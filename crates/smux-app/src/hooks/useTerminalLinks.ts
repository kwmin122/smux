import { useCallback, useRef } from 'react'
import type { Terminal, IDisposable, ILink, ILinkProvider } from '@xterm/xterm'
import { WebLinksAddon } from '@xterm/addon-web-links'

// ---------------------------------------------------------------------------
// File-path patterns
// ---------------------------------------------------------------------------

interface FileMatch {
  file: string
  line: number
  col: number
  /** Start index within the source line text */
  start: number
  /** End index (exclusive) within the source line text */
  end: number
}

/**
 * Each entry: [regex, file-group, line-group, col-group].
 * col-group can be 0 (no column captured) — defaults to 1.
 */
const FILE_PATTERNS: [RegExp, number, number, number][] = [
  // Rust:  --> src/lib.rs:10:5
  [/-->\s+([\w./-]+):(\d+):(\d+)/g, 1, 2, 3],

  // TypeScript / ESLint:  src/app.ts(10,5)
  [/([\w./-]+)\((\d+),\s*(\d+)\)/g, 1, 2, 3],

  // Python:  File "foo.py", line 10
  [/File\s+"([^"]+)",\s*line\s+(\d+)/g, 1, 2, 0],

  // General:  path/to/file.ext:line:col  or  path/to/file.ext:line
  [/([\w./-]+\.[\w]+):(\d+)(?::(\d+))?/g, 1, 2, 3],
]

function findFileMatches(text: string): FileMatch[] {
  const matches: FileMatch[] = []

  for (const [pattern, fileIdx, lineIdx, colIdx] of FILE_PATTERNS) {
    // Reset lastIndex for global regex reuse
    pattern.lastIndex = 0
    let m: RegExpExecArray | null

    while ((m = pattern.exec(text)) !== null) {
      const file = m[fileIdx]
      const line = parseInt(m[lineIdx], 10)
      const col = colIdx && m[colIdx] ? parseInt(m[colIdx], 10) : 1

      // Sanity: skip if file has no extension or line is unreasonable
      if (!/\.\w+$/.test(file) || line <= 0) continue

      matches.push({
        file,
        line,
        col,
        start: m.index,
        end: m.index + m[0].length,
      })
    }
  }

  return matches
}

// ---------------------------------------------------------------------------
// URL opener — uses Tauri shell.open when available, window.open otherwise
// ---------------------------------------------------------------------------

function openUrl(url: string): void {
  window.open(url, '_blank', 'noopener')
}

// ---------------------------------------------------------------------------
// File-link opener — emits a Tauri event so the Rust side (or another
// component) can handle "open in editor" logic.
// ---------------------------------------------------------------------------

async function openFileLink(file: string, line: number, col: number): Promise<void> {
  try {
    const { emit } = await import('@tauri-apps/api/event')
    await emit('open-file-link', { file, line, col })
  } catch {
    // Non-Tauri environment — no-op or log for debugging
    console.debug('[useTerminalLinks] open-file-link:', { file, line, col })
  }
}

// ---------------------------------------------------------------------------
// Custom link provider for file paths
// ---------------------------------------------------------------------------

function createFileLinkProviderForTerminal(terminal: Terminal): ILinkProvider {
  return {
    provideLinks(bufferLineNumber: number, callback: (links: ILink[] | undefined) => void): void {
      const buffer = terminal.buffer.active
      const line = buffer.getLine(bufferLineNumber - 1)
      if (!line) {
        callback(undefined)
        return
      }

      const text = line.translateToString(true)
      const matches = findFileMatches(text)

      if (matches.length === 0) {
        callback(undefined)
        return
      }

      const links: ILink[] = matches.map((m) => ({
        range: {
          start: { x: m.start + 1, y: bufferLineNumber },
          end: { x: m.end + 1, y: bufferLineNumber },
        },
        text: text.substring(m.start, m.end),
        activate() {
          openFileLink(m.file, m.line, m.col)
        },
      }))

      callback(links)
    },
  }
}

// ---------------------------------------------------------------------------
// Hook
// ---------------------------------------------------------------------------

export function useTerminalLinks() {
  const disposablesRef = useRef<IDisposable[]>([])

  const attach = useCallback((terminal: Terminal) => {
    // Clean up previous attachments
    disposablesRef.current.forEach((d) => d.dispose())
    disposablesRef.current = []

    // 1. Web links addon — clickable http(s) URLs
    const webLinksAddon = new WebLinksAddon((_event: MouseEvent, url: string) => {
      openUrl(url)
    })
    terminal.loadAddon(webLinksAddon)
    disposablesRef.current.push(webLinksAddon)

    // 2. Custom file-link provider
    const fileLinkProvider = createFileLinkProviderForTerminal(terminal)
    const providerDisposable = terminal.registerLinkProvider(fileLinkProvider)
    disposablesRef.current.push(providerDisposable)
  }, [])

  const detach = useCallback(() => {
    disposablesRef.current.forEach((d) => d.dispose())
    disposablesRef.current = []
  }, [])

  return { attach, detach }
}
