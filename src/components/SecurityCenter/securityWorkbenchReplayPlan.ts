import type { Evidence } from './vulnerabilityFindingTypes'
import type {
  WorkbenchCase,
  WorkbenchObjectAnalysis,
  WorkbenchReplayPlan,
  WorkbenchReplayPlanStep,
  WorkbenchReplayPlanStopCondition,
  WorkbenchSuggestionStrategy,
  WorkbenchTestSuggestion,
} from './securityWorkbenchTypes'
import { wb } from './securityWorkbenchLocale'

const findEvidence = (evidences: Evidence[], evidenceId: string) =>
  evidences.find(item => item.id === evidenceId) || null

const buildPlanSteps = (
  strategy: WorkbenchSuggestionStrategy,
  targetField: string,
  targetMethod: string,
  targetUrl: string,
  candidateValues: string[],
): WorkbenchReplayPlanStep[] => {
  const base: WorkbenchReplayPlanStep[] = [
    {
      id: 'baseline',
      title: wb('plan.step.baselineTitle'),
      detail: wb('plan.step.baselineDetail', { method: targetMethod, url: targetUrl }),
    },
    {
      id: 'field',
      title: wb('plan.step.fieldTitle'),
      detail: wb('plan.step.fieldDetail', { field: targetField }),
    },
    {
      id: 'candidate',
      title: wb('plan.step.candidateTitle'),
      detail: wb('plan.step.candidateDetail', { values: candidateValues.join(' / ') }),
    },
    {
      id: 'compare',
      title: wb('plan.step.compareTitle'),
      detail: wb('plan.step.compareDetail'),
    },
  ]

  if (strategy === 'export_check') {
    base.splice(2, 0, {
      id: 'readonly',
      title: wb('plan.step.readonlyTitle'),
      detail: wb('plan.step.readonlyDetail'),
    })
  } else if (strategy === 'owner_swap') {
    base.splice(2, 0, {
      id: 'owner',
      title: wb('plan.step.ownerTitle'),
      detail: wb('plan.step.ownerDetail'),
    })
  } else if (strategy === 'tenant_swap') {
    base.splice(2, 0, {
      id: 'tenant',
      title: wb('plan.step.tenantTitle'),
      detail: wb('plan.step.tenantDetail'),
    })
  }

  return base
}

const buildStopConditions = (strategy: WorkbenchSuggestionStrategy): WorkbenchReplayPlanStopCondition[] => {
  const common: WorkbenchReplayPlanStopCondition[] = [
    {
      id: '403',
      detail: wb('plan.stop.authBlocked'),
    },
    {
      id: 'mutation',
      detail: wb('plan.stop.mutationRisk'),
    },
  ]

  if (strategy === 'export_check') {
    common.push({
      id: 'file-download',
      detail: wb('plan.stop.fileDownload'),
    })
  }

  return common
}

const buildReplayPlan = (
  suggestion: WorkbenchTestSuggestion,
  evidence: Evidence,
): WorkbenchReplayPlan => ({
  id: `plan:${suggestion.id}`,
  title: suggestion.title,
  summary: suggestion.summary,
  strategy: suggestion.strategy,
  severity: suggestion.severity,
  readOnly: suggestion.strategy === 'export_check' || evidence.method === 'GET',
  targetEvidenceId: suggestion.targetEvidenceId,
  targetField: suggestion.targetField,
  targetMethod: evidence.method,
  targetUrl: evidence.url,
  candidateValues: suggestion.candidateValues,
  steps: buildPlanSteps(
    suggestion.strategy,
    suggestion.targetField,
    evidence.method,
    evidence.url,
    suggestion.candidateValues,
  ),
  stopConditions: buildStopConditions(suggestion.strategy),
  rationale: suggestion.why,
})

export const buildWorkbenchReplayPlans = (
  caseItem: WorkbenchCase,
  analysis: WorkbenchObjectAnalysis,
): WorkbenchReplayPlan[] => {
  const evidences = caseItem.finding.evidence || []
  return analysis.suggestions
    .map(suggestion => {
      const evidence = findEvidence(evidences, suggestion.targetEvidenceId)
      if (!evidence) return null
      return buildReplayPlan(suggestion, evidence)
    })
    .filter((item): item is WorkbenchReplayPlan => Boolean(item))
}

export const formatWorkbenchReplayPlan = (plan: WorkbenchReplayPlan) => {
  const lines = [
    wb('plan.formatted.title', { title: plan.title }),
    wb('plan.formatted.strategy', { strategy: plan.strategy }),
    wb('plan.formatted.severity', { severity: wb(`priority.${plan.severity}`) }),
    wb('plan.formatted.readonly', { value: plan.readOnly ? wb('common.yes') : wb('common.no') }),
    wb('plan.formatted.targetRequest', { value: `${plan.targetMethod} ${plan.targetUrl}` }),
    wb('plan.formatted.targetField', { value: plan.targetField }),
    wb('plan.formatted.candidateValues', { value: plan.candidateValues.join(', ') }),
    '',
    wb('plan.formatted.steps'),
    ...plan.steps.map((step, index) => `${index + 1}. ${step.title} - ${step.detail}`),
    '',
    wb('plan.formatted.stopConditions'),
    ...plan.stopConditions.map((item, index) => `${index + 1}. ${item.detail}`),
    '',
    wb('plan.formatted.rationale', { value: plan.rationale }),
  ]

  return lines.join('\n')
}
