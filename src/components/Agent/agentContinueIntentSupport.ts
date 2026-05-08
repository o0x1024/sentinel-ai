const CONTINUE_ONLY_INPUTS = new Set([
  'continue',
  'continueplease',
  'goon',
  'proceed',
  'next',
  'nextstep',
  '继续',
  '继续吧',
  '继续执行',
  '继续任务',
  '继续当前任务',
  '继续下一步',
  '下一步',
  '接着',
  '接着做',
  '接着执行',
])

export const compactContinueIntentText = (input: string): string =>
  input
    .trim()
    .toLowerCase()
    .replace(/[\s.,!?;:"'。，！？；：]/g, '')

export const isContinueOnlyInput = (input: string): boolean =>
  CONTINUE_ONLY_INPUTS.has(compactContinueIntentText(input))

export const buildActiveContinueBlockedMessage = (executionId?: string | null): string => {
  const suffix = executionId?.trim() ? ` (${executionId.trim()})` : ''
  return `当前任务仍在执行${suffix}，输入“继续”不会启动新任务。请等待当前执行完成，或先停止当前执行。`
}
