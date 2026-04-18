import { invoke } from '@tauri-apps/api/core'

const DEFAULT_DICTIONARY_PREFIX = 'default:'
const DICTIONARY_TYPE_TRANSLATION_KEYS: Record<string, string> = {
  http_param: 'parameter',
  sql_injection_payload: 'sql_injection',
}

export const INTRUDER_DICTIONARY_TYPE_OPTIONS = [
  'subdomain',
  'username',
  'password',
  'path',
  'filename',
  'extension',
  'port',
  'api_endpoint',
  'sensitive_file',
  'service_probe_rule',
  'fingerprint_rule',
  'poc_rule',
  'http_param',
  'xss_payload',
  'sql_injection_payload',
  'custom',
] as const

export interface IntruderDictionarySummary {
  id: string
  name: string
  description?: string | null
  dict_type: string
  service_type?: string | null
  is_builtin?: boolean
  is_active?: boolean
  word_count?: number
  updated_at?: string
}

interface DictionaryPageResponse {
  items: IntruderDictionarySummary[]
  total: number
}

export interface IntruderDictionaryPage {
  items: IntruderDictionarySummary[]
  total: number
}

export function isIntruderDefaultDictionaryReference(value: string): boolean {
  return value.trim().startsWith(DEFAULT_DICTIONARY_PREFIX)
}

export function parseIntruderDefaultDictionaryReference(value: string): string | null {
  if (!isIntruderDefaultDictionaryReference(value)) {
    return null
  }

  const dictType = value.trim().slice(DEFAULT_DICTIONARY_PREFIX.length).trim()
  return dictType || null
}

export function createIntruderDefaultDictionaryReference(dictType: string): string {
  return `${DEFAULT_DICTIONARY_PREFIX}${dictType.trim()}`
}

export function getIntruderDictionaryTypeTranslationKey(dictType: string): string {
  const normalizedType = dictType.trim()
  return DICTIONARY_TYPE_TRANSLATION_KEYS[normalizedType] || normalizedType
}

export async function listIntruderDictionaries(options: {
  dictType?: string | null
  searchTerm?: string | null
  limit?: number
} = {}): Promise<IntruderDictionarySummary[]> {
  const page = await listIntruderDictionariesPaged({
    dictType: options.dictType,
    searchTerm: options.searchTerm,
    offset: 0,
    limit: options.limit ?? 200,
  })

  return page.items
}

export async function listIntruderDictionariesPaged(options: {
  dictType?: string | null
  searchTerm?: string | null
  offset?: number
  limit?: number
} = {}): Promise<IntruderDictionaryPage> {
  const result = await invoke<DictionaryPageResponse>('get_dictionaries_paged', {
    dict_type: options.dictType || null,
    service_type: null,
    category: null,
    is_builtin: null,
    is_active: null,
    search_term: options.searchTerm || null,
    subtype: null,
    offset: options.offset ?? 0,
    limit: options.limit ?? 200,
  })

  return {
    items: Array.isArray(result?.items) ? result.items : [],
    total: Number(result?.total) || 0,
  }
}

export async function listIntruderDictionaryWords(
  dictionaryId: string,
  limit = 200,
): Promise<string[]> {
  const result = await invoke<Array<{ word?: string | null }>>('get_dictionary_words_paged', {
    dictionary_id: dictionaryId,
    offset: 0,
    limit,
    pattern: null,
  })

  if (!Array.isArray(result)) {
    return []
  }

  return result
    .map((item) => (typeof item?.word === 'string' ? item.word.trim() : ''))
    .filter(Boolean)
}

export async function getIntruderDefaultDictionaryId(dictType: string): Promise<string | null> {
  const result = await invoke<string | null>('get_default_dictionary_id', {
    dict_type: dictType,
  })
  return typeof result === 'string' && result.trim() ? result.trim() : null
}

export async function getIntruderDefaultDictionaryMap(): Promise<Record<string, string>> {
  const result = await invoke<Record<string, string>>('get_default_dictionary_map')
  return result && typeof result === 'object' ? result : {}
}
