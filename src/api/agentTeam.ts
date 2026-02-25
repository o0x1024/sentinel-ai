/**
 * Agent Team API 层 - Tauri command 封装
 */
import { invoke } from '@tauri-apps/api/core'
import type {
    AgentTeamTemplate,
    AgentTeamSession,
    AgentTeamMessage,
    AgentTeamBlackboardEntry,
    AgentTeamArtifact,
    AgentTeamRunStatus,
    CreateAgentTeamTemplateRequest,
    UpdateAgentTeamTemplateRequest,
    CreateAgentTeamSessionRequest,
    UpdateAgentTeamSessionRequest,
    UpdateBlackboardRequest,
    SubmitAgentTeamMessageRequest,
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

    async listSessions(conversationId?: string, limit = 20): Promise<AgentTeamSession[]> {
        return invoke('agent_team_list_sessions', {
            conversationId: conversationId ?? null,
            limit,
        })
    },

    async updateSession(id: string, req: UpdateAgentTeamSessionRequest): Promise<void> {
        return invoke('agent_team_update_session', { sessionId: id, request: req })
    },

    // Run lifecycle
    async startRun(sessionId: string): Promise<void> {
        return invoke('agent_team_start_run', { sessionId })
    },

    async getRunStatus(sessionId: string): Promise<AgentTeamRunStatus | null> {
        return invoke('agent_team_get_run_status', { sessionId })
    },

    // Messages
    async getMessages(sessionId: string): Promise<AgentTeamMessage[]> {
        return invoke('agent_team_get_messages', { sessionId })
    },

    async submitMessage(req: SubmitAgentTeamMessageRequest): Promise<void> {
        return invoke('agent_team_submit_message', { request: req })
    },

    // Blackboard
    async getBlackboard(sessionId: string): Promise<AgentTeamBlackboardEntry[]> {
        return invoke('agent_team_get_blackboard', { sessionId })
    },

    async addBlackboardEntry(req: UpdateBlackboardRequest): Promise<AgentTeamBlackboardEntry> {
        return invoke('agent_team_add_blackboard_entry', { request: req })
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
        role_count?: number
    }): Promise<{
        name: string
        description: string
        domain: string
        members: Array<{
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
        members: Array<{
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
