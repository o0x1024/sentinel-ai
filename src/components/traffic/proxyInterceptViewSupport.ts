import type { FilterRule } from './proxyInterceptTypes'

type TranslateFn = (key: string) => string

export function buildInterceptViewTabs(t: TranslateFn, locale: string) {
  return [
    {
      value: 'pretty',
      label: t('trafficAnalysis.intercept.tabs.pretty'),
      shortLabel: locale.startsWith('zh') ? '格式' : 'Fmt',
    },
    {
      value: 'raw',
      label: t('trafficAnalysis.intercept.tabs.raw'),
      shortLabel: locale.startsWith('zh') ? '原始' : 'Raw',
    },
    {
      value: 'hex',
      label: t('trafficAnalysis.intercept.tabs.hex'),
      shortLabel: 'Hex',
    },
  ]
}

export function createDefaultInterceptFilterRule(): FilterRule {
  return {
    type: 'request',
    matchType: 'domain',
    relationship: 'matches',
    condition: '',
    action: 'exclude',
  }
}
