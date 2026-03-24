<template>
  <div class="space-y-4">
    <div class="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-4 gap-4">
      <div class="stat bg-base-100 rounded-lg border border-base-300">
        <div class="stat-title">{{ t('bugBounty.surface.stats.topologyNodes') }}</div>
        <div class="stat-value text-primary text-2xl">{{ topology.node_count }}</div>
      </div>
      <div class="stat bg-base-100 rounded-lg border border-base-300">
        <div class="stat-title">{{ t('bugBounty.surface.topology.currentNodes') }}</div>
        <div class="stat-value text-secondary text-2xl">{{ topology.visible_node_count }}</div>
      </div>
      <div class="stat bg-base-100 rounded-lg border border-base-300">
        <div class="stat-title">{{ t('bugBounty.surface.stats.topologyEdges') }}</div>
        <div class="stat-value text-info text-2xl">{{ topology.edge_count }}</div>
      </div>
      <div class="stat bg-base-100 rounded-lg border border-base-300">
        <div class="stat-title">{{ t('bugBounty.surface.topology.currentEdges') }}</div>
        <div class="stat-value text-accent text-2xl">{{ topology.visible_edge_count }}</div>
      </div>
    </div>

    <div class="card bg-base-100 border border-base-300 shadow-sm">
      <div class="card-body space-y-4">
        <div class="flex flex-col gap-3 xl:flex-row xl:items-center xl:justify-between">
          <div>
            <h3 class="card-title text-base">{{ t('bugBounty.surface.tabs.topology') }}</h3>
            <p class="text-sm text-base-content/60">{{ t('bugBounty.surface.topology.description') }}</p>
          </div>

          <div class="flex flex-wrap items-center gap-2">
            <div class="join">
              <button
                class="join-item btn btn-sm"
                :class="viewMode === 'list' ? 'btn-primary' : 'btn-ghost'"
                @click="viewMode = 'list'"
              >
                {{ t('bugBounty.surface.topology.viewModes.list') }}
              </button>
              <button
                class="join-item btn btn-sm"
                :class="viewMode === 'graph' ? 'btn-primary' : 'btn-ghost'"
                @click="viewMode = 'graph'"
              >
                {{ t('bugBounty.surface.topology.viewModes.graph') }}
              </button>
            </div>

            <button class="btn btn-sm btn-ghost" :disabled="loading" @click="loadTopology">
              <span v-if="loading" class="loading loading-spinner loading-xs"></span>
              <i v-else class="fas fa-rotate"></i>
            </button>
          </div>
        </div>

        <div class="grid grid-cols-1 xl:grid-cols-2 gap-4">
          <div class="rounded-xl border border-base-300 bg-base-200/40 p-4">
            <div class="flex items-center justify-between gap-3">
              <div>
                <div class="text-sm font-medium">{{ t('bugBounty.surface.topology.nodePage') }}</div>
                <div class="text-xs text-base-content/60">{{ nodeRangeText }}</div>
              </div>
              <select v-model.number="nodePageSize" class="select select-bordered select-sm w-28" @change="changeNodePageSize">
                <option v-for="size in nodePageSizes" :key="size" :value="size">{{ size }}/{{ t('bugBounty.surface.topology.pageSize') }}</option>
              </select>
            </div>

            <div class="mt-3 flex items-center justify-between gap-2">
              <button class="btn btn-sm" :disabled="loading || nodePage <= 1" @click="goToPreviousNodePage">
                {{ t('common.previous') }}
              </button>
              <span class="text-sm">{{ t('bugBounty.surface.inventory.pageInfo', { page: nodePage, total: nodePageCount }) }}</span>
              <button class="btn btn-sm" :disabled="loading || !topology.has_more_nodes" @click="goToNextNodePage">
                {{ t('common.next') }}
              </button>
            </div>
          </div>

          <div class="rounded-xl border border-base-300 bg-base-200/40 p-4">
            <div class="flex items-center justify-between gap-3">
              <div>
                <div class="text-sm font-medium">{{ t('bugBounty.surface.topology.edgePage') }}</div>
                <div class="text-xs text-base-content/60">{{ edgeRangeText }}</div>
              </div>
              <select v-model.number="edgePageSize" class="select select-bordered select-sm w-28" @change="changeEdgePageSize">
                <option v-for="size in edgePageSizes" :key="size" :value="size">{{ size }}/{{ t('bugBounty.surface.topology.pageSize') }}</option>
              </select>
            </div>

            <div class="mt-3 flex items-center justify-between gap-2">
              <button class="btn btn-sm" :disabled="loading || edgePage <= 1" @click="goToPreviousEdgePage">
                {{ t('common.previous') }}
              </button>
              <span class="text-sm">{{ t('bugBounty.surface.inventory.pageInfo', { page: edgePage, total: edgePageCount }) }}</span>
              <button class="btn btn-sm" :disabled="loading || !topology.has_more_edges" @click="goToNextEdgePage">
                {{ t('common.next') }}
              </button>
            </div>
          </div>
        </div>

        <div class="rounded-xl border border-dashed border-base-300 bg-base-200/20 px-4 py-3 text-sm text-base-content/70">
          {{ viewMode === 'graph' ? t('bugBounty.surface.topology.graphHint') : t('bugBounty.surface.topology.listHint') }}
        </div>

        <SurfaceTopologyListView
          v-if="viewMode === 'list'"
          :topology="topology"
          :loading="loading"
          :format-asset-type="formatAssetType"
          :format-status="formatStatus"
          :format-time="formatTime"
          @open-asset="$emit('open-asset', $event)"
        />
        <SurfaceTopologyGraphView
          v-else
          :nodes="topology.nodes"
          :edges="topology.edges"
          :loading="loading"
          :format-asset-type="formatAssetType"
          :format-status="formatStatus"
          @open-asset="$emit('open-asset', $event)"
        />
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useI18n } from 'vue-i18n'
import SurfaceTopologyGraphView from './SurfaceTopologyGraphView.vue'
import SurfaceTopologyListView from './SurfaceTopologyListView.vue'
import {
  SURFACE_TOPOLOGY_EDGE_PAGE_SIZES,
  SURFACE_TOPOLOGY_NODE_PAGE_SIZES,
  createEmptySurfaceTopology,
} from './surfaceTopology'
import type { SurfaceTopologyResponse } from './surfaceTopology'

const props = defineProps<{
  programId?: string | null
  refreshToken?: number
}>()

defineEmits<{
  (e: 'open-asset', assetId: string): void
}>()

const { t } = useI18n()
const loading = ref(false)
const viewMode = ref<'list' | 'graph'>('list')
const nodePage = ref(1)
const edgePage = ref(1)
const nodePageSize = ref(SURFACE_TOPOLOGY_NODE_PAGE_SIZES[0])
const edgePageSize = ref(SURFACE_TOPOLOGY_EDGE_PAGE_SIZES[1])
const topology = ref<SurfaceTopologyResponse>(createEmptySurfaceTopology())
const nodePageSizes = SURFACE_TOPOLOGY_NODE_PAGE_SIZES
const edgePageSizes = SURFACE_TOPOLOGY_EDGE_PAGE_SIZES

const nodePageCount = computed(() => Math.max(1, Math.ceil(topology.value.node_count / nodePageSize.value)))
const edgePageCount = computed(() => Math.max(1, Math.ceil(topology.value.visible_edge_count / edgePageSize.value)))

const nodeRangeText = computed(() => {
  if (!topology.value.node_count) return t('bugBounty.surface.topology.emptyNodes')
  const start = topology.value.node_offset + 1
  const end = topology.value.node_offset + topology.value.nodes.length
  return t('bugBounty.surface.topology.pageSummary', { start, end, total: topology.value.node_count })
})

const edgeRangeText = computed(() => {
  if (!topology.value.visible_edge_count) {
    return t('bugBounty.surface.topology.edgeScope', { count: 0, total: topology.value.edge_count })
  }

  const start = topology.value.edge_offset + 1
  const end = topology.value.edge_offset + topology.value.edges.length
  return t('bugBounty.surface.topology.edgePageSummary', {
    start,
    end,
    count: topology.value.visible_edge_count,
    total: topology.value.edge_count,
  })
})

const loadTopology = async () => {
  try {
    loading.value = true
    topology.value = await invoke<SurfaceTopologyResponse>('surface_get_topology', {
      programId: props.programId || null,
      nodeLimit: nodePageSize.value,
      nodeOffset: (nodePage.value - 1) * nodePageSize.value,
      edgeLimit: edgePageSize.value,
      edgeOffset: (edgePage.value - 1) * edgePageSize.value,
    })
  } catch (error) {
    console.error('Failed to load surface topology:', error)
    topology.value = createEmptySurfaceTopology()
  } finally {
    loading.value = false
  }
}

const resetPaging = () => {
  nodePage.value = 1
  edgePage.value = 1
}

const changeNodePageSize = async () => {
  nodePage.value = 1
  edgePage.value = 1
  await loadTopology()
}

const changeEdgePageSize = async () => {
  edgePage.value = 1
  await loadTopology()
}

const goToPreviousNodePage = async () => {
  if (nodePage.value <= 1 || loading.value) return
  nodePage.value -= 1
  edgePage.value = 1
  await loadTopology()
}

const goToNextNodePage = async () => {
  if (!topology.value.has_more_nodes || loading.value) return
  nodePage.value += 1
  edgePage.value = 1
  await loadTopology()
}

const goToPreviousEdgePage = async () => {
  if (edgePage.value <= 1 || loading.value) return
  edgePage.value -= 1
  await loadTopology()
}

const goToNextEdgePage = async () => {
  if (!topology.value.has_more_edges || loading.value) return
  edgePage.value += 1
  await loadTopology()
}

const formatTime = (value?: string | null) => {
  if (!value) return '-'
  const date = new Date(value)
  if (Number.isNaN(date.getTime())) return value
  return date.toLocaleString()
}

const formatStatus = (value?: string | null) => {
  if (!value) return '-'
  const key = `bugBounty.surface.status.${value}`
  const translated = t(key)
  return translated === key ? value : translated
}

const formatAssetType = (value?: string | null) => {
  if (!value) return '-'
  const key = `bugBounty.surface.assetTypes.${value}`
  const translated = t(key)
  return translated === key ? value : translated
}

watch(
  [() => props.programId, () => props.refreshToken],
  async () => {
    resetPaging()
    await loadTopology()
  },
)

onMounted(() => {
  loadTopology()
})
</script>
