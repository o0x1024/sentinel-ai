<template>
  <div
    v-if="minimizedItems.length > 0"
    ref="trayRef"
    class="pointer-events-none fixed z-[82] flex max-w-[min(22rem,calc(100vw-1.5rem))] flex-col gap-2"
    :class="alignmentClass"
    :style="trayStyle"
    @mouseenter="setTrayHovering(true)"
    @mouseleave="setTrayHovering(false)"
    @focusin="setTrayHovering(true)"
    @focusout="setTrayHovering(false)"
  >
    <div
      v-if="dockState.side"
      class="tray-edge-glow absolute rounded-full"
      :class="dockGlowClass"
    ></div>

    <div
      v-if="snapPreviewEdges.left"
      class="pointer-events-none fixed bottom-4 left-2 top-2 w-1 rounded-full bg-primary/25"
    ></div>
    <div
      v-if="snapPreviewEdges.right"
      class="pointer-events-none fixed bottom-4 right-2 top-2 w-1 rounded-full bg-primary/25"
    ></div>
    <div
      v-if="snapPreviewEdges.top"
      class="pointer-events-none fixed left-2 right-2 top-2 h-1 rounded-full bg-primary/25"
    ></div>
    <div
      v-if="snapPreviewEdges.bottom"
      class="pointer-events-none fixed bottom-2 left-2 right-2 h-1 rounded-full bg-primary/25"
    ></div>

    <section
      v-if="expanded"
      id="immersive-minimized-tool-tray-panel"
      class="pointer-events-auto w-full overflow-hidden rounded-[28px] border border-base-300/80 bg-base-100/96 p-2 shadow-[0_28px_72px_rgba(15,23,42,0.18)] backdrop-blur-xl"
      aria-label="最小化工具托盘"
    >
      <div class="flex items-center justify-between gap-3 px-2 py-1.5">
        <div class="flex items-center gap-3">
          <button
            type="button"
            class="flex h-9 w-9 cursor-grab items-center justify-center rounded-2xl bg-primary/10 text-primary transition hover:bg-primary/14 active:cursor-grabbing"
            :aria-label="trayDragHandleLabel"
            :aria-keyshortcuts="trayDragHandleShortcuts"
            @pointerdown.stop.prevent="startDrag"
            @keydown.stop.prevent="handleTrayHandleKeydown"
          >
            <i class="fas fa-grip-lines text-sm"></i>
          </button>
          <div>
            <p class="text-[11px] font-semibold uppercase tracking-[0.2em] text-primary/80">
              Tool Tray
            </p>
            <p class="text-sm font-semibold text-base-content">
              {{ t('common.minimize', '最小化') }} {{ minimizedItems.length }}
            </p>
          </div>
        </div>
        <button
          type="button"
          class="btn btn-sm btn-ghost rounded-2xl"
          :aria-label="t('common.close', '关闭')"
          @click="expanded = false"
        >
          <i class="fas fa-times"></i>
        </button>
      </div>

      <div class="mt-1 flex max-h-[min(22rem,calc(100vh-8rem))] flex-col gap-2 overflow-auto px-1 pb-1">
        <div
          v-for="item in minimizedItems"
          :key="item.id"
          class="flex items-center gap-3 rounded-[22px] border border-base-300/70 bg-base-100/92 px-3 py-2 shadow-[0_16px_40px_rgba(15,23,42,0.08)]"
        >
          <span class="flex h-10 w-10 shrink-0 items-center justify-center rounded-2xl bg-primary/12 text-primary">
            <i :class="`${item.icon} text-sm`"></i>
          </span>
          <div class="min-w-0 flex-1">
            <p class="truncate text-sm font-semibold text-base-content">{{ item.title }}</p>
            <p class="truncate text-xs text-base-content/60">{{ item.description }}</p>
          </div>
          <button
            type="button"
            class="btn btn-sm btn-outline rounded-2xl"
            :aria-label="item.restoreLabel"
            @click="handleRestore(item)"
          >
            <i class="fas fa-up-right-and-down-left-from-center"></i>
          </button>
          <button
            type="button"
            class="btn btn-sm btn-ghost rounded-2xl"
            :aria-label="t('common.close', '关闭')"
            @click="handleClose(item)"
          >
            <i class="fas fa-times"></i>
          </button>
        </div>
      </div>
    </section>

    <div
      class="pointer-events-auto flex items-center gap-2 rounded-[24px] border border-base-300/80 bg-base-100/94 px-2 py-2 shadow-[0_24px_64px_rgba(15,23,42,0.16)] backdrop-blur-xl transition-all duration-200 hover:border-primary/30 hover:shadow-[0_28px_72px_rgba(15,23,42,0.2)]"
    >
      <button
        type="button"
        class="flex h-9 w-9 shrink-0 cursor-grab items-center justify-center rounded-2xl bg-primary/10 text-primary transition hover:bg-primary/14 active:cursor-grabbing"
        :aria-label="trayDragHandleLabel"
        :aria-keyshortcuts="trayDragHandleShortcuts"
        @pointerdown.stop.prevent="startDrag"
        @keydown.stop.prevent="handleTrayHandleKeydown"
      >
        <i class="fas fa-grip-lines text-sm"></i>
      </button>
      <button
        type="button"
        class="flex min-w-0 flex-1 items-center gap-3 rounded-[20px] px-1 py-0.5 text-left"
        :aria-expanded="expanded"
        aria-controls="immersive-minimized-tool-tray-panel"
        :aria-label="trayLauncherLabel"
        @click="handleLauncherToggle"
      >
        <div class="flex items-center -space-x-2">
          <span
            v-for="item in leadingItems"
            :key="`tray-icon-${item.id}`"
            class="flex h-9 w-9 items-center justify-center rounded-2xl border border-base-100 bg-primary/12 text-primary shadow-sm"
          >
            <i :class="`${item.icon} text-sm`"></i>
          </span>
        </div>
        <div class="min-w-0 flex-1 text-left">
          <p class="text-sm font-semibold text-base-content">
            {{ t('common.minimize', '最小化') }} {{ minimizedItems.length }}
          </p>
          <p class="truncate text-xs text-base-content/60">
            {{ minimizedItems[0]?.title }}
          </p>
        </div>
        <span class="flex h-8 min-w-8 items-center justify-center rounded-2xl bg-base-200 px-2 text-xs font-semibold text-base-content/70">
          {{ minimizedItems.length }}
        </span>
        <i
          class="fas fa-chevron-up text-xs text-base-content/50 transition-transform duration-200"
          :class="expanded ? 'rotate-0' : 'rotate-180'"
        ></i>
      </button>
    </div>

    <p class="sr-only" aria-live="polite" aria-atomic="true">{{ trayAnnouncement }}</p>
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, reactive, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import {
  immersiveMinimizedToolOrder,
  type ImmersiveMinimizedToolId,
} from '@/services/immersiveMinimizedToolTray'
import {
  clearImmersiveSecurityCenterReturnPath,
  closeImmersiveSecurityCenterSidebar,
  immersiveSecurityCenterSidebarMinimized,
  restoreImmersiveSecurityCenterSidebar,
} from '@/services/immersiveSecurityCenterSidebar'
import {
  closeTrafficAssistant,
  restoreTrafficAssistant,
  trafficAssistantMinimized,
} from '@/services/trafficAssistantWorkspace'
import {
  applyImmersiveMinimizedToolTrayMagneticAttraction,
  buildDefaultImmersiveMinimizedToolTrayPosition,
  clampImmersiveMinimizedToolTrayPosition,
  IMMERSIVE_MINIMIZED_TOOL_TRAY_MARGIN,
  type ImmersiveTrayDockSide,
  persistImmersiveMinimizedToolTrayPosition,
  resolveImmersiveMinimizedToolTrayDockSide,
  readImmersiveMinimizedToolTrayPosition,
  resolveImmersiveMinimizedToolTraySnapEdges,
  resolveImmersiveMinimizedToolTrayPosition,
  snapImmersiveMinimizedToolTrayPosition,
  type ImmersiveTrayRect,
} from './immersiveMinimizedToolTrayPosition'
import {
  buildImmersiveFloatingPositionAnnouncement,
  describeImmersiveFloatingDockSide,
} from './immersiveFloatingA11y'
import { useImmersiveAnnouncement } from './useImmersiveAnnouncement'

defineOptions({
  name: 'ImmersiveMinimizedToolTray',
})

const { t } = useI18n()
const { announcement: trayAnnouncement, announce: announceTray } = useImmersiveAnnouncement()
const trayRef = ref<HTMLElement | null>(null)
const expanded = ref(false)
const trayDragHandleShortcuts =
  'ArrowUp ArrowDown ArrowLeft ArrowRight Shift+ArrowUp Shift+ArrowDown Shift+ArrowLeft Shift+ArrowRight Home End PageUp PageDown'
const position = reactive({
  x: 16,
  y: 16,
})
const viewport = reactive({
  width: typeof window === 'undefined' ? 1440 : window.innerWidth,
  height: typeof window === 'undefined' ? 900 : window.innerHeight,
})
const traySize = reactive({
  width: 280,
  height: 60,
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
const dockState = reactive<{
  side: ImmersiveTrayDockSide
  hovering: boolean
}>({
  side: null,
  hovering: false,
})

const minimizedItems = computed(() => {
  const itemById = new Map<ImmersiveMinimizedToolId, {
    id: string
    title: string
    description: string
    icon: string
    restoreLabel: string
    onRestore: () => void
    onClose: () => void
  }>()

  if (immersiveSecurityCenterSidebarMinimized.value) {
    itemById.set('security-center', {
      id: 'security-center',
      title: t('securityCenter.title', '安全中心'),
      description: t('securityCenter.immersiveSidebar.minimizedHint', '已最小化，可随时恢复'),
      icon: 'fas fa-shield-alt',
      restoreLabel: t('securityCenter.immersiveSidebar.restore', '恢复安全中心'),
      onRestore: restoreImmersiveSecurityCenterSidebar,
      onClose: () => {
        closeImmersiveSecurityCenterSidebar()
        clearImmersiveSecurityCenterReturnPath()
      },
    })
  }

  if (trafficAssistantMinimized.value) {
    itemById.set('traffic-assistant', {
      id: 'traffic-assistant',
      title: t('trafficAnalysis.aiWorkspace.title', 'AI 助手'),
      description: t('trafficAnalysis.aiWorkspace.panelDescription', '在当前分析上下文中继续协作'),
      icon: 'fas fa-robot',
      restoreLabel: t('trafficAnalysis.aiWorkspace.expand', '展开'),
      onRestore: restoreTrafficAssistant,
      onClose: closeTrafficAssistant,
    })
  }

  return immersiveMinimizedToolOrder.value
    .map(id => itemById.get(id))
    .filter((item): item is NonNullable<typeof item> => Boolean(item))
})

const leadingItems = computed(() => minimizedItems.value.slice(0, 2))
const alignmentClass = computed(() =>
  position.x <= IMMERSIVE_MINIMIZED_TOOL_TRAY_MARGIN + 1 ? 'items-start' : 'items-end',
)
const trayDockDescription = computed(() => describeImmersiveFloatingDockSide(dockState.side))
const trayDragHandleLabel = computed(
  () => `工具托盘。${trayDockDescription.value}。按方向键移动，按 Shift 加速，按 Home 吸附左侧，按 End 吸附右侧。`,
)
const trayLauncherLabel = computed(
  () => `工具托盘，当前有 ${minimizedItems.value.length} 个最小化工具。${trayDockDescription.value}。`,
)
const dockGlowClass = computed(() => {
  if (dockState.side === 'left') {
    return 'left-0 top-2.5 bottom-2.5 w-1'
  }

  if (dockState.side === 'right') {
    return 'right-0 top-2.5 bottom-2.5 w-1'
  }

  if (dockState.side === 'top') {
    return 'left-2.5 right-2.5 top-0 h-1'
  }

  return 'bottom-0 left-2.5 right-2.5 h-1'
})
const shouldCollapseToEdge = computed(
  () => !expanded.value && !dragState.isDragging && !!dockState.side && !dockState.hovering,
)
const snapPreviewEdges = computed(() =>
  dragState.isDragging
    ? resolveImmersiveMinimizedToolTraySnapEdges(
        {
          x: dragState.pendingX,
          y: dragState.pendingY,
        },
        viewport,
        traySize,
      )
    : { left: false, right: false, top: false, bottom: false },
)
const trayStyle = computed(() => ({
  left: `${position.x}px`,
  top: `${position.y}px`,
  transform: buildTrayTransform(),
  transition: dragState.isDragging ? 'none' : 'left 180ms ease, top 180ms ease, transform 180ms ease',
  willChange: dragState.isDragging ? 'left, top' : 'transform',
}))

let resizeObserver: ResizeObserver | null = null
const TRAY_KEYBOARD_STEP = 18
const TRAY_KEYBOARD_FAST_STEP = 54

function handleRestore(item: NonNullable<(typeof minimizedItems.value)[number]>) {
  item.onRestore()
  expanded.value = false
}

function handleClose(item: NonNullable<(typeof minimizedItems.value)[number]>) {
  item.onClose()
  if (minimizedItems.value.length <= 1) {
    expanded.value = false
  }
}

function handleLauncherToggle() {
  expanded.value = !expanded.value
}

function announceTrayPosition() {
  announceTray(buildImmersiveFloatingPositionAnnouncement('工具托盘', dockState.side, position))
}

function buildTrayTransform() {
  if (!shouldCollapseToEdge.value || !dockState.side) {
    return 'translate3d(0, 0, 0)'
  }

  const horizontalHiddenOffset = Math.max(0, traySize.width - 20)
  const verticalHiddenOffset = Math.max(0, traySize.height - 20)

  if (dockState.side === 'left') {
    return `translate3d(${-horizontalHiddenOffset}px, 0, 0)`
  }

  if (dockState.side === 'right') {
    return `translate3d(${horizontalHiddenOffset}px, 0, 0)`
  }

  if (dockState.side === 'top') {
    return `translate3d(0, ${-verticalHiddenOffset}px, 0)`
  }

  return `translate3d(0, ${verticalHiddenOffset}px, 0)`
}

function handleWindowPointerDown(event: MouseEvent) {
  if (!expanded.value) {
    return
  }

  const target = event.target as Node | null
  if (trayRef.value?.contains(target)) {
    return
  }

  expanded.value = false
}

function handleWindowKeydown(event: KeyboardEvent) {
  if (event.key === 'Escape') {
    expanded.value = false
  }
}

function updateViewport() {
  viewport.width = window.innerWidth
  viewport.height = window.innerHeight
}

function updateTraySize() {
  traySize.width = trayRef.value?.offsetWidth ?? traySize.width
  traySize.height = trayRef.value?.offsetHeight ?? traySize.height
}

function syncDockSide() {
  dockState.side = resolveImmersiveMinimizedToolTrayDockSide(position, viewport, traySize)
}

function getDockRect(): ImmersiveTrayRect | null {
  const dockElement = document.querySelector('.immersive-drill-toolbar') as HTMLElement | null
  if (!dockElement) {
    return null
  }

  const rect = dockElement.getBoundingClientRect()
  if (!Number.isFinite(rect.width) || !Number.isFinite(rect.height)) {
    return null
  }

  return {
    x: rect.left,
    y: rect.top,
    width: rect.width,
    height: rect.height,
  }
}

function reconcilePosition(preferredPosition?: { x: number; y: number }) {
  const preferred = preferredPosition
    ?? readImmersiveMinimizedToolTrayPosition()
    ?? buildDefaultImmersiveMinimizedToolTrayPosition(viewport, traySize)

  const nextPosition = resolveImmersiveMinimizedToolTrayPosition(
    preferred,
    viewport,
    traySize,
    getDockRect(),
  )
  position.x = nextPosition.x
  position.y = nextPosition.y
  syncDockSide()
}

function applyPendingPosition() {
  dragState.frameId = 0
  const next = clampImmersiveMinimizedToolTrayPosition(
    {
      x: dragState.pendingX,
      y: dragState.pendingY,
    },
    viewport,
    traySize,
  )
  position.x = next.x
  position.y = next.y
  syncDockSide()
}

function schedulePositionUpdate() {
  if (dragState.frameId !== 0) {
    return
  }

  dragState.frameId = window.requestAnimationFrame(applyPendingPosition)
}

function handlePointerMove(event: PointerEvent) {
  if (!dragState.isDragging || event.pointerId !== dragState.pointerId) {
    return
  }

  const nextPosition = applyImmersiveMinimizedToolTrayMagneticAttraction(
    {
      x: event.clientX - dragState.pointerOffsetX,
      y: event.clientY - dragState.pointerOffsetY,
    },
    viewport,
    traySize,
  )
  dragState.pendingX = nextPosition.x
  dragState.pendingY = nextPosition.y
  schedulePositionUpdate()
}

function stopDrag(pointerId?: number) {
  if (pointerId !== undefined && dragState.pointerId !== pointerId) {
    return
  }

  if (!dragState.isDragging) {
    return
  }

  if (dragState.frameId !== 0) {
    window.cancelAnimationFrame(dragState.frameId)
    dragState.frameId = 0
  }

  dragState.isDragging = false
  dragState.pointerId = -1
  const snapped = snapImmersiveMinimizedToolTrayPosition(position, viewport, traySize)
  reconcilePosition(snapped)
  persistImmersiveMinimizedToolTrayPosition(position)
  announceTrayPosition()
  window.removeEventListener('pointermove', handlePointerMove)
  window.removeEventListener('pointerup', handlePointerUp)
}

function handlePointerUp(event: PointerEvent) {
  stopDrag(event.pointerId)
}

function startDrag(event: PointerEvent) {
  const bounds = trayRef.value?.getBoundingClientRect()
  if (!bounds) {
    return
  }

  dragState.isDragging = true
  dragState.pointerId = event.pointerId
  dragState.pointerOffsetX = event.clientX - bounds.left
  dragState.pointerOffsetY = event.clientY - bounds.top
  dragState.pendingX = position.x
  dragState.pendingY = position.y
  dockState.hovering = false
  window.addEventListener('pointermove', handlePointerMove)
  window.addEventListener('pointerup', handlePointerUp)
}

function nudgeTrayPosition(deltaX: number, deltaY: number) {
  if (dragState.isDragging) {
    stopDrag()
  }

  const nextPosition = applyImmersiveMinimizedToolTrayMagneticAttraction(
    {
      x: position.x + deltaX,
      y: position.y + deltaY,
    },
    viewport,
    traySize,
  )
  const snapped = snapImmersiveMinimizedToolTrayPosition(nextPosition, viewport, traySize)
  reconcilePosition(snapped)
  persistImmersiveMinimizedToolTrayPosition(position)
  announceTrayPosition()
}

function moveTrayToDockSide(side: 'left' | 'right') {
  if (dragState.isDragging) {
    stopDrag()
  }

  const targetX =
    side === 'left'
      ? IMMERSIVE_MINIMIZED_TOOL_TRAY_MARGIN
      : viewport.width - traySize.width - IMMERSIVE_MINIMIZED_TOOL_TRAY_MARGIN
  const snapped = snapImmersiveMinimizedToolTrayPosition(
    {
      x: targetX,
      y: position.y,
    },
    viewport,
    traySize,
  )
  reconcilePosition(snapped)
  persistImmersiveMinimizedToolTrayPosition(position)
  announceTrayPosition()
}

function moveTrayVertically(direction: 'top' | 'bottom') {
  if (dragState.isDragging) {
    stopDrag()
  }

  const targetY =
    direction === 'top'
      ? IMMERSIVE_MINIMIZED_TOOL_TRAY_MARGIN
      : viewport.height - traySize.height - IMMERSIVE_MINIMIZED_TOOL_TRAY_MARGIN
  const snapped = snapImmersiveMinimizedToolTrayPosition(
    {
      x: position.x,
      y: targetY,
    },
    viewport,
    traySize,
  )
  reconcilePosition(snapped)
  persistImmersiveMinimizedToolTrayPosition(position)
  announceTrayPosition()
}

function handleTrayHandleKeydown(event: KeyboardEvent) {
  const step = event.shiftKey ? TRAY_KEYBOARD_FAST_STEP : TRAY_KEYBOARD_STEP

  if (event.key === 'ArrowUp') {
    nudgeTrayPosition(0, -step)
    return
  }

  if (event.key === 'ArrowDown') {
    nudgeTrayPosition(0, step)
    return
  }

  if (event.key === 'ArrowLeft') {
    nudgeTrayPosition(-step, 0)
    return
  }

  if (event.key === 'ArrowRight') {
    nudgeTrayPosition(step, 0)
    return
  }

  if (event.key === 'Home') {
    moveTrayToDockSide('left')
    return
  }

  if (event.key === 'End') {
    moveTrayToDockSide('right')
    return
  }

  if (event.key === 'PageUp') {
    moveTrayVertically('top')
    return
  }

  if (event.key === 'PageDown') {
    moveTrayVertically('bottom')
  }
}

function setTrayHovering(hovering: boolean) {
  if (dragState.isDragging) {
    dockState.hovering = false
    return
  }

  dockState.hovering = hovering
}

function handleWindowResize() {
  updateViewport()
  updateTraySize()
  reconcilePosition(position)
}

function handleDockPositionChanged() {
  updateTraySize()
  reconcilePosition(position)
}

watch(expanded, async () => {
  await nextTick()
  updateTraySize()
  reconcilePosition(position)
})

watch(minimizedItems, async items => {
  if (items.length === 0) {
    expanded.value = false
    stopDrag()
    return
  }

  await nextTick()
  updateTraySize()
  if (readImmersiveMinimizedToolTrayPosition()) {
    reconcilePosition(position)
    return
  }

  reconcilePosition()
})

onMounted(() => {
  window.addEventListener('mousedown', handleWindowPointerDown)
  window.addEventListener('keydown', handleWindowKeydown)
  window.addEventListener('resize', handleWindowResize)
  window.addEventListener('immersive-drill-dock-position-changed', handleDockPositionChanged)

  updateViewport()
  updateTraySize()
  reconcilePosition()

  if (typeof ResizeObserver !== 'undefined') {
    resizeObserver = new ResizeObserver(() => {
      updateTraySize()
      reconcilePosition(position)
    })

    if (trayRef.value) {
      resizeObserver.observe(trayRef.value)
    }
  }
})

onUnmounted(() => {
  stopDrag()
  resizeObserver?.disconnect()
  resizeObserver = null
  window.removeEventListener('mousedown', handleWindowPointerDown)
  window.removeEventListener('keydown', handleWindowKeydown)
  window.removeEventListener('resize', handleWindowResize)
  window.removeEventListener('immersive-drill-dock-position-changed', handleDockPositionChanged)
})
</script>

<style scoped>
.tray-edge-glow {
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
</style>
