export type TranslateFn = (key: string, params?: Record<string, unknown>) => string

export type SkillManagerViewMode = 'card' | 'list'

export type SuppressionRuleCategory =
  | 'duplicate_noise'
  | 'stale_preference'
  | 'overfit_procedure'
  | 'incorrect_pattern'

interface CandidateContext {
  id: string
  title: string
  memory_kind: string
  scope: string
  source: string
  confidence: number
  description: string
  content: string
}

interface BuildSkillPromptInput {
  brief: string
  hasExistingContent: boolean
  existingName: string
  existingDescription: string
  existingArgumentHint: string
  existingContent: string
  existingModel: string
  existingContext: string
  existingAgent: string
  existingHooks: string
  candidateContext?: CandidateContext | null
  locale: string
}

export const createRejectionCategoryOptions = (t: TranslateFn) => [
  {
    category: 'duplicate_noise' as const,
    label: t('agent.rejectionCategoryDuplicateNoise'),
    buttonClass: 'btn-neutral',
    icon: 'fas fa-volume-xmark'
  },
  {
    category: 'stale_preference' as const,
    label: t('agent.rejectionCategoryStalePreference'),
    buttonClass: 'btn-info',
    icon: 'fas fa-sliders'
  },
  {
    category: 'overfit_procedure' as const,
    label: t('agent.rejectionCategoryOverfitProcedure'),
    buttonClass: 'btn-warning',
    icon: 'fas fa-route'
  },
  {
    category: 'incorrect_pattern' as const,
    label: t('agent.rejectionCategoryIncorrectPattern'),
    buttonClass: 'btn-error',
    icon: 'fas fa-triangle-exclamation'
  }
]

export const getCurrentViewModeLabel = (
  t: TranslateFn,
  viewMode: SkillManagerViewMode
) => (viewMode === 'list' ? t('agent.skillListView') : t('agent.skillCardView'))

export const getCandidateArgumentHint = (t: TranslateFn, candidateId: string) =>
  t('agent.skillDerivedFromCandidate', { id: candidateId })

export const getRefineCandidateBrief = (t: TranslateFn, memoryKind: string) =>
  t('agent.refineCandidateBrief', { memoryKind })

export const getPromoteCandidateConfirmMessage = (t: TranslateFn, title: string) =>
  t('agent.promoteCandidateConfirm', { title })

export const getReviewCandidateConfirmMessage = (
  t: TranslateFn,
  title: string,
  actionLabel: string,
  actionSuffix = ''
) => t('agent.reviewCandidateConfirm', { title, actionLabel, actionSuffix })

export const getExtendSuppressionRuleConfirmMessage = (
  t: TranslateFn,
  ruleId: string,
  days: number
) => t('agent.extendSuppressionRuleConfirm', { ruleId, days })

export const getExpireSuppressionRuleConfirmMessage = (t: TranslateFn, ruleId: string) =>
  t('agent.expireSuppressionRuleConfirm', { ruleId })

export const getRemoveSuppressionRuleConfirmMessage = (t: TranslateFn, ruleId: string) =>
  t('agent.removeSuppressionRuleConfirm', { ruleId })

export const getRejectionReviewNote = (
  t: TranslateFn,
  category: SuppressionRuleCategory
) => {
  switch (category) {
    case 'duplicate_noise':
      return t('agent.rejectionReviewNoteDuplicateNoise')
    case 'stale_preference':
      return t('agent.rejectionReviewNoteStalePreference')
    case 'overfit_procedure':
      return t('agent.rejectionReviewNoteOverfitProcedure')
    case 'incorrect_pattern':
      return t('agent.rejectionReviewNoteIncorrectPattern')
    default:
      return t('agent.rejectionReviewNoteDefault')
  }
}

export const buildSkillTemplateContent = (t: TranslateFn) => t('agent.skillTemplateContent')

export const formatCandidateDateTime = (
  timestamp: number | null | undefined,
  locale: string
) => {
  if (!timestamp) return '-'

  const normalizedLocale = locale.toLowerCase().startsWith('zh') ? 'zh-CN' : 'en-US'
  return new Date(timestamp).toLocaleString(normalizedLocale, { hour12: false })
}

export const formatSuppressionRuleCategory = (t: TranslateFn, category: string) => {
  switch (category) {
    case 'duplicate_noise':
      return t('agent.rejectionCategoryDuplicateNoise')
    case 'stale_preference':
      return t('agent.rejectionCategoryStalePreference')
    case 'overfit_procedure':
      return t('agent.rejectionCategoryOverfitProcedure')
    case 'incorrect_pattern':
      return t('agent.rejectionCategoryIncorrectPattern')
    default:
      return category || t('agent.uncategorized')
  }
}

export const candidateStatusLabel = (t: TranslateFn, status: string) => {
  switch (status) {
    case 'draft':
      return t('agent.candidateStatusDraft')
    case 'promoted':
      return t('agent.candidateStatusPromoted')
    case 'rejected':
      return t('agent.candidateStatusRejected')
    case 'archived':
      return t('agent.candidateStatusArchived')
    default:
      return status
  }
}

export const candidateStatusBadgeClass = (status: string) => {
  switch (status) {
    case 'promoted':
      return 'badge-success badge-outline'
    case 'rejected':
      return 'badge-error badge-outline'
    case 'archived':
      return 'badge-neutral badge-outline'
    default:
      return 'badge-warning badge-outline'
  }
}

export const candidateReviewDotClass = (status: string) => {
  switch (status) {
    case 'promoted':
      return 'bg-success'
    case 'rejected':
      return 'bg-error'
    case 'archived':
      return 'bg-neutral'
    default:
      return 'bg-warning'
  }
}

export const suppressionRuleCategoryClass = (category: string) => {
  switch (category) {
    case 'duplicate_noise':
      return 'badge-neutral'
    case 'stale_preference':
      return 'badge-info'
    case 'overfit_procedure':
      return 'badge-warning'
    case 'incorrect_pattern':
      return 'badge-error'
    default:
      return 'badge-ghost'
  }
}

export const formatFileSize = (size: number) => {
  if (size < 1024) return `${size}B`
  if (size < 1024 * 1024) return `${(size / 1024).toFixed(1)}KB`
  return `${(size / (1024 * 1024)).toFixed(1)}MB`
}

export const hashSkillId = (value: string) => {
  let hash = 0
  for (let i = 0; i < value.length; i += 1) {
    hash = (hash * 31 + value.charCodeAt(i)) >>> 0
  }
  return hash
}

export const buildSkillPrompt = ({
  brief,
  hasExistingContent,
  existingName,
  existingDescription,
  existingArgumentHint,
  existingContent,
  existingModel,
  existingContext,
  existingAgent,
  existingHooks,
  candidateContext,
  locale
}: BuildSkillPromptInput) => {
  const emptyValue = locale.toLowerCase().startsWith('zh') ? '(空)' : '(empty)'
  const safeName = existingName || emptyValue
  const safeDescription = existingDescription || emptyValue
  const safeArgumentHint = existingArgumentHint || emptyValue
  const safeContent = existingContent || emptyValue
  const safeModel = existingModel || emptyValue
  const safeContext = existingContext || emptyValue
  const safeAgent = existingAgent || emptyValue
  const safeHooks = existingHooks || emptyValue

  if (brief) {
    return {
      prompt: `Generate a skill configuration based on the brief description below.

Brief:
${brief}

Existing Fields (if any, treat as constraints and keep if valid):
- Name: ${safeName}
- Description: ${safeDescription}
- Argument Hint: ${safeArgumentHint}
- Content: ${safeContent}
- Model: ${safeModel}
- Context: ${safeContext}
- Agent: ${safeAgent}
- Hooks (JSON): ${safeHooks}
${candidateContext ? `

Candidate Context:
- Candidate ID: ${candidateContext.id}
- Candidate Title: ${candidateContext.title}
- Memory Kind: ${candidateContext.memory_kind}
- Memory Scope: ${candidateContext.scope}
- Memory Source: ${candidateContext.source}
- Confidence: ${candidateContext.confidence.toFixed(2)}
- Candidate Description: ${candidateContext.description}
- Candidate Draft Content:
${candidateContext.content}
` : ''}

Return ONLY a JSON object with this structure:
{
  "name": "skill-name-in-kebab-case",
  "description": "Third-person description: what it does and when to use it",
  "argument_hint": "Short argument hint if needed",
  "content": "Skill content in markdown",
  "disable_model_invocation": false,
  "user_invocable": true,
  "model": "",
  "context": "",
  "agent": "",
  "hooks": {}
}
Ensure the name is lowercase kebab-case, <= 64 chars, and avoid reserved words.`,
      systemPrompt:
        'You are a helpful assistant that generates configuration for AI agent skills. Return valid JSON only.'
    }
  }

  if (hasExistingContent) {
    return {
      prompt: `Based on the existing content below, please enhance and expand the skill configuration.

Existing Content:
- Name: ${safeName}
- Description: ${safeDescription}
- Argument Hint: ${safeArgumentHint}
- Content: ${safeContent}
- Model: ${safeModel}
- Context: ${safeContext}
- Agent: ${safeAgent}
- Hooks (JSON): ${safeHooks}

Please:
1. Keep the existing content that is good
2. Enhance empty or incomplete fields
3. Expand the skill content with more detail and clearer workflow
4. Keep hooks as valid JSON (return an object, or {} if none)

Return ONLY a JSON object with the following structure:
{
  "name": "Enhanced Skill Name",
  "description": "Enhanced short description (one sentence)",
  "argument_hint": "Short argument hint if needed",
  "content": "Enhanced skill content in markdown",
  "disable_model_invocation": false,
  "user_invocable": true,
  "model": "",
  "context": "",
  "agent": "",
  "hooks": {}
}
Ensure the content is in the same language as the existing content (or Chinese if unsure).
`,
      systemPrompt:
        'You are a helpful assistant that enhances and expands configuration for AI agent skills. Keep good existing content, enhance incomplete parts, and add missing details. Return valid JSON only.'
    }
  }

  return {
    prompt: `Please generate a skill configuration.

Return ONLY a JSON object with the following structure:
{
  "name": "Skill Name",
  "description": "Short description (one sentence)",
  "argument_hint": "Short argument hint if needed",
  "content": "Skill content in markdown",
  "disable_model_invocation": false,
  "user_invocable": true,
  "model": "",
  "context": "",
  "agent": "",
  "hooks": {}
}
Ensure the content is in Chinese unless the existing content clearly uses another language.
`,
    systemPrompt:
      'You are a helpful assistant that generates configuration for AI agent skills. Return valid JSON only.'
  }
}
