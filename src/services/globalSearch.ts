export interface GlobalSearchEntry {
  id: string
  title: string
  description: string
  path: string
  icon: string
  category: 'action' | 'page' | 'shortcut' | 'task' | 'workflow' | 'document' | 'plugin' | 'notification' | 'finding'
  keywords: string[]
  aliases?: string[]
  query?: Record<string, string>
  featured?: boolean
  execute?: () => void | Promise<void>
}

export interface GlobalSearchResult extends GlobalSearchEntry {
  score: number
}

export function dedupeGlobalSearchEntries(entries: GlobalSearchEntry[]) {
  const uniqueEntries = new Map<string, GlobalSearchEntry>()

  for (const entry of entries) {
    if (!uniqueEntries.has(entry.id)) {
      uniqueEntries.set(entry.id, entry)
    }
  }

  return Array.from(uniqueEntries.values())
}

export function getSearchCategoryLabel(category: GlobalSearchEntry['category']) {
  switch (category) {
    case 'action':
      return '操作'
    case 'document':
      return '文档'
    case 'finding':
      return '漏洞'
    case 'notification':
      return '通知'
    case 'plugin':
      return '插件'
    case 'shortcut':
      return '快捷入口'
    case 'task':
      return '任务'
    case 'workflow':
      return '工作流'
    default:
      return '页面'
  }
}

const normalizeSearchText = (value: string) =>
  value
    .toLowerCase()
    .trim()
    .replace(/[_/\\-]+/g, ' ')
    .replace(/\s+/g, ' ')

const tokenizeSearchText = (value: string) =>
  normalizeSearchText(value)
    .split(' ')
    .map(token => token.trim())
    .filter(Boolean)

export function searchGlobalEntries(
  entries: GlobalSearchEntry[],
  query: string,
  limit = 20,
): GlobalSearchResult[] {
  const normalizedQuery = normalizeSearchText(query)
  if (!normalizedQuery) {
    return []
  }

  const queryTokens = tokenizeSearchText(normalizedQuery)

  return entries
    .map(entry => {
      const normalizedTitle = normalizeSearchText(entry.title)
      const normalizedDescription = normalizeSearchText(entry.description)
      const normalizedKeywords = entry.keywords.map(keyword => normalizeSearchText(keyword))
      const normalizedAliases = (entry.aliases || []).map(alias => normalizeSearchText(alias))
      const keywordHaystack = normalizedKeywords.join(' ')
      const aliasHaystack = normalizedAliases.join(' ')
      const fullHaystack = `${normalizedTitle} ${normalizedDescription} ${keywordHaystack} ${aliasHaystack} ${normalizeSearchText(entry.path)} ${entry.category}`

      if (!queryTokens.every(token => fullHaystack.includes(token))) {
        return null
      }

      let score = 0

      if (normalizedTitle === normalizedQuery) {
        score += 120
      } else if (normalizedTitle.startsWith(normalizedQuery)) {
        score += 70
      } else if (normalizedTitle.includes(normalizedQuery)) {
        score += 45
      }

      if (normalizedDescription.includes(normalizedQuery)) {
        score += 18
      }

      for (const keyword of normalizedKeywords) {
        if (keyword === normalizedQuery) {
          score += 90
        } else if (keyword.startsWith(normalizedQuery)) {
          score += 36
        } else if (keyword.includes(normalizedQuery)) {
          score += 18
        }
      }

      for (const alias of normalizedAliases) {
        if (alias === normalizedQuery) {
          score += 110
        } else if (alias.startsWith(normalizedQuery)) {
          score += 48
        } else if (alias.includes(normalizedQuery)) {
          score += 24
        }
      }

      for (const token of queryTokens) {
        if (normalizedTitle.includes(token)) {
          score += 16
        }

        if (normalizedDescription.includes(token)) {
          score += 6
        }

        if (normalizedKeywords.some(keyword => keyword.includes(token))) {
          score += 10
        }

        if (normalizedAliases.some(alias => alias.includes(token))) {
          score += 14
        }
      }

      if (entry.featured) {
        score += 4
      }

      return {
        ...entry,
        score,
      }
    })
    .filter((entry): entry is GlobalSearchResult => entry !== null)
    .sort((left, right) => {
      if (right.score !== left.score) {
        return right.score - left.score
      }

      if (left.category !== right.category) {
        return left.category === 'page' ? -1 : 1
      }

      return left.title.localeCompare(right.title, 'zh-CN')
    })
    .slice(0, limit)
}

export function isStrongSearchMatch(result: GlobalSearchResult | null, query: string) {
  if (!result) {
    return false
  }

  const normalizedQuery = normalizeSearchText(query)
  if (!normalizedQuery) {
    return false
  }

  const normalizedTitle = normalizeSearchText(result.title)
  if (normalizedTitle === normalizedQuery) {
    return true
  }

  if (result.keywords.some(keyword => normalizeSearchText(keyword) === normalizedQuery)) {
    return true
  }

  return (result.aliases || []).some(alias => normalizeSearchText(alias) === normalizedQuery)
}
