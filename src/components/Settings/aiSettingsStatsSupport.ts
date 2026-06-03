export const totalUsageInputTokens = (stats: any) =>
  Object.values(stats).reduce((sum: number, usage: any) => sum + (usage.input_tokens || 0), 0) as number

export const totalUsageOutputTokens = (stats: any) =>
  Object.values(stats).reduce((sum: number, usage: any) => sum + (usage.output_tokens || 0), 0) as number

export const totalUsageCost = (stats: any) =>
  Object.values(stats).reduce((sum: number, usage: any) => sum + (usage.cost || 0), 0) as number

export const maxUsageTokens = (stats: any) =>
  Math.max(...Object.values(stats).map((usage: any) => usage.total_tokens || 0), 1)

export const formatUsageTotalTokens = (total: number) => {
  if (total >= 1_000_000) {
    return `${(total / 1_000_000).toFixed(2)}M`
  }
  if (total >= 1_000) {
    return `${(total / 1_000).toFixed(2)}K`
  }
  return total.toLocaleString()
}

export const formatUsageLastUsed = (
  timestamp: string | null,
  translate: (key: string, params?: Record<string, any>) => string,
) => {
  if (!timestamp) return '-'
  const date = new Date(timestamp)
  const now = new Date()
  const diff = now.getTime() - date.getTime()

  if (diff < 60000) return translate('settings.ai.justNow')
  if (diff < 3600000) return translate('settings.ai.minutesAgo', { n: Math.floor(diff / 60000) })
  if (diff < 86400000) return translate('settings.ai.hoursAgo', { n: Math.floor(diff / 3600000) })
  if (diff < 604800000) return translate('settings.ai.daysAgo', { n: Math.floor(diff / 86400000) })
  return date.toLocaleDateString()
}
