import { describe, expect, it } from 'vitest'
import { buildCommandPaletteFooterHints } from '@/services/commandPaletteFooterHints'

describe('commandPaletteFooterHints', () => {
  it('builds suggestion-focused hints when command completions are visible', () => {
    expect(buildCommandPaletteFooterHints({
      hasCommandSuggestions: true,
      showCommandHelp: false,
      includeCloseHint: true,
    })).toEqual([
      { key: '?', label: 'Help', active: false },
      { key: '↑ ↓', label: 'Suggest', active: true },
      { key: 'Tab', label: 'Complete', active: true },
      { key: 'Enter', label: 'Apply', active: true },
      { key: 'Esc', label: 'Close' },
    ])
  })

  it('builds help-focused hints when help mode is active', () => {
    expect(buildCommandPaletteFooterHints({
      hasCommandSuggestions: false,
      showCommandHelp: true,
    })).toEqual([
      { key: '?', label: 'Help', active: true },
      { key: 'Click', label: 'Use Example' },
    ])
  })

  it('falls back to navigation hints in normal search mode', () => {
    expect(buildCommandPaletteFooterHints({
      hasCommandSuggestions: false,
      showCommandHelp: false,
    })).toEqual([
      { key: '?', label: 'Help', active: false },
      { key: 'Tab', label: 'Group' },
      { key: '↑ ↓', label: 'Navigate' },
      { key: 'Enter', label: 'Open' },
    ])
  })
})
