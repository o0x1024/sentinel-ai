import { open, save } from '@tauri-apps/plugin-dialog'
import { readTextFile, writeTextFile } from '@tauri-apps/plugin-fs'
import {
  loadLlmSuitesFromConfig,
  saveLlmSuitesToConfig,
  type LlmSuiteDefinition,
} from '../../api/llmTest'

export interface LlmSuiteImportPreview {
  open: boolean
  formatVersion: string
  candidates: LlmSuiteDefinition[]
  conflictIds: string[]
  invalidCount: number
}

export const persistSuites = async (suites: LlmSuiteDefinition[]) => {
  await saveLlmSuitesToConfig(suites)
}

export const loadSuites = async (): Promise<LlmSuiteDefinition[]> => {
  try {
    return await loadLlmSuitesFromConfig()
  } catch {
    return []
  }
}

export const exportSuitesJson = async (params: {
  suites: LlmSuiteDefinition[]
  exportFileLabel: string
  title: string
}) => {
  const suites = params.suites.map((suite) => ({
    id: suite.id,
    name: suite.name,
    version: suite.version,
    description: suite.description || '',
    cases: Array.isArray(suite.cases) ? suite.cases : [],
  }))

  const selected = await save({
    defaultPath: `llm_test_suites_${new Date().toISOString().split('T')[0]}.json`,
    filters: [{ name: params.exportFileLabel, extensions: ['json'] }],
    title: params.title,
  })
  if (!selected) return

  await writeTextFile(
    selected,
    JSON.stringify(
      {
        format_version: '1.0',
        exported_at: new Date().toISOString(),
        suites,
      },
      null,
      2,
    ),
  )
}

export const importSuitesJson = async (existingSuites: LlmSuiteDefinition[]): Promise<LlmSuiteImportPreview | null> => {
  const selected = await open({
    directory: false,
    multiple: false,
    filters: [{ name: 'JSON', extensions: ['json'] }],
    title: 'Import LLM Test Suites',
  })
  if (!selected) return null

  const content = await readTextFile(selected as string)
  const parsed = JSON.parse(content)
  const decoded = decodeImportPayload(parsed)
  const existingIds = new Set(existingSuites.map((suite) => suite.id))

  return {
    open: true,
    formatVersion: decoded.formatVersion,
    candidates: decoded.candidates,
    conflictIds: decoded.candidates.filter((suite) => existingIds.has(suite.id)).map((suite) => suite.id),
    invalidCount: decoded.invalidCount,
  }
}

export const applyImportedSuites = (params: {
  existingSuites: LlmSuiteDefinition[]
  selectedSuiteIds: string[]
  candidates: LlmSuiteDefinition[]
  overwriteConflicts: boolean
}) => {
  const existingMap = new Map(params.existingSuites.map((suite) => [suite.id, suite] as const))
  const selectedSuiteIds = [...params.selectedSuiteIds]

  for (const suite of params.candidates) {
    if (existingMap.has(suite.id) && !params.overwriteConflicts) continue
    existingMap.set(suite.id, suite)
    if (!selectedSuiteIds.includes(suite.id)) {
      selectedSuiteIds.push(suite.id)
    }
  }

  return {
    suites: Array.from(existingMap.values()),
    selectedSuiteIds,
  }
}

export const decodeImportPayload = (
  parsed: any,
): { formatVersion: string; candidates: LlmSuiteDefinition[]; invalidCount: number } => {
  let rawSuites: any[] = []
  let formatVersion = 'legacy-array'

  if (Array.isArray(parsed)) {
    rawSuites = parsed
  } else if (parsed && typeof parsed === 'object') {
    formatVersion = String(parsed.format_version || 'unknown')
    if (Array.isArray(parsed.suites)) rawSuites = parsed.suites
    else throw new Error('Invalid import payload: suites array missing')
  } else {
    throw new Error('Invalid JSON payload')
  }

  const unique = new Map<string, LlmSuiteDefinition>()
  let invalidCount = 0

  for (const item of rawSuites) {
    const id = String(item?.id || '').trim()
    const name = String(item?.name || '').trim()
    const version = String(item?.version || '').trim()
    if (!id || !name || !version) {
      invalidCount += 1
      continue
    }

    unique.set(id, {
      id,
      name,
      version,
      description: item?.description ? String(item.description) : '',
      cases: Array.isArray(item?.cases)
        ? item.cases
            .filter((entry: any) => {
              if (!entry?.case_id) return false
              if (typeof entry?.user_prompt === 'string' && entry.user_prompt.trim().length > 0) return true
              if (!Array.isArray(entry?.messages)) return false
              return entry.messages.some((message: any) => message?.role && typeof message?.content === 'string')
            })
            .map((entry: any) => ({
              case_id: String(entry.case_id),
              owasp_id: entry.owasp_id ? String(entry.owasp_id) : '',
              owasp_title: entry.owasp_title ? String(entry.owasp_title) : '',
              user_prompt: entry.user_prompt ? String(entry.user_prompt) : '',
              messages: Array.isArray(entry.messages)
                ? entry.messages
                    .filter((message: any) => message?.role && typeof message?.content === 'string')
                    .map((message: any) => ({
                      role: String(message.role),
                      content: String(message.content),
                    }))
                : undefined,
              regex_not_match: entry.regex_not_match ? String(entry.regex_not_match) : '',
            }))
        : [],
    })
  }

  return {
    formatVersion,
    candidates: Array.from(unique.values()),
    invalidCount,
  }
}

export const casePromptPreview = (entry: NonNullable<LlmSuiteDefinition['cases']>[number]) => {
  if (entry.user_prompt) return entry.user_prompt
  if (Array.isArray(entry.messages)) {
    const lastUser = [...entry.messages].reverse().find((message) => message.role === 'user')
    if (lastUser?.content) return lastUser.content
  }
  return '-'
}
