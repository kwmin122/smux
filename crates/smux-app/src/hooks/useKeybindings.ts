import { useCallback, useEffect, useMemo, useState } from 'react'

/**
 * Keybinding configuration module for smux terminal app.
 *
 * Manages keyboard shortcuts with support for customization and presets
 * (default, tmux, vim). Custom overrides are persisted in localStorage.
 */

export interface KeyBinding {
  id: string
  action: string
  defaultKey: string
  customKey?: string
  category: 'general' | 'terminal' | 'ai' | 'navigation'
}

export type KeyPreset = 'default' | 'tmux' | 'vim'

const STORAGE_KEY = 'smux-keybindings'

const DEFAULT_BINDINGS: KeyBinding[] = [
  { id: 'new-tab', action: 'New Tab', defaultKey: 'Meta+t', category: 'general' },
  { id: 'close-tab', action: 'Close Tab', defaultKey: 'Meta+w', category: 'general' },
  { id: 'split-vertical', action: 'Split Vertical', defaultKey: 'Meta+d', category: 'terminal' },
  { id: 'split-horizontal', action: 'Split Horizontal', defaultKey: 'Meta+Shift+d', category: 'terminal' },
  { id: 'search', action: 'Search', defaultKey: 'Meta+f', category: 'terminal' },
  { id: 'selection-to-ai', action: 'Selection to AI', defaultKey: 'Meta+l', category: 'ai' },
  { id: 'toggle-browser', action: 'Toggle Browser', defaultKey: 'Meta+b', category: 'navigation' },
  { id: 'save-layout', action: 'Save Layout', defaultKey: 'Meta+s', category: 'general' },
  { id: 'swap-panels', action: 'Swap Panels', defaultKey: 'Meta+Shift+s', category: 'navigation' },
  { id: 'next-tab', action: 'Next Tab', defaultKey: 'Meta+]', category: 'navigation' },
  { id: 'prev-tab', action: 'Previous Tab', defaultKey: 'Meta+[', category: 'navigation' },
  { id: 'zoom-pane', action: 'Zoom Pane', defaultKey: 'Meta+Shift+Enter', category: 'terminal' },
]

const TMUX_OVERRIDES: Partial<Record<string, string>> = {
  'split-vertical': 'Ctrl+b %',
  'split-horizontal': 'Ctrl+b "',
  'next-tab': 'Ctrl+b n',
  'prev-tab': 'Ctrl+b p',
  'new-tab': 'Ctrl+b c',
  'close-tab': 'Ctrl+b x',
}

const VIM_OVERRIDES: Partial<Record<string, string>> = {
  'split-vertical': 'Ctrl+w v',
  'split-horizontal': 'Ctrl+w s',
  'next-tab': 'g t',
  'prev-tab': 'g T',
  'close-tab': 'Ctrl+w q',
}

const PRESET_OVERRIDES: Record<KeyPreset, Partial<Record<string, string>>> = {
  default: {},
  tmux: TMUX_OVERRIDES,
  vim: VIM_OVERRIDES,
}

interface StoredState {
  preset: KeyPreset
  customKeys: Record<string, string>
}

function loadStoredState(): StoredState {
  try {
    const raw = localStorage.getItem(STORAGE_KEY)
    if (raw) {
      const parsed = JSON.parse(raw) as StoredState
      if (parsed && typeof parsed.preset === 'string' && parsed.customKeys) {
        return parsed
      }
    }
  } catch {
    // Corrupted data — fall through to defaults
  }
  return { preset: 'default', customKeys: {} }
}

function saveStoredState(state: StoredState): void {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(state))
  } catch {
    // Storage full or unavailable — silently ignore
  }
}

function buildBindings(
  preset: KeyPreset,
  customKeys: Record<string, string>,
): KeyBinding[] {
  const presetOverrides = PRESET_OVERRIDES[preset] ?? {}
  return DEFAULT_BINDINGS.map((binding) => {
    const customKey = customKeys[binding.id] ?? presetOverrides[binding.id]
    return customKey ? { ...binding, customKey } : { ...binding }
  })
}

export function useKeybindings() {
  const [preset, setPreset] = useState<KeyPreset>(() => loadStoredState().preset)
  const [customKeys, setCustomKeys] = useState<Record<string, string>>(
    () => loadStoredState().customKeys,
  )

  // Persist whenever preset or custom keys change
  useEffect(() => {
    saveStoredState({ preset, customKeys })
  }, [preset, customKeys])

  const bindings = useMemo(
    () => buildBindings(preset, customKeys),
    [preset, customKeys],
  )

  /** Returns the effective key combination for a given action id. */
  const getKey = useCallback(
    (actionId: string): string | undefined => {
      const presetOverrides = PRESET_OVERRIDES[preset] ?? {}
      const custom = customKeys[actionId]
      if (custom) return custom
      const presetKey = presetOverrides[actionId]
      if (presetKey) return presetKey
      const binding = DEFAULT_BINDINGS.find((b) => b.id === actionId)
      return binding?.defaultKey
    },
    [preset, customKeys],
  )

  /** Sets a custom key override for a given action id. */
  const setCustomKey = useCallback((actionId: string, key: string) => {
    setCustomKeys((prev) => ({ ...prev, [actionId]: key }))
  }, [])

  /** Clears all custom overrides and switches to the given preset. */
  const resetToPreset = useCallback((newPreset: KeyPreset) => {
    setCustomKeys({})
    setPreset(newPreset)
  }, [])

  return {
    bindings,
    preset,
    getKey,
    setCustomKey,
    resetToPreset,
  }
}
