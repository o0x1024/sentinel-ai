import { invoke } from '@tauri-apps/api/core'

import type { TrafficContextExtractionSettings } from '@/components/traffic/proxyConfigurationTypes'
import type {
  RecommendTrafficContextDictionaryCandidatesResponse,
  TrafficContextDictionaryCandidate,
  TrafficContextExtractionPreviewResponse,
} from '@/components/traffic/trafficContextCandidateTypes'

type CommandResponse<T> = {
  success: boolean
  data?: T
  error?: string | null
}

type RecommendPayload = {
  requestIds: number[]
  includeBehaviorContext?: boolean
  maxCandidatesPerCategory?: number
}

type PreviewPayload = {
  requestIds: number[]
  currentSettings: TrafficContextExtractionSettings
  previewSettings: TrafficContextExtractionSettings
  sampleLimit?: number
}

function assertSuccess<T>(response: CommandResponse<T>): T {
  if (!response.success || !response.data) {
    throw new Error(response.error || '请求失败')
  }
  return response.data
}

export async function recommendTrafficContextDictionaryCandidates(payload: RecommendPayload) {
  const response = await invoke<CommandResponse<RecommendTrafficContextDictionaryCandidatesResponse>>(
    'recommend_traffic_context_dictionary_candidates_command',
    { payload },
  )
  return assertSuccess(response)
}

export async function getTrafficContextExtractionSettings() {
  const response = await invoke<CommandResponse<TrafficContextExtractionSettings>>(
    'get_traffic_context_extraction_settings',
  )
  return assertSuccess(response)
}

export async function previewTrafficContextExtractionChanges(payload: PreviewPayload) {
  const response = await invoke<CommandResponse<TrafficContextExtractionPreviewResponse>>(
    'preview_traffic_context_extraction_changes_command',
    { payload },
  )
  return assertSuccess(response)
}

export async function setTrafficContextExtractionSettings(settings: TrafficContextExtractionSettings) {
  const response = await invoke<CommandResponse<TrafficContextExtractionSettings>>(
    'set_traffic_context_extraction_settings',
    {
      payload: {
        settings,
      },
    },
  )
  return assertSuccess(response)
}

export function mergeCandidatesIntoTrafficContextExtractionSettings(
  settings: TrafficContextExtractionSettings,
  candidates: TrafficContextDictionaryCandidate[],
) {
  const next: TrafficContextExtractionSettings = {
    principalKeys: [...settings.principalKeys],
    resourceKeyHints: [...settings.resourceKeyHints],
    authHeaderKeys: [...settings.authHeaderKeys],
    authTokenKeys: [...settings.authTokenKeys],
    cookieHintKeys: [...settings.cookieHintKeys],
    actionAliases: Object.fromEntries(
      Object.entries(settings.actionAliases).map(([action, aliases]) => [action, [...aliases]]),
    ),
  }

  for (const candidate of candidates) {
    switch (candidate.category) {
      case 'principal':
        appendUniqueValue(next.principalKeys, candidate.key)
        break
      case 'resource':
        appendUniqueValue(next.resourceKeyHints, candidate.key)
        break
      case 'auth_header':
        appendUniqueValue(next.authHeaderKeys, candidate.key)
        break
      case 'auth_token':
        appendUniqueValue(next.authTokenKeys, candidate.key)
        break
      case 'cookie_hint':
        appendUniqueValue(next.cookieHintKeys, candidate.key)
        break
      case 'action_alias': {
        const action = candidate.suggestedCanonicalAction?.trim().toLowerCase()
        if (!action) break
        const aliases = next.actionAliases[action] || []
        appendUniqueValue(aliases, candidate.key)
        next.actionAliases[action] = aliases
        break
      }
    }
  }

  return next
}

function appendUniqueValue(target: string[], value: string) {
  const trimmed = value.trim()
  if (!trimmed) return
  const normalized = normalizeLookupKey(trimmed)
  if (!normalized) return
  if (target.some(item => normalizeLookupKey(item) === normalized)) {
    return
  }
  target.push(trimmed)
}

function normalizeLookupKey(raw: string) {
  return raw
    .split('')
    .filter(ch => /[a-z0-9]/i.test(ch))
    .join('')
    .toLowerCase()
}
