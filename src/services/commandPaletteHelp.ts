export interface CommandPaletteHelpItem {
  id: string
  title: string
  syntax: string
  description: string
  examples: string[]
  keywords: string[]
}

const COMMAND_PALETTE_HELP_ITEMS: CommandPaletteHelpItem[] = [
  {
    id: 'theme',
    title: '主题命令',
    syntax: 'theme set <light|dark|corporate>',
    description: '直接切换应用主题，不必先打开设置页面。',
    examples: ['theme set dark', 'theme set light', 'theme set corporate'],
    keywords: ['theme', 'dark', 'light', 'corporate', '主题'],
  },
  {
    id: 'finding',
    title: '漏洞命令',
    syntax: 'finding severity:<critical|high>',
    description: '直接跳到漏洞列表，并附带严重级别筛选。',
    examples: ['finding severity:critical', 'finding severity:high'],
    keywords: ['finding', 'severity', 'critical', 'high', '漏洞'],
  },
  {
    id: 'notification',
    title: '通知命令',
    syntax: 'notify open <center|rules>',
    description: '快速打开消息中心或通知规则页面。',
    examples: ['notify open center', 'notify open rules'],
    keywords: ['notify', 'notification', 'center', 'rules', '通知'],
  },
]

const normalizeHelpText = (value: string) =>
  value
    .toLowerCase()
    .trim()
    .replace(/[_/\\-]+/g, ' ')
    .replace(/\s+/g, ' ')

function extractCommandHelpTerm(query: string) {
  const normalizedQuery = normalizeHelpText(query)
  if (!normalizedQuery) {
    return null
  }

  if (normalizedQuery === '?' || normalizedQuery === 'help' || normalizedQuery === 'commands') {
    return ''
  }

  if (normalizedQuery.startsWith('? ')) {
    return normalizedQuery.slice(2).trim()
  }

  if (normalizedQuery.startsWith('help ')) {
    return normalizedQuery.slice(5).trim()
  }

  if (normalizedQuery.startsWith('commands ')) {
    return normalizedQuery.slice(9).trim()
  }

  return null
}

export function isCommandHelpQuery(query: string) {
  return extractCommandHelpTerm(query) !== null
}

export function getCommandPaletteHelpItems(query: string) {
  const helpTerm = extractCommandHelpTerm(query)
  if (helpTerm === null) {
    return []
  }

  if (!helpTerm) {
    return COMMAND_PALETTE_HELP_ITEMS
  }

  const helpTokens = helpTerm.split(' ').filter(Boolean)

  return COMMAND_PALETTE_HELP_ITEMS.filter(item => {
    const haystack = normalizeHelpText([
      item.title,
      item.syntax,
      item.description,
      ...item.examples,
      ...item.keywords,
    ].join(' '))

    return helpTokens.every(token => haystack.includes(token))
  })
}
