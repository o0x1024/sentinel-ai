<template>
  <div class="rounded-lg border border-base-300 bg-base-100 overflow-hidden">
    <div v-if="loading" class="p-8 text-center">
      <span class="loading loading-spinner loading-lg"></span>
      <p class="mt-2 text-sm text-base-content/60">{{ wb('ignoredList.loading') }}</p>
    </div>
    <div v-else-if="findings.length > 0" class="space-y-3 p-4">
      <div class="overflow-x-auto">
        <table class="table table-zebra">
          <thead>
            <tr>
              <th>{{ wb('ignoredList.finding') }}</th>
              <th>{{ wb('ignoredList.riskType') }}</th>
              <th>{{ wb('ignoredList.severity') }}</th>
              <th>{{ wb('ignoredList.lastSeen') }}</th>
              <th>{{ wb('ignoredList.actions') }}</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="item in findings" :key="item.id">
              <td class="max-w-sm">
                <div class="space-y-1 py-1">
                  <p class="font-medium line-clamp-2">{{ item.title }}</p>
                  <p class="text-xs text-base-content/60 line-clamp-1">
                    {{ item.url || item.description || item.pluginId }}
                  </p>
                </div>
              </td>
              <td><span class="badge badge-outline">{{ item.vulnType }}</span></td>
              <td>
                <span class="badge" :class="getSeverityBadgeClass(item.severity)">
                  {{ item.severity }}
                </span>
              </td>
              <td class="text-xs text-base-content/60">{{ formatWorkbenchTime(item.lastSeenAt) }}</td>
              <td>
                <div class="flex flex-wrap gap-2">
                  <button class="btn btn-xs btn-warning btn-outline" @click="$emit('restore-finding', item.id)">
                    {{ wb('ignoredList.restore') }}
                  </button>
                  <button class="btn btn-xs btn-outline" @click="$emit('open-finding', item.id)">
                    {{ wb('ignoredList.openFinding') }}
                  </button>
                </div>
              </td>
            </tr>
          </tbody>
        </table>
      </div>
      <div class="flex flex-col gap-3 border-t border-base-300 px-1 pt-3 text-sm xl:flex-row xl:items-center xl:justify-between">
        <div class="flex items-center gap-2 text-sm">
          <span class="text-base-content/60">{{ wb('ignoredList.perPage') }}</span>
          <select class="select select-bordered select-sm" :value="pageSize" @change="handlePageSizeChange">
            <option v-for="size in [10, 20, 50]" :key="size" :value="size">
              {{ size }}
            </option>
          </select>
          <span class="text-base-content/60">{{ wb('ignoredList.totalCount', { total }) }}</span>
        </div>
        <div class="join">
          <button class="join-item btn btn-sm" :disabled="page <= 1" @click="$emit('change-page', 1)">
            {{ wb('ignoredList.firstPage') }}
          </button>
          <button class="join-item btn btn-sm" :disabled="page <= 1" @click="$emit('change-page', page - 1)">
            {{ wb('ignoredList.previousPage') }}
          </button>
          <button class="join-item btn btn-sm">
            {{ wb('ignoredList.pageIndicator', { page, totalPages }) }}
          </button>
          <button class="join-item btn btn-sm" :disabled="page >= totalPages" @click="$emit('change-page', page + 1)">
            {{ wb('ignoredList.nextPage') }}
          </button>
          <button class="join-item btn btn-sm" :disabled="page >= totalPages" @click="$emit('change-page', totalPages)">
            {{ wb('ignoredList.lastPage') }}
          </button>
        </div>
      </div>
    </div>
    <div v-else class="p-8 text-center text-sm text-base-content/60">
      {{ wb('ignoredList.empty') }}
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import type { WorkbenchIgnoredFindingListItem } from './securityWorkbenchTypes'
import { formatWorkbenchTime } from './securityWorkbenchPresentation'
import { wb } from './securityWorkbenchLocale'

const props = defineProps<{
  findings: WorkbenchIgnoredFindingListItem[]
  total: number
  page: number
  pageSize: number
  loading: boolean
}>()

const emit = defineEmits<{
  'change-page': [page: number]
  'change-page-size': [pageSize: number]
  'restore-finding': [findingId: string]
  'open-finding': [findingId: string]
}>()

const totalPages = computed(() => Math.max(1, Math.ceil(props.total / props.pageSize)))

const getSeverityBadgeClass = (severity: string) => {
  switch (severity) {
    case 'critical':
      return 'badge-error'
    case 'high':
      return 'badge-warning'
    case 'medium':
      return 'badge-info'
    case 'low':
      return 'badge-ghost'
    default:
      return 'badge-outline'
  }
}

const handlePageSizeChange = (event: Event) => {
  const target = event.target as HTMLSelectElement | null
  const nextValue = Number(target?.value || props.pageSize)
  if (Number.isFinite(nextValue) && nextValue > 0) {
    emit('change-page-size', nextValue)
  }
}
</script>
