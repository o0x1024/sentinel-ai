// Plugin Management Types

export interface PluginMetadata {
  id: string
  name: string
  version: string
  author?: string
  category: string
  main_category: string
  description?: string
  default_severity: string
  tags: string[]
}

export interface PluginRecord {
  metadata: PluginMetadata
  status: 'Enabled' | 'Disabled' | 'Error'
  file_path: string
  is_favorited?: boolean
}

export interface ReviewPlugin {
  plugin_id: string
  plugin_name: string
  code: string
  description: string
  vuln_type: string
  quality_score: number
  quality_breakdown: {
    syntax_score: number
    logic_score: number
    security_score: number
    code_quality_score: number
  }
  validation: {
    is_valid: boolean
    syntax_valid: boolean
    has_required_functions: boolean
    security_check_passed: boolean
    errors: string[]
    warnings: string[]
  }
  status: string
  generated_at: string
  model: string
}

export interface CommandResponse<T> {
  success: boolean
  data?: T
  error?: string
}

export interface TestResult {
  success: boolean
  message?: string
  findings?: Array<{
    title: string
    description: string
    severity: string
  }>
  error?: string
}

export interface AdvancedRunStat {
  run_index: number
  duration_ms: number
  findings: number
  error?: string | null
}

export interface BatchToggleResult {
  enabled_count: number
  disabled_count: number
  failed_ids: string[]
}

export interface AdvancedTestResult {
  plugin_id: string
  success: boolean
  total_runs: number
  concurrency: number
  total_duration_ms: number
  avg_duration_ms: number
  total_findings: number
  unique_findings: number
  findings: Array<{ title: string; description: string; severity: string }>
  runs: AdvancedRunStat[]
  message?: string
  error?: string
  outputs?: any[]
}

export interface ReviewStats {
  total: number
  pending: number
  approved: number
  rejected: number
  failed: number
}

export interface NewPluginMetadata {
  id: string
  name: string
  version: string
  author: string
  mainCategory: string
  category: string
  default_severity: string
  description: string
  tagsString: string
}

export interface AdvancedForm {
  url: string
  method: string
  headersText: string
  bodyText: string
  agent_inputs_text: string
  runs: number
  concurrency: number
}

// Category definitions
export interface Category {
  value: string
  label: string
  icon: string
}

export interface SubCategory {
  value: string
  label: string
  icon: string
}

// Main categories
export const mainCategories: Category[] = [
  { value: 'traffic', label: '流量分析插件', icon: 'fas fa-shield-alt' },
  { value: 'agent', label: 'Agent插件', icon: 'fas fa-robot' }
]

// Traffic analysis plugin subcategories
export const trafficCategories = [
  'sqli', 'command_injection', 'xss', 'idor', 'auth_bypass', 'csrf',
  'info_leak', 'file_upload', 'file_inclusion', 'path_traversal',
  'xxe', 'ssrf', 'report', 'custom'
]

// Agent plugin subcategories
export const agentsCategories = [
  'scanner', 'analyzer', 'reporter', 'recon', 'exploit', 'utility', 'custom'
]

// Code reference type for AI assistant
export interface CodeReference {
  code: string
  preview: string
  startLine: number
  endLine: number
  isFullCode: boolean
}

// Test result reference type for AI assistant
export interface TestResultReference {
  success: boolean
  message: string
  preview: string
  findings?: Array<{ title: string; description: string; severity: string }>
  error?: string
  executionTime?: number
  timestamp: number
}

// AI chat message type
export interface AiChatMessage {
  role: 'user' | 'assistant'
  content: string
  codeBlock?: string
  codeBlocks?: string[]
  codeRef?: CodeReference
  testResultRef?: TestResultReference
}
