<template>
  <div class="border-b border-base-300/80 bg-base-100 px-3 py-2.5">
    <div class="flex flex-wrap items-center gap-2">
      <div class="tabs tabs-boxed tabs-xs bg-base-200 p-0.5">
        <a class="tab tab-xs" :class="{ 'tab-active': protocolFilter === 'all' }" @click="$emit('update:protocolFilter', 'all')">
          {{ $t('trafficAnalysis.history.protocol.all') }}
        </a>
        <a class="tab tab-xs" :class="{ 'tab-active': protocolFilter === 'http' }" @click="$emit('update:protocolFilter', 'http')">
          HTTP/S
        </a>
        <a class="tab tab-xs" :class="{ 'tab-active': protocolFilter === 'websocket' }" @click="$emit('update:protocolFilter', 'websocket')">
          WebSocket
        </a>
      </div>

      <button
        type="button"
        class="btn btn-xs"
        :class="isMultiSelectMode ? 'btn-primary' : 'btn-ghost'"
        @click="toggleMultiSelectMode"
      >
        <i class="fas fa-check-square mr-1"></i>
        {{ $t('trafficAnalysis.history.multiSelect') }}
      </button>

      <template v-if="isMultiSelectMode">
        <button type="button" class="btn btn-xs btn-ghost" @click="selectAllVisible">
          <i class="fas fa-check-double mr-1"></i>
          {{ $t('trafficAnalysis.history.selectAll') }}
        </button>
        <button type="button" class="btn btn-xs btn-ghost" @click="clearSelection">
          <i class="fas fa-times mr-1"></i>
          {{ $t('trafficAnalysis.history.clearSelection') }}
        </button>
        <button
          type="button"
          class="btn btn-xs btn-outline"
          :disabled="selectedCount === 0"
          @click="addSelectedToBasket"
        >
          <i class="fas fa-basket-shopping mr-1"></i>
          加入篮子
        </button>
      </template>

      <div class="min-w-0 flex-1"></div>

      <div class="flex items-center gap-2 text-xs text-base-content/60">
        <span class="rounded-full bg-base-200 px-2 py-1">
          已筛选 {{ filteredCount }}
        </span>
        <span class="rounded-full bg-base-200 px-2 py-1">
          篮子 {{ basketCount }}
        </span>
      </div>

      <button type="button" class="btn btn-xs btn-ghost" @click="refreshRequests">
        <i class="fas fa-sync-alt mr-1"></i>
        {{ $t('trafficAnalysis.history.refresh') }}
      </button>
      <button type="button" class="btn btn-xs btn-ghost text-error" @click="clearHistory">
        <i class="fas fa-trash mr-1"></i>
        {{ $t('trafficAnalysis.history.clear') }}
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
import type { ProxyHistoryProtocolFilter } from './proxyHistoryTypes'

defineProps<{
  protocolFilter: ProxyHistoryProtocolFilter
  isMultiSelectMode: boolean
  selectedCount: number
  filteredCount: number
  basketCount: number
  toggleMultiSelectMode: () => void
  selectAllVisible: () => void
  clearSelection: () => void
  addSelectedToBasket: () => void
  refreshRequests: () => void
  clearHistory: () => void
}>()

defineEmits<{
  'update:protocolFilter': [value: ProxyHistoryProtocolFilter]
}>()
</script>
