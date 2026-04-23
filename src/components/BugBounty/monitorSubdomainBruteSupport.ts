import { invoke } from '@tauri-apps/api/core'

export interface MonitorSubdomainDictionarySummary {
  id: string
  name: string
  dict_type?: string | null
  word_count?: number | null
}

export const normalizeSubdomainBruteInlineWords = (value: unknown) => {
  const lines = Array.isArray(value)
    ? value
    : typeof value === 'string'
      ? value.split(/\r?\n|,/)
      : []

  return Array.from(new Set(lines.map(item => String(item || '').trim()).filter(Boolean)))
}

export const listMonitorSubdomainDictionaries = async () => {
  const result = await invoke<MonitorSubdomainDictionarySummary[]>('get_dictionaries', {
    dict_type: 'subdomain',
    service_type: null,
    category: null,
    is_builtin: null,
    is_active: true,
    search_term: null,
  })

  return Array.isArray(result) ? result : []
}
