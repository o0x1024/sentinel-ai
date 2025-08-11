// 增强的执行状态管理类型定义

// 更细粒度的执行状态
export enum DetailedExecutionStatus {
  // 初始状态
  INITIALIZED = 'initialized',
  
  // 规划阶段
  PLANNING_STARTED = 'planning_started',
  ANALYZING_INTENT = 'analyzing_intent',
  RETRIEVING_MEMORY = 'retrieving_memory',
  GENERATING_PLAN = 'generating_plan',
  OPTIMIZING_PLAN = 'optimizing_plan',
  VALIDATING_PLAN = 'validating_plan',
  PLANNING_COMPLETED = 'planning_completed',
  PLANNING_FAILED = 'planning_failed',
  
  // 执行阶段
  EXECUTION_STARTED = 'execution_started',
  PREPARING_CONTEXT = 'preparing_context',
  STEP_EXECUTING = 'step_executing',
  TOOL_CALLING = 'tool_calling',
  WAITING_TOOL_RESPONSE = 'waiting_tool_response',
  PROCESSING_RESULT = 'processing_result',
  STEP_COMPLETED = 'step_completed',
  STEP_FAILED = 'step_failed',
  
  // 监控阶段
  MONITORING_STARTED = 'monitoring_started',
  CHECKING_PROGRESS = 'checking_progress',
  DETECTING_ANOMALY = 'detecting_anomaly',
  EVALUATING_PERFORMANCE = 'evaluating_performance',
  
  // 重规划阶段
  REPLAN_TRIGGERED = 'replan_triggered',
  ANALYZING_FAILURE = 'analyzing_failure',
  GENERATING_ALTERNATIVE = 'generating_alternative',
  UPDATING_PLAN = 'updating_plan',
  REPLAN_COMPLETED = 'replan_completed',
  REPLAN_FAILED = 'replan_failed',
  
  // 暂停和恢复
  PAUSING = 'pausing',
  PAUSED = 'paused',
  RESUMING = 'resuming',
  
  // 终止状态
  COMPLETING = 'completing',
  COMPLETED = 'completed',
  CANCELLING = 'cancelling',
  CANCELLED = 'cancelled',
  FAILED = 'failed',
  
  // 需要干预
  REQUIRES_INTERVENTION = 'requires_intervention',
  WAITING_USER_INPUT = 'waiting_user_input'
}

// 状态转换规则
export interface StateTransition {
  from: DetailedExecutionStatus
  to: DetailedExecutionStatus
  condition?: string
  action?: string
  metadata?: Record<string, any>
}

// 状态机配置
export interface StateMachineConfig {
  initialState: DetailedExecutionStatus
  transitions: StateTransition[]
  allowedTransitions: Map<DetailedExecutionStatus, DetailedExecutionStatus[]>
  stateHandlers: Map<DetailedExecutionStatus, (context: ExecutionContext) => Promise<void>>
}

// 增强的执行上下文
export interface EnhancedExecutionContext {
  sessionId: string
  currentState: DetailedExecutionStatus
  previousState?: DetailedExecutionStatus
  stateHistory: StateHistoryEntry[]
  currentStep?: PlanStep
  executionPlan: ExecutionPlan
  variables: Record<string, any>
  metrics: ExecutionMetrics
  errors: ExecutionError[]
  warnings: ExecutionWarning[]
  interventions: UserIntervention[]
  flowchartData: FlowchartData
}

// 状态历史记录
export interface StateHistoryEntry {
  state: DetailedExecutionStatus
  timestamp: Date
  duration?: number
  metadata?: Record<string, any>
  triggeredBy?: 'system' | 'user' | 'external'
}

// 执行警告
export interface ExecutionWarning {
  id: string
  type: 'performance' | 'resource' | 'logic' | 'security'
  message: string
  severity: 'low' | 'medium' | 'high'
  timestamp: Date
  step?: string
  metadata?: Record<string, any>
}

// 用户干预
export interface UserIntervention {
  id: string
  type: 'approval' | 'input' | 'decision' | 'correction'
  prompt: string
  options?: string[]
  response?: string
  timestamp: Date
  resolvedAt?: Date
  metadata?: Record<string, any>
}

// 流程图数据
export interface FlowchartData {
  nodes: FlowchartNode[]
  connections: FlowchartConnection[]
  layout: 'auto' | 'manual'
  viewport: {
    x: number
    y: number
    zoom: number
  }
}

export interface FlowchartNode {
  id: string
  name: string
  description: string
  type: 'start' | 'planner' | 'agent' | 'tools' | 'replan' | 'end' | 'decision' | 'parallel'
  status: DetailedExecutionStatus
  progress?: number
  position: { x: number; y: number }
  size: { width: number; height: number }
  dependencies: string[]
  metadata?: Record<string, any>
  style?: {
    color?: string
    backgroundColor?: string
    borderColor?: string
    icon?: string
  }
}

export interface FlowchartConnection {
  id: string
  from: string
  to: string
  type: 'sequence' | 'condition' | 'loop' | 'parallel'
  status: 'inactive' | 'active' | 'completed' | 'failed'
  condition?: string
  label?: string
  style?: {
    color?: string
    width?: number
    dashArray?: string
  }
}

// 动态流程调整
export interface FlowAdjustment {
  id: string
  type: 'add_step' | 'remove_step' | 'modify_step' | 'change_order' | 'add_condition' | 'modify_condition'
  target: string // 目标步骤或连接ID
  data: any
  reason: string
  timestamp: Date
  appliedBy: 'system' | 'user'
  metadata?: Record<string, any>
}

// 执行策略
export interface ExecutionStrategy {
  id: string
  name: string
  description: string
  retryPolicy: RetryPolicy
  timeoutPolicy: TimeoutPolicy
  parallelismPolicy: ParallelismPolicy
  errorHandlingPolicy: ErrorHandlingPolicy
  monitoringPolicy: MonitoringPolicy
}

export interface RetryPolicy {
  maxRetries: number
  backoffStrategy: 'linear' | 'exponential' | 'fixed'
  baseDelay: number
  maxDelay: number
  retryableErrors: string[]
}

export interface TimeoutPolicy {
  stepTimeout: number
  sessionTimeout: number
  toolTimeout: number
  warningThreshold: number
}

export interface ParallelismPolicy {
  maxConcurrentSteps: number
  maxConcurrentTools: number
  resourceLimits: {
    cpu: number
    memory: number
    network: number
  }
}

export interface ErrorHandlingPolicy {
  autoReplan: boolean
  escalationThreshold: number
  fallbackStrategies: string[]
  notificationRules: NotificationRule[]
}

export interface MonitoringPolicy {
  metricsCollection: boolean
  anomalyDetection: boolean
  performanceThresholds: {
    stepDuration: number
    memoryUsage: number
    errorRate: number
  }
  alertRules: AlertRule[]
}

export interface NotificationRule {
  id: string
  condition: string
  channels: ('ui' | 'log' | 'webhook')[]
  severity: 'info' | 'warning' | 'error' | 'critical'
  template: string
}

export interface AlertRule {
  id: string
  metric: string
  operator: '>' | '<' | '=' | '>=' | '<=' | '!='
  threshold: number
  duration: number
  action: 'log' | 'notify' | 'pause' | 'replan' | 'cancel'
}

// API接口类型
export interface EnhancedExecuteRequest {
  userRequest: string
  context?: Record<string, any>
  strategy?: ExecutionStrategy
  flowchartConfig?: {
    enableVisualization: boolean
    autoLayout: boolean
    realTimeUpdates: boolean
  }
  monitoringConfig?: {
    enableRealTimeMonitoring: boolean
    metricsInterval: number
    anomalyDetection: boolean
  }
}

export interface EnhancedExecutionResponse {
  sessionId: string
  status: DetailedExecutionStatus
  context: EnhancedExecutionContext
  flowchartData: FlowchartData
  metrics: ExecutionMetrics
  nextActions?: string[]
  interventionRequired?: UserIntervention
}

export interface StateTransitionEvent {
  sessionId: string
  from: DetailedExecutionStatus
  to: DetailedExecutionStatus
  timestamp: Date
  metadata?: Record<string, any>
  flowchartUpdate?: {
    nodeUpdates: Partial<FlowchartNode>[]
    connectionUpdates: Partial<FlowchartConnection>[]
  }
}

// 实时更新事件
export interface RealTimeUpdate {
  type: 'state_change' | 'progress_update' | 'error' | 'warning' | 'intervention_required'
  sessionId: string
  data: any
  timestamp: Date
}

// 导入基础类型
export interface ExecutionPlan {
  id: string
  name: string
  description: string
  steps: PlanStep[]
  metadata: Record<string, any>
  createdAt: Date
  updatedAt: Date
}

export interface PlanStep {
  id: string
  name: string
  description: string
  type: 'tool_call' | 'condition' | 'parallel' | 'loop' | 'human_input'
  dependencies: string[]
  config: Record<string, any>
  retryConfig?: RetryConfig
  timeoutMs?: number
  metadata?: Record<string, any>
}

export interface RetryConfig {
  maxRetries: number
  backoffMs: number
  retryableErrors: string[]
}

export interface ExecutionMetrics {
  startTime: Date
  endTime?: Date
  duration?: number
  stepsCompleted: number
  stepsTotal: number
  toolCallsCount: number
  errorsCount: number
  warningsCount: number
  memoryUsage?: number
  cpuUsage?: number
  networkRequests?: number
}

export interface ExecutionError {
  id: string
  type: 'tool_error' | 'validation_error' | 'timeout_error' | 'system_error'
  message: string
  step?: string
  timestamp: Date
  stack?: string
  metadata?: Record<string, any>
}

export interface ExecutionContext {
  sessionId: string
  userId?: string
  variables: Record<string, any>
  environment: Record<string, any>
  permissions: string[]
  resourceLimits: {
    maxDuration: number
    maxMemory: number
    maxToolCalls: number
  }
}