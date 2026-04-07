<template>
  <div class="bg-base-100 border-b border-base-300 p-2 flex-shrink-0">
    <div class="mb-2 flex items-center gap-3">
      <button
        type="button"
        class="flex min-w-0 flex-1 items-center gap-2 rounded-md border border-base-300 bg-base-200/60 px-3 py-1.5 text-left transition-colors hover:border-primary/40 hover:bg-base-200"
        @click="openFilterDialog"
      >
        <i class="fas fa-filter text-sm" :class="filtersEnabled && hasActiveFilters ? 'text-primary' : 'text-base-content/50'"></i>
        <span class="shrink-0 text-sm font-medium text-base-content/90">
          {{ $t('trafficAnalysis.history.filterBar.label') }}:
        </span>
        <span class="min-w-0 truncate text-sm text-base-content/80">
          {{ filterSummary }}
        </span>
      </button>
      <label class="flex shrink-0 cursor-pointer items-center gap-2 text-sm text-base-content/80">
        <input
          type="checkbox"
          class="toggle toggle-primary toggle-sm"
          :checked="filtersEnabled"
          @change="toggleFiltersEnabled"
        />
        <span>{{ filtersEnabled ? $t('trafficAnalysis.history.filterBar.on') : $t('trafficAnalysis.history.filterBar.off') }}</span>
      </label>
    </div>

    <div class="flex items-center gap-2">
      <div class="tabs tabs-boxed tabs-xs bg-base-200 p-0.5">
        <a class="tab tab-xs" :class="{ 'tab-active': protocolFilter === 'all' }" @click="$emit('update:protocolFilter', 'all')">{{ $t('trafficAnalysis.history.protocol.all') }}</a>
        <a class="tab tab-xs" :class="{ 'tab-active': protocolFilter === 'http' }" @click="$emit('update:protocolFilter', 'http')">HTTP/S</a>
        <a class="tab tab-xs" :class="{ 'tab-active': protocolFilter === 'websocket' }" @click="$emit('update:protocolFilter', 'websocket')">WebSocket</a>
      </div>
      <button class="btn btn-xs" :class="hasActiveFilters ? 'btn-primary' : 'btn-ghost'" @click="openFilterDialog">
        <i class="fas fa-filter mr-1"></i>{{ $t('trafficAnalysis.history.filters') }}
      </button>
      <div class="flex-1"></div>
      <button class="btn btn-xs" :class="isMultiSelectMode ? 'btn-accent' : 'btn-ghost'" @click="toggleMultiSelectMode">
        <i class="fas fa-check-square mr-1"></i>{{ $t('trafficAnalysis.history.multiSelect') }}
      </button>
      <template v-if="isMultiSelectMode">
        <button class="btn btn-xs btn-ghost" @click="selectAllVisible"><i class="fas fa-check-double mr-1"></i>{{ $t('trafficAnalysis.history.selectAll') }}</button>
        <button class="btn btn-xs btn-ghost" @click="clearSelection"><i class="fas fa-times mr-1"></i>{{ $t('trafficAnalysis.history.clearSelection') }}</button>
        <div class="dropdown dropdown-end">
          <label tabindex="0" class="btn btn-xs btn-ghost">
            <i class="fas fa-not-equal mr-1"></i>{{ $t('trafficAnalysis.history.sendToComparer') }}
          </label>
          <ul tabindex="0" class="dropdown-content z-[1] menu p-2 shadow bg-base-100 rounded-box w-56 mt-1">
            <li><a @click="sendSelectedRequestVersionsToComparer">{{ $t('trafficAnalysis.history.batchCompare.requestVersions') }}</a></li>
            <li><a @click="sendSelectedResponseVersionsToComparer">{{ $t('trafficAnalysis.history.batchCompare.responseVersions') }}</a></li>
          </ul>
        </div>
        <button class="btn btn-xs btn-primary" @click="sendSelectedToAssistant('request')"><i class="fas fa-robot mr-1"></i>{{ $t('trafficAnalysis.history.sendToAssistant') }}</button>
        <div class="dropdown dropdown-end">
          <label tabindex="0" class="btn btn-xs btn-ghost"><i class="fas fa-download mr-1"></i>{{ $t('trafficAnalysis.history.export.export') }}</label>
          <ul tabindex="0" class="dropdown-content z-[1] menu p-2 shadow bg-base-100 rounded-box w-52 mt-1">
            <li><a @click="exportSelectedToFile('request')">{{ $t('trafficAnalysis.history.export.requests') }}</a></li>
            <li><a @click="exportSelectedToFile('response')">{{ $t('trafficAnalysis.history.export.responses') }}</a></li>
            <li><a @click="exportAsHAR">HAR</a></li>
          </ul>
        </div>
      </template>
      <button class="btn btn-xs btn-ghost" @click="refreshRequests"><i class="fas fa-sync-alt mr-1"></i>{{ $t('trafficAnalysis.history.refresh') }}</button>
      <button class="btn btn-xs btn-ghost text-error" @click="clearHistory"><i class="fas fa-trash mr-1"></i>{{ $t('trafficAnalysis.history.clear') }}</button>
    </div>
  </div>
</template>

<script setup lang="ts">
import type { ProxyHistoryProtocolFilter } from './proxyHistoryTypes'
defineProps<{ protocolFilter: ProxyHistoryProtocolFilter; hasActiveFilters: boolean; filtersEnabled: boolean; filterSummary: string; isMultiSelectMode: boolean; openFilterDialog: () => void; toggleFiltersEnabled: () => void; toggleMultiSelectMode: () => void; selectAllVisible: () => void; clearSelection: () => void; sendSelectedToAssistant: (type?: 'request') => void; sendSelectedRequestVersionsToComparer: () => void; sendSelectedResponseVersionsToComparer: () => void; exportSelectedToFile: (type: 'request' | 'response') => void; exportAsHAR: () => void; refreshRequests: () => void; clearHistory: () => void }>()
defineEmits<{ 'update:protocolFilter': [value: ProxyHistoryProtocolFilter] }>()
</script>
