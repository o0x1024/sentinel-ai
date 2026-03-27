<template>
  <div class="flex h-full min-h-0 flex-col bg-base-100">
    <div class="flex items-center gap-2 border-b border-base-300 bg-base-200 px-2 py-1">
      <div class="flex min-w-0 flex-1 items-center gap-1 overflow-x-auto">
        <div
          v-for="item in items"
          :key="item.id"
          class="flex items-center gap-2 rounded border border-base-300 px-3 py-1.5 text-sm"
          :class="activeItemId === item.id ? 'bg-base-100 border-primary' : 'bg-base-200 hover:bg-base-300'"
        >
          <button class="truncate" type="button" :title="item.name" @click="activeItemId = item.id">
            {{ item.name }}
          </button>
          <button class="btn btn-ghost btn-xs btn-circle" type="button" @click="closeItem(item.id)">
            <i class="fas fa-times text-[10px]"></i>
          </button>
        </div>
      </div>
    </div>

    <div v-if="currentItem" class="flex min-h-0 flex-1 flex-col">
      <div class="flex flex-wrap items-center gap-2 border-b border-base-300 px-4 py-3 text-sm text-base-content/70">
        <span class="badge badge-outline">{{ currentItem.leftLabel }}: {{ currentItem.leftText.length }}</span>
        <span class="badge badge-outline">{{ currentItem.rightLabel }}: {{ currentItem.rightText.length }}</span>
        <span class="badge badge-outline">{{ $t('trafficAnalysis.comparer.labels.changedLines') }}: {{ diffSummary.changedLines }}</span>
        <span class="badge badge-outline">{{ $t('trafficAnalysis.comparer.labels.similarity') }}: {{ diffSummary.similarity }}%</span>
      </div>

      <div class="min-h-0 flex-1 p-4">
        <CodeDiffViewer :left-text="currentItem.leftText" :right-text="currentItem.rightText" />
      </div>
    </div>

    <div v-else class="flex flex-1 items-center justify-center text-sm text-base-content/60">
      {{ $t('trafficAnalysis.comparer.empty.noItems') }}
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'
import CodeDiffViewer from '@/components/traffic/CodeDiffViewer.vue'
import type { TrafficComparePayload } from './transfers'

interface CompareItem extends TrafficComparePayload {
  id: string
}

const items = ref<CompareItem[]>([])
const activeItemId = ref<string | null>(null)

const currentItem = computed(() => items.value.find((item) => item.id === activeItemId.value) ?? null)
const diffSummary = computed(() => {
  if (!currentItem.value) {
    return { changedLines: 0, similarity: 100 }
  }

  const leftLines = currentItem.value.leftText.split(/\r\n|\r|\n/)
  const rightLines = currentItem.value.rightText.split(/\r\n|\r|\n/)
  const maxLines = Math.max(leftLines.length, rightLines.length)
  let changedLines = 0

  for (let index = 0; index < maxLines; index += 1) {
    if ((leftLines[index] ?? '') !== (rightLines[index] ?? '')) {
      changedLines += 1
    }
  }

  const similarityBase = Math.max(maxLines, 1)
  const similarity = Math.max(0, Math.round(((similarityBase - changedLines) / similarityBase) * 100))
  return { changedLines, similarity }
})

function addComparison(payload: TrafficComparePayload) {
  const item: CompareItem = {
    id: `compare-${Date.now()}-${Math.random().toString(36).slice(2, 10)}`,
    ...payload,
  }
  items.value.push(item)
  activeItemId.value = item.id
}

function closeItem(itemId: string) {
  items.value = items.value.filter((item) => item.id !== itemId)
  if (activeItemId.value === itemId) {
    activeItemId.value = items.value[0]?.id ?? null
  }
}

defineExpose({
  addComparison,
})
</script>
