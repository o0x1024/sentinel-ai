<template>
  <div class="space-y-4">
    <div class="rounded-lg border border-base-300 bg-base-100 p-4">
      <div class="mb-3 flex flex-wrap items-center justify-between gap-2">
        <div>
          <p class="text-sm font-semibold">{{ wb('plan.title') }}</p>
          <p class="text-xs text-base-content/60">{{ wb('plan.summary') }}</p>
        </div>
        <span class="badge badge-outline">{{ wb('plan.count', { count: plans.length }) }}</span>
      </div>

      <div v-if="plans.length > 0" class="space-y-3">
        <div
          v-for="plan in plans"
          :key="plan.id"
          class="rounded-lg border border-base-300 bg-base-200/50 p-4 space-y-3"
        >
          <div class="flex flex-wrap items-center gap-2">
            <span class="badge badge-secondary">{{ getSuggestionStrategyLabel(plan.strategy) }}</span>
            <span
              :class="[
                'badge',
                plan.severity === 'high' ? 'badge-error' : plan.severity === 'medium' ? 'badge-warning' : 'badge-ghost',
              ]"
            >
              {{ wb(`priority.${plan.severity}`) }}
            </span>
            <span :class="plan.readOnly ? 'badge badge-success' : 'badge badge-warning'">
              {{ plan.readOnly ? wb('plan.readonlyPreferred') : wb('plan.manualConfirm') }}
            </span>
            <span class="badge badge-outline">{{ plan.targetField }}</span>
          </div>

          <div>
            <p class="font-medium">{{ plan.title }}</p>
            <p class="mt-1 text-sm text-base-content/70">{{ plan.summary }}</p>
            <p class="mt-2 text-xs text-base-content/60">{{ plan.rationale }}</p>
          </div>

          <div class="rounded-lg bg-base-100 p-3">
            <p class="text-xs text-base-content/60 mb-1">{{ wb('plan.targetRequest') }}</p>
            <p class="font-mono text-xs break-all">{{ plan.targetMethod }} {{ plan.targetUrl }}</p>
          </div>

          <div>
            <p class="text-xs text-base-content/60 mb-2">{{ wb('plan.candidateValues') }}</p>
            <div class="flex flex-wrap gap-2">
              <span
                v-for="value in plan.candidateValues"
                :key="value"
                class="badge badge-primary"
              >
                {{ value }}
              </span>
            </div>
          </div>

          <div class="grid gap-4 lg:grid-cols-2">
            <div>
              <p class="text-xs text-base-content/60 mb-2">{{ wb('plan.steps') }}</p>
              <div class="space-y-2">
                <div
                  v-for="step in plan.steps"
                  :key="step.id"
                  class="rounded-lg bg-base-100 p-3"
                >
                  <p class="text-sm font-medium">{{ step.title }}</p>
                  <p class="mt-1 text-xs text-base-content/60">{{ step.detail }}</p>
                </div>
              </div>
            </div>
            <div>
              <p class="text-xs text-base-content/60 mb-2">{{ wb('plan.stopConditions') }}</p>
              <div class="space-y-2">
                <div
                  v-for="condition in plan.stopConditions"
                  :key="condition.id"
                  class="rounded-lg bg-base-100 p-3"
                >
                  <p class="text-xs text-base-content/70">{{ condition.detail }}</p>
                </div>
              </div>
            </div>
          </div>

          <div class="flex flex-wrap justify-end gap-2">
            <button class="btn btn-sm btn-outline" @click="copyPlan(plan)">{{ wb('plan.copySummary') }}</button>
            <button class="btn btn-sm btn-primary" @click="$emit('create-draft', plan)">{{ wb('plan.saveDraft') }}</button>
          </div>
        </div>
      </div>

      <p v-else class="text-sm text-base-content/60">
        {{ wb('plan.empty') }}
      </p>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { dialog } from '@/composables/useDialog'
import type { WorkbenchCase, WorkbenchReplayPlan } from './securityWorkbenchTypes'
import { buildWorkbenchObjectAnalysis, getSuggestionStrategyLabel } from './securityWorkbenchObjectAnalysis'
import { wb } from './securityWorkbenchLocale'
import { buildWorkbenchReplayPlans, formatWorkbenchReplayPlan } from './securityWorkbenchReplayPlan'

const props = defineProps<{
  caseItem: WorkbenchCase
}>()

defineEmits<{
  'create-draft': [plan: WorkbenchReplayPlan]
}>()

const analysis = computed(() => buildWorkbenchObjectAnalysis(props.caseItem))
const plans = computed(() => buildWorkbenchReplayPlans(props.caseItem, analysis.value))

const copyPlan = async (plan: WorkbenchReplayPlan) => {
  try {
    await navigator.clipboard.writeText(formatWorkbenchReplayPlan(plan))
    dialog.toast.success(wb('plan.copySuccess'))
  } catch (error) {
    console.error('Failed to copy replay plan', error)
    dialog.toast.error(wb('plan.copyFailed'))
  }
}
</script>
