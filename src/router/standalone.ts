const HELP_CENTER_STANDALONE = 'help-center'

export const HELP_CENTER_WINDOW_LABEL = 'help-center'

export function buildHelpCenterWindowUrl(): string {
  const params = new URLSearchParams({
    standalone: HELP_CENTER_STANDALONE,
  })

  return `/?${params.toString()}`
}

export function resolveStandaloneBootstrapRoute(
  locationLike: Pick<Location, 'search' | 'hash'> = window.location
): string | null {
  const searchParams = new URLSearchParams(locationLike.search)
  const standalone = searchParams.get('standalone')

  if (standalone === HELP_CENTER_STANDALONE) {
    return '/help-center'
  }

  const hashPath = locationLike.hash.startsWith('#')
    ? locationLike.hash.slice(1)
    : locationLike.hash
  if (hashPath === '/help-center') {
    return hashPath
  }

  return null
}
