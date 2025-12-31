import { defineStore } from 'pinia'
import { ref } from 'vue'
import type { 
  PluginRecord, NewPluginMetadata, AiChatMessage, 
  CodeReference, TestResultReference 
} from '../components/PluginManagement/types'

export const usePluginEditorStore = defineStore('pluginEditor', () => {
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

  // 操作
  const openEditor = (plugin: PluginRecord | null = null, code: string = '', metadata?: NewPluginMetadata) => {
    isOpen.value = true
    isMinimized.value = false
    editingPlugin.value = plugin
    pluginCode.value = code
    originalCode.value = code
    
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
    } else {
      // 重置为新插件
      newPluginMetadata.value = {
        id: '', name: '', version: '1.0.0', author: '',
        mainCategory: 'traffic', category: 'vulnerability',
        default_severity: 'medium', description: '', tagsString: ''
      }
    }
  }

  const closeEditor = () => {
    isOpen.value = false
    isMinimized.value = false
    editingPlugin.value = null
    pluginCode.value = ''
    aiChatMessages.value = []
    // 其他重置逻辑...
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

  return {
    isOpen, isMinimized, isFullscreen,
    editingPlugin, pluginCode, originalCode, isEditing, saving, codeError,
    newPluginMetadata,
    showAiPanel, aiChatMessages, aiChatStreaming, aiChatStreamingContent,
    selectedCodeRef, selectedTestResultRef, pluginTesting,
    isPreviewMode, previewCode,
    openEditor, closeEditor, minimizeEditor, restoreEditor, toggleFullscreen
  }
})
