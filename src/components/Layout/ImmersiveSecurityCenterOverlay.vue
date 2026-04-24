<template>
  <div v-if="showWorkspace" ref="overlayRootRef" class="pointer-events-none fixed inset-0 z-[75]">
    <section
      class="pointer-events-auto absolute flex overflow-hidden rounded-[32px] border border-base-300/80 bg-base-100/95 shadow-[0_32px_90px_rgba(15,23,42,0.18)] backdrop-blur-xl transition-all duration-300 ease-out"
      :class="[workspaceClasses, { 'security-center-overlay-resizing': isResizing }]"
      :style="workspaceStyle"
    >
      <div
        v-if="showResizeHandle"
        class="security-center-overlay-resizer absolute bottom-0 left-0 top-0 z-[2]"
        :title="t('securityCenter.immersiveSidebar.resize', '调整安全中心宽度')"
        @mousedown="startResize"
      ></div>

      <SecurityCenterImmersiveSidebar
        class="border-r border-base-300/70 bg-base-100/92"
        :active-tab="activeTab"
        @select-tab="handleSelectTab"
      />

      <div class="min-w-0 flex flex-1 flex-col overflow-hidden rounded-[28px] border border-base-300/70 bg-base-100">
        <div class="flex items-start justify-between gap-3 border-b border-base-300/70 bg-base-200/70 px-4 py-3">
          <div>
            <p class="text-[11px] font-semibold uppercase tracking-[0.22em] text-primary/80">
              {{ t('securityCenter.title') }}
            </p>
            <h2 class="mt-1 text-lg font-semibold text-base-content">
              {{ t('securityCenter.immersiveSidebar.title') }}
            </h2>
          </div>
          <div class="flex items-center gap-2">
            <button
              type="button"
              class="btn btn-sm btn-outline rounded-2xl"
              :aria-label="t('securityCenter.immersiveSidebar.minimize', '最小化安全中心')"
              @click="handleMinimize"
            >
              <i class="fas fa-window-minimize"></i>
            </button>
            <button
              type="button"
              class="btn btn-sm btn-ghost rounded-2xl"
              :aria-label="t('common.close', '关闭')"
              @click="handleClose"
            >
              <i class="fas fa-times"></i>
            </button>
          </div>
        </div>

        <SecurityCenter class="min-h-0 flex-1" :immersive-active-tab="activeTab" />
      </div>
    </section>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRoute, useRouter } from 'vue-router'
import SecurityCenterImmersiveSidebar from '@/components/SecurityCenter/SecurityCenterImmersiveSidebar.vue'
import SecurityCenter from '@/views/SecurityCenter.vue'
import {
  clearImmersiveSecurityCenterReturnPath,
  closeImmersiveSecurityCenterSidebar,
  immersiveSecurityCenterReturnPath,
  immersiveSecurityCenterSidebarOpen,
  minimizeImmersiveSecurityCenterSidebar,
} from '@/services/immersiveSecurityCenterSidebar'
import { closeTopmostImmersiveTool } from '@/services/immersiveToolCoordinator'
import {
  clampImmersiveSecurityCenterWidth,
  persistImmersiveSecurityCenterWidth,
  readImmersiveSecurityCenterWidth,
  shouldUseImmersiveSecurityCenterDesktopPanel,
} from './immersiveSecurityCenterOverlaySizing'

defineOptions({
  name: 'ImmersiveSecurityCenterOverlay',
})

const { t } = useI18n()
const route = useRoute()
const router = useRouter()
const overlayRootRef = ref<HTMLElement | null>(null)
const overlayWidth = ref(0)
const panelWidth = ref(readImmersiveSecurityCenterWidth())
const selectedTab = ref<'workbench' | 'vulnerabilities'>('workbench')
const resizeState = ref<{
  startX: number
  startWidth: number
} | null>(null)
const pendingWidth = ref<number | null>(null)

const showWorkspace = computed(() => immersiveSecurityCenterSidebarOpen.value)
const useDesktopPanel = computed(() =>
  shouldUseImmersiveSecurityCenterDesktopPanel(overlayWidth.value),
)
const effectiveWidth = computed(() =>
  clampImmersiveSecurityCenterWidth(panelWidth.value, overlayWidth.value),
)
const isResizing = computed(() => resizeState.value !== null)
const showResizeHandle = computed(() => showWorkspace.value && useDesktopPanel.value)
const workspaceClasses = computed(() =>
  useDesktopPanel.value
    ? 'bottom-4 right-4 top-[calc(var(--app-navbar-height,4rem)+1rem)]'
    : 'bottom-4 left-4 right-4 top-[calc(var(--app-navbar-height,4rem)+1rem)]',
)
const workspaceStyle = computed(() => {
  if (!useDesktopPanel.value) {
    return {}
  }

  return {
    width: `${effectiveWidth.value}px`,
  }
})

const activeTab = computed<'workbench' | 'vulnerabilities'>(() => selectedTab.value)

let resizeObserver: ResizeObserver | null = null
let resizeFrameId = 0

function updateOverlayWidth() {
  overlayWidth.value = overlayRootRef.value?.offsetWidth ?? window.innerWidth
}

function applyPanelWidth(width: number) {
  panelWidth.value = clampImmersiveSecurityCenterWidth(width, overlayWidth.value)
}

function flushPendingWidth() {
  resizeFrameId = 0
  if (pendingWidth.value === null) {
    return
  }

  applyPanelWidth(pendingWidth.value)
  pendingWidth.value = null
}

function scheduleWidth(width: number) {
  pendingWidth.value = width
  if (resizeFrameId !== 0) {
    return
  }

  resizeFrameId = requestAnimationFrame(flushPendingWidth)
}

function handleResize(event: MouseEvent) {
  const state = resizeState.value
  if (!state) {
    return
  }

  const diffX = event.clientX - state.startX
  scheduleWidth(state.startWidth - diffX)
}

function stopResize() {
  if (!resizeState.value) {
    return
  }

  flushPendingWidth()
  resizeState.value = null
  document.body.style.cursor = ''
  document.body.style.userSelect = ''
  window.removeEventListener('mousemove', handleResize)
  window.removeEventListener('mouseup', stopResize)
  persistImmersiveSecurityCenterWidth(panelWidth.value)
}

function startResize(event: MouseEvent) {
  if (!showResizeHandle.value) {
    return
  }

  resizeState.value = {
    startX: event.clientX,
    startWidth: effectiveWidth.value,
  }
  document.body.style.cursor = 'col-resize'
  document.body.style.userSelect = 'none'
  window.addEventListener('mousemove', handleResize)
  window.addEventListener('mouseup', stopResize)
  event.preventDefault()
}

async function handleSelectTab(tab: 'workbench' | 'vulnerabilities') {
  selectedTab.value = tab
}

async function handleClose() {
  const returnPath = immersiveSecurityCenterReturnPath.value
  closeImmersiveSecurityCenterSidebar()
  clearImmersiveSecurityCenterReturnPath()

  if (returnPath && returnPath !== route.fullPath) {
    await router.push(returnPath)
  }
}

function handleMinimize() {
  stopResize()
  minimizeImmersiveSecurityCenterSidebar()
}

function handleWindowKeydown(event: KeyboardEvent) {
  if (!showWorkspace.value) {
    return
  }

  if (event.defaultPrevented || event.isComposing || event.repeat) {
    return
  }

  if (event.key !== 'Escape') {
    return
  }

  if (!closeTopmostImmersiveTool()) {
    return
  }

  event.preventDefault()
  event.stopPropagation()
}

watch(showWorkspace, open => {
  if (open) {
    updateOverlayWidth()
    selectedTab.value = resolveTabFromRoute()
  } else {
    stopResize()
  }
})

watch(overlayWidth, () => {
  applyPanelWidth(panelWidth.value)
})

watch(useDesktopPanel, enabled => {
  if (enabled) {
    applyPanelWidth(panelWidth.value)
    return
  }

  stopResize()
})

onMounted(() => {
  updateOverlayWidth()
  selectedTab.value = resolveTabFromRoute()

  if (typeof ResizeObserver !== 'undefined' && overlayRootRef.value) {
    resizeObserver = new ResizeObserver(() => {
      updateOverlayWidth()
    })
    resizeObserver.observe(overlayRootRef.value)
  }

  window.addEventListener('resize', updateOverlayWidth)
  window.addEventListener('keydown', handleWindowKeydown)
})

onUnmounted(() => {
  stopResize()
  if (resizeFrameId !== 0) {
    cancelAnimationFrame(resizeFrameId)
    resizeFrameId = 0
  }
  resizeObserver?.disconnect()
  resizeObserver = null
  window.removeEventListener('resize', updateOverlayWidth)
  window.removeEventListener('keydown', handleWindowKeydown)
})

function resolveTabFromRoute(): 'workbench' | 'vulnerabilities' {
  if (route.path.startsWith('/security-center/workbench')) {
    return 'workbench'
  }

  if (typeof route.query.tab === 'string' && route.query.tab === 'vulnerabilities') {
    return 'vulnerabilities'
  }

  return 'vulnerabilities'
}
</script>

<style scoped>
.security-center-overlay-resizer {
  width: 10px;
  cursor: col-resize;
  background: linear-gradient(180deg, transparent 0%, rgb(148 163 184 / 0.12) 50%, transparent 100%);
  transition: background-color 160ms ease, opacity 160ms ease;
  opacity: 0.45;
}

.security-center-overlay-resizer:hover {
  opacity: 1;
  background: linear-gradient(180deg, transparent 0%, rgb(59 130 246 / 0.3) 50%, transparent 100%);
}

.security-center-overlay-resizing {
  transition: none !important;
  will-change: width;
}
</style>
