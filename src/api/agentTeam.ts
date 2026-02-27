/**
 * Agent Team API 层 - Tauri command 封装
 */
import { invoke } from '@tauri-apps/api/core'
import type {
    AgentTeamTemplate,
    AgentTeamSession,
    AgentTeamRound,
    AgentTeamMessage,
    AgentTeamBlackboardEntry,
    AgentTeamBlackboardArchive,
    AgentTeamArtifact,
    AgentTeamRunStatus,
    TeamTask,
    MailboxMessage,
    UpdateTaskRequest,
    CreateAgentTeamTemplateRequest,
    UpdateAgentTeamTemplateRequest,
    CreateAgentTeamSessionRequest,
    UpdateAgentTeamSessionRequest,
    UpdateBlackboardRequest,
    SubmitAgentTeamMessageRequest,
    AppendAgentTeamPartialMessageRequest,
} from '@/types/agentTeam'

// ==================== Template CRUD ====================

export const agentTeamApi = {
    // Templates
    async createTemplate(req: CreateAgentTeamTemplateRequest): Promise<AgentTeamTemplate> {
        return invoke('agent_team_create_template', { request: req })
    },

    async listTemplates(domain?: string): Promise<AgentTeamTemplate[]> {
        return invoke('agent_team_list_templates', { domain: domain ?? null })
    },

    async getTemplate(id: string): Promise<AgentTeamTemplate | null> {
        return invoke('agent_team_get_template', { templateId: id })
    },

    async updateTemplate(id: string, req: UpdateAgentTeamTemplateRequest): Promise<void> {
        return invoke('agent_team_update_template', { templateId: id, request: req })
    },

    async deleteTemplate(id: string): Promise<void> {
        return invoke('agent_team_delete_template', { templateId: id })
    },

    async seedBuiltinTemplates(): Promise<void> {
        return invoke('agent_team_seed_builtin_templates')
    },

    // Sessions
    async createSession(req: CreateAgentTeamSessionRequest): Promise<AgentTeamSession> {
        return invoke('agent_team_create_session', { request: req })
    },

    async getSession(id: string): Promise<AgentTeamSession | null> {
        return invoke('agent_team_get_session', { sessionId: id })
    },

    async listSessions(conversationId?: string, limit = 20, offset = 0): Promise<AgentTeamSession[]> {
        return invoke('agent_team_list_sessions', {
            conversationId: conversationId ?? null,
            limit,
            offset,
        })
    },

    async updateSession(id: string, req: UpdateAgentTeamSessionRequest): Promise<void> {
        return invoke('agent_team_update_session', { sessionId: id, request: req })
    },

    async deleteSession(id: string): Promise<void> {
        return invoke('agent_team_delete_session', { sessionId: id })
    },

    async listTasks(sessionId: string): Promise<TeamTask[]> {
        return invoke('agent_team_list_tasks', { sessionId })
    },

    async updateTask(
        sessionId: string,
        taskId: string,
        patch: Omit<UpdateTaskRequest, 'task_id'>,
    ): Promise<void> {
        return invoke('agent_team_update_task', {
            sessionId,
            taskId,
            patch: {
                task_id: taskId,
                ...patch,
            },
        })
    },

    async listMailbox(sessionId: string, agentId?: string): Promise<MailboxMessage[]> {
        return invoke('agent_team_list_mailbox', {
            sessionId,
            agentId: agentId ?? null,
        })
    },

    async ackMailbox(messageId: string): Promise<void> {
        return invoke('agent_team_ack_mailbox', { messageId })
    },

    async upgradeTemplatesToV2(force = false): Promise<number> {
        return invoke('agent_team_upgrade_templates_to_v2', { force })
    },

    // Run lifecycle
    async startRun(sessionId: string): Promise<void> {
        return invoke('agent_team_start_run', { sessionId })
    },

    async stopRun(sessionId: string): Promise<void> {
        return invoke('agent_team_stop_run', { sessionId })
    },

    async getRunStatus(sessionId: string): Promise<AgentTeamRunStatus | null> {
        return invoke('agent_team_get_run_status', { sessionId })
    },

    // Messages
    async getMessages(sessionId: string): Promise<AgentTeamMessage[]> {
        return invoke('agent_team_get_messages', { sessionId })
    },

    async getRounds(sessionId: string): Promise<AgentTeamRound[]> {
        return invoke('agent_team_get_rounds', { sessionId })
    },

    async submitMessage(req: SubmitAgentTeamMessageRequest): Promise<void> {
        return invoke('agent_team_submit_message', { request: req })
    },

    async appendPartialMessage(req: AppendAgentTeamPartialMessageRequest): Promise<void> {
        return invoke('agent_team_append_partial_message', { request: req })
    },

    // Blackboard
    async getBlackboard(sessionId: string): Promise<AgentTeamBlackboardEntry[]> {
        return invoke('agent_team_get_blackboard', { sessionId })
    },

    async addBlackboardEntry(req: UpdateBlackboardRequest): Promise<AgentTeamBlackboardEntry> {
        return invoke('agent_team_add_blackboard_entry', { request: req })
    },

    async resolveBlackboardEntry(sessionId: string, entryId: string): Promise<AgentTeamBlackboardEntry> {
        return invoke('agent_team_resolve_blackboard_entry', { sessionId, entryId })
    },

    async getBlackboardEntryArchive(
        sessionId: string,
        entryId: string,
        limit = 80,
    ): Promise<AgentTeamBlackboardArchive> {
        return invoke('agent_team_get_blackboard_entry_archive', { sessionId, entryId, limit })
    },

    // Artifacts
    async listArtifacts(sessionId: string): Promise<AgentTeamArtifact[]> {
        return invoke('agent_team_list_artifacts', { sessionId })
    },

    async getArtifact(artifactId: string): Promise<AgentTeamArtifact | null> {
        return invoke('agent_team_get_artifact', { artifactId })
    },

    // AI Template Generation
    async generateTemplate(req: {
        description: string
        domain?: string | null
        agent_count?: number
    }): Promise<{
        name: string
        description: string
        domain: string
        agents: Array<{
            name: string
            responsibility: string
            system_prompt: string
            decision_style: string
            risk_preference: string
            weight: number
        }>
        raw_json: string
    }> {
        return invoke('agent_team_generate_template', { request: req })
    },

    async saveGeneratedTemplate(generated: {
        name: string
        description: string
        domain: string
        agents: Array<{
            name: string
            responsibility: string
            system_prompt: string
            decision_style: string
            risk_preference: string
            weight: number
        }>
        raw_json: string
    }): Promise<AgentTeamTemplate> {
        return invoke('agent_team_save_generated_template', { generated })
    },
}

export default agentTeamApi
