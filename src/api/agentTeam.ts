import { invoke } from '@tauri-apps/api/core'
import type {
  AgentTeamMessage,
  AgentTeamRunStatus,
  AgentTeamSession,
  TeamBlackboardEntry,
  TeamTask,
  CreateAgentTeamSessionRequest,
  UpdateAgentTeamSessionRequest,
  SubmitAgentTeamMessageRequest,
} from '@/types/agentTeam'

type TeamV3MessageRow = {
  id: string
  session_id: string
  thread_id: string
  from_agent_id?: string | null
  to_agent_id?: string | null
  message_type: string
  payload: any
  created_at: string
}

type TeamV3BlackboardRow = {
  id: string
  session_id: string
  task_id?: string | null
  agent_id?: string | null
  entry_type: string
  content: string
  metadata?: any
  created_at: string
  updated_at: string
}

function mapStateMembers(stateData: any, sessionId: string, createdAt: string, updatedAt: string) {
  const rawMembers = Array.isArray(stateData?.members) ? stateData.members : []
  return rawMembers.map((member: any, index: number) => {
    const id = typeof member?.id === 'string' && member.id.trim()
      ? member.id.trim()
      : `member-${index + 1}`
    const name = typeof member?.name === 'string' && member.name.trim()
      ? member.name.trim()
      : `Agent ${index + 1}`
    const tokenUsage = Number(member?.token_usage ?? 0)
    const toolCallsCount = Number(member?.tool_calls_count ?? 0)
    const weightValue = Number(member?.weight ?? 1)
    const sortOrder = Number(member?.sort_order ?? index)
    return {
      id,
      session_id: sessionId,
      name,
      responsibility: typeof member?.responsibility === 'string' ? member.responsibility : undefined,
      system_prompt: typeof member?.system_prompt === 'string' ? member.system_prompt : undefined,
      decision_style: typeof member?.decision_style === 'string' ? member.decision_style : undefined,
      risk_preference: typeof member?.risk_preference === 'string' ? member.risk_preference : undefined,
      weight: Number.isFinite(weightValue) ? weightValue : 1,
      tool_policy: member?.tool_policy,
      output_schema: member?.output_schema,
      sort_order: Number.isFinite(sortOrder) ? sortOrder : index,
      token_usage: Number.isFinite(tokenUsage) ? tokenUsage : 0,
      tool_calls_count: Number.isFinite(toolCallsCount) ? toolCallsCount : 0,
      is_active: member?.is_active === true,
      created_at: createdAt,
      updated_at: updatedAt,
    }
  })
}

function mapV3SessionToLegacy(v3: any): AgentTeamSession {
  const stateData = v3?.state_data && typeof v3.state_data === 'object' ? v3.state_data : undefined
  return {
    id: v3.id,
    conversation_id: v3.conversation_id ?? undefined,
    template_id: undefined,
    name: v3.name,
    goal: v3.goal ?? undefined,
    orchestration_plan: undefined,
    schema_version: 3,
    runtime_spec_v2: undefined,
    plan_version: 1,
    state: v3.state || 'PLAN_DRAFT',
    state_machine: stateData,
    current_round: 0,
    max_rounds: 1,
    blackboard_state: undefined,
    divergence_scores: undefined,
    total_tokens: 0,
    estimated_cost: 0,
    suspended_reason: undefined,
    started_at: undefined,
    completed_at: undefined,
    error_message: undefined,
    created_at: v3.created_at,
    updated_at: v3.updated_at,
    members: mapStateMembers(stateData, v3.id, v3.created_at, v3.updated_at),
  }
}

function mapV3MessageToLegacy(row: TeamV3MessageRow): AgentTeamMessage {
  const payload = row.payload && typeof row.payload === 'object' ? row.payload : {}
  const messageType = String(row.message_type || 'chat').toLowerCase()
  const toolName = typeof payload.name === 'string'
    ? payload.name
    : (typeof payload.tool_name === 'string' ? payload.tool_name : 'unknown')
  const toolCallId = typeof payload.tool_call_id === 'string'
    ? payload.tool_call_id
    : row.id
  const content =
    typeof payload.content === 'string'
      ? payload.content
      : (typeof payload.message === 'string' ? payload.message : JSON.stringify(payload))
  const role = (() => {
    if (messageType === 'system' || messageType === 'status') return 'system'
    if (messageType === 'human_input' || messageType === 'user') return 'user'
    if (messageType === 'tool_call') return 'tool_call'
    if (messageType === 'tool_result') return 'tool_result'
    return 'assistant'
  })()
  const normalizedContent = (() => {
    if (role === 'tool_call') return content || `[Tool Call] ${toolName}`
    if (role === 'tool_result') return content || `[Tool Result] ${toolName}`
    return content
  })()
  const toolCalls = role === 'tool_call' || role === 'tool_result'
    ? [{
      id: toolCallId,
      name: toolName,
      arguments: payload.arguments ?? payload.tool_args,
      result: payload.result ?? payload.tool_result,
      success: payload.success !== false,
    }]
    : payload.tool_calls
  return {
    id: row.id,
    session_id: row.session_id,
    round_id: undefined,
    member_id: row.from_agent_id || undefined,
    member_name: row.from_agent_id || undefined,
    role,
    content: normalizedContent,
    tool_calls: toolCalls,
    token_count: undefined,
    timestamp: row.created_at,
  }
}

export const agentTeamApi = {
  async ensureSchema(): Promise<void> {
    return invoke('team_v3_ensure_schema')
  },

  async resetSchema(): Promise<void> {
    return invoke('team_v3_reset_schema')
  },

  async createSession(req: CreateAgentTeamSessionRequest): Promise<AgentTeamSession> {
    const v3 = await invoke<any>('team_v3_create_session', {
      request: {
        conversation_id: req.conversation_id ?? null,
        name: req.name,
        goal: req.goal ?? null,
      },
    })
    return mapV3SessionToLegacy(v3)
  },

  async getSession(id: string): Promise<AgentTeamSession | null> {
    const v3 = await invoke<any | null>('team_v3_get_session', { sessionId: id })
    return v3 ? mapV3SessionToLegacy(v3) : null
  },

  async listSessions(conversationId?: string, limit = 20, offset = 0): Promise<AgentTeamSession[]> {
    const list = await invoke<any[]>('team_v3_list_sessions', {
      conversationId: conversationId ?? null,
      limit,
      offset,
    })
    return list.map(mapV3SessionToLegacy)
  },

  async updateSession(id: string, req: UpdateAgentTeamSessionRequest): Promise<void> {
    return invoke('team_v3_update_session', {
      sessionId: id,
      request: {
        name: req.name ?? null,
        goal: req.goal ?? null,
        state: req.state ?? null,
        state_data: req.state_machine ?? null,
      },
    })
  },

  async listTasks(sessionId: string): Promise<TeamTask[]> {
    const tasks = await invoke<any[]>('team_v3_list_tasks', { sessionId })
    return tasks.map((t) => {
      const metadata = t?.metadata && typeof t.metadata === 'object' ? t.metadata : {}
      const dependsOn = Array.isArray(metadata.depends_on)
        ? metadata.depends_on.filter((value: unknown): value is string => typeof value === 'string' && value.trim().length > 0)
        : []
      const lastError = typeof metadata.last_error === 'string' ? metadata.last_error : null
      const startedAt = typeof metadata.started_at === 'string' ? metadata.started_at : null
      const completedAt = typeof metadata.completed_at === 'string' ? metadata.completed_at : null
      const attempt = Number.isFinite(Number(metadata.attempt)) ? Number(metadata.attempt) : 0
      const maxAttempts = Number.isFinite(Number(metadata.max_attempts)) ? Number(metadata.max_attempts) : 1
      return {
        id: t.id,
        session_id: t.session_id,
        task_id: t.task_key,
        title: t.title,
        instruction: t.instruction,
        status: t.status,
        assignee_agent_id: t.claimed_by_agent_id ?? t.owner_agent_id ?? null,
        depends_on: dependsOn,
        attempt,
        max_attempts: maxAttempts,
        last_error: lastError,
        started_at: startedAt,
        completed_at: completedAt,
        created_at: t.created_at,
        updated_at: t.updated_at,
      }
    })
  },

  async startRun(sessionId: string, conversationId?: string, ragEnabled?: boolean): Promise<void> {
    return invoke('team_v3_start_execution', {
      sessionId,
      conversationId: conversationId ?? null,
      ragEnabled: ragEnabled ?? null,
    })
  },

  async stopRun(sessionId: string): Promise<void> {
    return invoke('team_v3_stop_execution', { sessionId })
  },

  async finalizeRun(sessionId: string, success: boolean, summary?: string): Promise<void> {
    return invoke('team_v3_finalize_execution', {
      sessionId,
      success,
      summary: summary ?? null,
    })
  },

  async getRunStatus(sessionId: string): Promise<AgentTeamRunStatus | null> {
    const v3 = await invoke<any | null>('team_v3_get_run_status', { sessionId })
    if (!v3) return null
    return {
      session_id: v3.session_id,
      state: v3.state,
      current_round: 0,
      blackboard_snapshot: undefined,
      latest_message: undefined,
      divergence_score: undefined,
      is_suspended: v3.state === 'SUSPENDED_FOR_HUMAN',
    }
  },

  async getMessages(sessionId: string): Promise<AgentTeamMessage[]> {
    const rows = await invoke<TeamV3MessageRow[]>('team_v3_list_messages', { sessionId })
    return rows.map(mapV3MessageToLegacy)
  },

  async listBlackboardEntries(sessionId: string, limit = 100): Promise<TeamBlackboardEntry[]> {
    const rows = await invoke<TeamV3BlackboardRow[]>('team_v3_list_blackboard_entries', {
      sessionId,
      limit,
    })
    return rows.map((row) => ({
      id: row.id,
      session_id: row.session_id,
      task_id: row.task_id ?? undefined,
      agent_id: row.agent_id ?? undefined,
      entry_type: row.entry_type,
      content: row.content,
      metadata: row.metadata,
      created_at: row.created_at,
      updated_at: row.updated_at,
    }))
  },

  async submitMessage(req: SubmitAgentTeamMessageRequest): Promise<void> {
    return invoke('team_v3_send_message', {
      sessionId: req.session_id,
      request: {
        thread_id: req.session_id,
        from_agent_id: 'human',
        to_agent_id: null,
        message_type: 'human_input',
        payload: { content: req.content, resume: req.resume },
      },
    })
  },

}

export default agentTeamApi
