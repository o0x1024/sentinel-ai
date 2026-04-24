import type {
  IntruderBuiltInPayloadListDefinition,
  IntruderBuiltInPayloadRecommendationContext,
  IntruderPayloadQualityStats,
} from './intruderBuiltInPayloadCatalogShared'
import { INTRUDER_ACCOUNT_PAYLOAD_LISTS, INTRUDER_PAYLOAD_TEMPLATES } from './intruderBuiltInPayloadCatalogAccounts'
import { INTRUDER_ATOM_PAYLOAD_LISTS } from './intruderBuiltInPayloadCatalogAtoms'
import { INTRUDER_DISCOVERY_PAYLOAD_LISTS } from './intruderBuiltInPayloadCatalogDiscovery'
import { INTRUDER_FUZZING_PAYLOAD_LISTS } from './intruderBuiltInPayloadCatalogFuzzing'
import { deduplicatePayloadItems } from './intruderBuiltInPayloadCatalogShared'

const SEARCH_SYNONYMS: Record<string, string[]> = {
  username: ['user', 'login', 'account', 'member', '用户名', '账号'],
  password: ['passwd', 'pwd', 'pass', '密码'],
  email: ['mail', '邮箱', '邮件'],
  phone: ['mobile', 'tel', 'telephone', 'msisdn', '手机号', '手机', '电话'],
  path: ['dir', 'directory', 'folder', 'route', '路径', '目录'],
  file: ['filename', 'extension', 'resource', '文件', '文件名', '扩展名'],
  api: ['endpoint', 'route', 'url', '接口', '端点'],
  chinese: ['cn', 'zh', '中文', '拼音'],
}

const WEAK_ENTRY_SET = new Set([
  'admin',
  'password',
  '123456',
  '12345678',
  '0000',
  '1111',
  '1234',
  'qwerty',
  'welcome',
  'letmein',
  'secret',
])

export const INTRUDER_BUILT_IN_PAYLOAD_LISTS: IntruderBuiltInPayloadListDefinition[] = [
  ...INTRUDER_FUZZING_PAYLOAD_LISTS,
  ...INTRUDER_ACCOUNT_PAYLOAD_LISTS,
  ...INTRUDER_ATOM_PAYLOAD_LISTS,
  ...INTRUDER_DISCOVERY_PAYLOAD_LISTS,
]

export { INTRUDER_PAYLOAD_TEMPLATES }

export function getIntruderBuiltInPayloadListItems(listId: string): string[] {
  return getIntruderBuiltInPayloadListDefinition(listId)?.items ?? []
}

export function getIntruderBuiltInPayloadListDefinition(listId: string): IntruderBuiltInPayloadListDefinition | null {
  return INTRUDER_BUILT_IN_PAYLOAD_LISTS.find((list) => list.id === listId) ?? null
}

export function normalizeIntruderPayloadSearchText(value: string): string {
  return value
    .trim()
    .toLowerCase()
    .normalize('NFKC')
    .replace(/[_\-.:/\\\s]+/g, '')
}

export function expandIntruderPayloadSearchTokens(query: string): string[] {
  const tokens = query
    .trim()
    .toLowerCase()
    .split(/[\s,]+/)
    .map((token) => token.trim())
    .filter(Boolean)

  const expanded = new Set<string>()
  tokens.forEach((token) => {
    expanded.add(token)
    expanded.add(normalizeIntruderPayloadSearchText(token))

    for (const [canonical, synonyms] of Object.entries(SEARCH_SYNONYMS)) {
      if (token === canonical || synonyms.includes(token)) {
        expanded.add(canonical)
        synonyms.forEach((item) => expanded.add(item))
      }
    }
  })

  return Array.from(expanded).filter(Boolean)
}

export function matchesIntruderBuiltInPayloadList(
  definition: IntruderBuiltInPayloadListDefinition,
  query: string,
): boolean {
  if (!query.trim()) {
    return true
  }

  const haystack = [
    definition.id,
    definition.categoryKey,
    definition.labelKey,
    definition.descriptionKey,
    ...definition.aliases,
    ...definition.tags,
    ...definition.recommendedPositionHints,
    ...definition.items,
  ]
    .map((value) => normalizeIntruderPayloadSearchText(value))
    .filter(Boolean)

  const tokenGroups = query
    .trim()
    .toLowerCase()
    .split(/[\s,]+/)
    .map((token) => token.trim())
    .filter(Boolean)
    .map((token) => expandIntruderPayloadSearchTokens(token))

  return tokenGroups.every((group) =>
    group.some((token) => {
      const normalizedToken = normalizeIntruderPayloadSearchText(token)
      return haystack.some((entry) => entry.includes(normalizedToken))
    }),
  )
}

export function rankIntruderBuiltInPayloadLists(
  context: IntruderBuiltInPayloadRecommendationContext,
): string[] {
  const keywords = collectIntruderRecommendationKeywords(context)
  return INTRUDER_BUILT_IN_PAYLOAD_LISTS
    .map((definition) => ({
      id: definition.id,
      score: scoreIntruderBuiltInPayloadList(definition, context, keywords),
    }))
    .sort((left, right) => right.score - left.score || left.id.localeCompare(right.id))
    .filter((item) => item.score > 0)
    .map((item) => item.id)
}

export function getIntruderPayloadQualityStats(items: string[]): IntruderPayloadQualityStats {
  const normalized = items.map((item) => item ?? '')
  const deduplicated = deduplicatePayloadItems(normalized)
  const totalLength = normalized.reduce((sum, item) => sum + item.length, 0)

  const charsetDistribution = normalized.reduce(
    (distribution, item) => {
      const hasUnicode = /[^\u0000-\u007f]/.test(item)
      const hasAlphabetic = /[a-z]/i.test(item)
      const hasNumeric = /\d/.test(item)
      const hasSymbols = /[^a-z0-9\u0000-\u007f]/i.test(item)

      if (hasUnicode) {
        distribution.withUnicode += 1
      }
      if (hasSymbols) {
        distribution.withSymbols += 1
      }

      if (/^\d+$/.test(item)) {
        distribution.numericOnly += 1
      } else if (/^[a-z]+$/i.test(item)) {
        distribution.alphabeticOnly += 1
      } else if (/^[a-z0-9]+$/i.test(item)) {
        distribution.alphaNumeric += 1
      } else {
        distribution.mixed += 1
      }

      return distribution
    },
    {
      numericOnly: 0,
      alphabeticOnly: 0,
      alphaNumeric: 0,
      withUnicode: 0,
      withSymbols: 0,
      mixed: 0,
    },
  )

  const weakCount = normalized.filter((item) => {
    const normalizedItem = item.trim().toLowerCase()
    return WEAK_ENTRY_SET.has(normalizedItem) || /^\d{4,6}$/.test(normalizedItem)
  }).length

  const dirtyCount = normalized.filter((item) => item !== item.trim() || /[\u0000-\u001f]/.test(item)).length

  return {
    total: normalized.length,
    unique: deduplicated.length,
    duplicates: Math.max(0, normalized.length - deduplicated.length),
    averageLength: normalized.length ? Number((totalLength / normalized.length).toFixed(1)) : 0,
    weakRatio: normalized.length ? Number((weakCount / normalized.length).toFixed(2)) : 0,
    dirtyCount,
    charsetDistribution,
  }
}

function scoreIntruderBuiltInPayloadList(
  definition: IntruderBuiltInPayloadListDefinition,
  context: IntruderBuiltInPayloadRecommendationContext,
  keywords: string[],
): number {
  let score = 0

  if (definition.recommendedAttackTypes.includes(context.attackType)) {
    score += 4
  }

  keywords.forEach((keyword) => {
    if (definition.recommendedPositionHints.includes(keyword)) {
      score += 5
    }
    if (definition.aliases.includes(keyword) || definition.tags.includes(keyword)) {
      score += 3
    }
  })

  return score
}

function collectIntruderRecommendationKeywords(
  context: IntruderBuiltInPayloadRecommendationContext,
): string[] {
  const joinedValues = context.positionValues.join(' ')
  const normalizedRequest = context.requestText.toLowerCase()
  const keywords = new Set<string>()

  for (const [canonical, synonyms] of Object.entries(SEARCH_SYNONYMS)) {
    if (
      joinedValues.toLowerCase().includes(canonical)
      || normalizedRequest.includes(canonical)
      || synonyms.some((synonym) => joinedValues.toLowerCase().includes(synonym) || normalizedRequest.includes(synonym))
    ) {
      keywords.add(canonical)
      synonyms.forEach((synonym) => keywords.add(synonym))
    }
  }

  if (/\d{11}/.test(joinedValues)) {
    keywords.add('phone')
  }
  if (joinedValues.includes('@')) {
    keywords.add('email')
  }
  if (/[\\/]/.test(joinedValues)) {
    keywords.add('path')
  }

  return Array.from(keywords)
}
