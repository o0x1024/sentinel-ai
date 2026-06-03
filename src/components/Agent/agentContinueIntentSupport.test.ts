import { describe, expect, it } from 'vitest'

import { compactContinueIntentText, isContinueOnlyInput } from './agentContinueIntentSupport'

describe('agentContinueIntentSupport', () => {
  it('detects continue-only inputs', () => {
    expect(isContinueOnlyInput('继续')).toBe(true)
    expect(isContinueOnlyInput('继续下一步')).toBe(true)
    expect(isContinueOnlyInput('continue')).toBe(true)
    expect(isContinueOnlyInput('go on')).toBe(true)
    expect(isContinueOnlyInput('next step')).toBe(true)
  })

  it('does not classify continue with a new goal as continue-only', () => {
    expect(isContinueOnlyInput('继续修复 login timeout')).toBe(false)
    expect(isContinueOnlyInput('continue fixing login timeout')).toBe(false)
  })

  it('normalizes punctuation and spacing', () => {
    expect(compactContinueIntentText('  Continue, please! ')).toBe('continueplease')
    expect(compactContinueIntentText('继续，下一步！')).toBe('继续下一步')
  })
})
