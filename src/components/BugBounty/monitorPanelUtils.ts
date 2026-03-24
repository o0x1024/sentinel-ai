export const formatInvokeError = (error: any, fallback: string) => {
  if (typeof error === 'string' && error.trim()) return error
  if (error?.message && typeof error.message === 'string') return error.message
  const text = error?.toString?.()
  if (text && text !== '[object Object]') return text
  return fallback
}

export const formatUptime = (secs: number) => {
  const hours = Math.floor(secs / 3600)
  const minutes = Math.floor((secs % 3600) / 60)

  if (hours > 0) {
    return `${hours}h ${minutes}m`
  }
  return `${minutes}m`
}
