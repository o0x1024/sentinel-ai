import { invoke } from '@tauri-apps/api/core'
import type { AgentMessage } from '@/types/agent'

export const persistTeamV4Message = async (params: {
  conversationId: string | null
  message: AgentMessage
  role: 'user' | 'assistant'
}) => {
  if (!params.conversationId) return
  await invoke('save_ai_message', {
    request: {
      id: params.message.id,
      conversation_id: params.conversationId,
      role: params.role,
      content: params.message.content,
      metadata: params.message.metadata ?? null,
      architecture_type: 'team_v4',
      architecture_meta: JSON.stringify({
        team_run_id: params.message.metadata?.team_session_id ?? null,
        role: params.role,
      }),
      structured_data: null,
    },
  })
}
