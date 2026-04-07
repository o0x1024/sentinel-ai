<template>
  <div class="space-y-6">
    <VulnerabilitiesStatsOverview :stats="stats" :lifecycle-stats="lifecycleStats" />

    <VulnerabilitiesEvaluationSummaryPanel
      :current="evaluationComparison"
      :history="evaluationComparisonHistory"
      @select-history="selectEvaluationHistoryEntry"
      @clear-current="clearEvaluationComparison"
      @clear-history="clearEvaluationComparisonHistory"
    />

    <!-- 筛选器 -->
    <div class="bg-base-100 rounded-lg p-4 shadow-sm border border-base-300">
      <div class="mb-3 flex flex-wrap items-center gap-2">
        <span class="text-sm text-base-content/70">视图</span>
        <div class="join">
          <button
            v-for="option in lifecycleViewOptions"
            :key="option.value"
            class="join-item btn btn-sm"
            :class="filters.lifecycleView === option.value ? 'btn-primary' : 'btn-outline'"
            @click="setLifecycleView(option.value)"
          >
            {{ option.label }}
          </button>
        </div>
      </div>
      <div class="flex flex-wrap gap-3 items-center">
        <!-- 批量操作 -->
        <div v-if="selectedIds.size > 0" class="flex items-center gap-2 mr-auto">
          <span class="text-sm text-base-content/70">已选择 {{ selectedIds.size }} 项</span>
          <button @click="deleteSelected" class="btn btn-error btn-sm">
            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path
                stroke-linecap="round"
                stroke-linejoin="round"
                stroke-width="2"
                d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"
              ></path>
            </svg>
            删除选中
          </button>
          <button
            @click="deleteAll"
            class="btn btn-error btn-outline btn-sm"
            :disabled="totalCount === 0"
          >
            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path
                stroke-linecap="round"
                stroke-linejoin="round"
                stroke-width="2"
                d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"
              ></path>
            </svg>
            删除全部
          </button>
          <button @click="selectedIds.clear()" class="btn btn-ghost btn-sm">取消选择</button>
        </div>

        <div class="form-control">
          <select
            v-model="filters.severity"
            class="select select-bordered select-sm"
            @change="applyFilters"
          >
            <option value="">{{ $t('vulnerabilities.allSeverities') }}</option>
            <option value="critical">{{ $t('vulnerabilities.severity.critical') }}</option>
            <option value="high">{{ $t('vulnerabilities.severity.high') }}</option>
            <option value="medium">{{ $t('vulnerabilities.severity.medium') }}</option>
            <option value="low">{{ $t('vulnerabilities.severity.low') }}</option>
          </select>
        </div>
        <div class="form-control">
          <select
            v-model="filters.status"
            class="select select-bordered select-sm"
            @change="applyStatusFilter"
          >
            <option value="">全部阶段</option>
            <option value="candidate">候选待验证</option>
            <option value="reviewed">已验证</option>
            <option value="false_positive">误报</option>
            <option value="open">开放</option>
            <option value="fixed">已修复</option>
          </select>
        </div>
        <div class="form-control flex-1">
          <input
            v-model="filters.search"
            type="text"
            :placeholder="$t('common.search') + '...'"
            class="input input-bordered input-sm"
            @input="applyFilters"
          />
        </div>
        <button @click="refreshFindings" class="btn btn-outline btn-sm">
          <svg class="w-4 h-4 mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"
            ></path>
          </svg>
          {{ $t('common.refresh') }}
        </button>
        <button
          @click="exportCurrentSnapshot"
          class="btn btn-outline btn-sm"
          :disabled="exportingSnapshot"
        >
          <span v-if="exportingSnapshot" class="loading loading-spinner loading-xs mr-1"></span>
          <svg v-else class="w-4 h-4 mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M12 16V4m0 12l-4-4m4 4l4-4M4 20h16"
            ></path>
          </svg>
          导出评测快照
        </button>
        <button @click="loadEvaluationComparison" class="btn btn-outline btn-sm">
          <svg class="w-4 h-4 mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M12 4v16m8-8H4"
            ></path>
          </svg>
          导入评测对照
        </button>
      </div>
      <details class="mt-3 rounded-lg border border-base-300 bg-base-200/40">
        <summary class="cursor-pointer select-none px-4 py-3 text-sm font-medium">
          System Agent 扩展筛选与保存视图
        </summary>
        <div class="space-y-3 border-t border-base-300 px-4 py-4">
          <p class="text-xs text-base-content/60">
            SQLi、RCE、SSRF 等普通漏洞通常不需要下面这些筛选。只有在排查行为分析、语义抽象、逻辑漏洞候选时再展开。
          </p>
          <VulnerabilitiesSavedViewsBar
            :presets="filterPresets"
            :active-preset-id="activeFilterPreset"
            :saved-views="savedViews"
            :active-saved-view-id="activeSavedViewId"
            @apply-preset="applyFilterPreset"
            @reset-filters="resetAdvancedFilters"
            @apply-saved-view="applySavedView"
            @save-current-view="saveCurrentView"
            @export-saved-views="exportSavedViews"
            @import-saved-views="importSavedViews"
            @delete-saved-view="deleteSavedView"
          />
          <div class="flex flex-wrap gap-3 items-center">
            <div class="form-control">
              <select
                v-model="filters.semanticSource"
                class="select select-bordered select-sm"
                @change="applyFilters"
              >
                <option value="">全部语义来源</option>
                <option value="ai_augmented">AI 增强</option>
                <option value="fallback">确定性回退</option>
              </select>
            </div>
            <div class="form-control">
              <select
                v-model="filters.hypothesisRiskType"
                class="select select-bordered select-sm"
                @change="applyHypothesisRiskTypeFilter"
              >
                <option value="">全部假设类型</option>
                <option value="idor">IDOR</option>
                <option value="bola">BOLA</option>
                <option value="bfla">BFLA</option>
                <option value="workflow">流程</option>
                <option value="logic">逻辑</option>
                <option value="race">竞态</option>
              </select>
            </div>
          </div>
        </div>
      </details>
    </div>

    <!-- 漏洞列表 -->
    <div class="bg-base-100 rounded-lg shadow-sm border border-base-300 overflow-hidden">
      <div v-if="isLoading" class="text-center py-8">
        <span class="loading loading-spinner loading-lg"></span>
        <p class="mt-2 text-sm">{{ $t('common.loading') }}...</p>
      </div>

      <div v-else-if="findings.length > 0" class="overflow-x-auto">
        <table class="table table-zebra">
          <thead>
            <tr>
              <th>
                <label>
                  <input
                    type="checkbox"
                    class="checkbox checkbox-sm"
                    :checked="isAllSelected"
                    @change="toggleSelectAll"
                  />
                </label>
              </th>
              <th>{{ $t('vulnerabilities.severity.title') }}</th>
              <th>{{ $t('vulnerabilities.title') }}</th>
              <th>类型</th>
              <th>{{ $t('common.url') }}</th>
              <th>{{ $t('vulnerabilities.plugin') }}</th>
              <th>命中</th>
              <th>{{ $t('common.time') }}</th>
              <th>{{ $t('common.actions') }}</th>
            </tr>
          </thead>
          <tbody>
            <VulnerabilityFindingRow
              v-for="finding in findings"
              :key="finding.id"
              :finding="finding"
              :selected="selectedIds.has(finding.id)"
              :verifying-id="verifyingId"
              @toggle-select="toggleSelect"
              @open-details="openDetails"
              @verify="verifyWithSystemAgent"
              @delete="deleteSingle"
            />
          </tbody>
        </table>

        <!-- 分页 -->
        <div
          v-if="totalCount > 0"
          class="flex flex-col gap-3 py-4 px-4 xl:flex-row xl:items-center xl:justify-between"
        >
          <div class="flex items-center gap-2 text-sm">
            <span class="text-base-content/70">{{
              $t('vulnerabilities.pagination.pageSize')
            }}</span>
            <select
              v-model.number="pageSize"
              class="select select-bordered select-sm"
              :disabled="isLoading"
            >
              <option v-for="size in pageSizeOptions" :key="size" :value="size">
                {{ size }}
              </option>
            </select>
            <span class="text-base-content/70">
              {{ $t('vulnerabilities.pagination.totalRecords', { total: totalCount }) }}
            </span>
          </div>
          <div class="flex flex-col gap-2 sm:flex-row sm:flex-wrap sm:items-center sm:justify-end">
            <div class="join">
              <button
                class="join-item btn btn-sm"
                :disabled="currentPage <= 1 || isLoading"
                @click="goToFirstPage"
              >
                {{ $t('vulnerabilities.pagination.firstPage') }}
              </button>
              <button
                class="join-item btn btn-sm"
                :disabled="currentPage <= 1 || isLoading"
                @click="goToPreviousPage"
              >
                {{ $t('common.previous') }}
              </button>
              <button class="join-item btn btn-sm">
                {{
                  $t('vulnerabilities.pagination.pageInfo', {
                    page: currentPage,
                    total: totalPages,
                  })
                }}
              </button>
              <button
                class="join-item btn btn-sm"
                :disabled="currentPage >= totalPages || isLoading"
                @click="goToNextPage"
              >
                {{ $t('common.next') }}
              </button>
              <button
                class="join-item btn btn-sm"
                :disabled="currentPage >= totalPages || isLoading"
                @click="goToLastPage"
              >
                {{ $t('vulnerabilities.pagination.lastPage') }}
              </button>
            </div>

            <div class="flex items-center gap-2">
              <input
                v-model="pageInput"
                type="number"
                min="1"
                :max="totalPages"
                class="input input-bordered input-sm w-24"
                :placeholder="$t('vulnerabilities.pagination.jumpPlaceholder')"
                @keyup.enter="applyPageJump"
              />
              <button class="btn btn-sm btn-outline" :disabled="isLoading" @click="applyPageJump">
                {{ $t('vulnerabilities.pagination.jump') }}
              </button>
            </div>
          </div>
        </div>
      </div>

      <div v-else class="text-center py-8 text-base-content/50">
        <svg
          class="w-16 h-16 mx-auto mb-2 opacity-30"
          fill="none"
          stroke="currentColor"
          viewBox="0 0 24 24"
        >
          <path
            stroke-linecap="round"
            stroke-linejoin="round"
            stroke-width="2"
            d="M20 13V6a2 2 0 00-2-2H6a2 2 0 00-2 2v7m16 0v5a2 2 0 01-2 2H6a2 2 0 01-2-2v-5m16 0h-2.586a1 1 0 00-.707.293l-2.414 2.414a1 1 0 01-.707.293h-3.172a1 1 0 01-.707-.293l-2.414-2.414A1 1 0 006.586 13H4"
          ></path>
        </svg>
        <p class="text-sm">{{ $t('vulnerabilities.noFindings') }}</p>
      </div>
    </div>

    <!-- 详情模态框 -->
    <dialog
      :class="['modal', { 'modal-open': showDetailsModal }]"
      @click.self="closeDetails"
      @keydown.esc="closeDetails"
    >
      <div class="modal-box w-11/12 max-w-5xl max-h-[90vh] overflow-y-auto">
        <div v-if="selectedFinding">
          <div class="flex justify-between items-start mb-4 sticky top-0 bg-base-100 z-10 pb-2">
            <h3 class="font-bold text-lg">{{ $t('vulnerabilities.details') }}</h3>
            <button @click="closeDetails" class="btn btn-sm btn-circle btn-ghost">✕</button>
          </div>

          <div class="space-y-4">
            <div class="tabs tabs-boxed">
              <button
                v-for="tab in detailTabs"
                :key="tab.id"
                class="tab"
                :class="{ 'tab-active': detailTab === tab.id }"
                @click="detailTab = tab.id"
              >
                {{ tab.label }}
              </button>
            </div>

            <VulnerabilityDetailOverview
              v-if="detailTab === 'overview'"
              :finding="selectedFinding"
            />

            <VulnerabilityEvidenceList
              v-else-if="detailTab === 'evidence'"
              :finding="selectedFinding"
            />

            <VulnerabilitySystemAgentPanel
              v-else-if="detailTab === 'system_agent'"
              :finding="selectedFinding"
              :feedbacking-id="feedbackingId"
              @feedback="({ findingId, feedbackType }) => submitSystemAgentFeedback(findingId, feedbackType)"
            />

            <VulnerabilityTimelinePanel
              v-else
              :finding="selectedFinding"
            />
          </div>
        </div>

        <div class="modal-action sticky bottom-0 bg-base-100 pt-4">
          <button @click="closeDetails" class="btn btn-sm">{{ $t('common.close') }}</button>
        </div>
      </div>
      <form method="dialog" class="modal-backdrop">
        <button @click="closeDetails">close</button>
      </form>
    </dialog>

    <!-- 删除全部确认对话框 -->
    <dialog :class="['modal', { 'modal-open': showDeleteAllModal }]">
      <div class="modal-box">
        <h3 class="font-bold text-lg text-error">
          <svg
            class="w-6 h-6 inline-block mr-2"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"
            ></path>
          </svg>
          确认删除
        </h3>
        <p class="py-4">
          确定要清空所有 <span class="font-bold text-error">{{ totalCount }}</span> 条漏洞记录吗？
        </p>
        <p class="text-sm text-warning pb-4">
          ⚠️ 此操作不可恢复，所有漏洞记录及相关证据将被永久删除！
        </p>
        <div class="modal-action">
          <button @click="showDeleteAllModal = false" class="btn btn-ghost">取消</button>
          <button @click="confirmDeleteAll" class="btn btn-error">
            <svg class="w-4 h-4 mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path
                stroke-linecap="round"
                stroke-linejoin="round"
                stroke-width="2"
                d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"
              ></path>
            </svg>
            确认删除
          </button>
        </div>
      </div>
      <form method="dialog" class="modal-backdrop">
        <button @click="showDeleteAllModal = false">close</button>
      </form>
    </dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { exportFindingSnapshot } from './vulnerabilitiesExportSupport'
import {
  clearPersistedEvaluationComparisonHistory,
  clearPersistedEvaluationComparison,
  importEvaluationComparison,
  loadPersistedEvaluationComparison,
  loadPersistedEvaluationComparisonHistory,
  persistEvaluationComparison,
  persistEvaluationComparisonHistory,
  type EvaluationComparisonSummary,
} from './vulnerabilitiesEvaluationSupport'
import {
  findEvaluationHistoryEntry,
  mergeEvaluationHistory,
} from './vulnerabilitiesEvaluationHistorySupport'
import {
  detectActiveFilterPreset,
  filterPresets, type FindingFilterPresetId, type VulnerabilityLifecycleView,
} from './vulnerabilitiesFilterPresets'
import VulnerabilitiesSavedViewsBar from './VulnerabilitiesSavedViewsBar.vue'
import {
  cloneFilterState, confirmAndDeleteSavedVulnerabilityFilterView, detectActiveSavedViewId,
  exportSavedVulnerabilityFilterViews, importSavedVulnerabilityFilterViews,
  loadSavedVulnerabilityFilterViews, mergeSavedVulnerabilityFilterViews,
  promptAndSaveVulnerabilityFilterView, saveSavedVulnerabilityFilterViews,
  type SavedVulnerabilityFilterView,
} from './vulnerabilitiesSavedViews'
import VulnerabilityDetailOverview from './VulnerabilityDetailOverview.vue'
import VulnerabilityEvidenceList from './VulnerabilityEvidenceList.vue'
import VulnerabilityFindingRow from './VulnerabilityFindingRow.vue'
import VulnerabilitySystemAgentPanel from './VulnerabilitySystemAgentPanel.vue'
import VulnerabilityTimelinePanel from './VulnerabilityTimelinePanel.vue'
import VulnerabilitiesStatsOverview from './VulnerabilitiesStatsOverview.vue'
import VulnerabilitiesEvaluationSummaryPanel from './VulnerabilitiesEvaluationSummaryPanel.vue'
import type { Finding } from './vulnerabilityFindingTypes'
import { isSystemAgentFinding } from './vulnerabilityFindingPresentation'

const { t } = useI18n()
const emit = defineEmits<{
  'stats-updated': [stats: { total: number; critical: number }]
}>()

const findings = ref<Finding[]>([])
const isLoading = ref(false)
const showDetailsModal = ref(false)
const showDeleteAllModal = ref(false)
const selectedFinding = ref<Finding | null>(null)
const selectedIds = ref<Set<string>>(new Set())
const verifyingId = ref<string | null>(null)
const feedbackingId = ref<string | null>(null)
type DetailTabId = 'overview' | 'evidence' | 'system_agent' | 'timeline'
const detailTab = ref<DetailTabId>('overview')
const exportingSnapshot = ref(false)
const evaluationComparison = ref<EvaluationComparisonSummary | null>(null)
const evaluationComparisonHistory = ref<EvaluationComparisonSummary[]>([])

const stats = ref({ critical: 0, high: 0, medium: 0, low: 0 })
const lifecycleStats = ref({ candidate: 0, verified: 0, falsePositive: 0 })
const savedViews = ref<SavedVulnerabilityFilterView[]>(loadSavedVulnerabilityFilterViews())

const filters = ref({
  severity: '',
  status: '',
  lifecycleView: 'formal' as VulnerabilityLifecycleView,
  search: '',
  semanticSource: '',
  hypothesisRiskType: '',
  hypothesisRiskTypes: [] as string[],
})

const lifecycleViewOptions: Array<{ value: VulnerabilityLifecycleView; label: string }> = [
  { value: 'formal', label: '正式漏洞' }, { value: 'candidate', label: '候选待验证' },
  { value: 'verified', label: '已验证' }, { value: 'false_positive', label: '误报' },
  { value: 'all', label: '全部' },
]
const currentPage = ref(1)
const pageSize = ref(10)
const pageInput = ref('1')
const pageSizeOptions = [10, 20, 50, 100]
const totalCount = ref(0) // 总条数
const totalPages = computed(() => Math.max(1, Math.ceil(totalCount.value / pageSize.value)))
const isAllSelected = computed(
  () => findings.value.length > 0 && findings.value.every(f => selectedIds.value.has(f.id))
)
const activeFilterPreset = computed<FindingFilterPresetId | null>(() => detectActiveFilterPreset(filters.value))
const activeSavedViewId = computed<string | null>(() => detectActiveSavedViewId(filters.value, savedViews.value))
const detailTabs = computed<Array<{ id: DetailTabId; label: string }>>(() => {
  const tabs: Array<{ id: DetailTabId; label: string }> = [
    { id: 'overview', label: '概览' },
    { id: 'evidence', label: '证据' },
  ]
  if (selectedFinding.value && isSystemAgentFinding(selectedFinding.value)) {
    tabs.push({ id: 'system_agent', label: 'System Agent' })
  }
  tabs.push({ id: 'timeline', label: '时间线' })
  return tabs
})

const resolveStatusFilters = () => {
  if (filters.value.status) {
    return {
      statusFilter: filters.value.status,
      statusFilters: null as string[] | null,
      analysisStageFilters: null as string[] | null,
    }
  }

  switch (filters.value.lifecycleView) {
    case 'candidate':
      return { statusFilter: null, statusFilters: null, analysisStageFilters: ['hypothesis'] }
    case 'verified':
      return { statusFilter: null, statusFilters: null, analysisStageFilters: ['verified'] }
    case 'false_positive':
      return { statusFilter: null, statusFilters: null, analysisStageFilters: ['false_positive'] }
    case 'formal':
      return {
        statusFilter: null,
        statusFilters: null,
        analysisStageFilters: ['formal_open', 'verified', 'fixed'],
      }
    default:
      return { statusFilter: null, statusFilters: null, analysisStageFilters: null }
  }
}

const countFindings = async (
  severityFilter: string | null,
  statusFilter: string | null = null,
  statusFilters: string[] | null = null,
  analysisStageFilters: string[] | null = null,
  search: string | null = null,
  semanticSourceFilter: string | null = null,
  hypothesisRiskTypeFilter: string | null = null,
  hypothesisRiskTypeFilters: string[] | null = null
) => {
  const response = await invoke<any>('count_findings', {
    severityFilter,
    statusFilter,
    statusFilters,
    analysisStageFilters,
    search,
    semanticSourceFilter,
    hypothesisRiskTypeFilter,
    hypothesisRiskTypeFilters,
  })
  return response.success ? Number(response.data || 0) : 0
}

const updateStats = (nextStats: {
  total: number
  critical: number
  high: number
  medium: number
  low: number
}) => {
  stats.value = {
    critical: nextStats.critical,
    high: nextStats.high,
    medium: nextStats.medium,
    low: nextStats.low,
  }

  emit('stats-updated', {
    total: nextStats.total,
    critical: nextStats.critical,
  })
}

const refreshFindings = async () => {
  isLoading.value = true
  try {
    const severityFilter = filters.value.severity || null
    const { statusFilter, statusFilters, analysisStageFilters } = resolveStatusFilters()
    const search = filters.value.search.trim() || null
    const semanticSourceFilter = filters.value.semanticSource || null
    const hypothesisRiskTypeFilter = filters.value.hypothesisRiskType || null
    const hypothesisRiskTypeFilters = filters.value.hypothesisRiskTypes.length
      ? filters.value.hypothesisRiskTypes
      : null
    const offset = (currentPage.value - 1) * pageSize.value
    const [filteredTotal, lifecycleStatsResponse, response] = await Promise.all([
      countFindings(
        severityFilter,
        statusFilter,
        statusFilters,
        analysisStageFilters,
        search,
        semanticSourceFilter,
        hypothesisRiskTypeFilter,
        hypothesisRiskTypeFilters
      ),
      invoke<any>('get_finding_lifecycle_stats'),
      invoke<any>('list_findings', {
        limit: pageSize.value,
        offset,
        severityFilter,
        statusFilter,
        statusFilters,
        analysisStageFilters,
        search,
        semanticSourceFilter,
        hypothesisRiskTypeFilter,
        hypothesisRiskTypeFilters,
      }),
    ])

    totalCount.value = filteredTotal
    if (lifecycleStatsResponse.success && lifecycleStatsResponse.data) {
      const metrics = lifecycleStatsResponse.data
      updateStats({
        total: Number(metrics.formalTotal || 0),
        critical: Number(metrics.critical || 0),
        high: Number(metrics.high || 0),
        medium: Number(metrics.medium || 0),
        low: Number(metrics.low || 0),
      })
      lifecycleStats.value = {
        candidate: Number(metrics.candidate || 0),
        verified: Number(metrics.verified || 0),
        falsePositive: Number(metrics.falsePositive || 0),
      }
    } else {
      updateStats({ total: 0, critical: 0, high: 0, medium: 0, low: 0 })
      lifecycleStats.value = { candidate: 0, verified: 0, falsePositive: 0 }
    }

    const nextTotalPages = Math.max(1, Math.ceil(filteredTotal / pageSize.value))
    if (currentPage.value > nextTotalPages) {
      currentPage.value = nextTotalPages
      return
    }

    if (response.success && response.data) {
      findings.value = response.data
        .map((item: any) => {
          if (!item.id || !item.severity) {
            console.error('Missing required fields in item:', item)
            return null
          }

          return {
            ...item,
            evidence: item.evidence || [],
          }
        })
        .filter((f: any) => f !== null)

      syncSelectedFinding()
      return
    }

    findings.value = []
  } catch (error) {
    console.error('Failed to refresh findings:', error)
    findings.value = []
    totalCount.value = 0
    updateStats({ total: 0, critical: 0, high: 0, medium: 0, low: 0 })
    lifecycleStats.value = { candidate: 0, verified: 0, falsePositive: 0 }
  } finally {
    isLoading.value = false
  }
}

const setLifecycleView = (view: VulnerabilityLifecycleView) => {
  if (filters.value.lifecycleView === view) return
  filters.value.lifecycleView = view
  filters.value.status = ''
  pageInput.value = '1'
  selectedIds.value.clear()
  if (currentPage.value !== 1) {
    currentPage.value = 1
    return
  }
  refreshFindings()
}

const applyFilters = () => {
  pageInput.value = '1'
  selectedIds.value.clear()
  if (currentPage.value !== 1) {
    currentPage.value = 1
    return
  }
  refreshFindings()
}

const applyStatusFilter = () => {
  filters.value.lifecycleView = filters.value.status ? 'all' : filters.value.lifecycleView
  applyFilters()
}

const applyHypothesisRiskTypeFilter = () => {
  filters.value.hypothesisRiskTypes = filters.value.hypothesisRiskType
    ? [filters.value.hypothesisRiskType]
    : []
  applyFilters()
}

const resetAdvancedFilters = () => {
  filters.value.semanticSource = ''
  filters.value.hypothesisRiskType = ''
  filters.value.hypothesisRiskTypes = []
  filters.value.search = ''
  filters.value.severity = ''
  filters.value.status = ''
  filters.value.lifecycleView = 'formal'
  applyFilters()
}

const applyFilterPreset = (presetId: FindingFilterPresetId) => {
  filters.value.severity = ''
  filters.value.status = ''
  filters.value.search = ''
  if (presetId === 'high_value_candidate') {
    filters.value.lifecycleView = 'candidate'
    filters.value.semanticSource = 'ai_augmented'
    filters.value.hypothesisRiskType = ''
    filters.value.hypothesisRiskTypes = ['workflow', 'race']
  } else if (presetId === 'object_boundary_candidate') {
    filters.value.lifecycleView = 'candidate'
    filters.value.semanticSource = 'ai_augmented'
    filters.value.hypothesisRiskType = ''
    filters.value.hypothesisRiskTypes = ['idor', 'bola', 'bfla']
  } else {
    filters.value.lifecycleView = 'all'
    filters.value.semanticSource = 'ai_augmented'
    filters.value.hypothesisRiskType = ''
    filters.value.hypothesisRiskTypes = []
  }
  applyFilters()
}

const applySavedView = (viewId: string | null) => {
  const target = savedViews.value.find(view => view.id === viewId)
  if (!target) return
  filters.value = cloneFilterState(target.filters)
  applyFilters()
}

const saveCurrentView = () => {
  savedViews.value = promptAndSaveVulnerabilityFilterView({
    views: savedViews.value,
    activeViewId: activeSavedViewId.value,
    filters: filters.value,
  })
  saveSavedVulnerabilityFilterViews(savedViews.value)
}

const deleteSavedView = (viewId: string | null) => {
  savedViews.value = confirmAndDeleteSavedVulnerabilityFilterView({
    views: savedViews.value,
    viewId,
  })
  saveSavedVulnerabilityFilterViews(savedViews.value)
}

const exportSavedViews = async () => {
  try {
    await exportSavedVulnerabilityFilterViews(savedViews.value)
  } catch (error) {
    console.error('Failed to export saved views:', error)
  }
}

const importSavedViews = async () => {
  try {
    const imported = await importSavedVulnerabilityFilterViews()
    if (!imported.length) return
    savedViews.value = mergeSavedVulnerabilityFilterViews(savedViews.value, imported)
    saveSavedVulnerabilityFilterViews(savedViews.value)
  } catch (error) {
    console.error('Failed to import saved views:', error)
  }
}

const loadEvaluationComparison = async () => {
  try {
    const imported = await importEvaluationComparison()
    evaluationComparison.value = imported
    if (imported) {
      await persistEvaluationComparison(imported)
      evaluationComparisonHistory.value = mergeEvaluationHistory(
        evaluationComparisonHistory.value,
        imported,
      )
      await persistEvaluationComparisonHistory(evaluationComparisonHistory.value)
    }
  } catch (error) {
    console.error('Failed to import evaluation comparison:', error)
  }
}

const clearEvaluationComparison = async () => {
  evaluationComparison.value = null
  try {
    await clearPersistedEvaluationComparison()
  } catch (error) {
    console.error('Failed to clear persisted evaluation comparison:', error)
  }
}

const clearEvaluationComparisonHistory = async () => {
  evaluationComparisonHistory.value = evaluationComparison.value ? [evaluationComparison.value] : []
  try {
    if (evaluationComparison.value) {
      await persistEvaluationComparisonHistory(evaluationComparisonHistory.value)
    } else {
      await clearPersistedEvaluationComparisonHistory()
    }
  } catch (error) {
    console.error('Failed to clear persisted evaluation comparison history:', error)
  }
}

const selectEvaluationHistoryEntry = (comparedAt: string) => {
  const target = findEvaluationHistoryEntry(evaluationComparisonHistory.value, comparedAt)
  if (target) {
    evaluationComparison.value = target
  }
}

const exportCurrentSnapshot = async () => {
  exportingSnapshot.value = true
  try {
    const { statusFilter, statusFilters, analysisStageFilters } = resolveStatusFilters()
    await exportFindingSnapshot({
      severityFilter: filters.value.severity || null,
      statusFilter,
      statusFilters,
      analysisStageFilters,
      lifecycleView: filters.value.lifecycleView,
      search: filters.value.search,
      semanticSourceFilter: filters.value.semanticSource || null,
      hypothesisRiskTypeFilter: filters.value.hypothesisRiskType || null,
      hypothesisRiskTypeFilters: filters.value.hypothesisRiskTypes.length
        ? filters.value.hypothesisRiskTypes
        : null,
    })
  } catch (error) {
    console.error('Failed to export finding snapshot:', error)
  } finally {
    exportingSnapshot.value = false
  }
}

const setCurrentPage = (page: number) => {
  const nextPage = Math.min(Math.max(1, page), totalPages.value)
  if (currentPage.value === nextPage) {
    refreshFindings()
    return
  }
  currentPage.value = nextPage
}

const goToFirstPage = () => {
  if (isLoading.value || currentPage.value <= 1) return
  setCurrentPage(1)
}

const goToPreviousPage = () => {
  if (isLoading.value || currentPage.value <= 1) return
  setCurrentPage(currentPage.value - 1)
}

const goToNextPage = () => {
  if (isLoading.value || currentPage.value >= totalPages.value) return
  setCurrentPage(currentPage.value + 1)
}

const goToLastPage = () => {
  if (isLoading.value || currentPage.value >= totalPages.value) return
  setCurrentPage(totalPages.value)
}

const applyPageJump = () => {
  if (isLoading.value) return
  const nextPage = Number.parseInt(pageInput.value, 10)
  if (Number.isNaN(nextPage)) {
    pageInput.value = String(currentPage.value)
    return
  }
  setCurrentPage(nextPage)
}

const openDetails = (finding: Finding) => {
  selectedFinding.value = finding
  detailTab.value = 'overview'
  showDetailsModal.value = true
}

const closeDetails = () => {
  showDetailsModal.value = false
  selectedFinding.value = null
  detailTab.value = 'overview'
}

const toggleSelect = (id: string) => {
  if (selectedIds.value.has(id)) {
    selectedIds.value.delete(id)
  } else {
    selectedIds.value.add(id)
  }
}

const toggleSelectAll = () => {
  if (isAllSelected.value) {
    // 取消全选当前页
    findings.value.forEach(f => selectedIds.value.delete(f.id))
  } else {
    // 全选当前页
    findings.value.forEach(f => selectedIds.value.add(f.id))
  }
}

const deleteSingle = async (id: string) => {
  try {
    const response = await invoke<any>('delete_traffic_vulnerability', { vulnId: id })
    if (response.success) {
      console.log('Vulnerability deleted:', id)
      await refreshFindings()
      selectedIds.value.delete(id)
    } else {
      alert('删除失败: ' + (response.error || '未知错误'))
    }
  } catch (error) {
    console.error('Failed to delete vulnerability:', error)
    alert('删除失败: ' + error)
  }
}

const verifyWithSystemAgent = async (findingId: string) => {
  if (verifyingId.value) return
  verifyingId.value = findingId
  try {
    const response = await invoke<any>('verify_finding_with_system_agent', {
      request: { findingId },
    })
    if (!response.success) {
      throw new Error(response.error || '系统 Agent 验证失败')
    }
    await refreshFindings()
  } catch (error) {
    console.error('Failed to verify finding with system agent:', error)
    alert('验证失败: ' + error)
  } finally {
    verifyingId.value = null
  }
}

const syncSelectedFinding = () => {
  if (!selectedFinding.value) return
  const nextFinding = findings.value.find(item => item.id === selectedFinding.value?.id)
  if (nextFinding) {
    selectedFinding.value = nextFinding
  }
}

const submitSystemAgentFeedback = async (
  findingId: string,
  feedbackType: 'confirm' | 'false_positive'
) => {
  if (feedbackingId.value) return
  feedbackingId.value = findingId
  try {
    const response = await invoke<any>('submit_system_agent_finding_feedback', {
      request: {
        findingId,
        feedbackType,
      },
    })
    if (!response.success) {
      throw new Error(response.error || '反馈提交失败')
    }
    await refreshFindings()
    syncSelectedFinding()
  } catch (error) {
    console.error('Failed to submit system agent finding feedback:', error)
    alert('反馈提交失败: ' + error)
  } finally {
    feedbackingId.value = null
  }
}

const deleteSelected = async () => {
  if (selectedIds.value.size === 0) return

  try {
    const ids = Array.from(selectedIds.value)
    const response = await invoke<any>('delete_traffic_vulnerabilities_batch', { vulnIds: ids })
    if (response.success) {
      console.log(`Deleted ${ids.length} vulnerabilities`)
      await refreshFindings()
      selectedIds.value.clear()
    } else {
      alert('批量删除失败: ' + (response.error || '未知错误'))
    }
  } catch (error) {
    console.error('Failed to delete vulnerabilities:', error)
    alert('批量删除失败: ' + error)
  }
}

const deleteAll = () => {
  if (totalCount.value === 0) return
  showDeleteAllModal.value = true
}

const confirmDeleteAll = async () => {
  showDeleteAllModal.value = false
  isLoading.value = true

  try {
    const response = await invoke<any>('delete_all_traffic_vulnerabilities')
    if (response.success) {
      console.log('All vulnerabilities deleted')
      await refreshFindings()
      selectedIds.value.clear()
    } else {
      alert('清空失败: ' + (response.error || '未知错误'))
    }
  } catch (error) {
    console.error('Failed to delete all vulnerabilities:', error)
    alert('清空失败: ' + error)
  } finally {
    isLoading.value = false
  }
}

const formatTime = (timestamp: string) => {
  if (!timestamp) return '-'
  const date = new Date(timestamp)
  return date.toLocaleString('zh-CN')
}

const handleRefresh = () => {
  refreshFindings()
}

let unlistenFinding: UnlistenFn | null = null
let unlistenVerification: UnlistenFn | null = null
let unlistenRunUpdate: UnlistenFn | null = null

// 监听页码变化
watch(currentPage, () => {
  pageInput.value = String(currentPage.value)
  refreshFindings()
})

watch(pageSize, () => {
  pageInput.value = '1'
  if (currentPage.value !== 1) {
    currentPage.value = 1
    return
  }
  refreshFindings()
})

watch(selectedFinding, finding => {
  if (!finding && detailTab.value !== 'overview') {
    detailTab.value = 'overview'
    return
  }
  if (finding && !isSystemAgentFinding(finding) && detailTab.value === 'system_agent') {
    detailTab.value = 'overview'
  }
})

const handleKeyDown = (e: KeyboardEvent) => {
  if (e.key === 'Escape' && showDetailsModal.value) {
    closeDetails()
  }
}

onMounted(async () => {
  pageInput.value = String(currentPage.value)
  try {
    evaluationComparison.value = await loadPersistedEvaluationComparison()
  } catch (error) {
    console.error('Failed to load persisted evaluation comparison:', error)
  }
  try {
    evaluationComparisonHistory.value = await loadPersistedEvaluationComparisonHistory()
  } catch (error) {
    console.error('Failed to load persisted evaluation comparison history:', error)
  }
  if (evaluationComparison.value) {
    evaluationComparisonHistory.value = mergeEvaluationHistory(
      evaluationComparisonHistory.value,
      evaluationComparison.value,
    )
  }
  refreshFindings()
  window.addEventListener('security-center-refresh', handleRefresh)
  window.addEventListener('keydown', handleKeyDown)
  unlistenFinding = await listen('scan:finding', () => {
    refreshFindings()
  })
  unlistenVerification = await listen('system-agent:verification-complete', () => {
    refreshFindings()
  })
  unlistenRunUpdate = await listen<any>('system-agent:run-updated', event => {
    if (event.payload?.profileId === 'traffic_active_verifier') {
      refreshFindings()
    }
  })
})

onUnmounted(() => {
  window.removeEventListener('security-center-refresh', handleRefresh)
  window.removeEventListener('keydown', handleKeyDown)
  unlistenFinding?.()
  unlistenVerification?.()
  unlistenRunUpdate?.()
})
</script>
