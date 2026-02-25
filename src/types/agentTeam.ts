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
    is_system: boolean
    created_by?: string
    created_at: string
    updated_at: string
    members: AgentTeamTemplateMember[]
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
    members: AgentTeamMember[]
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
    members: CreateAgentTeamTemplateMemberRequest[]
}

export interface UpdateAgentTeamTemplateRequest {
    name?: string
    description?: string
    domain?: string
    default_rounds_config?: any
    default_tool_policy?: any
    members?: CreateAgentTeamTemplateMemberRequest[]
}

export interface CreateAgentTeamSessionRequest {
    name: string
    goal?: string
    template_id?: string
    conversation_id?: string
    max_rounds?: number
    members?: CreateAgentTeamTemplateMemberRequest[]
}

export interface UpdateAgentTeamSessionRequest {
    name?: string
    goal?: string
    state?: string
    max_rounds?: number
    error_message?: string
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

// ==================== 执行模式 ====================

export type ExecutionMode = 'single' | 'team'
