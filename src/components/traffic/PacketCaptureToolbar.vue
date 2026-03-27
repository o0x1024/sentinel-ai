<template>
  <div class="flex items-center gap-2 p-3 bg-base-200 border-b border-base-300">
    <select
      :value="selectedInterface"
      class="select select-sm select-bordered min-w-56"
      :disabled="isCapturing"
      @change="$emit('update:selectedInterface', ($event.target as HTMLSelectElement).value)"
    >
      <option value="">{{ $t('trafficAnalysis.packetCapture.toolbar.selectInterface') }}</option>
      <option v-for="iface in interfaces" :key="iface.name" :value="iface.name">
        {{ getInterfaceDisplayName(iface) }}
      </option>
    </select>

    <button class="btn btn-sm" :class="isCapturing ? 'btn-error' : 'btn-success'" @click="onToggleCapture" :disabled="!selectedInterface && !isCapturing">
      <i :class="isCapturing ? 'fas fa-stop' : 'fas fa-play'" class="mr-1"></i>
      {{ isCapturing ? $t('trafficAnalysis.packetCapture.toolbar.stop') : $t('trafficAnalysis.packetCapture.toolbar.start') }}
    </button>

    <button class="btn btn-sm btn-ghost" @click="onClearPackets" :disabled="packetCount === 0">
      <i class="fas fa-trash mr-1"></i>
      {{ $t('trafficAnalysis.packetCapture.toolbar.clear') }}
    </button>

    <div class="divider divider-horizontal mx-0"></div>

    <button class="btn btn-sm btn-ghost" @click="onOpenPcapFile" :disabled="isCapturing">
      <i class="fas fa-folder-open mr-1"></i>
      {{ $t('trafficAnalysis.packetCapture.toolbar.open') }}
    </button>

    <button class="btn btn-sm btn-ghost" @click="onSavePcapFile" :disabled="packetCount === 0">
      <i class="fas fa-save mr-1"></i>
      {{ $t('trafficAnalysis.packetCapture.toolbar.save') }}
    </button>

    <button class="btn btn-sm btn-ghost" @click="onOpenExtractDialog" :disabled="packetCount === 0">
      <i class="fas fa-file-export mr-1"></i>
      {{ $t('trafficAnalysis.packetCapture.toolbar.export') }}
    </button>

    <div class="divider divider-horizontal mx-0"></div>

    <button class="btn btn-sm btn-ghost" @click="onOpenFilterDialog">
      <i class="fas fa-sliders-h mr-1"></i>
      {{ $t('trafficAnalysis.packetCapture.toolbar.advancedFilter') }}
    </button>

    <div class="flex-1 flex items-center gap-2">
      <div class="relative flex-1">
        <input
          :value="filterText"
          type="text"
          :placeholder="filterPlaceholder"
          class="input input-sm input-bordered w-full pr-20"
          @input="$emit('update:filterText', ($event.target as HTMLInputElement).value)"
          @keyup.enter="onApplyFilter"
        />
        <div class="absolute right-1 top-1/2 -translate-y-1/2 flex gap-1">
          <button v-if="filterText || hasAdvancedFilter" class="btn btn-xs btn-ghost btn-circle" @click="onClearAllFilters">
            <i class="fas fa-times"></i>
          </button>
          <button class="btn btn-xs btn-primary" @click="onApplyFilter">
            <i class="fas fa-filter"></i>
          </button>
        </div>
      </div>
    </div>

    <div class="flex items-center gap-2">
      <span v-if="hasAdvancedFilter" class="badge badge-info badge-sm">{{ $t('trafficAnalysis.packetCapture.toolbar.advancedFilterBadge') }}</span>
      <span class="badge badge-ghost">{{ filteredPacketCount }} / {{ packetCount }}</span>
    </div>
  </div>
</template>

<script setup lang="ts">
import type { NetworkInterface } from './packetCaptureTypes'

defineProps<{
  selectedInterface: string
  interfaces: NetworkInterface[]
  isCapturing: boolean
  packetCount: number
  filteredPacketCount: number
  filterText: string
  filterPlaceholder: string
  hasAdvancedFilter: boolean
  getInterfaceDisplayName: (iface: NetworkInterface) => string
  onToggleCapture: () => void
  onClearPackets: () => void
  onOpenPcapFile: () => void
  onSavePcapFile: () => void
  onOpenExtractDialog: () => void
  onOpenFilterDialog: () => void
  onApplyFilter: () => void
  onClearAllFilters: () => void
}>()

defineEmits<{
  'update:selectedInterface': [value: string]
  'update:filterText': [value: string]
}>()
</script>
