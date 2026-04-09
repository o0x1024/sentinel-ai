import type {
  WorkbenchAssessmentSuggestion,
  WorkbenchCase,
  WorkbenchExecutionAttempt,
  WorkbenchExecutionRun,
} from './securityWorkbenchTypes'

const flattenAttempts = (runs: WorkbenchExecutionRun[]) => runs.flatMap(run => run.attempts)

const buildChangedSignals = (attempts: WorkbenchExecutionAttempt[]) =>
  attempts
    .flatMap(attempt => attempt.diff.changedSignals || [])
    .filter((value, index, items) => Boolean(value) && items.indexOf(value) === index)
    .slice(0, 4)

export const buildWorkbenchAssessmentSuggestion = (
  caseItem: WorkbenchCase,
  executionRuns: WorkbenchExecutionRun[],
): WorkbenchAssessmentSuggestion | null => {
  if (executionRuns.length === 0) return null

  const attempts = flattenAttempts(executionRuns)
  if (attempts.length === 0) {
    return {
      title: '建议继续人工分析',
      summary: '当前只有阻断或未执行的草案，没有足够的自动执行结果来支持状态推进。',
      suggestedStatus: 'investigating',
      suggestedConclusion:
        '工作台已有执行草案，但当前执行结果不足以确认对象边界是否被突破，建议继续补充基线请求和候选对象，再做人工复核。',
      confidence: 'low',
      signals: ['当前没有可用于归因的自动执行尝试结果'],
    }
  }

  const changedAttempts = attempts.filter(item => item.outcome === 'changed')
  const blockedAttempts = attempts.filter(item => item.outcome === 'blocked')
  const sameAttempts = attempts.filter(item => item.outcome === 'same')
  const errorAttempts = attempts.filter(item => item.outcome === 'error')
  const notFoundAttempts = attempts.filter(item => item.outcome === 'not_found')

  if (changedAttempts.length > 0) {
    const signals = [
      `${changedAttempts.length} 次只读替换后返回了变化后的对象结果`,
      ...buildChangedSignals(changedAttempts),
    ]
    return {
      title: '建议推进到待验证',
      summary: '自动执行已经观察到对象边界变化，建议把案件状态推进到待验证，并补充人工确认。',
      suggestedStatus: 'awaiting_verification',
      suggestedConclusion: [
        `工作台只读执行共观察到 ${changedAttempts.length} 次对象变化响应，当前案件更像对象边界异常，需要人工进一步确认是否构成水平越权。`,
        changedAttempts[0]?.diff.changedSignals?.length
          ? `关键差异：${changedAttempts[0].diff.changedSignals.join('；')}`
          : '',
      ]
        .filter(Boolean)
        .join('\n'),
      confidence: changedAttempts.length >= 2 ? 'high' : 'medium',
      signals,
    }
  }

  if (blockedAttempts.length === attempts.length) {
    return {
      title: '建议维持调查中',
      summary: '自动执行全部被权限阻断，当前更像边界正常生效，但还不足以直接归为误报。',
      suggestedStatus: 'investigating',
      suggestedConclusion:
        '当前自动只读执行全部被权限拒绝，暂未观察到对象边界突破。建议继续结合更多对象池样本或其它入口做复核，不建议直接标记为误报。',
      confidence: 'medium',
      signals: [`${blockedAttempts.length} 次尝试全部被权限阻断`],
    }
  }

  if (sameAttempts.length > 0 && changedAttempts.length === 0) {
    return {
      title: '建议继续调查',
      summary: '当前自动执行未观察到明显对象变化，但存在相似响应，建议继续补充样本或更换验证入口。',
      suggestedStatus: 'investigating',
      suggestedConclusion:
        '工作台自动执行暂未观察到明确的对象边界突破，当前结果更接近相似响应或无显著差异。建议继续补充候选对象、切换入口或结合人工验证判断。',
      confidence: sameAttempts.length >= 2 ? 'medium' : 'low',
      signals: [
        `${sameAttempts.length} 次尝试返回相似响应`,
        ...(notFoundAttempts.length > 0 ? [`${notFoundAttempts.length} 次尝试返回资源不存在`] : []),
        ...(errorAttempts.length > 0 ? [`${errorAttempts.length} 次尝试执行异常`] : []),
      ],
    }
  }

  return {
    title: '建议保持调查中',
    summary: '当前自动执行结果偏弱，建议继续补更多执行样本后再推进案件状态。',
    suggestedStatus: caseItem.status === 'new' ? 'investigating' : caseItem.status,
    suggestedConclusion:
      '当前工作台自动执行结果不足以形成稳定结论，建议继续补充可替换对象、基线请求和相关证据，再决定是否推进状态或回写漏洞。',
    confidence: 'low',
    signals: [
      ...(notFoundAttempts.length > 0 ? [`${notFoundAttempts.length} 次尝试返回资源不存在`] : []),
      ...(errorAttempts.length > 0 ? [`${errorAttempts.length} 次尝试执行异常`] : []),
    ],
  }
}
