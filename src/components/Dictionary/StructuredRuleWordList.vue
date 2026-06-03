<template>
  <div class="border rounded-lg">
    <div class="sticky top-0 bg-base-200 px-4 py-2 items-center text-sm font-medium border-b dict-grid-structured">
      <div>
        <input
          type="checkbox"
          class="checkbox"
          :checked="selectedWords.length === items.length && items.length > 0"
          @change="toggleSelectAll"
        >
      </div>
      <div>词条</div>
      <div>添加时间</div>
      <div class="text-right pr-2">操作</div>
    </div>

    <VirtualList
      :items="items"
      :itemHeight="itemHeight"
      :height="height"
      class="virtual-list-host"
      keyField="id"
      @scroll="emit('scroll', $event)"
    >
      <template #default="{ item }">
        <div class="px-4 items-center text-sm h-full w-full dict-grid-structured">
          <div class="py-2">
            <input
              v-model="selectedModel"
              type="checkbox"
              class="checkbox"
              :value="item.id"
            >
          </div>
          <div class="min-w-0 py-2">
            <div class="font-medium truncate" :title="item.word">{{ item.word }}</div>
            <div class="text-xs text-base-content/70 mt-1 line-clamp-2">
              {{ describeRuleEntrySummary(item, dictionaryType) }}
            </div>
            <div
              v-if="item.category || item.weight != null || !isRuleEnabled(item) || getRuleSeverity(item) || getRuleMatcherCount(item) > 0"
              class="flex gap-2 mt-2"
            >
              <span v-if="item.category" class="badge badge-outline badge-xs">{{ item.category }}</span>
              <span v-if="!isRuleEnabled(item)" class="badge badge-error badge-xs">disabled</span>
              <span v-if="getRuleSeverity(item)" class="badge badge-warning badge-xs">{{ getRuleSeverity(item) }}</span>
              <span v-if="getRuleMatcherCount(item) > 0" class="badge badge-info badge-xs">{{ getRuleMatcherCount(item) }} matcher</span>
              <span v-if="item.weight != null" class="badge badge-ghost badge-xs">w={{ item.weight }}</span>
            </div>
          </div>
          <div class="py-2 whitespace-nowrap text-base-content/80">{{ formatDate(item.created_at) }}</div>
          <div class="py-2 text-right pr-2 whitespace-nowrap">
            <button class="btn btn-ghost btn-xs" @click="emit('edit', item)">
              <i class="fas fa-edit"></i>
            </button>
            <button class="btn btn-ghost btn-xs text-error" @click="emit('remove', item.id)">
              <i class="fas fa-trash"></i>
            </button>
          </div>
        </div>
      </template>
    </VirtualList>

    <div class="px-4 py-2 text-center text-sm opacity-70">
      <span v-if="isLoadingMore">加载中...</span>
      <span v-else-if="!hasMore">没有更多了</span>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import VirtualList from '@/components/VirtualList.vue'
import {
  describeRuleEntry as describeRuleEntrySummary,
  getRuleMatcherCount,
  getRuleSeverity,
  isRuleEnabled,
} from '@/components/Dictionary/ruleEntryUtils'

interface DictionaryWord {
  id: string
  word: string
  weight?: number | null
  category?: string | null
  metadata?: string | null
  created_at: string
}

const props = defineProps<{
  items: DictionaryWord[]
  selectedWords: string[]
  dictionaryType: string
  itemHeight: number
  height: number
  isLoadingMore: boolean
  hasMore: boolean
}>()

const emit = defineEmits<{
  (e: 'update:selectedWords', value: string[]): void
  (e: 'edit', value: DictionaryWord): void
  (e: 'remove', value: string): void
  (e: 'scroll', value: { scrollTop: number; clientHeight: number; scrollHeight: number }): void
}>()

const selectedModel = computed({
  get: () => props.selectedWords,
  set: (value: string[]) => emit('update:selectedWords', value),
})

function toggleSelectAll() {
  if (props.selectedWords.length === props.items.length) {
    emit('update:selectedWords', [])
    return
  }
  emit('update:selectedWords', props.items.map(item => item.id))
}

function formatDate(dateString: string) {
  if (!dateString) return '-'
  return new Date(dateString).toLocaleDateString()
}
</script>

<style scoped>
.sticky {
  position: sticky;
  z-index: 10;
}

.dict-grid-structured {
  display: grid;
  grid-template-columns: 3rem minmax(0, 1fr) 10rem 6rem;
  column-gap: 1rem;
}

.virtual-list-host :deep(.virtual-list-item) {
  border-bottom: 1px solid var(--fallback-b3, oklch(var(--b3)));
  width: 100%;
}

.virtual-list-host :deep(.virtual-list-container) {
  scrollbar-gutter: stable both-edges;
}
</style>
