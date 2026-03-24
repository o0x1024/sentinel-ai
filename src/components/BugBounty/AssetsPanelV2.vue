<template>
  <div class="space-y-4">
    <div class="flex items-center justify-between">
      <div>
        <h2 class="text-xl font-semibold">{{ t('bugBounty.surface.title') }}</h2>
        <p class="text-sm text-base-content/60">
          {{ t('bugBounty.surface.description') }}
        </p>
      </div>
      <div class="flex items-center gap-2">
        <select v-model="selectedProgramId" class="select select-bordered select-sm">
          <option value="">{{ t('bugBounty.surface.allPrograms') }}</option>
          <option v-for="program in programs" :key="program.id" :value="program.id">
            {{ program.name }}
          </option>
        </select>
        <button class="btn btn-primary btn-sm" :disabled="loading" @click="refreshAll">
          <span v-if="loading" class="loading loading-spinner loading-xs"></span>
          <i v-else class="fas fa-rotate mr-2"></i>
          {{ t('bugBounty.surface.refresh') }}
        </button>
      </div>
    </div>

    <div class="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-6 gap-4">
      <div class="stat bg-base-200 rounded-lg">
        <div class="stat-title">{{ t('bugBounty.surface.stats.totalAssets') }}</div>
        <button class="stat-value text-primary text-2xl text-left hover:underline" @click="openAssetStatsModal('all')">
          {{ overview.total_assets }}
        </button>
      </div>
      <div class="stat bg-base-200 rounded-lg">
        <div class="stat-title">{{ t('bugBounty.surface.stats.activeAssets') }}</div>
        <button class="stat-value text-success text-2xl text-left hover:underline" @click="openAssetStatsModal('active')">
          {{ overview.active_assets }}
        </button>
      </div>
      <div class="stat bg-base-200 rounded-lg">
        <div class="stat-title">{{ t('bugBounty.surface.stats.highRiskAssets') }}</div>
        <div class="stat-value text-error text-2xl">{{ overview.high_risk_assets }}</div>
      </div>
      <div class="stat bg-base-200 rounded-lg">
        <div class="stat-title">{{ t('bugBounty.surface.stats.totalRelations') }}</div>
        <div class="stat-value text-info text-2xl">{{ overview.total_relations }}</div>
      </div>
      <div class="stat bg-base-200 rounded-lg">
        <div class="stat-title">{{ t('bugBounty.surface.stats.recentRuns') }}</div>
        <div class="stat-value text-secondary text-2xl">{{ overview.recent_runs }}</div>
      </div>
      <div class="stat bg-base-200 rounded-lg">
        <div class="stat-title">{{ t('bugBounty.surface.stats.recentChanges') }}</div>
        <div class="stat-value text-warning text-2xl">{{ overview.recent_changes }}</div>
      </div>
    </div>

    <div class="tabs tabs-boxed">
      <a class="tab" :class="{ 'tab-active': activeTab === 'inventory' }" @click="activeTab = 'inventory'">{{ t('bugBounty.surface.tabs.inventory') }}</a>
      <a class="tab" :class="{ 'tab-active': activeTab === 'topology' }" @click="activeTab = 'topology'">{{ t('bugBounty.surface.tabs.topology') }}</a>
      <a class="tab" :class="{ 'tab-active': activeTab === 'runs' }" @click="activeTab = 'runs'">{{ t('bugBounty.surface.tabs.runs') }}</a>
    </div>

    <div v-if="activeTab === 'inventory'" class="card bg-base-100 shadow-sm border border-base-300">
      <div class="card-body">
        <div class="flex flex-col gap-3 xl:flex-row xl:items-center xl:justify-between">
          <h3 class="card-title text-base">{{ t('bugBounty.surface.inventory.title') }}</h3>
          <div class="flex flex-col gap-2 sm:flex-row sm:flex-wrap sm:items-center">
            <select v-model="assetTypeFilter" class="select select-bordered select-sm">
              <option value="">{{ t('bugBounty.surface.inventory.allTypes') }}</option>
              <option v-for="option in assetTypeOptions" :key="option.type" :value="option.type">
                {{ formatAssetType(option.type) }} ({{ option.count }})
              </option>
            </select>
            <select v-model="statusFilter" class="select select-bordered select-sm">
              <option value="">{{ t('bugBounty.filter.allStatuses') }}</option>
              <option value="active">{{ formatStatus('active') }}</option>
              <option value="inactive">{{ formatStatus('inactive') }}</option>
              <option value="unknown">{{ formatStatus('unknown') }}</option>
            </select>
            <input
              v-model="search"
              type="text"
              class="input input-bordered input-sm w-full sm:w-72"
              :placeholder="t('bugBounty.surface.inventory.searchPlaceholder')"
            />
          </div>
        </div>
        <div class="overflow-x-auto">
          <table class="table table-sm">
            <thead>
              <tr>
                <th v-for="column in inventoryColumns" :key="column.key">
                  {{ column.label }}
                </th>
              </tr>
            </thead>
            <tbody>
              <tr
                v-for="item in inventoryItems"
                :key="item.asset.id"
                class="cursor-pointer hover"
                @click="openAssetDetail(item.asset.id)"
              >
                <td
                  v-for="column in inventoryColumns"
                  :key="column.key"
                  :class="column.mono ? 'font-mono text-xs break-all' : ''"
                >
                  <span
                    v-if="column.badge === 'type'"
                    class="badge badge-outline badge-sm"
                  >
                    {{ formatAssetType(item.asset.asset_type) }}
                  </span>
                  <span
                    v-else-if="column.badge === 'status'"
                    class="inline-flex"
                    :title="formatStatus(item.asset.status)"
                    :aria-label="formatStatus(item.asset.status)"
                  >
                    <span
                      class="inline-block h-3 w-3 rounded-full"
                      :class="getStatusIndicatorClass(item.asset.status)"
                    ></span>
                    <span class="sr-only">{{ formatStatus(item.asset.status) }}</span>
                  </span>
                  <span v-else>
                    {{ formatInventoryValue(item, column) }}
                  </span>
                </td>
              </tr>
              <tr v-if="inventoryLoading">
                <td :colspan="inventoryColumns.length" class="py-8 text-center text-sm text-base-content/60">
                  <span class="loading loading-spinner loading-sm mr-2"></span>
                  {{ t('common.loading') }}
                </td>
              </tr>
            </tbody>
          </table>
        </div>
        <div v-if="!inventoryLoading && !assets.length" class="text-sm text-base-content/60">{{ t('bugBounty.surface.inventory.empty') }}</div>
        <div v-if="inventoryTotal > 0" class="flex flex-col gap-3 pt-2 xl:flex-row xl:items-center xl:justify-between">
          <div class="flex items-center gap-2 text-sm">
            <span class="text-base-content/70">{{ t('bugBounty.surface.inventory.pageSizeLabel') }}</span>
            <select v-model.number="inventoryPageSize" class="select select-bordered select-sm">
              <option v-for="size in inventoryPageSizeOptions" :key="size" :value="size">
                {{ size }}
              </option>
            </select>
          </div>

          <div class="flex flex-col gap-2 sm:flex-row sm:flex-wrap sm:items-center sm:justify-end">
            <div class="join">
              <button
                class="join-item btn btn-sm"
                :disabled="inventoryPage <= 1 || inventoryLoading"
                @click="goToFirstInventoryPage"
              >
                {{ t('bugBounty.surface.inventory.firstPage') }}
              </button>
              <button
                class="join-item btn btn-sm"
                :disabled="inventoryPage <= 1 || inventoryLoading"
                @click="goToPreviousInventoryPage"
              >
                {{ t('common.previous') }}
              </button>
              <button class="join-item btn btn-sm">
                {{ t('bugBounty.surface.inventory.pageInfo', { page: inventoryPage, total: inventoryPageCount }) }}
              </button>
              <button
                class="join-item btn btn-sm"
                :disabled="inventoryPage >= inventoryPageCount || inventoryLoading"
                @click="goToNextInventoryPage"
              >
                {{ t('common.next') }}
              </button>
              <button
                class="join-item btn btn-sm"
                :disabled="inventoryPage >= inventoryPageCount || inventoryLoading"
                @click="goToLastInventoryPage"
              >
                {{ t('bugBounty.surface.inventory.lastPage') }}
              </button>
            </div>

            <div class="flex items-center gap-2">
              <input
                v-model="inventoryPageInput"
                type="number"
                min="1"
                :max="inventoryPageCount"
                class="input input-bordered input-sm w-24"
                :placeholder="t('bugBounty.surface.inventory.jumpPlaceholder')"
                @keyup.enter="applyInventoryPageJump"
              />
              <button
                class="btn btn-sm btn-outline"
                :disabled="inventoryLoading"
                @click="applyInventoryPageJump"
              >
                {{ t('bugBounty.surface.inventory.jump') }}
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>

    <SurfaceTopologyPanel
      v-else-if="activeTab === 'topology'"
      :program-id="selectedProgramId || null"
      :refresh-token="topologyRefreshToken"
      @open-asset="openAssetDetail"
    />

    <div v-else-if="activeTab === 'runs'" class="card bg-base-100 shadow-sm border border-base-300">
      <div class="card-body">
        <h3 class="card-title text-base">{{ t('bugBounty.surface.runs.title') }}</h3>
        <div class="overflow-x-auto">
          <table class="table table-sm">
            <thead>
              <tr>
                <th>{{ t('bugBounty.surface.columns.status') }}</th>
                <th>{{ t('bugBounty.surface.columns.plugin') }}</th>
                <th>{{ t('bugBounty.surface.columns.trigger') }}</th>
                <th>{{ t('bugBounty.surface.columns.observations') }}</th>
                <th>{{ t('bugBounty.surface.columns.imported') }}</th>
                <th>{{ t('bugBounty.surface.columns.changed') }}</th>
                <th>{{ t('bugBounty.surface.columns.started') }}</th>
              </tr>
            </thead>
            <tbody>
              <tr
                v-for="run in runs"
                :key="run.id"
                class="cursor-pointer hover"
                @click="openRunDetail(run.id)"
              >
                <td><span class="badge badge-sm" :class="run.status === 'completed' ? 'badge-success' : run.status === 'failed' ? 'badge-error' : 'badge-warning'">{{ formatStatus(run.status) }}</span></td>
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
        <div v-if="!runs.length" class="text-sm text-base-content/60">{{ t('bugBounty.surface.runs.empty') }}</div>
      </div>
    </div>

  </div>
  <SurfaceAssetListModal
    :visible="showAssetStatsModal"
    :title="assetStatsModalTitle"
    :program-id="selectedProgramId || null"
    :status-filter="assetStatsFilter"
    @close="closeAssetStatsModal"
  />
  <SurfaceDiscoveryRunDetailModal
    :visible="showRunDetailModal"
    :run-id="selectedRunId"
    @close="closeRunDetail"
    @open-asset="openAssetDetail"
  />
  <SurfaceAssetDetailModal
    :visible="showDetailModal"
    :asset-id="selectedAssetId"
    @close="closeDetailModal"
  />
</template>

<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useI18n } from 'vue-i18n'
import SurfaceAssetListModal from './SurfaceAssetListModal.vue'
import SurfaceAssetDetailModal from './SurfaceAssetDetailModal.vue'
import SurfaceDiscoveryRunDetailModal from './SurfaceDiscoveryRunDetailModal.vue'
import SurfaceTopologyPanel from './SurfaceTopologyPanel.vue'

const props = defineProps<{
  programId?: string | null
  programs?: Array<{ id: string; name: string }>
}>()

const emit = defineEmits<{
  (e: 'stats-updated', stats: { total: number; active: number }): void
  (e: 'refresh'): void
}>()

const { t } = useI18n()

const loading = ref(false)
const inventoryLoading = ref(false)
const activeTab = ref<'inventory' | 'topology' | 'runs'>('inventory')
const selectedProgramId = ref(props.programId || '')
const search = ref('')
const assetTypeFilter = ref('')
const statusFilter = ref('')
const inventoryPage = ref(1)
const inventoryHasNext = ref(false)
const inventoryTotal = ref(0)
const inventoryPageSize = ref(10)
const inventoryPageInput = ref('1')
const inventoryPageSizeOptions = [10, 20, 50, 100]
let searchDebounceTimer: ReturnType<typeof setTimeout> | null = null
const topologyRefreshToken = ref(0)

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
const inventoryItems = ref<any[]>([])
const runs = ref<any[]>([])
const showDetailModal = ref(false)
const selectedAssetId = ref<string | null>(null)
const showRunDetailModal = ref(false)
const selectedRunId = ref<string | null>(null)
const showAssetStatsModal = ref(false)
const assetStatsFilter = ref<'all' | 'active'>('all')

const assetTypeOptions = computed(() => {
  return Object.entries(overview.value.by_type || {})
    .filter(([, count]) => Number(count) > 0)
    .sort((a, b) => Number(b[1]) - Number(a[1]))
    .map(([type, count]) => ({ type, count: Number(count) }))
})

const inventoryPageCount = computed(() => Math.max(1, Math.ceil(inventoryTotal.value / inventoryPageSize.value)))

const inventoryColumns = computed(() => {
  const generic = [
    { key: 'type', label: t('bugBounty.surface.columns.type'), badge: 'type' },
    { key: 'name', label: t('bugBounty.surface.columns.name'), valueKey: 'asset_name', mono: true },
    { key: 'display', label: t('bugBounty.surface.columns.display'), valueKey: 'display_name' },
    { key: 'status', label: t('bugBounty.surface.columns.status'), badge: 'status' },
    { key: 'exposure', label: t('bugBounty.surface.columns.exposure'), valueKey: 'internet_exposure' },
    { key: 'source', label: t('bugBounty.surface.columns.source'), valueKey: 'source' },
    { key: 'lastSeen', label: t('bugBounty.surface.columns.lastSeen'), valueKey: 'last_seen_at', formatter: 'time' },
  ]

  const typedColumns: Record<string, Array<any>> = {
    org: [
      { key: 'name', label: t('bugBounty.surface.columns.name'), valueKey: 'asset_name', mono: true },
      { key: 'org_name', label: t('bugBounty.surface.inventory.fields.orgName'), detailKey: 'org_name' },
      { key: 'business_line', label: t('bugBounty.surface.inventory.fields.businessLine'), detailKey: 'business_line' },
      { key: 'importance_level', label: t('bugBounty.surface.inventory.fields.importanceLevel'), detailKey: 'importance_level' },
      { key: 'status', label: t('bugBounty.surface.columns.status'), badge: 'status' },
      { key: 'lastSeen', label: t('bugBounty.surface.columns.lastSeen'), valueKey: 'last_seen_at', formatter: 'time' },
    ],
    domain: [
      { key: 'name', label: t('bugBounty.surface.columns.name'), valueKey: 'asset_name', mono: true },
      { key: 'root_domain', label: t('bugBounty.surface.inventory.fields.rootDomain'), detailKey: 'root_domain' },
      { key: 'record_type', label: t('bugBounty.surface.inventory.fields.recordType'), detailKey: 'record_type' },
      { key: 'record_value', label: t('bugBounty.surface.inventory.fields.recordValue'), detailKey: 'record_value', mono: true },
      { key: 'registrar', label: t('bugBounty.surface.inventory.fields.registrar'), detailKey: 'registrar' },
      { key: 'status', label: t('bugBounty.surface.columns.status'), badge: 'status' },
      { key: 'lastSeen', label: t('bugBounty.surface.columns.lastSeen'), valueKey: 'last_seen_at', formatter: 'time' },
    ],
    ip: [
      { key: 'name', label: t('bugBounty.surface.columns.name'), valueKey: 'asset_name', mono: true },
      { key: 'cidr', label: t('bugBounty.surface.inventory.fields.cidr'), detailKey: 'cidr' },
      { key: 'asn', label: t('bugBounty.surface.inventory.fields.asn'), detailKey: 'asn' },
      { key: 'cloud_provider', label: t('bugBounty.surface.inventory.fields.cloudProvider'), detailKey: 'cloud_provider' },
      { key: 'network_boundary_type', label: t('bugBounty.surface.inventory.fields.boundaryType'), detailKey: 'network_boundary_type' },
      { key: 'status', label: t('bugBounty.surface.columns.status'), badge: 'status' },
      { key: 'lastSeen', label: t('bugBounty.surface.columns.lastSeen'), valueKey: 'last_seen_at', formatter: 'time' },
    ],
    host: [
      { key: 'name', label: t('bugBounty.surface.columns.name'), valueKey: 'asset_name', mono: true },
      { key: 'fqdn', label: t('bugBounty.surface.inventory.fields.fqdn'), detailKey: 'fqdn', mono: true },
      { key: 'operating_system', label: t('bugBounty.surface.inventory.fields.operatingSystem'), detailKey: 'operating_system' },
      { key: 'device_type', label: t('bugBounty.surface.inventory.fields.deviceType'), detailKey: 'device_type' },
      { key: 'region_or_datacenter', label: t('bugBounty.surface.inventory.fields.regionOrDatacenter'), detailKey: 'region_or_datacenter' },
      { key: 'last_online_at', label: t('bugBounty.surface.inventory.fields.lastOnlineAt'), detailKey: 'last_online_at', formatter: 'time' },
    ],
    port: [
      { key: 'name', label: t('bugBounty.surface.columns.name'), valueKey: 'asset_name', mono: true },
      { key: 'ip_address', label: t('bugBounty.surface.inventory.fields.ipAddress'), detailKey: 'ip_address', mono: true },
      { key: 'port_number', label: t('bugBounty.surface.inventory.fields.portNumber'), detailKey: 'port_number' },
      { key: 'transport_protocol', label: t('bugBounty.surface.inventory.fields.transportProtocol'), detailKey: 'transport_protocol' },
      { key: 'port_state', label: t('bugBounty.surface.inventory.fields.portState'), detailKey: 'port_state' },
      { key: 'lastSeen', label: t('bugBounty.surface.columns.lastSeen'), valueKey: 'last_seen_at', formatter: 'time' },
    ],
    service: [
      { key: 'name', label: t('bugBounty.surface.columns.name'), valueKey: 'asset_name', mono: true },
      { key: 'application_service_name', label: t('bugBounty.surface.inventory.fields.applicationServiceName'), detailKey: 'application_service_name' },
      { key: 'port_number', label: t('bugBounty.surface.inventory.fields.portNumber'), detailKey: 'port_number' },
      { key: 'product_name', label: t('bugBounty.surface.inventory.fields.productName'), detailKey: 'product_name' },
      { key: 'version', label: t('bugBounty.surface.inventory.fields.version'), detailKey: 'version' },
      { key: 'auth_type', label: t('bugBounty.surface.inventory.fields.authType'), detailKey: 'auth_type' },
      { key: 'lastSeen', label: t('bugBounty.surface.columns.lastSeen'), valueKey: 'last_seen_at', formatter: 'time' },
    ],
    web: [
      { key: 'name', label: t('bugBounty.surface.columns.name'), valueKey: 'asset_name', mono: true },
      { key: 'canonical_url', label: t('bugBounty.surface.inventory.fields.canonicalUrl'), detailKey: 'canonical_url', mono: true },
      { key: 'site_title', label: t('bugBounty.surface.inventory.fields.siteTitle'), detailKey: 'site_title' },
      { key: 'http_status_code', label: t('bugBounty.surface.inventory.fields.httpStatusCode'), detailKey: 'http_status_code' },
      { key: 'framework', label: t('bugBounty.surface.inventory.fields.framework'), detailKey: 'framework' },
      { key: 'business_type', label: t('bugBounty.surface.inventory.fields.businessType'), detailKey: 'business_type' },
      { key: 'lastSeen', label: t('bugBounty.surface.columns.lastSeen'), valueKey: 'last_seen_at', formatter: 'time' },
    ],
    certificate: [
      { key: 'name', label: t('bugBounty.surface.columns.name'), valueKey: 'asset_name', mono: true },
      { key: 'subject', label: t('bugBounty.surface.inventory.fields.subject'), detailKey: 'subject' },
      { key: 'issuer', label: t('bugBounty.surface.inventory.fields.issuer'), detailKey: 'issuer' },
      { key: 'valid_to', label: t('bugBounty.surface.inventory.fields.validTo'), detailKey: 'valid_to', formatter: 'time' },
      { key: 'risk_status', label: t('bugBounty.surface.inventory.fields.riskStatus'), detailKey: 'risk_status' },
      { key: 'sha256', label: t('bugBounty.surface.inventory.fields.sha256'), detailKey: 'sha256', mono: true },
    ],
  }

  return typedColumns[assetTypeFilter.value] || generic
})

const loadInventory = async (programId = selectedProgramId.value || null, useLoading = true) => {
  try {
    if (useLoading) inventoryLoading.value = true
    const response = await invoke<any>('surface_list_inventory', {
      filter: {
        program_id: programId,
        asset_type: assetTypeFilter.value || null,
        status: statusFilter.value || null,
        search: search.value.trim() || null,
        limit: inventoryPageSize.value + 1,
        offset: (inventoryPage.value - 1) * inventoryPageSize.value,
      },
    })

    const rows = Array.isArray(response?.items) ? response.items : []
    inventoryTotal.value = Number(response?.total || 0)
    inventoryHasNext.value = rows.length > inventoryPageSize.value
    inventoryItems.value = rows.slice(0, inventoryPageSize.value)
    assets.value = inventoryItems.value.map((item) => item.asset)
  } catch (error) {
    console.error('Failed to load surface inventory:', error)
    inventoryTotal.value = 0
    inventoryItems.value = []
    assets.value = []
    inventoryHasNext.value = false
  } finally {
    if (useLoading) inventoryLoading.value = false
  }
}

const loadAll = async () => {
  try {
    loading.value = true
    const programId = selectedProgramId.value || null
    const [overviewData, runData] = await Promise.all([
      invoke<any>('surface_get_overview', { programId }),
      invoke<any[]>('surface_list_discovery_runs', { programId, limit: 50 }),
    ])
    await loadInventory(programId, false)

    overview.value = overviewData
    runs.value = runData

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
    inventoryTotal.value = 0
    inventoryItems.value = []
    runs.value = []
  } finally {
    loading.value = false
  }
}

const refreshAll = async () => {
  await loadAll()
  topologyRefreshToken.value += 1
  emit('refresh')
}

const assetStatsModalTitle = computed(() =>
  assetStatsFilter.value === 'active'
    ? t('bugBounty.surface.stats.activeAssets')
    : t('bugBounty.surface.stats.totalAssets'),
)

const openAssetDetail = (assetId?: string | null) => {
  if (!assetId) return
  showRunDetailModal.value = false
  showAssetStatsModal.value = false
  selectedRunId.value = null
  selectedAssetId.value = assetId
  showDetailModal.value = true
}

const openRunDetail = (runId?: string | null) => {
  if (!runId) return
  selectedRunId.value = runId
  showRunDetailModal.value = true
}

const closeRunDetail = () => {
  showRunDetailModal.value = false
  selectedRunId.value = null
}

const openAssetStatsModal = (scope: 'all' | 'active') => {
  assetStatsFilter.value = scope
  showAssetStatsModal.value = true
}

const closeAssetStatsModal = () => {
  showAssetStatsModal.value = false
}

const closeDetailModal = () => {
  showDetailModal.value = false
  selectedAssetId.value = null
}

const formatInventoryValue = (item: any, column: any) => {
  const raw =
    column.detailKey != null
      ? item?.typed_details?.[column.detailKey]
      : item?.asset?.[column.valueKey]

  if (raw === null || raw === undefined || raw === '') return '-'
  if (column.formatter === 'time') return formatTime(String(raw))
  if (Array.isArray(raw)) return raw.join(', ')
  if (typeof raw === 'object') return JSON.stringify(raw)
  return String(raw)
}

const reloadInventoryFromFirstPage = () => {
  if (inventoryPage.value !== 1) {
    inventoryPage.value = 1
    return
  }
  loadInventory()
}

const setInventoryPage = (page: number) => {
  const nextPage = Math.min(Math.max(1, page), inventoryPageCount.value)
  if (inventoryPage.value === nextPage) {
    loadInventory()
    return
  }
  inventoryPage.value = nextPage
}

const goToFirstInventoryPage = () => {
  if (inventoryLoading.value || inventoryPage.value <= 1) return
  setInventoryPage(1)
}

const goToPreviousInventoryPage = () => {
  if (inventoryPage.value <= 1 || inventoryLoading.value) return
  setInventoryPage(inventoryPage.value - 1)
}

const goToNextInventoryPage = () => {
  if (inventoryPage.value >= inventoryPageCount.value || inventoryLoading.value) return
  setInventoryPage(inventoryPage.value + 1)
}

const goToLastInventoryPage = () => {
  if (inventoryLoading.value || inventoryPage.value >= inventoryPageCount.value) return
  setInventoryPage(inventoryPageCount.value)
}

const applyInventoryPageJump = () => {
  if (inventoryLoading.value) return
  const page = Number.parseInt(inventoryPageInput.value, 10)
  if (Number.isNaN(page)) {
    inventoryPageInput.value = String(inventoryPage.value)
    return
  }
  setInventoryPage(page)
}

const formatTime = (value?: string) => {
  if (!value) return '-'
  const date = new Date(value)
  if (Number.isNaN(date.getTime())) return value
  return date.toLocaleString()
}

const formatStatus = (value?: string) => {
  if (!value) return '-'
  const key = `bugBounty.surface.status.${value}`
  const translated = t(key)
  return translated === key ? value : translated
}

const getStatusIndicatorClass = (value?: string) => {
  switch (value) {
    case 'active':
      return 'bg-success'
    case 'inactive':
      return 'bg-base-content/25'
    case 'unknown':
      return 'bg-warning'
    default:
      return 'bg-base-content/20'
  }
}

const formatAssetType = (value?: string) => {
  if (!value) return '-'
  const key = `bugBounty.surface.assetTypes.${value}`
  const translated = t(key)
  return translated === key ? value : translated
}

watch(
  () => props.programId,
  (value) => {
    selectedProgramId.value = value || ''
  },
)

watch(selectedProgramId, () => {
  inventoryPage.value = 1
  loadAll()
})

watch([assetTypeFilter, statusFilter], () => {
  reloadInventoryFromFirstPage()
})

watch(inventoryPage, () => {
  inventoryPageInput.value = String(inventoryPage.value)
  loadInventory()
})

watch(inventoryPageSize, () => {
  inventoryPageInput.value = '1'
  reloadInventoryFromFirstPage()
})

watch(search, () => {
  if (searchDebounceTimer) {
    clearTimeout(searchDebounceTimer)
  }

  searchDebounceTimer = setTimeout(() => {
    reloadInventoryFromFirstPage()
  }, 250)
})

onBeforeUnmount(() => {
  if (searchDebounceTimer) {
    clearTimeout(searchDebounceTimer)
  }
})

onMounted(() => {
  inventoryPageInput.value = String(inventoryPage.value)
  loadAll()
})
</script>
