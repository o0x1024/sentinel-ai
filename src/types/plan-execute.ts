/**
 * Plan-and-Execute架构的TypeScript类型定义和API接口
 */

// ============= 基础类型定义 =============

/**
 * 执行状态枚举
 */

import { invoke } from '@tauri-apps/api/core';



export enum ExecutionStatus {
  Pending = 'pending',
  Planning = 'planning',
  Running = 'running',
  Paused = 'paused',
  Completed = 'completed',
  Failed = 'failed',
  Cancelled = 'cancelled'
}

/**
 * 步骤类型枚举
 */
export enum StepType {
  Scan = 'scan',
  Analysis = 'analysis',
  Verification = 'verification',
  Report = 'report',
  Custom = 'custom'
}

/**
 * 优先级枚举
 */
export enum Priority {
  Low = 'low',
  Normal = 'normal',
  High = 'high',
  Critical = 'critical'
}

// ============= 核心数据结构 =============

/**
 * 执行计划
 */
export interface ExecutionPlan {
  id: string;
  name: string;
  description?: string;
  user_input: string;
  intent_analysis: string;
  steps: PlanStep[];
  estimated_duration: number;
  priority: Priority;
  created_at: string;
  created_by: string;
  metadata?: Record<string, any>;
}

/**
 * 计划步骤
 */
export interface PlanStep {
  id: string;
  plan_id: string;
  step_order: number;
  step_type: StepType;
  name: string;
  description?: string;
  tool_name: string;
  parameters: Record<string, any>;
  expected_output?: string;
  estimated_duration: number;
  retry_count: number;
  timeout_seconds?: number;
  dependencies: string[];
  parallel_group?: string;
  conditions?: Record<string, any>;
  metadata?: Record<string, any>;
}

/**
 * 执行会话
 */
export interface ExecutionSession {
  id: string;
  plan_id: string;
  user_id: string;
  status: ExecutionStatus;
  current_step_id?: string;
  progress: number;
  started_at?: string;
  completed_at?: string;
  error_message?: string;
  results_summary?: string;
  metadata?: Record<string, any>;
}

/**
 * 步骤执行结果
 */
export interface StepExecutionResult {
  id: string;
  session_id: string;
  step_id: string;
  status: ExecutionStatus;
  started_at: string;
  completed_at?: string;
  execution_time?: number;
  output?: string;
  error_message?: string;
  retry_count: number;
  metadata?: Record<string, any>;
}

/**
 * 执行指标
 */
export interface ExecutionMetrics {
  id: string;
  session_id: string;
  step_id?: string;
  metric_type: string;
  metric_name: string;
  metric_value: number;
  unit?: string;
  timestamp: string;
  metadata?: Record<string, any>;
}

// ============= API请求/响应类型 =============

/**
 * 命令响应包装器
 */
export interface CommandResponse<T> {
  success: boolean;
  data?: T;
  error?: string;
  timestamp: number;
}

/**
 * 执行请求参数
 */
export interface ExecuteRequestParams {
  user_input: string;
  user_id: string;
  session_id?: string;
  auto_execute?: boolean;
  require_confirmation?: boolean;
  priority?: string;
  max_execution_time_seconds?: number;
  custom_parameters?: Record<string, any>;
}

/**
 * 执行响应
 */
export interface ExecutionResponse {
  session_id: string;
  plan: ExecutionPlan;
  status: ExecutionStatus;
  message: string;
  next_action?: string;
}

/**
 * 会话状态
 */
export interface SessionStatus {
  session: ExecutionSession;
  plan: ExecutionPlan;
  current_step?: PlanStep;
  completed_steps: StepExecutionResult[];
  metrics: ExecutionMetrics[];
  logs: ExecutionLog[];
}

/**
 * 执行日志
 */
export interface ExecutionLog {
  id: string;
  session_id: string;
  step_id?: string;
  level: 'debug' | 'info' | 'warn' | 'error';
  message: string;
  timestamp: string;
  metadata?: Record<string, any>;
}

/**
 * 会话列表请求参数
 */
export interface ListSessionsParams {
  user_id?: string;
  status?: string;
  page?: number;
  page_size?: number;
}

/**
 * 会话列表响应
 */
export interface SessionListResponse {
  sessions: SessionStatus[];
  total_count: number;
  pagination: PaginationInfo;
}

/**
 * 分页信息
 */
export interface PaginationInfo {
  current_page: number;
  page_size: number;
  total_pages: number;
  has_next: boolean;
  has_previous: boolean;
}

/**
 * 服务统计信息
 */
export interface ServiceStatistics {
  total_sessions: number;
  active_sessions: number;
  completed_sessions: number;
  failed_sessions: number;
  average_execution_time: number;
  success_rate: number;
  most_used_tools: Array<{ tool_name: string; usage_count: number }>;
  performance_metrics: Record<string, number>;
}

/**
 * 服务状态信息
 */
export interface ServiceStatusInfo {
  initialized: boolean;
  version: string;
  uptime_seconds?: number;
}

/**
 * 健康检查结果
 */
export interface HealthCheckResult {
  status: string;
  checks: Record<string, CheckResult>;
  timestamp: number;
}

/**
 * 单项检查结果
 */
export interface CheckResult {
  status: string;
  message: string;
}

/**
 * 工具信息
 */
export interface ToolInfo {
  id: string;
  name: string;
  description: string;
  category: string;
  parameters: ParameterInfo[];
}

/**
 * LLM配置请求参数
 */
export interface LlmConfigRequest {
  provider?: string;
  model?: string;
  api_key?: string;
  endpoint?: string;
  max_tokens?: number;
  temperature?: number;
}

/**
 * LLM提供商枚举
 */
export enum LlmProvider {
  OpenAI = 'openai',
  Anthropic = 'anthropic',
  Google = 'google',
  ModelScope = 'modelscope',
  OpenRouter = 'openrouter'
}

/**
 * 参数信息
 */
export interface ParameterInfo {
  name: string;
  description: string;
  parameter_type: string;
  required: boolean;
  default_value?: string;
}

// ============= API函数 =============

/**
 * Plan-and-Execute API类
 */
export class PlanExecuteAPI {
  /**
   * 执行安全测试请求
   */
  static async executeSecurityTest(
    params: ExecuteRequestParams
  ): Promise<CommandResponse<ExecutionResponse>> {
    return invoke('execute_security_test', { params });
  }

  /**
   * 开始执行
   */
  static async startExecution(
    sessionId: string
  ): Promise<CommandResponse<void>> {
    return invoke('start_execution', { sessionId });
  }

  /**
   * 暂停执行
   */
  static async pauseExecution(
    sessionId: string
  ): Promise<CommandResponse<void>> {
    return invoke('pause_execution', { sessionId });
  }

  /**
   * 恢复执行
   */
  static async resumeExecution(
    sessionId: string
  ): Promise<CommandResponse<void>> {
    return invoke('resume_execution', { sessionId });
  }

  /**
   * 取消执行
   */
  static async cancelExecution(
    sessionId: string
  ): Promise<CommandResponse<void>> {
    return invoke('cancel_execution', { sessionId });
  }

  /**
   * 获取会话状态
   */
  static async getSessionStatus(
    sessionId: string
  ): Promise<CommandResponse<SessionStatus>> {
    return invoke('get_session_status', { sessionId });
  }

  /**
   * 列出会话
   */
  static async listSessions(
    params: ListSessionsParams
  ): Promise<CommandResponse<SessionListResponse>> {
    return invoke('list_sessions', { params });
  }

  /**
   * 获取服务统计信息
   */
  static async getServiceStatistics(): Promise<CommandResponse<ServiceStatistics>> {
    return invoke('get_service_statistics');
  }

  /**
   * 删除会话
   */
  static async deleteSession(
    sessionId: string
  ): Promise<CommandResponse<void>> {
    return invoke('delete_session', { sessionId });
  }

  /**
   * 初始化Plan-and-Execute服务
   */
  static async initializePlanExecuteService(): Promise<CommandResponse<string>> {
    return invoke('initialize_plan_execute_service');
  }

  /**
   * 获取服务状态
   */
  static async getServiceStatus(): Promise<CommandResponse<ServiceStatusInfo>> {
    return invoke('get_service_status');
  }

  /**
   * 健康检查
   */
  static async healthCheck(): Promise<CommandResponse<HealthCheckResult>> {
    return invoke('health_check');
  }

  /**
   * 获取可用工具列表
   */
  static async getAvailableTools(): Promise<CommandResponse<ToolInfo[]>> {
    return invoke('get_available_tools');
  }

  /**
   * 更新LLM配置
   */
  static async updateLlmConfig(
    configRequest: LlmConfigRequest
  ): Promise<CommandResponse<void>> {
    return invoke('update_llm_config', { configRequest });
  }
}

// ============= 工具函数 =============

/**
 * 格式化执行时间
 */
export function formatExecutionTime(seconds: number): string {
  if (seconds < 60) {
    return `${seconds.toFixed(1)}秒`;
  } else if (seconds < 3600) {
    const minutes = Math.floor(seconds / 60);
    const remainingSeconds = seconds % 60;
    return `${minutes}分${remainingSeconds.toFixed(0)}秒`;
  } else {
    const hours = Math.floor(seconds / 3600);
    const minutes = Math.floor((seconds % 3600) / 60);
    return `${hours}小时${minutes}分钟`;
  }
}

/**
 * 获取状态显示文本
 */
export function getStatusDisplayText(status: ExecutionStatus): string {
  const statusMap: Record<ExecutionStatus, string> = {
    [ExecutionStatus.Pending]: '等待中',
    [ExecutionStatus.Planning]: '规划中',
    [ExecutionStatus.Running]: '执行中',
    [ExecutionStatus.Paused]: '已暂停',
    [ExecutionStatus.Completed]: '已完成',
    [ExecutionStatus.Failed]: '执行失败',
    [ExecutionStatus.Cancelled]: '已取消'
  };
  return statusMap[status] || status;
}

/**
 * 获取状态颜色
 */
export function getStatusColor(status: ExecutionStatus): string {
  const colorMap: Record<ExecutionStatus, string> = {
    [ExecutionStatus.Pending]: '#faad14',
    [ExecutionStatus.Planning]: '#1890ff',
    [ExecutionStatus.Running]: '#52c41a',
    [ExecutionStatus.Paused]: '#fa8c16',
    [ExecutionStatus.Completed]: '#52c41a',
    [ExecutionStatus.Failed]: '#ff4d4f',
    [ExecutionStatus.Cancelled]: '#8c8c8c'
  };
  return colorMap[status] || '#8c8c8c';
}

/**
 * 获取优先级显示文本
 */
export function getPriorityDisplayText(priority: Priority): string {
  const priorityMap: Record<Priority, string> = {
    [Priority.Low]: '低',
    [Priority.Normal]: '普通',
    [Priority.High]: '高',
    [Priority.Critical]: '紧急'
  };
  return priorityMap[priority] || priority;
}

/**
 * 获取优先级颜色
 */
export function getPriorityColor(priority: Priority): string {
  const colorMap: Record<Priority, string> = {
    [Priority.Low]: '#52c41a',
    [Priority.Normal]: '#1890ff',
    [Priority.High]: '#fa8c16',
    [Priority.Critical]: '#ff4d4f'
  };
  return colorMap[priority] || '#1890ff';
}

/**
 * 计算进度百分比
 */
export function calculateProgress(session: ExecutionSession, steps: PlanStep[]): number {
  if (!steps.length) return 0;
  
  // 如果会话已完成，返回100%
  if (session.status === ExecutionStatus.Completed) {
    return 100;
  }
  
  // 如果会话失败或取消，返回当前进度
  if (session.status === ExecutionStatus.Failed || session.status === ExecutionStatus.Cancelled) {
    return session.progress;
  }
  
  // 否则返回会话记录的进度
  return session.progress;
}

/**
 * 验证执行请求参数
 */
export function validateExecuteRequestParams(params: ExecuteRequestParams): string[] {
  const errors: string[] = [];
  
  if (!params.user_input?.trim()) {
    errors.push('用户输入不能为空');
  }
  
  if (!params.user_id?.trim()) {
    errors.push('用户ID不能为空');
  }
  
  if (params.max_execution_time_seconds && params.max_execution_time_seconds <= 0) {
    errors.push('最大执行时间必须大于0');
  }
  
  return errors;
}

/**
 * 创建默认执行请求参数
 */
export function createDefaultExecuteRequestParams(userInput: string, userId: string): ExecuteRequestParams {
  return {
    user_input: userInput,
    user_id: userId,
    auto_execute: false,
    require_confirmation: true,
    priority: Priority.Normal,
    max_execution_time_seconds: 3600, // 1小时
    custom_parameters: {}
  };
}