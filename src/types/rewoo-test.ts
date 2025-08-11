/**
 * ReWOO 测试相关的 TypeScript 类型定义
 */

// ============= 基础类型定义 =============

/**
 * 引擎状态信息
 */
export interface EngineStatusInfo {
  active: boolean
  ready: boolean
  version: string
  uptime_seconds: number
  total_sessions: number
  active_sessions: number
}

/**
 * ReWOO 配置
 */
export interface ReWOOConfig {
  planner: {
    model_name: string
    temperature: number
    max_tokens: number
    max_steps: number
  }
  worker: {
    timeout_seconds: number
    max_retries: number
    enable_parallel: boolean
  }
  solver: {
    model_name: string
    temperature: number
    max_tokens: number
  }
}

/**
 * 测试配置
 */
export interface TestConfig {
  name: string
  task: string
  expected_tools: string[]
  timeout_seconds: number
  rewoo_config: ReWOOConfig
}

/**
 * 测试指标
 */
export interface TestMetrics {
  total_time_ms: number
  tool_calls: number
  successful_tool_calls: number
  total_tokens: number
}

/**
 * 日志级别
 */
export type LogLevel = 'DEBUG' | 'INFO' | 'WARN' | 'ERROR'

/**
 * 执行日志条目
 */
export interface LogEntry {
  timestamp: string | { secs_since_epoch: number }
  level: LogLevel
  component: string // 'planner' | 'worker' | 'solver' | 'system'
  message: string
  details?: any
}

/**
 * 测试结果
 */
export interface TestResult {
  id: string
  test_name: string
  task: string
  result?: string
  error?: string
  metrics: TestMetrics
  started_at: string | { secs_since_epoch: number }
  completed_at?: string | { secs_since_epoch: number }
  success: boolean
  logs: LogEntry[] // 新增：执行日志
}

/**
 * 自定义测试配置
 */
export interface CustomTestConfig {
  name: string
  task: string
  timeout_seconds: number
  expected_tools: string[]
  rewoo_config: ReWOOConfig
}

// ============= 默认配置 =============

/**
 * 默认 ReWOO 配置
 */
export const defaultReWOOConfig: ReWOOConfig = {
  planner: {
    model_name: 'gpt-4',
    temperature: 0.0,
    max_tokens: 4000,
    max_steps: 10
  },
  worker: {
    timeout_seconds: 300,
    max_retries: 3,
    enable_parallel: false
  },
  solver: {
    model_name: 'gpt-4',
    temperature: 0.0,
    max_tokens: 2000
  }
}

/**
 * 默认测试指标
 */
export const defaultTestMetrics: TestMetrics = {
  total_time_ms: 0,
  tool_calls: 0,
  successful_tool_calls: 0,
  total_tokens: 0
}