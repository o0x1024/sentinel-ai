import type { IntruderAttackType } from './types'

export interface IntruderBuiltInPayloadListDefinition {
  id: string
  categoryKey: string
  labelKey: string
  descriptionKey: string
  aliases: string[]
  tags: string[]
  recommendedAttackTypes: IntruderAttackType[]
  recommendedPositionHints: string[]
  items: string[]
}

export interface IntruderPayloadTemplateSetDefinition {
  nameKey: string
  sourceId: string
}

export interface IntruderPayloadTemplateDefinition {
  id: string
  labelKey: string
  descriptionKey: string
  tags: string[]
  recommendedAttackType: IntruderAttackType
  recommendedPositionHints: string[]
  sets: IntruderPayloadTemplateSetDefinition[]
}

export interface IntruderPayloadCharsetDistribution {
  numericOnly: number
  alphabeticOnly: number
  alphaNumeric: number
  withUnicode: number
  withSymbols: number
  mixed: number
}

export interface IntruderPayloadQualityStats {
  total: number
  unique: number
  duplicates: number
  averageLength: number
  weakRatio: number
  dirtyCount: number
  charsetDistribution: IntruderPayloadCharsetDistribution
}

export interface IntruderBuiltInPayloadRecommendationContext {
  attackType: IntruderAttackType
  requestText: string
  positionValues: string[]
}

export function buildRange(start: number, end: number): string[] {
  return Array.from({ length: end - start + 1 }, (_, index) => String.fromCharCode(start + index))
}

export function deduplicatePayloadItems(items: string[]): string[] {
  return Array.from(new Set(items.map((item) => item.trim()).filter(Boolean)))
}
