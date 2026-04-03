<template>
  <div
    ref="scrollContainerEl"
    class="flex-1 min-h-0 overflow-auto bg-base-100 p-2 rounded-lg"
    @scroll="handleScroll"
  >
    <div class="relative" :style="{ height: `${totalHeight}px` }">
      <div
        v-for="item in visibleRows"
        :key="item.index"
        class="byte-row"
        :style="{ transform: `translateY(${item.offset}px)` }"
      >
        {{ item.text }}
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'

const props = defineProps<{
  rawData: number[]
  mode: 'hex' | 'ascii' | 'raw'
}>()

const scrollContainerEl = ref<HTMLElement | null>(null)
const scrollTop = ref(0)
const containerHeight = ref(0)

const ROW_HEIGHT = 18
const BUFFER_ROWS = 8

const bytesPerRow = computed(() => {
  if (props.mode === 'hex') return 16
  if (props.mode === 'ascii') return 64
  return 32
})

const totalRows = computed(() => {
  if (props.rawData.length === 0) return 1
  return Math.ceil(props.rawData.length / bytesPerRow.value)
})

const totalHeight = computed(() => totalRows.value * ROW_HEIGHT)

const visibleRows = computed(() => {
  const visibleCount = Math.max(1, Math.ceil(containerHeight.value / ROW_HEIGHT))
  const start = Math.max(0, Math.floor(scrollTop.value / ROW_HEIGHT) - BUFFER_ROWS)
  const end = Math.min(totalRows.value, start + visibleCount + BUFFER_ROWS * 2)
  const rows: Array<{ index: number; offset: number; text: string }> = []

  for (let index = start; index < end; index += 1) {
    rows.push({
      index,
      offset: index * ROW_HEIGHT,
      text: formatRow(index),
    })
  }

  return rows
})

let resizeObserver: ResizeObserver | null = null

function updateContainerHeight() {
  containerHeight.value = scrollContainerEl.value?.clientHeight ?? 0
}

function handleScroll(event: Event) {
  const target = event.target as HTMLElement | null
  if (!target) return
  scrollTop.value = target.scrollTop
}

function formatRow(index: number): string {
  if (props.rawData.length === 0) {
    return ''
  }

  const start = index * bytesPerRow.value
  const end = Math.min(start + bytesPerRow.value, props.rawData.length)
  const bytes = props.rawData.slice(start, end)

  if (props.mode === 'hex') {
    const hex = bytes.map((byte) => byte.toString(16).padStart(2, '0')).join(' ')
    const ascii = bytes.map(formatAsciiPreviewByte).join('')
    return `${start.toString(16).padStart(8, '0')}  ${hex.padEnd(47)}  ${ascii}`
  }

  if (props.mode === 'ascii') {
    return bytes.map(formatAsciiByte).join('')
  }

  return bytes.map((byte) => byte.toString(16).padStart(2, '0')).join(' ')
}

function formatAsciiPreviewByte(byte: number): string {
  if (byte >= 32 && byte <= 126) {
    return String.fromCharCode(byte)
  }

  return '.'
}

function formatAsciiByte(byte: number): string {
  if (byte >= 32 && byte <= 126) {
    return String.fromCharCode(byte)
  }

  if (byte === 10) return '\\n'
  if (byte === 13) return '\\r'
  if (byte === 9) return '\\t'
  return '.'
}

watch(
  () => [props.rawData, props.mode],
  () => {
    scrollTop.value = 0
    if (scrollContainerEl.value) {
      scrollContainerEl.value.scrollTop = 0
    }
    updateContainerHeight()
  },
)

onMounted(() => {
  updateContainerHeight()
  if (scrollContainerEl.value) {
    resizeObserver = new ResizeObserver(() => {
      updateContainerHeight()
    })
    resizeObserver.observe(scrollContainerEl.value)
  }
})

onUnmounted(() => {
  if (resizeObserver) {
    resizeObserver.disconnect()
    resizeObserver = null
  }
})
</script>

<style scoped>
.byte-row {
  position: absolute;
  left: 0;
  right: 0;
  height: 18px;
  line-height: 18px;
  white-space: pre;
}
</style>
