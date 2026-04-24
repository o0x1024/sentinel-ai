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
        :class="hasActiveFilters ? 'btn-primary' : 'btn-ghost'"
        @click="openFilterDialog"
      >
        <i class="fas fa-filter mr-1"></i>
        {{ $t('trafficAnalysis.history.filters') }}
      </button>

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
        <div class="dropdown dropdown-end">
          <label
            tabindex="0"
            class="btn btn-xs btn-ghost"
            :class="{ 'btn-disabled pointer-events-none opacity-50': selectedCount === 0 }"
          >
            <i class="fas fa-download mr-1"></i>
            {{ $t('trafficAnalysis.history.export.export') }}
          </label>
          <ul tabindex="0" class="dropdown-content z-[1] menu mt-1 w-52 rounded-box bg-base-100 p-2 shadow">
            <li><a @click="exportSelectedToFile('request')">{{ $t('trafficAnalysis.history.export.requests') }}</a></li>
            <li><a @click="exportSelectedToFile('response')">{{ $t('trafficAnalysis.history.export.responses') }}</a></li>
            <li><a @click="exportAsHar">HAR</a></li>
          </ul>
        </div>
        <div class="dropdown dropdown-end">
          <label
            tabindex="0"
            class="btn btn-xs btn-ghost"
            :class="{ 'btn-disabled pointer-events-none opacity-50': filteredCount === 0 && selectedCount === 0 }"
          >
            <i class="fas fa-wand-magic-sparkles mr-1"></i>
            词典候选
          </label>
          <ul tabindex="0" class="dropdown-content z-[1] menu mt-1 w-56 rounded-box bg-base-100 p-2 shadow">
            <li>
              <a :class="{ 'pointer-events-none opacity-50': filteredCount === 0 }" @click="generateCandidatesFromFiltered">
                基于当前过滤结果
              </a>
            </li>
            <li>
              <a :class="{ 'pointer-events-none opacity-50': selectedCount === 0 }" @click="generateCandidatesFromSelection">
                基于当前多选记录
              </a>
            </li>
          </ul>
        </div>
      </template>

      <div class="min-w-0 flex-1"></div>

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
  hasActiveFilters: boolean
  isMultiSelectMode: boolean
  selectedCount: number
  filteredCount: number
  openFilterDialog: () => void
  toggleMultiSelectMode: () => void
  selectAllVisible: () => void
  exportSelectedToFile: (type: 'request' | 'response') => void
  exportAsHar: () => void
  generateCandidatesFromFiltered: () => void
  generateCandidatesFromSelection: () => void
  refreshRequests: () => void
  clearHistory: () => void
}>()

defineEmits<{
  'update:protocolFilter': [value: ProxyHistoryProtocolFilter]
}>()
</script>
