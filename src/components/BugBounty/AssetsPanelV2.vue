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
        <div class="stat-title">{{ t('bugBounty.surface.stats.newAssets') }}</div>
        <button class="stat-value text-info text-2xl text-left hover:underline" @click="openAssetStatsModal('new')">
          {{ newAssetTotal }}
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
      <a class="tab" :class="{ 'tab-active': activeTab === 'types' }" @click="activeTab = 'types'">{{ t('bugBounty.surface.tabs.types') }}</a>
      <a class="tab" :class="{ 'tab-active': activeTab === 'topology' }" @click="activeTab = 'topology'">{{ t('bugBounty.surface.tabs.topology') }}</a>
      <a class="tab" :class="{ 'tab-active': activeTab === 'runs' }" @click="activeTab = 'runs'">{{ t('bugBounty.surface.tabs.runs') }}</a>
    </div>

    <div v-show="activeTab === 'inventory'" class="card bg-base-100 shadow-sm border border-base-300">
      <div class="card-body">
        <div class="flex flex-col gap-3 xl:flex-row xl:items-center xl:justify-between">
          <h3 class="card-title text-base">{{ t('bugBounty.surface.inventory.title') }}</h3>
          <div class="flex flex-col gap-2 sm:flex-row sm:flex-wrap sm:items-center sm:justify-end">
            <input
              v-model="search"
              type="text"
              class="input input-bordered input-sm w-full sm:w-72"
              :placeholder="t('bugBounty.surface.inventory.searchPlaceholder')"
            />
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
            <button
              type="button"
              class="btn btn-outline btn-sm"
              :class="favoriteFilter ? 'btn-warning' : ''"
              :title="t('bugBounty.surface.inventory.favorite.only')"
              @click="toggleFavoriteFilter"
            >
              <i class="fas fa-star mr-2"></i>
              {{ t('bugBounty.surface.inventory.favorite.only') }}
            </button>
            <select
              v-if="showServiceFacetFilters"
              v-model="serviceNameFilter"
              class="select select-bordered select-sm"
            >
              <option value="">{{ t('bugBounty.filter.allServices') }}</option>
              <option
                v-for="option in serviceNameOptions"
                :key="option.value"
                :value="option.value"
              >
                {{ option.value }} ({{ option.count }})
              </option>
            </select>
            <select
              v-if="showServiceFacetFilters"
              v-model="transportProtocolFilter"
              class="select select-bordered select-sm"
            >
              <option value="">{{ t('bugBounty.filter.allServiceTypes') }}</option>
              <option
                v-for="option in transportProtocolOptions"
                :key="option.value"
                :value="option.value"
              >
                {{ option.value }} ({{ option.count }})
              </option>
            </select>
            <button
              class="btn btn-outline btn-sm"
              :class="showMoreInventoryActions ? 'btn-active' : ''"
              @click="showMoreInventoryActions = !showMoreInventoryActions"
            >
              <i class="fas fa-ellipsis-h mr-2"></i>
              {{ t('bugBounty.surface.inventory.actions.moreActions') }}
            </button>
          </div>
        </div>
        <div
          v-if="showMoreInventoryActions"
          class="rounded-lg border border-base-300 bg-base-200/40 p-3"
        >
          <div class="flex flex-col gap-2 sm:flex-row sm:flex-wrap sm:items-center">
            <button class="btn btn-outline btn-sm" @click="openImportModal">
              <i class="fas fa-file-import mr-2"></i>
              {{ t('bugBounty.surface.inventory.actions.manualImport') }}
            </button>
            <button class="btn btn-outline btn-sm" :disabled="exportingAssets" @click="openExportModal()">
              <span v-if="exportingAssets" class="loading loading-spinner loading-xs mr-2"></span>
              <i v-else class="fas fa-download mr-2"></i>
              {{ t('bugBounty.surface.inventory.actions.exportAssets') }}
            </button>
          </div>
        </div>
        <div
          v-if="inventoryTotal > 0 && selectedAssetIds.length > 0"
          class="flex flex-col gap-3 rounded-lg border border-base-300 bg-base-200/40 p-3 lg:flex-row lg:items-center lg:justify-between"
        >
          <div class="text-sm text-base-content/70">
            {{ t('bugBounty.surface.inventory.actions.selectionSummary', { selected: selectedAssetIds.length, page: inventoryItems.length, total: inventoryTotal }) }}
          </div>
          <div class="flex flex-wrap items-center gap-2">
            <button class="btn btn-sm btn-warning" :disabled="!selectedAssetIds.length || inventoryLoading || bulkDeleting" @click="deleteSelectedAssets">
              {{ t('bugBounty.surface.inventory.actions.deleteSelected') }}
            </button>
            <button class="btn btn-sm btn-error" :disabled="!inventoryTotal || inventoryLoading || bulkDeleting" @click="deleteAllFilteredAssets">
              {{ t('bugBounty.surface.inventory.actions.deleteAll') }}
            </button>
            <button class="btn btn-sm btn-outline" :disabled="!selectedAssetIds.length || inventoryLoading || bulkDeleting" @click="markSelectedAssetsViewed">
              {{ t('bugBounty.surface.inventory.actions.markSelectedViewed') }}
            </button>
            <button class="btn btn-sm btn-outline" :disabled="!inventoryTotal || inventoryLoading || bulkDeleting" @click="markFilteredAssetsViewed">
              {{ t('bugBounty.surface.inventory.actions.markFilteredViewed') }}
            </button>
          </div>
        </div>
        <div class="overflow-x-auto">
          <table class="table table-sm">
            <thead>
              <tr>
                <th class="w-12">
                  <input
                    type="checkbox"
                    class="checkbox checkbox-sm"
                    :checked="allCurrentPageSelected"
                    :disabled="!inventoryItems.length || inventoryLoading"
                    @click.stop
                    @change="toggleSelectCurrentPage"
                  />
                </th>
                <th v-for="column in inventoryColumns" :key="column.key">
                  <SurfaceTableColumnFilter
                    v-if="column.key === 'favicon_hash'"
                    v-model="faviconPresenceFilter"
                    :label="column.label"
                    :options="faviconColumnFilterOptions"
                  />
                  <SurfaceTableColumnFilter
                    v-else-if="column.key === 'http_status_code'"
                    v-model="httpStatusFilter"
                    :label="column.label"
                    :options="httpStatusColumnFilterOptions"
                    :groups="httpStatusColumnFilterGroups"
                  />
                  <SurfaceTableTextFilter
                    v-else-if="isColumnTextFilterable(column)"
                    :model-value="columnFilterValues[column.key] || ''"
                    :label="column.label"
                    :placeholder="t('bugBounty.surface.inventory.columnSearchPlaceholder', { column: column.label })"
                    @update:model-value="setColumnFilterValue(column.key, $event)"
                  />
                  <span v-else>{{ column.label }}</span>
	                </th>
                <th>
                  <SurfaceTableColumnFilter
                    v-model="viewStateFilter"
                    :label="t('bugBounty.surface.inventory.viewStates.column')"
                    :options="viewStateColumnFilterOptions"
                  />
                </th>
                <th class="w-56">{{ t('bugBounty.surface.inventory.actions.column') }}</th>
              </tr>
            </thead>
            <tbody>
              <tr
                v-for="item in inventoryItems"
                :key="item.asset.id"
                class="cursor-pointer hover"
                :class="isNewAsset(item.asset) ? 'bg-info/5' : ''"
                @click="openAssetDetail(item.asset.id)"
              >
                <td @click.stop>
                  <input
                    type="checkbox"
                    class="checkbox checkbox-sm"
                    :checked="selectedAssetIdSet.has(item.asset.id)"
                    @change="toggleAssetSelection(item.asset.id)"
                  />
                </td>
                <td
                  v-for="column in inventoryColumns"
                  :key="column.key"
                  :class="column.mono ? 'font-mono text-xs break-all' : ''"
                >
                  <SurfaceAssetTypeIcon
                    v-if="column.badge === 'type'"
                    :type="item.asset.asset_type"
                    :label="formatAssetType(item.asset.asset_type)"
                  />
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
                  <button
                    v-else-if="column.key === 'favicon_hash' && hasFaviconHash(item)"
                    type="button"
                    class="link link-hover font-mono text-xs"
                    @click.stop="openFaviconAssetsModal(getFaviconHash(item))"
                  >
                    {{ getFaviconHash(item) }}
                  </button>
                  <span v-else>
                    {{ formatInventoryValue(item, column) }}
                  </span>
                </td>
                <td>
                  <span
                    class="badge badge-sm"
                    :class="isNewAsset(item.asset) ? 'badge-info' : 'badge-outline'"
                  >
                    {{ formatViewState(item.asset) }}
                  </span>
                </td>
                <td @click.stop>
                  <div class="flex flex-wrap gap-1">
                    <button
                      type="button"
                      class="btn btn-ghost btn-xs h-7 min-h-0 px-1.5"
                      :class="item.asset.is_favorite ? 'text-warning' : 'text-base-content/35'"
                      :disabled="isFavoriteUpdating(item.asset.id)"
                      :title="formatFavoriteAction(item.asset)"
                      :aria-label="formatFavoriteAction(item.asset)"
                      @click="toggleAssetFavorite(item)"
                    >
                      <span
                        v-if="isFavoriteUpdating(item.asset.id)"
                        class="loading loading-spinner loading-xs"
                      ></span>
                      <i v-else class="fas fa-star"></i>
                    </button>
                    <SurfaceIconButton
                      :label="t('bugBounty.surface.inventory.actions.edit')"
                      icon="edit"
                      @click="openEditModal(item)"
                    />
                    <SurfaceIconButton
                      :label="t('bugBounty.surface.inventory.actions.sendToAssistant')"
                      icon="assistant"
                      tone="primary"
                      @click="sendAssetToAssistant(item.asset.id)"
                    />
                    <SurfaceIconButton
                      :label="t('bugBounty.surface.inventory.actions.delete')"
                      icon="delete"
                      tone="error"
                      @click="deleteSingleAsset(item.asset)"
                    />
                  </div>
                </td>
              </tr>
              <tr v-if="inventoryLoading">
                <td :colspan="inventoryColumns.length + 3" class="py-8 text-center text-sm text-base-content/60">
                  <span class="loading loading-spinner loading-sm mr-2"></span>
                  {{ t('common.loading') }}
                </td>
              </tr>
            </tbody>
          </table>
        </div>
        <div v-if="!inventoryLoading && !assets.length" class="text-sm text-base-content/60">{{ t('bugBounty.surface.inventory.empty') }}</div>
        <div v-if="inventoryItems.length > 0 || inventoryTotal > 0" class="flex flex-col gap-3 pt-2 xl:flex-row xl:items-center xl:justify-between">
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
                :disabled="!inventoryHasNext || inventoryLoading"
                @click="goToNextInventoryPage"
              >
                {{ t('common.next') }}
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>

    <SurfaceFingerprintCategoryPanel
      v-if="activeTab === 'types'"
      :program-id="selectedProgramId || null"
      :refresh-token="fingerprintRefreshToken"
      @open-asset="openAssetDetail"
    />

    <SurfaceTopologyPanel
      v-show="activeTab === 'topology'"
      :program-id="selectedProgramId || null"
      :refresh-token="topologyRefreshToken"
      @open-asset="openAssetDetail"
    />

    <div v-show="activeTab === 'runs'" class="card bg-base-100 shadow-sm border border-base-300">
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
                v-for="run in pagedRuns"
                :key="run.id"
                class="cursor-pointer hover"
                @click="openRunDetail(run.id)"
              >
                <td><span class="badge badge-sm" :class="run.status === 'completed' ? 'badge-success' : run.status === 'failed' ? 'badge-error' : 'badge-warning'">{{ formatStatus(run.status) }}</span></td>
                <td>{{ run.plugin_id || '-' }}</td>
                <td>{{ run.trigger_source }}</td>
                <td>{{ run.observation_count || 0 }}</td>
                <td>{{ runImportedOrEnrichedCount(run) }}</td>
                <td>{{ run.changed_asset_count || 0 }}</td>
                <td>{{ formatTime(run.started_at) }}</td>
              </tr>
            </tbody>
          </table>
        </div>
        <div v-if="!runTotal" class="text-sm text-base-content/60">{{ t('bugBounty.surface.runs.empty') }}</div>
        <div v-if="runTotal > 0" class="flex flex-col gap-3 pt-2 xl:flex-row xl:items-center xl:justify-between">
          <div class="flex items-center gap-2 text-sm">
            <span class="text-base-content/70">{{ t('bugBounty.surface.inventory.pageSizeLabel') }}</span>
            <select v-model.number="runPageSize" class="select select-bordered select-sm">
              <option v-for="size in runPageSizeOptions" :key="size" :value="size">
                {{ size }}
              </option>
            </select>
          </div>

          <div class="flex flex-col gap-2 sm:flex-row sm:flex-wrap sm:items-center sm:justify-end">
            <div class="join">
              <button
                class="join-item btn btn-sm"
                :disabled="runPage <= 1 || loading"
                @click="goToFirstRunPage"
              >
                {{ t('bugBounty.surface.inventory.firstPage') }}
              </button>
              <button
                class="join-item btn btn-sm"
                :disabled="runPage <= 1 || loading"
                @click="goToPreviousRunPage"
              >
                {{ t('common.previous') }}
              </button>
              <button class="join-item btn btn-sm">
                {{ t('bugBounty.surface.inventory.pageInfo', { page: runPage, total: runPageCount }) }}
              </button>
              <button
                class="join-item btn btn-sm"
                :disabled="runPage >= runPageCount || loading"
                @click="goToNextRunPage"
              >
                {{ t('common.next') }}
              </button>
              <button
                class="join-item btn btn-sm"
                :disabled="runPage >= runPageCount || loading"
                @click="goToLastRunPage"
              >
                {{ t('bugBounty.surface.inventory.lastPage') }}
              </button>
            </div>

            <div class="flex items-center gap-2">
              <input
                v-model="runPageInput"
                type="number"
                min="1"
                :max="runPageCount"
                class="input input-bordered input-sm w-24"
                :placeholder="t('bugBounty.surface.inventory.jumpPlaceholder')"
                @keyup.enter="applyRunPageJump"
              />
              <button
                class="btn btn-sm btn-outline"
                :disabled="loading"
                @click="applyRunPageJump"
              >
                {{ t('bugBounty.surface.inventory.jump') }}
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>

  </div>
  <SurfaceAssetListModal
    :visible="showAssetStatsModal"
    :title="assetStatsModalTitle"
    :program-id="selectedProgramId || null"
    :status-filter="assetStatsFilter"
    @close="closeAssetStatsModal"
    @select-type="applyAssetStatsFilter"
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
  <SurfaceAssetEditModal
    :visible="showEditModal"
    :target="editingAsset"
    @close="closeEditModal"
    @save="saveAssetEdit"
  />
  <SurfaceAssetImportModal
    :visible="showImportModal"
    :submitting="importing"
    :program-id="selectedProgramId || null"
    :programs="programs"
    @close="closeImportModal"
    @submit="submitImport"
  />
  <SurfaceAssetExportModal
    :visible="showExportModal"
    :exporting="exportingAssets"
    :program-name="selectedProgramName"
    :current-asset-type="assetTypeFilter || null"
    :available-asset-types="exportAssetTypes"
    :initial-export-type="preferredExportType"
    @close="closeExportModal"
    @submit="exportInventoryAssets"
  />
  <SurfaceFaviconAssetsModal
    :visible="showFaviconAssetsModal"
    :program-id="selectedProgramId || null"
    :favicon-hash="selectedFaviconHash"
    @close="closeFaviconAssetsModal"
    @open-asset="openAssetDetail"
  />
</template>

<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { save } from '@tauri-apps/plugin-dialog'
import { useI18n } from 'vue-i18n'
import { useRouter } from 'vue-router'
import SurfaceAssetListModal from './SurfaceAssetListModal.vue'
import SurfaceAssetDetailModal from './SurfaceAssetDetailModal.vue'
import SurfaceDiscoveryRunDetailModal from './SurfaceDiscoveryRunDetailModal.vue'
import SurfaceFingerprintCategoryPanel from './SurfaceFingerprintCategoryPanel.vue'
import SurfaceTopologyPanel from './SurfaceTopologyPanel.vue'
import SurfaceAssetEditModal from './SurfaceAssetEditModal.vue'
import SurfaceAssetExportModal, { type SurfaceAssetExportPayload } from './SurfaceAssetExportModal.vue'
import SurfaceFaviconAssetsModal from './SurfaceFaviconAssetsModal.vue'
import SurfaceAssetImportModal, { type SurfaceAssetImportPayload } from './SurfaceAssetImportModal.vue'
import SurfaceAssetTypeIcon from './SurfaceAssetTypeIcon.vue'
import SurfaceIconButton from './SurfaceIconButton.vue'
import SurfaceTableColumnFilter from './SurfaceTableColumnFilter.vue'
import SurfaceTableTextFilter from './SurfaceTableTextFilter.vue'
import { type SurfaceAssetEditPayload, type SurfaceAssetEditTarget } from './surfaceAssetEditSupport'
import { buildReferencedSurfaceAsset, queueSurfaceAssetForAssistant } from './surfaceAssetUtils'
import {
  sanitizeExportFilenamePart,
  type SurfaceAssetExportType,
} from './surfaceAssetExportSupport'
import { useToast } from '../../composables/useToast'
import { dialog } from '../../composables/useDialog'

const props = defineProps<{
  programId?: string | null
  programs?: Array<{ id: string; name: string }>
}>()

const emit = defineEmits<{
  (e: 'refresh', payload?: { programId: string | null }): void
}>()

const { t } = useI18n()
const router = useRouter()
const toast = useToast()

const loading = ref(false)
const inventoryLoading = ref(false)
const bulkDeleting = ref(false)
const importing = ref(false)
const exportingAssets = ref(false)
const activeTab = ref<'inventory' | 'types' | 'topology' | 'runs'>('inventory')
const selectedProgramId = ref(props.programId || '')
const search = ref('')
const assetTypeFilter = ref('web')
const statusFilter = ref('')
const viewStateFilter = ref('')
const favoriteFilter = ref(false)
const faviconPresenceFilter = ref('')
const serviceNameFilter = ref('')
const transportProtocolFilter = ref('')
const columnFilterValues = ref<Record<string, string>>({})
const inventoryPage = ref(1)
const inventoryHasNext = ref(false)
const inventoryTotal = ref(0)
const inventoryTotalLoading = ref(false)
const inventoryPageSize = ref(10)
const inventoryPageSizeOptions = [10, 20, 50, 100]
const newAssetTotal = ref(0)
const runPage = ref(1)
const runPageSize = ref(10)
const runPageInput = ref('1')
const runPageSizeOptions = [10, 20, 50, 100]
let searchDebounceTimer: ReturnType<typeof setTimeout> | null = null
const topologyRefreshToken = ref(0)
const fingerprintRefreshToken = ref(0)

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
const assetStatsFilter = ref<'all' | 'new'>('all')
const selectedAssetIds = ref<string[]>([])
const selectedAssetTypeById = ref<Record<string, string>>({})
const favoriteUpdatingIds = ref<Set<string>>(new Set())
const showEditModal = ref(false)
const editingAsset = ref<SurfaceAssetEditTarget | null>(null)
const showImportModal = ref(false)
const showExportModal = ref(false)
const showFaviconAssetsModal = ref(false)
const selectedFaviconHash = ref<string | null>(null)
const serviceNameOptions = ref<Array<{ value: string; count: number }>>([])
const transportProtocolOptions = ref<Array<{ value: string; count: number }>>([])
const httpStatusOptions = ref<Array<{ value: string; count: number }>>([])
const preferredExportType = ref<SurfaceAssetExportType>('all')
const httpStatusFilter = ref('')
const showMoreInventoryActions = ref(false)
type SurfaceInventoryCursor = { last_seen_at: string; id: string }
const inventoryCursorByPage = ref<Record<number, SurfaceInventoryCursor | null>>({ 1: null })
const inventoryNextCursorByPage = ref<Record<number, SurfaceInventoryCursor | null>>({})
const commonHttpStatusCodes = ['200', '201', '204', '301', '302', '304', '400', '401', '403', '404', '405', '429', '500', '502', '503', '504']
const httpStatusGroupLabels: Record<string, string> = {
  '1': '1xx',
  '2': '2xx',
  '3': '3xx',
  '4': '4xx',
  '5': '5xx',
}
const assetColumnFilterKeys = new Set(['type', 'name', 'display', 'status', 'exposure', 'source'])
const typedColumnFilterKeysByType: Record<string, Set<string>> = {
  org: new Set(['org_name', 'business_line', 'importance_level']),
  domain: new Set(['root_domain', 'subdomain_level', 'record_type', 'record_value', 'registrar']),
  ip: new Set(['cidr', 'asn', 'cloud_provider', 'network_boundary_type']),
  host: new Set(['fqdn', 'operating_system', 'device_type', 'region_or_datacenter']),
  port: new Set(['ip_address', 'port_number', 'transport_protocol', 'port_state']),
  service: new Set(['application_service_name', 'transport_protocol', 'port_number', 'product_name', 'version', 'auth_type']),
  web: new Set(['canonical_url', 'site_title', 'http_status_code', 'favicon_hash']),
  certificate: new Set(['subject', 'issuer', 'risk_status', 'sha256']),
}
const numericColumnFilterKeys = new Set(['asn', 'http_status_code', 'port_number', 'subdomain_level'])

const assetTypeOptions = computed(() => {
  return Object.entries(overview.value.by_type || {})
    .filter(([, count]) => Number(count) > 0)
    .sort((a, b) => Number(b[1]) - Number(a[1]))
    .map(([type, count]) => ({ type, count: Number(count) }))
})
const exportAssetTypes = computed(() =>
  Array.from(
    new Set([
      ...assetTypeOptions.value.map(option => option.type),
      'org',
      'domain',
      'ip',
      'host',
      'port',
      'service',
      'web',
      'certificate',
    ]),
  ),
)
const showServiceFacetFilters = computed(() => assetTypeFilter.value === 'service')
const showWebFaviconFilter = computed(() => assetTypeFilter.value === 'web')
const showWebHttpStatusFilter = computed(() => assetTypeFilter.value === 'web')
const selectedProgramName = computed(() => {
  const selected = (props.programs || []).find(program => program.id === selectedProgramId.value)
  return selected?.name || null
})

const inventoryPageCount = computed(() => Math.max(1, Math.ceil(inventoryTotal.value / inventoryPageSize.value)))
const runTotal = computed(() => runs.value.length)
const runPageCount = computed(() => Math.max(1, Math.ceil(runTotal.value / runPageSize.value)))
const pagedRuns = computed(() => {
  const offset = (runPage.value - 1) * runPageSize.value
  return runs.value.slice(offset, offset + runPageSize.value)
})
const runImportedOrEnrichedCount = (run: any) =>
  Number(run?.imported_asset_count || 0) + Number(run?.changed_asset_count || 0)
const currentPageAssetIds = computed(() => inventoryItems.value.map((item) => item.asset.id).filter(Boolean))
const selectedAssetIdSet = computed(() => new Set(selectedAssetIds.value))
const allCurrentPageSelected = computed(() =>
  currentPageAssetIds.value.length > 0 &&
  currentPageAssetIds.value.every((id) => selectedAssetIdSet.value.has(id)),
)
const hasFaviconHashFilterValue = computed(() => {
  if (!showWebFaviconFilter.value) return null
  if (faviconPresenceFilter.value === 'has') return true
  if (faviconPresenceFilter.value === 'missing') return false
  return null
})
const httpStatusFilterValue = computed(() => {
  if (!showWebHttpStatusFilter.value) return null
  const parsed = Number.parseInt(httpStatusFilter.value, 10)
  return Number.isFinite(parsed) ? parsed : null
})

const resetInventoryCursorPagination = () => {
  inventoryCursorByPage.value = { 1: null }
  inventoryNextCursorByPage.value = {}
}

const columnFilterKeyMatchesAssetType = (key: string, assetType?: string | null) => {
  if (assetColumnFilterKeys.has(key)) return true
  if (!assetType) return false
  return Boolean(typedColumnFilterKeysByType[assetType]?.has(key))
}

const buildColumnFilterPayload = (assetType?: string | null) =>
  Object.entries(columnFilterValues.value)
    .map(([key, value]) => ({ key, value: value.trim() }))
    .filter(({ key, value }) => value && columnFilterKeyMatchesAssetType(key, assetType))
    .map(({ key, value }) => ({
      key,
      operator: numericColumnFilterKeys.has(key) ? 'equals' : 'contains',
      value,
    }))

const clearInvalidColumnFilters = (assetType?: string | null) => {
  const nextFilters: Record<string, string> = {}
  for (const [key, value] of Object.entries(columnFilterValues.value)) {
    if (columnFilterKeyMatchesAssetType(key, assetType)) {
      nextFilters[key] = value
    }
  }
  columnFilterValues.value = nextFilters
}

const setColumnFilterValue = (key: string, value: string) => {
  const nextFilters = { ...columnFilterValues.value }
  const trimmed = value.trim()
  if (trimmed) {
    nextFilters[key] = trimmed
  } else {
    delete nextFilters[key]
  }
  columnFilterValues.value = nextFilters
}

const isColumnTextFilterable = (column: any) => {
  if (!column?.key || column.badge || column.formatter === 'time') return false
  if (column.key === 'favicon_hash' || column.key === 'http_status_code') return false
  return columnFilterKeyMatchesAssetType(column.key, assetTypeFilter.value || null)
}

const hasInventoryFilters = computed(() =>
  Boolean(
    selectedProgramId.value ||
      assetTypeFilter.value ||
      statusFilter.value ||
      viewStateFilter.value ||
      favoriteFilter.value ||
      faviconPresenceFilter.value ||
      serviceNameFilter.value ||
      transportProtocolFilter.value ||
      httpStatusFilter.value ||
      columnFilterPayload.value.length ||
      search.value.trim(),
  ),
)
const columnFilterPayload = computed(() => buildColumnFilterPayload(assetTypeFilter.value || null))
const viewStateColumnFilterOptions = computed(() => [
  { value: '', label: t('bugBounty.surface.inventory.viewStates.all') },
  { value: 'new', label: t('bugBounty.surface.inventory.viewStates.new') },
  { value: 'viewed', label: t('bugBounty.surface.inventory.viewStates.viewed') },
])
const faviconColumnFilterOptions = computed(() => [
  { value: '', label: t('bugBounty.surface.inventory.faviconPresence.all') },
  { value: 'has', label: t('bugBounty.surface.inventory.faviconPresence.has') },
  { value: 'missing', label: t('bugBounty.surface.inventory.faviconPresence.missing') },
])
const httpStatusColumnFilterOptions = computed(() => [
  { value: '', label: t('bugBounty.surface.inventory.httpStatus.all') },
  ...Array.from(
    new Set([
      ...commonHttpStatusCodes,
      ...httpStatusOptions.value.map(option => option.value),
    ]),
  )
    .sort((a, b) => Number(a) - Number(b))
    .map((value) => {
      const matched = httpStatusOptions.value.find(option => option.value === value)
      return {
        value,
        label: matched ? `${value} (${matched.count})` : value,
      }
    }),
])
const httpStatusColumnFilterGroups = computed(() => {
  const grouped = new Map<string, Array<{ value: string; label: string }>>()
  for (const option of httpStatusColumnFilterOptions.value.slice(1)) {
    const groupKey = option.value.charAt(0)
    const targetKey = httpStatusGroupLabels[groupKey] ? groupKey : 'other'
    if (!grouped.has(targetKey)) {
      grouped.set(targetKey, [])
    }
    grouped.get(targetKey)?.push(option)
  }

  return Array.from(grouped.entries()).map(([key, options]) => ({
    label: httpStatusGroupLabels[key] || t('bugBounty.surface.inventory.httpStatus.other'),
    options,
  }))
})

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
      {
        key: 'subdomain_level',
        label: t('bugBounty.surface.inventory.fields.subdomainLevel'),
        detailKey: 'subdomain_level',
        formatter: 'domainLevel',
      },
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
      { key: 'transport_protocol', label: t('bugBounty.surface.inventory.fields.transportProtocol'), detailKey: 'transport_protocol' },
      { key: 'port_number', label: t('bugBounty.surface.inventory.fields.portNumber'), detailKey: 'port_number' },
      { key: 'product_name', label: t('bugBounty.surface.inventory.fields.productName'), detailKey: 'product_name' },
      { key: 'version', label: t('bugBounty.surface.inventory.fields.version'), detailKey: 'version' },
      { key: 'auth_type', label: t('bugBounty.surface.inventory.fields.authType'), detailKey: 'auth_type' },
      { key: 'lastSeen', label: t('bugBounty.surface.columns.lastSeen'), valueKey: 'last_seen_at', formatter: 'time' },
    ],
    web: [
      { key: 'canonical_url', label: t('bugBounty.surface.inventory.fields.canonicalUrl'), detailKey: 'canonical_url', mono: true },
      { key: 'site_title', label: t('bugBounty.surface.inventory.fields.siteTitle'), detailKey: 'site_title' },
      { key: 'http_status_code', label: t('bugBounty.surface.inventory.fields.httpStatusCode'), detailKey: 'http_status_code' },
      { key: 'favicon_hash', label: t('bugBounty.surface.inventory.fields.favicon'), detailKey: 'favicon_hash', mono: true },
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
    const cursor = inventoryCursorByPage.value[inventoryPage.value] || null
    const response = await invoke<any>('surface_list_inventory', {
      filter: {
        program_id: programId,
        asset_type: assetTypeFilter.value || null,
        status: statusFilter.value || null,
        view_state: viewStateFilter.value || null,
        is_favorite: favoriteFilter.value ? true : null,
        search: search.value.trim() || null,
        has_favicon_hash: hasFaviconHashFilterValue.value,
        http_status_code: httpStatusFilterValue.value,
        service_name: serviceNameFilter.value || null,
        transport_protocol: transportProtocolFilter.value || null,
        column_filters: columnFilterPayload.value.length ? columnFilterPayload.value : null,
        limit: inventoryPageSize.value,
        offset: null,
      },
      cursor,
    })

    const rows = Array.isArray(response?.items) ? response.items : []
    inventoryHasNext.value = Boolean(response?.has_next)
    inventoryItems.value = rows
    assets.value = inventoryItems.value.map((item) => item.asset)
    const nextCursor = response?.next_cursor || null
    if (inventoryHasNext.value && nextCursor) {
      inventoryNextCursorByPage.value[inventoryPage.value] = nextCursor
      inventoryCursorByPage.value[inventoryPage.value + 1] = nextCursor
    } else {
      const nextCursors = { ...inventoryNextCursorByPage.value }
      delete nextCursors[inventoryPage.value]
      inventoryNextCursorByPage.value = nextCursors
    }
    for (const item of inventoryItems.value) {
      const assetId = String(item?.asset?.id || '').trim()
      const assetType = String(item?.asset?.asset_type || '').trim()
      if (!assetId || !assetType) continue
      selectedAssetTypeById.value[assetId] = assetType
    }
  } catch (error) {
    console.error('Failed to load surface inventory:', error)
    inventoryTotal.value = 0
    inventoryItems.value = []
    assets.value = []
    inventoryHasNext.value = false
    selectedAssetIds.value = []
  } finally {
    if (useLoading) inventoryLoading.value = false
  }
}

const loadInventoryTotal = async (programId = selectedProgramId.value || null) => {
  try {
    inventoryTotalLoading.value = true
    const count = await invoke<number>('surface_count_assets', {
      filter: getInventoryFilterPayload(programId),
    })
    inventoryTotal.value = Number(count || 0)
  } catch (error) {
    console.error('Failed to load surface inventory count:', error)
    inventoryTotal.value = 0
  } finally {
    inventoryTotalLoading.value = false
  }
}

const loadInventoryFacets = async (programId = selectedProgramId.value || null) => {
  if (!showServiceFacetFilters.value && !showWebHttpStatusFilter.value) {
    serviceNameOptions.value = []
    transportProtocolOptions.value = []
    httpStatusOptions.value = []
    return
  }

  try {
    const response = await invoke<any>('surface_get_inventory_facets', {
      filter: {
        program_id: programId,
        asset_type: assetTypeFilter.value || null,
        status: statusFilter.value || null,
        view_state: viewStateFilter.value || null,
        is_favorite: favoriteFilter.value ? true : null,
        search: search.value.trim() || null,
        has_favicon_hash: hasFaviconHashFilterValue.value,
        http_status_code: null,
        service_name: null,
        transport_protocol: null,
        column_filters: columnFilterPayload.value.length ? columnFilterPayload.value : null,
        limit: null,
        offset: null,
      },
    })

    serviceNameOptions.value = Array.isArray(response?.service_names) ? response.service_names : []
    transportProtocolOptions.value = Array.isArray(response?.transport_protocols) ? response.transport_protocols : []
    httpStatusOptions.value = Array.isArray(response?.http_status_codes) ? response.http_status_codes : []
  } catch (error) {
    console.error('Failed to load surface inventory facets:', error)
    serviceNameOptions.value = []
    transportProtocolOptions.value = []
    httpStatusOptions.value = []
  }
}

const loadNewAssetCount = async (programId = selectedProgramId.value || null) => {
  try {
    const count = await invoke<number>('surface_count_assets', {
      filter: {
        program_id: programId,
        asset_type: null,
        status: null,
        view_state: 'new',
        is_favorite: null,
        search: null,
        favicon_hash: null,
        has_favicon_hash: null,
        http_status_code: null,
        service_name: null,
        transport_protocol: null,
        column_filters: null,
        limit: null,
        offset: null,
      },
    })

    newAssetTotal.value = Number(count)
  } catch (error) {
    console.error('Failed to load new surface asset count:', error)
    newAssetTotal.value = 0
  }
}

const loadAll = async () => {
  try {
    loading.value = true
    const programId = selectedProgramId.value || null
    const [overviewData, runData] = await Promise.all([
      invoke<any>('surface_get_overview', { programId }),
      invoke<any[]>('surface_list_discovery_runs', { programId, limit: 500 }),
    ])
    await loadInventory(programId, false)
    await loadInventoryTotal(programId)
    await loadInventoryFacets(programId)
    await loadNewAssetCount(programId)

    overview.value = overviewData
    runs.value = Array.isArray(runData) ? runData : []
    if (runPage.value > runPageCount.value) {
      runPage.value = runPageCount.value
    }
    runPageInput.value = String(runPage.value)
    emit('refresh', { programId })
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
    newAssetTotal.value = 0
    runs.value = []
    runPage.value = 1
    runPageInput.value = '1'
  } finally {
    loading.value = false
  }
}

const refreshAll = async () => {
  await loadAll()
  topologyRefreshToken.value += 1
  fingerprintRefreshToken.value += 1
}

const assetStatsModalTitle = computed(() =>
  assetStatsFilter.value === 'new'
    ? t('bugBounty.surface.stats.newAssets')
    : t('bugBounty.surface.stats.totalAssets'),
)

const isNewAsset = (asset?: any) => !asset?.viewed_at

const formatViewState = (asset?: any) =>
  isNewAsset(asset)
    ? t('bugBounty.surface.inventory.viewStates.new')
    : t('bugBounty.surface.inventory.viewStates.viewed')

const isFavoriteUpdating = (assetId?: string | null) =>
  Boolean(assetId && favoriteUpdatingIds.value.has(assetId))

const setFavoriteUpdating = (assetId: string, updating: boolean) => {
  const next = new Set(favoriteUpdatingIds.value)
  if (updating) {
    next.add(assetId)
  } else {
    next.delete(assetId)
  }
  favoriteUpdatingIds.value = next
}

const formatFavoriteAction = (asset?: any) =>
  asset?.is_favorite
    ? t('bugBounty.surface.inventory.favorite.remove')
    : t('bugBounty.surface.inventory.favorite.add')

const toggleFavoriteFilter = () => {
  favoriteFilter.value = !favoriteFilter.value
}

const toggleAssetFavorite = async (item: any) => {
  const asset = item?.asset
  const assetId = String(asset?.id || '').trim()
  if (!assetId) return

  const previousFavorite = Boolean(asset.is_favorite)
  const nextFavorite = !previousFavorite
  setFavoriteUpdating(assetId, true)
  asset.is_favorite = nextFavorite

  try {
    const updated = await invoke<boolean>('surface_set_asset_favorite', {
      assetId,
      isFavorite: nextFavorite,
    })
    if (!updated) {
      asset.is_favorite = previousFavorite
      toast.error(t('bugBounty.surface.inventory.favorite.updateFailed'))
      return
    }

    toast.success(
      nextFavorite
        ? t('bugBounty.surface.inventory.favorite.added')
        : t('bugBounty.surface.inventory.favorite.removed'),
    )
    if (favoriteFilter.value && !nextFavorite) {
      await loadInventory(selectedProgramId.value || null, false)
      await loadInventoryFacets(selectedProgramId.value || null)
    }
  } catch (error) {
    asset.is_favorite = previousFavorite
    console.error('Failed to update surface asset favorite:', error)
    toast.error(t('bugBounty.surface.inventory.favorite.updateFailed'))
  } finally {
    setFavoriteUpdating(assetId, false)
  }
}

const markAssetsViewedLocally = (assetIds: string[]) => {
  if (!assetIds.length) return
  const viewedAt = new Date().toISOString()
  const idSet = new Set(assetIds)
  for (const item of inventoryItems.value) {
    if (item?.asset?.id && idSet.has(item.asset.id)) {
      item.asset.viewed_at = item.asset.viewed_at || viewedAt
      item.asset.viewed_by = item.asset.viewed_by || 'surface_inventory'
    }
  }
  for (const asset of assets.value) {
    if (asset?.id && idSet.has(asset.id)) {
      asset.viewed_at = asset.viewed_at || viewedAt
      asset.viewed_by = asset.viewed_by || 'surface_inventory'
    }
  }
  newAssetTotal.value = Math.max(0, newAssetTotal.value - assetIds.length)
}

const openAssetDetail = async (assetId?: string | null) => {
  if (!assetId) return
  const asset = inventoryItems.value.find((item) => item?.asset?.id === assetId)?.asset
  if (isNewAsset(asset)) {
    try {
      await invoke('surface_mark_asset_viewed', { assetId })
      markAssetsViewedLocally([assetId])
      await loadNewAssetCount(selectedProgramId.value || null)
    } catch (error) {
      console.error('Failed to mark surface asset as viewed:', error)
      toast.error(t('bugBounty.surface.inventory.actions.markViewedFailed'))
      return
    }
  }
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

const openAssetStatsModal = (scope: 'all' | 'new') => {
  assetStatsFilter.value = scope
  showAssetStatsModal.value = true
}

const closeAssetStatsModal = () => {
  showAssetStatsModal.value = false
}

const applyAssetStatsFilter = (assetType?: string | null) => {
  if (!assetType) return
  showAssetStatsModal.value = false
  activeTab.value = 'inventory'
  assetTypeFilter.value = assetType
  statusFilter.value = ''
  viewStateFilter.value = assetStatsFilter.value === 'new' ? 'new' : ''
  favoriteFilter.value = false
  faviconPresenceFilter.value = ''
  httpStatusFilter.value = ''
  serviceNameFilter.value = ''
  transportProtocolFilter.value = ''
  columnFilterValues.value = {}
  search.value = ''
  clearSelection()
  reloadInventoryFromFirstPage()
  loadInventoryFacets()
}

const closeDetailModal = () => {
  showDetailModal.value = false
  selectedAssetId.value = null
}

const openEditModal = (item: any) => {
  editingAsset.value = item
  showEditModal.value = true
}

const closeEditModal = () => {
  showEditModal.value = false
  editingAsset.value = null
}

const openImportModal = () => {
  showImportModal.value = true
}

const closeImportModal = () => {
  showImportModal.value = false
}

const openExportModal = (options: { programId?: string | null; exportType?: SurfaceAssetExportType } = {}) => {
  if (options.programId !== undefined) {
    selectedProgramId.value = options.programId || ''
  }
  preferredExportType.value = options.exportType || (assetTypeFilter.value ? 'current' : 'all')
  showExportModal.value = true
}

const closeExportModal = () => {
  showExportModal.value = false
}

const getFaviconHash = (item: any) => String(item?.typed_details?.favicon_hash || '').trim()

const hasFaviconHash = (item: any) => getFaviconHash(item).length > 0

const openFaviconAssetsModal = (faviconHash?: string | null) => {
  const normalized = String(faviconHash || '').trim()
  if (!normalized) return
  selectedFaviconHash.value = normalized
  showFaviconAssetsModal.value = true
}

const closeFaviconAssetsModal = () => {
  showFaviconAssetsModal.value = false
  selectedFaviconHash.value = null
}

const formatInventoryValue = (item: any, column: any) => {
  const raw =
    column.detailKey != null
      ? item?.typed_details?.[column.detailKey]
      : item?.asset?.[column.valueKey]

  if (column.key === 'application_service_name') {
    const fallback = item?.typed_details?.protocol_name
    const preferred = raw ?? fallback
    if (preferred === null || preferred === undefined || preferred === '') return '-'
    return String(preferred)
  }

  if (raw === null || raw === undefined || raw === '') return '-'
  if (column.formatter === 'time') return formatTime(String(raw))
  if (column.formatter === 'domainLevel') return formatDomainLevel(raw)
  if (Array.isArray(raw)) return raw.join(', ')
  if (typeof raw === 'object') return JSON.stringify(raw)
  return String(raw)
}

const reloadInventoryFromFirstPage = () => {
  resetInventoryCursorPagination()
  if (inventoryPage.value !== 1) {
    inventoryPage.value = 1
    return
  }
  loadInventory()
}

const clearSelection = () => {
  selectedAssetIds.value = []
  selectedAssetTypeById.value = {}
}

const toggleAssetSelection = (assetId: string) => {
  if (!assetId) return
  if (!selectedAssetIds.value.includes(assetId)) {
    const item = inventoryItems.value.find((row) => row?.asset?.id === assetId)
    const assetType = String(item?.asset?.asset_type || '').trim()
    if (assetType) {
      selectedAssetTypeById.value[assetId] = assetType
    }
  } else {
    const nextTypes = { ...selectedAssetTypeById.value }
    delete nextTypes[assetId]
    selectedAssetTypeById.value = nextTypes
  }
  selectedAssetIds.value = selectedAssetIds.value.includes(assetId)
    ? selectedAssetIds.value.filter((id) => id !== assetId)
    : [...selectedAssetIds.value, assetId]
}

const toggleSelectCurrentPage = () => {
  if (!currentPageAssetIds.value.length) return
  if (allCurrentPageSelected.value) {
    const currentPageIdSet = new Set(currentPageAssetIds.value)
    selectedAssetIds.value = selectedAssetIds.value.filter((id) => !currentPageIdSet.has(id))
    const nextTypes = { ...selectedAssetTypeById.value }
    for (const assetId of currentPageAssetIds.value) {
      delete nextTypes[assetId]
    }
    selectedAssetTypeById.value = nextTypes
    return
  }

  const nextSelected = new Set(selectedAssetIds.value)
  const nextTypes = { ...selectedAssetTypeById.value }
  for (const item of inventoryItems.value) {
    const assetId = String(item?.asset?.id || '').trim()
    if (!assetId) continue
    nextSelected.add(assetId)
    const assetType = String(item?.asset?.asset_type || '').trim()
    if (assetType) {
      nextTypes[assetId] = assetType
    }
  }
  selectedAssetIds.value = Array.from(nextSelected)
  selectedAssetTypeById.value = nextTypes
}

const getInventoryFilterPayload = (programId = selectedProgramId.value || null) => ({
  program_id: programId,
  asset_type: assetTypeFilter.value || null,
  status: statusFilter.value || null,
  view_state: viewStateFilter.value || null,
  is_favorite: favoriteFilter.value ? true : null,
  search: search.value.trim() || null,
  has_favicon_hash: hasFaviconHashFilterValue.value,
  http_status_code: httpStatusFilterValue.value,
  service_name: serviceNameFilter.value || null,
  transport_protocol: transportProtocolFilter.value || null,
  column_filters: columnFilterPayload.value.length ? columnFilterPayload.value : null,
  limit: null,
  offset: null,
})

const refreshAfterMutation = async (deletedCount = 0) => {
  clearSelection()
  if (deletedCount > 0 && inventoryPage.value > 1 && deletedCount >= inventoryItems.value.length) {
    inventoryPage.value = Math.max(1, inventoryPage.value - 1)
  }
  await loadAll()
}

const deleteSingleAsset = async (asset: any) => {
  if (!asset?.id) return
  if (!(await dialog.confirm(t('bugBounty.surface.inventory.actions.confirmDeleteSingle', { name: asset.display_name || asset.asset_name })))) {
    return
  }

  try {
    await invoke('surface_delete_asset', { assetId: asset.id })
    toast.success(t('bugBounty.surface.inventory.actions.deleteSuccess', { count: 1 }))
    await refreshAfterMutation(1)
  } catch (error) {
    console.error('Failed to delete surface asset:', error)
    toast.error(t('bugBounty.surface.inventory.actions.deleteFailed'))
  }
}

const deleteSelectedAssets = async () => {
  if (!selectedAssetIds.value.length) return
  if (!(await dialog.confirm(t('bugBounty.surface.inventory.actions.confirmDeleteSelected', { count: selectedAssetIds.value.length })))) {
    return
  }

  try {
    bulkDeleting.value = true
    const deleted = await invoke<number>('surface_batch_delete_assets', { assetIds: selectedAssetIds.value })
    toast.success(t('bugBounty.surface.inventory.actions.deleteSuccess', { count: deleted }))
    await refreshAfterMutation(deleted)
  } catch (error) {
    console.error('Failed to batch delete surface assets:', error)
    toast.error(t('bugBounty.surface.inventory.actions.deleteFailed'))
  } finally {
    bulkDeleting.value = false
  }
}

const deleteAllFilteredAssets = async () => {
  if (!inventoryTotal.value) return
  const confirmKey = hasInventoryFilters.value
    ? 'bugBounty.surface.inventory.actions.confirmDeleteFiltered'
    : 'bugBounty.surface.inventory.actions.confirmDeleteAll'
  if (!(await dialog.confirm(t(confirmKey, { count: inventoryTotal.value })))) {
    return
  }

  try {
    bulkDeleting.value = true
    const deleted = await invoke<number>('surface_delete_inventory', { filter: getInventoryFilterPayload() })
    inventoryPage.value = 1
    toast.success(t('bugBounty.surface.inventory.actions.deleteSuccess', { count: deleted }))
    await refreshAfterMutation(deleted)
  } catch (error) {
    console.error('Failed to delete filtered surface inventory:', error)
    toast.error(t('bugBounty.surface.inventory.actions.deleteFailed'))
  } finally {
    bulkDeleting.value = false
  }
}

const markSelectedAssetsViewed = async () => {
  const newAssetIds = selectedAssetIds.value.filter((assetId) => {
    const item = inventoryItems.value.find((row) => row?.asset?.id === assetId)
    return isNewAsset(item?.asset)
  })
  if (!newAssetIds.length) {
    toast.warning(t('bugBounty.surface.inventory.actions.noNewSelected'))
    return
  }

  try {
    bulkDeleting.value = true
    const updated = await invoke<number>('surface_batch_mark_assets_viewed', { assetIds: newAssetIds })
    markAssetsViewedLocally(newAssetIds)
    await loadNewAssetCount(selectedProgramId.value || null)
    toast.success(t('bugBounty.surface.inventory.actions.markViewedSuccess', { count: updated }))
    clearSelection()
    if (viewStateFilter.value === 'new') {
      await loadAll()
    }
  } catch (error) {
    console.error('Failed to mark selected surface assets as viewed:', error)
    toast.error(t('bugBounty.surface.inventory.actions.markViewedFailed'))
  } finally {
    bulkDeleting.value = false
  }
}

const markFilteredAssetsViewed = async () => {
  if (!inventoryTotal.value) return
  if (!(await dialog.confirm(t('bugBounty.surface.inventory.actions.confirmMarkFilteredViewed', { count: inventoryTotal.value })))) {
    return
  }

  try {
    bulkDeleting.value = true
    const updated = await invoke<number>('surface_mark_inventory_viewed', { filter: getInventoryFilterPayload() })
    toast.success(t('bugBounty.surface.inventory.actions.markViewedSuccess', { count: updated }))
    clearSelection()
    await loadAll()
  } catch (error) {
    console.error('Failed to mark filtered surface assets as viewed:', error)
    toast.error(t('bugBounty.surface.inventory.actions.markViewedFailed'))
  } finally {
    bulkDeleting.value = false
  }
}

const saveAssetEdit = async (payload: SurfaceAssetEditPayload) => {
  const assetId = editingAsset.value?.asset?.id
  if (!assetId) return

  try {
    await invoke('surface_update_asset', { assetId, request: payload })
    toast.success(t('bugBounty.surface.inventory.actions.updateSuccess'))
    closeEditModal()
    await loadAll()
  } catch (error) {
    console.error('Failed to update surface asset:', error)
    toast.error(t('bugBounty.surface.inventory.actions.updateFailed'))
  }
}

const submitImport = async (payload: SurfaceAssetImportPayload) => {
  try {
    importing.value = true
    const result = await invoke<{ requested: number; created: number; skipped: number; scopes_created: number }>('surface_manual_import_assets', {
      request: payload,
    })

    if (result.created > 0 && result.skipped > 0 && result.scopes_created > 0) {
      toast.success(
        t('bugBounty.surface.inventory.import.partialSuccessWithScopes', {
          created: result.created,
          skipped: result.skipped,
          scopes: result.scopes_created,
        }),
      )
    } else if (result.created > 0 && result.scopes_created > 0) {
      toast.success(
        t('bugBounty.surface.inventory.import.successWithScopes', {
          count: result.created,
          scopes: result.scopes_created,
        }),
      )
    } else if (result.scopes_created > 0) {
      toast.success(
        t('bugBounty.surface.inventory.import.scopeOnlySuccess', {
          scopes: result.scopes_created,
        }),
      )
    } else if (result.created > 0 && result.skipped > 0) {
      toast.success(
        t('bugBounty.surface.inventory.import.partialSuccess', {
          created: result.created,
          skipped: result.skipped,
        }),
      )
    } else if (result.created > 0) {
      toast.success(t('bugBounty.surface.inventory.import.success', { count: result.created }))
    } else {
      toast.warning(t('bugBounty.surface.inventory.import.noop', { count: result.skipped || result.requested }))
    }

    closeImportModal()
    await loadAll()
  } catch (error) {
    console.error('Failed to manually import surface assets:', error)
    toast.error(t('bugBounty.surface.inventory.import.failed'))
  } finally {
    importing.value = false
  }
}

const sendAssetToAssistant = async (assetId?: string | null) => {
  if (!assetId) return

  try {
    const detail = await invoke<any>('surface_get_asset_detail', { assetId })
    if (!detail?.asset) {
      toast.warning(t('bugBounty.surface.inventory.actions.assistantMissing'))
      return
    }

    queueSurfaceAssetForAssistant(buildReferencedSurfaceAsset(detail))
    toast.success(t('bugBounty.surface.inventory.actions.assistantSuccess'))
    await router.push('/ai-assistant')
  } catch (error) {
    console.error('Failed to send surface asset to AI assistant:', error)
    toast.error(t('bugBounty.surface.inventory.actions.assistantFailed'))
  }
}

const buildExportFilterPayload = (exportType: SurfaceAssetExportType) => {
  const useCurrentType = exportType === 'current'
  const resolvedAssetType = useCurrentType
    ? assetTypeFilter.value || null
    : exportType === 'all' || exportType === 'api'
      ? null
      : exportType
  const useServiceFacetFilters =
    (useCurrentType && assetTypeFilter.value === 'service') || exportType === 'service'
  const useWebFaviconFilter =
    (useCurrentType && assetTypeFilter.value === 'web') || exportType === 'web'
  const columnFilterAssetType = useCurrentType
    ? assetTypeFilter.value || null
    : resolvedAssetType || assetTypeFilter.value || null
  const exportColumnFilters = buildColumnFilterPayload(columnFilterAssetType)

  return {
    program_id: selectedProgramId.value || null,
    asset_type: resolvedAssetType,
    status: statusFilter.value || null,
    view_state: viewStateFilter.value || null,
    is_favorite: favoriteFilter.value ? true : null,
    search: search.value.trim() || null,
    has_favicon_hash: useWebFaviconFilter ? hasFaviconHashFilterValue.value : null,
    http_status_code:
      ((useCurrentType && assetTypeFilter.value === 'web') || exportType === 'web')
        ? httpStatusFilterValue.value
        : null,
    service_name: useServiceFacetFilters ? serviceNameFilter.value || null : null,
    transport_protocol: useServiceFacetFilters ? transportProtocolFilter.value || null : null,
    column_filters: exportColumnFilters.length ? exportColumnFilters : null,
    limit: null,
    offset: null,
  }
}

const buildExportFilename = (exportType: SurfaceAssetExportType, format: 'csv' | 'json') => {
  const programPart = sanitizeExportFilenamePart(selectedProgramName.value || 'all_programs')
  const typePart = sanitizeExportFilenamePart(
    exportType === 'current' ? assetTypeFilter.value || 'current' : exportType,
  )
  return `surface-assets-${programPart}-${typePart}-${Date.now()}.${format}`
}

const exportInventoryAssets = async (payload: SurfaceAssetExportPayload) => {
  try {
    exportingAssets.value = true
    const filePath = await save({
      defaultPath: buildExportFilename(payload.exportType, payload.format),
      filters: [{ name: payload.format.toUpperCase(), extensions: [payload.format] }],
    })
    if (!filePath) return

    const result = await invoke<{ exported: number; output_path: string }>('surface_export_inventory', {
      request: {
        filter: buildExportFilterPayload(payload.exportType),
        export_type: payload.exportType,
        format: payload.format,
        output_path: filePath,
      },
    })
    closeExportModal()
    if (!result.exported) {
      toast.warning(t('bugBounty.surface.inventory.export.empty'))
      return
    }
    toast.success(t('bugBounty.surface.inventory.export.success', { count: result.exported }))
  } catch (error) {
    console.error('Failed to export surface inventory assets:', error)
    toast.error(t('bugBounty.surface.inventory.export.failed'))
  } finally {
    exportingAssets.value = false
  }
}

const setInventoryPage = (page: number) => {
  const maxKnownPage = inventoryHasNext.value
    ? Math.max(inventoryPage.value + 1, inventoryPageCount.value)
    : Math.max(inventoryPage.value, inventoryPageCount.value)
  const nextPage = Math.min(Math.max(1, page), maxKnownPage)
  if (nextPage > inventoryPage.value && !inventoryNextCursorByPage.value[inventoryPage.value]) {
    return
  }
  if (inventoryPage.value === nextPage) {
    loadInventory()
    return
  }
  inventoryPage.value = nextPage
}

const goToFirstInventoryPage = () => {
  if (inventoryLoading.value || inventoryPage.value <= 1) return
  resetInventoryCursorPagination()
  setInventoryPage(1)
}

const goToPreviousInventoryPage = () => {
  if (inventoryPage.value <= 1 || inventoryLoading.value) return
  setInventoryPage(inventoryPage.value - 1)
}

const goToNextInventoryPage = () => {
  if (!inventoryHasNext.value || inventoryLoading.value) return
  setInventoryPage(inventoryPage.value + 1)
}

const setRunPage = (page: number) => {
  const nextPage = Math.min(Math.max(1, page), runPageCount.value)
  runPage.value = nextPage
}

const goToFirstRunPage = () => {
  if (loading.value || runPage.value <= 1) return
  setRunPage(1)
}

const goToPreviousRunPage = () => {
  if (runPage.value <= 1 || loading.value) return
  setRunPage(runPage.value - 1)
}

const goToNextRunPage = () => {
  if (runPage.value >= runPageCount.value || loading.value) return
  setRunPage(runPage.value + 1)
}

const goToLastRunPage = () => {
  if (loading.value || runPage.value >= runPageCount.value) return
  setRunPage(runPageCount.value)
}

const applyRunPageJump = () => {
  if (loading.value) return
  const page = Number.parseInt(runPageInput.value, 10)
  if (Number.isNaN(page)) {
    runPageInput.value = String(runPage.value)
    return
  }
  setRunPage(page)
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

const formatDomainLevel = (value: unknown) => {
  const level = Number(value)
  if (!Number.isInteger(level) || level < 0) return '-'
  if (level === 0) return t('bugBounty.surface.inventory.domainLevels.root')
  if (level === 1) return t('bugBounty.surface.inventory.domainLevels.level1')
  if (level === 2) return t('bugBounty.surface.inventory.domainLevels.level2')
  return t('bugBounty.surface.inventory.domainLevels.level3Plus')
}

watch(
  () => props.programId,
  (value) => {
    selectedProgramId.value = value || ''
  },
)

watch(selectedProgramId, () => {
  clearSelection()
  resetInventoryCursorPagination()
  inventoryPage.value = 1
  runPage.value = 1
  runPageInput.value = '1'
  loadAll()
})

watch([assetTypeFilter, statusFilter, viewStateFilter, favoriteFilter, faviconPresenceFilter, httpStatusFilter, serviceNameFilter, transportProtocolFilter, columnFilterValues], () => {
  clearSelection()
  reloadInventoryFromFirstPage()
  loadInventoryTotal()
  loadInventoryFacets()
})

watch(assetTypeFilter, (value) => {
  clearInvalidColumnFilters(value || null)
  if (value !== 'web') {
    faviconPresenceFilter.value = ''
    httpStatusFilter.value = ''
    httpStatusOptions.value = []
  }
  if (value !== 'service') {
    serviceNameFilter.value = ''
    transportProtocolFilter.value = ''
    serviceNameOptions.value = []
    transportProtocolOptions.value = []
  }
  if (value === 'service' || value === 'web') {
    loadInventoryFacets()
  }
})

watch(inventoryPage, () => {
  loadInventory()
})

watch(inventoryPageSize, () => {
  reloadInventoryFromFirstPage()
  loadInventoryTotal()
})

watch(runPage, () => {
  runPageInput.value = String(runPage.value)
})

watch(runPageSize, () => {
  runPageInput.value = '1'
  runPage.value = 1
})

watch(search, () => {
  if (searchDebounceTimer) {
    clearTimeout(searchDebounceTimer)
  }

  searchDebounceTimer = setTimeout(() => {
    clearSelection()
    reloadInventoryFromFirstPage()
    loadInventoryTotal()
    loadInventoryFacets()
  }, 250)
})

onBeforeUnmount(() => {
  if (searchDebounceTimer) {
    clearTimeout(searchDebounceTimer)
  }
})

onMounted(() => {
  runPageInput.value = String(runPage.value)
  loadAll()
})

defineExpose({
  openExportModal,
})
</script>
