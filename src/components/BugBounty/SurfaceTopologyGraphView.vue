<template>
  <div class="grid grid-cols-1 xl:grid-cols-[minmax(0,1fr)_320px] gap-4">
    <div class="rounded-2xl border border-base-300 bg-[radial-gradient(circle_at_top,_rgba(14,116,144,0.12),_transparent_55%),linear-gradient(180deg,_rgba(255,255,255,0.02),_rgba(15,23,42,0.02))]">
      <div class="border-b border-base-300 px-4 py-3">
        <div class="text-sm font-medium">{{ t('bugBounty.surface.topology.graphTitle') }}</div>
        <div class="mt-1 text-xs text-base-content/60">{{ t('bugBounty.surface.topology.graphHint') }}</div>
      </div>

      <div v-if="loading" class="flex min-h-[420px] items-center justify-center text-sm text-base-content/60">
        <span class="loading loading-spinner loading-md mr-2"></span>
        {{ t('common.loading') }}
      </div>

      <div v-else-if="!nodes.length" class="flex min-h-[420px] items-center justify-center text-sm text-base-content/60">
        {{ t('bugBounty.surface.topology.emptyNodes') }}
      </div>

      <div v-else-if="!edges.length" class="flex min-h-[420px] items-center justify-center text-sm text-base-content/60">
        {{ t('bugBounty.surface.topology.emptyGraph') }}
      </div>

      <div v-else class="overflow-auto p-4">
        <svg :viewBox="`0 0 ${layout.width} ${layout.height}`" :style="{ minWidth: `${layout.width}px`, minHeight: `${layout.height}px` }" class="w-full">
          <g v-for="column in layout.columns" :key="column.type">
            <text
              :x="column.x"
              y="44"
              text-anchor="middle"
              class="fill-current text-[14px] font-semibold text-base-content/70"
            >
              {{ formatAssetType(column.type) }}
            </text>
          </g>

          <g>
            <path
              v-for="edge in edges"
              :key="edge.id"
              :d="buildEdgePath(edge)"
              fill="none"
              :stroke="edgeColor(edge.relation_type)"
              :stroke-opacity="edgeOpacity(edge)"
              stroke-width="2.5"
              stroke-linecap="round"
            />
          </g>

          <g
            v-for="node in nodes"
            :key="node.id"
            class="cursor-pointer"
            @click="selectedNodeId = node.id"
          >
            <rect
              :x="nodeLayout[node.id]?.x"
              :y="nodeLayout[node.id]?.y"
              :width="nodeWidth"
              :height="nodeHeight"
              rx="18"
              :fill="nodeFill(node)"
              :fill-opacity="nodeOpacity(node)"
              :stroke="selectedNodeId === node.id ? '#0f766e' : '#94a3b8'"
              :stroke-width="selectedNodeId === node.id ? 3 : 1.5"
            />
            <text
              :x="(nodeLayout[node.id]?.x || 0) + 16"
              :y="(nodeLayout[node.id]?.y || 0) + 26"
              class="fill-current text-[11px] font-semibold text-base-content/80"
            >
              {{ formatAssetType(node.asset_type) }}
            </text>
            <text
              :x="(nodeLayout[node.id]?.x || 0) + 16"
              :y="(nodeLayout[node.id]?.y || 0) + 48"
              class="fill-current font-mono text-[12px] text-base-content"
            >
              {{ truncate(node.asset_name, 26) }}
            </text>
            <text
              :x="(nodeLayout[node.id]?.x || 0) + 16"
              :y="(nodeLayout[node.id]?.y || 0) + 69"
              class="fill-current text-[11px] text-base-content/65"
            >
              {{ truncate(node.display_name || formatStatus(node.status), 28) }}
            </text>
            <title>{{ node.asset_name }}</title>
          </g>
        </svg>
      </div>
    </div>

    <div class="space-y-4">
      <div class="card bg-base-100 border border-base-300 shadow-sm">
        <div class="card-body">
          <h3 class="card-title text-base">{{ t('bugBounty.surface.topology.legend') }}</h3>
          <div class="flex flex-wrap gap-2">
            <span v-for="column in layout.columns" :key="column.type" class="badge badge-outline badge-lg">
              {{ formatAssetType(column.type) }}
            </span>
          </div>
        </div>
      </div>

      <div class="card bg-base-100 border border-base-300 shadow-sm">
        <div class="card-body">
          <h3 class="card-title text-base">{{ t('bugBounty.surface.topology.selectedNode') }}</h3>
          <div v-if="selectedNode" class="space-y-3">
            <div>
              <div class="badge badge-outline badge-sm">{{ formatAssetType(selectedNode.asset_type) }}</div>
              <div class="mt-2 font-mono text-xs break-all">{{ selectedNode.asset_name }}</div>
              <div class="mt-1 text-sm">{{ selectedNode.display_name || '-' }}</div>
            </div>

            <div class="grid grid-cols-2 gap-3 text-sm">
              <div class="rounded-lg bg-base-200/60 p-3">
                <div class="text-xs text-base-content/60">{{ t('bugBounty.surface.topology.incoming') }}</div>
                <div class="mt-1 font-semibold">{{ selectedNodeEdges.incoming.length }}</div>
              </div>
              <div class="rounded-lg bg-base-200/60 p-3">
                <div class="text-xs text-base-content/60">{{ t('bugBounty.surface.topology.outgoing') }}</div>
                <div class="mt-1 font-semibold">{{ selectedNodeEdges.outgoing.length }}</div>
              </div>
            </div>

            <div class="space-y-2">
              <div class="text-xs text-base-content/60">{{ t('bugBounty.surface.topology.connectedEdges') }}</div>
              <div v-for="edge in selectedNodeRelatedEdges" :key="edge.id" class="rounded-lg border border-base-300 px-3 py-2 text-sm">
                <div class="font-medium">{{ edge.relation_type }}</div>
                <div class="mt-1 text-xs text-base-content/60 break-all">
                  {{ edge.from_display_name || edge.from_asset_name }} -> {{ edge.to_display_name || edge.to_asset_name }}
                </div>
              </div>
            </div>

            <button class="btn btn-primary btn-sm" @click="$emit('open-asset', selectedNode.id)">
              {{ t('bugBounty.surface.topology.openAsset') }}
            </button>
          </div>

          <div v-else class="text-sm text-base-content/60">
            {{ t('bugBounty.surface.topology.selectedNodeHint') }}
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { SURFACE_TOPOLOGY_TYPE_ORDER } from './surfaceTopology'
import type { SurfaceTopologyEdge, SurfaceTopologyNode } from './surfaceTopology'

interface NodePosition {
  x: number
  y: number
}

const props = defineProps<{
  nodes: SurfaceTopologyNode[]
  edges: SurfaceTopologyEdge[]
  loading: boolean
  formatAssetType: (value?: string | null) => string
  formatStatus: (value?: string | null) => string
}>()

defineEmits<{
  (e: 'open-asset', assetId: string): void
}>()

const { t } = useI18n()
const selectedNodeId = ref<string | null>(null)
const nodeWidth = 210
const nodeHeight = 84

const nodeTypeOrder = new Map(SURFACE_TOPOLOGY_TYPE_ORDER.map((type, index) => [type, index]))

const groupedNodes = computed(() => {
  return props.nodes.reduce<Record<string, SurfaceTopologyNode[]>>((acc, node) => {
    const key = node.asset_type || 'unknown'
    acc[key] ??= []
    acc[key].push(node)
    return acc
  }, {})
})

const layout = computed(() => {
  const columnGap = 260
  const rowGap = 114
  const startX = 140
  const startY = 78
  const columns = Object.entries(groupedNodes.value)
    .sort(([left], [right]) => (nodeTypeOrder.get(left) ?? 999) - (nodeTypeOrder.get(right) ?? 999))
    .map(([type, nodes], index) => ({
      type,
      nodes,
      x: startX + index * columnGap + nodeWidth / 2,
    }))

  const positions: Record<string, NodePosition> = {}
  columns.forEach((column) => {
    column.nodes.forEach((node, index) => {
      positions[node.id] = {
        x: column.x - nodeWidth / 2,
        y: startY + index * rowGap,
      }
    })
  })

  const maxColumnNodes = columns.reduce((max, column) => Math.max(max, column.nodes.length), 0)
  return {
    columns,
    width: Math.max(980, startX * 2 + Math.max(columns.length - 1, 0) * columnGap + nodeWidth),
    height: Math.max(420, startY + maxColumnNodes * rowGap + 80),
    positions,
  }
})

const nodeLayout = computed(() => layout.value.positions)

const selectedNode = computed(() => props.nodes.find((node) => node.id === selectedNodeId.value) || null)

const selectedNodeEdges = computed(() => {
  if (!selectedNodeId.value) {
    return { incoming: [] as SurfaceTopologyEdge[], outgoing: [] as SurfaceTopologyEdge[] }
  }

  return props.edges.reduce(
    (acc, edge) => {
      if (edge.from_asset_id === selectedNodeId.value) acc.outgoing.push(edge)
      if (edge.to_asset_id === selectedNodeId.value) acc.incoming.push(edge)
      return acc
    },
    { incoming: [] as SurfaceTopologyEdge[], outgoing: [] as SurfaceTopologyEdge[] },
  )
})

const selectedNodeRelatedEdges = computed(() => [
  ...selectedNodeEdges.value.outgoing,
  ...selectedNodeEdges.value.incoming,
].slice(0, 8))

const connectedNodeIds = computed(() => {
  if (!selectedNodeId.value) return new Set<string>()

  return props.edges.reduce((acc, edge) => {
    if (edge.from_asset_id === selectedNodeId.value) acc.add(edge.to_asset_id)
    if (edge.to_asset_id === selectedNodeId.value) acc.add(edge.from_asset_id)
    return acc
  }, new Set<string>())
})

watch(
  () => props.nodes,
  (nodes) => {
    if (!nodes.length) {
      selectedNodeId.value = null
      return
    }

    if (!selectedNodeId.value || !nodes.some((node) => node.id === selectedNodeId.value)) {
      selectedNodeId.value = nodes[0].id
    }
  },
  { immediate: true },
)

const truncate = (value: string, maxLength: number) => {
  return value.length > maxLength ? `${value.slice(0, maxLength - 1)}…` : value
}

const edgeColor = (relationType: string) => {
  const palette = ['#0f766e', '#0369a1', '#7c3aed', '#c2410c', '#b91c1c', '#4f46e5']
  const hash = relationType.split('').reduce((sum, char) => sum + char.charCodeAt(0), 0)
  return palette[hash % palette.length]
}

const nodeFill = (node: SurfaceTopologyNode) => {
  if (selectedNodeId.value === node.id) return '#ccfbf1'
  if (connectedNodeIds.value.has(node.id)) return '#e0f2fe'
  return '#f8fafc'
}

const nodeOpacity = (node: SurfaceTopologyNode) => {
  if (!selectedNodeId.value) return 1
  return selectedNodeId.value === node.id || connectedNodeIds.value.has(node.id) ? 1 : 0.35
}

const edgeOpacity = (edge: SurfaceTopologyEdge) => {
  if (!selectedNodeId.value) return 0.55
  return edge.from_asset_id === selectedNodeId.value || edge.to_asset_id === selectedNodeId.value ? 0.95 : 0.14
}

const buildEdgePath = (edge: SurfaceTopologyEdge) => {
  const from = nodeLayout.value[edge.from_asset_id]
  const to = nodeLayout.value[edge.to_asset_id]
  if (!from || !to) return ''

  const startX = from.x + nodeWidth
  const startY = from.y + nodeHeight / 2
  const endX = to.x
  const endY = to.y + nodeHeight / 2
  const controlOffset = Math.max(Math.abs(endX - startX) * 0.45, 80)

  if (Math.abs(endX - startX) < 24) {
    const midY = (startY + endY) / 2
    return `M ${startX} ${startY} C ${startX + 80} ${startY}, ${endX + 80} ${midY}, ${endX} ${endY}`
  }

  return `M ${startX} ${startY} C ${startX + controlOffset} ${startY}, ${endX - controlOffset} ${endY}, ${endX} ${endY}`
}
</script>
