import { invoke } from '@tauri-apps/api/core'
import i18n from '@/i18n'
import type { CodecExportData } from './trafficCodecTypes'

const VALID_EXPORT_FORMATS = ['sentinel-codec-rules', 'sentinel-traffic-codec-rules'] as const

function t(key: string): string {
  return i18n.global.t(key)
}

export async function exportCodecRules(ruleIds: string[]): Promise<string> {
  const response = await invoke<{ data: CodecExportData }>('codec_export_rules', { ruleIds })
  return JSON.stringify(response.data, null, 2)
}

export async function importCodecRules(json: string, keyValues: Record<string, string>): Promise<string[]> {
  const data: CodecExportData = JSON.parse(json)
  if (!VALID_EXPORT_FORMATS.includes(data.format as (typeof VALID_EXPORT_FORMATS)[number])) {
    throw new Error(t('trafficAnalysis.codec.importDialog.invalidFormat'))
  }
  const response = await invoke<{ data: string[] }>('codec_import_rules', { data, keyValues })
  return response.data
}

export function downloadAsFile(content: string, filename: string) {
  const blob = new Blob([content], { type: 'application/json' })
  const url = URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.href = url
  a.download = filename
  a.click()
  URL.revokeObjectURL(url)
}

export function parseCodecExportJson(json: string): CodecExportData {
  const data: CodecExportData = JSON.parse(json)
  if (!VALID_EXPORT_FORMATS.includes(data.format as (typeof VALID_EXPORT_FORMATS)[number])) {
    throw new Error(t('trafficAnalysis.codec.importDialog.invalidFormat'))
  }
  return data
}
