<template>
  <div
    ref="scrollContainerEl"
    class="min-h-0 overflow-auto border-b border-base-300 relative"
    :style="{ flex: `0 0 ${listHeight}px` }"
    @scroll="onHandleScroll"
  >
    <div v-if="filteredPackets.length > 0" class="relative" :style="{ height: totalHeight + 'px', minWidth: '900px' }">
      <div class="sticky top-0 z-10 flex bg-base-200 border-b border-base-300" :style="{ height: headerHeight + 'px' }">
        <div class="flex items-center justify-center px-2 border-r border-base-300 relative column-header" :style="{ width: columnWidths.mark + 'px' }">
          <div class="column-resize-handle" @mousedown="onStartColumnResize($event, 'mark')"></div>
        </div>
        <div class="flex items-center px-2 border-r border-base-300 text-xs font-semibold relative column-header" :style="{ width: columnWidths.no + 'px' }">
          {{ $t('trafficAnalysis.packetCapture.table.no') }}
          <div class="column-resize-handle" @mousedown="onStartColumnResize($event, 'no')"></div>
        </div>
        <div class="flex items-center px-2 border-r border-base-300 text-xs font-semibold relative column-header" :style="{ width: columnWidths.time + 'px' }">
          {{ $t('trafficAnalysis.packetCapture.table.time') }}
          <div class="column-resize-handle" @mousedown="onStartColumnResize($event, 'time')"></div>
        </div>
        <div class="flex items-center px-2 border-r border-base-300 text-xs font-semibold relative column-header" :style="{ width: columnWidths.source + 'px' }">
          {{ $t('trafficAnalysis.packetCapture.table.source') }}
          <div class="column-resize-handle" @mousedown="onStartColumnResize($event, 'source')"></div>
        </div>
        <div class="flex items-center px-2 border-r border-base-300 text-xs font-semibold relative column-header" :style="{ width: columnWidths.dest + 'px' }">
          {{ $t('trafficAnalysis.packetCapture.table.destination') }}
          <div class="column-resize-handle" @mousedown="onStartColumnResize($event, 'dest')"></div>
        </div>
        <div class="flex items-center px-2 border-r border-base-300 text-xs font-semibold relative column-header" :style="{ width: columnWidths.protocol + 'px' }">
          {{ $t('trafficAnalysis.packetCapture.table.protocol') }}
          <div class="column-resize-handle" @mousedown="onStartColumnResize($event, 'protocol')"></div>
        </div>
        <div class="flex items-center px-2 border-r border-base-300 text-xs font-semibold relative column-header" :style="{ width: columnWidths.length + 'px' }">
          {{ $t('trafficAnalysis.packetCapture.table.length') }}
          <div class="column-resize-handle" @mousedown="onStartColumnResize($event, 'length')"></div>
        </div>
        <div class="flex-1 flex items-center px-2 text-xs font-semibold">{{ $t('trafficAnalysis.packetCapture.table.info') }}</div>
      </div>

      <div
        v-for="item in visibleItems"
        :key="item.data.id"
        class="absolute left-0 right-0 flex cursor-pointer packet-row"
        :class="[
          getProtocolRowClass(item.data.protocol),
          { 'selected-row': selectedPacketId === item.data.id },
          { 'marked-row': markedPackets.has(item.data.id) },
          { 'ignored-row': ignoredPackets.has(item.data.id) },
        ]"
        :style="{ top: item.offset + headerHeight + 'px', height: rowHeight + 'px' }"
        @click="onSelectPacket(item.data)"
        @contextmenu.prevent="onShowContextMenu($event, item.data)"
      >
        <div class="flex items-center justify-center px-2 border-r border-base-300" :style="{ width: columnWidths.mark + 'px' }">
          <i v-if="markedPackets.has(item.data.id)" class="fas fa-bookmark text-warning text-xs"></i>
        </div>
        <div class="flex items-center px-2 border-r border-base-300 font-mono text-xs" :style="{ width: columnWidths.no + 'px' }">{{ item.data.id }}</div>
        <div class="flex items-center px-2 border-r border-base-300 font-mono text-xs" :style="{ width: columnWidths.time + 'px' }">{{ formatTime(item.data.timestamp) }}</div>
        <div class="flex items-center px-2 border-r border-base-300 font-mono text-xs truncate" :style="{ width: columnWidths.source + 'px' }">{{ item.data.src }}</div>
        <div class="flex items-center px-2 border-r border-base-300 font-mono text-xs truncate" :style="{ width: columnWidths.dest + 'px' }">{{ item.data.dst }}</div>
        <div class="flex items-center px-2 border-r border-base-300" :style="{ width: columnWidths.protocol + 'px' }">
          <span class="badge badge-sm" :class="getProtocolBadgeClass(item.data.protocol)">{{ item.data.protocol }}</span>
        </div>
        <div class="flex items-center px-2 border-r border-base-300 font-mono text-xs" :style="{ width: columnWidths.length + 'px' }">{{ item.data.length }}</div>
        <div class="flex-1 flex items-center px-2 text-xs truncate">{{ item.data.info }}</div>
      </div>
    </div>

    <div v-if="filteredPackets.length === 0" class="flex flex-col items-center justify-center h-full text-base-content/50 py-12">
      <template v-if="isLoading">
        <span class="loading loading-spinner loading-lg mb-4"></span>
        <p>{{ $t('trafficAnalysis.packetCapture.emptyState.loadingInterfaces') }}</p>
      </template>
      <template v-else-if="loadError === 'no_interfaces'">
        <i class="fas fa-exclamation-triangle text-4xl mb-4 text-warning"></i>
        <p class="mb-2">{{ $t('trafficAnalysis.packetCapture.emptyState.noInterfaces') }}</p>
        <p class="text-sm mb-4">{{ $t('trafficAnalysis.packetCapture.emptyState.npcapRequired') }}</p>
        <a href="https://nmap.org/npcap/" target="_blank" class="btn btn-sm btn-primary">
          <i class="fas fa-download mr-2"></i>{{ $t('trafficAnalysis.packetCapture.emptyState.downloadNpcap') }}
        </a>
      </template>
      <template v-else-if="loadError">
        <i class="fas fa-exclamation-circle text-4xl mb-4 text-error"></i>
        <p class="text-sm text-error">{{ loadError }}</p>
      </template>
      <template v-else>
        <i class="fas fa-broadcast-tower text-4xl mb-4"></i>
        <p v-if="!isCapturing">{{ $t('trafficAnalysis.packetCapture.emptyState.selectAndStart') }}</p>
        <p v-else>{{ $t('trafficAnalysis.packetCapture.emptyState.waitingForPackets') }}</p>
      </template>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import type { PacketSummary, VirtualItem } from './packetCaptureTypes'

const scrollContainerEl = ref<HTMLElement | null>(null)

defineProps<{
  filteredPackets: PacketSummary[]
  visibleItems: VirtualItem<PacketSummary>[]
  totalHeight: number
  headerHeight: number
  rowHeight: number
  listHeight: number
  columnWidths: Record<'mark' | 'no' | 'time' | 'source' | 'dest' | 'protocol' | 'length', number>
  selectedPacketId: number | null
  markedPackets: Set<number>
  ignoredPackets: Set<number>
  isLoading: boolean
  loadError: string | null
  isCapturing: boolean
  formatTime: (timestamp: number) => string
  getProtocolRowClass: (protocol: string) => string
  getProtocolBadgeClass: (protocol: string) => string
  onHandleScroll: (event: Event) => void
  onStartColumnResize: (event: MouseEvent, column: 'mark' | 'no' | 'time' | 'source' | 'dest' | 'protocol' | 'length') => void
  onSelectPacket: (packet: PacketSummary) => void
  onShowContextMenu: (event: MouseEvent, packet: PacketSummary) => void
}>()

defineExpose({
  scrollContainerEl,
})
</script>

<style scoped>
.column-header {
  position: relative;
  flex-shrink: 0;
}

.column-resize-handle {
  position: absolute;
  right: 0;
  top: 0;
  bottom: 0;
  width: 6px;
  cursor: col-resize;
  background: transparent;
  z-index: 5;
}

.column-resize-handle:hover {
  background: oklch(var(--p) / 0.3);
}

.column-resize-handle:active {
  background: oklch(var(--p) / 0.5);
}

.packet-row {
  transition: background-color 0.1s;
  border-bottom: 1px solid oklch(var(--bc) / 0.1);
}

.row-tcp { background-color: rgba(168, 162, 217, 0.15); }
.row-udp { background-color: rgba(125, 211, 252, 0.15); }
.row-http { background-color: rgba(134, 239, 172, 0.2); }
.row-https { background-color: rgba(253, 224, 71, 0.15); }
.row-dns { background-color: rgba(125, 211, 252, 0.2); }
.row-icmp { background-color: rgba(251, 146, 150, 0.15); }
.row-arp { background-color: rgba(253, 186, 116, 0.2); }
.row-other { background-color: transparent; }
.packet-row:hover { filter: brightness(0.95); }
.selected-row { @apply bg-primary/20 outline outline-1 outline-primary/50; }
.marked-row { @apply border-l-4 border-warning; }
.ignored-row { @apply opacity-40; }

:global(.dark) .row-tcp { background-color: rgba(100, 100, 160, 0.25); }
:global(.dark) .row-udp { background-color: rgba(80, 120, 160, 0.25); }
:global(.dark) .row-http { background-color: rgba(80, 160, 80, 0.25); }
:global(.dark) .row-https { background-color: rgba(160, 160, 80, 0.25); }
:global(.dark) .row-dns { background-color: rgba(80, 120, 160, 0.3); }
:global(.dark) .row-icmp { background-color: rgba(160, 100, 100, 0.25); }
:global(.dark) .row-arp { background-color: rgba(160, 140, 80, 0.25); }
</style>
