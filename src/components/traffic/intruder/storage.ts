import { createIntruderId } from './http'
import { createDefaultVisibleColumns, getIntruderResultColumns, getIntruderResultColumnValue } from './analysis'
import type {
  IntruderAttackOptions,
  IntruderAttackResult,
  IntruderAttackType,
  IntruderGrepExtractRule,
  IntruderGrepMatchRule,
  IntruderGrepPayloadSettings,
  IntruderPluginProcessorBinding,
  IntruderPayloadProcessingRule,
  IntruderPayloadSet,
  IntruderResourcePool,
  IntruderResultFilter,
  IntruderResultSort,
  IntruderTarget,
} from './types'

const RESOURCE_POOLS_STORAGE_KEY = 'trafficAnalysis.intruder.resourcePools.v1'
const ATTACK_TEMPLATES_STORAGE_KEY = 'trafficAnalysis.intruder.attackTemplates.v1'

export interface IntruderAttackTemplate {
  id: string
  name: string
  requestText: string
  target: IntruderTarget
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
  captureFilter: IntruderResultFilter
  viewFilter: IntruderResultFilter
  sort: IntruderResultSort
  visibleColumns: string[]
}

export function createBuiltInResourcePools(): IntruderResourcePool[] {
  return [
    {
      id: 'default',
      name: 'Default resource pool',
      concurrencyEnabled: true,
      concurrency: 10,
      delayEnabled: false,
      delayMs: 0,
      randomDelayEnabled: false,
      randomDelayMs: 0,
      delayIncrementEnabled: false,
      delayIncrementMs: 0,
      autoThrottleEnabled: false,
      autoThrottleStatusCodes: [429, 503],
      builtIn: true,
    },
  ]
}

function normalizeIntruderResourcePool(pool: Partial<IntruderResourcePool>): IntruderResourcePool {
  return {
    id: String(pool.id || '').trim() || createIntruderId('resource-pool'),
    name: String(pool.name || '').trim() || 'Resource pool',
    concurrencyEnabled: pool.concurrencyEnabled ?? true,
    concurrency: Math.max(1, Number(pool.concurrency) || 1),
    delayEnabled: pool.delayEnabled ?? false,
    delayMs: Math.max(0, Number(pool.delayMs) || 0),
    randomDelayEnabled: pool.randomDelayEnabled ?? false,
    randomDelayMs: Math.max(0, Number(pool.randomDelayMs) || 0),
    delayIncrementEnabled: pool.delayIncrementEnabled ?? false,
    delayIncrementMs: Math.max(0, Number(pool.delayIncrementMs) || 0),
    autoThrottleEnabled: pool.autoThrottleEnabled ?? false,
    autoThrottleStatusCodes: Array.isArray(pool.autoThrottleStatusCodes)
      ? pool.autoThrottleStatusCodes
        .map((value) => Number(value))
        .filter((value) => Number.isInteger(value) && value >= 100 && value <= 999)
      : [],
    builtIn: Boolean(pool.builtIn),
  }
}

export function loadIntruderResourcePools(defaultPools: IntruderResourcePool[]): IntruderResourcePool[] {
  const raw = localStorage.getItem(RESOURCE_POOLS_STORAGE_KEY)
  if (!raw) return defaultPools

  try {
    const customPools = (JSON.parse(raw) as Partial<IntruderResourcePool>[]).map((pool) => normalizeIntruderResourcePool(pool))
    const builtIns = defaultPools.map((pool) => ({ ...pool, builtIn: true }))
    return [...builtIns, ...customPools.filter((pool) => !builtIns.some((item) => item.id === pool.id))]
  } catch {
    return defaultPools
  }
}

export function persistIntruderResourcePools(resourcePools: IntruderResourcePool[]): void {
  const customPools = resourcePools.filter((pool) => !pool.builtIn)
  localStorage.setItem(RESOURCE_POOLS_STORAGE_KEY, JSON.stringify(customPools))
}

export function createIntruderResourcePool(
  name: string,
  options: Pick<
    IntruderResourcePool,
    | 'concurrencyEnabled'
    | 'concurrency'
    | 'delayEnabled'
    | 'delayMs'
    | 'randomDelayEnabled'
    | 'randomDelayMs'
    | 'delayIncrementEnabled'
    | 'delayIncrementMs'
    | 'autoThrottleEnabled'
    | 'autoThrottleStatusCodes'
  >,
): IntruderResourcePool {
  return {
    id: createIntruderId('resource-pool'),
    name,
    concurrencyEnabled: options.concurrencyEnabled,
    concurrency: options.concurrency,
    delayEnabled: options.delayEnabled,
    delayMs: options.delayMs,
    randomDelayEnabled: options.randomDelayEnabled,
    randomDelayMs: options.randomDelayMs,
    delayIncrementEnabled: options.delayIncrementEnabled,
    delayIncrementMs: options.delayIncrementMs,
    autoThrottleEnabled: options.autoThrottleEnabled,
    autoThrottleStatusCodes: options.autoThrottleStatusCodes,
    builtIn: false,
  }
}

export function loadIntruderAttackTemplates(): IntruderAttackTemplate[] {
  const raw = localStorage.getItem(ATTACK_TEMPLATES_STORAGE_KEY)
  if (!raw) return []

  try {
    return JSON.parse(raw) as IntruderAttackTemplate[]
  } catch {
    return []
  }
}

export function persistIntruderAttackTemplates(templates: IntruderAttackTemplate[]): void {
  localStorage.setItem(ATTACK_TEMPLATES_STORAGE_KEY, JSON.stringify(templates))
}

export function createIntruderAttackTemplate(
  name: string,
  value: Omit<IntruderAttackTemplate, 'id' | 'name'>,
): IntruderAttackTemplate {
  return {
    id: createIntruderId('attack-template'),
    name,
    ...value,
  }
}

export function exportIntruderResultsCsv(
  results: IntruderAttackResult[],
  grepMatchRules: IntruderGrepMatchRule[],
  grepExtractRules: IntruderGrepExtractRule[],
  grepPayloadSettings: IntruderGrepPayloadSettings,
  visibleColumns?: string[],
): string {
  const allColumns = getIntruderResultColumns(grepMatchRules, grepExtractRules, grepPayloadSettings)
  const selectedColumns = visibleColumns?.length
    ? visibleColumns
      .map((key) => allColumns.find((column) => column.key === key))
      .filter((column): column is NonNullable<typeof column> => Boolean(column))
    : allColumns
  const header = selectedColumns.map((column) => column.label)

  const rows = results.map((result) =>
    selectedColumns.map((column) => {
      if (column.key === 'payloadSummary' && result.isBaseline) {
        return 'Baseline'
      }
      return getIntruderResultColumnValue(result, column.key) ?? ''
    }),
  )

  return [header, ...rows]
    .map((row) => row.map((value) => escapeCsv(String(value))).join(','))
    .join('\n')
}

export function normalizeVisibleColumns(
  visibleColumns: string[] | undefined,
  grepMatchRules: IntruderGrepMatchRule[],
  grepExtractRules: IntruderGrepExtractRule[],
  grepPayloadSettings: IntruderGrepPayloadSettings,
): string[] {
  const availableColumns = createDefaultVisibleColumns(grepMatchRules, grepExtractRules, grepPayloadSettings)
  if (!visibleColumns?.length) {
    return availableColumns
  }

  const normalized = visibleColumns.filter((key) => availableColumns.includes(key))
  return normalized.length ? normalized : availableColumns
}

function escapeCsv(value: string): string {
  if (!/[",\n]/.test(value)) return value
  return `"${value.replace(/"/g, '""')}"`
}
