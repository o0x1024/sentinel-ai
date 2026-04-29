<template>
  <div class="h-full flex flex-col overflow-hidden">
    <div class="border-b border-base-300 px-4 py-3">
      <div class="flex items-center justify-between gap-3">
        <div class="min-w-0">
          <div class="text-sm font-semibold text-base-content">Team v4</div>
          <div class="truncate text-xs text-base-content/60">
            {{ run?.goal || 'No active Team v4 run' }}
          </div>
        </div>
        <span class="badge badge-sm" :class="runStateBadgeClass">{{ run?.state || 'idle' }}</span>
      </div>
      <div v-if="run?.id" class="mt-2 text-[11px] text-base-content/50 break-all">
        run: {{ run.id }}
      </div>
    </div>

    <div class="flex border-b border-base-300 overflow-x-auto">
      <button
        v-for="item in tabs"
        :key="item.key"
        class="flex-1 whitespace-nowrap px-2 py-2 text-xs font-medium transition-colors"
        :class="activeTab === item.key ? item.activeClass : 'text-base-content/50 hover:text-base-content'"
        @click="activeTab = item.key"
      >
        <i :class="item.icon" class="mr-1"></i>{{ item.label }}
      </button>
    </div>

    <div v-if="loading" class="flex-1 flex items-center justify-center text-sm text-base-content/50">
      <i class="fas fa-spinner fa-spin mr-2"></i> Loading Team v4 workspace...
    </div>

    <div v-else-if="!run" class="flex-1 overflow-auto p-4">
      <div class="rounded-lg border border-base-300 bg-base-100 p-4 text-sm text-base-content/70">
        Team v4 run will appear here after a Team-mode submission.
      </div>
    </div>

    <div v-else-if="activeTab === 'activity'" class="flex-1 overflow-auto p-4 space-y-3">
      <div class="rounded-lg border border-base-300 bg-base-100 p-3">
        <div class="flex items-start justify-between gap-3">
          <div class="min-w-0">
            <div class="text-sm font-semibold text-base-content">{{ observability.headline }}</div>
            <div class="mt-1 text-xs leading-relaxed text-base-content/70">{{ observability.detail }}</div>
          </div>
          <span class="badge badge-sm" :class="phaseBadgeClass(observability.phase)">
            {{ observability.phase }}
          </span>
        </div>
        <div class="mt-3 grid grid-cols-4 gap-2">
          <StatTile label="Tasks" :value="`${observability.taskProgress.completed}/${observability.taskProgress.total}`" />
          <StatTile label="Running" :value="String(observability.taskProgress.running)" />
          <StatTile label="Failed" :value="String(observability.taskProgress.failed)" />
          <StatTile label="Tools" :value="String(observability.toolEvents.length)" />
        </div>
      </div>

      <div class="rounded-lg border border-base-300 bg-base-100 p-3">
        <div class="text-xs font-semibold uppercase tracking-wide text-base-content/50">Live Agents</div>
        <div class="mt-3 space-y-2">
          <div
            v-for="item in observability.agentStates"
            :key="item.agent.id"
            class="rounded border border-base-300 bg-base-200/50 p-2"
          >
            <div class="flex items-center justify-between gap-2">
              <div class="min-w-0">
                <div class="truncate text-sm font-medium text-base-content">{{ item.agent.name }}</div>
                <div class="truncate text-xs text-base-content/60">{{ item.agent.role_type }} · {{ item.detail }}</div>
              </div>
              <span class="badge badge-xs" :class="agentBadgeClass(item.status)">{{ item.status }}</span>
            </div>
          </div>
        </div>
      </div>

      <div class="rounded-lg border border-base-300 bg-base-100 p-3">
        <div class="text-xs font-semibold uppercase tracking-wide text-base-content/50">Recent Activity</div>
        <div class="mt-3 space-y-2">
          <div
            v-for="event in observability.recentEvents"
            :key="event.id"
            class="rounded border border-base-300 bg-base-200/50 p-2"
          >
            <div class="flex items-center justify-between gap-2">
              <div class="min-w-0">
                <div class="truncate text-sm font-medium text-base-content">{{ event.title }}</div>
                <div class="mt-1 text-xs leading-relaxed text-base-content/60">{{ event.description }}</div>
              </div>
              <span class="badge badge-xs" :class="severityBadgeClass(event.severity)">
                #{{ event.sequence }}
              </span>
            </div>
            <div class="mt-1 text-[11px] text-base-content/40">
              {{ formatTimestamp(event.createdAt) }} · {{ event.role }} · {{ event.phase }}
            </div>
          </div>
          <EmptyState v-if="observability.recentEvents.length === 0" label="No observable activity yet." />
        </div>
      </div>
    </div>

    <div v-else-if="activeTab === 'tasks'" class="flex-1 overflow-auto p-4 space-y-3">
      <div
        v-for="task in tasks"
        :key="task.id"
        class="rounded-lg border border-base-300 bg-base-100 p-3"
      >
        <div class="flex items-start justify-between gap-3">
          <div class="min-w-0">
            <div class="text-sm font-medium text-base-content">{{ task.title }}</div>
            <div class="mt-1 text-xs text-base-content/60 break-words">{{ task.instruction }}</div>
          </div>
          <span class="badge badge-sm" :class="taskBadgeClass(task.status)">{{ task.status }}</span>
        </div>
        <div class="mt-2 flex flex-wrap gap-2 text-[11px] text-base-content/50">
          <span>key: {{ task.task_key }}</span>
          <span v-if="task.assigned_agent_id">assignee: {{ agentName(task.assigned_agent_id) }}</span>
          <span v-if="task.context_snapshot_id">context: {{ task.context_snapshot_id }}</span>
        </div>
      </div>
      <EmptyState v-if="tasks.length === 0" label="No Team v4 tasks yet." />
    </div>

    <div v-else-if="activeTab === 'evidence'" class="flex-1 overflow-auto p-4 space-y-3">
      <div
        v-for="memory in evidenceMemories"
        :key="memory.id"
        class="rounded-lg border border-base-300 bg-base-100 p-3"
      >
        <div class="flex items-start justify-between gap-3">
          <div class="min-w-0">
            <div class="text-sm font-medium text-base-content">{{ memory.kind }}</div>
            <div v-if="memorySummary(memory)" class="mt-1 text-xs font-medium text-base-content/80">
              {{ memorySummary(memory) }}
            </div>
            <div class="mt-1 whitespace-pre-wrap text-xs leading-relaxed text-base-content/70">{{ memory.content }}</div>
          </div>
          <span class="badge badge-sm" :class="memoryKindBadgeClass(memory.kind)">
            {{ confidenceLabel(memory.confidence) }}
          </span>
        </div>
        <div class="mt-2 flex flex-wrap gap-2 text-[11px] text-base-content/50">
          <span v-if="memory.task_id">task: {{ taskTitle(memory.task_id) }}</span>
          <span>{{ memory.accepted_by_commander ? 'accepted' : 'candidate' }}</span>
          <span v-if="memory.promoted_to_long_term">long-term</span>
        </div>
        <div v-if="memoryEvidence(memory).length" class="mt-2 space-y-1">
          <div
            v-for="item in memoryEvidence(memory)"
            :key="item"
            class="rounded bg-base-200 px-2 py-1 text-[11px] leading-relaxed text-base-content/70"
          >
            {{ item }}
          </div>
        </div>
      </div>
      <EmptyState v-if="evidenceMemories.length === 0" label="No curated evidence yet." />
    </div>

    <div v-else-if="activeTab === 'memory'" class="flex-1 overflow-auto p-4 space-y-3">
      <div class="grid grid-cols-2 gap-2">
        <StatTile label="Accepted" :value="String(acceptedMemoryCount)" />
        <StatTile label="Long-term" :value="String(promotedMemoryCount)" />
      </div>
      <div
        v-for="memory in memories"
        :key="memory.id"
        class="rounded-lg border border-base-300 bg-base-100 p-3"
      >
        <div class="flex items-start justify-between gap-3">
          <div class="min-w-0">
            <div class="text-sm font-medium text-base-content">{{ memory.kind }}</div>
            <div class="mt-1 whitespace-pre-wrap text-xs leading-relaxed text-base-content/70">{{ memory.content }}</div>
          </div>
          <div class="flex flex-col items-end gap-1">
            <span class="badge badge-xs" :class="memory.accepted_by_commander ? 'badge-success' : 'badge-warning'">
              {{ memory.accepted_by_commander ? 'accepted' : 'candidate' }}
            </span>
            <span v-if="memory.promoted_to_long_term" class="badge badge-xs badge-primary">long-term</span>
          </div>
        </div>
        <div class="mt-2 text-[11px] text-base-content/50">
          {{ formatTimestamp(memory.created_at) }}
          <span v-if="memory.task_id"> · {{ taskTitle(memory.task_id) }}</span>
        </div>
      </div>
      <EmptyState v-if="memories.length === 0" label="No Team v4 memories yet." />
    </div>

    <div v-else-if="activeTab === 'harness'" class="flex-1 overflow-auto p-4 space-y-3">
      <div
        v-for="harness in harnessRuns"
        :key="harness.id"
        class="rounded-lg border border-base-300 bg-base-100 p-3"
      >
        <div class="flex items-start justify-between gap-3">
          <div class="min-w-0">
            <div class="text-sm font-medium text-base-content">{{ taskTitle(harness.task_id) }}</div>
            <div class="mt-1 text-xs text-base-content/60">
              {{ agentName(harness.actor_id) }} · checkpoint #{{ harness.checkpoint_sequence }}
            </div>
          </div>
          <span class="badge badge-sm" :class="harnessBadgeClass(harness)">
            {{ harness.status }}
          </span>
        </div>
        <div class="mt-2 grid grid-cols-1 gap-1 text-[11px] text-base-content/50">
          <div>lease: {{ formatTimestamp(harness.lease_expires_at) || 'none' }}</div>
          <div>heartbeat: {{ formatTimestamp(harness.last_heartbeat_at) || 'none' }}</div>
          <div class="break-all">harness: {{ harness.id }}</div>
        </div>
        <div class="mt-3 flex flex-wrap gap-2">
          <button
            class="btn btn-xs btn-outline btn-warning"
            @click="emit('resumeHarness', harness.id)"
          >
            Resume
          </button>
          <button
            class="btn btn-xs btn-outline btn-error"
            :disabled="harness.status === 'cancelled'"
            @click="emit('cancelHarness', harness.id)"
          >
            Cancel
          </button>
        </div>
      </div>
      <EmptyState v-if="harnessRuns.length === 0" label="No Harness runs yet." />
    </div>

    <div v-else-if="activeTab === 'tools'" class="flex-1 overflow-auto p-4 space-y-3">
      <div
        v-for="event in observability.toolEvents"
        :key="event.id"
        class="rounded-lg border border-base-300 bg-base-100 p-3"
      >
        <div class="flex items-start justify-between gap-3">
          <div class="min-w-0">
            <div class="text-sm font-medium text-base-content">{{ event.toolName || event.title }}</div>
            <div class="mt-1 text-xs leading-relaxed text-base-content/70">{{ event.description }}</div>
          </div>
          <span class="badge badge-sm" :class="severityBadgeClass(event.severity)">
            {{ event.phase }}
          </span>
        </div>
        <div class="mt-2 flex flex-wrap gap-2 text-[11px] text-base-content/50">
          <span>#{{ event.sequence }}</span>
          <span>{{ formatTimestamp(event.createdAt) }}</span>
          <span>{{ event.role }}</span>
          <span v-if="event.taskId">{{ taskTitle(event.taskId) }}</span>
        </div>
      </div>
      <EmptyState v-if="observability.toolEvents.length === 0" label="No Team v4 tool runs yet." />
    </div>

    <div v-else-if="activeTab === 'events'" class="flex-1 overflow-auto p-4 space-y-3">
      <div
        v-for="event in events"
        :key="event.id"
        class="rounded-lg border border-base-300 bg-base-100 p-3"
      >
        <div class="flex items-center justify-between gap-3">
          <div class="text-sm font-medium text-base-content">{{ event.event_type }}</div>
          <span class="badge badge-xs" :class="eventVisibilityBadgeClass(event.visibility)">
            #{{ event.sequence }} {{ event.visibility }}
          </span>
        </div>
        <div class="mt-1 text-xs text-base-content/50">
          {{ formatTimestamp(event.created_at) }}
          <span v-if="event.actor_id"> · {{ agentName(event.actor_id) }}</span>
        </div>
        <pre class="mt-2 max-h-40 overflow-auto rounded bg-base-200 p-2 text-[11px] leading-relaxed text-base-content/70">{{ formatPayload(event.payload) }}</pre>
      </div>
      <EmptyState v-if="events.length === 0" label="No Team v4 events yet." />
    </div>

    <div v-else-if="activeTab === 'agents'" class="flex-1 overflow-auto p-4 space-y-3">
      <div
        v-for="agent in agents"
        :key="agent.id"
        class="rounded-lg border border-base-300 bg-base-100 p-3"
      >
        <div class="flex items-start justify-between gap-3">
          <div class="min-w-0">
            <div class="text-sm font-medium text-base-content">{{ agent.name }}</div>
            <div class="mt-1 text-xs text-base-content/60">
              {{ agent.role_type }} · {{ agent.model || 'default model' }} · {{ agent.context_mode || 'default context' }}
            </div>
          </div>
          <span class="badge badge-sm" :class="agentBadgeClass(agentRuntimeStatus(agent))">
            {{ agentRuntimeStatus(agent) }}
          </span>
        </div>
        <pre class="mt-2 max-h-32 overflow-auto rounded bg-base-200 p-2 text-[11px] leading-relaxed text-base-content/70">{{ formatPayload(agent.metadata) }}</pre>
      </div>
      <EmptyState v-if="agents.length === 0" label="No Team v4 agents yet." />
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, defineComponent, h, ref, watch } from 'vue'
import { deriveTeamV4Observability } from './teamV4Observability'
import type {
  TeamV4Agent,
  TeamV4Event,
  TeamV4HarnessRun,
  TeamV4Memory,
  TeamV4Run,
  TeamV4Task,
} from '@/types/teamRuntime'

const props = defineProps<{
  agents: TeamV4Agent[]
  events: TeamV4Event[]
  harnessRuns: TeamV4HarnessRun[]
  loading: boolean
  memories: TeamV4Memory[]
  run: TeamV4Run | null
  tasks: TeamV4Task[]
}>()

const emit = defineEmits<{
  (e: 'cancelHarness', harnessRunId: string): void
  (e: 'resumeHarness', harnessRunId: string): void
}>()

const activeTab = ref<'activity' | 'tasks' | 'evidence' | 'memory' | 'harness' | 'tools' | 'events' | 'agents'>('activity')

watch(
  () => props.run?.id,
  () => {
    activeTab.value = 'activity'
  },
)

const tabs = [
  {
    key: 'activity' as const,
    label: 'Activity',
    icon: 'fas fa-signal',
    activeClass: 'text-info border-b-2 border-info bg-info/5',
  },
  {
    key: 'tasks' as const,
    label: 'Tasks',
    icon: 'fas fa-list-check',
    activeClass: 'text-secondary border-b-2 border-secondary bg-secondary/5',
  },
  {
    key: 'evidence' as const,
    label: 'Evidence',
    icon: 'fas fa-circle-check',
    activeClass: 'text-success border-b-2 border-success bg-success/5',
  },
  {
    key: 'memory' as const,
    label: 'Memory',
    icon: 'fas fa-brain',
    activeClass: 'text-primary border-b-2 border-primary bg-primary/5',
  },
  {
    key: 'harness' as const,
    label: 'Harness',
    icon: 'fas fa-heart-pulse',
    activeClass: 'text-warning border-b-2 border-warning bg-warning/5',
  },
  {
    key: 'tools' as const,
    label: 'Tools',
    icon: 'fas fa-screwdriver-wrench',
    activeClass: 'text-primary border-b-2 border-primary bg-primary/5',
  },
  {
    key: 'events' as const,
    label: 'Events',
    icon: 'fas fa-timeline',
    activeClass: 'text-accent border-b-2 border-accent bg-accent/5',
  },
  {
    key: 'agents' as const,
    label: 'Agents',
    icon: 'fas fa-users',
    activeClass: 'text-info border-b-2 border-info bg-info/5',
  },
]

const agentNamesById = computed(() => {
  const out = new Map<string, string>()
  props.agents.forEach((agent) => out.set(agent.id, agent.name))
  return out
})

const taskTitlesById = computed(() => {
  const out = new Map<string, string>()
  props.tasks.forEach((task) => out.set(task.id, task.title))
  return out
})

const evidenceMemories = computed(() =>
  props.memories.filter((memory) =>
    ['evidence', 'risk', 'blocker', 'artifact_summary'].includes(memory.kind),
  ),
)

const acceptedMemoryCount = computed(() =>
  props.memories.filter((memory) => memory.accepted_by_commander).length,
)

const promotedMemoryCount = computed(() =>
  props.memories.filter((memory) => memory.promoted_to_long_term).length,
)

const observability = computed(() =>
  deriveTeamV4Observability({
    agents: props.agents,
    events: props.events,
    harnessRuns: props.harnessRuns,
    runState: props.run?.state,
    tasks: props.tasks,
  }),
)

const runStateBadgeClass = computed(() => {
  const state = String(props.run?.state || '').toLowerCase()
  if (state === 'completed') return 'badge-success'
  if (state === 'failed' || state === 'cancelled') return 'badge-error'
  if (state === 'running' || state === 'planning') return 'badge-info'
  if (state === 'waiting_human') return 'badge-warning'
  return 'badge-ghost'
})

const agentName = (agentId?: string | null) =>
  agentId ? agentNamesById.value.get(agentId) || agentId : 'unassigned'

const taskTitle = (taskId?: string | null) =>
  taskId ? taskTitlesById.value.get(taskId) || taskId : 'run-level'

const agentBadgeClass = (status: string) => {
  const normalized = status.toLowerCase()
  if (normalized === 'completed') return 'badge-success'
  if (normalized === 'failed' || normalized === 'cancelled') return 'badge-error'
  if (normalized === 'running' || normalized === 'waiting') return 'badge-info'
  return 'badge-ghost'
}

const phaseBadgeClass = (phase: string) => {
  if (phase === 'completed') return 'badge-success'
  if (phase === 'failed' || phase === 'cancelled') return 'badge-error'
  if (phase === 'waiting_user' || phase === 'recovering') return 'badge-warning'
  if (phase === 'tool_running' || phase === 'solver_running' || phase === 'scheduling') return 'badge-info'
  return 'badge-ghost'
}

const severityBadgeClass = (severity: string) => {
  if (severity === 'success') return 'badge-success'
  if (severity === 'warning') return 'badge-warning'
  if (severity === 'error') return 'badge-error'
  return 'badge-info'
}

const agentRuntimeStatus = (agent: TeamV4Agent) => {
  const latestAgentEvent = [...props.events]
    .reverse()
    .find((event) => event.actor_id === agent.id)
  const eventType = String(latestAgentEvent?.event_type || '').toLowerCase()
  if (eventType.startsWith('solver_execution_')) {
    if (eventType.endsWith('_failed')) return 'failed'
    if (eventType.endsWith('_completed')) return 'completed'
    if (eventType.endsWith('_started') || eventType.endsWith('_heartbeat')) return 'running'
  }
  return agent.status
}

const taskBadgeClass = (status: string) => {
  const normalized = status.toLowerCase()
  if (normalized === 'completed') return 'badge-success'
  if (normalized === 'failed' || normalized === 'cancelled') return 'badge-error'
  if (normalized === 'blocked') return 'badge-warning'
  if (normalized === 'running' || normalized === 'ready') return 'badge-info'
  return 'badge-ghost'
}

const eventVisibilityBadgeClass = (visibility: string) => {
  if (visibility === 'user') return 'badge-primary'
  if (visibility === 'internal') return 'badge-ghost'
  return 'badge-secondary'
}

const memoryKindBadgeClass = (kind: string) => {
  if (kind === 'risk' || kind === 'blocker') return 'badge-warning'
  if (kind === 'evidence') return 'badge-success'
  if (kind === 'decision') return 'badge-info'
  return 'badge-primary'
}

const harnessBadgeClass = (harness: TeamV4HarnessRun) => {
  const status = harness.status.toLowerCase()
  if (status === 'completed') return 'badge-success'
  if (status === 'failed' || status === 'cancelled') return 'badge-error'
  const leaseTime = new Date(harness.lease_expires_at || '').getTime()
  if (Number.isFinite(leaseTime) && leaseTime < Date.now()) return 'badge-warning'
  if (status === 'running') return 'badge-info'
  return 'badge-ghost'
}

const confidenceLabel = (value: number) => `${Math.round(value * 100)}%`

const memorySummary = (memory: TeamV4Memory) =>
  typeof memory.metadata?.summary === 'string' ? memory.metadata.summary : ''

const memoryEvidence = (memory: TeamV4Memory) =>
  Array.isArray(memory.metadata?.evidence)
    ? memory.metadata.evidence.filter((item): item is string => typeof item === 'string').slice(0, 3)
    : []

const formatTimestamp = (value?: string | null) => {
  if (!value) return ''
  const time = new Date(value).getTime()
  if (!Number.isFinite(time)) return value
  return new Date(time).toLocaleString()
}

const formatPayload = (payload: unknown) => {
  try {
    return JSON.stringify(payload ?? {}, null, 2)
  } catch {
    return String(payload ?? '')
  }
}

const EmptyState = defineComponent({
  props: {
    label: {
      type: String,
      required: true,
    },
  },
  setup(componentProps) {
    return () =>
      h(
        'div',
        { class: 'rounded-lg border border-dashed border-base-300 p-4 text-sm text-base-content/50' },
        componentProps.label,
      )
  },
})

const StatTile = defineComponent({
  props: {
    label: {
      type: String,
      required: true,
    },
    value: {
      type: String,
      required: true,
    },
  },
  setup(componentProps) {
    return () =>
      h('div', { class: 'rounded-lg border border-base-300 bg-base-100 p-3' }, [
        h('div', { class: 'text-[11px] uppercase text-base-content/50' }, componentProps.label),
        h('div', { class: 'mt-1 text-lg font-semibold text-base-content' }, componentProps.value),
      ])
  },
})
</script>
