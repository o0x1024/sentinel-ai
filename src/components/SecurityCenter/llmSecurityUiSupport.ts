export const formatDateTime = (iso: string | undefined): string => {
  if (!iso) return '-'
  try {
    return new Date(iso).toLocaleString(undefined, {
      month: '2-digit',
      day: '2-digit',
      hour: '2-digit',
      minute: '2-digit',
      second: '2-digit',
    })
  } catch {
    return iso
  }
}

export const formatDuration = (startIso: string | undefined, endIso: string | undefined): string => {
  if (!startIso || !endIso) return '-'
  try {
    const ms = new Date(endIso).getTime() - new Date(startIso).getTime()
    if (ms < 0) return '-'
    if (ms < 1000) return `${ms}ms`
    const seconds = Math.floor(ms / 1000)
    if (seconds < 60) return `${seconds}s`
    const minutes = Math.floor(seconds / 60)
    return `${minutes}m ${seconds % 60}s`
  } catch {
    return '-'
  }
}

export const riskBadgeClass = (risk: string) => {
  const normalized = (risk || '').toLowerCase()
  if (normalized === 'critical') return 'badge-error'
  if (normalized === 'high') return 'badge-warning'
  if (normalized === 'medium') return 'badge-info'
  if (normalized === 'low') return 'badge-success'
  return 'badge-ghost'
}

export const statusBadgeClass = (status: string) => {
  const normalized = (status || '').toLowerCase()
  if (normalized === 'completed') return 'badge-success'
  if (normalized === 'running') return 'badge-info'
  if (normalized === 'failed' || normalized === 'cancelled') return 'badge-error'
  return 'badge-ghost'
}
