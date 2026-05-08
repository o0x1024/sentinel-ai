import { invoke } from '@tauri-apps/api/core'

export interface BotAccount {
  id: string
  transport: string
  account_id: string
  display_name: string | null
  status: string | null
  last_seen_at: string | null
  created_at: string
  updated_at: string
}

export interface BotPeer {
  id: string
  transport: string
  account_id: string
  peer_type: string
  peer_id: string
  display_name: string | null
  last_sender_id: string | null
  last_message_at: string | null
  last_inbound_message_at: string | null
  last_outbound_message_at: string | null
  message_count: number
  execution_run_count: number
  failed_execution_count: number
  running_execution_count: number
  enabled_schedule_count: number
  failed_schedule_run_count: number
  latest_execution_status: string | null
  latest_execution_started_at: string | null
  created_at: string
  updated_at: string
}

export interface BotMessage {
  id: string
  transport: string
  account_id: string
  peer_type: string
  peer_id: string
  sender_id: string
  direction: string
  content: string
  transport_message_id: string | null
  context_token: string | null
  conversation_id: string | null
  ai_message_id: string | null
  linked_execution_run_id: string | null
  metadata_json: string | null
  created_at: string
}

export interface BotExecutionRun {
  id: string
  transport: string
  account_id: string
  peer_type: string
  peer_id: string
  sender_id: string
  conversation_id: string
  ai_execution_id: string
  assistant_profile_id: string | null
  trigger_kind: string
  trigger_bot_message_id: string | null
  trigger_ai_message_id: string | null
  task_text: string
  status: string
  result_text: string | null
  error_message: string | null
  started_at: string
  completed_at: string | null
  created_at: string
  updated_at: string
}

export interface BotSchedule {
  id: string
  transport: string
  account_id: string
  peer_type: string
  peer_id: string
  sender_id: string
  assistant_profile_id: string | null
  source_text: string
  task_text: string
  cron_expr: string
  timezone: string
  enabled: boolean
  last_run_at: string | null
  next_run_at: string | null
  last_error: string | null
  created_at: string
  updated_at: string
}

export interface BotScheduleRun {
  id: string
  schedule_id: string
  transport: string
  account_id: string
  peer_type: string
  peer_id: string
  sender_id: string
  execution_run_id: string | null
  status: string
  result_text: string | null
  error_message: string | null
  triggered_at: string
  completed_at: string | null
  created_at: string
  updated_at: string
}

export interface AgentTaskHistoryItem {
  id: string
  execution_id: string
  item_index: number
  content: string
  status: string
  result: string | null
  created_at_ms: number
  updated_at_ms: number
}

export interface AgentHarnessEvent {
  id: string
  run_id: string
  conversation_id: string
  generation: number
  event_type: string
  payload: unknown | null
  created_at: string
}

export interface AgentHarnessCheckpoint {
  id: string
  run_id: string
  checkpoint_type: string
  payload: unknown | null
  created_at: string
}

export async function listBotAccounts(transport?: string | null): Promise<BotAccount[]> {
  return await invoke<BotAccount[]>('list_bot_accounts', { transport: transport ?? null })
}

export async function listBotPeers(params: {
  transport?: string | null
  accountId?: string | null
  peerType?: string | null
  limit?: number
}): Promise<BotPeer[]> {
  return await invoke<BotPeer[]>('list_bot_peers', {
    transport: params.transport ?? null,
    account_id: params.accountId ?? null,
    peer_type: params.peerType ?? null,
    limit: params.limit ?? null,
  })
}

export async function listBotMessagesForPeer(params: {
  transport: string
  accountId: string
  peerType: string
  peerId: string
  limit?: number
}): Promise<BotMessage[]> {
  return await invoke<BotMessage[]>('list_bot_messages_for_peer', {
    transport: params.transport,
    account_id: params.accountId,
    peer_type: params.peerType,
    peer_id: params.peerId,
    limit: params.limit ?? null,
  })
}

export async function listBotExecutionRunsForPeer(params: {
  transport: string
  accountId: string
  peerType: string
  peerId: string
  limit?: number
}): Promise<BotExecutionRun[]> {
  return await invoke<BotExecutionRun[]>('list_bot_execution_runs_for_peer', {
    transport: params.transport,
    account_id: params.accountId,
    peer_type: params.peerType,
    peer_id: params.peerId,
    limit: params.limit ?? null,
  })
}

export async function listBotSchedules(params: {
  transport: string
  accountId: string
  peerType?: string | null
  peerId?: string | null
}): Promise<BotSchedule[]> {
  return await invoke<BotSchedule[]>('list_bot_schedules', {
    transport: params.transport,
    account_id: params.accountId,
    peer_type: params.peerType ?? null,
    peer_id: params.peerId ?? null,
  })
}

export async function listBotScheduleRuns(
  scheduleId: string,
  limit?: number,
): Promise<BotScheduleRun[]> {
  return await invoke<BotScheduleRun[]>('list_bot_schedule_runs', {
    schedule_id: scheduleId,
    limit: limit ?? null,
  })
}

export async function getBotExecutionTasks(executionId: string): Promise<AgentTaskHistoryItem[]> {
  return await invoke<AgentTaskHistoryItem[]>('get_agent_tasks', { executionId, execution_id: executionId })
}

export async function getBotHarnessEvents(runId: string): Promise<AgentHarnessEvent[]> {
  return await invoke<AgentHarnessEvent[]>('get_agent_harness_events', { runId, run_id: runId })
}

export async function getBotHarnessCheckpoints(runId: string): Promise<AgentHarnessCheckpoint[]> {
  return await invoke<AgentHarnessCheckpoint[]>('get_agent_harness_checkpoints', { runId, run_id: runId })
}
