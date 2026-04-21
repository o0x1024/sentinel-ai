<template>
  <div
    v-if="minimizedItems.length > 0"
    ref="trayRef"
    class="pointer-events-none fixed bottom-4 right-4 z-[82] flex max-w-[min(22rem,calc(100vw-1.5rem))] flex-col items-end gap-2"
  >
    <section
      v-if="expanded"
      class="pointer-events-auto w-full overflow-hidden rounded-[28px] border border-base-300/80 bg-base-100/96 p-2 shadow-[0_28px_72px_rgba(15,23,42,0.18)] backdrop-blur-xl"
    >
      <div class="flex items-center justify-between gap-3 px-2 py-1.5">
        <div>
          <p class="text-[11px] font-semibold uppercase tracking-[0.2em] text-primary/80">
            Tool Tray
          </p>
          <p class="text-sm font-semibold text-base-content">
            {{ t('common.minimize', '最小化') }} {{ minimizedItems.length }}
          </p>
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

    <button
      type="button"
      class="pointer-events-auto flex items-center gap-3 rounded-[24px] border border-base-300/80 bg-base-100/94 px-3 py-2 shadow-[0_24px_64px_rgba(15,23,42,0.16)] backdrop-blur-xl transition-all duration-200 hover:border-primary/30 hover:shadow-[0_28px_72px_rgba(15,23,42,0.2)]"
      :aria-expanded="expanded"
      :aria-label="t('common.minimize', '最小化')"
      @click="expanded = !expanded"
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
      <div class="min-w-0 text-left">
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
</template>

<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'
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

defineOptions({
  name: 'ImmersiveMinimizedToolTray',
})

const { t } = useI18n()
const trayRef = ref<HTMLElement | null>(null)
const expanded = ref(false)

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

watch(minimizedItems, items => {
  if (items.length === 0) {
    expanded.value = false
  }
})

onMounted(() => {
  window.addEventListener('mousedown', handleWindowPointerDown)
  window.addEventListener('keydown', handleWindowKeydown)
})

onUnmounted(() => {
  window.removeEventListener('mousedown', handleWindowPointerDown)
  window.removeEventListener('keydown', handleWindowKeydown)
})
</script>
