<template>
  <div class="h-full flex flex-col overflow-hidden">
    <div class="flex border-b border-base-300 overflow-x-auto">
      <button
        class="flex-1 py-2 text-xs font-medium transition-colors whitespace-nowrap px-2"
        :class="activeTab === 'tasks' ? 'text-secondary border-b-2 border-secondary bg-secondary/5' : 'text-base-content/50 hover:text-base-content'"
        @click="activeTab = 'tasks'"
      >
        <i class="fas fa-list-check mr-1"></i> {{ t('agent.teamWorkspaceTabTasks') }}
      </button>
      <button
        class="flex-1 py-2 text-xs font-medium transition-colors whitespace-nowrap px-2"
        :class="activeTab === 'inbox' ? 'text-accent border-b-2 border-accent bg-accent/5' : 'text-base-content/50 hover:text-base-content'"
        @click="activeTab = 'inbox'"
      >
        <i class="fas fa-inbox mr-1"></i> {{ t('agent.teamWorkspaceTabInbox') }}
      </button>
      <button
        class="flex-1 py-2 text-xs font-medium transition-colors whitespace-nowrap px-2"
        :class="activeTab === 'blackboard' ? 'text-warning border-b-2 border-warning bg-warning/5' : 'text-base-content/50 hover:text-base-content'"
        @click="activeTab = 'blackboard'"
      >
        <i class="fas fa-chalkboard mr-1"></i> {{ t('agent.teamWorkspaceTabBlackboard') }}
      </button>
      <button
        class="flex-1 py-2 text-xs font-medium transition-colors whitespace-nowrap px-2"
        :class="activeTab === 'agents' ? 'text-info border-b-2 border-info bg-info/5' : 'text-base-content/50 hover:text-base-content'"
        @click="activeTab = 'agents'"
      >
        <i class="fas fa-users mr-1"></i> {{ t('agent.teamWorkspaceTabAgents') }}
      </button>
    </div>

    <div v-if="loading" class="flex-1 flex items-center justify-center text-sm text-base-content/50">
      <i class="fas fa-spinner fa-spin mr-2"></i> {{ t('agent.teamWorkspaceLoading') }}
    </div>

    <TeamWorkspaceTasksTab
      v-else-if="activeTab === 'tasks'"
      :pending-create-task="pendingCreateTask"
      :tasks="tasks"
      :selected-task-id="selectedTaskId"
      :selected-task-title="selectedTaskTitle"
      :pending-task-action-kind="pendingTaskActionKind || null"
      :pending-task-action-task-id="pendingTaskActionTaskId || null"
      :team-members="sessionDetail?.members || []"
      :resolve-agent-name="resolveAgentName"
      @clear-selected-task="emit('clear-selected-task')"
      @create-task="emit('create-task', $event)"
      @toggle-selected-task="emit('toggle-selected-task', $event)"
      @claim-task="emit('claim-task', $event)"
      @release-task="emit('release-task', $event)"
      @complete-task="emit('complete-task', $event)"
      @fail-task="emit('fail-task', $event)"
      @block-task="emit('block-task', $event)"
    />

    <TeamWorkspaceInboxTab
      v-else-if="activeTab === 'inbox'"
      :session-messages="sessionMessages"
      :format-timestamp="formatTimestamp"
    />

    <TeamWorkspaceBlackboardTab
      v-else-if="activeTab === 'blackboard'"
      :blackboard-entries="blackboardEntries"
      :format-timestamp="formatTimestamp"
      :resolve-agent-name="resolveAgentName"
      :badge-class="teamBlackboardEntryBadgeClass"
      :label-key="blackboardEntryLabelKey"
      :is-artifact-ref-entry="isArtifactRefEntry"
      :artifact-summary="artifactSummary"
      :artifact-path="artifactPath"
      :artifact-container-path="artifactContainerPath"
      :artifact-host-path="artifactHostPath"
      :artifact-bytes="artifactBytes"
      :format-bytes="formatBytes"
    />

    <TeamWorkspaceAgentsTab
      v-else-if="activeTab === 'agents'"
      :members="sessionDetail?.members || []"
      :agent-status-i18n-key="agentStatusI18nKey"
      :agent-status-badge-class="agentStatusBadgeClass"
    />
    
    <div v-else class="flex-1 overflow-auto p-4">
      <div class="rounded-xl border border-base-300 bg-base-100 p-4 text-sm text-base-content/70">
        {{ t('agent.teamWorkspaceEnabledMessage') }}
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import type {
  AgentTeamMessage,
  AgentTeamSession,
  TeamBlackboardEntry,
  TeamTaskCreateInput,
  TeamTaskReasonActionInput,
  TeamTask,
} from '@/types/agentTeam'
import {
  teamBlackboardEntryTypeI18nKey,
  teamWorkspaceAgentStatusI18nKey,
} from './teamWorkspacePresentation'
import TeamWorkspaceAgentsTab from './TeamWorkspaceAgentsTab.vue'
import TeamWorkspaceBlackboardTab from './TeamWorkspaceBlackboardTab.vue'
import TeamWorkspaceInboxTab from './TeamWorkspaceInboxTab.vue'
import TeamWorkspaceTasksTab from './TeamWorkspaceTasksTab.vue'

const props = defineProps<{
  tab: 'tasks' | 'inbox' | 'blackboard' | 'agents'
  loading: boolean
  pendingCreateTask?: boolean
  tasks: TeamTask[]
  selectedTaskId: string | null
  selectedTaskTitle: string | null
  pendingTaskActionKind?: 'claim' | 'release' | 'complete' | 'fail' | 'block' | null
  pendingTaskActionTaskId?: string | null
  sessionMessages: AgentTeamMessage[]
  blackboardEntries: TeamBlackboardEntry[]
  sessionDetail: AgentTeamSession | null
  resolveAgentName: (agentId?: string | null) => string
}>()

const emit = defineEmits<{
  (e: 'update:tab', tab: 'tasks' | 'inbox' | 'blackboard' | 'agents'): void
  (e: 'clear-selected-task'): void
  (e: 'claim-task', task: TeamTask): void
  (e: 'complete-task', task: TeamTask): void
  (e: 'create-task', input: TeamTaskCreateInput): void
  (e: 'fail-task', input: TeamTaskReasonActionInput): void
  (e: 'block-task', input: TeamTaskReasonActionInput): void
  (e: 'release-task', task: TeamTask): void
  (e: 'toggle-selected-task', task: TeamTask): void
}>()

const { t } = useI18n()

const activeTab = computed({
  get: () => props.tab,
  set: (val) => emit('update:tab', val)
})

const AGENT_STATUS_PRIORITY: Record<string, number> = {
  idle: 0,
  pending: 1,
  completed: 2,
  running: 3,
  blocked: 4,
  failed: 5,
}

const normalizeAgentRuntimeStatus = (value: unknown): string => {
  const normalized = String(value || '').trim().toLowerCase()
  if (!normalized) return 'idle'
  if (normalized.includes('fail') || normalized.includes('error')) return 'failed'
  if (normalized.includes('block')) return 'blocked'
  if (
    normalized.includes('run') ||
    normalized.includes('execut') ||
    normalized.includes('claim') ||
    normalized.includes('wait')
  ) {
    return 'running'
  }
  if (normalized.includes('done') || normalized.includes('complete')) return 'completed'
  if (normalized.includes('queue') || normalized.includes('pending') || normalized.includes('ready')) {
    return 'pending'
  }
  return 'idle'
}

const mergeAgentStatus = (current: string | undefined, candidate: string): string => {
  const currentRank = AGENT_STATUS_PRIORITY[current || 'idle'] ?? 0
  const candidateRank = AGENT_STATUS_PRIORITY[candidate] ?? 0
  return candidateRank >= currentRank ? candidate : (current || 'idle')
}

const memberStatusById = computed(() => {
  const out = new Map<string, string>()

  for (const member of props.sessionDetail?.members || []) {
    if (!member?.id) continue
    const base = member.is_active ? 'running' : 'idle'
    out.set(member.id, mergeAgentStatus(out.get(member.id), base))
  }

  for (const task of props.tasks || []) {
    const assignee = typeof task.assignee_agent_id === 'string' ? task.assignee_agent_id.trim() : ''
    if (!assignee) continue
    const candidate = normalizeAgentRuntimeStatus(task.status)
    out.set(assignee, mergeAgentStatus(out.get(assignee), candidate))
  }

  return out
})

const agentRuntimeStatus = (memberId?: string, _memberName?: string, isActive?: boolean): string => {
  const key = typeof memberId === 'string' ? memberId.trim() : ''
  if (key && memberStatusById.value.has(key)) {
    return memberStatusById.value.get(key) || 'idle'
  }
  return isActive ? 'running' : 'idle'
}

const agentStatusI18nKey = (memberId?: string, memberName?: string, isActive?: boolean): string => {
  const status = agentRuntimeStatus(memberId, memberName, isActive)
  return teamWorkspaceAgentStatusI18nKey(status)
}

const agentStatusBadgeClass = (memberId?: string, memberName?: string, isActive?: boolean): string => {
  const status = agentRuntimeStatus(memberId, memberName, isActive)
  if (status === 'running') return 'badge-info'
  if (status === 'failed') return 'badge-error'
  if (status === 'blocked') return 'badge-warning'
  if (status === 'completed') return 'badge-success'
  if (status === 'pending') return 'badge-secondary'
  return 'badge-ghost'
}

const teamBlackboardEntryBadgeClass = (entryType?: string | null) => {
  const normalized = String(entryType || '').toLowerCase()
  if (normalized === 'task_output') return 'badge-success'
  if (normalized === 'artifact_ref') return 'badge-neutral'
  if (normalized === 'task_error') return 'badge-error'
  if (normalized === 'task_start') return 'badge-info'
  if (normalized === 'plan') return 'badge-secondary'
  if (normalized === 'plan_fallback') return 'badge-warning'
  if (normalized === 'goal') return 'badge-accent'
  return 'badge-ghost'
}

const blackboardEntryLabelKey = (entryType?: string | null) => {
  return teamBlackboardEntryTypeI18nKey(entryType)
}

const isArtifactRefEntry = (entry: TeamBlackboardEntry) => {
  return String(entry.entry_type || '').toLowerCase() === 'artifact_ref'
}

const artifactSummary = (entry: TeamBlackboardEntry) => {
  const raw = entry.metadata?.summary
  return typeof raw === 'string' ? raw : ''
}

const artifactPath = (entry: TeamBlackboardEntry) => {
  const raw = entry.metadata?.artifact?.path
  return typeof raw === 'string' ? raw : ''
}

const artifactBytes = (entry: TeamBlackboardEntry) => {
  const raw = entry.metadata?.artifact?.bytes
  const bytes = Number(raw)
  return Number.isFinite(bytes) && bytes >= 0 ? bytes : null
}

const artifactContainerPath = (entry: TeamBlackboardEntry) => {
  const raw = entry.metadata?.artifact?.container_path
  return typeof raw === 'string' ? raw : ''
}

const artifactHostPath = (entry: TeamBlackboardEntry) => {
  const raw = entry.metadata?.artifact?.host_path
  return typeof raw === 'string' ? raw : ''
}

const formatBytes = (bytes: number) => {
  if (bytes < 1024) return `${bytes} B`
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`
}

const formatTimestamp = (value?: string | null) => {
  if (!value) return '—'
  const time = new Date(value).getTime()
  if (!Number.isFinite(time)) return value
  return new Date(time).toLocaleString()
}
</script>
