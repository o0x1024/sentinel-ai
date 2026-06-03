import { computed, ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import {
  getPromoteCandidateConfirmMessage,
  getRejectionReviewNote,
  getReviewCandidateConfirmMessage,
  formatSuppressionRuleCategory,
  type TranslateFn,
} from '../skillsManagerHelpers'
import type {
  SkillCandidate,
  SuppressionRuleCategory,
} from './skillsManagerTypes'

interface UseSkillCandidatesOptions {
  t: TranslateFn
  confirm: (message: string) => Promise<boolean>
  loadSkills: () => Promise<void>
  loadSuppressionRules: () => Promise<void>
  onChanged: () => void
}

export const useSkillCandidates = ({
  t,
  confirm,
  loadSkills,
  loadSuppressionRules,
  onChanged,
}: UseSkillCandidatesOptions) => {
  const skillCandidates = ref<SkillCandidate[]>([])
  const loadingCandidates = ref(false)
  const showReviewedCandidates = ref(false)
  const promotingCandidateIds = ref<string[]>([])
  const reviewingCandidateIds = ref<string[]>([])
  const showCandidateDetails = ref(false)
  const candidateDetailsTouched = ref(false)

  const activeSkillCandidates = computed(() =>
    skillCandidates.value.filter(candidate => candidate.status === 'draft')
  )

  const reviewedSkillCandidateCount = computed(
    () => skillCandidates.value.length - activeSkillCandidates.value.length
  )

  const visibleSkillCandidates = computed(() => {
    if (showReviewedCandidates.value) {
      return skillCandidates.value
    }
    return skillCandidates.value.filter(candidate => candidate.status === 'draft')
  })

  const toggleCandidateDetails = () => {
    candidateDetailsTouched.value = true
    showCandidateDetails.value = !showCandidateDetails.value
  }

  const loadSkillCandidates = async () => {
    loadingCandidates.value = true
    try {
      skillCandidates.value = await invoke<SkillCandidate[]>('list_skill_candidates')
      if (!candidateDetailsTouched.value) {
        showCandidateDetails.value = skillCandidates.value.length > 0
      }
    } catch (error) {
      console.error('Failed to load skill candidates:', error)
      skillCandidates.value = []
      if (!candidateDetailsTouched.value) {
        showCandidateDetails.value = false
      }
    } finally {
      loadingCandidates.value = false
    }
  }

  const promoteCandidate = async (candidate: SkillCandidate) => {
    if (candidate.status !== 'draft') return
    if (promotingCandidateIds.value.includes(candidate.id)) return
    if (!(await confirm(getPromoteCandidateConfirmMessage(t, candidate.title)))) {
      return
    }
    promotingCandidateIds.value = [...promotingCandidateIds.value, candidate.id]
    try {
      await invoke('promote_skill_candidate', {
        payload: {
          candidateId: candidate.id,
        },
      })
      await Promise.all([loadSkills(), loadSkillCandidates()])
      onChanged()
    } catch (error) {
      console.error('Failed to promote skill candidate:', error)
      alert(t('agent.promoteCandidateFailed', { error: String(error) }))
    } finally {
      promotingCandidateIds.value = promotingCandidateIds.value.filter(id => id !== candidate.id)
    }
  }

  const reviewCandidate = async (
    candidate: SkillCandidate,
    status: 'rejected' | 'archived',
    suppressionCategory?: SuppressionRuleCategory
  ) => {
    if (candidate.status !== 'draft') return
    if (status === 'rejected' && !suppressionCategory) return
    if (reviewingCandidateIds.value.includes(candidate.id)) return
    const actionLabel =
      status === 'rejected'
        ? t('agent.reviewCandidateActionReject')
        : t('agent.reviewCandidateActionArchive')
    const actionSuffix =
      status === 'rejected' && suppressionCategory
        ? t('agent.reviewCandidateActionSuffix', {
            category: formatSuppressionRuleCategory(t, suppressionCategory),
          })
        : ''
    if (!(await confirm(
      getReviewCandidateConfirmMessage(t, candidate.title, actionLabel, actionSuffix)
    ))) {
      return
    }
    reviewingCandidateIds.value = [...reviewingCandidateIds.value, candidate.id]
    try {
      await invoke('review_skill_candidate', {
        payload: {
          candidateId: candidate.id,
          status,
          suppressionCategory: status === 'rejected' ? suppressionCategory : undefined,
          reviewNote:
            status === 'rejected' && suppressionCategory
              ? getRejectionReviewNote(t, suppressionCategory)
              : t('agent.reviewArchivedFromGate'),
        },
      })
      await Promise.all([loadSkillCandidates(), loadSuppressionRules()])
    } catch (error) {
      console.error('Failed to review skill candidate:', error)
      alert(t('agent.reviewCandidateFailed', { error: String(error) }))
    } finally {
      reviewingCandidateIds.value = reviewingCandidateIds.value.filter(id => id !== candidate.id)
    }
  }

  return {
    skillCandidates,
    loadingCandidates,
    showReviewedCandidates,
    promotingCandidateIds,
    reviewingCandidateIds,
    showCandidateDetails,
    activeSkillCandidates,
    reviewedSkillCandidateCount,
    visibleSkillCandidates,
    toggleCandidateDetails,
    loadSkillCandidates,
    promoteCandidate,
    reviewCandidate,
  }
}
