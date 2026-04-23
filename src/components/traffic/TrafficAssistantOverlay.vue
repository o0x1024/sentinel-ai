<template>
  <div ref="overlayRootRef" class="pointer-events-none absolute inset-0 z-[34]">
    <div
      v-if="isOpen && isImmersive"
      class="pointer-events-auto absolute inset-0 bg-slate-950/18 backdrop-blur-[2px]"
      @click="setTrafficAssistantDisplayMode('panel')"
    ></div>

    <section
      class="pointer-events-auto absolute flex flex-col overflow-hidden border border-base-300/80 bg-base-100 shadow-[0_32px_90px_rgba(15,23,42,0.2)] transition-all duration-300 ease-out"
      :class="[workspaceClasses, { 'assistant-panel-resizing': isPanelResizing }]"
      :style="workspaceStyle"
    >
      <div
        v-if="showPanelResizeHandle"
        class="assistant-panel-resizer absolute bottom-0 left-0 top-0 z-[2]"
        :title="t('trafficAnalysis.aiWorkspace.resize')"
        @mousedown="startPanelResize"
      ></div>

      <div class="border-b border-base-300/70 bg-base-200/85 px-4 py-3 backdrop-blur-sm">
        <div class="flex items-start justify-between gap-3">
          <div class="min-w-0 flex-1">
            <div class="flex flex-wrap items-center gap-2">
              <span class="rounded-full bg-primary/12 px-2.5 py-1 text-[11px] font-semibold uppercase tracking-[0.22em] text-primary">
                {{ t('trafficAnalysis.aiWorkspace.eyebrow') }}
              </span>
              <span class="badge badge-sm badge-outline">{{ modeLabel }}</span>
            </div>
            <h3 class="mt-2 text-lg font-semibold text-base-content">
              {{ t('trafficAnalysis.aiWorkspace.title') }}
            </h3>
            <p class="mt-1 text-sm text-base-content/65">
              {{ isImmersive
                ? t('trafficAnalysis.aiWorkspace.immersiveDescription')
                : t('trafficAnalysis.aiWorkspace.panelDescription') }}
            </p>
          </div>

          <div class="flex items-center gap-2">
            <button
              type="button"
              class="btn btn-sm btn-outline rounded-2xl"
              :aria-label="t('common.minimize', '最小化')"
              @click="minimizeTrafficAssistant"
            >
              <i class="fas fa-window-minimize"></i>
            </button>
            <button
              type="button"
              class="btn btn-sm btn-outline rounded-2xl"
              @click="toggleMode"
            >
              <i :class="isImmersive ? 'fas fa-compress-alt' : 'fas fa-expand-alt'"></i>
              {{ isImmersive
                ? t('trafficAnalysis.aiWorkspace.collapse')
                : t('trafficAnalysis.aiWorkspace.expand') }}
            </button>
            <button
              type="button"
              class="btn btn-sm btn-ghost rounded-2xl"
              :aria-label="t('trafficAnalysis.aiWorkspace.close')"
              @click="closeTrafficAssistant"
            >
              <i class="fas fa-times"></i>
            </button>
          </div>
        </div>
      </div>

      <AIAssistantWorkspace
        presentation-target="traffic"
        :active="isOpen"
        class="min-h-0 flex-1"
      />
    </section>
  </div>
</template>

<script setup lang="ts">
import {
  computed,
  onActivated,
  onDeactivated,
  onMounted,
  onUnmounted,
  ref,
  watch,
} from 'vue'
import { useI18n } from 'vue-i18n'
import AIAssistantWorkspace from '@/components/assistant/AIAssistantWorkspace.vue'
import {
  claimAssistantPresentationTarget,
  releaseAssistantPresentationTarget,
} from '@/services/assistantPresentation'
import { immersiveDrillModeEnabled } from '@/services/immersiveDrillMode'
import { closeTopmostImmersiveTool } from '@/services/immersiveToolCoordinator'
import {
  closeTrafficAssistant,
  minimizeTrafficAssistant,
  openTrafficAssistantImmersive,
  setTrafficAssistantDisplayMode,
  trafficAssistantDisplayMode,
  trafficAssistantVisible,
} from '@/services/trafficAssistantWorkspace'
import {
  clampTrafficAssistantPanelWidth,
  persistTrafficAssistantPanelWidth,
  readTrafficAssistantPanelWidth,
  shouldUseTrafficAssistantDesktopPanel,
} from './trafficAssistantOverlaySizing'

const { t } = useI18n()

const overlayRootRef = ref<HTMLElement | null>(null)
const panelWidth = ref(readTrafficAssistantPanelWidth())
const overlayWidth = ref(0)
const panelResizeState = ref<{
  startX: number
  startWidth: number
} | null>(null)
const panelResizeRequestedWidth = ref<number | null>(null)

const isOpen = computed(() => trafficAssistantVisible.value)
const isImmersive = computed(() => trafficAssistantDisplayMode.value === 'immersive')
const isPanelResizing = computed(() => panelResizeState.value !== null)
const useDesktopPanel = computed(() => shouldUseTrafficAssistantDesktopPanel(overlayWidth.value))
const effectivePanelWidth = computed(() =>
  clampTrafficAssistantPanelWidth(panelWidth.value, overlayWidth.value),
)
const showPanelResizeHandle = computed(() =>
  isOpen.value && !isImmersive.value && useDesktopPanel.value,
)
const modeLabel = computed(() =>
  isImmersive.value
    ? t('trafficAnalysis.aiWorkspace.immersiveBadge')
    : t('trafficAnalysis.aiWorkspace.panelBadge'),
)
const workspaceClasses = computed(() =>
  isOpen.value
    ? isImmersive.value
      ? 'inset-4 flex rounded-[30px]'
      : 'bottom-4 left-4 right-4 top-4 flex rounded-[28px] lg:left-auto'
    : 'bottom-6 left-6 right-auto top-auto flex h-0 w-0 overflow-hidden rounded-[28px] opacity-0',
)
const workspaceStyle = computed(() => {
  if (!isOpen.value || isImmersive.value || !useDesktopPanel.value) {
    return {}
  }

  return {
    width: `${effectivePanelWidth.value}px`,
  }
})

let resizeObserver: ResizeObserver | null = null
let panelResizeFrameId = 0

function updateOverlayWidth() {
  overlayWidth.value = overlayRootRef.value?.offsetWidth ?? window.innerWidth
}

function applyPanelWidth(width: number) {
  panelWidth.value = clampTrafficAssistantPanelWidth(width, overlayWidth.value)
}

function flushRequestedPanelWidth() {
  panelResizeFrameId = 0

  if (panelResizeRequestedWidth.value === null) {
    return
  }

  applyPanelWidth(panelResizeRequestedWidth.value)
  panelResizeRequestedWidth.value = null
}

function schedulePanelWidth(width: number) {
  panelResizeRequestedWidth.value = width
  if (panelResizeFrameId !== 0) {
    return
  }

  panelResizeFrameId = requestAnimationFrame(flushRequestedPanelWidth)
}

function handlePanelResize(event: MouseEvent) {
  const state = panelResizeState.value
  if (!state) {
    return
  }

  const diffX = event.clientX - state.startX
  schedulePanelWidth(state.startWidth - diffX)
}

function stopPanelResize() {
  if (!panelResizeState.value) {
    return
  }

  flushRequestedPanelWidth()
  panelResizeState.value = null
  document.body.style.cursor = ''
  document.body.style.userSelect = ''
  window.removeEventListener('mousemove', handlePanelResize)
  window.removeEventListener('mouseup', stopPanelResize)
  persistTrafficAssistantPanelWidth(panelWidth.value)
}

function startPanelResize(event: MouseEvent) {
  if (!showPanelResizeHandle.value) {
    return
  }

  panelResizeState.value = {
    startX: event.clientX,
    startWidth: effectivePanelWidth.value,
  }
  document.body.style.cursor = 'col-resize'
  document.body.style.userSelect = 'none'
  window.addEventListener('mousemove', handlePanelResize)
  window.addEventListener('mouseup', stopPanelResize)
  event.preventDefault()
}

function claimTrafficAssistant() {
  if (isOpen.value) {
    claimAssistantPresentationTarget('traffic')
  }
}

function releaseTrafficAssistant() {
  releaseAssistantPresentationTarget('traffic')
}

function toggleMode() {
  if (isImmersive.value) {
    setTrafficAssistantDisplayMode('panel')
    return
  }

  openTrafficAssistantImmersive()
}

function handleWindowKeydown(event: KeyboardEvent) {
  if (!isOpen.value) {
    return
  }

  if (event.defaultPrevented || event.isComposing || event.repeat) {
    return
  }

  if (event.key !== 'Escape') {
    return
  }

  if (immersiveDrillModeEnabled.value) {
    if (!closeTopmostImmersiveTool()) {
      return
    }

    event.preventDefault()
    event.stopPropagation()
    return
  }

  event.preventDefault()
  event.stopPropagation()
  closeTrafficAssistant()
}

watch(isOpen, (open) => {
  if (open) {
    claimTrafficAssistant()
    updateOverlayWidth()
    return
  }

  releaseTrafficAssistant()
})

watch(overlayWidth, () => {
  applyPanelWidth(panelWidth.value)
})

watch(useDesktopPanel, (enabled) => {
  if (enabled) {
    applyPanelWidth(panelWidth.value)
    return
  }

  stopPanelResize()
})

onMounted(() => {
  updateOverlayWidth()

  if (typeof ResizeObserver !== 'undefined' && overlayRootRef.value) {
    resizeObserver = new ResizeObserver(() => {
      updateOverlayWidth()
    })
    resizeObserver.observe(overlayRootRef.value)
  }

  window.addEventListener('resize', updateOverlayWidth)
  window.addEventListener('keydown', handleWindowKeydown)
})
onActivated(claimTrafficAssistant)
onDeactivated(() => {
  releaseTrafficAssistant()
  stopPanelResize()
})
onUnmounted(() => {
  releaseTrafficAssistant()
  stopPanelResize()
  if (panelResizeFrameId !== 0) {
    cancelAnimationFrame(panelResizeFrameId)
    panelResizeFrameId = 0
  }
  resizeObserver?.disconnect()
  resizeObserver = null
  window.removeEventListener('resize', updateOverlayWidth)
  window.removeEventListener('keydown', handleWindowKeydown)
})
</script>

<style scoped>
.assistant-panel-resizer {
  width: 10px;
  cursor: col-resize;
  background: linear-gradient(180deg, transparent 0%, rgb(148 163 184 / 0.12) 50%, transparent 100%);
  transition: background-color 160ms ease, opacity 160ms ease;
  opacity: 0.45;
}

.assistant-panel-resizer:hover {
  opacity: 1;
  background: linear-gradient(180deg, transparent 0%, rgb(59 130 246 / 0.3) 50%, transparent 100%);
}

.assistant-panel-resizing {
  transition: none !important;
  will-change: width;
}
</style>
