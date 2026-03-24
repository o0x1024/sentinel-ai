<template>
  <div class="space-y-4">
    <div class="card bg-base-100 border border-base-300 shadow-sm">
      <div class="card-body">
        <div class="flex items-center justify-between gap-3">
          <h3 class="card-title text-base">{{ t('bugBounty.surface.topology.nodesTitle') }}</h3>
          <div class="text-xs text-base-content/60">
            {{ t('bugBounty.surface.topology.showing', { count: topology.nodes.length }) }}
          </div>
        </div>

        <div class="flex flex-wrap gap-2">
          <span v-for="(count, type) in topology.by_type" :key="type" class="badge badge-outline badge-lg">
            {{ formatAssetType(type) }}: {{ count }}
          </span>
        </div>

        <div v-if="loading" class="py-8 text-center text-sm text-base-content/60">
          <span class="loading loading-spinner loading-sm mr-2"></span>
          {{ t('common.loading') }}
        </div>

        <div v-else-if="!topology.nodes.length" class="text-sm text-base-content/60">
          {{ t('bugBounty.surface.topology.emptyNodes') }}
        </div>

        <div v-else class="grid grid-cols-1 xl:grid-cols-2 gap-3">
          <div
            v-for="node in topology.nodes"
            :key="node.id"
            class="rounded-xl border border-base-300 bg-base-200/50 p-4"
          >
            <div class="flex items-start justify-between gap-3">
              <div class="min-w-0">
                <div class="flex items-center gap-2">
                  <span class="badge badge-outline badge-sm">{{ formatAssetType(node.asset_type) }}</span>
                  <span class="badge badge-sm" :class="node.status === 'active' ? 'badge-success' : 'badge-ghost'">
                    {{ formatStatus(node.status) }}
                  </span>
                </div>
                <div class="mt-2 font-mono text-xs break-all">{{ node.asset_name }}</div>
                <div class="mt-1 text-sm truncate">{{ node.display_name || '-' }}</div>
              </div>
              <button class="btn btn-ghost btn-xs" @click="toggleExpandedNode(node.id)">
                {{ expandedNodeId === node.id ? t('bugBounty.surface.topology.collapse') : t('bugBounty.surface.topology.expand') }}
              </button>
            </div>

            <div class="mt-3 flex items-center justify-between gap-3 text-xs text-base-content/60">
              <span>{{ formatTime(node.last_seen_at) }}</span>
              <button class="btn btn-primary btn-xs" @click="$emit('open-asset', node.id)">
                {{ t('bugBounty.surface.topology.openAsset') }}
              </button>
            </div>

            <div v-if="expandedNodeId === node.id" class="mt-3 grid grid-cols-2 gap-3 rounded-lg bg-base-100 p-3 text-sm">
              <div>
                <div class="text-xs text-base-content/60">{{ t('bugBounty.surface.topology.risk') }}</div>
                <div class="mt-1">{{ node.risk_level || '-' }}</div>
              </div>
              <div>
                <div class="text-xs text-base-content/60">{{ t('bugBounty.surface.columns.source') }}</div>
                <div class="mt-1 break-all">{{ node.source || '-' }}</div>
              </div>
              <div>
                <div class="text-xs text-base-content/60">{{ t('bugBounty.surface.topology.incoming') }}</div>
                <div class="mt-1">{{ degreeByNode[node.id]?.incoming || 0 }}</div>
              </div>
              <div>
                <div class="text-xs text-base-content/60">{{ t('bugBounty.surface.topology.outgoing') }}</div>
                <div class="mt-1">{{ degreeByNode[node.id]?.outgoing || 0 }}</div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>

    <div class="card bg-base-100 border border-base-300 shadow-sm">
      <div class="card-body">
        <div class="flex items-center justify-between gap-3">
          <h3 class="card-title text-base">{{ t('bugBounty.surface.topology.edgesTitle') }}</h3>
          <div class="text-xs text-base-content/60">
            {{ t('bugBounty.surface.topology.edgeScope', { count: topology.visible_edge_count, total: topology.edge_count }) }}
          </div>
        </div>

        <div class="overflow-x-auto">
          <table class="table table-sm">
            <thead>
              <tr>
                <th>{{ t('bugBounty.surface.columns.relationType') }}</th>
                <th>{{ t('bugBounty.surface.columns.from') }}</th>
                <th>{{ t('bugBounty.surface.columns.to') }}</th>
                <th>{{ t('bugBounty.surface.columns.source') }}</th>
                <th>{{ t('bugBounty.surface.columns.lastSeen') }}</th>
              </tr>
            </thead>
            <tbody>
              <tr v-if="loading">
                <td colspan="5" class="py-8 text-center text-sm text-base-content/60">
                  <span class="loading loading-spinner loading-sm mr-2"></span>
                  {{ t('common.loading') }}
                </td>
              </tr>
              <tr v-for="edge in topology.edges" :key="edge.id">
                <td><span class="badge badge-outline badge-sm">{{ edge.relation_type }}</span></td>
                <td>
                  <div class="text-xs text-base-content/60">{{ formatAssetType(edge.from_asset_type) }}</div>
                  <button class="font-mono text-xs break-all text-left hover:underline" @click="$emit('open-asset', edge.from_asset_id)">
                    {{ edge.from_display_name || edge.from_asset_name }}
                  </button>
                </td>
                <td>
                  <div class="text-xs text-base-content/60">{{ formatAssetType(edge.to_asset_type) }}</div>
                  <button class="font-mono text-xs break-all text-left hover:underline" @click="$emit('open-asset', edge.to_asset_id)">
                    {{ edge.to_display_name || edge.to_asset_name }}
                  </button>
                </td>
                <td>{{ edge.source || '-' }}</td>
                <td>{{ formatTime(edge.last_seen_at) }}</td>
              </tr>
            </tbody>
          </table>
        </div>

        <div v-if="!loading && !topology.edges.length" class="text-sm text-base-content/60">
          {{ t('bugBounty.surface.topology.emptyEdges') }}
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import type { SurfaceTopologyResponse } from './surfaceTopology'

const props = defineProps<{
  topology: SurfaceTopologyResponse
  loading: boolean
  formatAssetType: (value?: string | null) => string
  formatStatus: (value?: string | null) => string
  formatTime: (value?: string | null) => string
}>()

defineEmits<{
  (e: 'open-asset', assetId: string): void
}>()

const { t } = useI18n()
const expandedNodeId = ref<string | null>(null)

const degreeByNode = computed(() => {
  return props.topology.edges.reduce<Record<string, { incoming: number; outgoing: number }>>((acc, edge) => {
    acc[edge.from_asset_id] ??= { incoming: 0, outgoing: 0 }
    acc[edge.to_asset_id] ??= { incoming: 0, outgoing: 0 }
    acc[edge.from_asset_id].outgoing += 1
    acc[edge.to_asset_id].incoming += 1
    return acc
  }, {})
})

const toggleExpandedNode = (nodeId: string) => {
  expandedNodeId.value = expandedNodeId.value === nodeId ? null : nodeId
}
</script>
