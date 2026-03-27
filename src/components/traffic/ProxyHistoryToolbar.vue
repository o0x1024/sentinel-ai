<template>
  <div class="bg-base-100 border-b border-base-300 p-2 flex-shrink-0">
    <div class="flex items-center gap-2">
      <div class="tabs tabs-boxed tabs-xs bg-base-200 p-0.5">
        <a class="tab tab-xs" :class="{ 'tab-active': protocolFilter === 'all' }" @click="$emit('update:protocolFilter', 'all')">{{ $t('trafficAnalysis.history.protocol.all') }}</a>
        <a class="tab tab-xs" :class="{ 'tab-active': protocolFilter === 'http' }" @click="$emit('update:protocolFilter', 'http')">HTTP/S</a>
        <a class="tab tab-xs" :class="{ 'tab-active': protocolFilter === 'websocket' }" @click="$emit('update:protocolFilter', 'websocket')">WebSocket</a>
      </div>
      <button class="btn btn-xs" :class="hasActiveFilters ? 'btn-primary' : 'btn-ghost'" @click="openFilterDialog">
        <i class="fas fa-filter mr-1"></i>{{ $t('trafficAnalysis.history.filters') }}
      </button>
      <div v-if="hasActiveFilters" class="text-xs text-base-content/60 truncate max-w-80">{{ filterSummary }}</div>
      <div class="flex-1"></div>
      <button class="btn btn-xs" :class="isMultiSelectMode ? 'btn-accent' : 'btn-ghost'" @click="toggleMultiSelectMode">
        <i class="fas fa-check-square mr-1"></i>{{ $t('trafficAnalysis.history.multiSelect') }}
      </button>
      <template v-if="isMultiSelectMode">
        <button class="btn btn-xs btn-ghost" @click="selectAllVisible"><i class="fas fa-check-double mr-1"></i>{{ $t('trafficAnalysis.history.selectAll') }}</button>
        <button class="btn btn-xs btn-ghost" @click="clearSelection"><i class="fas fa-times mr-1"></i>{{ $t('trafficAnalysis.history.clearSelection') }}</button>
        <button class="btn btn-xs btn-primary" @click="sendSelectedToAssistant('both')"><i class="fas fa-robot mr-1"></i>{{ $t('trafficAnalysis.history.sendToAssistant') }}</button>
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
defineProps<{ protocolFilter: ProxyHistoryProtocolFilter; hasActiveFilters: boolean; filterSummary: string; isMultiSelectMode: boolean; openFilterDialog: () => void; toggleMultiSelectMode: () => void; selectAllVisible: () => void; clearSelection: () => void; sendSelectedToAssistant: (type?: 'request' | 'response' | 'both') => void; exportSelectedToFile: (type: 'request' | 'response') => void; exportAsHAR: () => void; refreshRequests: () => void; clearHistory: () => void }>()
defineEmits<{ 'update:protocolFilter': [value: ProxyHistoryProtocolFilter] }>()
</script>
