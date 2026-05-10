export interface TeamV4Run {
  id: string
  conversation_id?: string | null
  profile_id?: string | null
  goal: string
  state: string
  policy_json: Record<string, any>
  created_at: string
  updated_at: string
}

export interface TeamV4Agent {
  id: string
  run_id: string
  profile_id?: string | null
  role_type: 'orchestrator' | 'specialist' | 'monitor' | 'harness'
  name: string
  status: string
  model?: string | null
  context_mode?: string | null
  tool_policy_json: Record<string, any>
  metadata: Record<string, any>
  created_at: string
  updated_at: string
}

export interface TeamV4Task {
  id: string
  run_id: string
  parent_task_id?: string | null
  task_key: string
  title: string
  instruction: string
  status: string
  priority: number
  assigned_agent_id?: string | null
  depends_on: any[]
  acceptance_criteria?: string | null
  context_snapshot_id?: string | null
  metadata: Record<string, any>
  created_at: string
  updated_at: string
}

export interface TeamV4Event {
  id: string
  run_id: string
  sequence: number
  actor_id?: string | null
  task_id?: string | null
  event_type: string
  visibility: 'user' | 'workspace' | 'internal'
  payload: Record<string, any>
  created_at: string
}

export interface TeamV4ContextSnapshot {
  id: string
  run_id: string
  actor_id?: string | null
  task_id?: string | null
  role_type: string
  source_sequence: number
  policy_json: Record<string, any>
  sections_json: any[]
  token_estimate: number
  created_at: string
}

export interface TeamV4HarnessRun {
  id: string
  run_id: string
  actor_id?: string | null
  task_id?: string | null
  status: string
  lease_expires_at?: string | null
  last_heartbeat_at?: string | null
  checkpoint_sequence: number
  metadata: Record<string, any>
  created_at: string
  updated_at: string
}

export interface TeamV4SpecialistAssignment {
  specialist: TeamV4Agent
  task: TeamV4Task
  contextSnapshot: TeamV4ContextSnapshot
  harnessRun: TeamV4HarnessRun
}

export interface TeamV4Memory {
  id: string
  run_id: string
  task_id?: string | null
  kind: 'evidence' | 'decision' | 'risk' | 'blocker' | 'checkpoint' | 'artifact_summary'
  content: string
  confidence: number
  source_event_ids: string[]
  accepted_by_orchestrator: boolean
  promoted_to_long_term: boolean
  metadata: Record<string, any>
  created_at: string
  updated_at: string
}

export interface TeamV4RunBootstrap {
  run: TeamV4Run
  orchestrator: TeamV4Agent
  monitor: TeamV4Agent
  specialist: TeamV4Agent
  specialists: TeamV4Agent[]
  specialistAssignments: TeamV4SpecialistAssignment[]
  rootTask: TeamV4Task
  contextSnapshot: TeamV4ContextSnapshot
  harnessRun: TeamV4HarnessRun
  events: TeamV4Event[]
}

export interface TeamV4StartAssistantRunRequest {
  conversationId?: string | null
  profileId?: string | null
  teamProfileId?: string | null
  orchestratorProfileId?: string | null
  specialistProfileIds?: string[] | null
  monitorProfileId?: string | null
  goal: string
  model?: string | null
  contextMode?: string | null
  toolPolicyMatrix?: Record<string, any> | null
  memoryPolicy?: Record<string, any> | null
  harnessPolicy?: Record<string, any> | null
  concurrencyPolicy?: Record<string, any> | null
  safetyPolicy?: Record<string, any> | null
}

export interface TeamV4AppendEventRequest {
  actorId?: string | null
  taskId?: string | null
  eventType: string
  visibility?: 'user' | 'workspace' | 'internal' | null
  payload?: Record<string, any> | null
}

export interface TeamV4CreateMemoryRequest {
  taskId?: string | null
  kind: TeamV4Memory['kind']
  content: string
  confidence?: number | null
  sourceEventIds?: string[] | null
  metadata?: Record<string, any> | null
}

export interface TeamV4CreateContextSnapshotRequest {
  actorId?: string | null
  taskId?: string | null
  roleType: string
  sourceSequence?: number | null
  policyJson?: Record<string, any> | null
  sectionsJson: any[]
  tokenEstimate?: number | null
}

export interface TeamV4CreateTaskRequest {
  parentTaskId?: string | null
  taskKey: string
  title: string
  instruction: string
  priority?: number | null
  assignedAgentId?: string | null
  dependsOn?: any[] | null
  acceptanceCriteria?: string | null
  metadata?: Record<string, any> | null
}

export interface TeamV4StartHarnessRequest {
  actorId?: string | null
  taskId?: string | null
  leaseSecs?: number | null
  metadata?: Record<string, any> | null
}

export interface TeamV4FinishHarnessRequest {
  status: 'completed' | 'failed' | 'cancelled'
  checkpointSequence?: number | null
  error?: string | null
  payload?: Record<string, any> | null
}

export interface TeamV4ConversationHarnessSnapshot {
  agents: TeamV4Agent[]
  events: TeamV4Event[]
  harnessRuns: TeamV4HarnessRun[]
  tasks: TeamV4Task[]
}
