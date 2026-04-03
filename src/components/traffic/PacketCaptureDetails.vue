<template>
  <div v-if="selectedPacket" class="flex-1 flex min-h-0 overflow-hidden">
    <div class="w-1/2 overflow-auto border-r border-base-300 protocol-tree-panel">
      <div v-for="(layer, idx) in selectedPacket.layers" :key="idx" class="protocol-layer">
        <div class="layer-header" :class="getLayerBgClass(layer.name)" @click="onToggleLayer(`layer-${idx}`)">
          <i class="fas fa-caret-right layer-toggle" :class="{ expanded: expandedLayers[`layer-${idx}`] }"></i>
          <span class="layer-title">{{ layer.display }}</span>
        </div>
        <div v-show="expandedLayers[`layer-${idx}`]" class="layer-content">
          <template v-for="(field, fidx) in layer.fields" :key="fidx">
            <div v-if="field.children && field.children.length > 0" class="field-group">
              <div class="field-row field-parent" @click="onToggleField(`layer-${idx}-field-${fidx}`)">
                <i class="fas fa-caret-right field-toggle" :class="{ expanded: expandedFields[`layer-${idx}-field-${fidx}`] }"></i>
                <span class="field-name">{{ field.name }}:</span>
                <span class="field-value">{{ field.value }}</span>
              </div>
              <div v-show="expandedFields[`layer-${idx}-field-${fidx}`]" class="field-children">
                <div
                  v-for="(child, cidx) in field.children"
                  :key="cidx"
                  class="field-row field-child"
                  @contextmenu.prevent="onShowFieldContextMenu($event, child.name, child.value)"
                >
                  <span class="field-name">{{ child.name }}:</span>
                  <span class="field-value">{{ child.value }}</span>
                </div>
              </div>
            </div>
            <div v-else class="field-row" @contextmenu.prevent="onShowFieldContextMenu($event, field.name, field.value)">
              <span class="field-name">{{ field.name }}:</span>
              <span class="field-value" :class="{ highlight: isHighlightField(field.name) }">{{ field.value }}</span>
            </div>
          </template>
        </div>
      </div>
    </div>

    <div class="w-1/2 overflow-auto p-2 bg-base-200/30">
      <div class="tabs tabs-boxed bg-base-200 mb-2">
        <a class="tab tab-sm" :class="{ 'tab-active': hexViewMode === 'hex' }" @click="$emit('update:hexViewMode', 'hex')">Hex</a>
        <a class="tab tab-sm" :class="{ 'tab-active': hexViewMode === 'ascii' }" @click="$emit('update:hexViewMode', 'ascii')">ASCII</a>
        <a class="tab tab-sm" :class="{ 'tab-active': hexViewMode === 'raw' }" @click="$emit('update:hexViewMode', 'raw')">Raw</a>
      </div>
      <PacketCaptureByteView
        class="text-xs font-mono"
        :raw-data="selectedPacket.raw"
        :mode="hexViewMode"
      />
    </div>
  </div>
</template>

<script setup lang="ts">
import PacketCaptureByteView from './PacketCaptureByteView.vue'
import type { Packet } from './packetCaptureTypes'

defineProps<{
  selectedPacket: Packet | null
  expandedLayers: Record<string, boolean>
  expandedFields: Record<string, boolean>
  hexViewMode: 'hex' | 'ascii' | 'raw'
  getLayerBgClass: (name: string) => string
  isHighlightField: (key: string) => boolean
  onToggleLayer: (key: string) => void
  onToggleField: (key: string) => void
  onShowFieldContextMenu: (event: MouseEvent, key: string, value: string) => void
}>()

defineEmits<{
  'update:hexViewMode': ['hex' | 'ascii' | 'raw']
}>()
</script>

<style scoped>
.protocol-tree-panel {
  @apply text-xs font-mono;
  background: var(--fallback-b1, oklch(var(--b1)));
}

.protocol-layer {
  border-bottom: 1px solid oklch(var(--bc) / 0.1);
}

.layer-header {
  @apply flex items-center gap-1 px-1 py-0.5 cursor-pointer select-none;
  min-height: 20px;
}

.layer-header:hover { filter: brightness(0.95); }

.layer-toggle {
  @apply w-3 text-base-content/50 transition-transform duration-100;
  font-size: 10px;
}

.layer-toggle.expanded { transform: rotate(90deg); }
.layer-title { @apply flex-1 truncate; }
.layer-content { @apply pl-3; }

.field-row {
  @apply flex gap-1 px-1 py-px hover:bg-base-200/50;
  min-height: 18px;
  line-height: 18px;
}

.field-parent { @apply cursor-pointer; }

.field-toggle {
  @apply w-3 text-base-content/50 transition-transform duration-100;
  font-size: 10px;
}

.field-toggle.expanded { transform: rotate(90deg); }
.field-children { @apply pl-4; }
.field-child { @apply text-base-content/80; }
.field-name { @apply text-base-content/60 whitespace-nowrap; }
.field-value { @apply text-base-content flex-1; }
.field-value.highlight { @apply text-primary font-semibold; }

.layer-frame { background-color: #f5f5f5; }
.layer-eth { background-color: #e8f4e8; }
.layer-ip { background-color: #e8f0f8; }
.layer-tcp { background-color: #f0e8f8; }
.layer-udp { background-color: #e8f8f8; }
.layer-http { background-color: #f0f8e8; }
.layer-dns { background-color: #f8f0e8; }
.layer-icmp { background-color: #f8e8e8; }
.layer-arp { background-color: #f8f8e8; }

:global(.dark) .layer-frame { background-color: #2a2a2a; }
:global(.dark) .layer-eth { background-color: #1a2a1a; }
:global(.dark) .layer-ip { background-color: #1a1a2a; }
:global(.dark) .layer-tcp { background-color: #2a1a2a; }
:global(.dark) .layer-udp { background-color: #1a2a2a; }
:global(.dark) .layer-http { background-color: #2a2a1a; }
:global(.dark) .layer-dns { background-color: #2a1a1a; }
:global(.dark) .layer-icmp { background-color: #2a1a1a; }
:global(.dark) .layer-arp { background-color: #2a2a1a; }
</style>
