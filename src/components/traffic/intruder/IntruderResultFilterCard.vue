<template>
  <div class="rounded border border-base-300 bg-base-200 px-3 py-2 text-sm text-base-content/70">
    <div class="flex items-center gap-2">
      <i class="fas fa-filter"></i>
      <span class="font-medium text-base-content">{{ label }}</span>
      <span class="min-w-0 flex-1 truncate">{{ summary }}</span>
      <input
        :checked="filter.enabled"
        type="checkbox"
        class="checkbox checkbox-xs"
        @change="updateFilter({ enabled: ($event.target as HTMLInputElement).checked })"
      />
      <button class="btn btn-ghost btn-xs" type="button" @click="openDialog">
        {{ $t('trafficAnalysis.intruder.actions.edit') }}
      </button>
    </div>

    <AppDialog ref="dialogRef" class="modal">
      <div class="modal-box max-w-2xl">
        <h3 class="mb-4 text-lg font-semibold">{{ label }}</h3>

        <div class="space-y-4">
          <p class="text-sm text-base-content/70">
            {{ $t('trafficAnalysis.intruder.help.resultFilterHint') }}
          </p>

          <label class="flex items-center gap-2">
            <input v-model="draftFilter.enabled" type="checkbox" class="checkbox checkbox-sm" />
            <span>{{ $t('trafficAnalysis.intruder.labels.enabled') }}</span>
          </label>

          <div class="grid gap-3 md:grid-cols-2">
            <label class="form-control">
              <span class="label-text text-xs">{{ $t('trafficAnalysis.intruder.labels.rule') }}</span>
              <input
                v-model="draftFilter.query"
                type="text"
                class="input input-bordered input-sm"
                :placeholder="$t('trafficAnalysis.intruder.placeholders.resultFilter')"
              />
            </label>

            <label class="form-control">
              <span class="label-text text-xs">{{ $t('trafficAnalysis.intruder.labels.status') }}</span>
              <input
                v-model="draftFilter.statusCode"
                type="text"
                class="input input-bordered input-sm"
                :placeholder="$t('trafficAnalysis.intruder.placeholders.statusFilter')"
              />
            </label>
          </div>

          <div class="flex flex-wrap gap-2">
            <button class="btn btn-xs btn-ghost" type="button" @click="applyPreset('all')">
              {{ $t('trafficAnalysis.intruder.actions.reset') }}
            </button>
            <button class="btn btn-xs btn-ghost" type="button" @click="applyPreset('2xx')">2xx</button>
            <button class="btn btn-xs btn-ghost" type="button" @click="applyPreset('4xx')">4xx</button>
            <button class="btn btn-xs btn-ghost" type="button" @click="applyPreset('5xx')">5xx</button>
            <button class="btn btn-xs btn-ghost" type="button" @click="applyPreset('errors')">
              {{ $t('trafficAnalysis.intruder.labels.onlyErrors') }}
            </button>
          </div>

          <div class="flex flex-wrap items-center gap-3 text-sm">
            <label class="flex items-center gap-2">
              <input v-model="draftFilter.invert" type="checkbox" class="checkbox checkbox-sm" />
              <span>{{ $t('trafficAnalysis.intruder.labels.invert') }}</span>
            </label>
            <label class="flex items-center gap-2">
              <input v-model="draftFilter.onlyErrors" type="checkbox" class="checkbox checkbox-sm" />
              <span>{{ $t('trafficAnalysis.intruder.labels.onlyErrors') }}</span>
            </label>
            <label class="flex items-center gap-2">
              <input v-model="draftFilter.hideBaseline" type="checkbox" class="checkbox checkbox-sm" />
              <span>{{ $t('trafficAnalysis.intruder.labels.hideBaseline') }}</span>
            </label>
          </div>

          <div v-if="grepMatchRules.length" class="rounded-lg border border-base-300">
            <div class="border-b border-base-300 bg-base-100 px-4 py-2 text-xs font-semibold uppercase tracking-wide text-base-content/70">
              {{ $t('trafficAnalysis.intruder.labels.grepMatch') }}
            </div>
            <div class="grid gap-2 p-4 md:grid-cols-2">
              <label
                v-for="rule in grepMatchRules"
                :key="`${kind}-${rule.id}`"
                class="flex items-center gap-2 rounded border border-base-300 px-3 py-2"
              >
                <input
                  :checked="draftFilter.grepMatchRuleIds.includes(rule.id)"
                  type="checkbox"
                  class="checkbox checkbox-sm"
                  @change="toggleRule(rule.id, ($event.target as HTMLInputElement).checked)"
                />
                <span class="min-w-0 flex-1 truncate">{{ rule.name }}</span>
              </label>
            </div>
          </div>

          <div class="rounded-lg border border-base-300">
            <div class="flex items-center justify-between border-b border-base-300 bg-base-100 px-4 py-2 text-xs font-semibold uppercase tracking-wide text-base-content/70">
              <span>{{ $t('trafficAnalysis.intruder.labels.columns') }}</span>
              <button class="btn btn-ghost btn-xs" type="button" @click="addColumnFilter">
                {{ $t('trafficAnalysis.intruder.actions.add') }}
              </button>
            </div>
            <div class="space-y-3 p-4">
              <div
                v-for="columnFilter in draftFilter.columnFilters"
                :key="columnFilter.id"
                class="grid gap-2 rounded border border-base-300 p-3 md:grid-cols-[minmax(0,1fr)_10rem_minmax(0,1fr)_auto]"
              >
                <select
                  :value="columnFilter.key"
                  class="select select-bordered select-sm"
                  @change="updateColumnFilter(columnFilter.id, 'key', ($event.target as HTMLSelectElement).value)"
                >
                  <option v-for="column in columns" :key="column.key" :value="column.key">
                    {{ column.label }}
                  </option>
                </select>

                <select
                  :value="columnFilter.operator"
                  class="select select-bordered select-sm"
                  @change="updateColumnFilter(columnFilter.id, 'operator', ($event.target as HTMLSelectElement).value)"
                >
                  <option value="contains">{{ $t('trafficAnalysis.intruder.labels.operatorContains') }}</option>
                  <option value="equals">{{ $t('trafficAnalysis.intruder.labels.operatorEquals') }}</option>
                  <option value="notEquals">{{ $t('trafficAnalysis.intruder.labels.operatorNotEquals') }}</option>
                  <option value="startsWith">{{ $t('trafficAnalysis.intruder.labels.operatorStartsWith') }}</option>
                  <option value="greaterThan">{{ $t('trafficAnalysis.intruder.labels.operatorGreaterThan') }}</option>
                  <option value="lessThan">{{ $t('trafficAnalysis.intruder.labels.operatorLessThan') }}</option>
                </select>

                <input
                  :value="columnFilter.value"
                  type="text"
                  class="input input-bordered input-sm"
                  :placeholder="$t('trafficAnalysis.intruder.placeholders.ruleValue')"
                  @input="updateColumnFilter(columnFilter.id, 'value', ($event.target as HTMLInputElement).value)"
                />

                <button class="btn btn-ghost btn-sm" type="button" @click="removeColumnFilter(columnFilter.id)">
                  {{ $t('trafficAnalysis.intruder.actions.remove') }}
                </button>
              </div>

              <div v-if="!draftFilter.columnFilters.length" class="py-3 text-sm text-base-content/60">
                {{ $t('trafficAnalysis.intruder.empty.noColumnFilters') }}
              </div>
            </div>
          </div>
        </div>

        <div class="mt-6 flex justify-between gap-2">
          <button class="btn btn-ghost" type="button" @click="resetDraft">
            {{ $t('trafficAnalysis.intruder.actions.reset') }}
          </button>
          <div class="flex gap-2">
            <button class="btn btn-ghost" type="button" @click="closeDialog">
              {{ $t('common.cancel') }}
            </button>
            <button class="btn btn-primary" type="button" @click="saveFilter">
              {{ $t('common.save') }}
            </button>
          </div>
        </div>
      </div>

      <form method="dialog" class="modal-backdrop">
        <button>{{ $t('common.close') }}</button>
      </form>
    </AppDialog>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { createIntruderId } from './http'
import type { IntruderResultColumn } from './analysis'
import { createDefaultResultFilter, summarizeIntruderResultFilter } from './results'
import type { IntruderGrepMatchRule, IntruderResultColumnFilter, IntruderResultFilter } from './types'

const props = defineProps<{
  kind: 'capture' | 'view'
  label: string
  filter: IntruderResultFilter
  grepMatchRules: IntruderGrepMatchRule[]
  columns: IntruderResultColumn[]
}>()

const emit = defineEmits<{
  (e: 'update:filter', value: IntruderResultFilter): void
}>()

useI18n()

const dialogRef = ref<HTMLDialogElement | null>(null)
const draftFilter = ref<IntruderResultFilter>(cloneFilter(props.filter))
const summary = computed(() => summarizeIntruderResultFilter(props.filter, props.kind))

function openDialog() {
  draftFilter.value = cloneFilter(props.filter)
  dialogRef.value?.showModal()
}

function closeDialog() {
  dialogRef.value?.close()
}

function updateFilter(patch: Partial<IntruderResultFilter>) {
  emit('update:filter', { ...props.filter, ...patch })
}

function toggleRule(ruleId: string, checked: boolean) {
  const nextRuleIds = checked
    ? [...new Set([...draftFilter.value.grepMatchRuleIds, ruleId])]
    : draftFilter.value.grepMatchRuleIds.filter((id) => id !== ruleId)
  draftFilter.value = { ...draftFilter.value, grepMatchRuleIds: nextRuleIds }
}

function resetDraft() {
  draftFilter.value = createDefaultResultFilter()
}

function saveFilter() {
  emit('update:filter', cloneFilter(draftFilter.value))
  closeDialog()
}

function applyPreset(preset: 'all' | '2xx' | '4xx' | '5xx' | 'errors') {
  const nextFilter = createDefaultResultFilter()
  switch (preset) {
    case 'all':
      draftFilter.value = nextFilter
      return
    case '2xx':
    case '4xx':
    case '5xx':
      draftFilter.value = { ...nextFilter, enabled: true, statusCode: preset[0] }
      return
    case 'errors':
      draftFilter.value = { ...nextFilter, enabled: true, onlyErrors: true }
      return
  }
}

function addColumnFilter() {
  draftFilter.value = {
    ...draftFilter.value,
    columnFilters: [
      ...draftFilter.value.columnFilters,
      createColumnFilter(props.columns[0]?.key || 'statusCode'),
    ],
  }
}

function updateColumnFilter<K extends keyof IntruderResultColumnFilter>(
  filterId: string,
  key: K,
  value: IntruderResultColumnFilter[K] | string,
) {
  draftFilter.value = {
    ...draftFilter.value,
    columnFilters: draftFilter.value.columnFilters.map((columnFilter) =>
      columnFilter.id === filterId ? { ...columnFilter, [key]: value } : columnFilter,
    ),
  }
}

function removeColumnFilter(filterId: string) {
  draftFilter.value = {
    ...draftFilter.value,
    columnFilters: draftFilter.value.columnFilters.filter((columnFilter) => columnFilter.id !== filterId),
  }
}

function createColumnFilter(key: string): IntruderResultColumnFilter {
  return {
    id: createIntruderId('result-filter'),
    key,
    operator: 'contains',
    value: '',
  }
}

function cloneFilter(filter: IntruderResultFilter): IntruderResultFilter {
  return {
    ...filter,
    grepMatchRuleIds: [...filter.grepMatchRuleIds],
    columnFilters: filter.columnFilters.map((columnFilter) => ({ ...columnFilter })),
  }
}
</script>
