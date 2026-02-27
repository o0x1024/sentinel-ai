/**
 * Agent Team 模块前端类型定义
 */

// ==================== 状态枚举 ====================

export type TeamSessionState =
    | 'PENDING'
    | 'INITIALIZING'
    | 'PROPOSING'
    | 'CHALLENGING'
    | 'CONVERGENCE_CHECK'
    | 'REVISING'
    | 'DECIDING'
    | 'ARTIFACT_GENERATION'
    | 'COMPLETED'
    | 'FAILED'
    | 'ARCHIVED'
    | 'SUSPENDED_FOR_HUMAN'

// ==================== 模板结构 ====================

export interface AgentTeamTemplateMember {
    id: string
    template_id: string
    name: string
    responsibility?: string
    system_prompt?: string
    decision_style?: string
    risk_preference?: string
    weight: number
    tool_policy?: any
    output_schema?: any
    sort_order: number
    created_at: string
    updated_at: string
}

export interface AgentTeamTemplate {
    id: string
    name: string
    description?: string
    domain: string
    default_rounds_config?: any
    default_tool_policy?: any
    schema_version: number
    template_spec_v2?: TeamTemplateSpecV2
    upgrade_failed?: boolean
    upgrade_error?: string
    is_system: boolean
    created_by?: string
    created_at: string
    updated_at: string
    // Legacy snapshot; V2 主流程请优先使用 template_spec_v2.agents
    members?: AgentTeamTemplateMember[]
}

// ==================== 会话结构 ====================

export interface AgentTeamMember {
    id: string
    session_id: string
    name: string
    responsibility?: string
    system_prompt?: string
    decision_style?: string
    risk_preference?: string
    weight: number
    tool_policy?: any
    output_schema?: any
    sort_order: number
    token_usage: number
    tool_calls_count: number
    is_active: boolean
    created_at: string
    updated_at: string
}

export interface AgentTeamSession {
    id: string
    conversation_id?: string
    template_id?: string
    name: string
    goal?: string
    orchestration_plan?: any
    schema_version: number
    runtime_spec_v2?: TeamTemplateSpecV2
    plan_version: number
    state: string
    state_machine?: any
    current_round: number
    max_rounds: number
    blackboard_state?: any
    divergence_scores?: any
    total_tokens: number
    estimated_cost: number
    suspended_reason?: string
    started_at?: string
    completed_at?: string
    error_message?: string
    created_at: string
    updated_at: string
    // Legacy snapshot; V2 主流程请优先使用 runtime_spec_v2.agents
    members?: AgentTeamMember[]
}

// ==================== 轮次 ====================

export interface AgentTeamRound {
    id: string
    session_id: string
    round_number: number
    phase: string
    status: string
    divergence_score?: number
    started_at?: string
    completed_at?: string
    created_at: string
}

// ==================== 消息 ====================

export interface AgentTeamMessage {
    id: string
    session_id: string
    round_id?: string
    member_id?: string
    member_name?: string
    role: string
    content: string
    tool_calls?: any
    token_count?: number
    timestamp: string
}

// ==================== 白板 ====================

export interface AgentTeamBlackboardEntry {
    id: string
    session_id: string
    round_id?: string
    entry_type: string
    title: string
    content: string
    contributed_by?: string
    is_resolved: boolean
    created_at: string
    updated_at: string
}

export interface AgentTeamBlackboardArchive {
    entry: AgentTeamBlackboardEntry
    messages: AgentTeamMessage[]
    retrieval_scope: string
}

// ==================== 产物 ====================

export interface AgentTeamArtifact {
    id: string
    session_id: string
    artifact_type: string
    title: string
    content: string
    version: number
    parent_artifact_id?: string
    diff_summary?: string
    created_by?: string
    created_at: string
    updated_at: string
}

// ==================== 运行状态 ====================

export interface AgentTeamRunStatus {
    session_id: string
    state: string
    current_round: number
    blackboard_snapshot?: any
    latest_message?: string
    divergence_score?: number
    is_suspended: boolean
}

// ==================== API 请求类型 ====================

export interface CreateAgentTeamTemplateMemberRequest {
    name: string
    responsibility?: string
    system_prompt?: string
    decision_style?: string
    risk_preference?: string
    weight?: number
    tool_policy?: any
    output_schema?: any
    sort_order?: number
}

export interface CreateAgentTeamTemplateRequest {
    name: string
    description?: string
    domain: string
    default_rounds_config?: any
    default_tool_policy?: any
    schema_version?: number
    agents: AgentProfile[]
    task_graph: TeamTaskGraph
    hook_policy?: any
}

export interface UpdateAgentTeamTemplateRequest {
    name?: string
    description?: string
    domain?: string
    default_rounds_config?: any
    default_tool_policy?: any
    schema_version?: number
    agents?: AgentProfile[]
    task_graph?: TeamTaskGraph
    hook_policy?: any
}

export interface CreateAgentTeamSessionRequest {
    name: string
    goal?: string
    template_id?: string
    conversation_id?: string
    max_rounds?: number
    schema_version?: number
    runtime_spec_v2?: TeamTemplateSpecV2
    state_machine?: any
}

export interface UpdateAgentTeamSessionRequest {
    name?: string
    goal?: string
    state?: string
    max_rounds?: number
    schema_version?: number
    runtime_spec_v2?: TeamTemplateSpecV2
    state_machine?: any
    error_message?: string
}

export interface AgentProfile {
    id: string
    name: string
    system_prompt?: string
    model?: string
    tool_policy?: any
    skills?: string[]
    max_parallel_tasks?: number
}

export interface TeamTaskRetryPolicy {
    max_attempts?: number
    backoff_ms?: number
}

export interface TeamTaskSla {
    timeout_secs?: number
}

export interface TeamTaskNode {
    id: string
    title: string
    instruction: string
    depends_on?: string[]
    assignee_strategy?: any
    retry?: TeamTaskRetryPolicy
    sla?: TeamTaskSla
    input_schema?: any
    output_schema?: any
    phase?: string
}

export interface TeamTaskGraph {
    version?: number
    nodes: TeamTaskNode[]
}

export interface TeamTemplateSpecV2 {
    schema_version: number
    agents: AgentProfile[]
    task_graph: TeamTaskGraph
    hook_policy?: any
}

export interface TeamTask {
    id: string
    session_id: string
    task_id: string
    title: string
    instruction: string
    status: string
    assignee_agent_id?: string
    depends_on: string[]
    attempt: number
    max_attempts: number
    last_error?: string
    started_at?: string
    completed_at?: string
    created_at: string
    updated_at: string
}

export interface MailboxMessage {
    id: string
    session_id: string
    from_agent_id?: string
    to_agent_id?: string
    task_record_id?: string
    message_type: string
    payload: any
    is_acknowledged: boolean
    created_at: string
    acknowledged_at?: string
}

export interface UpdateTaskRequest {
    task_id: string
    status?: string
    assignee_agent_id?: string
    last_error?: string
}

export interface UpdateBlackboardRequest {
    session_id: string
    entry_type: string
    title: string
    content: string
    contributed_by?: string
    round_id?: string
}

export interface SubmitAgentTeamMessageRequest {
    session_id: string
    content: string
    resume: boolean
}

export interface AppendAgentTeamPartialMessageRequest {
    session_id: string
    member_id?: string
    member_name?: string
    role: string
    content: string
    tool_calls?: any
}

// ==================== 事件载荷 ====================

export interface AgentTeamRoleThinkingEvent {
    member_id: string
    member_name: string
    phase: string
}

export interface AgentTeamRoundEvent {
    round: number
    phase: string
    divergence_score?: number
}

export interface AgentTeamStateChangedEvent {
    session_id: string
    state: string
}

export interface AgentTeamArtifactEvent {
    session_id: string
    artifact_type: string
    title: string
}

export interface AgentTeamDivergenceAlertEvent {
    session_id: string
    divergence_score: number
    threshold: number
}

export interface AgentTeamMessageStreamStartEvent {
    session_id: string
    stream_id: string
    member_id?: string
    member_name?: string
    phase: string
}

export interface AgentTeamMessageStreamDeltaEvent {
    session_id: string
    stream_id: string
    member_id?: string
    member_name?: string
    phase: string
    delta: string
}

export interface AgentTeamMessageStreamDoneEvent {
    session_id: string
    stream_id: string
    member_id?: string
    member_name?: string
    phase: string
    content?: string
    error?: string
    had_delta?: boolean
}

export interface AgentTeamToolCallEvent {
    session_id: string
    stream_id: string
    member_id?: string
    member_name?: string
    phase: string
    tool_call_id: string
    name: string
    arguments: string
    timestamp?: string
}

export interface AgentTeamToolResultEvent {
    session_id: string
    stream_id: string
    member_id?: string
    member_name?: string
    phase: string
    tool_call_id: string
    result: string
    success?: boolean
    timestamp?: string
}

// ==================== 执行模式 ====================

export type ExecutionMode = 'single' | 'team'
