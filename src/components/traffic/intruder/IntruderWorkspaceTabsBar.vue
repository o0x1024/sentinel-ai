<template>
  <div
    class="flex items-center gap-1.5 border-b border-base-300"
    :class="immersiveDrillModeEnabled ? IMMERSIVE_TRAFFIC_TOP_BAR_CLASS : 'bg-base-200 px-1.5 py-0.5'"
  >
    <div
      v-if="tabContextMenu.visible"
      class="fixed z-50 min-w-40 rounded-lg border border-base-300 bg-base-100 py-1 shadow-xl"
      :style="{ left: `${tabContextMenu.x}px`, top: `${tabContextMenu.y}px` }"
      @click.stop
    >
      <TrafficContextMenuSections
        :sections="tabContextMenuSections"
        label-prefix="trafficAnalysis.intruder.tabContextMenu"
      />
    </div>

    <div class="flex min-w-0 flex-1 items-center gap-1 overflow-x-auto">
      <div
        v-for="(workspace, index) in workspaces"
        :key="workspace.id"
        :data-testid="`intruder-tab-${index}`"
        class="flex items-center gap-1.5 rounded border border-base-300 px-2.5 py-1 text-sm"
        :class="activeWorkspaceId === workspace.id ? 'bg-base-100 border-primary' : 'bg-base-200 hover:bg-base-300'"
        @contextmenu.prevent.stop="showTabContextMenu($event, workspace.id)"
      >
        <button
          class="truncate"
          type="button"
          :title="workspace.name"
          @click="$emit('update:activeWorkspaceId', workspace.id)"
        >
          {{ index + 1 }}
        </button>
        <button class="btn btn-ghost btn-xs btn-circle" type="button" @click="$emit('closeWorkspace', workspace.id)">
          <i class="fas fa-times text-[10px]"></i>
        </button>
      </div>

      <button class="btn btn-xs btn-ghost" type="button" @click="$emit('addWorkspace')">
        <i class="fas fa-plus"></i>
      </button>
    </div>

    <button
      data-testid="intruder-clear-all-workspaces"
      class="btn btn-xs btn-ghost text-error"
      type="button"
      :disabled="workspaces.length === 0"
      @click="$emit('clearAllWorkspaces')"
    >
      <i class="fas fa-trash-alt"></i>
      <span>{{ t('trafficAnalysis.intruder.actions.clearAll') }}</span>
    </button>
  </div>
</template>

<script setup lang="ts">
import { computed, onUnmounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { immersiveDrillModeEnabled } from '@/services/immersiveDrillMode'
import { IMMERSIVE_TRAFFIC_TOP_BAR_CLASS } from '../immersiveTrafficUi'
import TrafficContextMenuSections from '../TrafficContextMenuSections.vue'
import { buildTrafficContextMenuSections } from '../trafficContextMenuSectionSupport'

interface IntruderWorkspaceTabItem {
  id: string
  name: string
}

const { t } = useI18n()

const props = defineProps<{
  workspaces: IntruderWorkspaceTabItem[]
  activeWorkspaceId: string | null
}>()

const emit = defineEmits<{
  (e: 'update:activeWorkspaceId', workspaceId: string): void
  (e: 'closeWorkspace', workspaceId: string): void
  (e: 'closeOtherWorkspaces', workspaceId: string): void
  (e: 'addWorkspace'): void
  (e: 'clearAllWorkspaces'): void
}>()

const tabContextMenu = ref({
  visible: false,
  x: 0,
  y: 0,
  workspaceId: null as string | null,
})

const tabContextMenuSections = computed(() =>
  buildTrafficContextMenuSections([
    {
      key: 'workspace-tabs',
      items: [
        {
          key: 'deleteCurrent',
          iconClass: 'fas fa-trash-alt text-error',
          labelKey: 'deleteCurrent',
          onClick: () => {
            const workspaceId = tabContextMenu.value.workspaceId
            hideTabContextMenu()
            if (workspaceId) emit('closeWorkspace', workspaceId)
          },
        },
        {
          key: 'deleteOthers',
          iconClass: 'fas fa-layer-group text-warning',
          labelKey: 'deleteOthers',
          onClick: () => {
            const workspaceId = tabContextMenu.value.workspaceId
            hideTabContextMenu()
            if (workspaceId) emit('closeOtherWorkspaces', workspaceId)
          },
        },
        {
          key: 'deleteAll',
          iconClass: 'fas fa-trash text-error',
          labelKey: 'deleteAll',
          onClick: () => {
            hideTabContextMenu()
            emit('clearAllWorkspaces')
          },
          disabled: props.workspaces.length === 0,
        },
      ],
    },
  ]),
)

function showTabContextMenu(event: MouseEvent, workspaceId: string) {
  const menuWidth = 180
  const menuHeight = 136
  let x = event.clientX
  let y = event.clientY
  if (x + menuWidth > window.innerWidth) x = window.innerWidth - menuWidth - 10
  if (y + menuHeight > window.innerHeight) y = window.innerHeight - menuHeight - 10

  tabContextMenu.value = {
    visible: true,
    x: Math.max(0, x),
    y: Math.max(0, y),
    workspaceId,
  }

  window.setTimeout(() => {
    document.addEventListener('click', hideTabContextMenu)
  }, 0)
}

function hideTabContextMenu() {
  tabContextMenu.value.visible = false
  document.removeEventListener('click', hideTabContextMenu)
}

onUnmounted(() => {
  document.removeEventListener('click', hideTabContextMenu)
})
</script>
