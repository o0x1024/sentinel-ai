import { createIntruderId } from './http'
import { createDefaultVisibleColumns, getIntruderResultColumns, getIntruderResultColumnValue } from './analysis'
import type {
  IntruderAttackOptions,
  IntruderAttackResult,
  IntruderAttackType,
  IntruderGrepExtractRule,
  IntruderGrepMatchRule,
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
  grepMatchRules: IntruderGrepMatchRule[]
  grepExtractRules: IntruderGrepExtractRule[]
  selectedResourcePoolId: string
  attackOptions: IntruderAttackOptions
  captureFilter: IntruderResultFilter
  viewFilter: IntruderResultFilter
  sort: IntruderResultSort
  visibleColumns: string[]
}

export function createBuiltInResourcePools(): IntruderResourcePool[] {
  return [
    { id: 'default', name: 'Default resource pool', concurrency: 10, delayMs: 0, randomDelayMs: 0, builtIn: true },
    { id: 'gentle', name: 'Gentle', concurrency: 3, delayMs: 250, randomDelayMs: 50, builtIn: true },
    { id: 'slow', name: 'Slow and safe', concurrency: 1, delayMs: 1000, randomDelayMs: 150, builtIn: true },
  ]
}

export function loadIntruderResourcePools(defaultPools: IntruderResourcePool[]): IntruderResourcePool[] {
  const raw = localStorage.getItem(RESOURCE_POOLS_STORAGE_KEY)
  if (!raw) return defaultPools

  try {
    const customPools = JSON.parse(raw) as IntruderResourcePool[]
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

export function createIntruderResourcePool(name: string, options: Pick<IntruderAttackOptions, 'concurrency' | 'delayMs' | 'randomDelayMs'>): IntruderResourcePool {
  return {
    id: createIntruderId('resource-pool'),
    name,
    concurrency: options.concurrency,
    delayMs: options.delayMs,
    randomDelayMs: options.randomDelayMs,
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
  visibleColumns?: string[],
): string {
  const allColumns = getIntruderResultColumns(grepMatchRules, grepExtractRules)
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
): string[] {
  const availableColumns = createDefaultVisibleColumns(grepMatchRules, grepExtractRules)
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
