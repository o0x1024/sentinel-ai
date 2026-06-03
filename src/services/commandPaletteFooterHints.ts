export interface CommandPaletteFooterHint {
  key: string
  label: string
  active?: boolean
}

interface CommandPaletteFooterHintOptions {
  hasCommandSuggestions: boolean
  showCommandHelp: boolean
  includeCloseHint?: boolean
}

export function buildCommandPaletteFooterHints(
  options: CommandPaletteFooterHintOptions,
): CommandPaletteFooterHint[] {
  const hints: CommandPaletteFooterHint[] = [
    {
      key: '?',
      label: 'Help',
      active: options.showCommandHelp,
    },
  ]

  if (options.hasCommandSuggestions) {
    hints.push(
      { key: '↑ ↓', label: 'Suggest', active: true },
      { key: 'Tab', label: 'Complete', active: true },
      { key: 'Enter', label: 'Apply', active: true },
    )
  } else if (options.showCommandHelp) {
    hints.push({ key: 'Click', label: 'Use Example' })
  } else {
    hints.push(
      { key: 'Tab', label: 'Group' },
      { key: '↑ ↓', label: 'Navigate' },
      { key: 'Enter', label: 'Open' },
    )
  }

  if (options.includeCloseHint) {
    hints.push({ key: 'Esc', label: 'Close' })
  }

  return hints
}
