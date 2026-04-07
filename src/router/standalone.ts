const INTRUDER_RESULTS_STANDALONE = 'intruder-results'

export function buildIntruderResultsWindowUrl(workspaceId: string): string {
  const params = new URLSearchParams({
    standalone: INTRUDER_RESULTS_STANDALONE,
    workspaceId,
  })

  return `/?${params.toString()}`
}

export function resolveStandaloneBootstrapRoute(
  locationLike: Pick<Location, 'search' | 'hash'> = window.location,
): string | null {
  const searchParams = new URLSearchParams(locationLike.search)
  const standalone = searchParams.get('standalone')
  const workspaceId = searchParams.get('workspaceId')?.trim()

  if (standalone === INTRUDER_RESULTS_STANDALONE && workspaceId) {
    return `/intruder-results/${encodeURIComponent(workspaceId)}`
  }

  const hashPath = locationLike.hash.startsWith('#') ? locationLike.hash.slice(1) : locationLike.hash
  if (hashPath.startsWith('/intruder-results/')) {
    return hashPath
  }

  return null
}
