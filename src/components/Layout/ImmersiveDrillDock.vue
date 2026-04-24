<template>
  <aside
    ref="toolbarRef"
    class="immersive-drill-toolbar fixed z-[80]"
    :style="toolbarStyle"
    @mouseenter="setToolbarHovering(true)"
    @mouseleave="setToolbarHovering(false)"
    @focusin="setToolbarHovering(true)"
    @focusout="setToolbarHovering(false)"
    @keydown="handleToolbarKeydown"
  >
    <div
      v-if="snapPreviewSide"
      class="snap-preview-indicator fixed rounded-full"
      :class="snapPreviewSide === 'left' ? 'left-2' : 'right-2'"
      :style="snapPreviewStyle"
    ></div>

    <div
      v-if="dockState.side"
      class="toolbar-edge-glow absolute top-2.5 bottom-2.5 w-1 rounded-full"
      :class="[
        dockState.side === 'left' ? 'left-0' : 'right-0',
        shouldCollapseToEdge ? 'opacity-100' : 'opacity-45',
      ]"
    ></div>

    <div
      ref="panelRef"
      role="toolbar"
      aria-orientation="vertical"
      :aria-label="title"
      class="flex flex-col items-center gap-1.5 rounded-[1.6rem] border border-base-300/70 bg-base-100/92 px-1.5 py-2.5 shadow-2xl backdrop-blur-xl"
      :class="[
        dragState.isDragging ? 'select-none shadow-primary/10' : '',
        dockState.side ? 'ring-1 ring-base-300/50' : '',
      ]"
      :style="panelStyle"
    >
      <button
        type="button"
        class="drag-handle group relative flex h-9 w-9 cursor-grab items-center justify-center rounded-xl bg-primary/12 text-primary transition-all duration-200 hover:bg-primary/18 hover:shadow-lg hover:shadow-primary/15 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary/60 focus-visible:ring-offset-2 focus-visible:ring-offset-base-100 active:cursor-grabbing active:scale-[0.98]"
        :title="title"
        :aria-label="toolbarDragHandleLabel"
        :aria-keyshortcuts="toolbarDragHandleShortcuts"
        data-toolbar-focusable
        @pointerdown="startDrag"
        @keydown.stop.prevent="handleToolbarHandleKeydown"
      >
        <span class="drag-grip" aria-hidden="true">
          <span v-for="index in 6" :key="`drag-grip-${index}`" class="drag-grip-dot"></span>
        </span>
          <i class="fas fa-crosshairs text-sm"></i>
      </button>

      <nav v-if="showTrafficWorkbenchControls" class="flex flex-col gap-1.5">
        <button
          v-for="item in trafficWorkbenchItems"
          :key="item.id"
          type="button"
          data-toolbar-focusable
          class="group relative flex h-10 w-10 items-center justify-center rounded-xl border transition-all duration-200 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary/60 focus-visible:ring-offset-2 focus-visible:ring-offset-base-100"
          :class="itemClasses(item)"
          :title="item.label"
          :aria-label="item.label"
          @click="item.onClick"
        >
          <i :class="`${item.icon} text-[13px]`"></i>
          <span v-if="item.count > 0" class="toolbar-badge" :class="item.badgeClass">
            {{ item.count }}
          </span>
          <span
            class="toolbar-label pointer-events-none absolute whitespace-nowrap rounded-xl border border-base-300/70 bg-base-100 px-3 py-2 text-xs font-medium text-base-content opacity-0 shadow-lg transition-all duration-200 group-hover:opacity-100"
            :class="tooltipDockClass"
          >
            {{ item.label }}
          </span>
        </button>
      </nav>

      <div class="h-px w-6 bg-base-300/80"></div>

      <button
        type="button"
        class="group relative flex h-10 w-10 items-center justify-center rounded-xl border border-base-300/70 bg-base-200/75 text-base-content/72 transition-all duration-200 hover:border-error/40 hover:bg-error/10 hover:text-error focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary/60 focus-visible:ring-offset-2 focus-visible:ring-offset-base-100"
        :title="exitLabel"
        :aria-label="exitLabel"
        data-toolbar-focusable
        @click="exitImmersiveMode"
      >
        <i class="fas fa-arrow-right-from-bracket text-[13px]"></i>
        <span
          class="toolbar-label pointer-events-none absolute whitespace-nowrap rounded-xl border border-base-300/70 bg-base-100 px-3 py-2 text-xs font-medium text-base-content opacity-0 shadow-lg transition-all duration-200 group-hover:opacity-100"
          :class="tooltipDockClass"
        >
          {{ exitLabel }}
        </span>
      </button>
    </div>

    <p class="sr-only" aria-live="polite" aria-atomic="true">{{ toolbarAnnouncement }}</p>
  </aside>
</template>

<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, reactive, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRoute, useRouter } from 'vue-router'
import { setImmersiveDrillModeEnabled } from '@/services/immersiveDrillMode'
import {
  closeImmersiveSecurityCenterSidebar,
  clearImmersiveSecurityCenterReturnPath,
  immersiveSecurityCenterReturnPath,
  immersiveSecurityCenterSidebarOpen,
  openImmersiveSecurityCenterSidebar,
} from '@/services/immersiveSecurityCenterSidebar'
import {
  closeTrafficAssistant,
  openTrafficAssistantPanel,
  trafficAssistantVisible,
} from '@/services/trafficAssistantWorkspace'
import {
  closeAllImmersiveTools,
  closeTopmostImmersiveTool,
} from '@/services/immersiveToolCoordinator'
import {
  openImmersiveTrafficWorkbenchTool,
  showImmersiveTrafficHistory,
  toggleImmersiveTrafficWorkbenchTool,
  toggleImmersiveTrafficBasket,
  toggleImmersiveTrafficInterceptDrawer,
  toggleImmersiveTrafficPluginsPanel,
  toggleImmersiveTrafficProxySettings,
  useImmersiveTrafficDockState,
  type ImmersiveTrafficWorkbenchTool,
} from '@/components/traffic/immersiveTrafficDockState'
import {
  buildImmersiveFloatingPositionAnnouncement,
  describeImmersiveFloatingDockSide,
} from './immersiveFloatingA11y'
import { useImmersiveAnnouncement } from './useImmersiveAnnouncement'

interface TrafficWorkbenchToolbarItem {
  id: string
  label: string
  icon: string
  active: boolean
  warning?: boolean
  count: number
  badgeClass?: string
  onClick: () => void
}

const { t } = useI18n()
const { announcement: toolbarAnnouncement, announce: announceToolbar } = useImmersiveAnnouncement()
const route = useRoute()
const router = useRouter()
const toolbarRef = ref<HTMLElement | null>(null)
const panelRef = ref<HTMLElement | null>(null)
const {
  workbenchOpen,
  activeWorkbenchTool,
  interceptDrawerOpen,
  proxySettingsOpen,
  trafficPluginsOpen,
  basketOpen,
  captureCount,
  repeaterCount,
  intruderCount,
  comparerCount,
  oastCount,
  controlInterceptCount,
  basketCount,
} = useImmersiveTrafficDockState()

const title = computed(() => t('common.immersiveDrillMode', '沉浸式挖洞模式'))
const exitLabel = computed(() => t('common.exitImmersiveDrillMode', '退出挖洞模式'))
const toolbarDragHandleShortcuts =
  'ArrowUp ArrowDown ArrowLeft ArrowRight Shift+ArrowUp Shift+ArrowDown Shift+ArrowLeft Shift+ArrowRight Home End PageUp PageDown'

const TOOLBAR_POSITION_STORAGE_KEY = 'sentinel:immersive-drill-toolbar-position:v1'
const TOOLBAR_VIEWPORT_MARGIN = 14
const TOOLBAR_DOCK_OFFSET = 2
const TOOLBAR_SNAP_THRESHOLD = 20
const TOOLBAR_PEEK_WIDTH = 8
const TOOLBAR_MAGNETIC_THRESHOLD = 88
const TOOLBAR_KEYBOARD_STEP = 18
const TOOLBAR_KEYBOARD_FAST_STEP = 54

interface ToolbarPosition {
  x: number
  y: number
}

type ToolbarDockSide = 'left' | 'right' | null

const position = reactive<ToolbarPosition>({
  x: TOOLBAR_VIEWPORT_MARGIN,
  y: TOOLBAR_VIEWPORT_MARGIN,
})

const dockState = reactive<{
  side: ToolbarDockSide
  hovering: boolean
}>({
  side: null,
  hovering: false,
})

const dragState = reactive({
  pointerId: -1,
  isDragging: false,
  pointerOffsetX: 0,
  pointerOffsetY: 0,
  pendingX: 0,
  pendingY: 0,
  frameId: 0,
})

const shouldCollapseToEdge = computed(
  () => !dragState.isDragging && !!dockState.side && !dockState.hovering,
)

const snapPreviewSide = computed<ToolbarDockSide>(() =>
  dragState.isDragging ? resolveDockSide(dragState.pendingX, TOOLBAR_MAGNETIC_THRESHOLD) : null,
)

const snapPreviewStyle = computed(() => ({
  top: `${position.y}px`,
  height: `${panelRef.value?.offsetHeight ?? 260}px`,
}))
const toolbarDockDescription = computed(() => describeImmersiveFloatingDockSide(dockState.side))
const toolbarDragHandleLabel = computed(
  () => `${title.value}。${toolbarDockDescription.value}。按方向键移动，按 Shift 加速，按 Home 吸附左侧，按 End 吸附右侧。`,
)

const tooltipDockClass = computed(() =>
  dockState.side === 'left'
    ? 'toolbar-label-left left-full ml-3 group-hover:translate-x-0'
    : 'toolbar-label-right right-full mr-3 group-hover:translate-x-0',
)

const getPanelWidth = () => panelRef.value?.offsetWidth ?? 52

const getPanelHeight = () => panelRef.value?.offsetHeight ?? 228

const toolbarStyle = computed(() => {
  return {
    left: `${position.x}px`,
    top: `${position.y}px`,
    width: `${getPanelWidth()}px`,
    height: `${getPanelHeight()}px`,
    willChange: dragState.isDragging ? 'left, top' : 'auto',
  }
})

const panelStyle = computed(() => ({
  transform: buildToolbarTransform(),
  transition: dragState.isDragging ? 'none' : 'transform 180ms ease, box-shadow 180ms ease',
  willChange: 'transform',
}))

const toggleSecurityCenter = () => {
  if (!immersiveSecurityCenterSidebarOpen.value) {
    openImmersiveSecurityCenterSidebar(route.fullPath)
    return
  }

  closeImmersiveSecurityCenterSidebar()
  const returnPath = immersiveSecurityCenterReturnPath.value
  clearImmersiveSecurityCenterReturnPath()
  if (returnPath && returnPath !== route.fullPath) {
    void router.push(returnPath)
  }
}

const showTrafficWorkbenchControls = computed(() => true)

const activateTrafficTool = (tool: ImmersiveTrafficWorkbenchTool) => {
  if (route.path !== '/traffic') {
    void router.push('/traffic')
    openImmersiveTrafficWorkbenchTool(tool)
    return
  }

  toggleImmersiveTrafficWorkbenchTool(tool)
}

const toggleTrafficAssistant = () => {
  if (route.path !== '/traffic') {
    void router.push('/traffic')
  }

  if (trafficAssistantVisible.value) {
    closeTrafficAssistant()
    return
  }

  openTrafficAssistantPanel()
}

const toggleTrafficPlugins = () => {
  if (route.path !== '/traffic') {
    void router.push('/traffic')
    toggleImmersiveTrafficPluginsPanel()
    return
  }

  toggleImmersiveTrafficPluginsPanel()
}

const trafficWorkbenchItems = computed<TrafficWorkbenchToolbarItem[]>(() => [
  {
    id: 'security-center',
    label: t('securityCenter.title', '安全中心'),
    icon: 'fas fa-shield-alt',
    active: immersiveSecurityCenterSidebarOpen.value,
    count: 0,
    onClick: toggleSecurityCenter,
  },
  {
    id: 'history',
    label: t('trafficAnalysis.tabs.history', '历史记录'),
    icon: 'fas fa-history',
    active: !workbenchOpen.value
      && !trafficAssistantVisible.value
      && !immersiveSecurityCenterSidebarOpen.value
      && !interceptDrawerOpen.value
      && !basketOpen.value
      && !proxySettingsOpen.value,
    count: 0,
    onClick: () => {
      closeTrafficAssistant()
      closeImmersiveSecurityCenterSidebar()
      clearImmersiveSecurityCenterReturnPath()
      showImmersiveTrafficHistory()
    },
  },
  {
    id: 'capture',
    label: t('trafficAnalysis.tabs.capture', '抓包'),
    icon: 'fas fa-wave-square',
    active: workbenchOpen.value && activeWorkbenchTool.value === 'capture',
    count: captureCount.value,
    badgeClass: 'toolbar-badge-primary',
    onClick: () => activateTrafficTool('capture'),
  },
  {
    id: 'repeater',
    label: t('trafficAnalysis.tabs.repeater', '重放器'),
    icon: 'fas fa-redo',
    active: workbenchOpen.value && activeWorkbenchTool.value === 'repeater',
    count: repeaterCount.value,
    badgeClass: 'toolbar-badge-primary',
    onClick: () => activateTrafficTool('repeater'),
  },
  {
    id: 'intruder',
    label: t('trafficAnalysis.tabs.intruder', '爆破器'),
    icon: 'fas fa-crosshairs',
    active: workbenchOpen.value && activeWorkbenchTool.value === 'intruder',
    count: intruderCount.value,
    badgeClass: 'toolbar-badge-primary',
    onClick: () => activateTrafficTool('intruder'),
  },
  {
    id: 'comparer',
    label: t('trafficAnalysis.tabs.comparer', '对比器'),
    icon: 'fas fa-not-equal',
    active: workbenchOpen.value && activeWorkbenchTool.value === 'comparer',
    count: comparerCount.value,
    badgeClass: 'toolbar-badge-primary',
    onClick: () => activateTrafficTool('comparer'),
  },
  {
    id: 'oast',
    label: t('trafficAnalysis.tabs.oast', 'OAST'),
    icon: 'fas fa-satellite-dish',
    active: workbenchOpen.value && activeWorkbenchTool.value === 'oast',
    count: oastCount.value,
    badgeClass: 'toolbar-badge-primary',
    onClick: () => activateTrafficTool('oast'),
  },
  {
    id: 'traffic-plugins',
    label: t('trafficAnalysis.immersivePlugins.title', '流量分析插件'),
    icon: 'fas fa-puzzle-piece',
    active: trafficPluginsOpen.value,
    count: 0,
    onClick: toggleTrafficPlugins,
  },
  {
    id: 'assistant',
    label: t('trafficAnalysis.aiWorkspace.launcherTitle', 'AI 助手'),
    icon: 'fas fa-robot',
    active: trafficAssistantVisible.value,
    count: 0,
    onClick: toggleTrafficAssistant,
  },
  {
    id: 'control',
    label: t('trafficAnalysis.tabs.control', '代理控制'),
    icon: 'fas fa-sliders-h',
    active: interceptDrawerOpen.value,
    warning: controlInterceptCount.value > 0,
    count: controlInterceptCount.value,
    badgeClass: 'toolbar-badge-warning',
    onClick: () => toggleImmersiveTrafficInterceptDrawer(),
  },
  {
    id: 'basket',
    label: '请求篮子',
    icon: 'fas fa-basket-shopping',
    active: basketOpen.value,
    count: basketCount.value,
    badgeClass: 'toolbar-badge-primary',
    onClick: () => toggleImmersiveTrafficBasket(),
  },
  {
    id: 'settings',
    label: '代理设置',
    icon: 'fas fa-cog',
    active: proxySettingsOpen.value,
    count: 0,
    onClick: () => toggleImmersiveTrafficProxySettings(),
  },
])

const itemClasses = (item: TrafficWorkbenchToolbarItem) => {
  if (item.active) {
    return 'border-primary bg-primary text-primary-content shadow-lg shadow-primary/20'
  }

  if (item.warning) {
    return 'border-warning/35 bg-warning/10 text-warning hover:border-warning/50 hover:bg-warning/15'
  }

  return 'border-base-300/70 bg-base-200/75 text-base-content/72 hover:border-primary/40 hover:bg-base-200'
}

const exitImmersiveMode = () => {
  closeAllImmersiveTools()
  clearImmersiveSecurityCenterReturnPath()
  setImmersiveDrillModeEnabled(false)
}

const announceToolbarPosition = () => {
  announceToolbar(buildImmersiveFloatingPositionAnnouncement(title.value, dockState.side, position))
}

function buildToolbarTransform() {
  if (!shouldCollapseToEdge.value || !dockState.side) {
    return 'translate3d(0, 0, 0)'
  }

  const width = getPanelWidth()
  const hiddenOffset = Math.max(0, width - TOOLBAR_PEEK_WIDTH)
  const direction = dockState.side === 'left' ? -1 : 1
  return `translate3d(${direction * hiddenOffset}px, 0, 0)`
}

const setToolbarHovering = (hovering: boolean) => {
  if (dragState.isDragging) {
    dockState.hovering = false
    return
  }

  dockState.hovering = hovering
}

const clampPosition = (x: number, y: number): ToolbarPosition => {
  const width = getPanelWidth()
  const height = getPanelHeight()
  const maxX = Math.max(TOOLBAR_VIEWPORT_MARGIN, window.innerWidth - width - TOOLBAR_VIEWPORT_MARGIN)
  const maxY = Math.max(TOOLBAR_VIEWPORT_MARGIN, window.innerHeight - height - TOOLBAR_VIEWPORT_MARGIN)

  return {
    x: Math.min(Math.max(TOOLBAR_VIEWPORT_MARGIN, x), maxX),
    y: Math.min(Math.max(TOOLBAR_VIEWPORT_MARGIN, y), maxY),
  }
}

const getDockedX = (side: Exclude<ToolbarDockSide, null>) => {
  if (side === 'left') {
    return TOOLBAR_DOCK_OFFSET
  }

  const width = getPanelWidth()
  return Math.max(TOOLBAR_DOCK_OFFSET, window.innerWidth - width - TOOLBAR_DOCK_OFFSET)
}

const applyMagneticAttraction = (x: number) => {
  const previewSide = resolveDockSide(x, TOOLBAR_MAGNETIC_THRESHOLD)
  if (!previewSide) {
    return x
  }

  const snappedX = getDockedX(previewSide)
  const distance = Math.abs(snappedX - x)
  if (distance > TOOLBAR_MAGNETIC_THRESHOLD) {
    return x
  }

  const pullRatio = 1 - distance / TOOLBAR_MAGNETIC_THRESHOLD
  const easedPull = pullRatio * pullRatio * 0.35
  return x + (snappedX - x) * easedPull
}

const applyPendingPosition = () => {
  dragState.frameId = 0
  const next = clampPosition(dragState.pendingX, dragState.pendingY)
  position.x = next.x
  position.y = next.y
}

const schedulePositionUpdate = () => {
  if (dragState.frameId) {
    return
  }

  dragState.frameId = window.requestAnimationFrame(applyPendingPosition)
}

const persistPosition = () => {
  window.localStorage.setItem(
    TOOLBAR_POSITION_STORAGE_KEY,
    JSON.stringify({ x: position.x, y: position.y, side: dockState.side }),
  )
  window.dispatchEvent(new CustomEvent('immersive-drill-dock-position-changed'))
}

const resolveDockSide = (x: number, threshold = TOOLBAR_SNAP_THRESHOLD): ToolbarDockSide => {
  const width = getPanelWidth()
  const leftDistance = x - TOOLBAR_DOCK_OFFSET
  const rightDistance = window.innerWidth - (x + width) - TOOLBAR_DOCK_OFFSET

  if (leftDistance <= threshold || rightDistance <= threshold) {
    return leftDistance <= rightDistance ? 'left' : 'right'
  }

  return null
}

const applySnapToEdge = () => {
  const side = resolveDockSide(position.x)
  dockState.side = side

  if (side === 'left') {
    position.x = getDockedX(side)
    return
  }

  if (side === 'right') {
    position.x = getDockedX(side)
  }
}

const loadInitialPosition = () => {
  const raw = window.localStorage.getItem(TOOLBAR_POSITION_STORAGE_KEY)
  if (raw) {
    try {
      const parsed = JSON.parse(raw) as Partial<ToolbarPosition> & {
        side?: ToolbarDockSide
      }
      if (Number.isFinite(parsed.x) && Number.isFinite(parsed.y)) {
        const next = clampPosition(parsed.x as number, parsed.y as number)
        position.x = next.x
        position.y = next.y
        dockState.side = parsed.side === 'left' || parsed.side === 'right' ? parsed.side : null
        applySnapToEdge()
        return
      }
    } catch {
      // Ignore invalid persisted position and fall back to default placement.
    }
  }

  const next = clampPosition(
    window.innerWidth - getPanelWidth() - TOOLBAR_VIEWPORT_MARGIN,
    Math.round((window.innerHeight - getPanelHeight()) / 2),
  )
  position.x = next.x
  position.y = next.y
  dockState.side = 'right'
  applySnapToEdge()
}

const handlePointerMove = (event: PointerEvent) => {
  if (!dragState.isDragging || event.pointerId !== dragState.pointerId) {
    return
  }

  const nextX = event.clientX - dragState.pointerOffsetX
  dragState.pendingX = applyMagneticAttraction(nextX)
  dragState.pendingY = event.clientY - dragState.pointerOffsetY
  schedulePositionUpdate()
}

const handlePointerUp = (event: PointerEvent) => {
  stopDrag(event.pointerId)
}

const handlePointerCancel = (event: PointerEvent) => {
  stopDrag(event.pointerId)
}

const stopDrag = (pointerId?: number) => {
  if (pointerId !== undefined && dragState.pointerId !== pointerId) {
    return
  }

  if (dragState.frameId) {
    window.cancelAnimationFrame(dragState.frameId)
    dragState.frameId = 0
    applyPendingPosition()
  }

  dragState.isDragging = false
  dragState.pointerId = -1
  dockState.hovering = false
  applySnapToEdge()
  persistPosition()
  announceToolbarPosition()
}

const handleResize = () => {
  const next = clampPosition(position.x, position.y)
  position.x = next.x
  position.y = next.y
  applySnapToEdge()
  persistPosition()
}

const nudgeToolbarPosition = (deltaX: number, deltaY: number) => {
  if (dragState.isDragging) {
    stopDrag()
  }

  dockState.side = null
  const next = clampPosition(position.x + deltaX, position.y + deltaY)
  position.x = applyMagneticAttraction(next.x)
  position.y = next.y
  applySnapToEdge()
  persistPosition()
  announceToolbarPosition()
}

const dockToolbarToSide = (side: Exclude<ToolbarDockSide, null>) => {
  if (dragState.isDragging) {
    stopDrag()
  }

  dockState.side = side
  const next = clampPosition(getDockedX(side), position.y)
  position.x = next.x
  position.y = next.y
  applySnapToEdge()
  persistPosition()
  announceToolbarPosition()
}

const moveToolbarVertically = (direction: 'top' | 'bottom') => {
  if (dragState.isDragging) {
    stopDrag()
  }

  const targetY =
    direction === 'top'
      ? TOOLBAR_VIEWPORT_MARGIN
      : window.innerHeight - getPanelHeight() - TOOLBAR_VIEWPORT_MARGIN
  const next = clampPosition(position.x, targetY)
  position.x = next.x
  position.y = next.y
  applySnapToEdge()
  persistPosition()
  announceToolbarPosition()
}

const startDrag = (event: PointerEvent) => {
  if (!toolbarRef.value) {
    return
  }

  const bounds = toolbarRef.value.getBoundingClientRect()
  dragState.isDragging = true
  dragState.pointerId = event.pointerId
  dragState.pointerOffsetX = event.clientX - bounds.left
  dragState.pointerOffsetY = event.clientY - bounds.top
  dragState.pendingX = position.x
  dragState.pendingY = position.y
  dockState.hovering = false
  dockState.side = null

  ;(event.currentTarget as HTMLElement | null)?.setPointerCapture?.(event.pointerId)
  event.preventDefault()
}

const getFocusableToolbarItems = () =>
  Array.from(
    toolbarRef.value?.querySelectorAll<HTMLElement>('[data-toolbar-focusable]') ?? [],
  )

const handleToolbarKeydown = (event: KeyboardEvent) => {
  if (!['ArrowUp', 'ArrowDown', 'ArrowLeft', 'ArrowRight', 'Home', 'End'].includes(event.key)) {
    return
  }

  const focusableItems = getFocusableToolbarItems()
  if (focusableItems.length === 0) {
    return
  }

  const activeElement = document.activeElement as HTMLElement | null
  const currentIndex = activeElement ? focusableItems.indexOf(activeElement) : -1
  let nextIndex = currentIndex

  if (event.key === 'Home') {
    nextIndex = 0
  } else if (event.key === 'End') {
    nextIndex = focusableItems.length - 1
  } else if (event.key === 'ArrowUp' || event.key === 'ArrowLeft') {
    nextIndex = currentIndex <= 0 ? focusableItems.length - 1 : currentIndex - 1
  } else if (event.key === 'ArrowDown' || event.key === 'ArrowRight') {
    nextIndex = currentIndex === -1 || currentIndex >= focusableItems.length - 1 ? 0 : currentIndex + 1
  }

  focusableItems[nextIndex]?.focus()
  event.preventDefault()
}

const handleToolbarHandleKeydown = (event: KeyboardEvent) => {
  const step = event.shiftKey ? TOOLBAR_KEYBOARD_FAST_STEP : TOOLBAR_KEYBOARD_STEP

  if (event.key === 'ArrowUp') {
    nudgeToolbarPosition(0, -step)
    return
  }

  if (event.key === 'ArrowDown') {
    nudgeToolbarPosition(0, step)
    return
  }

  if (event.key === 'ArrowLeft') {
    nudgeToolbarPosition(-step, 0)
    return
  }

  if (event.key === 'ArrowRight') {
    nudgeToolbarPosition(step, 0)
    return
  }

  if (event.key === 'Home') {
    dockToolbarToSide('left')
    return
  }

  if (event.key === 'End') {
    dockToolbarToSide('right')
    return
  }

  if (event.key === 'PageUp') {
    moveToolbarVertically('top')
    return
  }

  if (event.key === 'PageDown') {
    moveToolbarVertically('bottom')
  }
}

const handleWindowKeydown = (event: KeyboardEvent) => {
  if (event.defaultPrevented || event.isComposing || event.repeat) {
    return
  }

  if (event.key !== 'Escape') {
    return
  }

  if (closeTopmostImmersiveTool()) {
    event.preventDefault()
    event.stopPropagation()
    return
  }

  event.preventDefault()
  event.stopPropagation()
  exitImmersiveMode()
}

onMounted(() => {
  loadInitialPosition()
  window.addEventListener('pointermove', handlePointerMove, { passive: true })
  window.addEventListener('pointerup', handlePointerUp)
  window.addEventListener('pointercancel', handlePointerCancel)
  window.addEventListener('resize', handleResize)
  window.addEventListener('keydown', handleWindowKeydown)
})

onBeforeUnmount(() => {
  if (dragState.frameId) {
    window.cancelAnimationFrame(dragState.frameId)
  }
  window.removeEventListener('pointermove', handlePointerMove)
  window.removeEventListener('pointerup', handlePointerUp)
  window.removeEventListener('pointercancel', handlePointerCancel)
  window.removeEventListener('resize', handleResize)
  window.removeEventListener('keydown', handleWindowKeydown)
})
</script>

<style scoped>
.immersive-drill-toolbar {
  pointer-events: auto;
}

.snap-preview-indicator {
  width: 4px;
  background:
    linear-gradient(
      180deg,
      rgb(59 130 246 / 0.18) 0%,
      rgb(59 130 246 / 0.95) 50%,
      rgb(59 130 246 / 0.18) 100%
    );
  box-shadow:
    0 0 0 1px rgb(59 130 246 / 0.18),
    0 0 22px rgb(59 130 246 / 0.45);
  pointer-events: none;
}

.toolbar-edge-glow {
  background:
    linear-gradient(
      180deg,
      rgb(59 130 246 / 0.06) 0%,
      rgb(59 130 246 / 0.82) 50%,
      rgb(59 130 246 / 0.06) 100%
    );
  box-shadow: 0 0 18px rgb(59 130 246 / 0.35);
  transition: opacity 180ms ease;
  pointer-events: none;
}

.toolbar-badge {
  position: absolute;
  right: -0.2rem;
  top: -0.2rem;
  min-width: 1.1rem;
  height: 1.1rem;
  border-radius: 999px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  padding: 0 0.22rem;
  font-size: 0.62rem;
  font-weight: 700;
  box-shadow: 0 8px 18px rgb(15 23 42 / 0.18);
}

.toolbar-badge-primary {
  background: hsl(var(--p));
  color: hsl(var(--pc));
}

.toolbar-badge-warning {
  background: hsl(var(--wa));
  color: hsl(var(--wac));
}

.toolbar-label {
  z-index: 1;
}

.toolbar-label-right {
  transform: translateX(6px);
}

.toolbar-label-left {
  transform: translateX(-6px);
}

.drag-grip {
  position: absolute;
  bottom: 5px;
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 2px;
  opacity: 0.42;
  transition: opacity 180ms ease, transform 180ms ease;
}

.drag-handle:hover .drag-grip,
.drag-handle:focus-visible .drag-grip,
.drag-handle:active .drag-grip {
  opacity: 0.82;
  transform: translateY(-1px);
}

.drag-grip-dot {
  width: 3px;
  height: 3px;
  border-radius: 9999px;
  background: currentColor;
}
</style>
