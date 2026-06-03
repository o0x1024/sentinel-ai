<template>
  <div
    v-if="visible"
    class="absolute top-full left-0 right-0 mt-2 overflow-hidden rounded-2xl border border-base-300 bg-base-100 shadow-2xl"
  >
    <div v-if="recentSearches.length > 0" class="border-b border-base-300/70 px-3 py-2">
      <div class="mb-2 flex items-center justify-between gap-3">
        <div class="text-[11px] font-semibold uppercase tracking-wider text-base-content/45">
          最近搜索
        </div>
        <button
          class="text-[11px] text-base-content/45 transition-colors hover:text-primary"
          @mousedown.prevent="$emit('clear-recent-searches')"
        >
          清空
        </button>
      </div>
      <div class="flex flex-wrap gap-2">
        <button
          v-for="item in recentSearches"
          :key="item"
          class="badge badge-outline badge-sm hover:border-primary hover:text-primary"
          @mousedown.prevent="$emit('recent-search', item)"
        >
          {{ item }}
        </button>
      </div>
    </div>

    <div v-if="results.length > 0" class="max-h-96 overflow-y-auto p-2">
      <div
        v-for="group in groupedResults"
        :key="group.category"
        class="mb-2 last:mb-0"
      >
        <div class="px-3 pb-1 pt-2 text-[11px] font-semibold uppercase tracking-wider text-base-content/45">
          {{ group.label }}
        </div>
        <button
          v-for="item in group.items"
          :key="item.result.id"
          class="flex w-full items-start gap-3 rounded-xl px-3 py-2 text-left transition-colors"
          :class="item.index === highlightedIndex ? 'bg-primary/10 text-primary' : 'hover:bg-base-200'"
          @mousedown.prevent="$emit('select', item.result)"
          @mouseenter="$emit('highlight', item.index)"
        >
          <div
            class="mt-0.5 flex h-9 w-9 shrink-0 items-center justify-center rounded-xl"
            :class="item.index === highlightedIndex ? 'bg-primary/15 text-primary' : 'bg-base-200 text-base-content/70'"
          >
            <i :class="item.result.icon"></i>
          </div>
          <div class="min-w-0 flex-1">
            <div class="flex items-center gap-2">
              <span class="truncate font-medium">
                <template v-for="(part, partIndex) in getHighlightedParts(item.result.title)" :key="`${item.result.id}-title-${partIndex}`">
                  <mark v-if="part.matched" class="rounded bg-warning/30 px-0.5 text-inherit">{{ part.text }}</mark>
                  <span v-else>{{ part.text }}</span>
                </template>
              </span>
              <span class="badge badge-ghost badge-xs">
                {{ getSearchCategoryLabel(item.result.category) }}
              </span>
            </div>
            <p class="mt-1 line-clamp-2 text-xs text-base-content/60">
              <template v-for="(part, partIndex) in getHighlightedParts(item.result.description)" :key="`${item.result.id}-description-${partIndex}`">
                <mark v-if="part.matched" class="rounded bg-warning/25 px-0.5 text-inherit">{{ part.text }}</mark>
                <span v-else>{{ part.text }}</span>
              </template>
            </p>
            <div class="mt-2 text-[11px] font-medium uppercase tracking-wider text-base-content/40">
              {{ getSearchActionLabel(item.result) }}
            </div>
          </div>
        </button>
      </div>
    </div>

    <div v-else-if="query" class="px-4 py-6 text-center text-sm text-base-content/55">
      没有匹配结果，按 Enter 查看完整搜索页
    </div>

    <div v-else class="px-4 py-6 text-center text-sm text-base-content/55">
      输入关键词搜索页面、消息和通知内容
    </div>

    <div class="flex items-center justify-between border-t border-base-300/70 bg-base-200/60 px-3 py-2 text-xs text-base-content/55">
      <span>Enter 打开</span>
      <span>↑ ↓ 选择</span>
      <span>Esc 关闭</span>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { getSearchCategoryLabel, type GlobalSearchResult } from '@/services/globalSearch'
import { getSearchActionLabel, groupSearchResults } from '@/services/searchPresentation'
import { buildHighlightedParts } from '@/utils/searchHighlight'

const props = defineProps<{
  visible: boolean
  query: string
  results: GlobalSearchResult[]
  highlightedIndex: number
  recentSearches: string[]
}>()

defineEmits<{
  select: [result: GlobalSearchResult]
  highlight: [index: number]
  'recent-search': [query: string]
  'clear-recent-searches': []
}>()

const groupedResults = computed(() =>
  groupSearchResults(props.results).map(group => ({
    ...group,
    items: props.results
      .map((result, index) => ({ result, index }))
      .filter(item => item.result.category === group.category),
  })),
)

const getHighlightedParts = (text: string) => buildHighlightedParts(text, props.query)
</script>
