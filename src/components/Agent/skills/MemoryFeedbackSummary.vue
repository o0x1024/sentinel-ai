<template>
  <div class="rounded-2xl border border-base-300 bg-base-200/80 p-4">
    <div class="flex flex-wrap items-start justify-between gap-3">
      <div>
        <div class="text-sm font-semibold">{{ t('agent.memoryFeedbackTitle') }}</div>
        <div class="text-xs text-base-content/60">
          {{ t('agent.memoryFeedbackDescription') }}
        </div>
      </div>
      <div class="flex flex-wrap gap-2 text-xs text-base-content/60">
        <span class="badge badge-sm badge-outline">{{ t('agent.skillCount', { count: skillsCount }) }}</span>
        <span class="badge badge-sm badge-outline">{{ t('agent.draftCount', { count: activeDraftCount }) }}</span>
        <span class="badge badge-sm badge-outline">{{ t('agent.ruleCount', { count: ruleCount }) }}</span>
      </div>
    </div>

    <div class="mt-4 grid gap-3 xl:grid-cols-[minmax(0,1.1fr)_minmax(0,0.9fr)]">
      <div class="rounded-xl border border-base-300 bg-base-100 px-4 py-3">
        <div class="flex items-start justify-between gap-3">
          <div class="flex items-start gap-3">
            <div class="mt-0.5 flex h-9 w-9 items-center justify-center rounded-xl bg-primary/10 text-primary">
              <i class="fas fa-seedling"></i>
            </div>
            <div>
              <div class="text-sm font-semibold">{{ t('agent.candidateDraftsTitle') }}</div>
              <div class="text-xs text-base-content/60">
                {{ t('agent.candidateDraftsDescription') }}
              </div>
            </div>
          </div>
          <span class="badge badge-sm badge-primary badge-outline">{{ totalDraftCount }}</span>
        </div>
        <div class="mt-3 flex flex-wrap gap-2 text-[11px]">
          <span class="badge badge-sm badge-outline">{{ t('agent.activeCount', { count: activeDraftCount }) }}</span>
          <span class="badge badge-sm badge-ghost">{{ t('agent.reviewedCount', { count: reviewedDraftCount }) }}</span>
        </div>
        <div class="mt-4 flex flex-wrap items-center justify-between gap-3">
          <div class="text-xs text-base-content/60">
            <template v-if="loadingCandidates">{{ t('agent.loadingCandidateDrafts') }}</template>
            <template v-else-if="totalDraftCount === 0">{{ t('agent.noDraftBacklog') }}</template>
            <template v-else>{{ t('agent.draftsNeedReview', { count: activeDraftCount }) }}</template>
          </div>
          <div class="flex items-center gap-2">
            <button class="btn btn-xs btn-ghost" @click="$emit('refresh-candidates')">
              <i class="fas fa-rotate-right mr-1"></i>
              {{ t('common.refresh') }}
            </button>
            <button
              v-if="totalDraftCount > 0"
              class="btn btn-xs btn-outline"
              @click="$emit('toggle-candidate-details')"
            >
              <i
                :class="showCandidateDetails
                  ? 'fas fa-chevron-up mr-1'
                  : 'fas fa-chevron-down mr-1'"
              ></i>
              {{ showCandidateDetails ? t('agent.hideDetails') : t('agent.viewDetails') }}
            </button>
          </div>
        </div>
      </div>

      <div class="rounded-xl border border-base-300 bg-base-100 px-4 py-3">
        <div class="flex items-start justify-between gap-3">
          <div class="flex items-start gap-3">
            <div class="mt-0.5 flex h-9 w-9 items-center justify-center rounded-xl bg-warning/10 text-warning">
              <i class="fas fa-shield-slash"></i>
            </div>
            <div>
              <div class="text-sm font-semibold">{{ t('agent.suppressionRulesTitle') }}</div>
              <div class="text-xs text-base-content/60">
                {{ t('agent.suppressionRulesDescription') }}
              </div>
            </div>
          </div>
          <span class="badge badge-sm badge-warning badge-outline">{{ ruleCount }}</span>
        </div>
        <div class="mt-3 flex flex-wrap gap-2 text-[11px]">
          <span class="badge badge-sm badge-outline">{{ t('agent.activeCount', { count: ruleCount }) }}</span>
          <span class="badge badge-sm badge-ghost">{{ t('agent.totalHitsCount', { count: totalRuleHits }) }}</span>
        </div>
        <div class="mt-4 flex flex-wrap items-center justify-between gap-3">
          <div class="text-xs text-base-content/60">
            <template v-if="ruleCount === 0">{{ t('agent.noSuppressionRules') }}</template>
            <template v-else>{{ t('agent.suppressionRulesShapingCapture', { count: ruleCount }) }}</template>
          </div>
          <div class="flex items-center gap-2">
            <button class="btn btn-xs btn-ghost" @click="$emit('refresh-rules')">
              <i class="fas fa-rotate-right mr-1"></i>
              {{ t('common.refresh') }}
            </button>
            <button
              v-if="ruleCount > 0"
              class="btn btn-xs btn-outline"
              @click="$emit('toggle-rule-details')"
            >
              <i
                :class="showSuppressionRuleDetails
                  ? 'fas fa-chevron-up mr-1'
                  : 'fas fa-chevron-down mr-1'"
              ></i>
              {{ showSuppressionRuleDetails ? t('agent.hideDetails') : t('agent.viewDetails') }}
            </button>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'

defineProps<{
  skillsCount: number
  totalDraftCount: number
  activeDraftCount: number
  reviewedDraftCount: number
  ruleCount: number
  totalRuleHits: number
  loadingCandidates: boolean
  showCandidateDetails: boolean
  showSuppressionRuleDetails: boolean
}>()

defineEmits<{
  'refresh-candidates': []
  'refresh-rules': []
  'toggle-candidate-details': []
  'toggle-rule-details': []
}>()

const { t } = useI18n()
</script>
