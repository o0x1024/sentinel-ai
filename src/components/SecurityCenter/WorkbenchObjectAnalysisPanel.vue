<template>
  <div class="space-y-4">
    <div class="rounded-lg border border-base-300 bg-base-100 p-4">
      <div class="mb-3 flex flex-wrap items-center justify-between gap-2">
        <div>
          <p class="text-sm font-semibold">{{ wb('analysis.behaviorTitle') }}</p>
          <p class="text-xs text-base-content/60">{{ wb('analysis.behaviorSummary') }}</p>
        </div>
        <span class="badge badge-outline">{{
          wb('analysis.behaviorCount', { count: analysis.behaviorSteps.length })
        }}</span>
      </div>

      <div v-if="analysis.behaviorSteps.length > 0" class="space-y-3">
        <div
          v-for="step in analysis.behaviorSteps"
          :key="step.evidenceId"
          class="rounded-lg bg-base-200/70 p-3"
        >
          <div class="flex flex-wrap items-center justify-between gap-2">
            <div class="flex flex-wrap items-center gap-2">
              <span class="badge badge-outline">{{ getBehaviorStepLabel(step.stepKind) }}</span>
              <span class="badge badge-ghost">{{ step.method }}</span>
            </div>
            <span class="text-xs text-base-content/60">{{
              formatWorkbenchTime(step.timestamp)
            }}</span>
          </div>
          <SecurityEvidenceTextBlock class="mt-2" :text="step.url" mono size="xs" />
          <SecurityEvidenceTextBlock class="mt-2" :text="step.reason" size="xs" />
        </div>
      </div>
      <p v-else class="text-sm text-base-content/60">{{ wb('analysis.behaviorEmpty') }}</p>
    </div>

    <div class="rounded-lg border border-base-300 bg-base-100 p-4">
      <div class="mb-3 flex flex-wrap items-center justify-between gap-2">
        <div>
          <p class="text-sm font-semibold">{{ wb('analysis.objectPoolTitle') }}</p>
          <p class="text-xs text-base-content/60">{{ wb('analysis.objectPoolSummary') }}</p>
        </div>
        <span class="badge badge-outline">{{
          wb('analysis.objectPoolCount', { count: analysis.objectPool.length })
        }}</span>
      </div>

      <div v-if="analysis.objectPool.length > 0" class="space-y-3">
        <div
          v-for="group in analysis.objectPool"
          :key="group.key"
          class="rounded-lg border border-base-300 bg-base-200/40 p-3"
        >
          <div class="flex flex-wrap items-center gap-2">
            <span class="badge badge-outline">{{ group.label }}</span>
            <span class="badge badge-ghost">{{ getObjectRoleLabel(group.role) }}</span>
            <span :class="getConfidenceBadgeClass(group.confidence)" class="badge">
              {{ wb(`priority.${group.confidence}`) }}
            </span>
            <span class="text-xs text-base-content/60">
              {{
                wb('analysis.objectPoolMeta', {
                  evidenceCount: group.evidenceCount,
                  referenceCount: group.referenceCount,
                })
              }}
            </span>
          </div>
          <div class="mt-3 flex flex-wrap gap-2">
            <span
              v-for="value in group.uniqueValues.slice(0, 8)"
              :key="value"
              class="badge badge-primary badge-outline"
            >
              {{ value }}
            </span>
            <span v-if="group.uniqueValues.length > 8" class="badge badge-ghost">
              {{ wb('analysis.moreValues', { count: group.uniqueValues.length - 8 }) }}
            </span>
          </div>
        </div>
      </div>
      <p v-else class="text-sm text-base-content/60">
        {{ wb('analysis.objectPoolEmpty') }}
      </p>
    </div>

    <div class="rounded-lg border border-base-300 bg-base-100 p-4">
      <div class="mb-3 flex flex-wrap items-center justify-between gap-2">
        <div>
          <p class="text-sm font-semibold">{{ wb('analysis.suggestionsTitle') }}</p>
          <p class="text-xs text-base-content/60">{{ wb('analysis.suggestionsSummary') }}</p>
        </div>
        <span class="badge badge-outline">{{
          wb('analysis.suggestionsCount', { count: analysis.suggestions.length })
        }}</span>
      </div>

      <div v-if="analysis.suggestions.length > 0" class="space-y-3">
        <div
          v-for="item in analysis.suggestions"
          :key="item.id"
          class="rounded-lg bg-base-200/70 p-3"
        >
          <div class="flex flex-wrap items-center gap-2">
            <span class="badge badge-secondary">{{
              getSuggestionStrategyLabel(item.strategy)
            }}</span>
            <span
              :class="[
                'badge',
                item.severity === 'high'
                  ? 'badge-error'
                  : item.severity === 'medium'
                    ? 'badge-warning'
                    : 'badge-ghost',
              ]"
            >
              {{ wb(`priority.${item.severity}`) }}
            </span>
            <span class="badge badge-outline">{{ item.targetField }}</span>
          </div>
          <p class="mt-2 font-medium">{{ item.title }}</p>
          <SecurityEvidenceTextBlock class="mt-1" :text="item.summary" size="sm" />
          <SecurityEvidenceTextBlock class="mt-2" :text="item.why" size="xs" />
          <div class="mt-3 flex flex-wrap gap-2">
            <span v-for="value in item.candidateValues" :key="value" class="badge badge-primary">
              {{ value }}
            </span>
          </div>
        </div>
      </div>
      <p v-else class="text-sm text-base-content/60">
        {{ wb('analysis.suggestionsEmpty') }}
      </p>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import SecurityEvidenceTextBlock from './SecurityEvidenceTextBlock.vue'
import type { WorkbenchCase } from './securityWorkbenchTypes'
import { formatWorkbenchTime } from './securityWorkbenchPresentation'
import { wb } from './securityWorkbenchLocale'
import {
  buildWorkbenchObjectAnalysis,
  getBehaviorStepLabel,
  getConfidenceBadgeClass,
  getObjectRoleLabel,
  getSuggestionStrategyLabel,
} from './securityWorkbenchObjectAnalysis'

const props = defineProps<{
  caseItem: WorkbenchCase
}>()

const analysis = computed(() => buildWorkbenchObjectAnalysis(props.caseItem))
</script>
