<template>
  <div class="space-y-6">
    <VulnerabilitiesStatsOverview :stats="stats" />

    <!-- 筛选器 -->
    <div class="bg-base-100 rounded-lg p-4 shadow-sm border border-base-300">
      <div class="space-y-3">
        <div class="flex flex-wrap gap-3 items-center">
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
              <option value="">全部状态</option>
              <option value="candidate">候选待验证</option>
              <option value="open">开放</option>
              <option value="reviewed">已验证</option>
              <option value="false_positive">误报</option>
              <option value="fixed">已修复</option>
            </select>
          </div>
          <div class="form-control">
            <select
              v-model="filters.readStatus"
              class="select select-bordered select-sm"
              @change="applyFilters"
            >
              <option value="">全部阅读状态</option>
              <option value="unread">仅未读</option>
              <option value="read">仅已读</option>
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
          <div class="ml-auto flex flex-wrap items-center justify-end gap-2">
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
            <span class="text-sm text-base-content/70">已选择 {{ selectedIds.size }} 项</span>
            <button
              @click="markCurrentPageAsRead"
              class="btn btn-success btn-sm btn-outline"
              :disabled="findings.length === 0"
            >
              标记本页已读
            </button>
            <button
              @click="markAllFilteredAsRead"
              class="btn btn-success btn-sm btn-outline"
              :disabled="!canMarkAllFilteredAsRead"
            >
              全部标记已读
            </button>
            <button
              @click="deleteSelected"
              class="btn btn-error btn-sm"
              :disabled="selectedIds.size === 0"
            >
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
          </div>
        </div>
      </div>
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
              :read="isFindingRead(finding.id)"
              :reviewing="reviewingFindingId === finding.id"
              @toggle-select="toggleSelect"
              @open-details="openDetails"
              @open-workbench="openWorkbenchForFinding"
              @mark-read="markSingleAsRead"
              @ai-review="reviewFindingWithAi"
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
    <AppDialog
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
              @feedback="
                ({ findingId, feedbackType }) => submitSystemAgentFeedback(findingId, feedbackType)
              "
            />

            <VulnerabilityTimelinePanel v-else :finding="selectedFinding" />
          </div>
        </div>

        <VulnerabilityDetailFooterActions
          :transferable-evidence="primaryTransferableEvidence"
          :transfer-messages="transferMessages"
          :finding="selectedFinding"
          :open-workbench-label="t('vulnerabilities.openWorkbench')"
          :ai-review-label="t('vulnerabilities.aiReview.action')"
          :ai-review-running-label="t('vulnerabilities.aiReview.running')"
          :reviewing="reviewingFindingId === selectedFinding?.id"
          :close-label="t('common.close')"
          @ai-review="reviewFindingWithAi"
          @open-workbench="openWorkbenchForFinding"
          @close="closeDetails"
        />
      </div>
      <form method="dialog" class="modal-backdrop">
        <button @click="closeDetails">close</button>
      </form>
    </AppDialog>

    <!-- 删除全部确认对话框 -->
    <AppDialog :class="['modal', { 'modal-open': showDeleteAllModal }]">
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
    </AppDialog>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onActivated, onDeactivated, onMounted, onUnmounted, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRoute, useRouter } from 'vue-router'
import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { dialog } from '@/composables/useDialog'
import VulnerabilityDetailFooterActions from './VulnerabilityDetailFooterActions.vue'
import VulnerabilityDetailOverview from './VulnerabilityDetailOverview.vue'
import VulnerabilityEvidenceList from './VulnerabilityEvidenceList.vue'
import VulnerabilityFindingRow from './VulnerabilityFindingRow.vue'
import VulnerabilitySystemAgentPanel from './VulnerabilitySystemAgentPanel.vue'
import VulnerabilityTimelinePanel from './VulnerabilityTimelinePanel.vue'
import VulnerabilitiesStatsOverview from './VulnerabilitiesStatsOverview.vue'
import { getOrCreateWorkbenchCaseForFinding } from './securityWorkbenchCaseSupport'
import {
  openSecurityEvidenceInTrafficWorkbench,
  findFirstTransferableSecurityEvidence,
  type SecurityEvidenceTransferMessages,
} from './securityEvidenceTransferSupport'
import { resolveSecurityEvidenceTransferShortcut } from './securityEvidenceTransferShortcut'
import { useSecurityCenterActivity } from '@/composables/useSecurityCenterActivity'
import type { Evidence, Finding } from './vulnerabilityFindingTypes'
import { isSystemAgentFinding } from './vulnerabilityFindingPresentation'

const { t } = useI18n()
const route = useRoute()
const router = useRouter()
const securityCenterActivity = useSecurityCenterActivity()
const { isFindingRead } = securityCenterActivity
const props = defineProps<{
  immersiveMode?: boolean
  immersiveOpenFindingRequest?: {
    findingId: string
    requestKey: number
  } | null
}>()
const emit = defineEmits<{
  'stats-updated': [stats: { total: number; critical: number }]
  'open-workbench-case': [caseId: string]
}>()

const findings = ref<Finding[]>([])
const isLoading = ref(false)
const showDetailsModal = ref(false)
const showDeleteAllModal = ref(false)
const selectedFinding = ref<Finding | null>(null)
const selectedIds = ref<Set<string>>(new Set())
const feedbackingId = ref<string | null>(null)
const reviewingFindingId = ref<string | null>(null)
type DetailTabId = 'overview' | 'evidence' | 'system_agent' | 'timeline'
const DEFAULT_DETAIL_TAB: DetailTabId = 'evidence'
const detailTab = ref<DetailTabId>(DEFAULT_DETAIL_TAB)
const consumedRouteFindingId = ref<string | null>(null)
const consumedImmersiveFindingRequestKey = ref<number | null>(null)

const stats = ref({ critical: 0, high: 0, medium: 0, low: 0 })

const filters = ref({
  severity: '',
  status: '',
  readStatus: '',
  search: '',
})

const VALID_SEVERITY_FILTERS = new Set(['', 'critical', 'high', 'medium', 'low', 'info'])
const VALID_STATUS_FILTERS = new Set(['', 'candidate', 'open', 'reviewed', 'false_positive', 'fixed'])
const SEVERITY_BREAKDOWN_KEYS = ['critical', 'high', 'medium', 'low'] as const
const currentPage = ref(1)
const pageSize = ref(10)
const pageInput = ref('1')
const pageSizeOptions = [10, 20, 50, 100]
const totalCount = ref(0) // 总条数
const totalPages = computed(() => Math.max(1, Math.ceil(totalCount.value / pageSize.value)))
const isAllSelected = computed(
  () => findings.value.length > 0 && findings.value.every(f => selectedIds.value.has(f.id))
)
const canMarkAllFilteredAsRead = computed(() =>
  totalCount.value > 0 && filters.value.readStatus !== 'read',
)
const primaryTransferableEvidence = computed(() =>
  findFirstTransferableSecurityEvidence(selectedFinding.value?.evidence)
)
const transferMessages = computed<SecurityEvidenceTransferMessages>(() => ({
  triggerLabel: t('vulnerabilities.transfer.triggerLabel'),
  createDraft: t('vulnerabilities.transfer.createDraft'),
  createAttackWorkspace: t('vulnerabilities.transfer.createAttackWorkspace'),
  noTransferableRequest: t('vulnerabilities.transfer.noFindingTransferableRequest'),
  draftCreated: t('vulnerabilities.transfer.draftCreated'),
  attackWorkspaceCreated: t('vulnerabilities.transfer.attackWorkspaceCreated'),
  transferFailed: t('vulnerabilities.transfer.transferFailed', { error: '{error}' }),
}))
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

const formatTransferError = (error: unknown) =>
  transferMessages.value.transferFailed.replace('{error}', String(error))

const transferSelectedFindingRequest = async (target: 'draft' | 'attackWorkspace') => {
  const evidence = primaryTransferableEvidence.value
  if (!evidence) {
    dialog.toast.warning(transferMessages.value.noTransferableRequest)
    return
  }

  try {
    const handled = await openSecurityEvidenceInTrafficWorkbench(router, evidence, target)
    if (!handled) {
      dialog.toast.warning(transferMessages.value.noTransferableRequest)
      return
    }

    dialog.toast.success(
      target === 'draft'
        ? transferMessages.value.draftCreated
        : transferMessages.value.attackWorkspaceCreated,
    )
  } catch (error) {
    console.error('Failed to send security evidence request from shortcut:', error)
    dialog.toast.error(formatTransferError(error))
  }
}

const resolveStatusFilters = () => {
  if (filters.value.status && VALID_STATUS_FILTERS.has(filters.value.status)) {
    return {
      statusFilter: filters.value.status,
      statusFilters: null as string[] | null,
      analysisStageFilters: null as string[] | null,
    }
  }

  return {
    statusFilter: null,
    statusFilters: null,
    analysisStageFilters: null,
  }
}

const countFindings = async (
  severityFilter: string | null,
  statusFilter: string | null = null,
  statusFilters: string[] | null = null,
  analysisStageFilters: string[] | null = null,
  search: string | null = null
) => {
  const response = await invoke<any>('count_findings', {
    severityFilter,
    statusFilter,
    statusFilters,
    analysisStageFilters,
    search,
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

const normalizeFindingsResponse = (response: any): Finding[] => {
  if (!response?.success || !response.data) {
    return []
  }

  return response.data
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
    .filter((f: Finding | null): f is Finding => f !== null)
}

const selectPrimaryEvidence = (evidence: Evidence[] = []) =>
  evidence.find(item => !item.location.startsWith('system_agent_'))
  || evidence.find(
    item => !['system_agent_verification', 'system_agent_feedback'].includes(item.location),
  )
  || evidence[0]

const buildDetailedFinding = (detail: any, fallbackFinding: Finding | null = null): Finding | null => {
  const vulnerability = detail?.vulnerability
  if (!vulnerability) {
    return fallbackFinding
  }

  const evidence = Array.isArray(detail?.evidence) ? detail.evidence : (fallbackFinding?.evidence || [])
  const primaryEvidence = selectPrimaryEvidence(evidence)

  return {
    ...(fallbackFinding || {}),
    ...vulnerability,
    url: primaryEvidence?.url || fallbackFinding?.url || '',
    method: primaryEvidence?.method || fallbackFinding?.method,
    evidence,
  } as Finding
}

const fetchFindingDetail = async (findingId: string, fallbackFinding: Finding | null = null) => {
  const response = await invoke<any>('get_finding', { findingId })
  if (!response?.success || !response?.data) {
    return fallbackFinding
  }
  return buildDetailedFinding(response.data, fallbackFinding)
}

const summarizeFindingsBySeverity = (items: Finding[]) => {
  const summary = {
    total: items.length,
    critical: 0,
    high: 0,
    medium: 0,
    low: 0,
  }

  for (const item of items) {
    const severity = item.severity?.toLowerCase()
    if (severity === 'critical') summary.critical += 1
    if (severity === 'high') summary.high += 1
    if (severity === 'medium') summary.medium += 1
    if (severity === 'low') summary.low += 1
  }

  return summary
}

const countVisibleStats = async (
  filteredTotal: number,
  severityFilter: string | null,
  statusFilter: string | null,
  statusFilters: string[] | null,
  analysisStageFilters: string[] | null,
  search: string | null,
) => {
  if (severityFilter) {
    return {
      total: filteredTotal,
      critical: severityFilter === 'critical' ? filteredTotal : 0,
      high: severityFilter === 'high' ? filteredTotal : 0,
      medium: severityFilter === 'medium' ? filteredTotal : 0,
      low: severityFilter === 'low' ? filteredTotal : 0,
    }
  }

  const [critical, high, medium, low] = await Promise.all(
    SEVERITY_BREAKDOWN_KEYS.map(level =>
      countFindings(level, statusFilter, statusFilters, analysisStageFilters, search),
    ),
  )

  return {
    total: filteredTotal,
    critical,
    high,
    medium,
    low,
  }
}

const filterFindingsByReadStatus = (items: Finding[]) => {
  if (filters.value.readStatus === 'unread') {
    return items.filter(item => !isFindingRead(item.id))
  }

  if (filters.value.readStatus === 'read') {
    return items.filter(item => isFindingRead(item.id))
  }

  return items
}

const loadAllFilteredFindings = async () => {
  const severityFilter = filters.value.severity || null
  const { statusFilter, statusFilters, analysisStageFilters } = resolveStatusFilters()
  const search = filters.value.search.trim() || null
  const filteredTotal = await countFindings(
    severityFilter,
    statusFilter,
    statusFilters,
    analysisStageFilters,
    search,
  )

  if (filteredTotal <= 0) {
    return [] as Finding[]
  }

  const response = await invoke<any>('list_findings', {
    limit: Math.max(filteredTotal, 1),
    offset: 0,
    severityFilter,
    statusFilter,
    statusFilters,
    analysisStageFilters,
    search,
  })

  return filterFindingsByReadStatus(normalizeFindingsResponse(response))
}

const refreshFindings = async () => {
  isLoading.value = true
  try {
    const severityFilter = filters.value.severity || null
    const { statusFilter, statusFilters, analysisStageFilters } = resolveStatusFilters()
    const search = filters.value.search.trim() || null
    const filteredTotal = await countFindings(
      severityFilter,
      statusFilter,
      statusFilters,
      analysisStageFilters,
      search,
    )

    if (filters.value.readStatus) {
      const response = await invoke<any>('list_findings', {
        limit: Math.max(filteredTotal, 1),
        offset: 0,
        severityFilter,
        statusFilter,
        statusFilters,
        analysisStageFilters,
        search,
      })

      const filteredFindings = filterFindingsByReadStatus(normalizeFindingsResponse(response))
      totalCount.value = filteredFindings.length
      updateStats(summarizeFindingsBySeverity(filteredFindings))

      const nextTotalPages = Math.max(1, Math.ceil(totalCount.value / pageSize.value))
      if (currentPage.value > nextTotalPages) {
        currentPage.value = nextTotalPages
        return
      }

      const offset = (currentPage.value - 1) * pageSize.value
      findings.value = filteredFindings.slice(offset, offset + pageSize.value)
      syncSelectedFinding()
      await openFindingFromRoute()
      return
    }

    const offset = (currentPage.value - 1) * pageSize.value
    const response = await invoke<any>('list_findings', {
      limit: pageSize.value,
      offset,
      severityFilter,
      statusFilter,
      statusFilters,
      analysisStageFilters,
      search,
    })

    totalCount.value = filteredTotal
    updateStats(
      await countVisibleStats(
        filteredTotal,
        severityFilter,
        statusFilter,
        statusFilters,
        analysisStageFilters,
        search,
      ),
    )

    const nextTotalPages = Math.max(1, Math.ceil(filteredTotal / pageSize.value))
    if (currentPage.value > nextTotalPages) {
      currentPage.value = nextTotalPages
      return
    }

    findings.value = normalizeFindingsResponse(response)
    syncSelectedFinding()
    await openFindingFromRoute()
    return
  } catch (error) {
    console.error('Failed to refresh findings:', error)
    findings.value = []
    totalCount.value = 0
    updateStats({ total: 0, critical: 0, high: 0, medium: 0, low: 0 })
  } finally {
    isLoading.value = false
  }
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
  applyFilters()
}

const syncFiltersFromRouteQuery = () => {
  const nextSeverity =
    typeof route.query.severity === 'string' ? route.query.severity.trim().toLowerCase() : ''
  const normalizedSeverity = VALID_SEVERITY_FILTERS.has(nextSeverity) ? nextSeverity : ''

  if (filters.value.severity === normalizedSeverity) {
    return false
  }

  filters.value.severity = normalizedSeverity
  return true
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

const openDetails = async (finding: Finding) => {
  securityCenterActivity.markFindingAsRead(finding.id)
  selectedFinding.value = finding
  detailTab.value = DEFAULT_DETAIL_TAB
  showDetailsModal.value = true

  try {
    const detailedFinding = await fetchFindingDetail(finding.id, finding)
    if (selectedFinding.value?.id === finding.id && detailedFinding) {
      selectedFinding.value = detailedFinding
    }
  } catch (error) {
    console.error('Failed to load finding detail:', error)
  }

  if (filters.value.readStatus === 'unread') {
    void refreshFindings()
  }
}

const closeDetails = () => {
  showDetailsModal.value = false
  selectedFinding.value = null
  detailTab.value = DEFAULT_DETAIL_TAB

  if (props.immersiveMode) {
    return
  }

  if (typeof route.query.findingId === 'string' && route.query.findingId.trim()) {
    const nextQuery = { ...route.query }
    delete nextQuery.findingId
    router.replace({ query: nextQuery })
  }
}

const openWorkbenchForFinding = async (finding: Finding) => {
  try {
    securityCenterActivity.markFindingAsRead(finding.id)
    const caseItem = await getOrCreateWorkbenchCaseForFinding(finding.id)
    showDetailsModal.value = false
    selectedFinding.value = null
    detailTab.value = DEFAULT_DETAIL_TAB
    if (props.immersiveMode) {
      emit('open-workbench-case', caseItem.id)
      return
    }
    await router.replace({ path: `/security-center/workbench/${caseItem.id}` })
  } catch (error) {
    console.error('Failed to open workbench for finding', error)
    dialog.toast.error('打开安全工作台失败')
  }
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

const markSingleAsRead = (id: string) => {
  securityCenterActivity.markFindingAsRead(id)
  selectedIds.value.delete(id)
  if (filters.value.readStatus) {
    void refreshFindings()
  }
}

const markCurrentPageAsRead = () => {
  if (findings.value.length === 0) return
  const targetIds = findings.value.map(item => item.id)
  securityCenterActivity.markFindingsAsRead(targetIds)
  targetIds.forEach(id => selectedIds.value.delete(id))
  if (filters.value.readStatus) {
    void refreshFindings()
  }
}

const markAllFilteredAsRead = async () => {
  if (!canMarkAllFilteredAsRead.value) return

  const confirmed = await dialog.confirm({
    title: '全部标记已读',
    message: `确认将当前筛选结果中的 ${totalCount.value} 条漏洞记录全部标记为已读吗？`,
    confirmText: '全部标记已读',
    cancelText: '取消',
    variant: 'warning',
  })
  if (!confirmed) return

  try {
    const targetIds = (await loadAllFilteredFindings()).map(item => item.id)
    if (!targetIds.length) return

    securityCenterActivity.markFindingsAsRead(targetIds)
    targetIds.forEach(id => selectedIds.value.delete(id))
    if (filters.value.readStatus) {
      await refreshFindings()
    }
  } catch (error) {
    console.error('Failed to mark all filtered findings as read:', error)
    dialog.toast.error('全部标记已读失败')
  }
}

const deleteSingle = async (id: string) => {
  try {
    const response = await invoke<any>('delete_traffic_vulnerability', { vulnId: id })
    if (response.success) {
      console.log('Vulnerability deleted:', id)
      await refreshFindings()
      await securityCenterActivity.refreshSecurityCenterActivity()
      selectedIds.value.delete(id)
    } else {
      alert('删除失败: ' + (response.error || '未知错误'))
    }
  } catch (error) {
    console.error('Failed to delete vulnerability:', error)
    alert('删除失败: ' + error)
  }
}

const syncSelectedFinding = () => {
  if (!selectedFinding.value) return
  const nextFinding = findings.value.find(item => item.id === selectedFinding.value?.id)
  if (nextFinding) {
    selectedFinding.value = {
      ...selectedFinding.value,
      ...nextFinding,
      url: selectedFinding.value.url || nextFinding.url || '',
      method: selectedFinding.value.method || nextFinding.method,
      evidence: selectedFinding.value.evidence?.length ? selectedFinding.value.evidence : (nextFinding.evidence || []),
    }
  }
}

const refreshSelectedFindingDetail = async (findingId: string) => {
  const fallbackFinding =
    findings.value.find(item => item.id === findingId)
    || (selectedFinding.value?.id === findingId ? selectedFinding.value : null)

  const detailedFinding = await fetchFindingDetail(findingId, fallbackFinding)
  if (selectedFinding.value?.id === findingId && detailedFinding) {
    selectedFinding.value = detailedFinding
  }
}

const openFindingFromRoute = async () => {
  const findingId = props.immersiveMode
    ? props.immersiveOpenFindingRequest?.findingId?.trim() || ''
    : typeof route.query.findingId === 'string'
      ? route.query.findingId.trim()
      : ''
  const immersiveRequestKey = props.immersiveMode
    ? props.immersiveOpenFindingRequest?.requestKey ?? null
    : null
  if (!findingId) {
    consumedRouteFindingId.value = null
    consumedImmersiveFindingRequestKey.value = null
    return
  }

  if (props.immersiveMode) {
    if (immersiveRequestKey !== null && consumedImmersiveFindingRequestKey.value === immersiveRequestKey) {
      return
    }
  } else if (consumedRouteFindingId.value === findingId) {
    return
  }

  const existingFinding = findings.value.find(item => item.id === findingId)
  if (existingFinding) {
    consumedRouteFindingId.value = findingId
    consumedImmersiveFindingRequestKey.value = immersiveRequestKey
    await openDetails(existingFinding)
    return
  }

  try {
    const detailedFinding = await fetchFindingDetail(findingId)
    if (!detailedFinding) {
      return
    }

    consumedRouteFindingId.value = findingId
    consumedImmersiveFindingRequestKey.value = immersiveRequestKey
    securityCenterActivity.markFindingAsRead(findingId)
    selectedFinding.value = detailedFinding
    detailTab.value = DEFAULT_DETAIL_TAB
    showDetailsModal.value = true
  } catch (error) {
    console.error('Failed to open finding from route:', error)
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

const reviewFindingWithAi = async (finding: Finding) => {
  if (reviewingFindingId.value) return

  reviewingFindingId.value = finding.id
  try {
    const response = await invoke<any>('review_finding_with_ai', {
      findingId: finding.id,
    })
    if (!response?.success || !response?.data) {
      throw new Error(response?.error || 'AI复核失败')
    }

    await refreshFindings()
    await refreshSelectedFindingDetail(finding.id)

    dialog.toast.success(
      response.data.appliedStatus === 'false_positive'
        ? t('vulnerabilities.aiReview.successFalsePositive')
        : t('vulnerabilities.aiReview.successReal'),
    )
  } catch (error) {
    console.error('Failed to review finding with AI:', error)
    dialog.toast.error(
      t('vulnerabilities.aiReview.failed', {
        error: error instanceof Error ? error.message : String(error),
      }),
    )
  } finally {
    reviewingFindingId.value = null
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
      await securityCenterActivity.refreshSecurityCenterActivity()
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
      await securityCenterActivity.refreshSecurityCenterActivity()
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
let unlistenRunUpdate: UnlistenFn | null = null
let unlistenFindingUpdated: UnlistenFn | null = null

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
  if (!finding && detailTab.value !== DEFAULT_DETAIL_TAB) {
    detailTab.value = DEFAULT_DETAIL_TAB
    return
  }
  if (finding && !isSystemAgentFinding(finding) && detailTab.value === 'system_agent') {
    detailTab.value = DEFAULT_DETAIL_TAB
  }
})

watch(
  () => props.immersiveMode ? props.immersiveOpenFindingRequest?.requestKey : route.query.findingId,
  () => {
    void openFindingFromRoute()
  }
)

watch(
  () => route.query.severity,
  () => {
    if (props.immersiveMode) {
      return
    }

    if (!syncFiltersFromRouteQuery()) {
      return
    }

    pageInput.value = '1'
    selectedIds.value.clear()
    if (currentPage.value !== 1) {
      currentPage.value = 1
      return
    }

    refreshFindings()
  }
)

const handleKeyDown = (e: KeyboardEvent) => {
  if (e.key === 'Escape' && showDetailsModal.value) {
    closeDetails()
    return
  }

  if (!selectedFinding.value || !showDetailsModal.value) {
    return
  }

  const target = resolveSecurityEvidenceTransferShortcut(e)
  if (!target) {
    return
  }

  e.preventDefault()
  e.stopPropagation()
  void transferSelectedFindingRequest(target)
}

onMounted(async () => {
  pageInput.value = String(currentPage.value)
  if (!props.immersiveMode) {
    syncFiltersFromRouteQuery()
  }
  await securityCenterActivity.initializeSecurityCenterActivity()
  refreshFindings()
  window.addEventListener('security-center-refresh', handleRefresh)
  unlistenFinding = await listen('scan:finding', () => {
    refreshFindings()
  })
  unlistenFindingUpdated = await listen('scan:finding-updated', () => {
    refreshFindings()
  })
  unlistenRunUpdate = await listen<any>('system-agent:run-updated', event => {
    if (event.payload?.profileId === 'traffic_active_verifier') {
      refreshFindings()
    }
  })
})

onActivated(() => {
  window.addEventListener('keydown', handleKeyDown)
})

onDeactivated(() => {
  window.removeEventListener('keydown', handleKeyDown)
})

onUnmounted(() => {
  window.removeEventListener('security-center-refresh', handleRefresh)
  window.removeEventListener('keydown', handleKeyDown)
  unlistenFinding?.()
  unlistenFindingUpdated?.()
  unlistenRunUpdate?.()
})
</script>
