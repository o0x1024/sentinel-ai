import { invoke } from '@tauri-apps/api/core'
import type {
  TeamV4Agent,
  TeamV4AppendEventRequest,
  TeamV4ContextSnapshot,
  TeamV4CreateContextSnapshotRequest,
  TeamV4CreateMemoryRequest,
  TeamV4CreateTaskRequest,
  TeamV4Event,
  TeamV4HarnessRun,
  TeamV4Memory,
  TeamV4Run,
  TeamV4RunBootstrap,
  TeamV4StartAssistantRunRequest,
  TeamV4StartHarnessRequest,
  TeamV4Task,
} from '@/types/teamRuntime'

export const teamRuntimeApi = {
  ensureSchema(): Promise<void> {
    return invoke('team_v4_ensure_schema')
  },

  startAssistantRun(request: TeamV4StartAssistantRunRequest): Promise<TeamV4RunBootstrap> {
    return invoke('team_v4_start_assistant_run', { request })
  },

  getRun(runId: string): Promise<TeamV4Run | null> {
    return invoke('team_v4_get_run', { runId })
  },

  listRuns(conversationId?: string | null, limit = 20): Promise<TeamV4Run[]> {
    return invoke('team_v4_list_runs', {
      conversationId: conversationId || null,
      limit,
    })
  },

  listAgents(runId: string): Promise<TeamV4Agent[]> {
    return invoke('team_v4_list_agents', { runId })
  },

  listTasks(runId: string): Promise<TeamV4Task[]> {
    return invoke('team_v4_list_tasks', { runId })
  },

  createTask(runId: string, request: TeamV4CreateTaskRequest): Promise<TeamV4Task> {
    return invoke('team_v4_create_task', { runId, request })
  },

  listEvents(runId: string, afterSequence = 0, limit = 100): Promise<TeamV4Event[]> {
    return invoke('team_v4_list_events', {
      runId,
      afterSequence,
      limit,
    })
  },

  listMemories(runId: string): Promise<TeamV4Memory[]> {
    return invoke('team_v4_list_memories', { runId })
  },

  listHarnessRuns(runId: string): Promise<TeamV4HarnessRun[]> {
    return invoke('team_v4_list_harness_runs', { runId })
  },

  appendEvent(runId: string, request: TeamV4AppendEventRequest): Promise<TeamV4Event> {
    return invoke('team_v4_append_event', { runId, request })
  },

  createContextSnapshot(
    runId: string,
    request: TeamV4CreateContextSnapshotRequest,
  ): Promise<TeamV4ContextSnapshot> {
    return invoke('team_v4_create_context_snapshot', { runId, request })
  },

  updateRunState(runId: string, state: string): Promise<void> {
    return invoke('team_v4_update_run_state', { runId, state })
  },

  checkpointHarnessRun(harnessRunId: string, checkpointSequence: number): Promise<void> {
    return invoke('team_v4_checkpoint_harness_run', {
      harnessRunId,
      checkpointSequence,
    })
  },

  startHarnessRun(runId: string, request: TeamV4StartHarnessRequest): Promise<TeamV4HarnessRun> {
    return invoke('team_v4_start_harness_run', { runId, request })
  },

  heartbeatHarnessRun(harnessRunId: string, leaseSecs = 600): Promise<void> {
    return invoke('team_v4_heartbeat_harness_run', {
      harnessRunId,
      leaseSecs,
    })
  },

  cancelHarnessRun(harnessRunId: string): Promise<TeamV4HarnessRun> {
    return invoke('team_v4_cancel_harness_run', { harnessRunId })
  },

  resumeHarnessRun(harnessRunId: string, leaseSecs = 600): Promise<TeamV4HarnessRun> {
    return invoke('team_v4_resume_harness_run', {
      harnessRunId,
      leaseSecs,
    })
  },

  createMemoryCandidate(runId: string, request: TeamV4CreateMemoryRequest): Promise<TeamV4Memory> {
    return invoke('team_v4_create_memory_candidate', { runId, request })
  },

  acceptMemory(
    runId: string,
    memoryId: string,
    promotedToLongTerm = false,
  ): Promise<TeamV4Memory> {
    return invoke('team_v4_accept_memory', {
      runId,
      memoryId,
      promotedToLongTerm,
    })
  },
}
