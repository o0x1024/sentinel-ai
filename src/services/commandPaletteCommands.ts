import type { GlobalSearchEntry } from '@/services/globalSearch'

export const COMMAND_PALETTE_ACTION_IDS = {
  themeLight: 'action:theme-light',
  themeDark: 'action:theme-dark',
  themeCorporate: 'action:theme-corporate',
  notificationRules: 'action:notifications-rules',
  notificationCenter: 'action:notification-center',
  notificationMessages: 'action:notification-messages',
  notificationNotifications: 'action:notification-notifications',
  findingsCritical: 'action:critical-findings',
  findingsHigh: 'action:high-findings',
  findingsMedium: 'action:medium-findings',
  findingsLow: 'action:low-findings',
} as const

export type CommandPaletteActionId =
  typeof COMMAND_PALETTE_ACTION_IDS[keyof typeof COMMAND_PALETTE_ACTION_IDS]

export interface ParsedCommandPaletteChip {
  key: string
  value: string
  text: string
}

export interface ParsedCommandPaletteQuery {
  command: 'theme' | 'notification' | 'finding'
  label: string
  status: 'partial' | 'resolved'
  actionIds: CommandPaletteActionId[]
  chips: ParsedCommandPaletteChip[]
}

export interface CommandPaletteSuggestion {
  query: string
  label: string
  description: string
}

const THEME_ACTIONS: CommandPaletteActionId[] = [
  COMMAND_PALETTE_ACTION_IDS.themeLight,
  COMMAND_PALETTE_ACTION_IDS.themeDark,
  COMMAND_PALETTE_ACTION_IDS.themeCorporate,
]

const NOTIFICATION_ACTIONS: CommandPaletteActionId[] = [
  COMMAND_PALETTE_ACTION_IDS.notificationCenter,
  COMMAND_PALETTE_ACTION_IDS.notificationMessages,
  COMMAND_PALETTE_ACTION_IDS.notificationNotifications,
  COMMAND_PALETTE_ACTION_IDS.notificationRules,
]

const FINDING_ACTIONS: CommandPaletteActionId[] = [
  COMMAND_PALETTE_ACTION_IDS.findingsCritical,
  COMMAND_PALETTE_ACTION_IDS.findingsHigh,
  COMMAND_PALETTE_ACTION_IDS.findingsMedium,
  COMMAND_PALETTE_ACTION_IDS.findingsLow,
]

const ROOT_COMMAND_SUGGESTIONS: CommandPaletteSuggestion[] = [
  {
    query: 'theme set dark',
    label: 'theme set dark',
    description: '切换到深色主题',
  },
  {
    query: 'finding severity:critical',
    label: 'finding severity:critical',
    description: '查看严重漏洞',
  },
  {
    query: 'notify open rules',
    label: 'notify open rules',
    description: '打开通知规则',
  },
  {
    query: 'notify open messages',
    label: 'notify open messages',
    description: '打开消息分类',
  },
]

const normalizeCommandToken = (value: string) =>
  value
    .toLowerCase()
    .trim()
    .replace(/[_/\\-]+/g, ' ')
    .replace(/\s+/g, ' ')

const tokenizeCommandQuery = (query: string) =>
  normalizeCommandToken(query)
    .split(' ')
    .map(token => token.trim())
    .filter(Boolean)

const extractNamedToken = (tokens: string[], key: string) => {
  const target = `${key}:`
  const match = tokens.find(token => token.startsWith(target))
  return match ? match.slice(target.length) : ''
}

const buildChip = (key: string, value: string): ParsedCommandPaletteChip => ({
  key,
  value,
  text: `${key}=${value}`,
})

const appendValueChip = (chips: ParsedCommandPaletteChip[], key: string, value: string) => {
  if (!value) {
    return chips
  }

  return [...chips, buildChip(key, value)]
}

const includesByPrefix = (input: string, candidate: string) =>
  candidate.startsWith(input) || input.startsWith(candidate)

const resolveThemeValue = (value: string) => {
  if (!value) {
    return {
      value: '',
      actionIds: THEME_ACTIONS,
    }
  }

  if (['light', 'bright', '浅色'].some(option => option.startsWith(value) || value.startsWith(option))) {
    return {
      value: 'light',
      actionIds: [COMMAND_PALETTE_ACTION_IDS.themeLight],
    }
  }

  if (['dark', '深色'].some(option => option.startsWith(value) || value.startsWith(option))) {
    return {
      value: 'dark',
      actionIds: [COMMAND_PALETTE_ACTION_IDS.themeDark],
    }
  }

  if (['corporate', 'business', '企业'].some(option => option.startsWith(value) || value.startsWith(option))) {
    return {
      value: 'corporate',
      actionIds: [COMMAND_PALETTE_ACTION_IDS.themeCorporate],
    }
  }

  return {
    value,
    actionIds: [] as CommandPaletteActionId[],
  }
}

const resolveNotificationValue = (value: string) => {
  if (!value) {
    return {
      value: '',
      actionIds: NOTIFICATION_ACTIONS,
    }
  }

  if (['center', 'inbox', 'messages', '消息中心'].some(option => option.startsWith(value) || value.startsWith(option))) {
    return {
      value: 'center',
      actionIds: [COMMAND_PALETTE_ACTION_IDS.notificationCenter],
    }
  }

  if (['message', 'messages', 'mail', '站内信', '消息'].some(option => option.startsWith(value) || value.startsWith(option))) {
    return {
      value: 'messages',
      actionIds: [COMMAND_PALETTE_ACTION_IDS.notificationMessages],
    }
  }

  if (['notification', 'notifications', 'alerts', '提醒', '通知'].some(option => option.startsWith(value) || value.startsWith(option))) {
    return {
      value: 'notifications',
      actionIds: [COMMAND_PALETTE_ACTION_IDS.notificationNotifications],
    }
  }

  if (['rules', 'rule', 'settings', 'preferences', '通知规则'].some(option => option.startsWith(value) || value.startsWith(option))) {
    return {
      value: 'rules',
      actionIds: [COMMAND_PALETTE_ACTION_IDS.notificationRules],
    }
  }

  return {
    value,
    actionIds: [] as CommandPaletteActionId[],
  }
}

const resolveFindingValue = (value: string) => {
  if (!value) {
    return {
      value: '',
      actionIds: FINDING_ACTIONS,
    }
  }

  if (['critical', '严重'].some(option => option.startsWith(value) || value.startsWith(option))) {
    return {
      value: 'critical',
      actionIds: [COMMAND_PALETTE_ACTION_IDS.findingsCritical],
    }
  }

  if (['high', '高危'].some(option => option.startsWith(value) || value.startsWith(option))) {
    return {
      value: 'high',
      actionIds: [COMMAND_PALETTE_ACTION_IDS.findingsHigh],
    }
  }

  if (['medium', '中危'].some(option => option.startsWith(value) || value.startsWith(option))) {
    return {
      value: 'medium',
      actionIds: [COMMAND_PALETTE_ACTION_IDS.findingsMedium],
    }
  }

  if (['low', '低危'].some(option => option.startsWith(value) || value.startsWith(option))) {
    return {
      value: 'low',
      actionIds: [COMMAND_PALETTE_ACTION_IDS.findingsLow],
    }
  }

  return {
    value,
    actionIds: [] as CommandPaletteActionId[],
  }
}

export function parseCommandPaletteQuery(query: string): ParsedCommandPaletteQuery | null {
  const tokens = tokenizeCommandQuery(query)
  if (tokens.length === 0) {
    return null
  }

  const firstToken = tokens[0]
  const namedTheme = extractNamedToken(tokens, 'theme')
  const namedSeverity = extractNamedToken(tokens, 'severity')

  if (firstToken === 'theme' || namedTheme) {
    const themeToken = namedTheme || tokens.find(token => !['theme', 'set', 'switch', 'use', 'to'].includes(token)) || ''
    const resolved = resolveThemeValue(themeToken)

    return {
      command: 'theme',
      label: '主题命令',
      status: resolved.actionIds.length === 1 ? 'resolved' : 'partial',
      actionIds: resolved.actionIds,
      chips: appendValueChip([buildChip('resource', 'theme')], 'theme', resolved.value),
    }
  }

  if (['notify', 'notification', 'notifications', 'message', 'messages'].includes(firstToken)) {
    const targetToken = tokens.find(token => !['notify', 'notification', 'notifications', 'message', 'messages', 'open', 'go', 'show'].includes(token)) || ''
    const resolved = resolveNotificationValue(targetToken)

    return {
      command: 'notification',
      label: '通知命令',
      status: resolved.actionIds.length === 1 ? 'resolved' : 'partial',
      actionIds: resolved.actionIds,
      chips: appendValueChip([buildChip('resource', 'notification')], 'target', resolved.value),
    }
  }

  if (['finding', 'findings', 'vulnerability', 'vulnerabilities', 'vuln', 'vulns'].includes(firstToken)) {
    const severityToken =
      namedSeverity ||
      tokens.find(token => !['finding', 'findings', 'vulnerability', 'vulnerabilities', 'vuln', 'vulns', 'severity', 'show', 'open'].includes(token)) ||
      ''
    const resolved = resolveFindingValue(severityToken)

    return {
      command: 'finding',
      label: '漏洞命令',
      status: resolved.actionIds.length === 1 ? 'resolved' : 'partial',
      actionIds: resolved.actionIds,
      chips: appendValueChip([buildChip('resource', 'finding')], 'severity', resolved.value),
    }
  }

  return null
}

export function resolveCommandPaletteActionIds(query: string): CommandPaletteActionId[] {
  return parseCommandPaletteQuery(query)?.actionIds || []
}

export function buildCommandPaletteSearchEntries(
  query: string,
  actionsById: Map<string, GlobalSearchEntry>,
) {
  const entries: GlobalSearchEntry[] = []

  for (const actionId of resolveCommandPaletteActionIds(query)) {
    const entry = actionsById.get(actionId)
    if (!entry) {
      continue
    }

    entries.push({
      ...entry,
      aliases: [query, ...(entry.aliases || [])],
    })
  }

  return entries
}

function getThemeCommandSuggestions(): CommandPaletteSuggestion[] {
  return [
    {
      query: 'theme set light',
      label: 'theme=light',
      description: '切换到浅色主题',
    },
    {
      query: 'theme set dark',
      label: 'theme=dark',
      description: '切换到深色主题',
    },
    {
      query: 'theme set corporate',
      label: 'theme=corporate',
      description: '切换到企业主题',
    },
  ]
}

function getNotificationCommandSuggestions(): CommandPaletteSuggestion[] {
  return [
    {
      query: 'notify open center',
      label: 'target=center',
      description: '打开消息中心',
    },
    {
      query: 'notify open messages',
      label: 'target=messages',
      description: '只看消息分类',
    },
    {
      query: 'notify open notifications',
      label: 'target=notifications',
      description: '只看通知分类',
    },
    {
      query: 'notify open rules',
      label: 'target=rules',
      description: '打开通知规则',
    },
  ]
}

function getFindingCommandSuggestions(): CommandPaletteSuggestion[] {
  return [
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
  ]
}

export function suggestCommandPaletteQueries(query: string): CommandPaletteSuggestion[] {
  const normalizedQuery = normalizeCommandToken(query)
  if (!normalizedQuery) {
    return []
  }

  const parsed = parseCommandPaletteQuery(query)
  const firstToken = tokenizeCommandQuery(query)[0] || ''

  if (parsed?.command === 'theme') {
    return getThemeCommandSuggestions().filter(item => item.query.includes(normalizedQuery) || normalizedQuery.includes('theme'))
  }

  if (parsed?.command === 'notification') {
    return getNotificationCommandSuggestions().filter(item => item.query.includes(normalizedQuery) || normalizedQuery.includes('notify'))
  }

  if (parsed?.command === 'finding') {
    return getFindingCommandSuggestions().filter(item => item.query.includes(normalizedQuery) || normalizedQuery.includes('finding'))
  }

  if (includesByPrefix(firstToken, 'theme')) {
    return ROOT_COMMAND_SUGGESTIONS.filter(item => item.query.startsWith('theme')).slice(0, 3)
  }

  if (includesByPrefix(firstToken, 'finding')) {
    return ROOT_COMMAND_SUGGESTIONS.filter(item => item.query.startsWith('finding')).slice(0, 3)
  }

  if (includesByPrefix(firstToken, 'notify')) {
    return ROOT_COMMAND_SUGGESTIONS.filter(item => item.query.startsWith('notify')).slice(0, 3)
  }

  return ROOT_COMMAND_SUGGESTIONS.filter(item => item.query.includes(normalizedQuery)).slice(0, 3)
}
