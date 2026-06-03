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
const INTRUDER_RESOURCE_POOL_AUTO_NAME_PREFIX = '资源池 '

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

export type IntruderResourcePoolConfigInput = Pick<
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
>

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

export function normalizeIntruderResourcePoolConfig(
  value: Partial<IntruderResourcePoolConfigInput>,
): IntruderResourcePoolConfigInput {
  const delayMs = Math.max(0, Number(value.delayMs) || 0)
  const randomDelayMs = Math.max(0, Number(value.randomDelayMs) || 0)
  const delayIncrementMs = Math.max(0, Number(value.delayIncrementMs) || 0)
  const autoThrottleEnabled = Boolean(value.autoThrottleEnabled)

  const delayEnabled = value.delayEnabled ?? (delayMs > 0 || randomDelayMs > 0 || delayIncrementMs > 0)
  const randomDelayEnabled = delayEnabled && (value.randomDelayEnabled ?? randomDelayMs > 0)
  const delayIncrementEnabled = delayEnabled && (value.delayIncrementEnabled ?? delayIncrementMs > 0)

  return {
    concurrencyEnabled: value.concurrencyEnabled ?? true,
    concurrency: Math.max(1, Number(value.concurrency) || 1),
    delayEnabled,
    delayMs: delayEnabled ? delayMs : 0,
    randomDelayEnabled,
    randomDelayMs: delayEnabled && randomDelayEnabled ? randomDelayMs : 0,
    delayIncrementEnabled,
    delayIncrementMs: delayEnabled && delayIncrementEnabled ? delayIncrementMs : 0,
    autoThrottleEnabled,
    autoThrottleStatusCodes: autoThrottleEnabled
      ? Array.isArray(value.autoThrottleStatusCodes)
        ? value.autoThrottleStatusCodes
          .map((item) => Number(item))
          .filter((item) => Number.isInteger(item) && item >= 100 && item <= 999)
          .sort((left, right) => left - right)
        : []
      : [],
  }
}

export function buildIntruderResourcePoolAutoName(
  resourcePools: IntruderResourcePool[],
  currentPoolId?: string,
): string {
  const currentPool = currentPoolId
    ? resourcePools.find((pool) => pool.id === currentPoolId && !pool.builtIn)
    : null
  if (currentPool && isIntruderResourcePoolAutoName(currentPool.name)) {
    return currentPool.name
  }

  const usedNumbers = new Set(
    resourcePools
      .map((pool) => parseIntruderResourcePoolAutoNameIndex(pool.name))
      .filter((value): value is number => value != null),
  )

  let nextIndex = 1
  while (usedNumbers.has(nextIndex)) {
    nextIndex += 1
  }

  return `${INTRUDER_RESOURCE_POOL_AUTO_NAME_PREFIX}${nextIndex}`
}

export function matchIntruderResourcePoolConfig(
  pool: IntruderResourcePool,
  value: Partial<IntruderResourcePoolConfigInput>,
): boolean {
  const normalizedPool = normalizeIntruderResourcePoolConfig(pool)
  const normalizedValue = normalizeIntruderResourcePoolConfig(value)

  return normalizedPool.concurrencyEnabled === normalizedValue.concurrencyEnabled
    && normalizedPool.concurrency === normalizedValue.concurrency
    && normalizedPool.delayEnabled === normalizedValue.delayEnabled
    && normalizedPool.delayMs === normalizedValue.delayMs
    && normalizedPool.randomDelayEnabled === normalizedValue.randomDelayEnabled
    && normalizedPool.randomDelayMs === normalizedValue.randomDelayMs
    && normalizedPool.delayIncrementEnabled === normalizedValue.delayIncrementEnabled
    && normalizedPool.delayIncrementMs === normalizedValue.delayIncrementMs
    && normalizedPool.autoThrottleEnabled === normalizedValue.autoThrottleEnabled
    && normalizedPool.autoThrottleStatusCodes.length === normalizedValue.autoThrottleStatusCodes.length
    && normalizedPool.autoThrottleStatusCodes.every((code, index) => code === normalizedValue.autoThrottleStatusCodes[index])
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
  options: IntruderResourcePoolConfigInput,
): IntruderResourcePool {
  const normalized = normalizeIntruderResourcePoolConfig(options)
  return {
    id: createIntruderId('resource-pool'),
    name,
    concurrencyEnabled: normalized.concurrencyEnabled,
    concurrency: normalized.concurrency,
    delayEnabled: normalized.delayEnabled,
    delayMs: normalized.delayMs,
    randomDelayEnabled: normalized.randomDelayEnabled,
    randomDelayMs: normalized.randomDelayMs,
    delayIncrementEnabled: normalized.delayIncrementEnabled,
    delayIncrementMs: normalized.delayIncrementMs,
    autoThrottleEnabled: normalized.autoThrottleEnabled,
    autoThrottleStatusCodes: normalized.autoThrottleStatusCodes,
    builtIn: false,
  }
}

export function upsertIntruderResourcePoolEntry(
  resourcePools: IntruderResourcePool[],
  value: Partial<IntruderResourcePool> & Partial<IntruderResourcePoolConfigInput>,
): {
  pools: IntruderResourcePool[]
  pool: IntruderResourcePool
} {
  const normalized = normalizeIntruderResourcePoolConfig(value)
  const requestedName = String(value.name || '').trim()
  const nextName = requestedName || buildIntruderResourcePoolAutoName(resourcePools, value.id)

  if (value.id) {
    const currentPool = resourcePools.find((pool) => pool.id === value.id)
    if (currentPool && !currentPool.builtIn) {
      const nextPool = {
        ...createIntruderResourcePool(nextName, normalized),
        id: currentPool.id,
      }
      return {
        pools: [
          ...resourcePools.filter((pool) => pool.id !== currentPool.id),
          nextPool,
        ],
        pool: nextPool,
      }
    }
  }

  const matchedPool = resourcePools.find((pool) => matchIntruderResourcePoolConfig(pool, normalized))
  if (matchedPool) {
    return {
      pools: resourcePools,
      pool: matchedPool,
    }
  }

  const nextPool = createIntruderResourcePool(nextName, normalized)
  return {
    pools: [...resourcePools, nextPool],
    pool: nextPool,
  }
}

function isIntruderResourcePoolAutoName(name: string): boolean {
  return parseIntruderResourcePoolAutoNameIndex(name) != null
}

function parseIntruderResourcePoolAutoNameIndex(name: string): number | null {
  const match = String(name).trim().match(/^资源池\s+(\d+)$/)
  if (!match) return null
  const value = Number.parseInt(match[1], 10)
  return Number.isInteger(value) && value > 0 ? value : null
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
