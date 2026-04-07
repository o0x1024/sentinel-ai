<template>
  <div v-if="current" class="bg-base-100 rounded-lg shadow-sm border border-base-300 p-4 space-y-3">
    <div class="flex flex-wrap items-center justify-between gap-3">
      <div>
        <div class="text-sm font-semibold">评测对照结果</div>
        <div class="text-xs text-base-content/60">
          {{ formatTime(current.comparedAt) }} · 场景 {{ current.evalScenarioCount }} · findings
          {{ current.findingCount }}
        </div>
      </div>
      <div class="flex items-center gap-2">
        <button
          v-if="history.length > 1"
          @click="$emit('clear-history')"
          class="btn btn-ghost btn-xs"
        >
          清空历史
        </button>
        <button @click="$emit('clear-current')" class="btn btn-ghost btn-xs">清空当前</button>
      </div>
    </div>

    <div class="grid grid-cols-2 md:grid-cols-5 gap-3">
      <div class="stat bg-base-200/60 rounded-lg">
        <div class="stat-title text-xs">TP 已验证</div>
        <div class="stat-value text-success text-base">{{ getCount('TP-VERIFIED') }}</div>
      </div>
      <div class="stat bg-base-200/60 rounded-lg">
        <div class="stat-title text-xs">TP 候选</div>
        <div class="stat-value text-info text-base">{{ getCount('TP-CANDIDATE') }}</div>
      </div>
      <div class="stat bg-base-200/60 rounded-lg">
        <div class="stat-title text-xs">FP</div>
        <div class="stat-value text-error text-base">{{ getCount('FP') }}</div>
      </div>
      <div class="stat bg-base-200/60 rounded-lg">
        <div class="stat-title text-xs">FP 候选</div>
        <div class="stat-value text-warning text-base">{{ getCount('FP-CANDIDATE') }}</div>
      </div>
      <div class="stat bg-base-200/60 rounded-lg">
        <div class="stat-title text-xs">FN</div>
        <div class="stat-value text-error text-base">{{ getCount('FN') }}</div>
      </div>
    </div>

    <div v-if="history.length > 1" class="space-y-2">
      <div class="text-xs font-medium text-base-content/70">最近评测历史</div>
      <div class="flex flex-wrap gap-2">
        <button
          v-for="item in history"
          :key="item.comparedAt"
          class="btn btn-xs"
          :class="item.comparedAt === current.comparedAt ? 'btn-primary' : 'btn-outline'"
          @click="$emit('select-history', item.comparedAt)"
        >
          {{ formatTime(item.comparedAt) }}
        </button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import type { EvaluationComparisonSummary } from './vulnerabilitiesEvaluationSupport'

const props = defineProps<{
  current: EvaluationComparisonSummary | null
  history: EvaluationComparisonSummary[]
}>()

defineEmits<{
  'select-history': [comparedAt: string]
  'clear-current': []
  'clear-history': []
}>()

const getCount = (key: string) => Number(props.current?.summary?.[key] || 0)

const formatTime = (value: string) => {
  if (!value) return '-'
  const date = new Date(value)
  if (Number.isNaN(date.getTime())) return value
  return date.toLocaleString()
}
</script>
