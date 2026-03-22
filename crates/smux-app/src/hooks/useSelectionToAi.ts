import { useCallback, useEffect, useState } from 'react'
import type { Terminal } from '@xterm/xterm'

/**
 * Hook for Selection-to-AI (⌘L): select text in terminal, send to AI as context.
 *
 * When ⌘L is pressed and there's a selection in the terminal,
 * the selected text is captured and passed to the callback.
 */
export function useSelectionToAi(onSendToAi?: (text: string) => void) {
  const [selectedText, setSelectedText] = useState<string>('')
  const [showToast, setShowToast] = useState(false)

  const captureSelection = useCallback((terminal: Terminal) => {
    const selection = terminal.getSelection()
    if (selection) {
      setSelectedText(selection)
      return selection
    }
    return ''
  }, [])

  // ⌘L keyboard handler
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key === 'l') {
        if (selectedText && onSendToAi) {
          e.preventDefault()
          onSendToAi(selectedText)
          setShowToast(true)
          setTimeout(() => setShowToast(false), 2000)
        }
      }
    }
    window.addEventListener('keydown', handler)
    return () => window.removeEventListener('keydown', handler)
  }, [selectedText, onSendToAi])

  return {
    selectedText,
    showToast,
    captureSelection,
  }
}
