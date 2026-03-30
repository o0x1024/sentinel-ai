export interface DictionarySubtypeView {
  id: string
  name: string
  dict_type: string
  category?: string | null
  tags?: string[] | string
  metadata?: string | Record<string, any> | null
}

export function normalizeDictionaryTags(dictionary: DictionarySubtypeView): string[] {
  if (Array.isArray(dictionary.tags)) {
    return dictionary.tags.map(tag => String(tag).trim().toLowerCase()).filter(Boolean)
  }
  if (typeof dictionary.tags === 'string') {
    return dictionary.tags.split(',').map(tag => tag.trim().toLowerCase()).filter(Boolean)
  }
  return []
}

export function parseDictionaryMetadata(dictionary: DictionarySubtypeView): Record<string, any> {
  if (!dictionary.metadata) return {}
  if (typeof dictionary.metadata === 'string') {
    try {
      return JSON.parse(dictionary.metadata)
    } catch {
      return {}
    }
  }
  return typeof dictionary.metadata === 'object' ? dictionary.metadata : {}
}

export function inferDictionarySubtypeKey(dictionary: DictionarySubtypeView): string | null {
  const metadata = parseDictionaryMetadata(dictionary)
  if (typeof metadata.subtype === 'string' && metadata.subtype.trim()) {
    return metadata.subtype.trim()
  }

  const dictId = dictionary.id.toLowerCase()
  const dictName = dictionary.name.toLowerCase()
  const tags = normalizeDictionaryTags(dictionary)
  const hasMarker = (...markers: string[]) =>
    markers.some(marker =>
      dictId.includes(marker) ||
      dictName.includes(marker) ||
      tags.includes(marker)
    )

  if (dictionary.dict_type === 'service_probe_rule') {
    return 'service_identification'
  }

  if (dictionary.dict_type === 'fingerprint_rule') {
    if (hasMarker('service', 'banner')) return 'service_identification'
    if (hasMarker('favicon')) return 'favicon_fingerprint'
    if (hasMarker('web', 'technology', 'tech')) return 'web_fingerprint'
    return 'generic_fingerprint'
  }

  if (dictionary.dict_type === 'poc_rule') {
    if (hasMarker('verification', 'poc') || dictionary.category === 'risk') {
      return 'risk_verification'
    }
  }

  return null
}
