import { createRawResponseFromReplayResponse } from './http/response'
import type { HttpReplayResponse } from './http/model'
import type { RepeaterTab } from './proxyRepeaterTypes'
import type { ReplayRun } from './workbench/model/replayRun'

function normalizeReplayResponse(response: HttpReplayResponse): HttpReplayResponse {
  const rawText = response.rawText || createRawResponseFromReplayResponse(response)
  return {
    ...response,
    headers: [...response.headers],
    rawText,
  }
}

export function findLatestCompletedReplayResponseForDraft(
  replayRuns: ReplayRun[],
  draftId: string,
): HttpReplayResponse | null {
  const latestRun = replayRuns
    .filter(run => run.draftId === draftId && run.state === 'done' && run.response)
    .sort((left, right) => right.updatedAt - left.updatedAt)[0]

  return latestRun?.response ? normalizeReplayResponse(latestRun.response) : null
}

export function restoreRepeaterTabResponseFromReplayRuns(
  tab: RepeaterTab,
  replayRuns: ReplayRun[],
): void {
  if (!tab.draftId) {
    return
  }

  const response = findLatestCompletedReplayResponseForDraft(replayRuns, tab.draftId)
  if (!response) {
    return
  }

  tab.response = response
  tab.rawResponse = response.rawText
  tab.lastCompletedRawResponse = response.rawText
  tab.previousRawResponse = ''
}
