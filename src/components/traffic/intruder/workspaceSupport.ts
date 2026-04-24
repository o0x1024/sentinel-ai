import {
  createDefaultResultFilter,
  createDefaultResultSort,
  normalizeIntruderResultFilter,
} from './results'
import {
  createDefaultVisibleColumns,
  normalizeGrepMatchRule,
  normalizeGrepPayloadSettings,
} from './analysis'
import {
  createDefaultIntruderDictionaryPayloadConfig,
  normalizeIntruderDictionaryPayloadConfig,
} from './intruderAppDictionaryPayloads'
import {
  createIntruderId,
  createRawRequestFromSource,
  extractTargetFromRequest,
} from './http'
import type {
  IntruderAttackOptions,
  IntruderAttackProgress,
  IntruderAttackResult,
  IntruderAttackType,
  IntruderGrepExtractRule,
  IntruderGrepMatchRule,
  IntruderGrepPayloadSettings,
  IntruderPluginProcessorBinding,
  IntruderPayloadProcessingRule,
  IntruderPayloadSet,
  IntruderPosition,
  IntruderRequestInput,
  IntruderRequestViewTab,
  IntruderResourcePool,
  IntruderResultFilter,
  IntruderResultSort,
  IntruderTarget,
} from './types'
import { extractIntruderPositions } from './attack'
import { normalizeVisibleColumns } from './storage'

export interface IntruderWorkspace {
  id: string
  name: string
  sourceRequestId: number | null
  requestText: string
  requestViewTab: IntruderRequestViewTab
  target: IntruderTarget
  positions: IntruderPosition[]
  attackType: IntruderAttackType
  payloadSets: IntruderPayloadSet[]
  payloadProcessingRules: IntruderPayloadProcessingRule[]
  payloadProcessorPlugins: IntruderPluginProcessorBinding[]
  requestProcessorPlugins: IntruderPluginProcessorBinding[]
  grepMatchRules: IntruderGrepMatchRule[]
  grepExtractRules: IntruderGrepExtractRule[]
  grepPayloadSettings: IntruderGrepPayloadSettings
  selectedResourcePoolId: string
  attackOptions: IntruderAttackOptions
  results: IntruderAttackResult[]
  selectedResultId: string | null
  progress: IntruderAttackProgress
  isRunning: boolean
  captureFilter: IntruderResultFilter
  viewFilter: IntruderResultFilter
  sort: IntruderResultSort
  visibleColumns: string[]
}

export const INTRUDER_SIDEBAR_WIDTH_KEY = 'trafficAnalysis.intruder.sidebarWidth.v1'
export const INTRUDER_SIDEBAR_MIN_WIDTH = 320
export const INTRUDER_SIDEBAR_MAX_WIDTH = 720

export function createDefaultProgress(): IntruderAttackProgress {
  return {
    total: 0,
    completed: 0,
    failed: 0,
    active: 0,
    truncated: false,
  }
}

export function createDefaultAttackOptions(): IntruderAttackOptions {
  return {
    concurrency: 10,
    delayMs: 0,
    randomDelayMs: 0,
    delayIncrementMs: 0,
    autoThrottleEnabled: false,
    autoThrottleStatusCodes: [429, 503],
    timeoutSecs: 30,
    maxRequests: 500,
    updateHostHeader: true,
    updateContentLength: true,
    setConnectionClose: true,
    followRedirects: false,
    maxRedirects: 5,
    processCookiesInRedirects: true,
    retryCount: 3,
    retryPauseMs: 2000,
    storeRequests: true,
    storeResponses: true,
    storeFullPayloads: false,
    denialOfServiceMode: false,
    makeUnmodifiedBaseline: true,
    autoPauseEnabled: false,
    autoPauseMode: 'contains',
    autoPauseExpression: '',
    autoPauseExpressions: [],
  }
}

export function normalizeAttackOptions(value?: Partial<IntruderAttackOptions>): IntruderAttackOptions {
  const defaults = createDefaultAttackOptions()
  const normalized = { ...defaults, ...(value || {}) }
  const autoThrottleStatusCodes = Array.isArray(value?.autoThrottleStatusCodes)
    ? value.autoThrottleStatusCodes
      .map(item => Number(item))
      .filter(item => Number.isInteger(item) && item >= 100 && item <= 999)
    : defaults.autoThrottleStatusCodes
  const expressions = Array.isArray(value?.autoPauseExpressions)
    ? value.autoPauseExpressions.filter(item => typeof item === 'string' && item.trim().length > 0)
    : []

  return {
    ...normalized,
    delayIncrementMs: Math.max(0, Number(normalized.delayIncrementMs) || 0),
    autoThrottleEnabled: Boolean(normalized.autoThrottleEnabled),
    autoThrottleStatusCodes,
    autoPauseExpressions: expressions.length
      ? expressions
      : normalized.autoPauseExpression.trim()
        ? [normalized.autoPauseExpression.trim()]
        : [],
  }
}

export function createDefaultPayloadSet(index: number, payloadLabel: string): IntruderPayloadSet {
  return {
    id: createIntruderId('payload-set'),
    name: `${payloadLabel} ${index + 1}`,
    payloadType: 'simpleList',
    payloadsText: '',
    urlEncode: false,
    urlEncodeCharacters: String.raw`./\=<>?+&*;:"' {}|^#`,
    dictionaryConfig: createDefaultIntruderDictionaryPayloadConfig(),
    pluginId: '',
    pluginPresetName: '',
    pluginConfig: '{}',
    filePath: '',
    bruteForceCharacterSet: 'abcdefghijklmnopqrstuvwxyz0123456789',
    bruteForceMinLength: 4,
    bruteForceMaxLength: 4,
    characterList: '',
    substitutionSource: '',
    substitutionRules: '',
    numberFrom: 0,
    numberTo: 100,
    numberStep: 1,
    numberPadWidth: 0,
    dateFrom: '2026-01-01',
    dateTo: '2026-01-07',
    dateStepDays: 1,
    dateFormat: 'yyyy-MM-dd',
    nullCount: 10,
    nullValue: '',
    usernameFirstNames: '',
    usernameLastNames: '',
    usernameFormats: '{first}.{last}\n{f}{last}\n{first}{l}',
  }
}

export function normalizePayloadSet(
  index: number,
  value: Partial<IntruderPayloadSet> | null | undefined,
  payloadLabel: string,
): IntruderPayloadSet {
  const defaults = createDefaultPayloadSet(index, payloadLabel)
  return {
    ...defaults,
    ...(value || {}),
    dictionaryConfig: normalizeIntruderDictionaryPayloadConfig(
      value?.dictionaryConfig || defaults.dictionaryConfig,
    ),
  }
}

export function createDefaultPluginProcessorBinding(): IntruderPluginProcessorBinding {
  return {
    id: createIntruderId('plugin-processor'),
    pluginId: '',
    presetName: '',
    enabled: true,
    config: '{}',
  }
}

export function createIntruderWorkspace(options: {
  source?: IntruderRequestInput
  id?: string
  name?: string
  existingCount: number
  attackLabel: string
  payloadLabel: string
}): IntruderWorkspace {
  const requestText = createRawRequestFromSource(options.source)
  const target = extractTargetFromRequest(requestText, options.source?.absoluteUrl)
  const positions = extractIntruderPositions(requestText)

  return {
    id: options.id || createIntruderId('intruder-workspace'),
    name: options.name || target.host || `${options.attackLabel} ${options.existingCount + 1}`,
    sourceRequestId: options.source?.sourceRequestId ?? null,
    requestText,
    requestViewTab: options.source?.preferredRequestView === 'pretty' ? 'pretty' : 'raw',
    target,
    positions,
    attackType: 'sniper',
    payloadSets: [createDefaultPayloadSet(0, options.payloadLabel)],
    payloadProcessingRules: [],
    payloadProcessorPlugins: [],
    requestProcessorPlugins: [],
    grepMatchRules: [],
    grepExtractRules: [],
    grepPayloadSettings: normalizeGrepPayloadSettings(),
    selectedResourcePoolId: 'default',
    attackOptions: createDefaultAttackOptions(),
    results: [],
    selectedResultId: null,
    progress: createDefaultProgress(),
    isRunning: false,
    captureFilter: createDefaultResultFilter(),
    viewFilter: createDefaultResultFilter(),
    sort: createDefaultResultSort(),
    visibleColumns: createDefaultVisibleColumns([], [], normalizeGrepPayloadSettings()),
  }
}

export function normalizePersistedIntruderWorkspaceList(
  persisted: {
    activeWorkspaceId?: string | null
    workspaces?: Array<Partial<IntruderWorkspace>>
  } | null | undefined,
  options: {
    attackLabel: string
    payloadLabel: string
  },
): {
  activeWorkspaceId: string | null
  workspaces: IntruderWorkspace[]
} | null {
  if (!Array.isArray(persisted.workspaces) || persisted.workspaces.length === 0) {
    return null
  }

  const workspaces = persisted.workspaces.map((workspace, index) => ({
    id: workspace.id || createIntruderId('intruder-workspace'),
    name: workspace.name || `${options.attackLabel} ${index + 1}`,
    sourceRequestId: workspace.sourceRequestId ?? null,
    requestText: workspace.requestText || createRawRequestFromSource(),
    requestViewTab: workspace.requestViewTab === 'pretty' ? 'pretty' : 'raw' as IntruderRequestViewTab,
    target: workspace.target || extractTargetFromRequest(workspace.requestText || ''),
    positions: workspace.positions || extractIntruderPositions(workspace.requestText || ''),
    attackType: workspace.attackType || 'sniper',
    payloadSets: (workspace.payloadSets?.length ? workspace.payloadSets : [createDefaultPayloadSet(0, options.payloadLabel)])
      .map((item, payloadIndex) => normalizePayloadSet(payloadIndex, item, options.payloadLabel)),
    payloadProcessingRules: workspace.payloadProcessingRules || [],
    payloadProcessorPlugins: (workspace.payloadProcessorPlugins || []).map(item => ({
      ...createDefaultPluginProcessorBinding(),
      ...item,
    })),
    requestProcessorPlugins: (workspace.requestProcessorPlugins || []).map(item => ({
      ...createDefaultPluginProcessorBinding(),
      ...item,
    })),
    grepMatchRules: (workspace.grepMatchRules || []).map(item => normalizeGrepMatchRule(item)),
    grepExtractRules: workspace.grepExtractRules || [],
    grepPayloadSettings: normalizeGrepPayloadSettings(workspace.grepPayloadSettings),
    selectedResourcePoolId: workspace.selectedResourcePoolId || 'default',
    attackOptions: normalizeAttackOptions(workspace.attackOptions),
    results: (workspace.results || []).map(result => ({
      ...result,
      payloadValues: [...(result.payloadValues || [])],
      redirectChain: (result.redirectChain || []).map(hop => ({ ...hop })),
      grepMatches: { ...(result.grepMatches || {}) },
      grepExtracts: { ...(result.grepExtracts || {}) },
      responseHeaders: result.responseHeaders ? result.responseHeaders.map(header => ({ ...header })) : [],
    })),
    selectedResultId: workspace.selectedResultId ?? null,
    progress: workspace.progress
      ? {
        ...createDefaultProgress(),
        ...workspace.progress,
      }
      : createDefaultProgress(),
    isRunning: Boolean(workspace.isRunning),
    captureFilter: normalizeIntruderResultFilter(workspace.captureFilter),
    viewFilter: normalizeIntruderResultFilter(workspace.viewFilter),
    sort: { ...createDefaultResultSort(), ...(workspace.sort || {}) },
    visibleColumns: normalizeVisibleColumns(
      workspace.visibleColumns,
      workspace.grepMatchRules || [],
      workspace.grepExtractRules || [],
      normalizeGrepPayloadSettings(workspace.grepPayloadSettings),
    ),
  }))

  const activeWorkspaceId = workspaces.some(workspace => workspace.id === persisted.activeWorkspaceId)
    ? persisted.activeWorkspaceId || null
    : workspaces[0]?.id ?? null

  return {
    activeWorkspaceId,
    workspaces,
  }
}

export function serializeIntruderWorkspaceSessionStore(activeWorkspaceId: string | null, workspaces: IntruderWorkspace[]) {
  return {
    activeWorkspaceId,
    workspaces: workspaces.map(workspace => ({
      id: workspace.id,
      name: workspace.name,
      sourceRequestId: workspace.sourceRequestId,
      requestText: workspace.requestText,
      requestViewTab: workspace.requestViewTab,
      target: workspace.target,
      positions: workspace.positions,
      attackType: workspace.attackType,
      payloadSets: workspace.payloadSets,
      payloadProcessingRules: workspace.payloadProcessingRules,
      payloadProcessorPlugins: workspace.payloadProcessorPlugins,
      requestProcessorPlugins: workspace.requestProcessorPlugins,
      grepMatchRules: workspace.grepMatchRules,
      grepExtractRules: workspace.grepExtractRules,
      grepPayloadSettings: workspace.grepPayloadSettings,
      selectedResourcePoolId: workspace.selectedResourcePoolId,
      attackOptions: workspace.attackOptions,
      results: workspace.results.map(result => ({
        ...result,
        payloadValues: [...result.payloadValues],
        redirectChain: result.redirectChain.map(hop => ({ ...hop })),
        grepMatches: { ...result.grepMatches },
        grepExtracts: { ...result.grepExtracts },
        responseHeaders: result.responseHeaders ? result.responseHeaders.map(header => ({ ...header })) : [],
      })),
      selectedResultId: workspace.selectedResultId,
      progress: { ...workspace.progress },
      isRunning: workspace.isRunning,
      captureFilter: workspace.captureFilter,
      viewFilter: workspace.viewFilter,
      sort: workspace.sort,
      visibleColumns: workspace.visibleColumns,
    })),
  }
}

export function loadIntruderSidebarWidth() {
  const raw = localStorage.getItem(INTRUDER_SIDEBAR_WIDTH_KEY)
  const width = raw ? Number.parseInt(raw, 10) : 448
  return Number.isFinite(width) ? width : 448
}

export function clampIntruderSidebarWidth(width: number, containerWidth: number) {
  const maxWidth = Math.max(
    INTRUDER_SIDEBAR_MIN_WIDTH,
    Math.min(INTRUDER_SIDEBAR_MAX_WIDTH, containerWidth - 360),
  )
  return Math.min(maxWidth, Math.max(INTRUDER_SIDEBAR_MIN_WIDTH, width))
}

export function buildIntruderTargetUrl(target: IntruderTarget): string {
  const protocol = target.useTls ? 'https' : 'http'
  const defaultPort = target.useTls ? 443 : 80
  const portSuffix = target.port === defaultPort ? '' : `:${target.port}`
  return `${protocol}://${target.host || 'example.com'}${portSuffix}`
}

export function parseIntruderTargetUrl(value: string): IntruderTarget | null {
  try {
    const url = new URL(value.includes('://') ? value : `https://${value}`)
    return {
      host: url.hostname,
      port: url.port ? Number.parseInt(url.port, 10) : url.protocol === 'https:' ? 443 : 80,
      useTls: url.protocol === 'https:',
    }
  } catch {
    return null
  }
}
