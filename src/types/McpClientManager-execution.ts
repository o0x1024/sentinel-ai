export interface ExecutionPlan {
  id: string
  name: string
  description: string
  steps: ExecutionStep[]
  dependencies: string[]
  expectedDuration: number
  priority: 'low' | 'medium' | 'high'
  tags: string[]
}

export interface ExecutionStep {
  id: string
  name: string
  description: string
  status: 'pending' | 'running' | 'completed' | 'failed' | 'cancelled'
  type: 'tool' | 'llm' | 'workflow' | 'validation'
  dependencies: string[]
  expectedDuration: number
  progress: number
  startTime?: Date
  endTime?: Date
  input: any
  output?: any
  error?: string
  metadata: Record<string, any>
}

export interface FlowAdjustment {
  id: string
  type: 'add_step' | 'remove_step' | 'modify_step' | 'reorder_steps' | 'change_order' | 'add_condition' | 'modify_condition'
  targetStepId?: string
  target?: string
  newStep?: ExecutionStep
  modifications?: Partial<ExecutionStep>
  newOrder?: string[]
  data?: any
  reason: string
  timestamp: Date
  appliedBy?: string
}

// DetailedExecutionStatus类型定义

export const DetailedExecutionStatus = {
  IDLE: 'idle' as const,
  INITIALIZED: 'initialized' as const,
  ANALYZING_INTENT: 'analyzing_intent' as const,
  RETRIEVING_MEMORY: 'retrieving_memory' as const,
  GENERATING_PLAN: 'generating_plan' as const,
  PLANNING_STARTED: 'planning_started' as const,
  OPTIMIZING_PLAN: 'optimizing_plan' as const,
  VALIDATING_PLAN: 'validating_plan' as const,
  PLANNING_COMPLETED: 'planning_completed' as const,
  PLANNING_FAILED: 'planning_failed' as const,
  EXECUTION_STARTED: 'execution_started' as const,
  PREPARING_CONTEXT: 'preparing_context' as const,
  STEP_EXECUTING: 'step_executing' as const,
  STEP_COMPLETED: 'step_completed' as const,
  STEP_FAILED: 'step_failed' as const,
  TOOL_CALLING: 'tool_calling' as const,
  WAITING_TOOL_RESPONSE: 'waiting_tool_response' as const,
  PROCESSING_RESULT: 'processing_result' as const,
  MONITORING_STARTED: 'monitoring_started' as const,
  CHECKING_PROGRESS: 'checking_progress' as const,
  DETECTING_ANOMALY: 'detecting_anomaly' as const,
  EVALUATING_PERFORMANCE: 'evaluating_performance' as const,
  REPLAN_TRIGGERED: 'replan_triggered' as const,
  ANALYZING_FAILURE: 'analyzing_failure' as const,
  GENERATING_ALTERNATIVE: 'generating_alternative' as const,
  UPDATING_PLAN: 'updating_plan' as const,
  REPLAN_COMPLETED: 'replan_completed' as const,
  REPLAN_FAILED: 'replan_failed' as const,
  REQUIRES_INTERVENTION: 'requires_intervention' as const,
  WAITING_USER_INPUT: 'waiting_user_input' as const,
  PAUSING: 'pausing' as const,
  PAUSED: 'paused' as const,
  RESUMING: 'resuming' as const,
  COMPLETING: 'completing' as const,
  CANCELLING: 'cancelling' as const,
  COMPLETED: 'completed' as const,
  FAILED: 'failed' as const,
  CANCELLED: 'cancelled' as const,
}

export type ExecutionStatusType = typeof DetailedExecutionStatus[keyof typeof DetailedExecutionStatus]
export type DetailedExecutionStatus = ExecutionStatusType

export interface ExecutionContext {
  executionId: string
  sessionId: string
  currentStep?: ExecutionStep
  executionPlan: ExecutionPlan
  status: ExecutionStatusType
  startTime: Date
  endTime?: Date
  progress: number
  error?: string
  metadata: Record<string, any>
  interventions: ExecutionIntervention[]
  flowchartData: FlowchartData
  configuration: ExecutionConfiguration
}

export interface ExecutionIntervention {
  id: string
  type: 'user_feedback' | 'error_recovery' | 'plan_adjustment'
  message: string
  options?: string[]
  timestamp: Date
  resolvedAt?: Date
  resolution?: string
}

export interface FlowchartData {
  nodes: FlowchartNode[]
  connections: FlowchartConnection[]
}

export interface FlowchartNode {
  id: string
  label: string
  type: 'start' | 'step' | 'decision' | 'end' | 'agent'
  position: { x: number; y: number }
  status: 'pending' | 'running' | 'completed' | 'failed'
  dependencies: string[]
}

export interface FlowchartConnection {
  id: string
  from: string
  to: string
  label?: string
  condition?: string
  status?: string
  type?: string
}

export interface ExecutionConfiguration {
  maxRetries: number
  timeout: number
  allowUserIntervention: boolean
  autoReplan: boolean
  parallelExecution: boolean
  maxConcurrentSteps: number
}

// 缺失类型定义
export interface StateTransition {
  from: ExecutionStatusType
  to: ExecutionStatusType
  condition?: string
  action?: string
}

export interface StateMachineConfig {
  initialState: ExecutionStatusType
  states: Map<ExecutionStatusType, ExecutionStatusType[]>
  stateHandlers: Map<ExecutionStatusType, (context: any) => Promise<void>>
  transitions: StateTransition[]
}

export type EnhancedExecutionContext = ExecutionContext

export interface StateHistoryEntry {
  state: ExecutionStatusType
  timestamp: Date
  duration: number
  metadata?: Record<string, any>
}

export interface StateTransitionEvent {
  from: ExecutionStatusType
  to: ExecutionStatusType
  timestamp: Date
  trigger: string
  metadata?: Record<string, any>
}

export interface RealTimeUpdate {
  type: 'state_change' | 'progress_update' | 'error' | 'intervention_required'
  payload: any
  timestamp: Date
}

export interface ExecutionStrategy {
  name: string
  description: string
  config: Record<string, any>
}

export interface UserIntervention {
  id: string
  type: 'manual_approval' | 'input_required' | 'error_resolution'
  message: string
  timestamp: Date
  response?: string
  resolvedAt?: Date
}

export interface ExecutionWarning {
  id: string
  level: 'low' | 'medium' | 'high' | 'critical'
  message: string
  timestamp: Date
  dismissed?: boolean
}
