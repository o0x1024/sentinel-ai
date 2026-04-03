import { computed, ref, watch } from 'vue'

export type TrafficMessageType = 'request' | 'response' | 'generic'
export type TrafficMessageViewTab = 'pretty' | 'raw'
export type TrafficCharsetMode = 'auto' | 'platformDefault' | 'rawBytes' | 'specific'

export interface TrafficDisplaySettings {
  fontFamily: string
  fontSize: number
  highlightRequestSyntax: boolean
  highlightResponseSyntax: boolean
  prettyPrintByDefault: boolean
  charsetMode: TrafficCharsetMode
  specificCharset: string
}

interface TrafficFontOption {
  label: string
  value: string
}

interface TrafficCharsetOption {
  label: string
  value: string
}

const TRAFFIC_DISPLAY_SETTINGS_STORAGE_KEY = 'trafficAnalysis.displaySettings.v1'

const DEFAULT_TRAFFIC_DISPLAY_SETTINGS: TrafficDisplaySettings = {
  fontFamily: 'ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", "Courier New", monospace',
  fontSize: 13,
  highlightRequestSyntax: true,
  highlightResponseSyntax: true,
  prettyPrintByDefault: true,
  charsetMode: 'auto',
  specificCharset: 'UTF-8',
}

export const TRAFFIC_FONT_OPTIONS: TrafficFontOption[] = [
  {
    label: 'Monospaced',
    value: 'ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", "Courier New", monospace',
  },
  {
    label: 'JetBrains Mono',
    value: '"JetBrains Mono", ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace',
  },
  {
    label: 'Fira Code',
    value: '"Fira Code", ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace',
  },
  {
    label: 'Consolas',
    value: 'Consolas, Monaco, "Courier New", monospace',
  },
  {
    label: 'Menlo',
    value: 'Menlo, Monaco, Consolas, "Courier New", monospace',
  },
]

export const TRAFFIC_CHARSET_OPTIONS: TrafficCharsetOption[] = [
  { label: 'UTF-8', value: 'UTF-8' },
  { label: 'GB18030', value: 'GB18030' },
  { label: 'GBK', value: 'GBK' },
  { label: 'Big5', value: 'Big5' },
  { label: 'Shift_JIS', value: 'Shift_JIS' },
  { label: 'EUC-KR', value: 'EUC-KR' },
  { label: 'windows-1252', value: 'windows-1252' },
  { label: 'ISO-8859-1', value: 'ISO-8859-1' },
]

function clampFontSize(value: number): number {
  if (!Number.isFinite(value)) return DEFAULT_TRAFFIC_DISPLAY_SETTINGS.fontSize
  return Math.min(20, Math.max(10, Math.round(value)))
}

function normalizeTrafficDisplaySettings(value?: Partial<TrafficDisplaySettings>): TrafficDisplaySettings {
  const normalized = {
    ...DEFAULT_TRAFFIC_DISPLAY_SETTINGS,
    ...(value || {}),
  }
  const fontFamily = typeof normalized.fontFamily === 'string' && normalized.fontFamily.trim()
    ? normalized.fontFamily
    : DEFAULT_TRAFFIC_DISPLAY_SETTINGS.fontFamily
  const specificCharset = typeof normalized.specificCharset === 'string' && normalized.specificCharset.trim()
    ? normalized.specificCharset
    : DEFAULT_TRAFFIC_DISPLAY_SETTINGS.specificCharset
  const charsetMode: TrafficCharsetMode = ['auto', 'platformDefault', 'rawBytes', 'specific'].includes(normalized.charsetMode)
    ? normalized.charsetMode
    : DEFAULT_TRAFFIC_DISPLAY_SETTINGS.charsetMode

  return {
    fontFamily,
    fontSize: clampFontSize(normalized.fontSize),
    highlightRequestSyntax: Boolean(normalized.highlightRequestSyntax),
    highlightResponseSyntax: Boolean(normalized.highlightResponseSyntax),
    prettyPrintByDefault: Boolean(normalized.prettyPrintByDefault),
    charsetMode,
    specificCharset,
  }
}

function loadTrafficDisplaySettings(): TrafficDisplaySettings {
  try {
    const raw = localStorage.getItem(TRAFFIC_DISPLAY_SETTINGS_STORAGE_KEY)
    if (!raw) return { ...DEFAULT_TRAFFIC_DISPLAY_SETTINGS }
    return normalizeTrafficDisplaySettings(JSON.parse(raw))
  } catch {
    return { ...DEFAULT_TRAFFIC_DISPLAY_SETTINGS }
  }
}

const trafficDisplaySettings = ref<TrafficDisplaySettings>(loadTrafficDisplaySettings())

watch(
  trafficDisplaySettings,
  (value) => {
    localStorage.setItem(TRAFFIC_DISPLAY_SETTINGS_STORAGE_KEY, JSON.stringify(value))
  },
  { deep: true },
)

export function useTrafficDisplaySettings() {
  const defaultMessageViewTab = computed<TrafficMessageViewTab>(() =>
    trafficDisplaySettings.value.prettyPrintByDefault ? 'pretty' : 'raw',
  )

  return {
    settings: trafficDisplaySettings,
    fontOptions: TRAFFIC_FONT_OPTIONS,
    charsetOptions: TRAFFIC_CHARSET_OPTIONS,
    defaultMessageViewTab,
  }
}

export function getDefaultTrafficMessageViewTab(): TrafficMessageViewTab {
  return trafficDisplaySettings.value.prettyPrintByDefault ? 'pretty' : 'raw'
}

export function shouldHighlightTrafficMessageSyntax(messageType: TrafficMessageType): boolean {
  if (messageType === 'request') return trafficDisplaySettings.value.highlightRequestSyntax
  if (messageType === 'response') return trafficDisplaySettings.value.highlightResponseSyntax
  return trafficDisplaySettings.value.highlightRequestSyntax || trafficDisplaySettings.value.highlightResponseSyntax
}
