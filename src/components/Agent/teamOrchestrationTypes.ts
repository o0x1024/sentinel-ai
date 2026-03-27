export type TeamOrchestrationStepType = 'agent' | 'serial' | 'parallel'

export interface TeamOrchestrationRetry {
  max_attempts?: number
  backoff_ms?: number
}

export interface TeamOrchestrationStep {
  id: string
  type: TeamOrchestrationStepType
  name?: string
  member?: string
  phase?: string
  instruction?: string
  retry?: TeamOrchestrationRetry
  children?: TeamOrchestrationStep[]
}

export interface TeamOrchestrationPlan {
  version: number
  steps: TeamOrchestrationStep[]
}

export interface TeamStepMovePayload {
  sourcePath: number[]
  targetPath: number[]
  mode: 'before' | 'inside'
}

export interface TeamRuntimeStepStat {
  step_id: string
  total_attempts: number
  success_count: number
  failure_count: number
  avg_duration_ms: number
  last_duration_ms?: number
  last_status?: string
  last_error?: string
}

export interface TeamRuntimeFailureMode {
  mode: string
  count: number
  latest_step_id?: string
  latest_error?: string
  hint?: string
}

export type TeamOrchestrationPresetId =
  | 'product_delivery_chain'
  | 'incident_response_flow'

export type TeamRecoveryPresetId = 'conservative' | 'balanced' | 'aggressive'

export interface TeamOrchestrationPresetMeta {
  id: TeamOrchestrationPresetId
  label: string
  description: string
}

export interface TeamRecoveryPreset {
  id: TeamRecoveryPresetId
  label: string
  description: string
  max_attempts: number
  backoff_ms: number
  human_intervention_timeout_secs: number
  max_human_interventions: number
  no_human_input_policy: TeamRecoveryPresetId
}
