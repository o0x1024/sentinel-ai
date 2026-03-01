<template>
  <div class="h-full flex flex-col overflow-hidden">
    <div class="flex border-b border-base-300 overflow-x-auto">
      <button
        class="flex-1 py-2 text-xs font-medium transition-colors whitespace-nowrap px-2"
        :class="activeTab === 'tasks' ? 'text-secondary border-b-2 border-secondary bg-secondary/5' : 'text-base-content/50 hover:text-base-content'"
        @click="activeTab = 'tasks'"
      >
        <i class="fas fa-list-check mr-1"></i> Tasks
      </button>
      <button
        class="flex-1 py-2 text-xs font-medium transition-colors whitespace-nowrap px-2"
        :class="activeTab === 'inbox' ? 'text-accent border-b-2 border-accent bg-accent/5' : 'text-base-content/50 hover:text-base-content'"
        @click="activeTab = 'inbox'"
      >
        <i class="fas fa-inbox mr-1"></i> Inbox
      </button>
      <button
        class="flex-1 py-2 text-xs font-medium transition-colors whitespace-nowrap px-2"
        :class="activeTab === 'blackboard' ? 'text-warning border-b-2 border-warning bg-warning/5' : 'text-base-content/50 hover:text-base-content'"
        @click="activeTab = 'blackboard'"
      >
        <i class="fas fa-chalkboard mr-1"></i> Blackboard
      </button>
      <button
        class="flex-1 py-2 text-xs font-medium transition-colors whitespace-nowrap px-2"
        :class="activeTab === 'agents' ? 'text-info border-b-2 border-info bg-info/5' : 'text-base-content/50 hover:text-base-content'"
        @click="activeTab = 'agents'"
      >
        <i class="fas fa-users mr-1"></i> Agents
      </button>
    </div>

    <div v-if="loading" class="flex-1 flex items-center justify-center text-sm text-base-content/50">
      <i class="fas fa-spinner fa-spin mr-2"></i> Team 工作台加载中...
    </div>

    <div v-else-if="activeTab === 'tasks'" class="flex-1 overflow-auto p-3">
      <div v-if="tasks.length === 0" class="text-sm text-base-content/50">暂无任务数据</div>
      <div v-else class="space-y-2">
        <div class="flex items-center justify-between rounded-lg border border-base-300 bg-base-100 px-2 py-1.5 text-[11px] text-base-content/65">
          <span>消息视图: {{ selectedTaskTitle || '全局' }}</span>
          <button
            v-if="selectedTaskId"
            class="btn btn-ghost btn-xs"
            @click="clearSelectedTask"
          >
            取消选择
          </button>
        </div>
        <div
          v-for="task in tasks"
          :key="task.id"
          class="cursor-pointer rounded-lg border bg-base-100 p-2 transition-colors"
          :class="selectedTaskId === task.id ? 'border-primary bg-primary/5 ring-1 ring-primary/30' : 'border-base-300 hover:border-primary/40 hover:bg-base-200/40'"
          @click="toggleSelectedTask(task)"
        >
          <div class="flex items-center justify-between gap-2">
            <div class="text-sm font-medium truncate">{{ task.title || task.task_id }}</div>
            <span class="badge badge-xs" :class="taskStatusBadgeClass(task.status)">{{ task.status }}</span>
          </div>
          <div class="mt-1 text-xs text-base-content/55 line-clamp-2">{{ task.instruction || '—' }}</div>
          <div class="mt-1 flex items-center gap-2 text-[11px] text-base-content/50">
            <span>负责人: {{ resolveAgentName(task.assignee_agent_id) }}</span>
            <span>Attempt: {{ task.attempt }}/{{ task.max_attempts }}</span>
          </div>
          <div class="mt-1 text-[11px] text-primary/90">
            {{ selectedTaskId === task.id ? '已选中，消息窗已切换到该 Agent' : '点击切换到该 Agent 消息窗' }}
          </div>
          <div v-if="task.last_error" class="mt-1 text-[11px] text-error line-clamp-2">{{ task.last_error }}</div>
        </div>
      </div>
    </div>

    <div v-else-if="activeTab === 'inbox'" class="flex-1 overflow-auto p-3">
      <div v-if="sessionMessages.length === 0" class="text-sm text-base-content/50">暂无线程消息</div>
      <div v-else class="space-y-2">
        <div
          v-for="msg in sessionMessages"
          :key="msg.id"
          class="rounded-lg border border-base-300 bg-base-100 p-2"
        >
          <div class="flex items-center justify-between gap-2">
            <div class="text-xs font-semibold">{{ msg.role }}</div>
            <span class="text-[11px] text-base-content/45">{{ formatTimestamp(msg.timestamp) }}</span>
          </div>
          <div class="mt-1 text-[11px] text-base-content/60">
            {{ msg.member_name || msg.member_id || 'system' }}
          </div>
          <div class="mt-1 text-[11px] bg-base-200/60 rounded p-1.5 whitespace-pre-wrap break-words">{{ msg.content || '—' }}</div>
        </div>
      </div>
    </div>

    <div v-else-if="activeTab === 'blackboard'" class="flex-1 overflow-auto p-3">
      <div v-if="blackboardEntries.length === 0" class="text-sm text-base-content/50">暂无白板信息</div>
      <div v-else class="space-y-2">
        <div
          v-for="entry in blackboardEntries"
          :key="entry.id"
          class="rounded-lg border border-base-300 bg-base-100 p-2"
        >
          <div class="flex items-center justify-between gap-2">
            <span class="badge badge-xs" :class="teamBlackboardEntryBadgeClass(entry.entry_type)">{{ entry.entry_type }}</span>
            <span class="text-[11px] text-base-content/45">{{ formatTimestamp(entry.created_at) }}</span>
          </div>
          <div class="mt-1 text-[11px] text-base-content/60">
            Agent: {{ resolveAgentName(entry.agent_id) }} · Task: {{ entry.task_id || '-' }}
          </div>
          <template v-if="isArtifactRefEntry(entry)">
            <div class="mt-1 text-[11px] bg-base-200/60 rounded p-1.5 whitespace-pre-wrap break-words">
              {{ artifactSummary(entry) || entry.content || '—' }}
            </div>
            <div v-if="artifactPath(entry)" class="mt-1 text-[11px] text-base-content/65 break-all">
              File: {{ artifactPath(entry) }}
            </div>
            <div v-if="artifactContainerPath(entry)" class="mt-0.5 text-[11px] text-base-content/55 break-all">
              Container: {{ artifactContainerPath(entry) }}
            </div>
            <div v-if="artifactHostPath(entry)" class="mt-0.5 text-[11px] text-base-content/55 break-all">
              Host: {{ artifactHostPath(entry) }}
            </div>
            <div v-if="artifactBytes(entry) !== null" class="mt-0.5 text-[11px] text-base-content/55">
              Size: {{ formatBytes(artifactBytes(entry) ?? 0) }}
            </div>
          </template>
          <div v-else class="mt-1 text-[11px] bg-base-200/60 rounded p-1.5 whitespace-pre-wrap break-words">{{ entry.content || '—' }}</div>
        </div>
      </div>
    </div>

    <div v-else-if="activeTab === 'agents'" class="flex-1 overflow-auto p-3">
      <div v-if="(sessionDetail?.members || []).length === 0" class="text-sm text-base-content/50">暂无 Agent</div>
      <div v-else class="space-y-2">
        <div
          v-for="member in (sessionDetail?.members || [])"
          :key="member.id"
          class="rounded-lg border border-base-300 bg-base-100 p-2"
        >
          <div class="flex items-center justify-between">
            <div class="text-sm font-medium">{{ member.name }}</div>
            <span class="badge badge-xs" :class="agentStatusBadgeClass(member.id, member.name, member.is_active)">
              {{ agentStatusLabel(member.id, member.name, member.is_active) }}
            </span>
          </div>
          <div class="mt-1 text-[11px] text-base-content/55">{{ member.responsibility || '未设置职责' }}</div>
          <div class="mt-1 text-[11px] text-base-content/50">
            Tokens: {{ member.token_usage }} · Tools: {{ member.tool_calls_count }}
          </div>
        </div>
      </div>
    </div>
    
    <div v-else class="flex-1 overflow-auto p-4">
      <div class="rounded-xl border border-base-300 bg-base-100 p-4 text-sm text-base-content/70">
        Team V3 工作台已启用。
        当前可在 Tasks / Inbox / Blackboard / Agents 查看运行状态。
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import type {
  AgentTeamMessage,
  AgentTeamSession,
  TeamBlackboardEntry,
  TeamTask,
} from '@/types/agentTeam'

const props = defineProps<{
  tab: 'tasks' | 'inbox' | 'blackboard' | 'agents'
  loading: boolean
  tasks: TeamTask[]
  selectedTaskId: string | null
  selectedTaskTitle: string | null
  sessionMessages: AgentTeamMessage[]
  blackboardEntries: TeamBlackboardEntry[]
  sessionDetail: AgentTeamSession | null
  resolveAgentName: (agentId?: string | null) => string
}>()

const emit = defineEmits<{
  (e: 'update:tab', tab: 'tasks' | 'inbox' | 'blackboard' | 'agents'): void
  (e: 'clear-selected-task'): void
  (e: 'toggle-selected-task', task: TeamTask): void
}>()

const activeTab = computed({
  get: () => props.tab,
  set: (val) => emit('update:tab', val)
})

const clearSelectedTask = () => {
  emit('clear-selected-task')
}

const toggleSelectedTask = (task: TeamTask) => {
  emit('toggle-selected-task', task)
}

const taskStatusBadgeClass = (status: string) => {
  const normalized = (status || '').toLowerCase()
  if (normalized === 'completed') return 'badge-success'
  if (normalized === 'running') return 'badge-info'
  if (normalized === 'failed') return 'badge-error'
  if (normalized === 'blocked') return 'badge-warning'
  return 'badge-ghost'
}

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

const agentStatusLabel = (memberId?: string, memberName?: string, isActive?: boolean): string => {
  const status = agentRuntimeStatus(memberId, memberName, isActive)
  if (status === 'running') return 'RUNNING'
  if (status === 'failed') return 'FAILED'
  if (status === 'blocked') return 'BLOCKED'
  if (status === 'completed') return 'COMPLETED'
  if (status === 'pending') return 'PENDING'
  return 'IDLE'
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
