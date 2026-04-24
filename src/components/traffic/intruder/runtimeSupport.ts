import type { IntruderAttackOptions } from './types'

export async function waitForIntruderDelay(ms: number) {
  if (ms <= 0) return
  await new Promise(resolve => window.setTimeout(resolve, ms))
}

export function getIntruderRuntimeDelayMs(
  options: IntruderAttackOptions,
  completedCount: number,
  throttlePenaltyMs: number,
): number {
  let delay = options.delayMs

  if (options.delayIncrementMs > 0) {
    delay += completedCount * options.delayIncrementMs
  } else if (options.randomDelayMs > 0) {
    delay += Math.floor(Math.random() * (options.randomDelayMs + 1))
  }

  return delay + throttlePenaltyMs
}

export function getIntruderAutoThrottleStepMs(options: IntruderAttackOptions): number {
  return Math.max(options.delayIncrementMs, options.delayMs > 0 ? Math.ceil(options.delayMs / 2) : 0, 200)
}

export function shouldIntruderThrottleForStatus(
  options: IntruderAttackOptions,
  statusCode: number | null,
): boolean {
  if (!options.autoThrottleEnabled || statusCode == null) return false
  return options.autoThrottleStatusCodes.includes(statusCode)
}
