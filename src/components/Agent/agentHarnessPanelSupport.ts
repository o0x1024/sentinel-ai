export interface HarnessRunListItem {
  id: string
  generation: number
  state: string
}

export const isActiveHarnessRun = (run: HarnessRunListItem) => {
  return String(run.state || '').toLowerCase() === 'running'
}

export const collapsedHarnessRuns = <T extends HarnessRunListItem>(runs: T[]) => {
  const visibleRunIds = new Set<string>()
  for (const run of runs) {
    if (isActiveHarnessRun(run)) {
      visibleRunIds.add(run.id)
    }
  }

  const latestHistoricalRun = runs.find((run) => !isActiveHarnessRun(run))
  if (latestHistoricalRun) {
    visibleRunIds.add(latestHistoricalRun.id)
  } else if (runs[0]) {
    visibleRunIds.add(runs[0].id)
  }

  return runs.filter((run) => visibleRunIds.has(run.id))
}

export const harnessRunsForDisplay = <T extends HarnessRunListItem>(runs: T[], showAllRuns: boolean) => {
  return showAllRuns ? runs : collapsedHarnessRuns(runs)
}

export const hiddenHarnessRunCount = <T extends HarnessRunListItem>(runs: T[], showAllRuns: boolean) => {
  if (showAllRuns) return 0
  return Math.max(0, runs.length - collapsedHarnessRuns(runs).length)
}

export const resolveHarnessSelectedRunId = <T extends HarnessRunListItem>(
  runs: T[],
  selectedRunId: string | null,
  showAllRuns: boolean,
  preserveSelectedRun = true,
) => {
  if (runs.length === 0) return null
  const displayRuns = harnessRunsForDisplay(runs, showAllRuns)
  if (preserveSelectedRun && selectedRunId && displayRuns.some((run) => run.id === selectedRunId)) {
    return selectedRunId
  }
  return displayRuns[0]?.id || null
}

export const timelineItemsForDisplay = <T>(items: T[], showAllItems: boolean, visibleCount = 3) => {
  const sourceItems = showAllItems ? items : items.slice(-Math.max(0, visibleCount))
  return [...sourceItems].reverse()
}

export const hiddenTimelineItemCount = <T>(items: T[], showAllItems: boolean, visibleCount = 3) => {
  if (showAllItems) return 0
  return Math.max(0, items.length - timelineItemsForDisplay(items, false, visibleCount).length)
}
