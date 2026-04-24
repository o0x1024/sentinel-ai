import { estimateAttackCount } from './attack'
import { parsePayloadLines } from './payloads'
import { getIntruderBuiltInPayloadListItems } from './intruderBuiltInPayloadLists'
import type { IntruderAttackType, IntruderPayloadSet } from './types'

export type IntruderPayloadImportMode = 'append' | 'replace' | 'mergeDeduplicate'

export interface IntruderPayloadLibraryImportResult {
  nextItems: string[]
  nextPayloadsText: string
}

export interface IntruderPayloadTemplateApplicationSet {
  name: string
  sourceId: string
  payloadsText: string
}

export function applyIntruderPayloadImportMode(
  currentItems: string[],
  incomingItems: string[],
  mode: IntruderPayloadImportMode,
): IntruderPayloadLibraryImportResult {
  if (mode === 'replace') {
    return {
      nextItems: incomingItems,
      nextPayloadsText: incomingItems.join('\n'),
    }
  }

  if (mode === 'mergeDeduplicate') {
    const nextItems = Array.from(new Set([...currentItems, ...incomingItems]))
    return {
      nextItems,
      nextPayloadsText: nextItems.join('\n'),
    }
  }

  const nextItems = [...currentItems, ...incomingItems]
  return {
    nextItems,
    nextPayloadsText: nextItems.join('\n'),
  }
}

export function estimateIntruderRequestsAfterImport(options: {
  attackType: IntruderAttackType
  positionsLength: number
  payloadSets: IntruderPayloadSet[]
  activePayloadSetId: string
  nextPayloadsText: string
}): number {
  const nextPayloadSets = options.payloadSets.map((payloadSet) =>
    payloadSet.id === options.activePayloadSetId
      ? {
        ...payloadSet,
        payloadType: 'simpleList' as const,
        payloadsText: options.nextPayloadsText,
      }
      : payloadSet,
  )

  return estimateAttackCount(options.attackType, options.positionsLength, nextPayloadSets)
}

export function buildIntruderPayloadPreview(items: string[], limit = 20): string[] {
  return items.slice(0, limit)
}

export function buildIntruderPayloadTemplateApplicationSets(sets: Array<{ name: string; sourceId: string }>): IntruderPayloadTemplateApplicationSet[] {
  return sets.map((item) => ({
    name: item.name,
    sourceId: item.sourceId,
    payloadsText: getIntruderBuiltInPayloadListItems(item.sourceId).join('\n'),
  }))
}

export function getIntruderPayloadSetItems(payloadSet: IntruderPayloadSet): string[] {
  return parsePayloadLines(payloadSet.payloadsText)
}
