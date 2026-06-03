import { describe, expect, it } from 'vitest'
import {
  buildCommandPaletteSearchEntries,
  COMMAND_PALETTE_ACTION_IDS,
  parseCommandPaletteQuery,
  resolveCommandPaletteActionIds,
  suggestCommandPaletteQueries,
} from '@/services/commandPaletteCommands'
import type { GlobalSearchEntry } from '@/services/globalSearch'

describe('commandPaletteCommands', () => {
  it('resolves theme commands with explicit verbs', () => {
    expect(resolveCommandPaletteActionIds('theme set dark')).toEqual([
      COMMAND_PALETTE_ACTION_IDS.themeDark,
    ])
  })

  it('resolves finding severity commands with named arguments', () => {
    expect(resolveCommandPaletteActionIds('finding severity:critical')).toEqual([
      COMMAND_PALETTE_ACTION_IDS.findingsCritical,
    ])
  })

  it('returns available notification actions for incomplete commands', () => {
    expect(resolveCommandPaletteActionIds('notify open')).toEqual([
      COMMAND_PALETTE_ACTION_IDS.notificationCenter,
      COMMAND_PALETTE_ACTION_IDS.notificationMessages,
      COMMAND_PALETTE_ACTION_IDS.notificationNotifications,
      COMMAND_PALETTE_ACTION_IDS.notificationRules,
    ])
  })

  it('returns available theme actions for namespace-only commands', () => {
    expect(resolveCommandPaletteActionIds('theme')).toEqual([
      COMMAND_PALETTE_ACTION_IDS.themeLight,
      COMMAND_PALETTE_ACTION_IDS.themeDark,
      COMMAND_PALETTE_ACTION_IDS.themeCorporate,
    ])
  })

  it('returns structured command feedback for parsed finding commands', () => {
    expect(parseCommandPaletteQuery('finding severity:critical')).toEqual({
      command: 'finding',
      label: '漏洞命令',
      status: 'resolved',
      actionIds: [COMMAND_PALETTE_ACTION_IDS.findingsCritical],
      chips: [
        { key: 'resource', value: 'finding', text: 'resource=finding' },
        { key: 'severity', value: 'critical', text: 'severity=critical' },
      ],
    })
  })

  it('builds search-ready command entries with the original query alias', () => {
    const actionsById = new Map<string, GlobalSearchEntry>([
      [COMMAND_PALETTE_ACTION_IDS.themeDark, {
        id: COMMAND_PALETTE_ACTION_IDS.themeDark,
        title: '切换到深色主题',
        description: '立即切换应用主题为深色',
        path: '',
        icon: 'fas fa-moon',
        category: 'action',
        keywords: ['theme', 'dark'],
      }],
    ])

    const [entry] = buildCommandPaletteSearchEntries('theme set dark', actionsById)
    expect(entry.aliases).toContain('theme set dark')
  })

  it('suggests command completions for partial finding queries', () => {
    expect(suggestCommandPaletteQueries('finding se')).toEqual([
      {
        query: 'finding severity:critical',
        label: 'severity=critical',
        description: '查看严重漏洞',
      },
      {
        query: 'finding severity:high',
        label: 'severity=high',
        description: '查看高危漏洞',
      },
      {
        query: 'finding severity:medium',
        label: 'severity=medium',
        description: '查看中危漏洞',
      },
      {
        query: 'finding severity:low',
        label: 'severity=low',
        description: '查看低危漏洞',
      },
    ])
  })

  it('suggests root commands for top-level prefixes', () => {
    expect(suggestCommandPaletteQueries('the')).toEqual([
      {
        query: 'theme set dark',
        label: 'theme set dark',
        description: '切换到深色主题',
      },
    ])
  })
})
