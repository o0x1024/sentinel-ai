import { ref, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { ChatMessage, Conversation } from '../types/chat'

export const useConversation = () => {
  const conversations = ref<Conversation[]>([])
  const currentConversationId = ref<string | null>(null)
  const isLoadingConversations = ref(false)
  const messages = ref<ChatMessage[]>([])

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
      
      const historyMessages = (history as any[]).map((msg: any) => ({
        id: msg.id,
        role: msg.role,
        content: msg.content,
        timestamp: new Date(msg.timestamp),
        isStreaming: false
      }))
      
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

  // Clear current conversation
  const clearCurrentConversation = () => {
    messages.value = []
  }

  // Save messages to conversation
  const saveMessagesToConversation = async (messagesToSave: ChatMessage[]) => {
    if (!currentConversationId.value) return
    
    try {
      for (const message of messagesToSave) {
        await invoke('save_ai_message', {
          request: {
            conversation_id: currentConversationId.value,
            role: message.role,
            content: message.content
          }
        })
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