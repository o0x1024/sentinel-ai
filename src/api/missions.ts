import { invoke } from '@tauri-apps/api/core'

export interface Mission {
  id: string
  title: string
  objective: string
  status: string
  owner_kind: string
  owner_ref: string
  source_json: string | null
  delivery_policy_json: string | null
  assistant_profile_id: string | null
  trigger_json: string | null
  step_plan_json: string | null
  success_criteria_json: string | null
  context_strategy_json: string | null
  budget_json: string | null
  failure_policy_json: string | null
  missed_run_policy: string
  next_run_at: string | null
  last_run_at: string | null
  last_error: string | null
  run_count: number
  created_at: string
  updated_at: string
}

export interface MissionRun {
  id: string
  mission_id: string
  run_index: number
  status: string
  trigger_kind: string
  started_at: string | null
  completed_at: string | null
  agent_execution_id: string | null
  bot_execution_run_id: string | null
  assistant_profile_snapshot_json: string | null
  tool_config_snapshot_json: string | null
  checkpoint_json: string | null
  context_injected_json: string | null
  result_summary: string | null
  error_message: string | null
  created_at: string
  updated_at: string
}

export interface CreateMissionRequest {
  title: string
  objective: string
  ownerKind: string
  ownerRef: string
  sourceJson?: string | null
  deliveryPolicyJson?: string | null
  assistantProfileId?: string | null
  triggerJson?: string | null
  stepPlanJson?: string | null
  successCriteriaJson?: string | null
  contextStrategyJson?: string | null
  budgetJson?: string | null
  failurePolicyJson?: string | null
  missedRunPolicy?: string
  nextRunAt?: string | null
}

export interface UpdateMissionFieldsRequest {
  id: string
  title?: string | null
  objective?: string | null
  triggerJson?: string | null
  deliveryPolicyJson?: string | null
  assistantProfileId?: string | null
  stepPlanJson?: string | null
  successCriteriaJson?: string | null
  contextStrategyJson?: string | null
  budgetJson?: string | null
  failurePolicyJson?: string | null
  missedRunPolicy?: string | null
  nextRunAt?: string | null
}

export interface ListMissionsFilter {
  ownerKind?: string | null
  ownerRef?: string | null
  status?: string | null
  limit?: number
  offset?: number
}

export async function createMission(request: CreateMissionRequest): Promise<Mission> {
  return await invoke<Mission>('mission_create', { request })
}

export async function listMissions(filter: ListMissionsFilter): Promise<Mission[]> {
  return await invoke<Mission[]>('mission_list', { filter })
}

export async function getMission(id: string): Promise<Mission | null> {
  return await invoke<Mission | null>('mission_get', { id })
}

export async function updateMissionFields(
  request: UpdateMissionFieldsRequest,
): Promise<Mission> {
  return await invoke<Mission>('mission_update_fields', { request })
}

export async function pauseMission(id: string): Promise<Mission> {
  return await invoke<Mission>('mission_pause', { id })
}

export async function resumeMission(id: string): Promise<Mission> {
  return await invoke<Mission>('mission_resume', { id })
}

export async function archiveMission(id: string): Promise<Mission> {
  return await invoke<Mission>('mission_archive', { id })
}

export async function activateMission(id: string): Promise<Mission> {
  return await invoke<Mission>('mission_activate', { id })
}

export async function deleteMission(id: string): Promise<void> {
  return await invoke<void>('mission_delete', { id })
}

export async function runMissionNow(id: string): Promise<MissionRun> {
  return await invoke<MissionRun>('mission_run_now', { id })
}

export async function listMissionRuns(
  missionId: string,
  limit?: number,
  offset?: number,
): Promise<MissionRun[]> {
  return await invoke<MissionRun[]>('mission_list_runs', {
    missionId,
    limit: limit ?? null,
    offset: offset ?? null,
  })
}

export async function getMissionRun(runId: string): Promise<MissionRun | null> {
  return await invoke<MissionRun | null>('mission_get_run', { runId })
}
