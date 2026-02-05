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
      <a class="tab" :class="{ 'tab-active': activeTab === 'findings' }" @click="switchTab('findings')">
        <i class="fas fa-bug mr-2"></i>
        {{ t('bugBounty.tabs.findings') }}
        <span v-if="findingStats.total_findings > 0" class="badge badge-sm ml-2">{{ findingStats.total_findings }}</span>
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
    <div class="flex-1 min-h-0 overflow-auto">
      <!-- Programs Tab -->
      <ProgramsPanel 
        v-if="activeTab === 'programs'"
        :programs="programs"
        :loading="loading"
        @create="showCreateProgramModal = true"
        @select="selectProgram"
        @edit="editProgram"
        @delete="deleteProgram"
      />

      <!-- Assets Tab -->
      <AssetsPanel 
        ref="assetsPanelRef"
        v-if="activeTab === 'assets'"
        :selected-program="selectedProgram"
        :programs="programs"
        @stats-updated="updateAssetStats"
        @discover-assets="showDiscoverAssetsModal = true"
      />

      <!-- Findings Tab -->
      <FindingsPanel 
        v-if="activeTab === 'findings'"
        :findings="findings"
        :programs="programs"
        :loading="loadingFindings"
        :page="findingPage"
        :page-size="pageSize"
        :has-next="findingHasNext"
        @create="showCreateFindingModal = true"
        @view="viewFinding"
        @delete="deleteFinding"
        @create-submission="createSubmissionFromFinding"
        @filter-change="onFindingFilterChange"
        @batch-update-status="batchUpdateFindingStatus"
        @batch-delete="batchDeleteFindings"
        @page-change="onFindingPageChange"
      />

      <!-- Submissions Tab -->
      <SubmissionsPanel 
        v-if="activeTab === 'submissions'"
        :submissions="submissions"
        :loading="loadingSubmissions"
        :page="submissionPage"
        :page-size="pageSize"
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

      <!-- Statistics Tab -->
      <StatisticsPanel 
        v-if="activeTab === 'statistics'"
        :finding-stats="findingStats"
        :submission-stats="submissionStats"
        :programs="programs"
        :findings="findings"
        :submissions="submissions"
      />

      <!-- Import/Export Tab -->
      <ImportExportPanel 
        v-if="activeTab === 'import-export'"
        :programs="programs"
        :findings="findings"
        :submissions="submissions"
        @imported="onDataImported"
      />

      <!-- Templates Tab -->
      <ReportTemplatesPanel 
        v-if="activeTab === 'templates'"
        @use-template="onUseTemplate"
      />

      <!-- Change Events Tab -->
      <ChangeEventsPanel 
        v-if="activeTab === 'changes'"
        @view="viewChangeEvent"
        @trigger-workflow="triggerWorkflowFromEvent"
        @create="showCreateChangeEventModal = true"
      />

      <!-- Workflow Templates Tab -->
      <WorkflowTemplatesPanel
        v-if="activeTab === 'workflows'"
        :programs="programs"
        :selected-program="selectedProgram"
        @view="viewWorkflowTemplate"
      />

      <!-- Monitor Tab -->
      <MonitorPanel
        v-if="activeTab === 'monitor'"
        :selected-program="selectedProgram"
        :programs="programs"
      />
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
import { ref, computed, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { invoke } from '@tauri-apps/api/core'
import { useToast } from '../composables/useToast'
import { 
  ProgramsPanel, 
  FindingsPanel, 
  SubmissionsPanel, 
  AssetsPanel,
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

const { t } = useI18n()
const toast = useToast()

// State
const loading = ref(false)
const loadingFindings = ref(false)
const loadingSubmissions = ref(false)
const creating = ref(false)
const activeTab = ref('programs')

// Component refs
const assetsPanelRef = ref<InstanceType<typeof AssetsPanel> | null>(null)

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
const submissionInitialData = ref<any>(null)
const selectedProgram = ref<any>(null)
const selectedFinding = ref<any>(null)
const selectedSubmission = ref<any>(null)
const selectedChangeEvent = ref<any>(null)
const selectedWorkflowTemplate = ref<any>(null)
const editingProgram = ref<any>(null) // Program being edited

// Data
const programs = ref<any[]>([])
const findings = ref<any[]>([])
const submissions = ref<any[]>([])
const programFindings = ref<any[]>([])
const findingPage = ref(1)
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

// Methods
const switchTab = async (tab: string) => {
  activeTab.value = tab
  if (tab === 'findings') {
    await loadFindings()
  } else if (tab === 'submissions') {
    await loadSubmissions()
  }
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

const loadFindings = async () => {
  try {
    loadingFindings.value = true
    const filter: any = {}
    if (findingFilter.value.severity) {
      filter.severities = [findingFilter.value.severity]
    }
    if (findingFilter.value.status) {
      filter.statuses = [findingFilter.value.status]
    }
    if (findingFilter.value.search) {
      filter.search = findingFilter.value.search
    }
    filter.sort_by = 'created_at'
    filter.sort_dir = 'desc'
    filter.limit = pageSize.value
    filter.offset = (findingPage.value - 1) * pageSize.value
    findings.value = await invoke('bounty_list_findings', { filter: Object.keys(filter).length > 0 ? filter : null })
    findingHasNext.value = findings.value.length === pageSize.value
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
    const filter: any = {}
    if (submissionFilter.value.status) {
      filter.statuses = [submissionFilter.value.status]
    }
    if (submissionFilter.value.search) {
      filter.search = submissionFilter.value.search
    }
    filter.sort_by = 'created_at'
    filter.sort_dir = 'desc'
    filter.limit = pageSize.value
    filter.offset = (submissionPage.value - 1) * pageSize.value
    submissions.value = await invoke('bounty_list_submissions', { filter: Object.keys(filter).length > 0 ? filter : null })
    submissionHasNext.value = submissions.value.length === pageSize.value
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

const updateAssetStats = (stats: any) => {
  assetStats.value = {
    total: stats?.total || 0,
    active: stats?.active || 0,
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
    await loadPrograms()
    await loadStats()
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
    await loadFindings()
    await loadFindingStats()
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
    closeSubmissionModal()
    await loadSubmissions()
    await loadSubmissionStats()
  } catch (error) {
    console.error('Failed to create submission:', error)
    toast.error(t('bugBounty.errors.createFailed'))
  } finally {
    creating.value = false
  }
}

const deleteFinding = async (finding: any) => {
  if (!confirm(t('bugBounty.confirm.deleteFinding'))) return
  try {
    await invoke('bounty_delete_finding', { id: finding.id })
    toast.success(t('bugBounty.success.findingDeleted'))
    await loadFindings()
    await loadFindingStats()
  } catch (error) {
    console.error('Failed to delete finding:', error)
    toast.error(t('bugBounty.errors.deleteFailed'))
  }
}

const deleteSubmission = async (submission: any) => {
  if (!confirm(t('bugBounty.confirm.deleteSubmission'))) return
  try {
    await invoke('bounty_delete_submission', { id: submission.id })
    toast.success(t('bugBounty.success.submissionDeleted'))
    await loadSubmissions()
    await loadSubmissionStats()
  } catch (error) {
    console.error('Failed to delete submission:', error)
    toast.error(t('bugBounty.errors.deleteFailed'))
  }
}

const deleteProgram = async (program: any) => {
  if (!confirm(t('bugBounty.confirm.deleteProgram'))) return
  try {
    await invoke('bounty_delete_program', { id: program.id })
    toast.success(t('bugBounty.success.programDeleted'))
    await loadPrograms()
    await loadStats()
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

const onProgramUpdated = async () => {
  await loadPrograms()
  await loadStats()
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
  await loadFindings()
  await loadFindingStats()
}

const viewSubmission = (submission: any) => {
  selectedSubmission.value = submission
  showSubmissionDetailModal.value = true
}

const onSubmissionUpdated = async () => {
  await loadSubmissions()
  await loadSubmissionStats()
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
  toast.info(t('bugBounty.comingSoon'))
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
  findingPage.value = page
  loadFindings()
}

const onSubmissionPageChange = (page: number) => {
  submissionPage.value = page
  loadSubmissions()
}

const onDataImported = async () => {
  await loadPrograms()
  await loadStats()
  await loadFindings()
  await loadFindingStats()
  await loadSubmissions()
  await loadSubmissionStats()
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
  // TODO: Trigger workflow from change event
  toast.info(t('bugBounty.comingSoon'))
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
  await loadChangeEventStats()
  
  // Refresh assets panel to show newly imported assets
  if (assetsPanelRef.value) {
    assetsPanelRef.value.refreshAssets()
  }
  
  showDiscoverAssetsModal.value = false
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
  if (!confirm(t('bugBounty.batch.confirmUpdateStatus', { count: ids.length }))) return
  
  try {
    let successCount = 0
    for (const id of ids) {
      try {
        await invoke('bounty_update_finding', { id, request: { status } })
        successCount++
      } catch (error) {
        console.error(`Failed to update finding ${id}:`, error)
      }
    }
    toast.success(t('bugBounty.batch.updateSuccess', { count: successCount }))
    await loadFindings()
    await loadFindingStats()
  } catch (error) {
    console.error('Batch update failed:', error)
    toast.error(t('bugBounty.errors.updateFailed'))
  }
}

const batchDeleteFindings = async (ids: string[]) => {
  
  try {
    let successCount = 0
    for (const id of ids) {
      try {
        await invoke('bounty_delete_finding', { id })
        successCount++
      } catch (error) {
        console.error(`Failed to delete finding ${id}:`, error)
      }
    }
    toast.success(t('bugBounty.batch.deleteSuccess', { count: successCount }))
    await loadFindings()
    await loadFindingStats()
  } catch (error) {
    console.error('Batch delete failed:', error)
    toast.error(t('bugBounty.errors.deleteFailed'))
  }
}

const batchUpdateSubmissionStatus = async (ids: string[], status: string) => {
  if (!confirm(t('bugBounty.batch.confirmUpdateStatus', { count: ids.length }))) return
  
  try {
    let successCount = 0
    for (const id of ids) {
      try {
        await invoke('bounty_update_submission', { id, request: { status } })
        successCount++
      } catch (error) {
        console.error(`Failed to update submission ${id}:`, error)
      }
    }
    toast.success(t('bugBounty.batch.updateSuccess', { count: successCount }))
    await loadSubmissions()
    await loadSubmissionStats()
  } catch (error) {
    console.error('Batch update failed:', error)
    toast.error(t('bugBounty.errors.updateFailed'))
  }
}

const batchDeleteSubmissions = async (ids: string[]) => {
  
  try {
    let successCount = 0
    for (const id of ids) {
      try {
        await invoke('bounty_delete_submission', { id })
        successCount++
      } catch (error) {
        console.error(`Failed to delete submission ${id}:`, error)
      }
    }
    toast.success(t('bugBounty.batch.deleteSuccess', { count: successCount }))
    await loadSubmissions()
    await loadSubmissionStats()
  } catch (error) {
    console.error('Batch delete failed:', error)
    toast.error(t('bugBounty.errors.deleteFailed'))
  }
}

// Lifecycle
onMounted(async () => {
  await loadPrograms()
  await loadStats()
  await loadFindingStats()
  await loadSubmissionStats()
  await loadChangeEventStats()
})
</script>
