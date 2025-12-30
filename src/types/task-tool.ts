/**
 * Task-Tool Integration Types
 * Types for tracking tool execution within scan tasks
 */

export type ToolType = 'plugin' | 'mcp_server' | 'builtin' | 'workflow';

export type ToolExecutionStatus = 'idle' | 'running' | 'waiting' | 'completed' | 'error';

export interface ActiveToolInfo {
  tool_id: string;
  tool_name: string;
  tool_type: ToolType;
  status: ToolExecutionStatus;
  execution_count: number;
  avg_execution_time: number;
  last_execution_time?: string;
  error_count: number;
}

export interface ToolStatistics {
  total_executions: number;
  successful_executions: number;
  failed_executions: number;
  total_execution_time: number;
  tools_used: string[];
}

export interface ExecutionRecord {
  id: string;
  tool_name: string;
  tool_type: ToolType;
  status: ToolExecutionStatus;
  started_at: string;
  completed_at?: string;
  execution_time_ms?: number;
  error_message?: string;
}

// Event payloads
export interface ToolStartedEvent {
  task_id: string;
  tool_id: string;
  tool_name: string;
  tool_type: string;
  log_id: string;
  timestamp: string;
}

export interface ToolCompletedEvent {
  task_id: string;
  tool_id: string;
  log_id: string;
  success: boolean;
  error_message?: string;
  timestamp: string;
}

export interface ToolFailedEvent {
  task_id: string;
  tool_id: string;
  log_id: string;
  error_message: string;
  timestamp: string;
}

export interface ToolStatusChangedEvent {
  task_id: string;
  active_tools: ActiveToolInfo[];
  statistics: ToolStatistics;
  timestamp: string;
}
