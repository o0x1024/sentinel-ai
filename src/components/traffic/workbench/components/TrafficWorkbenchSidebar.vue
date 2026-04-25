<template>
  <aside ref="sidebarRoot" class="flex h-full min-h-0 flex-col gap-2">
    <section class="workbench-solid-surface workbench-sidebar-card flex h-full min-h-0 flex-col overflow-hidden rounded-[18px] border border-base-300/70 shadow-[0_14px_34px_rgba(15,23,42,0.08)]">
      <div class="workbench-header-surface border-b border-base-300/70 px-3 py-2.5">
        <div class="flex flex-wrap items-center justify-between gap-2">
          <h2 class="text-lg font-semibold tracking-tight text-base-content">
            {{ t('trafficAnalysis.workbench.sidebar.title', '工作台') }}
          </h2>
          <div v-if="!compactHeaderActions" class="flex shrink-0 flex-wrap justify-end gap-2">
            <button type="button" class="btn btn-xs btn-outline rounded-2xl" @click="emitSidebarAction('openCapture')">
              {{ t('trafficAnalysis.tabs.capture', '抓包') }}
            </button>
            <button
              type="button"
              class="btn btn-xs btn-outline rounded-2xl"
              :title="effectiveLayoutLabel"
              @click="emitSidebarAction('toggleWorkbenchLayout')"
            >
              <i :class="`${layoutToggleIcon} mr-1`"></i>
              {{ layoutToggleLabel }}
            </button>
            <button type="button" class="btn btn-xs btn-outline rounded-2xl" @click="emitSidebarAction('toggleIntercept')">
              <i class="fas fa-sliders-h mr-1"></i>
              {{ t('trafficAnalysis.workbench.actions.control', '代理控制') }}
              <span v-if="controlInterceptCount > 0" class="badge badge-xs badge-warning">{{ controlInterceptCount }}</span>
            </button>
            <button type="button" class="btn btn-xs btn-outline rounded-2xl" @click="emitSidebarAction('toggleBasket')">
              <i class="fas fa-basket-shopping mr-1"></i>
              {{ t('trafficAnalysis.workbench.actions.basket', '篮子') }}
              <span v-if="basketCount > 0" class="badge badge-xs badge-primary">{{ basketCount }}</span>
            </button>
            <button type="button" class="btn btn-xs btn-outline rounded-2xl" @click="emitSidebarAction('openSettings')">
              <i class="fas fa-cog mr-1"></i>
              {{ t('trafficAnalysis.workbench.actions.settings', '代理设置') }}
            </button>
            <button type="button" class="btn btn-xs btn-outline rounded-2xl" @click="emitSidebarAction('openPlugins')">
              <i class="fas fa-puzzle-piece mr-1"></i>
              {{ t('trafficAnalysis.workbench.actions.plugins', '插件') }}
            </button>
          </div>
          <div v-else ref="actionMenuRef" class="relative shrink-0">
            <button
              type="button"
              class="btn btn-xs btn-outline rounded-2xl"
              @click="compactActionMenuOpen = !compactActionMenuOpen"
            >
              <i class="fas fa-ellipsis mr-1"></i>
              {{ t('trafficAnalysis.workbench.actions.more', '操作') }}
              <span
                v-if="compactActionBadgeCount > 0"
                class="badge badge-xs badge-primary"
              >
                {{ compactActionBadgeCount }}
              </span>
            </button>
            <ul
              v-if="compactActionMenuOpen"
              class="menu absolute right-0 top-full z-50 mt-2 w-56 rounded-box border border-base-300 bg-base-100 p-2 text-sm shadow-xl"
            >
              <li>
                <button type="button" @click="emitSidebarAction('openCapture')">
                  <i class="fas fa-wave-square"></i>
                  {{ t('trafficAnalysis.tabs.capture', '抓包') }}
                </button>
              </li>
              <li>
                <button type="button" @click="emitSidebarAction('toggleWorkbenchLayout')">
                  <i :class="layoutToggleIcon"></i>
                  {{ layoutToggleLabel }}
                </button>
              </li>
              <li>
                <button type="button" @click="emitSidebarAction('toggleIntercept')">
                  <i class="fas fa-sliders-h"></i>
                  {{ t('trafficAnalysis.workbench.actions.control', '代理控制') }}
                  <span v-if="controlInterceptCount > 0" class="badge badge-xs badge-warning ml-auto">{{ controlInterceptCount }}</span>
                </button>
              </li>
              <li>
                <button type="button" @click="emitSidebarAction('toggleBasket')">
                  <i class="fas fa-basket-shopping"></i>
                  {{ t('trafficAnalysis.workbench.actions.basket', '篮子') }}
                  <span v-if="basketCount > 0" class="badge badge-xs badge-primary ml-auto">{{ basketCount }}</span>
                </button>
              </li>
              <li>
                <button type="button" @click="emitSidebarAction('openSettings')">
                  <i class="fas fa-cog"></i>
                  {{ t('trafficAnalysis.workbench.actions.settings', '代理设置') }}
                </button>
              </li>
              <li>
                <button type="button" @click="emitSidebarAction('openPlugins')">
                  <i class="fas fa-puzzle-piece"></i>
                  {{ t('trafficAnalysis.workbench.actions.plugins', '插件') }}
                </button>
              </li>
            </ul>
          </div>
        </div>
      </div>

      <div class="min-h-0 space-y-2.5 overflow-auto px-3 py-3">
        <div class="workbench-stat-strip">
          <div class="workbench-stat-cell">
            <span class="workbench-stat-label">{{ t('trafficAnalysis.workbench.stats.repeaterHistory', '重放器历史') }}</span>
            <div class="workbench-stat-main">
              <strong class="workbench-stat-value">{{ repeaterHistoryCount }}</strong>
              <span class="workbench-stat-meta">{{ t('trafficAnalysis.workbench.stats.repeaterHistoryMeta', '重放记录') }}</span>
            </div>
          </div>
          <div class="workbench-stat-cell">
            <span class="workbench-stat-label">{{ t('trafficAnalysis.workbench.stats.intruderHistory', '爆破器历史') }}</span>
            <div class="workbench-stat-main">
              <strong class="workbench-stat-value">{{ intruderHistoryCount }}</strong>
              <span class="workbench-stat-meta">
                {{ t('trafficAnalysis.workbench.stats.runningMeta', { count: runningAttackCount }) }}
              </span>
            </div>
          </div>
        </div>

        <div class="space-y-2.5">
          <div class="workbench-status-card">
            <div class="grid grid-cols-2 gap-2">
              <button type="button" class="btn btn-sm btn-primary rounded-2xl" @click="$emit('openRepeater')">
                <i class="fas fa-redo mr-1"></i>
                {{ t('trafficAnalysis.tabs.repeater', '重放器') }}
              </button>
              <button type="button" class="btn btn-sm btn-outline rounded-2xl" @click="$emit('openIntruder')">
                <i class="fas fa-crosshairs mr-1"></i>
                {{ t('trafficAnalysis.tabs.intruder', '爆破器') }}
              </button>
            </div>
            <button
              type="button"
              class="btn btn-sm btn-error btn-outline rounded-2xl"
              :disabled="toolHistoryCount === 0"
              @click="$emit('clearToolHistory')"
            >
              <i class="fas fa-trash-alt mr-1"></i>
              {{ t('trafficAnalysis.workbench.sidebar.clearToolHistory', '清空工具历史') }}
            </button>
          </div>
        </div>
      </div>
    </section>
  </aside>
</template>

<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'

type SidebarAction =
  | 'openCapture'
  | 'openRepeater'
  | 'openIntruder'
  | 'toggleWorkbenchLayout'
  | 'toggleIntercept'
  | 'toggleBasket'
  | 'openSettings'
  | 'openPlugins'

const emit = defineEmits<{
  (e: 'openCapture'): void
  (e: 'openRepeater'): void
  (e: 'openIntruder'): void
  (e: 'clearToolHistory'): void
  (e: 'toggleWorkbenchLayout'): void
  (e: 'toggleIntercept'): void
  (e: 'toggleBasket'): void
  (e: 'openSettings'): void
  (e: 'openPlugins'): void
}>()

const { t } = useI18n()
const COMPACT_HEADER_ACTIONS_WIDTH = 560
const sidebarRoot = ref<HTMLElement | null>(null)
const actionMenuRef = ref<HTMLElement | null>(null)
const compactHeaderActions = ref(false)
const compactActionMenuOpen = ref(false)
let sidebarResizeObserver: ResizeObserver | null = null

const props = defineProps<{
  basketCount: number
  controlInterceptCount: number
  repeaterHistoryCount: number
  intruderHistoryCount: number
  runningAttackCount: number
  layoutToggleLabel: string
  layoutToggleIcon: string
  effectiveLayoutLabel: string
}>()

const compactActionBadgeCount = computed(() => props.basketCount + props.controlInterceptCount)
const toolHistoryCount = computed(() => props.repeaterHistoryCount + props.intruderHistoryCount)

function updateCompactHeaderActions(width: number) {
  compactHeaderActions.value = width < COMPACT_HEADER_ACTIONS_WIDTH
}

function closeActionMenu() {
  compactActionMenuOpen.value = false
}

function handleDocumentPointerDown(event: PointerEvent) {
  if (!compactActionMenuOpen.value) {
    return
  }

  const target = event.target instanceof Node ? event.target : null
  if (target && actionMenuRef.value?.contains(target)) {
    return
  }

  closeActionMenu()
}

function emitSidebarAction(action: SidebarAction) {
  switch (action) {
    case 'openCapture':
      emit('openCapture')
      break
    case 'openRepeater':
      emit('openRepeater')
      break
    case 'openIntruder':
      emit('openIntruder')
      break
    case 'toggleWorkbenchLayout':
      emit('toggleWorkbenchLayout')
      break
    case 'toggleIntercept':
      emit('toggleIntercept')
      break
    case 'toggleBasket':
      emit('toggleBasket')
      break
    case 'openSettings':
      emit('openSettings')
      break
    case 'openPlugins':
      emit('openPlugins')
      break
  }
  closeActionMenu()
}

onMounted(() => {
  if (!sidebarRoot.value) {
    return
  }

  updateCompactHeaderActions(sidebarRoot.value.clientWidth)
  sidebarResizeObserver = new ResizeObserver(entries => {
    const entry = entries[0]
    if (!entry) {
      return
    }
    updateCompactHeaderActions(entry.contentRect.width)
    closeActionMenu()
  })
  sidebarResizeObserver.observe(sidebarRoot.value)
  document.addEventListener('pointerdown', handleDocumentPointerDown)
})

onUnmounted(() => {
  sidebarResizeObserver?.disconnect()
  sidebarResizeObserver = null
  document.removeEventListener('pointerdown', handleDocumentPointerDown)
})
</script>

<style scoped>
.workbench-stat-strip {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 1px;
  overflow: hidden;
  border-radius: 0.75rem;
  border: 1px solid hsl(var(--b3) / 0.65);
  background: hsl(var(--b3) / 0.65);
}

.workbench-stat-cell {
  display: flex;
  min-width: 0;
  flex-direction: column;
  gap: 0.12rem;
  background: hsl(var(--b1) / 0.88);
  padding: 0.45rem 0.55rem;
}

.workbench-stat-main {
  display: flex;
  min-width: 0;
  flex-direction: column;
  gap: 0.12rem;
}

.workbench-stat-label {
  font-size: 10px;
  font-weight: 700;
  letter-spacing: 0.12em;
  text-transform: uppercase;
  color: hsl(var(--bc) / 0.42);
}

.workbench-stat-value {
  font-size: 1.4rem;
  line-height: 1;
  color: hsl(var(--bc));
}

.workbench-stat-meta {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: 10px;
  color: hsl(var(--bc) / 0.5);
}

.workbench-status-card,
.workbench-compact-status-row {
  border-radius: 0.75rem;
  border: 1px solid hsl(var(--b3) / 0.65);
  background: hsl(var(--b1) / 0.72);
}

.workbench-status-card {
  display: grid;
  gap: 0.65rem;
  padding: 0.75rem;
}

.workbench-compact-status-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.65rem;
  padding: 0.6rem 0.7rem;
}

</style>
