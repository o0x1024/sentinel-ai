import { ref, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { ChatMessage, Conversation } from '../types/chat'
import type { SimplifiedChatMessage } from '../types/ordered-chat'

export const useConversation = () => {
  const conversations = ref<Conversation[]>([])
  const currentConversationId = ref<string | null>(null)
  const isLoadingConversations = ref(false)
  const messages = ref<ChatMessage[]>([])

  // 记录已持久化的消息ID，按会话维度去重，避免重复保存
  const savedMessageIdsByConversation = new Map<string, Set<string>>()

  // Create new conversation
  const createNewConversation = async () => {
    isLoadingConversations.value = true
    try {
      const result = await invoke('create_ai_conversation', {
        request: {
          title: `AI会话 ${new Date().toLocaleString()}`,
          service_name: "default"
        }
      })
      
      currentConversationId.value = result as string
      messages.value = []
      
      await loadConversations()
      return result as string
    } catch (error) {
      console.error('Failed to create new conversation:', error)
      throw error
    } finally {
      isLoadingConversations.value = false
    }
  }

  // Load conversations list
  const loadConversations = async () => {
    isLoadingConversations.value = true
    try {
      const result = await invoke('get_ai_conversations')
      conversations.value = result as Conversation[]
      console.log('conversations', conversations.value)
    } catch (error) {
      console.error('Failed to load conversations:', error)
      conversations.value = []
    } finally {
      isLoadingConversations.value = false
    }
  }

  // Switch to conversation
  const switchToConversation = async (conversationId: string) => {
    try {
      currentConversationId.value = conversationId
      
      const history = await invoke('get_ai_conversation_history', {
        conversation_id: conversationId,
        service_name: "default"
      })
      
      const historyMessages = (history as any[])
        .map((msg: any) => {
          let citations = undefined
          let architectureType = undefined
          let architectureMeta = undefined
          let reactSteps = undefined
          let llmCompilerData = undefined
          let llmCompilerFinalResponse = undefined
          let planAndExecuteData = undefined
          let rewooData = undefined
          let travelData = undefined
          
          // 解析metadata
          try {
            if (msg.metadata) {
              const meta = typeof msg.metadata === 'string' ? JSON.parse(msg.metadata) : msg.metadata
              if (meta) {
                if (Array.isArray(meta.citations)) {
                  citations = meta.citations
                }
              }
            }
          } catch (e) {
            console.warn('[useConversation] Failed to parse metadata:', e)
          }
          
          // 恢复架构类型
          if (msg.architecture_type) {
            architectureType = msg.architecture_type
          }
          
          // 恢复架构元数据
          if (msg.architecture_meta) {
            try {
              architectureMeta = typeof msg.architecture_meta === 'string' 
                ? JSON.parse(msg.architecture_meta) 
                : msg.architecture_meta
            } catch (e) {
              console.warn('[useConversation] Failed to parse architecture_meta:', e)
            }
          }
          
          // 恢复结构化数据
          if (msg.structured_data) {
            try {
              const data = typeof msg.structured_data === 'string' 
                ? JSON.parse(msg.structured_data) 
                : msg.structured_data
              
              if (data) {
                reactSteps = data.reactSteps
                llmCompilerData = data.llmCompilerData
                llmCompilerFinalResponse = data.llmCompilerFinalResponse
                planAndExecuteData = data.planAndExecuteData
                rewooData = data.rewooData
                travelData = data.travelData
                
                if (reactSteps) {
                  console.log('[useConversation] Restored reactSteps for message:', msg.id, 'steps:', reactSteps.length)
                }
                if (llmCompilerFinalResponse) {
                  console.log('[useConversation] Restored llmCompilerFinalResponse for message:', msg.id)
                }
              }
            } catch (e) {
              console.warn('[useConversation] Failed to parse structured_data:', e)
            }
          }
          
          return {
            id: msg.id,
            role: msg.role,
            content: msg.content,
            timestamp: new Date(msg.timestamp),
            isStreaming: false,
            citations,
            architectureType,
            architectureMeta,
            reactSteps,
            llmCompilerData,
            llmCompilerFinalResponse,
            planAndExecuteData,
            rewooData,
            travelData,
          }
        })
        // Ensure messages are in ascending time order so newest appears at bottom
        .sort((a, b) => (a.timestamp as any) - (b.timestamp as any))
      
      messages.value = historyMessages
      
      return historyMessages
    } catch (error) {
      console.error('Failed to switch conversation:', error)
      throw error
    }
  }

  // Delete conversation
  const deleteConversation = async (conversationId: string) => {
    try {
      await invoke('delete_ai_conversation', {
        conversationId: conversationId,
        serviceName: "default"
      })
      
      if (conversationId === currentConversationId.value) {
        currentConversationId.value = null
        messages.value = []
      }
      
      await loadConversations()
    } catch (error) {
      console.error('Failed to delete conversation:', error)
      throw error
    }
  }

  // Clear current conversation (delete from DB and refresh)
  const clearCurrentConversation = async () => {
    if (!currentConversationId.value) return
    try {
      await invoke('delete_ai_conversation', {
        conversationId: currentConversationId.value,
        serviceName: 'default',
      })
      currentConversationId.value = null
      messages.value = []
      await loadConversations()
    } catch (error) {
      console.error('Failed to clear current conversation:', error)
      throw error
    }
  }

  // Save messages to conversation (支持新旧消息类型)
  const saveMessagesToConversation = async (messagesToSave: ChatMessage[] | SimplifiedChatMessage[]) => {
    if (!currentConversationId.value) return
    
    try {
      const convId = currentConversationId.value
      const savedSet = savedMessageIdsByConversation.get(convId) || new Set<string>()

      for (const message of messagesToSave) {
        const id = (message as any)?.id as string | undefined
        const role = (message as any)?.role
        const content = (message as any)?.content
        if (!id || savedSet.has(id)) continue

        // 提取架构相关数据
        const architectureType = (message as any)?.architectureType
        const architectureMeta = (message as any)?.architectureMeta
        
        // 提取结构化数据（各架构特定数据）
        const structuredData: any = {}
        if ((message as any)?.reactSteps) {
          structuredData.reactSteps = (message as any).reactSteps
        }
        if ((message as any)?.llmCompilerData) {
          structuredData.llmCompilerData = (message as any).llmCompilerData
        }
        // 保存LLMCompiler的最终响应内容
        if ((message as any)?.llmCompilerFinalResponse) {
          structuredData.llmCompilerFinalResponse = (message as any).llmCompilerFinalResponse
        }
        if ((message as any)?.planAndExecuteData) {
          structuredData.planAndExecuteData = (message as any).planAndExecuteData
        }
        if ((message as any)?.rewooData) {
          structuredData.rewooData = (message as any).rewooData
        }
        if ((message as any)?.travelData) {
          structuredData.travelData = (message as any).travelData
        }

        // 将 citations 等其他数据嵌入 metadata
        const metadata: any = {}
        if ((message as any)?.citations) {
          metadata.citations = (message as any).citations
        }
        
        console.log('[useConversation] Saving message with architecture:', architectureType, 'message_id:', id)
        
        await invoke('save_ai_message', {
          request: {
            id: id,
            conversation_id: convId,
            role,
            content,
            metadata: Object.keys(metadata).length > 0 ? metadata : undefined,
            architecture_type: architectureType,
            architecture_meta: architectureMeta ? JSON.stringify(architectureMeta) : undefined,
            structured_data: Object.keys(structuredData).length > 0 ? JSON.stringify(structuredData) : undefined,
          }
        })

        savedSet.add(id)
      }

      if (!savedMessageIdsByConversation.has(convId)) {
        savedMessageIdsByConversation.set(convId, savedSet)
      }
    } catch (error) {
      console.error('Failed to save messages to conversation:', error)
      throw error
    }
  }

  // Get current conversation title
  const getCurrentConversationTitle = () => {
    if (!currentConversationId.value) return '新会话'
    
    const conv = conversations.value.find(c => c.id === currentConversationId.value)
    return conv?.title || '无标题会话'
  }

  return {
    conversations,
    currentConversationId,
    isLoadingConversations,
    messages,
    createNewConversation,
    loadConversations,
    switchToConversation,
    deleteConversation,
    clearCurrentConversation,
    saveMessagesToConversation,
    getCurrentConversationTitle,
  }
}