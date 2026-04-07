import type { TrafficComparePayload } from './transfers'
import { inferComparerSideMeta } from './trafficComparerFormattingSupport'

export interface ComparerDraft {
  name: string
  leftLabel: string
  rightLabel: string
  leftText: string
  rightText: string
}

type ComparerDraftLabels = {
  defaultName: string
  leftLabel: string
  rightLabel: string
}

const normalizeText = (text: string): string => text.replace(/\r\n/g, '\n').replace(/\r/g, '\n')

export function createEmptyComparerDraft(labels: ComparerDraftLabels): ComparerDraft {
  return {
    name: labels.defaultName,
    leftLabel: labels.leftLabel,
    rightLabel: labels.rightLabel,
    leftText: '',
    rightText: '',
  }
}

export function hasComparerDraftContent(draft: ComparerDraft): boolean {
  return draft.leftText.trim().length > 0 || draft.rightText.trim().length > 0
}

export function canBuildComparerDraftPayload(draft: ComparerDraft): boolean {
  return draft.leftText.trim().length > 0 && draft.rightText.trim().length > 0
}

export function buildComparerDraftPayload(
  draft: ComparerDraft,
  fallbackName: string,
): TrafficComparePayload {
  const leftText = normalizeText(draft.leftText)
  const rightText = normalizeText(draft.rightText)

  return {
    name: draft.name.trim() || fallbackName,
    leftLabel: draft.leftLabel.trim() || 'Left',
    rightLabel: draft.rightLabel.trim() || 'Right',
    leftText,
    rightText,
    compareMeta: {
      source: 'generic',
      kind: 'generic',
    },
    leftMeta: inferComparerSideMeta(leftText),
    rightMeta: inferComparerSideMeta(rightText),
  }
}
