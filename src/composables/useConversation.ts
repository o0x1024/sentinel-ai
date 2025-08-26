import { ref, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'

interface ChatMessage {
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
}

interface Conversation {
  id: string
  title: string
  created_at: string
  total_messages: number
}

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
      
      const sessionKey = `ai_chat_session_${conversationId}`
      localStorage.removeItem(sessionKey)
      
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
    
    if (currentConversationId.value) {
      const sessionKey = `ai_chat_session_${currentConversationId.value}`
      localStorage.removeItem(sessionKey)
    }
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

  // Restore session state from localStorage
  const restoreSessionState = () => {
    const savedConversationId = localStorage.getItem('ai_chat_current_conversation_id')
    if (savedConversationId) {
      currentConversationId.value = savedConversationId
      
      const sessionKey = `ai_chat_session_${savedConversationId}`
      const savedMessages = localStorage.getItem(sessionKey)
      if (savedMessages) {
        try {
          const parsedMessages = JSON.parse(savedMessages)
          messages.value = parsedMessages.map((msg: any) => ({
            ...msg,
            timestamp: new Date(msg.timestamp)
          }))
        } catch (error) {
          console.error('Failed to restore session messages:', error)
          messages.value = []
        }
      }
    }
  }

  // Watch for conversation changes and persist to localStorage
  watch(currentConversationId, (newId) => {
    if (newId) {
      localStorage.setItem('ai_chat_current_conversation_id', newId)
    } else {
      localStorage.removeItem('ai_chat_current_conversation_id')
    }
  })

  // Watch for message changes and persist to localStorage
  watch(messages, (newMessages) => {
    if (currentConversationId.value) {
      const sessionKey = `ai_chat_session_${currentConversationId.value}`
      localStorage.setItem(sessionKey, JSON.stringify(newMessages))
    }
  }, { deep: true })

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
    restoreSessionState
  }
}