<template>
  <div class="flex-1 flex gap-4 min-h-0 overflow-hidden relative">
    <div
      :style="sidebarCollapsed ? undefined : { width: `${sidebarWidth}px` }"
      :class="[
        sidebarCollapsed ? 'w-12' : '',
        !sidebarTransitionReady || showWorkflowListPanel || isResizingSidebar ? 'transition-none no-node-lib-anim' : 'transition-[width] duration-300',
      ]"
      class="relative flex-shrink-0 flex flex-col"
    >
      <div v-if="!sidebarCollapsed" class="absolute top-0 right-0 h-full w-1.5 cursor-col-resize z-20 group" @mousedown="onStartSidebarResize">
        <div class="h-full w-full" :class="isResizingSidebar ? 'bg-primary/30' : 'bg-transparent group-hover:bg-base-content/10'"></div>
      </div>
      <div class="card bg-base-100 shadow-xl flex-1 flex flex-col overflow-hidden">
        <div class="card-body p-3 flex flex-col flex-1 overflow-hidden">
          <div class="flex items-center justify-between mb-2 flex-shrink-0">
            <h2 v-if="!sidebarCollapsed" class="text-base font-semibold">{{ t('trafficAnalysis.workflowStudio.sidebar.nodeLibrary') }}</h2>
            <button class="btn btn-xs btn-ghost" :title="sidebarCollapsed ? t('trafficAnalysis.workflowStudio.sidebar.expandSidebar') : t('trafficAnalysis.workflowStudio.sidebar.collapseSidebar')" @click="onToggleSidebarCollapsed">
              <svg v-if="sidebarCollapsed" xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 5l7 7-7 7M5 5l7 7-7 7" /></svg>
              <svg v-else xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11 19l-7-7 7-7m8 14l-7-7 7-7" /></svg>
            </button>
          </div>
          <div v-if="!sidebarCollapsed" class="flex flex-col flex-1 overflow-hidden">
            <div class="relative mb-2 flex-shrink-0">
              <input :value="searchQuery" class="input input-bordered input-sm w-full pr-16" :placeholder="t('trafficAnalysis.workflowStudio.sidebar.searchPlaceholder')" @input="$emit('update:searchQuery', ($event.target as HTMLInputElement).value); onSearchChange()" />
              <button v-if="searchQuery" class="btn btn-xs btn-ghost absolute right-8 top-1/2 -translate-y-1/2" :title="t('trafficAnalysis.workflowStudio.sidebar.clearSearchTooltip')" @click="onClearSearch">
                <svg xmlns="http://www.w3.org/2000/svg" class="h-3 w-3" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" /></svg>
              </button>
              <button class="btn btn-xs btn-ghost absolute right-1 top-1" :title="t('trafficAnalysis.workflowStudio.sidebar.searchInCanvasTooltip')" :disabled="!searchQuery" @click="onSearchInCanvas">
                <svg xmlns="http://www.w3.org/2000/svg" class="h-3 w-3" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" /></svg>
              </button>
            </div>
            <div class="form-control mb-2 flex-shrink-0">
              <label class="label cursor-pointer py-1">
                <span class="label-text text-xs">{{ t('trafficAnalysis.workflowStudio.sidebar.favoritesOnly') }}</span>
                <input :checked="showFavoritesOnly" type="checkbox" class="checkbox checkbox-xs" @change="$emit('update:showFavoritesOnly', ($event.target as HTMLInputElement).checked)" />
              </label>
            </div>
            <div class="space-y-2 overflow-y-auto flex-1">
              <div v-if="filteredGroups.length === 0" class="text-center text-sm text-base-content/60 py-4">{{ t('trafficAnalysis.workflowStudio.sidebar.noMatchingNodes') }}</div>
              <div v-for="group in filteredGroups" :key="group.name" class="collapse collapse-arrow bg-base-200">
                <input type="checkbox" />
                <div class="collapse-title text-sm font-medium py-2">{{ group.label }} ({{ group.items.length }})</div>
                <div class="collapse-content">
                  <div :class="['mcp', 'plugin'].includes(group.name) ? 'flex flex-col gap-1' : 'grid grid-cols-2 gap-2'">
                    <div
                      v-for="item in group.items"
                      :key="item.node_type"
                      class="btn btn-xs relative text-left justify-start"
                      :title="item.node_type"
                      role="button"
                      tabindex="0"
                      @click="onAddNode(item)"
                      @keydown.enter.prevent="onAddNode(item)"
                      @keydown.space.prevent="onAddNode(item)"
                    >
                      <span class="truncate flex-1">{{ item.label }}</span>
                      <button class="btn btn-ghost btn-xs p-0 w-4 h-4 ml-1 flex-shrink-0" :title="isFavorite(item.node_type) ? t('trafficAnalysis.workflowStudio.sidebar.unfavorite') : t('trafficAnalysis.workflowStudio.sidebar.favorite')" @click.stop="onToggleFavorite(item.node_type)">
                        <span v-if="isFavorite(item.node_type)">⭐</span>
                        <span v-else class="opacity-40">☆</span>
                      </button>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>

    <div class="flex-1 min-w-0">
      <slot name="canvas" />
    </div>
  </div>
</template>

<script setup lang="ts">
import type { NodeCatalogItem } from '@/types/workflow'
import { useI18n } from 'vue-i18n'

const { t } = useI18n()

defineProps<{
  sidebarCollapsed: boolean
  sidebarWidth: number
  sidebarTransitionReady: boolean
  isResizingSidebar: boolean
  showWorkflowListPanel: boolean
  searchQuery: string
  showFavoritesOnly: boolean
  filteredGroups: Array<{ name: string; label: string; items: NodeCatalogItem[] }>
  isFavorite: (nodeType: string) => boolean
  onToggleSidebarCollapsed: () => void
  onStartSidebarResize: (event: MouseEvent) => void
  onSearchChange: () => void
  onClearSearch: () => void
  onSearchInCanvas: () => void
  onAddNode: (item: NodeCatalogItem) => void
  onToggleFavorite: (nodeType: string) => void
}>()

defineEmits<{
  'update:searchQuery': [value: string]
  'update:showFavoritesOnly': [value: boolean]
}>()
</script>
