<template>
  <div class="rounded-lg border border-base-300 bg-base-100 overflow-hidden">
      <div v-if="loading" class="p-8 text-center">
      <span class="loading loading-spinner loading-lg"></span>
      <p class="mt-2 text-sm text-base-content/60">{{ wb('caseList.loading') }}</p>
    </div>
    <div
      v-else-if="cases.length > 0"
      class="space-y-3 p-4"
    >
      <div
        v-if="selectedCount > 0"
        class="flex flex-col gap-3 rounded-lg border border-base-300 bg-base-200/40 p-3 lg:flex-row lg:items-center lg:justify-between"
      >
        <div class="text-sm text-base-content/70">
          {{ wb('caseList.selectedSummary', { selectedCount, pageCount: cases.length, total }) }}
        </div>
        <div class="flex flex-wrap items-center gap-2">
          <button class="btn btn-sm btn-outline" @click="$emit('clear-selection')">
            {{ wb('caseList.clearSelection') }}
          </button>
          <button class="btn btn-sm btn-error" @click="$emit('delete-selected')">
            {{ wb('caseList.deleteSelected') }}
          </button>
        </div>
      </div>

      <div class="overflow-x-auto">
        <table class="table table-zebra">
          <thead>
            <tr>
              <th class="w-12">
                <input
                  type="checkbox"
                  class="checkbox checkbox-sm"
                  :checked="allCurrentPageSelected"
                  @click.stop
                  @change="$emit('toggle-select-current-page')"
                />
              </th>
              <th>{{ wb('caseList.case') }}</th>
              <th>{{ wb('caseList.riskType') }}</th>
              <th>{{ wb('caseList.status') }}</th>
              <th>{{ wb('caseList.noteCount') }}</th>
              <th>{{ wb('caseList.lastActivity') }}</th>
              <th>{{ wb('caseList.actions') }}</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="item in cases" :key="item.id">
              <td @click.stop>
                <input
                  type="checkbox"
                  class="checkbox checkbox-sm"
                  :checked="selectedIdSet.has(item.id)"
                  @change="$emit('toggle-selection', item.id)"
                />
              </td>
              <td class="max-w-sm">
                <div class="space-y-1">
                  <p class="font-medium line-clamp-2">{{ item.title }}</p>
                  <p class="text-xs text-base-content/60">{{ getWorkbenchCaseSubtitle(item) }}</p>
                </div>
              </td>
              <td><span class="badge badge-outline">{{ item.finding.vulnType }}</span></td>
              <td>
                <span :class="getWorkbenchStatusBadgeClass(item.status)" class="badge">
                  {{ getWorkbenchStatusLabel(item.status) }}
                </span>
              </td>
              <td><span class="badge badge-ghost">{{ item.noteCount }}</span></td>
              <td class="text-xs text-base-content/60">{{ formatWorkbenchTime(item.lastActivityAt) }}</td>
              <td>
                <div class="flex flex-wrap gap-2">
                  <button class="btn btn-xs btn-outline" @click="$emit('open-case', item.id)">
                    {{ wb('common.open') }}
                  </button>
                  <button class="btn btn-xs btn-error btn-outline" @click="$emit('delete-case', item.id)">
                    {{ wb('common.delete') }}
                  </button>
                </div>
              </td>
            </tr>
          </tbody>
        </table>
      </div>
      <div class="flex flex-col gap-3 border-t border-base-300 px-1 pt-3 text-sm xl:flex-row xl:items-center xl:justify-between">
        <div class="flex items-center gap-2 text-sm">
          <span class="text-base-content/60">{{ wb('caseList.perPage') }}</span>
          <select
            class="select select-bordered select-sm"
            :value="pageSize"
            @change="handlePageSizeChange"
          >
            <option v-for="size in [10, 20, 50]" :key="size" :value="size">
              {{ size }}
            </option>
          </select>
          <span class="text-base-content/60">{{ wb('caseList.totalCount', { total }) }}</span>
        </div>
        <div class="flex flex-col gap-2 sm:flex-row sm:flex-wrap sm:items-center sm:justify-end">
          <div class="join">
            <button
              class="join-item btn btn-sm"
              :disabled="page <= 1"
              @click="$emit('change-page', 1)"
            >
              {{ wb('caseList.firstPage') }}
            </button>
            <button
              class="join-item btn btn-sm"
              :disabled="page <= 1"
              @click="$emit('change-page', page - 1)"
            >
              {{ wb('caseList.previousPage') }}
            </button>
            <button class="join-item btn btn-sm">
              {{ wb('caseList.pageIndicator', { page, totalPages }) }}
            </button>
            <button
              class="join-item btn btn-sm"
              :disabled="page >= totalPages"
              @click="$emit('change-page', page + 1)"
            >
              {{ wb('caseList.nextPage') }}
            </button>
            <button
              class="join-item btn btn-sm"
              :disabled="page >= totalPages"
              @click="$emit('change-page', totalPages)"
            >
              {{ wb('caseList.lastPage') }}
            </button>
          </div>
        </div>
      </div>
    </div>
    <div
      v-else
      class="p-8 text-center text-sm text-base-content/60"
    >
      {{ wb('caseList.empty') }}
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import type { WorkbenchCaseListItem } from './securityWorkbenchTypes'
import {
  formatWorkbenchTime,
  getWorkbenchCaseSubtitle,
  getWorkbenchStatusBadgeClass,
  getWorkbenchStatusLabel,
} from './securityWorkbenchPresentation'
import { wb } from './securityWorkbenchLocale'

const props = defineProps<{
  cases: WorkbenchCaseListItem[]
  total: number
  page: number
  pageSize: number
  loading: boolean
  selectedIds: string[]
}>()

const emit = defineEmits<{
  'open-case': [caseId: string]
  'change-page': [page: number]
  'change-page-size': [pageSize: number]
  'toggle-selection': [caseId: string]
  'toggle-select-current-page': []
  'clear-selection': []
  'delete-case': [caseId: string]
  'delete-selected': []
}>()

const selectedIdSet = computed(() => new Set(props.selectedIds))
const selectedCount = computed(() => props.selectedIds.length)
const allCurrentPageSelected = computed(() =>
  props.cases.length > 0 && props.cases.every(item => selectedIdSet.value.has(item.id)),
)
const totalPages = computed(() => Math.max(1, Math.ceil(props.total / props.pageSize)))

const handlePageSizeChange = (event: Event) => {
  const target = event.target as HTMLSelectElement | null
  const nextValue = Number(target?.value || props.pageSize)
  emit('change-page-size', nextValue)
}
</script>
