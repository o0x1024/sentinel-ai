<template>
  <div class="space-y-4">
    <div class="flex items-center justify-between">
      <div>
        <h2 class="text-xl font-semibold">ASM + 网络资产测绘</h2>
        <p class="text-sm text-base-content/60">
          基于 Program 的 surface graph，展示 overview、inventory、relations 和 discovery runs。
        </p>
      </div>
      <div class="flex items-center gap-2">
        <select v-model="selectedProgramId" class="select select-bordered select-sm">
          <option value="">全部 Program</option>
          <option v-for="program in programs" :key="program.id" :value="program.id">
            {{ program.name }}
          </option>
        </select>
        <button class="btn btn-primary btn-sm" :disabled="loading" @click="refreshAll">
          <span v-if="loading" class="loading loading-spinner loading-xs"></span>
          <i v-else class="fas fa-rotate mr-2"></i>
          刷新
        </button>
      </div>
    </div>

    <div class="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-6 gap-4">
      <div class="stat bg-base-200 rounded-lg">
        <div class="stat-title">资产总数</div>
        <div class="stat-value text-primary text-2xl">{{ overview.total_assets }}</div>
      </div>
      <div class="stat bg-base-200 rounded-lg">
        <div class="stat-title">活跃资产</div>
        <div class="stat-value text-success text-2xl">{{ overview.active_assets }}</div>
      </div>
      <div class="stat bg-base-200 rounded-lg">
        <div class="stat-title">高风险资产</div>
        <div class="stat-value text-error text-2xl">{{ overview.high_risk_assets }}</div>
      </div>
      <div class="stat bg-base-200 rounded-lg">
        <div class="stat-title">关系边</div>
        <div class="stat-value text-info text-2xl">{{ overview.total_relations }}</div>
      </div>
      <div class="stat bg-base-200 rounded-lg">
        <div class="stat-title">最近 Runs</div>
        <div class="stat-value text-secondary text-2xl">{{ overview.recent_runs }}</div>
      </div>
      <div class="stat bg-base-200 rounded-lg">
        <div class="stat-title">最近变化</div>
        <div class="stat-value text-warning text-2xl">{{ overview.recent_changes }}</div>
      </div>
    </div>

    <div class="tabs tabs-boxed">
      <a class="tab" :class="{ 'tab-active': activeTab === 'inventory' }" @click="activeTab = 'inventory'">Inventory</a>
      <a class="tab" :class="{ 'tab-active': activeTab === 'topology' }" @click="activeTab = 'topology'">Topology</a>
      <a class="tab" :class="{ 'tab-active': activeTab === 'relations' }" @click="activeTab = 'relations'">Relations</a>
      <a class="tab" :class="{ 'tab-active': activeTab === 'runs' }" @click="activeTab = 'runs'">Discovery Runs</a>
      <a class="tab" :class="{ 'tab-active': activeTab === 'types' }" @click="activeTab = 'types'">By Type</a>
    </div>

    <div v-if="activeTab === 'inventory'" class="card bg-base-100 shadow-sm border border-base-300">
      <div class="card-body">
        <div class="flex items-center justify-between">
          <h3 class="card-title text-base">Surface Inventory</h3>
          <input
            v-model="search"
            type="text"
            class="input input-bordered input-sm w-64"
            placeholder="搜索 asset_name / display_name"
          />
        </div>
        <div class="overflow-x-auto">
          <table class="table table-sm">
            <thead>
              <tr>
                <th>Type</th>
                <th>Name</th>
                <th>Display</th>
                <th>Status</th>
                <th>Exposure</th>
                <th>Source</th>
                <th>Last Seen</th>
              </tr>
            </thead>
            <tbody>
              <tr
                v-for="asset in filteredAssets"
                :key="asset.id"
                class="cursor-pointer hover"
                @click="openAssetDetail(asset.id)"
              >
                <td><span class="badge badge-outline badge-sm">{{ asset.asset_type }}</span></td>
                <td class="font-mono text-xs">{{ asset.asset_name }}</td>
                <td>{{ asset.display_name || '-' }}</td>
                <td><span class="badge badge-sm" :class="asset.status === 'active' ? 'badge-success' : 'badge-ghost'">{{ asset.status }}</span></td>
                <td>{{ asset.internet_exposure || '-' }}</td>
                <td>{{ asset.source || '-' }}</td>
                <td>{{ formatTime(asset.last_seen_at) }}</td>
              </tr>
            </tbody>
          </table>
        </div>
        <div v-if="!filteredAssets.length" class="text-sm text-base-content/60">暂无 surface inventory 数据。</div>
      </div>
    </div>

    <div v-else-if="activeTab === 'topology'" class="space-y-4">
      <div class="grid grid-cols-1 md:grid-cols-3 gap-4">
        <div class="stat bg-base-100 rounded-lg border border-base-300">
          <div class="stat-title">Topology Nodes</div>
          <div class="stat-value text-primary text-2xl">{{ topology.node_count }}</div>
        </div>
        <div class="stat bg-base-100 rounded-lg border border-base-300">
          <div class="stat-title">Topology Edges</div>
          <div class="stat-value text-info text-2xl">{{ topology.edge_count }}</div>
        </div>
        <div class="stat bg-base-100 rounded-lg border border-base-300">
          <div class="stat-title">Node Types</div>
          <div class="stat-value text-secondary text-2xl">{{ Object.keys(topology.by_type).length }}</div>
        </div>
      </div>

      <div class="card bg-base-100 shadow-sm border border-base-300">
        <div class="card-body">
          <h3 class="card-title text-base">Topology Nodes</h3>
          <div class="flex flex-wrap gap-2">
            <span v-for="(count, type) in topology.by_type" :key="type" class="badge badge-outline badge-lg">
              {{ type }}: {{ count }}
            </span>
          </div>
          <div class="grid grid-cols-1 xl:grid-cols-2 gap-3 mt-2">
            <div
              v-for="node in topology.nodes.slice(0, 24)"
              :key="node.id"
              class="rounded-lg border border-base-300 bg-base-200/50 px-3 py-2 cursor-pointer hover:border-primary/40"
              @click="openAssetDetail(node.id)"
            >
              <div class="flex items-center justify-between gap-3">
                <span class="badge badge-outline badge-sm">{{ node.asset_type }}</span>
                <span class="text-xs text-base-content/60">{{ formatTime(node.last_seen_at) }}</span>
              </div>
              <div class="mt-2 font-mono text-xs break-all">{{ node.asset_name }}</div>
              <div class="text-sm truncate">{{ node.display_name || '-' }}</div>
            </div>
          </div>
          <div v-if="!topology.nodes.length" class="text-sm text-base-content/60">暂无 topology 节点数据。</div>
        </div>
      </div>

      <div class="card bg-base-100 shadow-sm border border-base-300">
        <div class="card-body">
          <h3 class="card-title text-base">Topology Edges</h3>
          <div class="overflow-x-auto">
            <table class="table table-sm">
              <thead>
                <tr>
                  <th>Relation</th>
                  <th>From</th>
                  <th>To</th>
                  <th>Source</th>
                  <th>Last Seen</th>
                </tr>
              </thead>
              <tbody>
                <tr v-for="edge in topology.edges" :key="edge.id">
                  <td><span class="badge badge-outline badge-sm">{{ edge.relation_type }}</span></td>
                  <td>
                    <div class="text-xs text-base-content/60">{{ edge.from_asset_type }}</div>
                    <div class="font-mono text-xs break-all">{{ edge.from_display_name || edge.from_asset_name }}</div>
                  </td>
                  <td>
                    <div class="text-xs text-base-content/60">{{ edge.to_asset_type }}</div>
                    <div class="font-mono text-xs break-all">{{ edge.to_display_name || edge.to_asset_name }}</div>
                  </td>
                  <td>{{ edge.source || '-' }}</td>
                  <td>{{ formatTime(edge.last_seen_at) }}</td>
                </tr>
              </tbody>
            </table>
          </div>
          <div v-if="!topology.edges.length" class="text-sm text-base-content/60">暂无 topology 边数据。</div>
        </div>
      </div>
    </div>

    <div v-else-if="activeTab === 'relations'" class="card bg-base-100 shadow-sm border border-base-300">
      <div class="card-body">
        <h3 class="card-title text-base">Surface Relations</h3>
        <div class="overflow-x-auto">
          <table class="table table-sm">
            <thead>
              <tr>
                <th>Type</th>
                <th>From</th>
                <th>To</th>
                <th>Source</th>
                <th>Last Seen</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="relation in relations" :key="relation.id">
                <td><span class="badge badge-outline badge-sm">{{ relation.relation_type }}</span></td>
                <td class="font-mono text-xs">{{ relation.from_asset_id }}</td>
                <td class="font-mono text-xs">{{ relation.to_asset_id }}</td>
                <td>{{ relation.source || '-' }}</td>
                <td>{{ formatTime(relation.last_seen_at) }}</td>
              </tr>
            </tbody>
          </table>
        </div>
        <div v-if="!relations.length" class="text-sm text-base-content/60">暂无 relation 数据。</div>
      </div>
    </div>

    <div v-else-if="activeTab === 'runs'" class="card bg-base-100 shadow-sm border border-base-300">
      <div class="card-body">
        <h3 class="card-title text-base">Discovery Runs</h3>
        <div class="overflow-x-auto">
          <table class="table table-sm">
            <thead>
              <tr>
                <th>Status</th>
                <th>Plugin</th>
                <th>Trigger</th>
                <th>Observations</th>
                <th>Imported</th>
                <th>Changed</th>
                <th>Started</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="run in runs" :key="run.id">
                <td><span class="badge badge-sm" :class="run.status === 'completed' ? 'badge-success' : run.status === 'failed' ? 'badge-error' : 'badge-warning'">{{ run.status }}</span></td>
                <td>{{ run.plugin_id || '-' }}</td>
                <td>{{ run.trigger_source }}</td>
                <td>{{ run.observation_count || 0 }}</td>
                <td>{{ run.imported_asset_count || 0 }}</td>
                <td>{{ run.changed_asset_count || 0 }}</td>
                <td>{{ formatTime(run.started_at) }}</td>
              </tr>
            </tbody>
          </table>
        </div>
        <div v-if="!runs.length" class="text-sm text-base-content/60">暂无 discovery run 数据。</div>
      </div>
    </div>

    <div v-else class="card bg-base-100 shadow-sm border border-base-300">
      <div class="card-body">
        <h3 class="card-title text-base">Asset Types</h3>
        <div class="flex flex-wrap gap-2">
          <span v-for="(count, type) in overview.by_type" :key="type" class="badge badge-lg badge-outline">
            {{ type }}: {{ count }}
          </span>
        </div>
        <div v-if="!Object.keys(overview.by_type).length" class="text-sm text-base-content/60">
          暂无按类型聚合的数据。
        </div>
      </div>
    </div>
  </div>
  <SurfaceAssetDetailModal
    :visible="showDetailModal"
    :asset-id="selectedAssetId"
    @close="closeDetailModal"
  />
</template>

<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import SurfaceAssetDetailModal from './SurfaceAssetDetailModal.vue'

const props = defineProps<{
  programId?: string | null
  programs?: Array<{ id: string; name: string }>
}>()

const emit = defineEmits<{
  (e: 'stats-updated', stats: { total: number; active: number }): void
  (e: 'refresh'): void
}>()

const loading = ref(false)
const activeTab = ref<'inventory' | 'topology' | 'relations' | 'runs' | 'types'>('inventory')
const selectedProgramId = ref(props.programId || '')
const search = ref('')

const overview = ref({
  total_assets: 0,
  active_assets: 0,
  high_risk_assets: 0,
  by_type: {} as Record<string, number>,
  total_relations: 0,
  recent_changes: 0,
  recent_runs: 0,
})

const assets = ref<any[]>([])
const relations = ref<any[]>([])
const runs = ref<any[]>([])
const topology = ref({
  nodes: [] as any[],
  edges: [] as any[],
  by_type: {} as Record<string, number>,
  node_count: 0,
  edge_count: 0,
})
const showDetailModal = ref(false)
const selectedAssetId = ref<string | null>(null)

const filteredAssets = computed(() => {
  if (!search.value) return assets.value
  const needle = search.value.toLowerCase()
  return assets.value.filter((asset) =>
    String(asset.asset_name || '').toLowerCase().includes(needle) ||
    String(asset.display_name || '').toLowerCase().includes(needle),
  )
})

const loadAll = async () => {
  try {
    loading.value = true
    const programId = selectedProgramId.value || null
    const [overviewData, assetData, relationData, runData, topologyData] = await Promise.all([
      invoke<any>('surface_get_overview', { programId }),
      invoke<any[]>('surface_list_assets', {
        filter: { program_id: programId, asset_type: null, search: null, limit: 100, offset: 0 },
      }),
      invoke<any[]>('surface_list_relations', {
        filter: { program_id: programId, asset_id: null, relation_type: null, limit: 100 },
      }),
      invoke<any[]>('surface_list_discovery_runs', { programId, limit: 50 }),
      invoke<any>('surface_get_topology', { programId, limit: 120 }),
    ])

    overview.value = overviewData
    assets.value = assetData
    relations.value = relationData
    runs.value = runData
    topology.value = topologyData

    emit('stats-updated', {
      total: overviewData?.total_assets || 0,
      active: overviewData?.active_assets || 0,
    })
  } catch (error) {
    console.error('Failed to load surface graph data:', error)
    overview.value = {
      total_assets: 0,
      active_assets: 0,
      high_risk_assets: 0,
      by_type: {},
      total_relations: 0,
      recent_changes: 0,
      recent_runs: 0,
    }
    assets.value = []
    relations.value = []
    runs.value = []
    topology.value = {
      nodes: [],
      edges: [],
      by_type: {},
      node_count: 0,
      edge_count: 0,
    }
  } finally {
    loading.value = false
  }
}

const refreshAll = async () => {
  await loadAll()
  emit('refresh')
}

const openAssetDetail = (assetId?: string | null) => {
  if (!assetId) return
  selectedAssetId.value = assetId
  showDetailModal.value = true
}

const closeDetailModal = () => {
  showDetailModal.value = false
  selectedAssetId.value = null
}

const formatTime = (value?: string) => {
  if (!value) return '-'
  const date = new Date(value)
  if (Number.isNaN(date.getTime())) return value
  return date.toLocaleString()
}

watch(
  () => props.programId,
  (value) => {
    selectedProgramId.value = value || ''
    loadAll()
  },
)

watch(selectedProgramId, () => {
  loadAll()
})

onMounted(() => {
  loadAll()
})
</script>
