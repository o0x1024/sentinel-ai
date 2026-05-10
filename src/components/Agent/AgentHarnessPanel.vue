<template>
  <div class="flex h-full flex-col bg-transparent">
    <div class="flex items-center justify-between border-b border-base-300 px-4 py-3">
      <div class="min-w-0">
        <div class="text-sm font-semibold text-base-content">Harness</div>
        <div class="mt-0.5 text-xs text-base-content/55">
          {{ totalHarnessRunCount }} run{{ totalHarnessRunCount === 1 ? '' : 's' }}
        </div>
      </div>
      <div class="flex items-center gap-2">
        <button class="btn btn-xs btn-ghost" :disabled="loading" title="刷新" @click="loadRuns">
          <i class="fas fa-rotate-right" :class="{ 'animate-spin': loading }"></i>
        </button>
        <button class="btn btn-xs btn-ghost" title="关闭" @click="emit('close')">
          <i class="fas fa-times"></i>
        </button>
      </div>
    </div>

    <div v-if="!conversationId" class="p-4 text-sm text-base-content/55">
      当前没有活动会话。
    </div>

    <div v-else class="flex-1 overflow-y-auto p-4">
      <div v-if="error" class="mb-3 rounded border border-error/30 bg-error/10 px-3 py-2 text-xs text-error">
        {{ error }}
      </div>

      <div v-if="totalHarnessRunCount === 0 && !loading" class="text-sm text-base-content/55">
        当前会话还没有 Harness run。
      </div>

      <div class="space-y-3">
        <div v-if="teamHarnessRunsForDisplay.length > 0" class="rounded-lg border border-base-300 bg-base-100 p-3">
          <div class="mb-2 flex items-center justify-between gap-2">
            <div class="text-xs font-semibold uppercase tracking-wide text-base-content/55">
              Team Runtime
            </div>
            <span class="text-[11px] text-base-content/45">{{ teamHarnessRunsForDisplay.length }} run{{ teamHarnessRunsForDisplay.length === 1 ? '' : 's' }}</span>
          </div>
          <div class="space-y-2">
            <div
              v-for="teamRun in teamHarnessRunsForDisplay"
              :key="teamRun.id"
              class="rounded border border-base-300/70 bg-base-200/35 px-2 py-2 text-xs"
            >
              <div class="flex items-start justify-between gap-2">
                <div class="min-w-0">
                  <div class="truncate font-medium text-base-content">
                    {{ teamTaskTitle(teamRun.task_id) }}
                  </div>
                  <div class="mt-0.5 truncate text-[11px] text-base-content/50">
                    {{ teamAgentName(teamRun.actor_id) }} · checkpoint #{{ teamRun.checkpoint_sequence }}
                  </div>
                </div>
                <span class="badge badge-xs shrink-0" :class="teamHarnessBadgeClass(teamRun.status)">
                  {{ teamRun.status }}
                </span>
              </div>
              <div class="mt-2 grid grid-cols-2 gap-1 text-[11px] text-base-content/60">
                <div>task: {{ teamTaskStatus(teamRun.task_id) || '-' }}</div>
                <div>updated: {{ formatTime(teamRun.updated_at) || '-' }}</div>
                <div>heartbeat: {{ formatTime(teamRun.last_heartbeat_at) || '-' }}</div>
                <div>lease: {{ formatTime(teamRun.lease_expires_at) || 'none' }}</div>
              </div>
              <div v-if="teamHarnessEventDetail(teamRun.id)" class="mt-2 rounded bg-base-100 px-2 py-1 text-[11px] text-base-content/60">
                {{ teamHarnessEventDetail(teamRun.id) }}
              </div>
              <div class="mt-2 flex flex-wrap gap-2">
                <button
                  class="btn btn-xs btn-outline btn-warning"
                  :disabled="!teamHarnessCanResume(teamRun.status)"
                  @click.stop="resumeTeamHarness(teamRun.id)"
                >
                  Resume
                </button>
                <button
                  class="btn btn-xs btn-outline btn-error"
                  :disabled="!teamHarnessCanCancel(teamRun.status)"
                  @click.stop="cancelTeamHarness(teamRun.id)"
                >
                  Cancel
                </button>
              </div>
            </div>
          </div>
        </div>

        <button
          v-for="run in runsForDisplay"
          :key="run.id"
          class="w-full rounded-lg border p-3 text-left transition-colors"
          :class="selectedRunId === run.id ? 'border-primary bg-primary/5' : 'border-base-300 bg-base-100 hover:border-base-content/25'"
          @click="selectRun(run.id)"
        >
          <div class="flex items-start justify-between gap-3">
            <div class="min-w-0">
              <div class="truncate text-sm font-medium text-base-content">
                {{ harnessMode(run) }} · generation #{{ run.generation }}
              </div>
              <div class="mt-1 line-clamp-2 text-xs text-base-content/60">
                {{ run.task }}
              </div>
            </div>
            <span class="badge badge-sm shrink-0" :class="statusBadgeClass(run.state)">
              {{ run.state }}
            </span>
          </div>
          <div class="mt-2 grid grid-cols-1 gap-1 text-[11px] text-base-content/50">
            <div>checkpoint: #{{ run.checkpoint_sequence }}</div>
            <div>heartbeat: {{ formatTime(run.last_heartbeat_at) }}</div>
            <div>lease: {{ formatTime(run.lease_expires_at) || 'none' }}</div>
          </div>
          <div v-if="run.error" class="mt-2 rounded bg-error/10 px-2 py-1 text-[11px] text-error">
            {{ run.error }}
          </div>
        </button>
        <button
          v-if="runs.length > 0 && (hiddenRunCount > 0 || showAllRuns)"
          class="btn btn-sm btn-ghost w-full justify-center text-base-content/60"
          @click="toggleRunHistory"
        >
          {{ showAllRuns ? '折叠历史 generation' : `展开历史 generation（${hiddenRunCount}）` }}
        </button>
      </div>

      <div v-if="selectedRun" class="mt-4 space-y-3">
        <div v-if="contextTimelineEvents.length > 0" class="rounded-lg border border-l-4 border-sky-300/70 border-l-sky-500 bg-sky-500/5 p-3">
          <div class="mb-2 flex items-center justify-between gap-2">
            <div class="text-xs font-semibold uppercase tracking-wide text-base-content/55">
              Context
            </div>
            <div class="flex items-center gap-2">
              <span class="text-[11px] text-base-content/45">
                {{ contextEventsToRender.length }}/{{ contextTimelineEvents.length }}
              </span>
              <button
                v-if="hiddenContextEventCount > 0 || showAllContextEvents"
                class="btn btn-xs btn-ghost text-base-content/55"
                @click="showAllContextEvents = !showAllContextEvents"
              >
                {{ showAllContextEvents ? '折叠' : '展开' }}
              </button>
            </div>
          </div>
          <div class="space-y-2">
            <div
              v-for="event in contextEventsToRender"
              :key="event.id"
              class="rounded border border-base-300/70 bg-base-200/40 px-2 py-2 text-xs"
            >
              <div class="flex items-start justify-between gap-2">
                <div class="min-w-0">
                  <div class="font-medium text-base-content">
                    {{ contextEventTitle(event) }}
                  </div>
                  <div class="mt-0.5 truncate text-[11px] text-base-content/50">
                    {{ contextEventPhase(event) }} · {{ formatTime(event.created_at) }}
                  </div>
                </div>
                <span class="badge badge-xs shrink-0" :class="contextPressureBadgeClass(contextEventPressure(event))">
                  {{ contextEventPressure(event) || 'unknown' }}
                </span>
              </div>
              <div class="mt-2 grid grid-cols-2 gap-1 text-[11px] text-base-content/65">
                <div>used: {{ formatTokenValue(contextEventNumber(event, 'used_tokens')) }}</div>
                <div>remaining: {{ formatTokenValue(contextEventNumber(event, 'remaining_tokens')) }}</div>
                <div>auto: {{ formatTokenValue(contextEventNumber(event, 'auto_compact_threshold_tokens')) }}</div>
                <div>block: {{ formatTokenValue(contextEventNumber(event, 'blocking_threshold_tokens')) }}</div>
              </div>
              <details class="mt-2">
                <summary class="cursor-pointer text-[11px] text-base-content/45 hover:text-base-content/70">
                  payload
                </summary>
                <pre class="mt-1 max-h-28 overflow-auto rounded bg-base-200 p-2 text-[11px] text-base-content/70">{{ compactJson(event.payload) }}</pre>
              </details>
            </div>
          </div>
        </div>

        <div v-if="continuationTimelineItems.length > 0" class="rounded-lg border border-base-300 bg-base-100 p-3">
          <div class="mb-2 flex items-center justify-between gap-2">
            <div class="text-xs font-semibold uppercase tracking-wide text-base-content/55">
              Continuations
            </div>
            <span class="text-[11px] text-base-content/45">{{ continuationTimelineItems.length }} item{{ continuationTimelineItems.length === 1 ? '' : 's' }}</span>
          </div>
          <div class="space-y-2">
            <div
              v-for="item in continuationTimelineItems"
              :key="item.id"
              class="rounded border border-base-300/70 bg-base-200/40 px-2 py-2 text-xs"
            >
              <div class="flex items-start justify-between gap-2">
                <div class="min-w-0">
                  <div class="font-medium text-base-content">{{ item.title }}</div>
                  <div class="mt-0.5 truncate text-[11px] text-base-content/50">
                    {{ item.detail }} · {{ formatTime(item.createdAt) }}
                  </div>
                </div>
                <span class="badge badge-xs shrink-0" :class="continuationBadgeClass(item)">
                  {{ item.badge }}
                </span>
              </div>
              <div class="mt-2 grid grid-cols-2 gap-1 text-[11px] text-base-content/65">
                <div>source: {{ item.source || '-' }}</div>
                <div>reason: {{ item.reason || '-' }}</div>
                <div>kind: {{ item.continuationKind || '-' }}</div>
                <div>ledger unchanged: {{ item.ledgerNoProgressCount ?? '-' }}</div>
                <div>unfinished: {{ item.unfinished ?? '-' }}</div>
                <div>failed: {{ item.failed ?? '-' }}</div>
                <div>tools: {{ item.toolCallCount ?? '-' }}/{{ item.toolResultCount ?? '-' }}</div>
                <div>final: {{ item.finalAfterToolResult == null ? '-' : (item.finalAfterToolResult ? 'yes' : 'no') }}</div>
              </div>
              <div v-if="item.recentTools.length > 0" class="mt-2 space-y-1">
                <div
                  v-for="tool in item.recentTools"
                  :key="tool.id"
                  class="truncate rounded bg-base-100 px-2 py-1 text-[11px] text-base-content/60"
                  :title="tool.resultExcerpt || tool.name"
                >
                  {{ tool.name }} · {{ tool.success ? 'success' : 'failed' }} · result {{ tool.hasResult ? 'present' : 'missing' }}
                </div>
              </div>
            </div>
          </div>
        </div>

        <div class="rounded-lg border border-l-4 border-violet-300/70 border-l-violet-500 bg-violet-500/5 p-3">
          <div class="mb-2 flex items-center justify-between gap-2">
            <div class="text-xs font-semibold uppercase tracking-wide text-base-content/55">
              Events
            </div>
            <div class="flex items-center gap-2">
              <span class="text-[11px] text-base-content/45">
                {{ eventsToRender.length }}/{{ eventsForCurrentFilter.length }}
              </span>
              <button
                v-if="hiddenEventCount > 0 || showAllEvents"
                class="btn btn-xs btn-ghost text-base-content/55"
                @click="showAllEvents = !showAllEvents"
              >
                {{ showAllEvents ? '折叠' : '展开' }}
              </button>
              <button
                class="btn btn-xs btn-ghost"
                :class="showContextEventsOnly ? 'text-primary' : 'text-base-content/55'"
                @click="toggleContextEventFilter"
              >
                context
              </button>
            </div>
          </div>
          <div v-if="eventsToRender.length === 0" class="text-xs text-base-content/50">No events.</div>
          <div v-else class="space-y-2">
            <div v-for="event in eventsToRender" :key="event.id" class="text-xs">
              <div class="flex items-center justify-between gap-2">
                <span class="font-medium text-base-content">{{ event.event_type }}</span>
                <span class="text-[11px] text-base-content/45">{{ formatTime(event.created_at) }}</span>
              </div>
              <pre v-if="event.payload" class="mt-1 max-h-24 overflow-auto rounded bg-base-200 p-2 text-[11px] text-base-content/70">{{ compactJson(event.payload) }}</pre>
            </div>
          </div>
        </div>

        <div class="rounded-lg border border-l-4 border-emerald-300/70 border-l-emerald-500 bg-emerald-500/5 p-3">
          <div class="mb-2 flex items-center justify-between gap-2">
            <div class="text-xs font-semibold uppercase tracking-wide text-base-content/55">
              Checkpoints
            </div>
            <div class="flex items-center gap-2">
              <span class="text-[11px] text-base-content/45">
                {{ checkpointsToRender.length }}/{{ checkpoints.length }}
              </span>
              <button
                v-if="hiddenCheckpointCount > 0 || showAllCheckpoints"
                class="btn btn-xs btn-ghost text-base-content/55"
                @click="showAllCheckpoints = !showAllCheckpoints"
              >
                {{ showAllCheckpoints ? '折叠' : '展开' }}
              </button>
            </div>
          </div>
          <div v-if="checkpointsToRender.length === 0" class="text-xs text-base-content/50">No checkpoints.</div>
          <div v-else class="space-y-2">
            <div v-for="checkpoint in checkpointsToRender" :key="checkpoint.id" class="text-xs">
              <div class="flex items-center justify-between gap-2">
                <span class="font-medium text-base-content">{{ checkpoint.checkpoint_type }}</span>
                <span class="text-[11px] text-base-content/45">{{ formatTime(checkpoint.created_at) }}</span>
              </div>
              <pre v-if="checkpoint.payload" class="mt-1 max-h-28 overflow-auto rounded bg-base-200 p-2 text-[11px] text-base-content/70">{{ compactJson(checkpoint.payload) }}</pre>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref, watch, onMounted, onUnmounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { teamRuntimeApi } from '@/api/teamRuntime'
import {
  harnessRunsForDisplay,
  hiddenHarnessRunCount,
  hiddenTimelineItemCount,
  resolveHarnessSelectedRunId,
  timelineItemsForDisplay,
} from './agentHarnessPanelSupport'
import type { TeamV4Agent, TeamV4Event, TeamV4HarnessRun, TeamV4Task } from '@/types/teamRuntime'

interface AgentHarnessRun {
  id: string
  conversation_id: string
  generation: number
  state: string
  task: string
  model?: string | null
  provider?: string | null
  error?: string | null
  metadata?: Record<string, unknown> | null
  started_at: string
  last_heartbeat_at: string
  lease_expires_at?: string | null
  checkpoint_sequence: number
  completed_at?: string | null
  created_at: string
  updated_at: string
}

interface AgentHarnessEvent {
  id: string
  run_id: string
  conversation_id: string
  generation: number
  event_type: string
  payload?: Record<string, unknown> | null
  created_at: string
}

interface AgentHarnessCheckpoint {
  id: string
  run_id: string
  checkpoint_type: string
  payload?: Record<string, unknown> | null
  created_at: string
}

const props = defineProps<{
  active: boolean
  conversationId?: string | null
  teamAgents?: TeamV4Agent[]
  teamEvents?: TeamV4Event[]
  teamHarnessRuns?: TeamV4HarnessRun[]
  teamTasks?: TeamV4Task[]
}>()

const emit = defineEmits<{
  close: []
}>()

const runs = ref<AgentHarnessRun[]>([])
const events = ref<AgentHarnessEvent[]>([])
const checkpoints = ref<AgentHarnessCheckpoint[]>([])
const snapshotTeamAgents = ref<TeamV4Agent[]>([])
const snapshotTeamEvents = ref<TeamV4Event[]>([])
const snapshotTeamHarnessRuns = ref<TeamV4HarnessRun[]>([])
const snapshotTeamTasks = ref<TeamV4Task[]>([])
const selectedRunId = ref<string | null>(null)
const manuallySelectedRun = ref(false)
const loading = ref(false)
const error = ref<string | null>(null)
const showContextEventsOnly = ref(false)
const showAllRuns = ref(false)
const showAllContextEvents = ref(false)
const showAllEvents = ref(false)
const showAllCheckpoints = ref(false)
let refreshTimer: number | undefined

const selectedRun = computed(() => runs.value.find((run) => run.id === selectedRunId.value) || null)
const effectiveTeamAgents = computed(() => snapshotTeamAgents.value.length ? snapshotTeamAgents.value : (props.teamAgents || []))
const effectiveTeamEvents = computed(() => snapshotTeamEvents.value.length ? snapshotTeamEvents.value : (props.teamEvents || []))
const effectiveTeamHarnessRuns = computed(() => snapshotTeamHarnessRuns.value.length ? snapshotTeamHarnessRuns.value : (props.teamHarnessRuns || []))
const effectiveTeamTasks = computed(() => snapshotTeamTasks.value.length ? snapshotTeamTasks.value : (props.teamTasks || []))
const teamHarnessRunsForDisplay = computed(() => [...effectiveTeamHarnessRuns.value]
  .sort((a, b) => new Date(b.updated_at || b.created_at).getTime() - new Date(a.updated_at || a.created_at).getTime())
)
const totalHarnessRunCount = computed(() => runs.value.length + teamHarnessRunsForDisplay.value.length)
const teamAgentsById = computed(() => new Map(effectiveTeamAgents.value.map((agent) => [agent.id, agent])))
const teamTasksById = computed(() => new Map(effectiveTeamTasks.value.map((task) => [task.id, task])))
const teamEventsByHarnessId = computed(() => {
  const map = new Map<string, TeamV4Event>()
  for (const event of effectiveTeamEvents.value) {
    const harnessRunId = String(event.payload?.harness_run_id || '').trim()
    if (!harnessRunId) continue
    const current = map.get(harnessRunId)
    if (!current || event.sequence >= current.sequence) map.set(harnessRunId, event)
  }
  return map
})
const runsForDisplay = computed(() => harnessRunsForDisplay(runs.value, showAllRuns.value))
const hiddenRunCount = computed(() => hiddenHarnessRunCount(runs.value, showAllRuns.value))
const contextTimelineEvents = computed(() => events.value
  .filter((event) => event.event_type === 'context_pressure' || event.event_type === 'context_compaction_requested')
)
const contextEventsToRender = computed(() => timelineItemsForDisplay(contextTimelineEvents.value, showAllContextEvents.value))
const hiddenContextEventCount = computed(() => hiddenTimelineItemCount(contextTimelineEvents.value, showAllContextEvents.value))
const eventsForCurrentFilter = computed(() => showContextEventsOnly.value ? contextTimelineEvents.value : events.value)
const eventsToRender = computed(() => timelineItemsForDisplay(eventsForCurrentFilter.value, showAllEvents.value))
const hiddenEventCount = computed(() => hiddenTimelineItemCount(eventsForCurrentFilter.value, showAllEvents.value))
const checkpointsToRender = computed(() => timelineItemsForDisplay(checkpoints.value, showAllCheckpoints.value))
const hiddenCheckpointCount = computed(() => hiddenTimelineItemCount(checkpoints.value, showAllCheckpoints.value))
const continuationTimelineItems = computed(() => {
  const checkpointItems = checkpoints.value
    .filter((checkpoint) => checkpoint.checkpoint_type === 'generation_continuing' || checkpoint.checkpoint_type === 'generation_incomplete')
    .map((checkpoint) => continuationTimelineItemFromPayload(
      `checkpoint:${checkpoint.id}`,
      checkpoint.checkpoint_type,
      checkpoint.payload,
      checkpoint.created_at,
    ))
  const eventItems = events.value
    .filter((event) => event.event_type === 'harness_continuation_scheduled')
    .map((event) => continuationTimelineItemFromPayload(
      `event:${event.id}`,
      event.event_type,
      event.payload,
      event.created_at,
    ))
  return [...checkpointItems, ...eventItems]
    .sort((a, b) => new Date(a.createdAt).getTime() - new Date(b.createdAt).getTime())
})

const loadRuns = async () => {
  const conversationId = String(props.conversationId || '').trim()
  if (!conversationId) {
    runs.value = []
    snapshotTeamAgents.value = []
    snapshotTeamEvents.value = []
    snapshotTeamHarnessRuns.value = []
    snapshotTeamTasks.value = []
    selectedRunId.value = null
    return
  }
  loading.value = true
  error.value = null
  try {
    const [nextRuns, teamSnapshot] = await Promise.all([
      invoke<AgentHarnessRun[]>('get_agent_harness_runs', { conversationId }),
      teamRuntimeApi.conversationHarnessSnapshot(conversationId),
    ])
    runs.value = nextRuns
    snapshotTeamAgents.value = teamSnapshot.agents
    snapshotTeamEvents.value = teamSnapshot.events
    snapshotTeamHarnessRuns.value = teamSnapshot.harnessRuns
    snapshotTeamTasks.value = teamSnapshot.tasks
    selectedRunId.value = resolveHarnessSelectedRunId(
      nextRuns,
      selectedRunId.value,
      showAllRuns.value,
      manuallySelectedRun.value,
    )
    await loadSelectedRunDetail()
  } catch (loadError: any) {
    error.value = loadError?.toString?.() || String(loadError)
  } finally {
    loading.value = false
  }
}

const loadSelectedRunDetail = async () => {
  const runId = selectedRunId.value
  if (!runId) {
    events.value = []
    checkpoints.value = []
    return
  }
  const [nextEvents, nextCheckpoints] = await Promise.all([
    invoke<AgentHarnessEvent[]>('get_agent_harness_events', { runId }),
    invoke<AgentHarnessCheckpoint[]>('get_agent_harness_checkpoints', { runId }),
  ])
  events.value = nextEvents
  checkpoints.value = nextCheckpoints
}

const selectRun = async (runId: string) => {
  selectedRunId.value = runId
  manuallySelectedRun.value = true
  resetTimelineExpansion()
  try {
    await loadSelectedRunDetail()
  } catch (loadError: any) {
    error.value = loadError?.toString?.() || String(loadError)
  }
}

const toggleRunHistory = async () => {
  showAllRuns.value = !showAllRuns.value
  selectedRunId.value = resolveHarnessSelectedRunId(
    runs.value,
    selectedRunId.value,
    showAllRuns.value,
    manuallySelectedRun.value,
  )
  try {
    await loadSelectedRunDetail()
  } catch (loadError: any) {
    error.value = loadError?.toString?.() || String(loadError)
  }
}

const resetTimelineExpansion = () => {
  showAllContextEvents.value = false
  showAllEvents.value = false
  showAllCheckpoints.value = false
}

const toggleContextEventFilter = () => {
  showContextEventsOnly.value = !showContextEventsOnly.value
  showAllEvents.value = false
}

const stopRefresh = () => {
  if (refreshTimer) {
    window.clearInterval(refreshTimer)
    refreshTimer = undefined
  }
}

const handleHarnessPruned = (event: Event) => {
  const detail = (event as CustomEvent<{ conversationId?: string | null }>).detail
  const prunedConversationId = String(detail?.conversationId || '').trim()
  const currentConversationId = String(props.conversationId || '').trim()
  if (!props.active || !currentConversationId || prunedConversationId !== currentConversationId) return
  void loadRuns()
}

watch(
  () => [props.active, props.conversationId] as const,
  ([active], previousValue) => {
    stopRefresh()
    if (!active) return
    if (previousValue?.[1] !== props.conversationId) {
      showAllRuns.value = false
      manuallySelectedRun.value = false
      resetTimelineExpansion()
    }
    void loadRuns()
    refreshTimer = window.setInterval(() => {
      void loadRuns()
    }, 5000)
  },
  { immediate: true },
)

onMounted(() => {
  window.addEventListener('agent-harness-pruned', handleHarnessPruned)
})

onUnmounted(stopRefresh)
onUnmounted(() => {
  window.removeEventListener('agent-harness-pruned', handleHarnessPruned)
})

const statusBadgeClass = (status: string) => {
  const normalized = String(status || '').toLowerCase()
  if (normalized === 'succeeded') return 'badge-success'
  if (normalized === 'failed') return 'badge-error'
  if (normalized === 'stalled_without_ledger_progress') return 'badge-error'
  if (normalized === 'cancelled') return 'badge-warning'
  if (normalized === 'incomplete') return 'badge-warning'
  if (normalized === 'max_continuations_reached') return 'badge-warning'
  return 'badge-info'
}

const harnessMode = (run: AgentHarnessRun) => {
  const mode = run.metadata?.harness_mode
  return typeof mode === 'string' && mode.trim() ? mode : 'unknown'
}

const teamAgentName = (agentId?: string | null) => {
  if (!agentId) return 'Team'
  return teamAgentsById.value.get(agentId)?.name || agentId
}

const teamTaskTitle = (taskId?: string | null) => {
  if (!taskId) return 'Team run'
  return teamTasksById.value.get(taskId)?.title || taskId
}

const teamTaskStatus = (taskId?: string | null) => {
  if (!taskId) return ''
  return teamTasksById.value.get(taskId)?.status || ''
}

const teamHarnessBadgeClass = (status: string) => {
  const normalized = String(status || '').toLowerCase()
  if (normalized === 'completed') return 'badge-success'
  if (normalized === 'failed' || normalized === 'cancelled') return 'badge-error'
  if (normalized === 'paused' || normalized === 'expired') return 'badge-warning'
  if (normalized === 'running') return 'badge-info'
  return 'badge-ghost'
}

const teamHarnessCanResume = (status: string) =>
  ['paused', 'expired', 'queued'].includes(String(status || '').toLowerCase())

const teamHarnessCanCancel = (status: string) =>
  ['queued', 'running', 'paused', 'expired'].includes(String(status || '').toLowerCase())

const reloadTeamHarnessSnapshot = async () => {
  const conversationId = String(props.conversationId || '').trim()
  if (!conversationId) return
  const teamSnapshot = await teamRuntimeApi.conversationHarnessSnapshot(conversationId)
  snapshotTeamAgents.value = teamSnapshot.agents
  snapshotTeamEvents.value = teamSnapshot.events
  snapshotTeamHarnessRuns.value = teamSnapshot.harnessRuns
  snapshotTeamTasks.value = teamSnapshot.tasks
}

const resumeTeamHarness = async (harnessRunId: string) => {
  try {
    await teamRuntimeApi.resumeHarnessRun(harnessRunId, 600)
    await reloadTeamHarnessSnapshot()
  } catch (resumeError: any) {
    error.value = resumeError?.toString?.() || String(resumeError)
  }
}

const cancelTeamHarness = async (harnessRunId: string) => {
  try {
    await teamRuntimeApi.finishHarnessRun(harnessRunId, {
      status: 'cancelled',
      error: 'Cancelled from Harness panel.',
    })
    await reloadTeamHarnessSnapshot()
  } catch (cancelError: any) {
    error.value = cancelError?.toString?.() || String(cancelError)
  }
}

const teamHarnessEventDetail = (harnessRunId: string) => {
  const event = teamEventsByHarnessId.value.get(harnessRunId)
  if (!event) return ''
  const error = event.payload?.error
  if (typeof error === 'string' && error.trim()) return error
  return event.event_type
}

const contextEventPayload = (event: AgentHarnessEvent): Record<string, unknown> => {
  return event.payload && typeof event.payload === 'object' ? event.payload : {}
}

const contextEventTitle = (event: AgentHarnessEvent) => {
  if (event.event_type === 'context_compaction_requested') return 'Compaction requested'
  return 'Pressure snapshot'
}

const contextEventPhase = (event: AgentHarnessEvent) => {
  const phase = contextEventPayload(event).phase
  return typeof phase === 'string' && phase.trim() ? phase : 'unknown phase'
}

const contextEventPressure = (event: AgentHarnessEvent) => {
  const pressure = contextEventPayload(event).context_pressure
  return typeof pressure === 'string' && pressure.trim() ? pressure : ''
}

const contextEventNumber = (event: AgentHarnessEvent, key: string) => {
  const raw = contextEventPayload(event)[key]
  const parsed = Number(raw)
  return Number.isFinite(parsed) ? parsed : null
}

const contextPressureBadgeClass = (pressure: string) => {
  if (pressure === 'Blocking') return 'badge-error'
  if (pressure === 'AutoCompact') return 'badge-warning'
  if (pressure === 'Warning') return 'badge-info'
  if (pressure === 'Low') return 'badge-success'
  return 'badge-ghost'
}

const continuationTimelineItemFromPayload = (
  id: string,
  type: string,
  payload: Record<string, unknown> | null | undefined,
  createdAt: string,
) => {
  const data = payload && typeof payload === 'object' ? payload : {}
  const continuation = Number(data.continuation ?? data.continuation_count)
  const maxContinuations = Number(data.max_continuations)
  const taskCounts = data.task_counts && typeof data.task_counts === 'object'
    ? data.task_counts as Record<string, unknown>
    : {}
  const stopReason = typeof data.stop_reason === 'string' ? data.stop_reason : ''
  const turnReason = typeof data.turn_incomplete_reason === 'string' ? data.turn_incomplete_reason : ''
  const reason = typeof data.interruption_reason === 'string' ? data.interruption_reason : (turnReason || stopReason)
  const source = typeof data.interruption_source === 'string' ? data.interruption_source : ''
  const continuationKind = typeof data.continuation_kind === 'string' ? data.continuation_kind : ''
  const ledgerWatchdog = data.ledger_watchdog && typeof data.ledger_watchdog === 'object'
    ? data.ledger_watchdog as Record<string, unknown>
    : {}
  const ledgerNoProgressCount = Number(ledgerWatchdog.no_progress_count)
  const toolProtocol = data.tool_protocol && typeof data.tool_protocol === 'object'
    ? data.tool_protocol as Record<string, unknown>
    : {}
  const recentTools = Array.isArray(data.recent_tools)
    ? data.recent_tools
      .filter((tool): tool is Record<string, unknown> => !!tool && typeof tool === 'object')
      .map((tool) => ({
        id: typeof tool.id === 'string' ? tool.id : `${id}:${String(tool.name || 'tool')}`,
        name: typeof tool.name === 'string' ? tool.name : 'tool',
        success: tool.success === true,
        hasResult: tool.has_result === true,
        resultExcerpt: typeof tool.result_excerpt === 'string' ? tool.result_excerpt : '',
      }))
    : []
  const toolCallCount = Number(toolProtocol.tool_call_count)
  const toolResultCount = Number(toolProtocol.tool_result_count)
  const finalAfterToolResult = typeof toolProtocol.final_assistant_after_last_tool_result === 'boolean'
    ? toolProtocol.final_assistant_after_last_tool_result
    : null
  const unfinished = Number(taskCounts.unfinished)
  const failed = Number(taskCounts.failed)
  const title = type === 'harness_continuation_scheduled'
    ? continuationKind === 'ledger_reconcile' ? 'Ledger reconcile scheduled' : 'Continuation scheduled'
    : type === 'generation_continuing'
      ? continuationKind === 'ledger_reconcile' ? 'Ledger reconcile checkpoint' : 'Generation continuing'
      : stopReason === 'stalled_without_ledger_progress'
        ? 'Ledger stalled'
        : 'Generation incomplete'
  const badge = stopReason === 'max_continuations_reached'
    ? 'max'
    : stopReason === 'stalled_without_ledger_progress' || reason === 'ledger_no_progress_after_reconcile'
      ? 'stalled'
      : continuationKind === 'ledger_reconcile'
        ? 'reconcile'
        : reason === 'llm_stopped_with_incomplete_tasks'
          ? 'llm'
          : type === 'generation_incomplete'
            ? 'incomplete'
            : 'scheduled'
  const detail = Number.isFinite(continuation) && Number.isFinite(maxContinuations)
    ? `${continuation}/${maxContinuations}`
    : stopReason || type
  return {
    id,
    type,
    title,
    detail,
    badge,
    source,
    reason,
    continuationKind,
    ledgerNoProgressCount: Number.isFinite(ledgerNoProgressCount) ? ledgerNoProgressCount : null,
    toolCallCount: Number.isFinite(toolCallCount) ? toolCallCount : null,
    toolResultCount: Number.isFinite(toolResultCount) ? toolResultCount : null,
    finalAfterToolResult,
    recentTools,
    unfinished: Number.isFinite(unfinished) ? unfinished : null,
    failed: Number.isFinite(failed) ? failed : null,
    createdAt,
  }
}

const continuationBadgeClass = (item: { type: string; reason?: string; badge: string }) => {
  if (item.badge === 'stalled') return 'badge-error'
  if (item.badge === 'reconcile') return 'badge-warning'
  if (item.type === 'generation_incomplete') return 'badge-warning'
  if (item.reason === 'llm_stopped_with_incomplete_tasks') return 'badge-info'
  return 'badge-ghost'
}

const formatTokenValue = (value: number | null) => {
  if (value == null) return '-'
  if (value >= 1_000_000) return `${(value / 1_000_000).toFixed(1)}M`
  if (value >= 1_000) return `${(value / 1_000).toFixed(1)}K`
  return String(value)
}

const formatTime = (value?: string | null) => {
  if (!value) return ''
  const time = new Date(value)
  if (Number.isNaN(time.getTime())) return value
  return time.toLocaleTimeString()
}

const compactJson = (value: unknown) => JSON.stringify(value, null, 2)
</script>
