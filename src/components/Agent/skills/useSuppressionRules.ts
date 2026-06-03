import { computed, ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import {
  getExpireSuppressionRuleConfirmMessage,
  getExtendSuppressionRuleConfirmMessage,
  getRemoveSuppressionRuleConfirmMessage,
  type TranslateFn,
} from '../skillsManagerHelpers'
import type { SkillCandidateSuppressionRule } from './skillsManagerTypes'

interface UseSuppressionRulesOptions {
  t: TranslateFn
  confirm: (message: string) => Promise<boolean>
}

export const useSuppressionRules = ({
  t,
  confirm,
}: UseSuppressionRulesOptions) => {
  const suppressionRules = ref<SkillCandidateSuppressionRule[]>([])
  const deletingSuppressionRuleIds = ref<string[]>([])
  const suppressionRuleActionById = ref<Record<string, 'delete' | 'extend' | 'expire'>>({})
  const showSuppressionRuleDetails = ref(false)
  const suppressionRuleDetailsTouched = ref(false)

  const suppressionRuleTotalHits = computed(() =>
    suppressionRules.value.reduce((total, rule) => total + (rule.hit_count ?? 0), 0)
  )

  const isSuppressionRuleBusy = (ruleId: string) =>
    deletingSuppressionRuleIds.value.includes(ruleId)

  const toggleSuppressionRuleDetails = () => {
    suppressionRuleDetailsTouched.value = true
    showSuppressionRuleDetails.value = !showSuppressionRuleDetails.value
  }

  const loadSuppressionRules = async () => {
    try {
      suppressionRules.value = await invoke<SkillCandidateSuppressionRule[]>(
        'list_skill_candidate_suppression_rules'
      )
      if (!suppressionRuleDetailsTouched.value) {
        showSuppressionRuleDetails.value = suppressionRules.value.length > 0
      }
    } catch (error) {
      console.error('Failed to load suppression rules:', error)
      suppressionRules.value = []
      if (!suppressionRuleDetailsTouched.value) {
        showSuppressionRuleDetails.value = false
      }
    }
  }

  const runSuppressionRuleAction = async (
    rule: SkillCandidateSuppressionRule,
    action: 'delete' | 'extend' | 'expire',
    task: () => Promise<unknown>
  ) => {
    if (isSuppressionRuleBusy(rule.id)) return
    deletingSuppressionRuleIds.value = [...deletingSuppressionRuleIds.value, rule.id]
    suppressionRuleActionById.value = {
      ...suppressionRuleActionById.value,
      [rule.id]: action,
    }
    try {
      await task()
      await loadSuppressionRules()
    } finally {
      deletingSuppressionRuleIds.value = deletingSuppressionRuleIds.value.filter(id => id !== rule.id)
      const nextActions = { ...suppressionRuleActionById.value }
      delete nextActions[rule.id]
      suppressionRuleActionById.value = nextActions
    }
  }

  const extendSuppressionRule = async (rule: SkillCandidateSuppressionRule, days: number) => {
    if (!(await confirm(getExtendSuppressionRuleConfirmMessage(t, rule.id, days)))) {
      return
    }

    try {
      await runSuppressionRuleAction(rule, 'extend', () =>
        invoke('extend_skill_candidate_suppression_rule', {
          ruleId: rule.id,
          days,
        })
      )
    } catch (error) {
      console.error('Failed to extend suppression rule:', error)
      alert(t('agent.extendSuppressionRuleFailed', { error: String(error) }))
    }
  }

  const expireSuppressionRule = async (rule: SkillCandidateSuppressionRule) => {
    if (!(await confirm(getExpireSuppressionRuleConfirmMessage(t, rule.id)))) {
      return
    }

    try {
      await runSuppressionRuleAction(rule, 'expire', () =>
        invoke('expire_skill_candidate_suppression_rule', {
          ruleId: rule.id,
        })
      )
    } catch (error) {
      console.error('Failed to expire suppression rule:', error)
      alert(t('agent.expireSuppressionRuleFailed', { error: String(error) }))
    }
  }

  const deleteSuppressionRule = async (rule: SkillCandidateSuppressionRule) => {
    if (!(await confirm(getRemoveSuppressionRuleConfirmMessage(t, rule.id)))) {
      return
    }

    try {
      await runSuppressionRuleAction(rule, 'delete', () =>
        invoke('delete_skill_candidate_suppression_rule', {
          ruleId: rule.id,
        })
      )
    } catch (error) {
      console.error('Failed to delete suppression rule:', error)
      alert(t('agent.deleteSuppressionRuleFailed', { error: String(error) }))
    }
  }

  return {
    suppressionRules,
    suppressionRuleActionById,
    showSuppressionRuleDetails,
    suppressionRuleTotalHits,
    toggleSuppressionRuleDetails,
    loadSuppressionRules,
    extendSuppressionRule,
    expireSuppressionRule,
    deleteSuppressionRule,
  }
}
