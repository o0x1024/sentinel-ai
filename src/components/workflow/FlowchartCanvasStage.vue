<template>
  <div
    ref="containerEl"
    class="flowchart-container bg-base-200 rounded-lg min-h-[80vh] relative overflow-hidden"
    :class="{ 'cursor-grab': !isDragging && !isPanningCanvas && !isSpacePressed, 'cursor-grabbing': isPanningCanvas || isSpacePressed, 'cursor-move': isSpacePressed && !isPanningCanvas }"
    @pointerdown="onPointerDown"
    @pointermove="onPointerMove"
    @pointerup="onPointerUp"
    @wheel="onWheel"
  >
    <div v-if="nodes.length === 0" class="absolute inset-0 flex items-center justify-center pointer-events-none">
      <div class="text-center text-base-content/40">
        <svg xmlns="http://www.w3.org/2000/svg" class="h-16 w-16 mx-auto mb-4 opacity-30" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" /></svg>
        <p class="text-lg font-semibold mb-2">{{ emptyTitle }}</p>
        <p class="text-sm">{{ emptyDescription }}</p>
        <p class="text-xs mt-2">{{ emptyTip }}</p>
      </div>
    </div>
    <div class="flowchart-content" :style="contentStyle">
      <svg class="absolute inset-0 w-full h-full" :viewBox="`0 0 ${containerSize.width} ${containerSize.height}`" :width="containerSize.width" :height="containerSize.height">
        <defs><marker id="arrowhead" markerWidth="10" markerHeight="7" refX="9" refY="3.5" orient="auto"><polygon points="0 0, 10 3.5, 0 7" class="fill-primary" /></marker></defs>
        <path
          v-for="connection in connections"
          :key="connection.id + '-hit'"
          :d="connection.path"
          class="flowchart-connection-hit stroke-transparent fill-none cursor-pointer"
          style="stroke-width: 16px; pointer-events: stroke;"
          @pointerdown.stop
          @click.stop="onConnectionClick(connection)"
          @contextmenu.prevent.stop="onConnectionContextMenu($event, connection)"
        />
        <path v-for="connection in connections" :key="connection.id" :d="connection.path" :class="['stroke-2 fill-none pointer-events-none', getConnectionClass(connection)]" marker-end="url(#arrowhead)" />
        <path v-if="isDraggingConnection && tempConnectionPath" :d="tempConnectionPath" class="stroke-2 fill-none stroke-primary stroke-dasharray-4 opacity-70 pointer-events-none" marker-end="url(#arrowhead)" />
        <rect v-if="isSelecting" :x="Math.min(selectionBox.startX, selectionBox.endX)" :y="Math.min(selectionBox.startY, selectionBox.endY)" :width="Math.abs(selectionBox.endX - selectionBox.startX)" :height="Math.abs(selectionBox.endY - selectionBox.startY)" class="fill-primary/10 stroke-primary stroke-1 stroke-dasharray-4 pointer-events-none" />
      </svg>
      <template v-for="connection in connections" :key="connection.id + '-label'">
        <div
          v-if="connection.label"
          class="absolute pointer-events-none -translate-x-1/2 -translate-y-1/2 rounded bg-base-100/95 border border-base-300 px-2 py-0.5 text-[10px] font-mono text-base-content/70 shadow-sm"
          :style="{ left: `${connection.labelX || 0}px`, top: `${connection.labelY || 0}px` }"
        >
          {{ connection.label }}
        </div>
      </template>
      <div v-for="node in nodes" :key="node.id" :data-node-id="node.id" :class="['flowchart-node absolute z-10', node.id === draggedNode?.id ? 'cursor-grabbing duration-0' : 'cursor-pointer transition-all duration-200', 'border-2 rounded-lg p-3 w-[180px]', selectedNodes.has(node.id) ? 'ring-2 ring-primary ring-offset-2' : '', highlightedNodes.has(node.id) ? 'ring-2 ring-warning ring-offset-2 animate-pulse' : '', getNodeClass(node)]" :style="{ transform: `translate3d(${node.x}px, ${node.y}px, 0) ${node.id === draggedNode?.id ? 'scale(1.05)' : 'scale(1)'}` }" @pointerdown="onNodePointerDown($event, node)" @contextmenu.prevent="onNodeContextMenu($event, node)" @mouseenter="onNodeEnter(node)" @mouseleave="onNodeLeave(node)">
        <div class="absolute left-0 top-1/2 -translate-y-1/2 -translate-x-1/2 flex flex-col gap-1">
          <div v-for="port in node.metadata?.input_ports || [{ id: 'in', name: inputLabel }]" :key="port.id" class="port port-input w-3 h-3 rounded-full bg-primary border-2 border-white cursor-pointer hover:scale-125 transition-transform" :class="{ 'ring-2 ring-success': isDraggingConnection && hoverPort?.nodeId === node.id && hoverPort?.portId === port.id }" :title="port.name" @pointerup.stop="endDragConnection(node.id, port.id, 'input')" @pointerenter="setHoverPort(node.id, port.id, 'input')" @pointerleave="clearHoverPort" @contextmenu.prevent></div>
        </div>
        <div v-if="breakpoints.has(node.id)" class="absolute -top-2 -left-2 w-4 h-4 rounded-full bg-error flex items-center justify-center z-10" :title="breakpointsTitle"><svg xmlns="http://www.w3.org/2000/svg" class="h-3 w-3 text-white" fill="currentColor" viewBox="0 0 24 24"><circle cx="12" cy="12" r="10" /></svg></div>
        <div class="flex items-center gap-2 mb-2">
          <div :class="['w-4 h-4 rounded-full flex items-center justify-center', getStatusIndicatorClass(node.status)]">
            <svg v-if="node.status === 'completed'" xmlns="http://www.w3.org/2000/svg" class="h-3 w-3 text-white" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="3" d="M5 13l4 4L19 7" /></svg>
            <svg v-else-if="node.status === 'failed'" xmlns="http://www.w3.org/2000/svg" class="h-3 w-3 text-white" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="3" d="M6 18L18 6M6 6l12 12" /></svg>
            <svg v-else-if="node.status === 'paused'" xmlns="http://www.w3.org/2000/svg" class="h-3 w-3 text-white" fill="currentColor" viewBox="0 0 24 24"><path d="M6 4h4v16H6V4zm8 0h4v16h-4V4z"/></svg>
          </div>
          <span class="font-semibold text-sm truncate flex-1">{{ node.name }}</span>
          <span v-if="getNodeIcon(node.type)" class="text-lg" :title="node.type">{{ getNodeIcon(node.type) }}</span>
        </div>
        <div class="absolute right-0 top-1/2 -translate-y-1/2 translate-x-1/2 flex flex-col gap-1">
          <div v-for="port in node.metadata?.output_ports || [{ id: 'out', name: outputLabel }]" :key="port.id" class="port port-output w-3 h-3 rounded-full bg-secondary border-2 border-white cursor-pointer hover:scale-125 transition-transform" :class="{ 'ring-2 ring-success': isDraggingConnection && dragConnectionStart?.nodeId === node.id && dragConnectionStart?.portId === port.id }" :title="port.name" @pointerdown.stop="startDragConnection(node.id, port.id, 'output', $event)" @pointerenter="setHoverPort(node.id, port.id, 'output')" @pointerleave="clearHoverPort" @contextmenu.prevent></div>
        </div>
        <div class="text-xs text-base-content/70 mb-2 line-clamp-2">{{ node.description }}</div>
        <div class="flex justify-between items-center text-xs">
          <span :class="['badge badge-xs', getStatusBadgeClass(node.status)]">{{ getStatusText(node.status) }}</span>
          <span v-if="node.progress !== undefined" class="text-base-content/60">{{ Math.round(node.progress) }}%</span>
        </div>
        <div v-if="node.progress !== undefined && node.status === 'running'" class="mt-2"><progress class="progress progress-primary w-full h-1" :value="node.progress" max="100"></progress></div>
      </div>
    </div>
    <div v-if="showMinimap && nodes.length > 0" class="absolute bottom-4 right-4 w-48 h-32 bg-base-100/90 border border-base-300 rounded-lg shadow-lg overflow-hidden z-20" @pointerdown.stop="onMinimapClick">
      <div class="absolute inset-0 p-1">
        <svg class="w-full h-full" :viewBox="minimapViewBox">
          <rect v-for="node in nodes" :key="'minimap-' + node.id" :x="node.x" :y="node.y" width="180" height="80" class="fill-primary/30 stroke-primary stroke-1" rx="4" />
          <rect :x="minimapViewportRect.x" :y="minimapViewportRect.y" :width="minimapViewportRect.width" :height="minimapViewportRect.height" class="fill-none stroke-error stroke-2" rx="2" />
        </svg>
      </div>
      <button class="absolute top-1 right-1 btn btn-xs btn-ghost" @click.stop="$emit('toggleMinimap')">✕</button>
    </div>
    <div class="absolute bottom-4 left-4 text-xs text-base-content/50 bg-base-100/80 px-2 py-1 rounded z-20">
      <span v-if="selectedNodes.size > 0" class="mr-3 text-primary font-medium">{{ selectedHint }}</span>
      <span class="mr-3">{{ spaceHint }}</span>
      <span class="mr-3">{{ scrollHint }}</span>
      <span class="mr-3">{{ dragHint }}</span>
      <span>{{ selectAllHint }}</span>
    </div>
    <button v-if="isFullscreen" class="btn btn-sm btn-outline absolute top-2 right-2" @click="onToggleFullscreen">{{ exitFullscreenLabel }}</button>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import type { CSSProperties } from 'vue'
import type { FlowchartConnection, FlowchartNode, NodeStatus } from './flowchartVisualizationSupport'

const containerEl = ref<HTMLElement | null>(null)

defineProps<{
  isDragging: boolean
  isPanningCanvas: boolean
  isSpacePressed: boolean
  nodes: FlowchartNode[]
  connections: FlowchartConnection[]
  containerSize: { width: number; height: number }
  contentStyle: CSSProperties
  getConnectionClass: (connection: FlowchartConnection) => string
  isDraggingConnection: boolean
  tempConnectionPath: string
  isSelecting: boolean
  selectionBox: { startX: number; startY: number; endX: number; endY: number }
  draggedNode: FlowchartNode | null
  selectedNodes: Set<string>
  highlightedNodes: Set<string>
  getNodeClass: (node: FlowchartNode) => string[]
  hoverPort: { nodeId: string; portId: string; type: 'input' | 'output' } | null
  dragConnectionStart: { nodeId: string; portId: string; portType: 'input' | 'output'; x: number; y: number } | null
  breakpoints: Set<string>
  breakpointsTitle: string
  getStatusIndicatorClass: (status: NodeStatus) => string
  getStatusBadgeClass: (status: NodeStatus) => string
  getStatusText: (status: NodeStatus) => string
  getNodeIcon: (nodeType: string) => string
  inputLabel: string
  outputLabel: string
  showMinimap: boolean
  minimapViewBox: string
  minimapViewportRect: { x: number; y: number; width: number; height: number }
  selectedHint: string
  spaceHint: string
  scrollHint: string
  dragHint: string
  selectAllHint: string
  isFullscreen: boolean
  exitFullscreenLabel: string
  emptyTitle: string
  emptyDescription: string
  emptyTip: string
  onPointerDown: (event: PointerEvent) => void
  onPointerMove: (event: PointerEvent) => void
  onPointerUp: (event: PointerEvent) => void
  onWheel: (event: WheelEvent) => void
  onConnectionClick: (connection: FlowchartConnection) => void
  onConnectionContextMenu: (event: MouseEvent, connection: FlowchartConnection) => void
  onNodePointerDown: (event: PointerEvent, node: FlowchartNode) => void
  onNodeContextMenu: (event: MouseEvent, node: FlowchartNode) => void
  onNodeEnter: (node: FlowchartNode) => void
  onNodeLeave: (node: FlowchartNode) => void
  startDragConnection: (nodeId: string, portId: string, portType: 'input' | 'output', event: PointerEvent) => void
  endDragConnection: (targetNodeId: string, targetPortId: string, targetPortType: 'input' | 'output') => void
  setHoverPort: (nodeId: string, portId: string, type: 'input' | 'output') => void
  clearHoverPort: () => void
  onMinimapClick: (event: PointerEvent) => void
  onToggleFullscreen: () => void
}>()

defineEmits<{ toggleMinimap: [] }>()

defineExpose({
  containerEl,
})
</script>
