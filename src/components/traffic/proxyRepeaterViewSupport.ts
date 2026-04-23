import type { RepeaterCompareLabels } from './proxyRepeaterTypes'

type TranslateFn = (key: string, params?: Record<string, unknown>) => string

function isZhLocale(locale: string) {
  return locale.startsWith('zh')
}

export function buildRepeaterRequestViewTabs(t: TranslateFn, locale: string) {
  return [
    {
      value: 'pretty',
      label: t('trafficAnalysis.repeater.contextMenu.pretty'),
      shortLabel: isZhLocale(locale) ? '格式' : 'Fmt',
    },
    {
      value: 'raw',
      label: t('trafficAnalysis.repeater.contextMenu.raw'),
      shortLabel: isZhLocale(locale) ? '原始' : 'Raw',
    },
    {
      value: 'hex',
      label: t('trafficAnalysis.repeater.contextMenu.hex'),
      shortLabel: 'Hex',
    },
  ]
}

export function buildRepeaterResponseViewTabs(t: TranslateFn, locale: string) {
  return [
    {
      value: 'pretty',
      label: t('trafficAnalysis.repeater.contextMenu.pretty'),
      shortLabel: isZhLocale(locale) ? '格式' : 'Fmt',
    },
    {
      value: 'raw',
      label: t('trafficAnalysis.repeater.contextMenu.raw'),
      shortLabel: isZhLocale(locale) ? '原始' : 'Raw',
    },
    {
      value: 'hex',
      label: t('trafficAnalysis.repeater.contextMenu.hex'),
      shortLabel: 'Hex',
    },
    {
      value: 'render',
      label: t('trafficAnalysis.repeater.contextMenu.render'),
      shortLabel: isZhLocale(locale) ? '渲染' : 'View',
    },
  ]
}

export function buildRepeaterCompareLabels(t: TranslateFn): RepeaterCompareLabels {
  return {
    defaultName: t('trafficAnalysis.tabs.repeater'),
    requestVersions: t('trafficAnalysis.repeater.compare.requestVersions'),
    responseVersions: t('trafficAnalysis.repeater.compare.responseVersions'),
    originalRequest: t('trafficAnalysis.repeater.compare.originalRequest'),
    currentRequest: t('trafficAnalysis.repeater.compare.currentRequest'),
    previousResponse: t('trafficAnalysis.repeater.compare.previousResponse'),
    currentResponse: t('trafficAnalysis.repeater.compare.currentResponse'),
  }
}
