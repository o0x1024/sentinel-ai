export interface AgentSessionStats {
  duration_ms: number
  input_tokens: number
  output_tokens: number
  total_tokens: number
  tokens_per_second: number
}

const toPositiveNumber = (value: unknown): number | null => {
  const numberValue = Number(value)
  return Number.isFinite(numberValue) && numberValue > 0 ? numberValue : null
}

const toNonNegativeInteger = (value: unknown): number | null => {
  const numberValue = Number(value)
  return Number.isFinite(numberValue) && numberValue >= 0 ? Math.floor(numberValue) : null
}

export function buildAgentSessionStats(input: {
  startedAt: unknown
  endedAt: unknown
  inputTokens: unknown
  outputTokens: unknown
}): AgentSessionStats | null {
  const startedAt = toPositiveNumber(input.startedAt)
  const endedAt = toPositiveNumber(input.endedAt)
  const inputTokens = toNonNegativeInteger(input.inputTokens)
  const outputTokens = toNonNegativeInteger(input.outputTokens)

  if (startedAt == null || endedAt == null || endedAt <= startedAt) return null
  if (inputTokens == null || outputTokens == null) return null

  const durationMs = Math.max(1, Math.round(endedAt - startedAt))
  const durationSeconds = durationMs / 1000
  const totalTokens = inputTokens + outputTokens

  return {
    duration_ms: durationMs,
    input_tokens: inputTokens,
    output_tokens: outputTokens,
    total_tokens: totalTokens,
    tokens_per_second: outputTokens / durationSeconds,
  }
}

export function formatSessionDuration(durationMs: unknown): string {
  const ms = Number(durationMs)
  if (!Number.isFinite(ms) || ms <= 0) return ''
  if (ms < 1000) return `${Math.round(ms)}ms`
  if (ms < 60_000) return `${(ms / 1000).toFixed(1)}s`
  const minutes = Math.floor(ms / 60_000)
  const seconds = Math.round((ms % 60_000) / 1000)
  return `${minutes}m ${seconds}s`
}

export function formatTokenRate(rate: unknown): string {
  const numericRate = Number(rate)
  if (!Number.isFinite(numericRate) || numericRate < 0) return ''
  return numericRate >= 10 ? numericRate.toFixed(1) : numericRate.toFixed(2)
}
