<template>
  <div class="card bg-base-200 border border-base-300 p-4">
    <div class="flex items-center justify-between gap-3">
      <div>
        <div class="text-sm font-semibold">{{ t('agent.suppressionRuleDetailsTitle') }}</div>
        <div class="text-xs text-base-content/60">
          {{ t('agent.suppressionRuleDetailsDescription') }}
        </div>
      </div>
      <div class="flex items-center gap-2">
        <span class="badge badge-sm badge-neutral badge-outline">{{ suppressionRules.length }}</span>
        <button class="btn btn-xs btn-ghost" @click="$emit('refresh')">
          <i class="fas fa-rotate-right mr-1"></i>
          {{ t('common.refresh') }}
        </button>
      </div>
    </div>
    <div class="mt-4 space-y-3">
      <div
        v-for="rule in suppressionRules"
        :key="rule.id"
        class="rounded-xl border border-base-300 bg-base-100 p-3"
      >
        <div class="flex items-start justify-between gap-3">
          <div class="min-w-0 flex-1">
            <div class="flex flex-wrap items-center gap-2">
              <div class="text-sm font-medium">{{ rule.memory_kind }} via {{ rule.source }}</div>
              <span class="badge badge-sm" :class="suppressionRuleCategoryClass(rule.category)">
                {{ formatSuppressionRuleCategory(t, rule.category) }}
              </span>
            </div>
            <div class="mt-1 truncate font-mono text-[10px] text-base-content/50">{{ rule.id }}</div>
          </div>
          <div class="flex flex-wrap justify-end gap-2">
            <button
              class="btn btn-xs btn-outline"
              :disabled="isBusy(rule.id)"
              @click="$emit('extend', rule, 7)"
            >
              <i
                :class="actionById[rule.id] === 'extend'
                  ? 'fas fa-spinner fa-spin mr-1'
                  : 'fas fa-clock-rotate-left mr-1'"
              ></i>
              {{ t('agent.extendSuppressionRule', { days: 7 }) }}
            </button>
            <button
              class="btn btn-xs btn-warning btn-outline"
              :disabled="isBusy(rule.id)"
              @click="$emit('expire', rule)"
            >
              <i
                :class="actionById[rule.id] === 'expire'
                  ? 'fas fa-spinner fa-spin mr-1'
                  : 'fas fa-hourglass-end mr-1'"
              ></i>
              {{ t('agent.expireSuppressionRuleNow') }}
            </button>
            <button
              class="btn btn-xs btn-error btn-outline"
              :disabled="isBusy(rule.id)"
              @click="$emit('delete', rule)"
            >
              <i
                :class="actionById[rule.id] === 'delete'
                  ? 'fas fa-spinner fa-spin mr-1'
                  : 'fas fa-rotate-left mr-1'"
              ></i>
              {{ t('common.delete') }}
            </button>
          </div>
        </div>
        <div class="mt-3 rounded-lg bg-base-200/60 px-3 py-2">
          <div class="text-[11px] font-medium text-base-content/70">{{ t('agent.suppressedPattern') }}</div>
          <div class="mt-2 text-xs text-base-content/75">{{ rule.excerpt }}</div>
          <div v-if="rule.note" class="mt-2 text-xs text-base-content/60">{{ rule.note }}</div>
          <div class="mt-3 flex flex-wrap gap-2">
            <span v-for="term in rule.match_terms" :key="term" class="badge badge-sm badge-ghost">
              {{ term }}
            </span>
          </div>
            <div class="mt-3 grid gap-2 text-[11px] text-base-content/60 sm:grid-cols-2">
              <div>{{ t('agent.matchedTimes', { count: rule.hit_count ?? 0 }) }}</div>
              <div v-if="rule.last_matched_at_ms">
                {{ t('agent.lastMatched', { time: formatDateTime(rule.last_matched_at_ms) }) }}
              </div>
              <div v-if="rule.ttl_days">
                {{ t('agent.ttlDays', { count: rule.ttl_days }) }}
              </div>
              <div v-if="rule.expires_at_ms">
                {{ t('agent.autoExpires', { time: formatDateTime(rule.expires_at_ms) }) }}
              </div>
            </div>
          <div v-if="rule.last_matched_excerpt" class="mt-3">
            <div class="text-[11px] font-medium text-base-content/70">{{ t('agent.lastMatchExcerpt') }}</div>
            <div class="mt-1 text-xs text-base-content/70">{{ rule.last_matched_excerpt }}</div>
          </div>
          <div class="mt-3 text-[11px] text-base-content/55">
            {{ t('agent.createdUpdated', {
              created: formatDateTime(rule.created_at_ms),
              updated: formatDateTime(rule.updated_at_ms)
            }) }}
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import {
  formatCandidateDateTime,
  formatSuppressionRuleCategory,
  suppressionRuleCategoryClass,
} from '../skillsManagerHelpers'
import type { SkillCandidateSuppressionRule } from './skillsManagerTypes'

const props = defineProps<{
  suppressionRules: SkillCandidateSuppressionRule[]
  actionById: Record<string, 'delete' | 'extend' | 'expire'>
}>()

defineEmits<{
  'refresh': []
  'extend': [rule: SkillCandidateSuppressionRule, days: number]
  'expire': [rule: SkillCandidateSuppressionRule]
  'delete': [rule: SkillCandidateSuppressionRule]
}>()

const { t, locale } = useI18n()

const isBusy = (ruleId: string) => !!props.actionById[ruleId]

const formatDateTime = (timestamp?: number | null) =>
  formatCandidateDateTime(timestamp, locale.value)
</script>
