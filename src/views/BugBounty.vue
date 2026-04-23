<template>
  <div class="p-4 space-y-4 h-full flex flex-col">
    <!-- Header -->
    <div class="flex items-center justify-between flex-shrink-0">
      <div class="flex items-center gap-3">
        <h1 class="text-2xl font-bold">{{ t('bugBounty.title') }}</h1>
      </div>
    </div>

    <!-- Statistics Cards -->
    <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-5 gap-4 flex-shrink-0">
      <div class="stat bg-base-200 rounded-lg">
        <div class="stat-figure text-primary">
          <i class="fas fa-trophy text-2xl"></i>
        </div>
        <div class="stat-title">{{ t('bugBounty.stats.totalPrograms') }}</div>
        <div class="stat-value text-primary text-2xl">{{ stats.total_programs }}</div>
        <div class="stat-desc">{{ stats.active_programs }} {{ t('bugBounty.stats.active') }}</div>
      </div>
      
      <div class="stat bg-base-200 rounded-lg">
        <div class="stat-figure text-info">
          <i class="fas fa-server text-2xl"></i>
        </div>
        <div class="stat-title">{{ t('bugBounty.stats.totalAssets') }}</div>
        <div class="stat-value text-info text-2xl">{{ assetStats.total }}</div>
        <div class="stat-desc">{{ assetStats.active }} {{ t('bugBounty.stats.active') }}</div>
      </div>
      
      <div class="stat bg-base-200 rounded-lg">
        <div class="stat-figure text-error">
          <i class="fas fa-bug text-2xl"></i>
        </div>
        <div class="stat-title">{{ t('bugBounty.stats.totalFindings') }}</div>
        <div class="stat-value text-error text-2xl">{{ findingStats.total_findings }}</div>
        <div class="stat-desc">{{ findingStats.by_severity?.critical || 0 }} {{ t('bugBounty.severity.critical') }}</div>
      </div>
      
      <div class="stat bg-base-200 rounded-lg">
        <div class="stat-figure text-success">
          <i class="fas fa-paper-plane text-2xl"></i>
        </div>
        <div class="stat-title">{{ t('bugBounty.stats.totalSubmissions') }}</div>
        <div class="stat-value text-success text-2xl">{{ submissionStats.total_submissions }}</div>
        <div class="stat-desc">{{ submissionStats.accepted_submissions }} {{ t('bugBounty.stats.accepted') }}</div>
      </div>
      
      <div class="stat bg-base-200 rounded-lg">
        <div class="stat-figure text-warning">
          <i class="fas fa-dollar-sign text-2xl"></i>
        </div>
        <div class="stat-title">{{ t('bugBounty.stats.totalEarnings') }}</div>
        <div class="stat-value text-warning text-2xl">${{ totalEarnings.toFixed(0) }}</div>
        <div class="stat-desc">{{ t('bugBounty.stats.lifetime') }}</div>
      </div>
    </div>

    <!-- Tabs -->
    <div class="tabs tabs-boxed flex-shrink-0">
      <a class="tab" :class="{ 'tab-active': activeTab === 'programs' }" @click="switchTab('programs')">
        <i class="fas fa-trophy mr-2"></i>
        {{ t('bugBounty.tabs.programs') }}
      </a>
      <a class="tab" :class="{ 'tab-active': activeTab === 'assets' }" @click="switchTab('assets')">
        <i class="fas fa-server mr-2"></i>
        {{ t('bugBounty.tabs.assets') }}
        <span v-if="assetStats.total > 0" class="badge badge-sm ml-2">{{ assetStats.total }}</span>
      </a>
      <a class="tab" :class="{ 'tab-active': activeTab === 'api-inventory' }" @click="switchTab('api-inventory')">
        <i class="fas fa-plug mr-2"></i>
        {{ t('bugBounty.tabs.apiInventory') }}
      </a>
      <a class="tab" :class="{ 'tab-active': activeTab === 'findings' }" @click="switchTab('findings')">
        <i class="fas fa-bug mr-2"></i>
        {{ t('bugBounty.tabs.findings') }}
        <span v-if="findingStats.total_findings > 0" class="badge badge-sm ml-2">{{ findingStats.total_findings }}</span>
      </a>
      <a class="tab" :class="{ 'tab-active': activeTab === 'knowledge' }" @click="switchTab('knowledge')">
        <i class="fas fa-note-sticky mr-2"></i>
        {{ t('bugBounty.tabs.knowledge') }}
      </a>
      <a class="tab" :class="{ 'tab-active': activeTab === 'submissions' }" @click="switchTab('submissions')">
        <i class="fas fa-paper-plane mr-2"></i>
        {{ t('bugBounty.tabs.submissions') }}
        <span v-if="submissionStats.total_submissions > 0" class="badge badge-sm ml-2">{{ submissionStats.total_submissions }}</span>
      </a>
      <a class="tab" :class="{ 'tab-active': activeTab === 'statistics' }" @click="switchTab('statistics')">
        <i class="fas fa-chart-line mr-2"></i>
        {{ t('bugBounty.tabs.statistics') }}
      </a>
      <a class="tab" :class="{ 'tab-active': activeTab === 'import-export' }" @click="switchTab('import-export')">
        <i class="fas fa-exchange-alt mr-2"></i>
        {{ t('bugBounty.tabs.importExport') }}
      </a>
      <a class="tab" :class="{ 'tab-active': activeTab === 'templates' }" @click="switchTab('templates')">
        <i class="fas fa-file-alt mr-2"></i>
        {{ t('bugBounty.tabs.templates') }}
      </a>
      <a class="tab" :class="{ 'tab-active': activeTab === 'changes' }" @click="switchTab('changes')">
        <i class="fas fa-bolt mr-2"></i>
        {{ t('bugBounty.tabs.changes') }}
        <span v-if="changeEventStats.pending_review > 0" class="badge badge-warning badge-sm ml-2">{{ changeEventStats.pending_review }}</span>
      </a>
      <a class="tab" :class="{ 'tab-active': activeTab === 'workflows' }" @click="switchTab('workflows')">
        <i class="fas fa-project-diagram mr-2"></i>
        {{ t('bugBounty.tabs.workflows') }}
      </a>
      <a class="tab" :class="{ 'tab-active': activeTab === 'monitor' }" @click="switchTab('monitor')">
        <i class="fas fa-radar mr-2"></i>
        {{ t('bugBounty.tabs.monitor') }}
      </a>
    </div>

    <!-- Tab Content -->
    <div class="flex-1 min-h-0">
      <div v-if="mountedTabs.programs" v-show="activeTab === 'programs'" class="h-full overflow-auto">
        <ProgramsPanel
          :programs="programs"
          :loading="loading"
          @create="showCreateProgramModal = true"
          @select="selectProgram"
          @export-assets="exportProgramAssets"
          @edit="editProgram"
          @delete="deleteProgram"
        />
      </div>

      <div v-if="mountedTabs.assets" v-show="activeTab === 'assets'" class="h-full overflow-auto">
        <AssetsPanelV2
          ref="assetsPanelRef"
          :program-id="selectedProgram?.id"
          :programs="programs"
          @refresh="onAssetsRefreshNeeded"
        />
      </div>

      <div v-if="mountedTabs['api-inventory']" v-show="activeTab === 'api-inventory'" class="h-full overflow-auto">
        <ApiInventoryPanel
          :selected-program="selectedProgram"
          :programs="programs"
        />
      </div>

      <div v-if="mountedTabs.findings" v-show="activeTab === 'findings'" class="h-full overflow-auto">
        <FindingsPanel
          :findings="findings"
          :programs="programs"
          :loading="loadingFindings"
          :batch-action-loading="findingBatchActionLoading"
          :batch-action-version="findingBatchActionVersion"
          :page="findingPage"
          :page-size="findingPageSize"
          :page-count="findingPageCount"
          :total="findingTotal"
          :global-total="findingStats.total_findings"
          :has-next="findingHasNext"
          @create="showCreateFindingModal = true"
          @refresh="refreshFindingsData"
          @view="viewFinding"
          @delete="deleteFinding"
          @delete-all="deleteAllFindings"
          @create-submission="createSubmissionFromFinding"
          @filter-change="onFindingFilterChange"
          @batch-update-status="batchUpdateFindingStatus"
          @batch-delete="batchDeleteFindings"
          @page-change="onFindingPageChange"
          @page-size-change="onFindingPageSizeChange"
        />
      </div>

      <div v-if="mountedTabs.knowledge" v-show="activeTab === 'knowledge'" class="h-full overflow-auto">
        <KnowledgeBasePanel
          :programs="programs"
          :selected-program="selectedProgram"
        />
      </div>

      <div v-if="mountedTabs.submissions" v-show="activeTab === 'submissions'" class="h-full overflow-auto">
        <SubmissionsPanel
          :submissions="submissions"
          :loading="loadingSubmissions"
          :batch-action-loading="submissionBatchActionLoading"
          :batch-action-version="submissionBatchActionVersion"
          :page="submissionPage"
          :page-size="pageSize"
          :total="submissionTotal"
          :has-next="submissionHasNext"
          @create="showCreateSubmissionModal = true"
          @view="viewSubmission"
          @edit="editSubmission"
          @delete="deleteSubmission"
          @filter-change="onSubmissionFilterChange"
          @batch-update-status="batchUpdateSubmissionStatus"
          @batch-delete="batchDeleteSubmissions"
          @page-change="onSubmissionPageChange"
        />
      </div>

      <div v-if="mountedTabs.statistics" v-show="activeTab === 'statistics'" class="h-full overflow-auto">
        <StatisticsPanel
          :finding-stats="findingStats"
          :submission-stats="submissionStats"
          :programs="programs"
          :findings="findings"
          :submissions="submissions"
        />
      </div>

      <div v-if="mountedTabs['import-export']" v-show="activeTab === 'import-export'" class="h-full overflow-auto">
        <ImportExportPanel
          :programs="programs"
          :findings="findings"
          :submissions="submissions"
          @imported="onDataImported"
        />
      </div>

      <div v-if="mountedTabs.templates" v-show="activeTab === 'templates'" class="h-full overflow-auto">
        <ReportTemplatesPanel
          @use-template="onUseTemplate"
        />
      </div>

      <div v-if="mountedTabs.changes" v-show="activeTab === 'changes'" class="h-full overflow-auto">
        <ChangeEventsPanel
          @view="viewChangeEvent"
          @trigger-workflow="triggerWorkflowFromEvent"
          @create="showCreateChangeEventModal = true"
        />
      </div>

      <div v-if="mountedTabs.workflows" v-show="activeTab === 'workflows'" class="h-full overflow-auto">
        <WorkflowTemplatesPanel
          :programs="programs"
          :selected-program="selectedProgram"
          @view="viewWorkflowTemplate"
        />
      </div>

      <div v-if="mountedTabs.monitor" v-show="activeTab === 'monitor'" class="h-full overflow-auto">
        <MonitorPanel
          :selected-program="selectedProgram"
          :programs="programs"
        />
      </div>
    </div>

    <!-- Modals -->
    <CreateProgramModal
      :visible="showCreateProgramModal"
      :submitting="creating"
      :program="editingProgram"
      @close="closeProgramModal"
      @submit="saveProgram"
    />

    <CreateFindingModal
      :visible="showCreateFindingModal"
      :submitting="creating"
      :programs="programs"
      @close="showCreateFindingModal = false"
      @submit="createFinding"
    />

    <CreateSubmissionModal
      :visible="showCreateSubmissionModal"
      :submitting="creating"
      :programs="programs"
      :program-findings="programFindings"
      :initial-data="submissionInitialData"
      @close="closeSubmissionModal"
      @submit="createSubmission"
      @program-change="loadProgramFindings"
    />

    <ProgramDetailModal
      ref="programDetailModalRef"
      :visible="showProgramDetailModal"
      :program="selectedProgram"
      @close="showProgramDetailModal = false"
      @updated="onProgramUpdated"
    />

    <FindingDetailModal
      :visible="showFindingDetailModal"
      :finding="selectedFinding"
      :programs="programs"
      @close="showFindingDetailModal = false"
      @updated="onFindingUpdated"
      @create-submission="createSubmissionFromFinding"
    />

    <SubmissionDetailModal
      :visible="showSubmissionDetailModal"
      :submission="selectedSubmission"
      @close="showSubmissionDetailModal = false"
      @updated="onSubmissionUpdated"
    />

    <ChangeEventDetailModal
      :visible="showChangeEventDetailModal"
      :event="selectedChangeEvent"
      @close="showChangeEventDetailModal = false"
      @updated="onChangeEventUpdated"
      @trigger-workflow="triggerWorkflowFromEvent"
    />

    <CreateChangeEventModal
      :visible="showCreateChangeEventModal"
      :submitting="creating"
      :programs="programs"
      :selected-program="selectedProgram"
      @close="showCreateChangeEventModal = false"
      @submit="createChangeEvent"
    />

    <DiscoverAssetsModal
      :visible="showDiscoverAssetsModal"
      :selected-program="selectedProgram"
      @close="showDiscoverAssetsModal = false"
      @success="onAssetsDiscovered"
    />

    <WorkflowTemplateDetailModal
      v-if="showWorkflowTemplateDetailModal && selectedWorkflowTemplate"
      :template="selectedWorkflowTemplate"
      :programs="programs"
      @close="showWorkflowTemplateDetailModal = false"
      @updated="onWorkflowTemplateUpdated"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, computed, nextTick, onMounted, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { invoke } from '@tauri-apps/api/core'
import { useRoute } from 'vue-router'
import { useToast } from '../composables/useToast'
import { dialog } from '../composables/useDialog'
import { 
  ApiInventoryPanel,
  ProgramsPanel, 
  FindingsPanel, 
  KnowledgeBasePanel,
  SubmissionsPanel, 
  ChangeEventsPanel,
  ChangeEventDetailModal,
  WorkflowTemplatesPanel,
  WorkflowTemplateDetailModal,
  StatisticsPanel,
  ImportExportPanel,
  ReportTemplatesPanel,
  CreateProgramModal,
  CreateFindingModal,
  CreateSubmissionModal,
  ProgramDetailModal,
  FindingDetailModal,
  SubmissionDetailModal,
  DiscoverAssetsModal
} from '../components/BugBounty'
import MonitorPanel from '../components/BugBounty/MonitorPanel.vue'
import CreateChangeEventModal from '../components/BugBounty/CreateChangeEventModal.vue'
import AssetsPanelV2 from '../components/BugBounty/AssetsPanelV2.vue'


defineOptions({
  name: 'BugBountyView',
});

const { t } = useI18n()
const toast = useToast()
const route = useRoute()

// State
const loading = ref(false)
const loadingFindings = ref(false)
const loadingSubmissions = ref(false)
const findingBatchActionLoading = ref(false)
const findingBatchActionVersion = ref(0)
const submissionBatchActionLoading = ref(false)
const submissionBatchActionVersion = ref(0)
const creating = ref(false)
type BugBountyTab =
  | 'programs'
  | 'assets'
  | 'api-inventory'
  | 'findings'
  | 'knowledge'
  | 'submissions'
  | 'statistics'
  | 'import-export'
  | 'templates'
  | 'changes'
  | 'workflows'
  | 'monitor'

const activeTab = ref<BugBountyTab>('programs')
const mountedTabs = ref<Record<BugBountyTab, boolean>>({
  programs: true,
  assets: false,
  'api-inventory': false,
  findings: false,
  knowledge: false,
  submissions: false,
  statistics: false,
  'import-export': false,
  templates: false,
  changes: false,
  workflows: false,
  monitor: false,
})
const loadedTabs = ref({
  findings: false,
  submissions: false,
})

// Modals
const showCreateProgramModal = ref(false)
const showCreateFindingModal = ref(false)
const showCreateSubmissionModal = ref(false)
const showCreateChangeEventModal = ref(false)
const showDiscoverAssetsModal = ref(false)
const showProgramDetailModal = ref(false)
const showFindingDetailModal = ref(false)
const showSubmissionDetailModal = ref(false)
const showChangeEventDetailModal = ref(false)
const showWorkflowTemplateDetailModal = ref(false)
const showSelectWorkflowModal = ref(false)
const submissionInitialData = ref<any>(null)
const selectedProgram = ref<any>(null)
const selectedFinding = ref<any>(null)
const selectedSubmission = ref<any>(null)
const selectedChangeEvent = ref<any>(null)
const selectedWorkflowEvent = ref<any>(null)
const selectedWorkflowTemplate = ref<any>(null)
const selectedWorkflowTemplateId = ref<string>('')
const workflowTemplates = ref<any[]>([])
const editingProgram = ref<any>(null) // Program being edited
const assetsPanelRef = ref<InstanceType<typeof AssetsPanelV2> | null>(null)
const programDetailModalRef = ref<InstanceType<typeof ProgramDetailModal> | null>(null)

// Data
const programs = ref<any[]>([])
const findings = ref<any[]>([])
const submissions = ref<any[]>([])
const programFindings = ref<any[]>([])
const findingPage = ref(1)
const findingPageSize = ref(20)
const findingTotal = ref(0)
const submissionPage = ref(1)
const pageSize = ref(20)
const findingHasNext = ref(false)
const submissionHasNext = ref(false)

const stats = ref({
  total_programs: 0,
  active_programs: 0,
  total_submissions: 0,
  total_accepted: 0,
  total_earnings: 0.0,
})

const assetStats = ref({
  total: 0,
  active: 0,
})

const findingStats = ref({
  total_findings: 0,
  by_severity: {} as Record<string, number>,
  by_status: {} as Record<string, number>,
})

const submissionStats = ref({
  total_submissions: 0,
  accepted_submissions: 0,
  total_rewards: 0.0,
  total_bonuses: 0.0,
})
const submissionTotal = ref(0)

const changeEventStats = ref({
  total_events: 0,
  by_type: {} as Record<string, number>,
  by_severity: {} as Record<string, number>,
  by_status: {} as Record<string, number>,
  pending_review: 0,
  average_risk_score: 0,
})

// Filters
const findingFilter = ref({
  severity: '',
  status: '',
  search: '',
})

const submissionFilter = ref({
  status: '',
  search: '',
})

// Computed
const totalEarnings = computed(() => 
  submissionStats.value.total_rewards + submissionStats.value.total_bonuses
)
const findingPageCount = computed(() => Math.max(1, Math.ceil(findingTotal.value / findingPageSize.value)))
const submissionPageCount = computed(() => Math.max(1, Math.ceil(submissionTotal.value / pageSize.value)))

const refreshProgramsOverview = async () => {
  await Promise.all([loadPrograms(), loadStats()])
}

const refreshFindingsData = async () => {
  await Promise.all([
    loadFindingStats(),
    loadedTabs.value.findings ? loadFindings() : Promise.resolve(),
  ])
}

const refreshSubmissionsData = async () => {
  await Promise.all([
    loadSubmissionStats(),
    loadedTabs.value.submissions ? loadSubmissions() : Promise.resolve(),
  ])
}

// Methods
const switchTab = async (tab: BugBountyTab) => {
  mountedTabs.value[tab] = true
  activeTab.value = tab
  if (tab === 'findings') {
    if (!loadedTabs.value.findings) {
      await loadFindings()
      loadedTabs.value.findings = true
    } else {
      await refreshFindingsData()
    }
  } else if (tab === 'submissions' && !loadedTabs.value.submissions) {
    await loadSubmissions()
    loadedTabs.value.submissions = true
  }
}

const isBugBountyTab = (value: string): value is BugBountyTab => {
  return [
    'programs',
    'assets',
    'api-inventory',
    'findings',
    'knowledge',
    'submissions',
    'statistics',
    'import-export',
    'templates',
    'changes',
    'workflows',
    'monitor',
  ].includes(value)
}

const syncTabFromRoute = async () => {
  const routeTab = typeof route.query.tab === 'string' ? route.query.tab : ''
  if (!routeTab || !isBugBountyTab(routeTab) || routeTab === activeTab.value) return
  await switchTab(routeTab)
}

const loadPrograms = async () => {
  try {
    loading.value = true
    programs.value = await invoke('bounty_list_programs', { filter: null })
  } catch (error) {
    console.error('Failed to load programs:', error)
    toast.error(t('bugBounty.errors.loadFailed'))
  } finally {
    loading.value = false
  }
}

const loadStats = async () => {
  try {
    stats.value = await invoke('bounty_get_program_stats')
  } catch (error) {
    console.error('Failed to load stats:', error)
  }
}

const loadAssetStats = async () => {
  try {
    const overview = await invoke<any>('surface_get_overview', { programId: null })
    assetStats.value = {
      total: overview?.total_assets || 0,
      active: overview?.active_assets || 0,
    }
  } catch (error) {
    console.error('Failed to load asset stats:', error)
    assetStats.value = {
      total: 0,
      active: 0,
    }
  }
}

const loadFindings = async () => {
  try {
    loadingFindings.value = true
    const baseFilter: any = {}
    if (findingFilter.value.severity) {
      baseFilter.severities = [findingFilter.value.severity]
    }
    if (findingFilter.value.status) {
      baseFilter.statuses = [findingFilter.value.status]
    }
    if (findingFilter.value.search) {
      baseFilter.search = findingFilter.value.search
    }
    baseFilter.sort_by = 'created_at'
    baseFilter.sort_dir = 'desc'

    const pagedFilter = {
      ...baseFilter,
      limit: findingPageSize.value,
      offset: (findingPage.value - 1) * findingPageSize.value,
    }

    const [pagedRows, total] = await Promise.all([
      invoke<any[]>('bounty_list_findings', {
        filter: Object.keys(pagedFilter).length > 0 ? pagedFilter : null,
      }),
      invoke<number>('bounty_count_findings', {
        filter: Object.keys(baseFilter).length > 0 ? baseFilter : null,
      }),
    ])

    findings.value = Array.isArray(pagedRows) ? pagedRows : []
    findingTotal.value = Number.isFinite(total) ? Number(total) : 0
    findingHasNext.value = findingPage.value < findingPageCount.value

    if (findingPage.value > findingPageCount.value) {
      findingPage.value = findingPageCount.value
      await loadFindings()
      return
    }
  } catch (error) {
    console.error('Failed to load findings:', error)
    toast.error(t('bugBounty.errors.loadFailed'))
  } finally {
    loadingFindings.value = false
  }
}

const loadFindingStats = async () => {
  try {
    findingStats.value = await invoke('bounty_get_finding_stats', { programId: null })
  } catch (error) {
    console.error('Failed to load finding stats:', error)
  }
}

const loadSubmissions = async () => {
  try {
    loadingSubmissions.value = true
    const baseFilter: any = {}
    if (submissionFilter.value.status) {
      baseFilter.statuses = [submissionFilter.value.status]
    }
    if (submissionFilter.value.search) {
      baseFilter.search = submissionFilter.value.search
    }
    baseFilter.sort_by = 'created_at'
    baseFilter.sort_dir = 'desc'

    const pagedFilter = {
      ...baseFilter,
      limit: pageSize.value,
      offset: (submissionPage.value - 1) * pageSize.value,
    }

    const [pagedRows, total] = await Promise.all([
      invoke<any[]>('bounty_list_submissions', {
        filter: Object.keys(pagedFilter).length > 0 ? pagedFilter : null,
      }),
      invoke<number>('bounty_count_submissions', {
        filter: Object.keys(baseFilter).length > 0 ? baseFilter : null,
      }),
    ])

    submissions.value = Array.isArray(pagedRows) ? pagedRows : []
    submissionTotal.value = Number.isFinite(total) ? Number(total) : 0
    submissionHasNext.value = submissionPage.value < submissionPageCount.value

    if (submissionPage.value > submissionPageCount.value) {
      submissionPage.value = submissionPageCount.value
      await loadSubmissions()
      return
    }
  } catch (error) {
    console.error('Failed to load submissions:', error)
    toast.error(t('bugBounty.errors.loadFailed'))
  } finally {
    loadingSubmissions.value = false
  }
}

const loadSubmissionStats = async () => {
  try {
    submissionStats.value = await invoke('bounty_get_submission_stats', { programId: null })
  } catch (error) {
    console.error('Failed to load submission stats:', error)
  }
}

const loadChangeEventStats = async () => {
  try {
    changeEventStats.value = await invoke('bounty_get_change_event_stats', { programId: null })
  } catch (error) {
    console.error('Failed to load change event stats:', error)
  }
}

const loadProgramFindings = async (programId: string) => {
  if (!programId) {
    programFindings.value = []
    return
  }
  try {
    programFindings.value = await invoke('bounty_list_findings', { 
      filter: { program_id: programId } 
    })
  } catch (error) {
    console.error('Failed to load program findings:', error)
  }
}

const saveProgram = async (data: any) => {
  try {
    creating.value = true
    
    if (data.id) {
      // Update existing program
      const request = {
        name: data.name,
        organization: data.organization,
        platform: data.platform || null,
        url: data.url || null,
        description: data.description || null,
        platform_handle: null,
        program_type: null,
        rewards: null,
        rules: null,
        tags: null,
        status: editingProgram.value?.status || 'active',
      }
      await invoke('bounty_update_program', { id: data.id, request })
      toast.success(t('bugBounty.success.programUpdated'))
    } else {
      // Create new program
      const request = {
        name: data.name,
        organization: data.organization,
        platform: data.platform || null,
        url: data.url || null,
        description: data.description || null,
        platform_handle: null,
        program_type: null,
        rewards: null,
        rules: null,
        tags: null,
      }
      await invoke('bounty_create_program', { request })
      toast.success(t('bugBounty.success.programCreated'))
    }
    
    closeProgramModal()
    await refreshProgramsOverview()
  } catch (error) {
    console.error('Failed to save program:', error)
    toast.error(data.id ? t('bugBounty.errors.updateFailed') : t('bugBounty.errors.createFailed'))
  } finally {
    creating.value = false
  }
}

const createFinding = async (data: any) => {
  try {
    creating.value = true
    const request = {
      program_id: data.program_id,
      title: data.title,
      finding_type: data.finding_type,
      severity: data.severity,
      cvss_score: data.cvss_score,
      affected_url: data.affected_url || null,
      affected_endpoint: data.affected_endpoint || null,
      description: data.description,
      impact: data.impact || null,
      scope_id: null,
      asset_id: null,
      confidence: null,
      cwe_id: data.cwe_id || null,
      affected_parameter: data.affected_parameter || null,
      reproduction_steps: data.reproduction_steps || null,
      remediation: data.remediation || null,
      tags: null,
    }
    await invoke('bounty_create_finding', { request })
    toast.success(t('bugBounty.success.findingCreated'))
    showCreateFindingModal.value = false
    await refreshFindingsData()
  } catch (error: any) {
    console.error('Failed to create finding:', error)
    if (error.toString().includes('Duplicate')) {
      toast.warning(t('bugBounty.errors.duplicateFinding'))
    } else {
      toast.error(t('bugBounty.errors.createFailed'))
    }
  } finally {
    creating.value = false
  }
}

const createSubmission = async (data: any) => {
  try {
    creating.value = true
    const isEdit = !!submissionInitialData.value?.id
    
    if (isEdit) {
      const request = {
        title: data.title,
        vulnerability_type: data.vulnerability_type,
        severity: data.severity,
        cvss_score: data.cvss_score,
        description: data.description,
        impact: data.impact,
        status: data.status || 'draft',
      }
      await invoke('bounty_update_submission', { 
        id: submissionInitialData.value.id, 
        request 
      })
      toast.success(t('bugBounty.success.submissionUpdated'))
    } else {
      const request = {
        program_id: data.program_id,
        finding_id: data.finding_id,
        title: data.title,
        vulnerability_type: data.vulnerability_type,
        severity: data.severity,
        cvss_score: data.cvss_score,
        description: data.description,
        impact: data.impact,
        cwe_id: null,
        reproduction_steps: null,
        remediation: null,
        evidence_ids: null,
        tags: null,
      }
      await invoke('bounty_create_submission', { request })
      toast.success(t('bugBounty.success.submissionCreated'))
    }
    closeSubmissionModal()
    await refreshSubmissionsData()
  } catch (error) {
    console.error('Failed to create submission:', error)
    toast.error(t('bugBounty.errors.createFailed'))
  } finally {
    creating.value = false
  }
}

const deleteFinding = async (finding: any) => {
  if (!(await dialog.confirm(t('bugBounty.confirm.deleteFinding')))) return
  try {
    await invoke('bounty_delete_finding', { id: finding.id })
    toast.success(t('bugBounty.success.findingDeleted'))
    await refreshFindingsData()
  } catch (error) {
    console.error('Failed to delete finding:', error)
    toast.error(t('bugBounty.errors.deleteFailed'))
  }
}

const deleteSubmission = async (submission: any) => {
  if (!(await dialog.confirm(t('bugBounty.confirm.deleteSubmission')))) return
  try {
    await invoke('bounty_delete_submission', { id: submission.id })
    toast.success(t('bugBounty.success.submissionDeleted'))
    await refreshSubmissionsData()
  } catch (error) {
    console.error('Failed to delete submission:', error)
    toast.error(t('bugBounty.errors.deleteFailed'))
  }
}

const deleteProgram = async (program: any) => {
  if (!(await dialog.confirm(t('bugBounty.confirm.deleteProgram')))) return
  try {
    await invoke('bounty_delete_program', { id: program.id })
    toast.success(t('bugBounty.success.programDeleted'))
    await refreshProgramsOverview()
  } catch (error) {
    console.error('Failed to delete program:', error)
    toast.error(t('bugBounty.errors.deleteFailed'))
  }
}

const createSubmissionFromFinding = (finding: any) => {
  // Close finding detail modal if open
  showFindingDetailModal.value = false
  
  submissionInitialData.value = {
    program_id: finding.program_id,
    finding_id: finding.id,
    title: finding.title,
    vulnerability_type: finding.finding_type,
    severity: finding.severity,
    cvss_score: finding.cvss_score,
    description: finding.description,
    impact: finding.impact || '',
  }
  loadProgramFindings(finding.program_id)
  showCreateSubmissionModal.value = true
}

const closeSubmissionModal = () => {
  showCreateSubmissionModal.value = false
  submissionInitialData.value = null
  programFindings.value = []
}

const selectProgram = (program: any) => {
  selectedProgram.value = program
  showProgramDetailModal.value = true
}

const exportProgramAssets = async (program: any) => {
  selectedProgram.value = program
  await switchTab('assets')
  await nextTick()
  assetsPanelRef.value?.openExportModal({
    programId: program?.id || null,
    exportType: 'all',
  })
}

const onProgramUpdated = async () => {
  await refreshProgramsOverview()
}

const editProgram = (program: any) => {
  editingProgram.value = program
  showCreateProgramModal.value = true
}

const closeProgramModal = () => {
  showCreateProgramModal.value = false
  editingProgram.value = null
}

const viewFinding = (finding: any) => {
  selectedFinding.value = finding
  showFindingDetailModal.value = true
}

const onFindingUpdated = async () => {
  await refreshFindingsData()
}

const viewSubmission = (submission: any) => {
  selectedSubmission.value = submission
  showSubmissionDetailModal.value = true
}

const onSubmissionUpdated = async () => {
  await refreshSubmissionsData()
  // Refresh selected submission data
  if (selectedSubmission.value?.id) {
    try {
      const updated = await invoke('bounty_get_submission', { id: selectedSubmission.value.id })
      if (updated) {
        selectedSubmission.value = updated
      }
    } catch (error) {
      console.error('Failed to refresh submission:', error)
    }
  }
}

const editSubmission = (submission: any) => {
  submissionInitialData.value = { ...submission }
  if (submission.program_id) {
    loadProgramFindings(submission.program_id)
  }
  showCreateSubmissionModal.value = true
}

const onFindingFilterChange = (filter: any) => {
  findingFilter.value = filter
  findingPage.value = 1
  loadFindings()
}

const onSubmissionFilterChange = (filter: any) => {
  submissionFilter.value = filter
  submissionPage.value = 1
  loadSubmissions()
}

const onFindingPageChange = (page: number) => {
  findingPage.value = Math.min(Math.max(1, page), findingPageCount.value)
  loadFindings()
}

const onFindingPageSizeChange = (size: number) => {
  if (!Number.isFinite(size) || size <= 0) return
  if (size === findingPageSize.value) return
  findingPageSize.value = size
  findingPage.value = 1
  loadFindings()
}

const onSubmissionPageChange = (page: number) => {
  submissionPage.value = page
  loadSubmissions()
}

const onDataImported = async () => {
  await Promise.all([
    refreshProgramsOverview(),
    loadAssetStats(),
    loadFindingStats(),
    loadSubmissionStats(),
  ])
  if (loadedTabs.value.findings) {
    await loadFindings()
  }
  if (loadedTabs.value.submissions) {
    await loadSubmissions()
  }
}

const onUseTemplate = (template: any) => {
  // Store template in session for use in create forms
  sessionStorage.setItem('bounty-active-template', JSON.stringify(template))
  // Switch to findings tab and open create modal with template
  activeTab.value = 'findings'
  showCreateFindingModal.value = true
}

// Change Event handlers
const viewChangeEvent = (event: any) => {
  selectedChangeEvent.value = event
  showChangeEventDetailModal.value = true
}

const triggerWorkflowFromEvent = async (event: any) => {
  selectedWorkflowEvent.value = event
  selectedWorkflowTemplateId.value = ''
  showSelectWorkflowModal.value = true
  try {
    // Load workflows to select from
    workflowTemplates.value = await invoke('bounty_list_workflow_templates', { category: null, isBuiltIn: null })
  } catch(e) {
    console.error('Failed to load templates:', e)
  }
}

const confirmTriggerWorkflow = async () => {
  if (!selectedWorkflowTemplateId.value || !selectedWorkflowEvent.value) return
  
  try {
    await invoke('bounty_run_workflow_template_for_event', {
      templateId: selectedWorkflowTemplateId.value,
      eventId: selectedWorkflowEvent.value.id,
    })
    if (selectedChangeEvent.value?.id === selectedWorkflowEvent.value.id) {
      selectedChangeEvent.value = await invoke('bounty_get_change_event', {
        id: selectedWorkflowEvent.value.id,
      })
    }
    toast.success(t('bugBounty.changeEvents.workflowTriggered') || 'Workflow triggered successfully')
    showSelectWorkflowModal.value = false
    await loadChangeEventStats() // refresh stats if status changed
  } catch (error: any) {
    console.error('Failed to trigger workflow:', error)
    toast.error(t('bugBounty.errors.createFailed') || 'Failed to trigger workflow')
  }
}

const onChangeEventUpdated = async () => {
  await loadChangeEventStats()
}

const createChangeEvent = async (data: any) => {
  try {
    creating.value = true
    await invoke('bounty_create_change_event', { request: data })
    toast.success(t('bugBounty.changeEvents.eventCreated'))
    showCreateChangeEventModal.value = false
    await loadChangeEventStats()
  } catch (error: any) {
    console.error('Failed to create change event:', error)
    toast.error(error || t('bugBounty.errors.createFailed'))
  } finally {
    creating.value = false
  }
}

const onAssetsDiscovered = async (result: any) => {
  // Refresh asset stats and change event stats after assets are discovered
  await loadAssetStats()
  await loadChangeEventStats()
  // v2 asset panel will auto-refresh via event emission
  showDiscoverAssetsModal.value = false
}

const onAssetsRefreshNeeded = async (payload?: { programId?: string | null }) => {
  await Promise.all([
    loadAssetStats(),
    loadChangeEventStats(),
  ])

  const refreshedProgramId = payload?.programId || null
  if (
    refreshedProgramId &&
    showProgramDetailModal.value &&
    selectedProgram.value?.id === refreshedProgramId
  ) {
    await programDetailModalRef.value?.refreshScopes()
  }
}

// Workflow Template
const viewWorkflowTemplate = (template: any) => {
  selectedWorkflowTemplate.value = template
  showWorkflowTemplateDetailModal.value = true
}

const onWorkflowTemplateUpdated = async () => {
  // Refresh the selected template with latest data from backend
  if (selectedWorkflowTemplate.value?.id) {
    try {
      const updated = await invoke('bounty_get_workflow_template', { 
        id: selectedWorkflowTemplate.value.id 
      }) as any
      if (updated) {
        selectedWorkflowTemplate.value = updated
      }
    } catch (error) {
      console.error('Failed to refresh workflow template:', error)
    }
  }
}

// Batch operations
const batchUpdateFindingStatus = async (ids: string[], status: string) => {
  if (findingBatchActionLoading.value || !ids.length) return
  if (!(await dialog.confirm(t('bugBounty.batch.confirmUpdateStatus', { count: ids.length })))) return
  
  try {
    findingBatchActionLoading.value = true
    const successCount = await invoke('bounty_batch_update_finding_status', { ids, status })
    toast.success(t('bugBounty.batch.updateSuccess', { count: successCount }))
    await refreshFindingsData()
    findingBatchActionVersion.value += 1
  } catch (error) {
    console.error('Batch update failed:', error)
    toast.error(t('bugBounty.errors.updateFailed'))
  } finally {
    findingBatchActionLoading.value = false
  }
}

const batchDeleteFindings = async (ids: string[]) => {
  if (findingBatchActionLoading.value || !ids.length) return
  if (!(await dialog.confirm(t('bugBounty.batch.confirmDelete', { count: ids.length })))) return
  try {
    findingBatchActionLoading.value = true
    const successCount = await invoke('bounty_batch_delete_findings', { ids })
    toast.success(t('bugBounty.batch.deleteSuccess', { count: successCount }))
    await refreshFindingsData()
    findingBatchActionVersion.value += 1
  } catch (error) {
    console.error('Batch delete failed:', error)
    toast.error(t('bugBounty.errors.deleteFailed'))
  } finally {
    findingBatchActionLoading.value = false
  }
}

const deleteAllFindings = async () => {
  if (findingBatchActionLoading.value || findingStats.value.total_findings <= 0) return
  if (!(await dialog.confirm(t('bugBounty.batch.confirmDeleteAll', { count: findingStats.value.total_findings })))) return
  try {
    findingBatchActionLoading.value = true
    const successCount = await invoke<number>('bounty_delete_all_findings')
    toast.success(t('bugBounty.batch.deleteAllSuccess', { count: successCount }))
    await refreshFindingsData()
    findingBatchActionVersion.value += 1
  } catch (error) {
    console.error('Delete all findings failed:', error)
    toast.error(t('bugBounty.errors.deleteFailed'))
  } finally {
    findingBatchActionLoading.value = false
  }
}

const batchUpdateSubmissionStatus = async (ids: string[], status: string) => {
  if (submissionBatchActionLoading.value || !ids.length) return
  if (!(await dialog.confirm(t('bugBounty.batch.confirmUpdateStatus', { count: ids.length })))) return
  
  try {
    submissionBatchActionLoading.value = true
    const successCount = await invoke('bounty_batch_update_submission_status', { ids, status })
    toast.success(t('bugBounty.batch.updateSuccess', { count: successCount }))
    await refreshSubmissionsData()
    submissionBatchActionVersion.value += 1
  } catch (error) {
    console.error('Batch update failed:', error)
    toast.error(t('bugBounty.errors.updateFailed'))
  } finally {
    submissionBatchActionLoading.value = false
  }
}

const batchDeleteSubmissions = async (ids: string[]) => {
  if (submissionBatchActionLoading.value || !ids.length) return
  if (!(await dialog.confirm(t('bugBounty.batch.confirmDeleteSubmissions', { count: ids.length })))) return
  try {
    submissionBatchActionLoading.value = true
    const successCount = await invoke('bounty_batch_delete_submissions', { ids })
    toast.success(t('bugBounty.batch.deleteSuccess', { count: successCount }))
    await refreshSubmissionsData()
    submissionBatchActionVersion.value += 1
  } catch (error) {
    console.error('Batch delete failed:', error)
    toast.error(t('bugBounty.errors.deleteFailed'))
  } finally {
    submissionBatchActionLoading.value = false
  }
}

// Lifecycle
onMounted(async () => {
  await Promise.all([
    loadPrograms(),
    loadStats(),
    loadAssetStats(),
    loadFindingStats(),
    loadSubmissionStats(),
    loadChangeEventStats(),
  ])
  await syncTabFromRoute()
})

watch(
  () => route.query.tab,
  async () => {
    await syncTabFromRoute()
  },
)
</script>
