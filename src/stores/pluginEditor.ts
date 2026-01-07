import { defineStore } from 'pinia'
import { ref, watch } from 'vue'
import type { 
  PluginRecord, NewPluginMetadata, AiChatMessage, 
  CodeReference, TestResultReference 
} from '../components/PluginManagement/types'

// 对话历史持久化接口
interface ChatHistoryEntry {
  pluginId: string
  messages: AiChatMessage[]
  lastUpdated: number
}

interface ChatHistoryStorage {
  entries: ChatHistoryEntry[]
  maxEntries: number
}

export const usePluginEditorStore = defineStore('pluginEditor', () => {
  const STORAGE_KEY = 'sentinel_plugin_chat_history'
  const MAX_HISTORY_ENTRIES = 50 // 最多保存50个插件的对话历史
  const MAX_MESSAGES_PER_PLUGIN = 100 // 每个插件最多保存100条消息

  // 窗口状态
  const isOpen = ref(false)
  const isMinimized = ref(false)
  const isFullscreen = ref(false)

  // 编辑数据
  const editingPlugin = ref<PluginRecord | null>(null)
  const pluginCode = ref('')
  const originalCode = ref('')
  const isEditing = ref(false)
  const saving = ref(false)
  const codeError = ref('')

  // 插件元数据
  const newPluginMetadata = ref<NewPluginMetadata>({
    id: '', name: '', version: '1.0.0', author: '',
    mainCategory: 'traffic', category: 'vulnerability',
    default_severity: 'medium', description: '', tagsString: ''
  })

  // AI 助手状态
  const showAiPanel = ref(true)
  const aiChatMessages = ref<AiChatMessage[]>([])
  const aiChatStreaming = ref(false)
  const aiChatStreamingContent = ref('')
  const selectedCodeRef = ref<CodeReference | null>(null)
  const selectedTestResultRef = ref<TestResultReference | null>(null)
  const pluginTesting = ref(false)
  const isPreviewMode = ref(false)
  const previewCode = ref('')
  
  // 对话历史管理
  const chatHistoryMap = ref<Map<string, AiChatMessage[]>>(new Map())

  // 持久化相关方法
  const loadChatHistoryFromStorage = () => {
    try {
      const stored = localStorage.getItem(STORAGE_KEY)
      if (stored) {
        const storage: ChatHistoryStorage = JSON.parse(stored)
        storage.entries.forEach(entry => {
          chatHistoryMap.value.set(entry.pluginId, entry.messages)
        })
      }
    } catch (error) {
      console.error('Failed to load chat history from storage:', error)
    }
  }

  const saveChatHistoryToStorage = () => {
    try {
      const entries: ChatHistoryEntry[] = Array.from(chatHistoryMap.value.entries())
        .map(([pluginId, messages]) => ({
          pluginId,
          messages: messages.slice(-MAX_MESSAGES_PER_PLUGIN), // 只保留最近的消息
          lastUpdated: Date.now()
        }))
        .sort((a, b) => b.lastUpdated - a.lastUpdated) // 按时间排序
        .slice(0, MAX_HISTORY_ENTRIES) // 只保留最近的N个插件

      const storage: ChatHistoryStorage = {
        entries,
        maxEntries: MAX_HISTORY_ENTRIES
      }
      
      localStorage.setItem(STORAGE_KEY, JSON.stringify(storage))
    } catch (error) {
      console.error('Failed to save chat history to storage:', error)
    }
  }

  const loadChatHistory = (pluginId: string) => {
    const history = chatHistoryMap.value.get(pluginId)
    if (history) {
      aiChatMessages.value = [...history]
    } else {
      aiChatMessages.value = []
    }
  }

  const saveChatHistory = (pluginId: string) => {
    // 总是保存，即使是空数组（用于清除历史的场景）
    chatHistoryMap.value.set(pluginId, [...aiChatMessages.value])
    saveChatHistoryToStorage()
  }

  const clearChatHistory = (pluginId?: string) => {
    if (pluginId) {
      chatHistoryMap.value.delete(pluginId)
    } else {
      chatHistoryMap.value.clear()
    }
    saveChatHistoryToStorage()
  }

  // 操作
  const openEditor = (plugin: PluginRecord | null = null, code: string = '', metadata?: NewPluginMetadata) => {
    isOpen.value = true
    isMinimized.value = false
    editingPlugin.value = plugin
    pluginCode.value = code
    originalCode.value = code
    
    // 如果是编辑现有插件，默认处于编辑状态
    isEditing.value = plugin !== null
    
    if (metadata) {
      newPluginMetadata.value = metadata
    } else if (plugin) {
      newPluginMetadata.value = {
        id: plugin.metadata.id,
        name: plugin.metadata.name,
        version: plugin.metadata.version,
        author: plugin.metadata.author || '',
        mainCategory: plugin.metadata.main_category,
        category: plugin.metadata.category,
        default_severity: plugin.metadata.default_severity,
        description: plugin.metadata.description || '',
        tagsString: plugin.metadata.tags.join(', ')
      }
      // 加载该插件的对话历史
      loadChatHistory(plugin.metadata.id)
    } else {
      // 重置为新插件
      newPluginMetadata.value = {
        id: '', name: '', version: '1.0.0', author: '',
        mainCategory: 'traffic', category: 'vulnerability',
        default_severity: 'medium', description: '', tagsString: ''
      }
      aiChatMessages.value = []
    }
  }

  const closeEditor = () => {
    // 保存当前插件的对话历史
    if (editingPlugin.value && aiChatMessages.value.length > 0) {
      saveChatHistory(editingPlugin.value.metadata.id)
    }
    
    isOpen.value = false
    isMinimized.value = false
    editingPlugin.value = null
    pluginCode.value = ''
    originalCode.value = ''
    isEditing.value = true
    saving.value = false
    codeError.value = ''
    aiChatMessages.value = []
    selectedCodeRef.value = null
    selectedTestResultRef.value = null
    isPreviewMode.value = false
    previewCode.value = ''
  }

  const minimizeEditor = () => {
    isMinimized.value = true
    // Keep isOpen true so the editor state persists
  }

  const restoreEditor = () => {
    isMinimized.value = false
    isOpen.value = true
  }

  const toggleFullscreen = () => {
    isFullscreen.value = !isFullscreen.value
  }

  // 初始化时加载历史
  loadChatHistoryFromStorage()

  // 监听 aiChatMessages 变化，自动保存
  // 使用防抖避免频繁保存
  let saveTimeout: ReturnType<typeof setTimeout> | null = null
  watch(aiChatMessages, () => {
    if (editingPlugin.value) {
      // 清除之前的定时器
      if (saveTimeout) {
        clearTimeout(saveTimeout)
      }
      // 延迟500ms保存，避免频繁写入
      saveTimeout = setTimeout(() => {
        if (editingPlugin.value) {
          saveChatHistory(editingPlugin.value.metadata.id)
        }
        saveTimeout = null
      }, 500)
    }
  }, { deep: true })

  return {
    isOpen, isMinimized, isFullscreen,
    editingPlugin, pluginCode, originalCode, isEditing, saving, codeError,
    newPluginMetadata,
    showAiPanel, aiChatMessages, aiChatStreaming, aiChatStreamingContent,
    selectedCodeRef, selectedTestResultRef, pluginTesting,
    isPreviewMode, previewCode,
    openEditor, closeEditor, minimizeEditor, restoreEditor, toggleFullscreen,
    // 对话历史管理方法
    loadChatHistory, saveChatHistory, clearChatHistory
  }
})
