<template>
  <aside ref="sidebarRoot" class="flex h-full min-h-0 flex-col gap-3">
    <section class="workbench-solid-surface workbench-sidebar-card flex h-full min-h-0 flex-col overflow-hidden rounded-[30px] border border-base-300/70 shadow-[0_20px_48px_rgba(15,23,42,0.08)]">
      <div class="workbench-header-surface border-b border-base-300/70 px-5 py-4">
        <div class="flex flex-wrap items-center justify-between gap-3">
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

      <div class="min-h-0 space-y-4 overflow-auto px-5 py-4">
        <div class="workbench-stat-strip">
          <div class="workbench-stat-cell">
            <span class="workbench-stat-label">{{ t('trafficAnalysis.workbench.stats.drafts', '草稿') }}</span>
            <div class="workbench-stat-main">
              <strong class="workbench-stat-value">{{ draftCount }}</strong>
              <span class="workbench-stat-meta">{{ t('trafficAnalysis.workbench.stats.draftsMeta', '草稿请求') }}</span>
            </div>
          </div>
          <div class="workbench-stat-cell">
            <span class="workbench-stat-label">{{ t('trafficAnalysis.workbench.stats.replay', '重放') }}</span>
            <div class="workbench-stat-main">
              <strong class="workbench-stat-value">{{ replaySummary.total }}</strong>
              <span class="workbench-stat-meta">
                {{ t('trafficAnalysis.workbench.stats.runningMeta', { count: replaySummary.running }) }}
              </span>
            </div>
          </div>
          <div class="workbench-stat-cell">
            <span class="workbench-stat-label">{{ t('trafficAnalysis.workbench.stats.attacks', '攻击') }}</span>
            <div class="workbench-stat-main">
              <strong class="workbench-stat-value">{{ attackWorkspaceCount }}</strong>
              <span class="workbench-stat-meta">
                {{ t('trafficAnalysis.workbench.stats.runningMeta', { count: runningAttackCount }) }}
              </span>
            </div>
          </div>
        </div>

        <div class="space-y-4">
          <div class="workbench-status-card">
            <div>
              <h4 class="text-sm font-semibold text-base-content">
                {{ t('trafficAnalysis.workbench.sidebar.workflowTitle', '当前工作流') }}
              </h4>
              <p class="mt-1 text-xs leading-5 text-base-content/55">
                {{ t('trafficAnalysis.workbench.sidebar.workflowDescription', '历史记录负责定位请求；重放器和爆破器的历史切换在各自工具内部完成。') }}
              </p>
            </div>
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
          </div>

          <div class="grid gap-2">
            <div class="workbench-compact-status-row">
              <div>
                <p class="text-xs font-semibold text-base-content">
                  {{ t('trafficAnalysis.workbench.sidebar.repeaterState', '重放器状态') }}
                </p>
                <p class="text-[11px] text-base-content/50">
                  {{ t('trafficAnalysis.workbench.sidebar.repeaterStateMeta', '历史由重放器 Tab 管理') }}
                </p>
              </div>
              <span class="rounded-full bg-base-200 px-2 py-1 text-[11px] font-semibold text-base-content/65">
                {{ draftCount }}
              </span>
            </div>
            <div class="workbench-compact-status-row">
              <div>
                <p class="text-xs font-semibold text-base-content">
                  {{ t('trafficAnalysis.workbench.sidebar.intruderState', '爆破器状态') }}
                </p>
                <p class="text-[11px] text-base-content/50">
                  {{ t('trafficAnalysis.workbench.sidebar.intruderStateMeta', '历史由爆破器工作区管理') }}
                </p>
              </div>
              <span class="rounded-full bg-base-200 px-2 py-1 text-[11px] font-semibold text-base-content/65">
                {{ attackWorkspaceCount }}
              </span>
            </div>
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
  draftCount: number
  attackWorkspaceCount: number
  replaySummary: { total: number; running: number }
  runningAttackCount: number
  layoutToggleLabel: string
  layoutToggleIcon: string
  effectiveLayoutLabel: string
}>()

const compactActionBadgeCount = computed(() => props.basketCount + props.controlInterceptCount)

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
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 1px;
  overflow: hidden;
  border-radius: 1rem;
  border: 1px solid hsl(var(--b3) / 0.65);
  background: hsl(var(--b3) / 0.65);
}

.workbench-stat-cell {
  display: flex;
  min-width: 0;
  flex-direction: column;
  gap: 0.2rem;
  background: hsl(var(--b1) / 0.88);
  padding: 0.6rem 0.65rem;
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
  border-radius: 1rem;
  border: 1px solid hsl(var(--b3) / 0.65);
  background: hsl(var(--b1) / 0.72);
}

.workbench-status-card {
  display: grid;
  gap: 1rem;
  padding: 1rem;
}

.workbench-compact-status-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
  padding: 0.8rem 0.9rem;
}

</style>
