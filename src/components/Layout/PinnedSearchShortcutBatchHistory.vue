<template>
  <div v-if="entries.length > 0" class="space-y-2 rounded-2xl border border-base-300 bg-base-100 px-3 py-3">
    <div class="flex items-center justify-between gap-3">
      <div class="text-xs font-medium text-base-content/55">最近批量操作</div>
      <button class="btn btn-ghost btn-xs" @click="$emit('clear')">清空</button>
    </div>

    <label class="input input-sm input-bordered flex items-center gap-2">
      <i class="fas fa-search text-base-content/40"></i>
      <input
        v-model="searchQuery"
        type="text"
        class="grow"
        placeholder="搜索操作标题、摘要或影响对象"
      />
    </label>

    <div v-if="filterOptions.length > 1" class="flex flex-wrap gap-2">
      <button
        v-for="option in filterOptions"
        :key="option.value"
        class="badge border transition-colors"
        :class="selectedFilter === option.value
          ? 'border-primary bg-primary/10 text-primary'
          : 'border-base-300 bg-base-100 text-base-content/60 hover:border-primary/40 hover:text-primary'"
        @click="selectedFilter = option.value"
      >
        {{ option.label }} {{ option.count }}
      </button>
    </div>

    <div v-if="groupedEntries.length > 0" class="space-y-3">
      <div
        v-for="group in groupedEntries"
        :key="group.key"
        class="space-y-2"
      >
        <div class="px-1 text-[11px] font-semibold uppercase tracking-wider text-base-content/40">
          {{ group.label }}
        </div>
        <div
          v-for="entry in group.entries"
          :key="entry.id"
          class="rounded-2xl border border-base-300 bg-base-50 px-3 py-2"
        >
          <div class="flex items-center justify-between gap-3">
            <div class="min-w-0">
              <div class="flex items-center gap-2">
                <div class="truncate text-sm font-medium">{{ entry.title }}</div>
                <span class="badge badge-ghost badge-xs">{{ getOperationLabel(entry.operationType) }}</span>
              </div>
              <div class="mt-1 text-xs text-base-content/55">
                {{ entry.summary }} · {{ formatHistoryTime(entry.createdAt, group.key) }}
              </div>
            </div>
            <div class="flex items-center gap-1">
              <button
                v-if="(entry.targetLabels || []).length > 0"
                class="btn btn-ghost btn-xs"
                @click="toggleExpandedEntry(entry.id)"
              >
                {{ expandedEntryIds.includes(entry.id) ? '收起对象' : '查看对象' }}
              </button>
              <button class="btn btn-ghost btn-xs" @click="$emit('undo', entry.id)">撤销</button>
            </div>
          </div>
          <div v-if="expandedEntryIds.includes(entry.id) && (entry.targetLabels || []).length > 0" class="mt-3 flex flex-wrap gap-2">
            <span
              v-for="label in entry.targetLabels"
              :key="`${entry.id}-${label}`"
              class="badge badge-outline badge-sm"
            >
              {{ label }}
            </span>
          </div>
        </div>
      </div>
    </div>

    <div
      v-else
      class="rounded-2xl border border-dashed border-base-300 px-3 py-6 text-center text-xs text-base-content/50"
    >
      当前筛选下没有批量操作记录。
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import type {
  PinnedSearchShortcutBatchHistoryEntry,
  PinnedSearchShortcutBatchHistoryOperationType,
} from '@/services/pinnedSearchShortcutBatchHistory'
import {
  collectPinnedSearchShortcutBatchHistoryFilterOptions,
  filterPinnedSearchShortcutBatchHistoryEntries,
  groupPinnedSearchShortcutBatchHistoryEntries,
  searchPinnedSearchShortcutBatchHistoryEntries,
  type PinnedSearchShortcutBatchHistoryFilter,
} from '@/services/pinnedSearchShortcutBatchHistoryPresentation'

const props = defineProps<{
  entries: PinnedSearchShortcutBatchHistoryEntry[]
}>()

defineEmits<{
  undo: [entryId: string]
  clear: []
}>()

const selectedFilter = ref<PinnedSearchShortcutBatchHistoryFilter>('all')
const searchQuery = ref('')
const expandedEntryIds = ref<string[]>([])
const filterOptions = computed(() => collectPinnedSearchShortcutBatchHistoryFilterOptions(props.entries))
const filteredEntries = computed(() => {
  const entriesByFilter = filterPinnedSearchShortcutBatchHistoryEntries(props.entries, selectedFilter.value)
  return searchPinnedSearchShortcutBatchHistoryEntries(entriesByFilter, searchQuery.value)
})
const toggleExpandedEntry = (entryId: string) => {
  expandedEntryIds.value = expandedEntryIds.value.includes(entryId)
    ? expandedEntryIds.value.filter(id => id !== entryId)
    : [...expandedEntryIds.value, entryId]
}

watch(filteredEntries, (entries) => {
  const entryIdSet = new Set(entries.map(entry => entry.id))
  expandedEntryIds.value = expandedEntryIds.value.filter(entryId => entryIdSet.has(entryId))
})

watch(filterOptions, (nextOptions) => {
  if (nextOptions.some(option => option.value === selectedFilter.value)) {
    return
  }

  selectedFilter.value = 'all'
}, { immediate: true })

watch(searchQuery, () => {
  expandedEntryIds.value = []
})

const groupedEntries = computed(() => groupPinnedSearchShortcutBatchHistoryEntries(filteredEntries.value))

const formatHistoryTime = (value: number, groupKey: 'today' | 'earlier') => {
  try {
    return groupKey === 'today'
      ? new Intl.DateTimeFormat('zh-CN', {
          hour: '2-digit',
          minute: '2-digit',
          second: '2-digit',
          hour12: false,
        }).format(value)
      : new Intl.DateTimeFormat('zh-CN', {
          month: '2-digit',
          day: '2-digit',
          hour: '2-digit',
          minute: '2-digit',
          hour12: false,
        }).format(value)
  } catch {
    return ''
  }
}

const getOperationLabel = (operationType: PinnedSearchShortcutBatchHistoryOperationType) => {
  switch (operationType) {
    case 'remove':
      return '取消固定'
    case 'tag-add':
      return '追加标签'
    case 'tag-remove':
      return '移除标签'
  }
}
</script>
