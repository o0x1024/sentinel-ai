<template>
  <div class="rounded-xl border border-base-300 bg-base-200/40 p-3 space-y-3">
    <div class="flex items-center justify-between">
      <h4 class="font-semibold">{{ title }}</h4>
      <slot name="actions" />
    </div>

    <div v-if="items.length > 0" class="space-y-3">
      <div class="flex flex-col gap-2 lg:flex-row lg:items-center">
        <label class="input input-bordered input-sm flex items-center gap-2 lg:flex-1">
          <i class="fas fa-search text-xs text-base-content/50"></i>
          <input
            v-model.trim="searchQuery"
            type="text"
            class="grow"
            placeholder="搜索名称、描述、模型、模式或标签"
          />
        </label>

        <label class="form-control lg:w-44">
          <select v-model="sortKey" class="select select-bordered select-sm">
            <option
              v-for="option in sortOptions"
              :key="option.key"
              :value="option.key"
            >
              {{ option.label }}
            </option>
          </select>
        </label>
      </div>

      <div v-if="filterOptions.length > 0" class="flex flex-wrap gap-2">
        <button
          class="btn btn-xs"
          :class="activeFilterKey === 'all' ? 'btn-primary' : 'btn-ghost'"
          @click="activeFilterKey = 'all'"
        >
          全部
        </button>
        <button
          v-for="option in filterOptions"
          :key="option.key"
          class="btn btn-xs"
          :class="activeFilterKey === option.key ? 'btn-primary' : 'btn-ghost'"
          @click="activeFilterKey = option.key"
        >
          {{ option.label }}
        </button>
      </div>

      <div class="text-xs text-base-content/50">
        {{ `显示 ${displayItems.length} / ${items.length}` }}
      </div>
    </div>

    <div v-if="loading" class="p-6 text-center">
      <span class="loading loading-spinner loading-md"></span>
    </div>

    <div v-else-if="items.length === 0" class="p-6 text-sm text-base-content/60">
      {{ emptyText }}
    </div>

    <div v-else-if="displayItems.length === 0" class="p-6 text-sm text-base-content/60">
      当前筛选条件下没有匹配的 Agent。
    </div>

    <div v-else class="space-y-2">
      <button
        v-for="item in displayItems"
        :key="item.id"
        class="w-full rounded-lg border px-3 py-3 text-left transition"
        :class="
          selectedId === item.id
            ? 'border-primary bg-primary/10'
            : 'border-base-300 bg-base-100 hover:border-primary/40'
        "
        @click="$emit('select', item.id)"
      >
        <div class="flex items-center justify-between gap-2">
          <p class="font-medium truncate">{{ item.title }}</p>
          <div v-if="item.badges.length" class="flex flex-wrap items-center justify-end gap-1.5">
            <span
              v-for="badge in item.badges"
              :key="`${item.id}-${badge.label}`"
              class="badge badge-sm"
              :class="badge.className"
            >
              {{ badge.label }}
            </span>
          </div>
        </div>

        <p v-if="item.description" class="mt-2 text-xs text-base-content/60 line-clamp-2">
          {{ item.description }}
        </p>
        <p v-if="item.metaLine" class="mt-1 text-[11px] text-base-content/50 truncate">
          {{ item.metaLine }}
        </p>
        <p class="mt-1 text-xs text-base-content/60 truncate">{{ item.id }}</p>
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'

import {
  DEFAULT_AGENT_LIST_SORT_OPTIONS,
  type AgentListFilterOption,
  type AgentListItemViewModel,
} from './agentListItemSupport'

const props = withDefaults(defineProps<{
  title: string
  emptyText: string
  loading?: boolean
  items: AgentListItemViewModel[]
  selectedId: string
  filterOptions?: AgentListFilterOption[]
}>(), {
  filterOptions: () => [],
})

defineEmits<{
  select: [id: string]
}>()

const searchQuery = ref('')
const activeFilterKey = ref('all')
const sortKey = ref<(typeof DEFAULT_AGENT_LIST_SORT_OPTIONS)[number]['key']>('default')

const filterOptions = computed(() => props.filterOptions || [])
const sortOptions = DEFAULT_AGENT_LIST_SORT_OPTIONS

const displayItems = computed(() => {
  let items = [...props.items]
  const query = searchQuery.value.trim().toLowerCase()

  if (activeFilterKey.value !== 'all') {
    items = items.filter(item => (item.filterKeys || []).includes(activeFilterKey.value))
  }

  if (query) {
    items = items.filter(item => {
      const haystack = [
        item.id,
        item.title,
        item.description,
        item.metaLine || '',
        item.searchText || '',
        ...(item.badges || []).map(badge => badge.label),
      ]
        .join(' ')
        .toLowerCase()
      return haystack.includes(query)
    })
  }

  if (sortKey.value === 'title-asc') {
    items.sort((left, right) => left.title.localeCompare(right.title))
  } else if (sortKey.value === 'title-desc') {
    items.sort((left, right) => right.title.localeCompare(left.title))
  }

  return items
})
</script>
