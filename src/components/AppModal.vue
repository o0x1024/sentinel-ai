<template>
  <Teleport to="body">
    <div
      v-if="open"
      :class="mergedRootClass"
      :style="mergedRootStyle"
      @click.self="$emit('close')"
    >
      <div ref="boxRef" :class="mergedBoxClass" :style="mergedBoxStyle">
        <slot />
        <button
          v-if="resizable"
          type="button"
          class="app-modal-resize-handle"
          aria-label="调整对话框大小"
          title="拖拽调整大小"
          @mousedown.stop.prevent="startResize"
        ></button>
      </div>
    </div>
  </Teleport>
</template>

<script setup lang="ts">
import { computed, normalizeClass, onUnmounted, ref, useAttrs, watch } from 'vue'
import type { StyleValue } from 'vue'

defineOptions({ inheritAttrs: false })

const props = withDefaults(defineProps<{
  open: boolean
  boxClass?: string
  topAligned?: boolean
  resizable?: boolean
  resizeStorageKey?: string
  minWidth?: number
  minHeight?: number
}>(), {
  boxClass: '',
  topAligned: false,
  resizable: false,
  resizeStorageKey: '',
  minWidth: 640,
  minHeight: 360,
})

defineEmits<{
  (e: 'close'): void
}>()

const attrs = useAttrs()
const boxRef = ref<HTMLElement | null>(null)
const manualSize = ref<{ width: number; height: number } | null>(null)
const manualSizeRatio = ref<{ widthRatio: number; heightRatio: number } | null>(null)
const resizeState = ref<{
  startX: number
  startY: number
  startWidth: number
  startHeight: number
} | null>(null)

const mergedRootClass = computed(() => {
  const className = normalizeClass(attrs.class)
  return [
    'modal',
    'modal-open',
    props.topAligned ? 'app-modal--top' : '',
    className,
  ]
})

const mergedRootStyle = computed<StyleValue | undefined>(() => attrs.style as StyleValue | undefined)
const mergedBoxClass = computed(() => [
  'modal-box',
  props.boxClass,
  props.resizable ? 'app-modal-box--resizable' : '',
])
const mergedBoxStyle = computed<StyleValue | undefined>(() => {
  if (!manualSize.value) {
    return undefined
  }

  return {
    width: `${manualSize.value.width}px`,
    maxWidth: `${manualSize.value.width}px`,
    height: `${manualSize.value.height}px`,
    maxHeight: `${manualSize.value.height}px`,
  }
})

function clampSize(width: number, height: number) {
  if (typeof window === 'undefined') {
    return { width, height }
  }

  const horizontalMargin = 32
  const verticalMargin = props.topAligned ? 96 : 32
  const maxWidth = Math.max(props.minWidth, window.innerWidth - horizontalMargin)
  const maxHeight = Math.max(props.minHeight, window.innerHeight - verticalMargin)

  return {
    width: Math.min(Math.max(width, props.minWidth), maxWidth),
    height: Math.min(Math.max(height, props.minHeight), maxHeight),
  }
}

function updateManualSizeRatio(size: { width: number; height: number }) {
  if (typeof window === 'undefined' || window.innerWidth <= 0 || window.innerHeight <= 0) {
    return
  }

  manualSizeRatio.value = {
    widthRatio: size.width / window.innerWidth,
    heightRatio: size.height / window.innerHeight,
  }
}

function persistSize() {
  if (!props.resizeStorageKey || !manualSizeRatio.value || typeof window === 'undefined') {
    return
  }

  window.localStorage.setItem(props.resizeStorageKey, JSON.stringify(manualSizeRatio.value))
}

function restoreSize() {
  if (!props.resizeStorageKey || typeof window === 'undefined') {
    return
  }

  const raw = window.localStorage.getItem(props.resizeStorageKey)
  if (!raw) {
    return
  }

  try {
    const parsed = JSON.parse(raw)
    if (Number.isFinite(parsed?.widthRatio) && Number.isFinite(parsed?.heightRatio)) {
      manualSizeRatio.value = {
        widthRatio: parsed.widthRatio,
        heightRatio: parsed.heightRatio,
      }
      manualSize.value = clampSize(
        window.innerWidth * parsed.widthRatio,
        window.innerHeight * parsed.heightRatio,
      )
      return
    }

    if (Number.isFinite(parsed?.width) && Number.isFinite(parsed?.height)) {
      manualSize.value = clampSize(parsed.width, parsed.height)
      updateManualSizeRatio(manualSize.value)
    }
  } catch {
    // Ignore invalid persisted size.
  }
}

function handleResize(event: MouseEvent) {
  if (!resizeState.value) {
    return
  }

  const nextWidth = resizeState.value.startWidth + (event.clientX - resizeState.value.startX)
  const nextHeight = resizeState.value.startHeight + (event.clientY - resizeState.value.startY)
  manualSize.value = clampSize(nextWidth, nextHeight)
  updateManualSizeRatio(manualSize.value)
}

function stopResize() {
  if (!resizeState.value) {
    return
  }

  resizeState.value = null
  window.removeEventListener('mousemove', handleResize)
  window.removeEventListener('mouseup', stopResize)
  document.body.style.userSelect = ''
  persistSize()
}

function startResize(event: MouseEvent) {
  if (!props.resizable || !boxRef.value) {
    return
  }

  const rect = boxRef.value.getBoundingClientRect()
  resizeState.value = {
    startX: event.clientX,
    startY: event.clientY,
    startWidth: rect.width,
    startHeight: rect.height,
  }
  document.body.style.userSelect = 'none'
  window.addEventListener('mousemove', handleResize)
  window.addEventListener('mouseup', stopResize)
}

function handleWindowResize() {
  if (!props.open || !manualSizeRatio.value) {
    return
  }

  manualSize.value = clampSize(
    window.innerWidth * manualSizeRatio.value.widthRatio,
    window.innerHeight * manualSizeRatio.value.heightRatio,
  )
}

watch(
  () => props.open,
  open => {
    if (!props.resizable) {
      return
    }

    if (open) {
      restoreSize()
      window.addEventListener('resize', handleWindowResize)
      return
    }

    stopResize()
    window.removeEventListener('resize', handleWindowResize)
  },
  { immediate: true },
)

onUnmounted(() => {
  stopResize()
  window.removeEventListener('resize', handleWindowResize)
})
</script>

<style>
.app-modal--top {
  align-items: flex-start;
  padding: 5rem 1rem 1.5rem;
}

.modal-box.traffic-proxy-config-modal-box {
  width: min(84vw, 1360px) !important;
  max-width: min(84vw, 1360px) !important;
  height: min(88vh, 960px) !important;
  max-height: min(88vh, 960px) !important;
  padding: 0 !important;
  overflow: hidden;
}

.app-modal-box--resizable {
  position: relative;
}

.app-modal-resize-handle {
  position: absolute;
  right: 0.55rem;
  bottom: 0.55rem;
  z-index: 5;
  height: 1.15rem;
  width: 1.15rem;
  cursor: nwse-resize;
  border: 0;
  border-radius: 999px;
  background:
    linear-gradient(135deg, transparent 0 46%, hsl(var(--b3)) 46% 58%, transparent 58% 100%),
    linear-gradient(135deg, transparent 0 62%, hsl(var(--b3)) 62% 74%, transparent 74% 100%);
  opacity: 0.72;
}

.app-modal-resize-handle:hover {
  opacity: 1;
}
</style>
