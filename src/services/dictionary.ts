// src/services/dictionary.ts
// Utilities for managing default dictionaries per type (DB-backed via Tauri commands)

import { invoke } from '@tauri-apps/api/core'

export type DictionaryType = string
export type DefaultMap = Record<DictionaryType, string>

export async function getDefaultMap(): Promise<DefaultMap> {
  return (await invoke('get_default_dictionary_map')) as DefaultMap
}

export async function getDefaultId(dict_type: DictionaryType): Promise<string | null> {
  const id = (await invoke('get_default_dictionary_id', { dict_type })) as string | null
  return id && id.length > 0 ? id : null
}

export async function setDefaultId(dict_type: DictionaryType, id: string): Promise<void> {
  await invoke('set_default_dictionary', { dict_type, dictionary_id: id })
}

export async function clearDefaultForType(dict_type: DictionaryType): Promise<void> {
  await invoke('clear_default_dictionary', { dict_type })
}

export async function isDefault(dict_type: DictionaryType, id: string): Promise<boolean> {
  const current = await getDefaultId(dict_type)
  return current === id
}

// Helper for consumers: pick default id or fallback to first available
export async function pickDefaultDictionaryId<T extends { id: string; dict_type: string }>(
  dict_type: DictionaryType,
  list: T[]
): Promise<string | null> {
  const preferred = await getDefaultId(dict_type)
  if (preferred && list.some(d => d.id === preferred)) return preferred
  const first = list.find(d => d.dict_type === dict_type)
  return first ? first.id : null
}

export async function getDefaultOrFirst<T extends { id: string; dict_type: string }>(
  dict_type: DictionaryType,
  list: T[]
): Promise<T | null> {
  const preferred = await getDefaultId(dict_type)
  if (preferred) {
    const hit = list.find(d => d.id === preferred)
    if (hit) return hit
  }
  return list.find(d => d.dict_type === dict_type) ?? null
}
