export interface DictionarySubtypeOption {
  value: string
  label: string
}

interface DictionarySubtypeDefinition {
  label: string
  dictionaryType: 'fingerprint_rule' | 'service_probe_rule' | 'poc_rule'
  hint: string
  emptyStateHintKey: 'emptyServiceRuleHint' | 'emptyWebRuleHint' | 'emptyGenericRuleHint'
  badgeClass: string
  recommendedServiceType?: string
  consistencyHint?: (serviceType: string) => string
  supportsNmapImport?: boolean
}

const subtypeDefinitions: Record<string, DictionarySubtypeDefinition> = {
  service_identification: {
    label: '服务识别',
    dictionaryType: 'service_probe_rule',
    hint: '用于服务识别、Banner 匹配和协议探测规则，推荐选择与目标服务接近的服务类型。',
    emptyStateHintKey: 'emptyServiceRuleHint',
    badgeClass: 'badge-info',
    recommendedServiceType: 'generic',
    supportsNmapImport: true,
    consistencyHint: serviceType =>
      serviceType === 'web'
        ? '服务识别规则通常更适合“通用服务”或具体协议类型；如果只用于 Web 站点识别，更建议使用 Web 指纹。'
        : '',
  },
  web_fingerprint: {
    label: 'Web 指纹',
    dictionaryType: 'fingerprint_rule',
    hint: '用于网站技术栈与产品识别，通常建议服务类型使用“网站服务”。',
    emptyStateHintKey: 'emptyWebRuleHint',
    badgeClass: 'badge-secondary',
    recommendedServiceType: 'web',
    consistencyHint: serviceType =>
      serviceType !== 'web'
        ? '当前子类型通常更适合“网站服务”，如有特殊场景可继续保留当前配置。'
        : '',
  },
  favicon_fingerprint: {
    label: 'Favicon 指纹',
    dictionaryType: 'fingerprint_rule',
    hint: '用于基于 favicon 哈希识别网站产品，通常建议服务类型使用“网站服务”。',
    emptyStateHintKey: 'emptyWebRuleHint',
    badgeClass: 'badge-outline',
    recommendedServiceType: 'web',
    consistencyHint: serviceType =>
      serviceType !== 'web'
        ? '当前子类型通常更适合“网站服务”，如有特殊场景可继续保留当前配置。'
        : '',
  },
  generic_fingerprint: {
    label: '通用指纹',
    dictionaryType: 'fingerprint_rule',
    hint: '用于不局限于单一协议或站点形态的通用指纹规则。',
    emptyStateHintKey: 'emptyGenericRuleHint',
    badgeClass: 'badge-ghost',
  },
  risk_verification: {
    label: '风险验证',
    dictionaryType: 'poc_rule',
    hint: '用于漏洞验证与风险确认规则，不要求绑定具体服务类型。',
    emptyStateHintKey: 'emptyGenericRuleHint',
    badgeClass: 'badge-warning',
  },
}

export function getSubtypeOptions(dictionaryType: string): DictionarySubtypeOption[] {
  return Object.entries(subtypeDefinitions)
    .filter(([, definition]) => definition.dictionaryType === dictionaryType)
    .map(([value, definition]) => ({
      value,
      label: definition.label,
    }))
}

export function getSubtypeHint(subtype: string): string {
  return subtypeDefinitions[subtype]?.hint || ''
}

export function getSubtypeConsistencyHint(subtype: string, serviceType: string): string {
  if (!subtype || !serviceType) return ''
  return subtypeDefinitions[subtype]?.consistencyHint?.(serviceType) || ''
}

export function getSubtypeRecommendedServiceType(subtype: string): string {
  return subtypeDefinitions[subtype]?.recommendedServiceType || ''
}

export function getSubtypeEmptyStateHintKey(subtype: string | null): string {
  return subtype && subtypeDefinitions[subtype]
    ? subtypeDefinitions[subtype].emptyStateHintKey
    : 'emptyGenericRuleHint'
}

export function subtypeSupportsNmapImport(subtype: string | null): boolean {
  return Boolean(subtype && subtypeDefinitions[subtype]?.supportsNmapImport)
}

export function getSubtypeLabel(subtype: string | null): string {
  return subtype && subtypeDefinitions[subtype]
    ? subtypeDefinitions[subtype].label
    : ''
}

export function getSubtypeBadgeClass(subtype: string | null): string {
  return subtype && subtypeDefinitions[subtype]
    ? subtypeDefinitions[subtype].badgeClass
    : 'badge-ghost'
}
