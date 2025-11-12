export interface TaskStep {
  id: string;
  name: string;
  status: 'Pending' | 'Running' | 'Completed' | 'Failed';
  description?: string;
  result?: any;
  error?: string;
  started_at?: string;
  completed_at?: string;
}

export interface TaskProgress {
  execution_id: string;
  task_name: string;
  status: 'Running' | 'Completed' | 'Failed';
  progress: number; // 0-100
  steps: TaskStep[];
  message_id?: string;
}

export interface Citation {
  id: string
  source_id: string
  file_name: string
  file_path?: string
  page_number?: number
  section_title?: string
  start_char: number
  end_char: number
  score: number
  content_preview: string
}

export interface ChatMessage {
  id: string
  role: 'user' | 'assistant'
  content: string
  timestamp: Date
  isStreaming?: boolean
  hasError?: boolean
  executionPlan?: any
  toolExecutions?: any[]
  executionResult?: any
  executionProgress?: number
  currentStep?: string
  totalSteps?: number
  completedSteps?: number
  selectedArchitecture?: string
  execution_id?: string
  citations?: Citation[]
  // 存储解析后的 ReAct 步骤数据（包含从 chunks 提取的 observation）
  reactSteps?: Array<{
    thought?: string
    action?: any
    observation?: any
    error?: string
    finalAnswer?: string
  }>
  segments?: Array<{
    id: string
    type: 'reasoning' | 'plan' | 'tool' | 'content' | 'error' | 'meta'
    seq: number
    stage?: string
    title?: string
    status?: 'pending' | 'running' | 'success' | 'error'
    content?: string
    data?: any
    collapsed?: boolean
  }>
  taskProgress?: TaskProgress
}

export interface Conversation {
  id: string
  title: string
  created_at: string
  total_messages: number
}
