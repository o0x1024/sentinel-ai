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
  taskProgress?: TaskProgress
}

export interface Conversation {
  id: string
  title: string
  created_at: string
  total_messages: number
}
