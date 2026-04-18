<template>
  <div class="rounded-xl border border-base-300 bg-base-200/30 p-4">
    <div class="text-xs font-semibold uppercase tracking-wide text-base-content/60 mb-3">
      {{ t('settings.agent.permissionHistory.filtersTitle') }}
    </div>
    <div class="grid grid-cols-1 lg:grid-cols-2 xl:grid-cols-6 gap-3">
      <label class="form-control">
        <span class="label-text text-sm">
          {{ t('settings.agent.permissionHistory.executionId') }}
        </span>
        <input
          :value="executionIdFilter"
          type="text"
          class="input input-bordered input-sm font-mono"
          :placeholder="t('settings.agent.permissionHistory.executionIdPlaceholder')"
          @input="updateExecutionIdFilter"
          @keyup.enter="$emit('apply')"
        />
      </label>

      <label class="form-control">
        <span class="label-text text-sm">
          {{ t('settings.agent.permissionHistory.commandSearch') }}
        </span>
        <input
          :value="commandSearchFilter"
          type="text"
          class="input input-bordered input-sm font-mono"
          :placeholder="t('settings.agent.permissionHistory.commandSearchPlaceholder')"
          @input="updateCommandSearchFilter"
        />
      </label>

      <label class="form-control">
        <span class="label-text text-sm">
          {{ t('settings.agent.permissionHistory.exactDate') }}
        </span>
        <input
          :value="selectedDate"
          type="date"
          class="input input-bordered input-sm"
          @input="updateSelectedDate"
        />
      </label>

      <label class="form-control">
        <span class="label-text text-sm">
          {{ t('settings.agent.permissionHistory.recentDays') }}
        </span>
        <select
          :value="recentDays"
          class="select select-bordered select-sm"
          :disabled="!!selectedDate"
          @change="updateRecentDays"
        >
          <option :value="1">1</option>
          <option :value="3">3</option>
          <option :value="7">7</option>
          <option :value="14">14</option>
          <option :value="30">30</option>
        </select>
      </label>

      <label class="form-control">
        <span class="label-text text-sm">
          {{ t('settings.agent.permissionHistory.decision') }}
        </span>
        <select
          :value="decisionFilter"
          class="select select-bordered select-sm"
          @change="updateDecisionFilter"
        >
          <option value="all">{{ t('settings.agent.permissionHistory.allDecisions') }}</option>
          <option value="allow">
            {{ t('settings.agent.permissionHistory.decisions.allow') }}
          </option>
          <option value="deny">
            {{ t('settings.agent.permissionHistory.decisions.deny') }}
          </option>
          <option value="allow_forever">
            {{ t('settings.agent.permissionHistory.decisions.allowForever') }}
          </option>
        </select>
      </label>

      <label class="form-control">
        <span class="label-text text-sm">
          {{ t('settings.agent.permissionHistory.semanticKind') }}
        </span>
        <select
          :value="semanticKindFilter"
          class="select select-bordered select-sm"
          @change="updateSemanticKindFilter"
        >
          <option value="all">{{ t('settings.agent.permissionHistory.allSemanticKinds') }}</option>
          <option value="read_only">{{ t('tools.shell.semanticLabels.read_only') }}</option>
          <option value="mutating">{{ t('tools.shell.semanticLabels.mutating') }}</option>
          <option value="dangerous">{{ t('tools.shell.semanticLabels.dangerous') }}</option>
        </select>
      </label>
    </div>

    <div class="flex flex-wrap gap-2 mt-3">
      <button class="btn btn-sm btn-primary" :disabled="loading" @click="$emit('apply')">
        {{ t('settings.agent.permissionHistory.applyFilters') }}
      </button>
      <button class="btn btn-sm btn-ghost" :disabled="loading" @click="$emit('reset')">
        {{ t('settings.agent.permissionHistory.clearFilters') }}
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'

const props = defineProps<{
  loading: boolean
  executionIdFilter: string
  commandSearchFilter: string
  selectedDate: string
  recentDays: number
  decisionFilter: string
  semanticKindFilter: string
}>()

const emit = defineEmits<{
  apply: []
  reset: []
  'update:executionIdFilter': [value: string]
  'update:commandSearchFilter': [value: string]
  'update:selectedDate': [value: string]
  'update:recentDays': [value: number]
  'update:decisionFilter': [value: string]
  'update:semanticKindFilter': [value: string]
}>()

const { t } = useI18n()

const updateExecutionIdFilter = (event: Event) => {
  emit('update:executionIdFilter', (event.target as HTMLInputElement).value.trim())
}

const updateCommandSearchFilter = (event: Event) => {
  emit('update:commandSearchFilter', (event.target as HTMLInputElement).value.trim())
}

const updateSelectedDate = (event: Event) => {
  emit('update:selectedDate', (event.target as HTMLInputElement).value)
}

const updateRecentDays = (event: Event) => {
  emit('update:recentDays', Number((event.target as HTMLSelectElement).value))
}

const updateDecisionFilter = (event: Event) => {
  emit('update:decisionFilter', (event.target as HTMLSelectElement).value)
}

const updateSemanticKindFilter = (event: Event) => {
  emit('update:semanticKindFilter', (event.target as HTMLSelectElement).value)
}

void props
</script>
