<template>
  <div>
    <!-- The actual dialog -->
    <PluginCodeEditorDialog
      v-if="store.isOpen"
      ref="dialogRef"
      :editing-plugin="store.editingPlugin"
      :new-plugin-metadata="store.newPluginMetadata"
      :is-editing="store.isEditing"
      :saving="store.saving"
      :code-error="store.codeError"
      :is-fullscreen-editor="store.isFullscreen"
      :is-minimized="store.isMinimized"
      :sub-categories="subCategories"
      :show-ai-panel="store.showAiPanel"
      :ai-messages="store.aiChatMessages"
      :ai-streaming="store.aiChatStreaming"
      :ai-streaming-content="store.aiChatStreamingContent"
      :selected-code-ref="store.selectedCodeRef"
      :selected-test-result-ref="store.selectedTestResultRef"
      :plugin-testing="store.pluginTesting"
      :is-preview-mode="store.isPreviewMode"
      @update:new-plugin-metadata="store.newPluginMetadata = $event"
      @insert-template="handleInsertTemplate"
      @format-code="handleFormatCode"
      @copy-plugin="handleCopyPlugin"
      @toggle-fullscreen="handleToggleFullscreen"
      @enable-editing="handleEnableEditing"
      @cancel-editing="handleCancelEditing"
      @save-plugin="handleSavePlugin"
      @create-new-plugin="handleCreateNewPlugin"
      @close="store.closeEditor"
      @minimize="store.minimizeEditor"
      @toggle-ai-panel="store.showAiPanel = !store.showAiPanel"
      @send-ai-message="handleSendAiMessage"
      @ai-quick-action="handleAiQuickAction"
      @apply-ai-code="handleApplyAiCode"
      @preview-ai-code="handlePreviewAiCode"
      @exit-preview-mode="handleExitPreviewMode"
      @test-current-plugin="handleTestPlugin"
    />

    <!-- Minimized Indicator -->
    <div v-if="store.isMinimized" 
      class="fixed bottom-4 right-4 z-[9999]"
      @click="store.restoreEditor">
      <div class="flex items-center gap-3 p-3 bg-primary text-primary-content rounded-2xl shadow-2xl cursor-pointer hover:scale-105 transition-transform border-2 border-primary-content/20 animate-bounce-in">
        <div class="relative">
          <i :class="store.isFullscreen ? 'fas fa-expand text-xl' : 'fas fa-code-branch text-xl'"></i>
          <span class="absolute -top-1 -right-1 flex h-3 w-3">
            <span class="animate-ping absolute inline-flex h-full w-full rounded-full bg-secondary opacity-75"></span>
            <span class="relative inline-flex rounded-full h-3 w-3 bg-secondary"></span>
          </span>
        </div>
        <div class="flex flex-col">
          <span class="text-xs font-bold opacity-70 uppercase tracking-wider">
            {{ store.isFullscreen ? $t('plugins.fullscreenMinimized', '全屏编辑器已缩小') : $t('plugins.editorMinimized', '编辑器已缩小') }}
          </span>
          <span class="text-sm font-bold truncate max-w-[150px]">
            {{ store.editingPlugin ? store.editingPlugin.metadata.name : $t('plugins.newPlugin', '新插件') }}
          </span>
        </div>
        <button class="btn btn-circle btn-xs btn-ghost ml-2" @click.stop="store.closeEditor">
          <i class="fas fa-times"></i>
        </button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch, nextTick, onUnmounted } from 'vue'
import { usePluginEditorStore } from '../../stores/pluginEditor'
import PluginCodeEditorDialog from './PluginCodeEditorDialog.vue'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { useI18n } from 'vue-i18n'
import type { SubCategory, CodeReference, CommandResponse, TestResult } from './types'
import { marked } from 'marked'
import DOMPurify from 'dompurify'
import { EditorView, type ViewUpdate } from '@codemirror/view'
import { Compartment } from '@codemirror/state'
import { basicSetup } from 'codemirror'
import { javascript } from '@codemirror/lang-javascript'
import { oneDark } from '@codemirror/theme-one-dark'

const store = usePluginEditorStore()
const { t } = useI18n()
const dialogRef = ref<InstanceType<typeof PluginCodeEditorDialog>>()

// CodeMirror Instances
let codeEditorView: EditorView | null = null
let fullscreenCodeEditorView: EditorView | null = null
let diffEditorViewA: EditorView | null = null
let diffEditorViewB: EditorView | null = null
const codeEditorReadOnly = new Compartment()

// Subcategories logic
const subCategories = computed<SubCategory[]>(() => {
  if (store.newPluginMetadata.mainCategory === 'traffic') {
    return [
      { value: 'sqli', label: 'SQL注入', icon: 'fas fa-database' },
      { value: 'command_injection', label: '命令注入', icon: 'fas fa-terminal' },
      { value: 'xss', label: '跨站脚本', icon: 'fas fa-code' },
      { value: 'idor', label: '越权访问', icon: 'fas fa-user-lock' },
      { value: 'auth_bypass', label: '认证绕过', icon: 'fas fa-unlock' },
      { value: 'csrf', label: 'CSRF', icon: 'fas fa-shield-alt' },
      { value: 'info_leak', label: '信息泄露', icon: 'fas fa-eye-slash' },
      { value: 'file_upload', label: '文件上传', icon: 'fas fa-file-upload' },
      { value: 'file_inclusion', label: '文件包含', icon: 'fas fa-file-code' },
      { value: 'path_traversal', label: '目录穿越', icon: 'fas fa-folder-open' },
      { value: 'xxe', label: 'XXE', icon: 'fas fa-file-code' },
      { value: 'ssrf', label: 'SSRF', icon: 'fas fa-server' },
      { value: 'custom', label: '自定义', icon: 'fas fa-wrench' }
    ]
  } else if (store.newPluginMetadata.mainCategory === 'agent') {
    return [
      { value: 'scanner', label: '扫描工具', icon: 'fas fa-radar' },
      { value: 'analyzer', label: '分析工具', icon: 'fas fa-microscope' },
      { value: 'reporter', label: '报告工具', icon: 'fas fa-file-alt' },
      { value: 'recon', label: '信息收集', icon: 'fas fa-search' },
      { value: 'exploit', label: '漏洞利用', icon: 'fas fa-bomb' },
      { value: 'utility', label: '实用工具', icon: 'fas fa-toolbox' },
      { value: 'custom', label: '自定义', icon: 'fas fa-wrench' }
    ]
  }
  return []
})

const showToast = (message: string, type: 'success' | 'error' | 'info' | 'warning' = 'success') => {
  const toast = document.createElement('div')
  toast.className = 'toast toast-top toast-end z-[99999]'
  toast.style.top = '5rem'
  const alertClass = { success: 'alert-success', error: 'alert-error', info: 'alert-info', warning: 'alert-warning' }[type]
  const icon = { success: 'fa-check-circle', error: 'fa-times-circle', info: 'fa-info-circle', warning: 'fa-exclamation-triangle' }[type]
  toast.innerHTML = `<div class="alert ${alertClass} shadow-lg"><i class="fas ${icon}"></i><span>${message}</span></div>`
  document.body.appendChild(toast)
  setTimeout(() => toast.remove(), 3000)
}

// Editor Initialization
const initCodeEditor = () => {
  const container = dialogRef.value?.codeEditorContainerRef
  if (!container) return

  if (codeEditorView) {
    codeEditorView.destroy()
    codeEditorView = null
  }

  const isReadOnly = store.editingPlugin ? !store.isEditing : false

  codeEditorView = new EditorView({
    doc: store.pluginCode,
    extensions: [
      basicSetup,
      javascript(),
      oneDark,
      EditorView.lineWrapping,
      codeEditorReadOnly.of(EditorView.editable.of(!isReadOnly)),
      EditorView.updateListener.of((update: ViewUpdate) => {
        if (update.docChanged) store.pluginCode = update.state.doc.toString()
      })
    ],
    parent: container
  })
}

const initFullscreenCodeEditor = () => {
  const container = dialogRef.value?.fullscreenCodeEditorContainerRef
  if (!container) return

  if (fullscreenCodeEditorView) {
    fullscreenCodeEditorView.destroy()
    fullscreenCodeEditorView = null
  }

  const isReadOnly = store.editingPlugin ? !store.isEditing : false

  fullscreenCodeEditorView = new EditorView({
    doc: store.pluginCode,
    extensions: [
      basicSetup,
      javascript(),
      oneDark,
      EditorView.lineWrapping,
      codeEditorReadOnly.of(EditorView.editable.of(!isReadOnly)),
      EditorView.updateListener.of((update: ViewUpdate) => {
        if (update.docChanged) store.pluginCode = update.state.doc.toString()
      })
    ],
    parent: container
  })
}

const initDiffEditor = () => {
  const container = dialogRef.value?.fullscreenDiffEditorContainerRef
  if (!container) return
  
  container.innerHTML = ''
  
  if (diffEditorViewA) { diffEditorViewA.destroy(); diffEditorViewA = null }
  if (diffEditorViewB) { diffEditorViewB.destroy(); diffEditorViewB = null }
  
  const wrapper = document.createElement('div')
  wrapper.style.cssText = 'display: flex; height: calc(100% - 3rem); width: 100%;'
  
  const leftContainer = document.createElement('div')
  leftContainer.style.cssText = 'flex: 1; border-right: 2px solid oklch(var(--bc) / 0.2);'
  
  const rightContainer = document.createElement('div')
  rightContainer.style.cssText = 'flex: 1;'
  
  wrapper.appendChild(leftContainer)
  wrapper.appendChild(rightContainer)
  container.appendChild(wrapper)
  
  diffEditorViewA = new EditorView({
    doc: store.pluginCode,
    extensions: [basicSetup, javascript(), oneDark, EditorView.lineWrapping, EditorView.editable.of(false)],
    parent: leftContainer
  })
  
  diffEditorViewB = new EditorView({
    doc: store.previewCode,
    extensions: [
      basicSetup, javascript(), oneDark, EditorView.lineWrapping,
      EditorView.updateListener.of((update: ViewUpdate) => {
        if (update.docChanged) store.previewCode = update.state.doc.toString()
      })
    ],
    parent: rightContainer
  })
}

const updateEditorsContent = (content: string) => {
  if (codeEditorView) {
    codeEditorView.dispatch({ changes: { from: 0, to: codeEditorView.state.doc.length, insert: content } })
  }
  if (fullscreenCodeEditorView) {
    fullscreenCodeEditorView.dispatch({ changes: { from: 0, to: fullscreenCodeEditorView.state.doc.length, insert: content } })
  }
}

const updateEditorsReadonly = (readonly: boolean) => {
  if (codeEditorView) {
    codeEditorView.dispatch({ effects: codeEditorReadOnly.reconfigure(EditorView.editable.of(!readonly)) })
  }
  if (fullscreenCodeEditorView) {
    fullscreenCodeEditorView.dispatch({ effects: codeEditorReadOnly.reconfigure(EditorView.editable.of(!readonly)) })
  }
}

// Handlers
const handleInsertTemplate = async () => {
  const isAgentPlugin = store.newPluginMetadata.mainCategory === 'agent'
  try {
    const templateType = isAgentPlugin ? 'agent' : 'traffic'
    const combinedTemplate = await invoke<string>('get_combined_plugin_prompt_api', {
      pluginType: templateType,
      vulnType: store.newPluginMetadata.category || 'custom',
      severity: store.newPluginMetadata.default_severity || 'medium'
    })

    let codeTemplate = ''
    const patterns = [/```typescript\n([\s\S]*?)\n```/, /```ts\n([\s\S]*?)\n```/, /```javascript\n([\s\S]*?)\n```/]
    for (const pattern of patterns) {
      const match = combinedTemplate.match(pattern)
      if (match) {
        codeTemplate = match[1].trim()
        break
      }
    }

    if (!codeTemplate) {
      codeTemplate = isAgentPlugin ? getAgentFallbackTemplate() : getTrafficFallbackTemplate()
    }

    store.pluginCode = codeTemplate
    updateEditorsContent(codeTemplate)
    showToast(t('plugins.templateInserted', '已插入模板代码'), 'success')
  } catch (error) {
    const fallback = isAgentPlugin ? getAgentFallbackTemplate() : getTrafficFallbackTemplate()
    store.pluginCode = fallback
    updateEditorsContent(fallback)
    showToast(t('plugins.usingBuiltinTemplate', '使用内置模板'), 'info')
  }
}

const getAgentFallbackTemplate = () => `export interface ToolInput { [key: string]: any; }
export interface ToolOutput { success: boolean; data?: any; error?: string; }

export async function analyze(input: ToolInput): Promise<ToolOutput> {
  try {
    // TODO: Implement your Agent tool logic
    return { success: true, data: {} };
  } catch (error) {
    return { success: false, error: error instanceof Error ? error.message : 'Unknown error' };
  }
}

globalThis.analyze = analyze;`

const getTrafficFallbackTemplate = () => `export interface HttpRequest { method: string; url: string; headers: Record<string, string>; body?: string; }
export interface HttpResponse { status: number; headers: Record<string, string>; body?: string; }
export interface PluginContext { request: HttpRequest; response: HttpResponse; }
export interface Finding { title: string; description: string; severity: 'info' | 'low' | 'medium' | 'high' | 'critical'; }

export async function analyze(context: PluginContext): Promise<Finding[]> {
  const findings: Finding[] = [];
  // TODO: Implement your traffic analysis logic
  return findings;
}

globalThis.analyze = analyze;`

const handleFormatCode = () => {
  try {
    const lines = store.pluginCode.split('\n')
    const formatted = lines.map(line => line.trimEnd()).join('\n').replace(/\n{3,}/g, '\n\n')
    store.pluginCode = formatted
    updateEditorsContent(formatted)
    showToast(t('plugins.codeFormatted', '代码已格式化'), 'success')
  } catch (error) {
    showToast(t('plugins.formatFailed', '格式化失败'), 'error')
  }
}

const handleCopyPlugin = async () => {
  try {
    const tags = store.newPluginMetadata.tagsString.split(',').map(t => t.trim()).filter(t => t.length > 0)
    const backendCategory = store.newPluginMetadata.category
    const metadataComment = `/**
 * @plugin ${store.newPluginMetadata.id}
 * @name ${store.newPluginMetadata.name}
 * @version ${store.newPluginMetadata.version}
 * @author ${store.newPluginMetadata.author || 'Unknown'}
 * @category ${backendCategory}
 * @default_severity ${store.newPluginMetadata.default_severity}
 * @tags ${tags.join(', ')}
 * @description ${store.newPluginMetadata.description || ''}
 */
`
    const codeWithoutMetadata = store.pluginCode.replace(/\/\*\*\s*[\s\S]*?\*\/\s*/, '')
    const fullCode = metadataComment + '\n' + codeWithoutMetadata
    await navigator.clipboard.writeText(fullCode)
    showToast(t('plugins.copySuccess', '已复制'), 'success')
  } catch (error) {
    showToast(t('plugins.copyFailed', '复制失败'), 'error')
  }
}

const handleToggleFullscreen = () => {
  if (!store.isFullscreen) {
    // Enter fullscreen
    if (!store.isMinimized) {
      dialogRef.value?.hideModalTemporary()
    }
    store.isFullscreen = true
    nextTick(() => {
      initFullscreenCodeEditor()
    })
  } else {
    // Exit fullscreen
    store.isFullscreen = false
    if (fullscreenCodeEditorView) {
      fullscreenCodeEditorView.destroy()
      fullscreenCodeEditorView = null
    }
    // Only restore modal if not minimized
    if (!store.isMinimized) {
      dialogRef.value?.restoreModal()
      nextTick(() => {
        initCodeEditor()
      })
    }
  }
}

const handleEnableEditing = () => {
  store.isEditing = true
  updateEditorsReadonly(false)
}

const handleCancelEditing = () => {
  store.pluginCode = store.originalCode
  updateEditorsContent(store.originalCode)
  store.isEditing = false
  updateEditorsReadonly(true)
}

const handleSavePlugin = async () => {
  if (!store.editingPlugin) return
  store.saving = true
  store.codeError = ''

  try {
    const tags = store.newPluginMetadata.tagsString.split(',').map(t => t.trim()).filter(t => t.length > 0)
    const backendCategory = store.newPluginMetadata.category

    const metadataComment = `/**
 * @plugin ${store.newPluginMetadata.id}
 * @name ${store.newPluginMetadata.name}
 * @version ${store.newPluginMetadata.version}
 * @author ${store.newPluginMetadata.author || 'Unknown'}
 * @category ${backendCategory}
 * @default_severity ${store.newPluginMetadata.default_severity}
 * @tags ${tags.join(', ')}
 * @description ${store.newPluginMetadata.description || ''}
 */
`

    const codeWithoutMetadata = store.pluginCode.replace(/\/\*\*\s*[\s\S]*?\*\/\s*/, '')
    const fullCode = metadataComment + '\n' + codeWithoutMetadata

    const metadata = {
      id: store.newPluginMetadata.id,
      name: store.newPluginMetadata.name,
      version: store.newPluginMetadata.version,
      author: store.newPluginMetadata.author || 'Unknown',
      main_category: store.newPluginMetadata.mainCategory,
      category: backendCategory,
      description: store.newPluginMetadata.description || '',
      default_severity: store.newPluginMetadata.default_severity,
      tags: tags
    }

    const response = await invoke<CommandResponse<void>>('update_plugin', {
      metadata,
      pluginCode: fullCode
    })

    if (response.success) {
      store.originalCode = store.pluginCode
      store.isEditing = false
      updateEditorsReadonly(true)
      showToast(t('plugins.pluginSaved', '插件已保存'), 'success')
      // Plugin change event is automatically handled by backend
    } else {
      store.codeError = response.error || t('common.saveFailed', '保存失败')
    }
  } catch (error) {
    store.codeError = error instanceof Error ? error.message : t('common.saveFailed', '保存失败')
  } finally {
    store.saving = false
  }
}

const handleCreateNewPlugin = async () => {
  store.saving = true
  store.codeError = ''

  try {
    const tags = store.newPluginMetadata.tagsString.split(',').map(t => t.trim()).filter(t => t.length > 0)
    const backendCategory = store.newPluginMetadata.category

    const metadataComment = `/**
 * @plugin ${store.newPluginMetadata.id}
 * @name ${store.newPluginMetadata.name}
 * @version ${store.newPluginMetadata.version}
 * @author ${store.newPluginMetadata.author || 'Unknown'}
 * @category ${backendCategory}
 * @default_severity ${store.newPluginMetadata.default_severity}
 * @tags ${tags.join(', ')}
 * @description ${store.newPluginMetadata.description || ''}
 */
`

    const fullCode = metadataComment + '\n' + store.pluginCode

    const metadata = {
      id: store.newPluginMetadata.id,
      name: store.newPluginMetadata.name,
      version: store.newPluginMetadata.version,
      author: store.newPluginMetadata.author || 'Unknown',
      main_category: store.newPluginMetadata.mainCategory,
      category: backendCategory,
      description: store.newPluginMetadata.description || '',
      default_severity: store.newPluginMetadata.default_severity,
      tags: tags
    }

    const response = await invoke<CommandResponse<string>>('create_plugin_in_db', {
      metadata,
      pluginCode: fullCode
    })

    if (response.success) {
      store.closeEditor()
      showToast(t('plugins.pluginCreated', '插件创建成功'), 'success')
      // Plugin change event is automatically handled by backend
    } else {
      store.codeError = response.error || t('common.createFailed', '创建失败')
    }
  } catch (error) {
    store.codeError = error instanceof Error ? error.message : t('common.createFailed', '创建失败')
  } finally {
    store.saving = false
  }
}

const handleSendAiMessage = async (message: string) => {
  if (!message.trim() || store.aiChatStreaming) return
  
  const latestCode = store.pluginCode
  const codeContext = store.selectedCodeRef?.code || null
  const history = store.aiChatMessages.map(msg => ({
    role: msg.role,
    content: msg.content
  }))

  store.aiChatMessages.push({ 
    role: 'user', 
    content: message,
    codeRef: store.selectedCodeRef || {
      code: latestCode,
      preview: latestCode.substring(0, 100) + '...',
      startLine: 1,
      endLine: latestCode.split('\n').length,
      isFullCode: true
    },
    testResultRef: store.selectedTestResultRef || undefined
  })
  
  store.aiChatStreaming = true
  store.aiChatStreamingContent = ''
  
  const streamId = `plugin_assistant_${Date.now()}`
  
  try {
    const isAgentPlugin = store.newPluginMetadata.mainCategory === 'agent'
    const baseSystemPrompt = await invoke<string>('get_combined_plugin_prompt_api', {
      pluginType: isAgentPlugin ? 'agent' : 'traffic',
      vulnType: 'custom',
      severity: store.newPluginMetadata.default_severity
    })
    
    // 为编辑对话场景设计的 System Prompt
    const systemPrompt = `${baseSystemPrompt}

## 重要：你现在是代码编辑助手模式

你正在帮助用户编辑和改进现有的插件代码。请遵循以下规则：

### 响应策略
1. **仅在需要时返回代码**：
   - 如果用户只是提问、咨询、解释需求 → 只用文字回答，不要返回代码
   - 如果用户明确要求修改、优化、添加功能 → 返回修改后的完整代码
   - 如果用户说"不要修改代码"、"只是问问" → 绝对不要返回代码块

2. **理解用户意图**：
   - 提问类：解释、说明、咨询 → 文字回答
   - 修改类：优化、修复、添加、改进 → 返回代码
   - 审查类：检查、分析、建议 → 列出问题和建议，不返回代码（除非用户明确要求）

3. **代码返回格式**：
   - 只有在确定需要修改代码时，才返回完整的 TypeScript 代码块
   - 代码必须用 \`\`\`typescript 包裹
   - 代码必须是完整可运行的

### 示例

**用户说："这段代码有什么问题吗？"**
→ 回答：分析代码问题，列出建议，不返回代码

**用户说："帮我优化一下性能"**
→ 回答：返回优化后的完整代码

**用户说："你好，不要做代码修改"**
→ 回答：好的，我不会修改代码。有什么我可以帮您的吗？

现在开始对话。`

    let generatedContent = ''
    
    const unlistenDelta = await listen('plugin_assistant_delta', (event: any) => {
      if (event.payload.stream_id === streamId) {
        generatedContent += event.payload.delta || ''
        store.aiChatStreamingContent = generatedContent
      }
    })
    
    const unlistenComplete = await listen('plugin_assistant_complete', (event: any) => {
      if (event.payload.stream_id === streamId) {
        generatedContent = event.payload.content || generatedContent
        finishAiChat(generatedContent)
        unlistenDelta()
        unlistenComplete()
        unlistenError()
      }
    })
    
    const unlistenError = await listen('plugin_assistant_error', (event: any) => {
      if (event.payload.stream_id === streamId) {
        const errorMsg = event.payload.error || 'AI processing failed'
        store.aiChatMessages.push({ role: 'assistant', content: `❌ ${errorMsg}` })
        store.aiChatStreaming = false
        store.aiChatStreamingContent = ''
        unlistenDelta()
        unlistenComplete()
        unlistenError()
      }
    })
    
    await invoke('plugin_assistant_chat_stream', {
      request: {
        stream_id: streamId,
        message: message,
        system_prompt: systemPrompt,
        service_name: 'default',
        history: history,
        current_code: latestCode,
        code_context: codeContext
      }
    })
  } catch (error) {
    store.aiChatStreaming = false
    store.aiChatStreamingContent = ''
  }
}

const finishAiChat = (content: string) => {
  store.aiChatStreaming = false
  store.aiChatStreamingContent = ''
  
  const codeBlocks: string[] = []
  const codeBlockRegex = /```(?:typescript|ts|javascript|js)?\n?([\s\S]*?)```/g
  let match
  while ((match = codeBlockRegex.exec(content)) !== null) {
    codeBlocks.push(match[1].trim())
  }
  
  marked.setOptions({ breaks: true, gfm: true })
  const rawHtml = marked.parse(content) as string
  const cleanHtml = DOMPurify.sanitize(rawHtml)
  
  store.aiChatMessages.push({ 
    role: 'assistant', 
    content: cleanHtml,
    codeBlock: codeBlocks[0],
    codeBlocks: codeBlocks
  })
}

const handleAiQuickAction = (action: string) => {
  const actions: Record<string, string> = {
    'explain': '请解释这段插件代码的功能和工作原理',
    'optimize': '请优化这段代码，提高性能和可读性',
    'fix': '请检查并修复这段代码中可能存在的问题'
  }
  handleSendAiMessage(actions[action] || action)
}

const handleApplyAiCode = (code: string, context?: CodeReference | null) => {
  if (!code) return
  let finalCode = store.pluginCode
  if (context && !context.isFullCode && store.pluginCode.includes(context.code)) {
    finalCode = store.pluginCode.replace(context.code, code)
  } else {
    finalCode = code
  }
  store.pluginCode = finalCode
  updateEditorsContent(finalCode)
  showToast(t('plugins.codeApplied', '代码已应用'), 'success')
  if (!store.isEditing) handleEnableEditing()
}

const handlePreviewAiCode = (code: string) => {
  store.previewCode = code
  store.isPreviewMode = true
  nextTick(() => {
    initDiffEditor()
  })
}

const handleExitPreviewMode = () => {
  store.isPreviewMode = false
  store.previewCode = ''
  if (diffEditorViewA) { diffEditorViewA.destroy(); diffEditorViewA = null }
  if (diffEditorViewB) { diffEditorViewB.destroy(); diffEditorViewB = null }
  nextTick(() => {
    if (store.isFullscreen) initFullscreenCodeEditor()
    else initCodeEditor()
  })
}

const handleTestPlugin = async () => {
  if (!store.editingPlugin) return
  store.pluginTesting = true
  try {
    const isAgentPlugin = store.editingPlugin.metadata.main_category === 'agent'
    const command = isAgentPlugin ? 'test_agent_plugin' : 'test_plugin'
    const resp = await invoke<CommandResponse<TestResult>>(command, { 
      pluginId: store.editingPlugin.metadata.id,
      inputs: isAgentPlugin ? {} : undefined
    })
    
    if (resp.success && resp.data) {
      showToast(resp.data.success ? t('plugins.testSuccess', '测试成功') : t('plugins.testFailed', '测试失败'), resp.data.success ? 'success' : 'error')
    } else {
      showToast(resp.error || t('plugins.testError', '测试错误'), 'error')
    }
  } catch (e) {
    showToast(t('plugins.testError', '测试异常'), 'error')
  } finally {
    store.pluginTesting = false
  }
}

// Watch for minimized state to handle visibility
watch(() => store.isMinimized, (minimized) => {
  if (minimized) {
    // When minimizing, just hide the UI but keep the state
    // If in fullscreen, keep fullscreen state
    if (dialogRef.value) {
      dialogRef.value.hideModalTemporary()
    }
    // Destroy editors to free resources
    if (codeEditorView) {
      codeEditorView.destroy()
      codeEditorView = null
    }
    if (fullscreenCodeEditorView) {
      fullscreenCodeEditorView.destroy()
      fullscreenCodeEditorView = null
    }
  } else if (!minimized && store.isOpen) {
    // When restoring, check if we should restore to fullscreen or normal mode
    nextTick(() => {
      if (store.isFullscreen) {
        // Restore to fullscreen mode
        // Don't restore modal, just reinitialize fullscreen editor
        initFullscreenCodeEditor()
      } else {
        // Restore to normal dialog mode
        dialogRef.value?.restoreModal()
        initCodeEditor()
      }
    })
  }
})

watch(() => store.isOpen, (open) => {
  if (open && !store.isMinimized) {
    nextTick(() => {
      dialogRef.value?.showDialog()
      initCodeEditor()
    })
  }
})

// ESC key handler for fullscreen mode
const handleKeyDown = (e: KeyboardEvent) => {
  if (e.key === 'Escape' && store.isFullscreen) {
    e.preventDefault()
    e.stopPropagation()
    handleToggleFullscreen()
  }
}

watch(() => store.isFullscreen, (isFullscreen) => {
  if (isFullscreen) {
    document.addEventListener('keydown', handleKeyDown, true)
  } else {
    document.removeEventListener('keydown', handleKeyDown, true)
  }
})

onUnmounted(() => {
  if (codeEditorView) codeEditorView.destroy()
  if (fullscreenCodeEditorView) fullscreenCodeEditorView.destroy()
  if (diffEditorViewA) diffEditorViewA.destroy()
  if (diffEditorViewB) diffEditorViewB.destroy()
  document.removeEventListener('keydown', handleKeyDown, true)
})
</script>

<style scoped>
.animate-bounce-in {
  animation: bounceIn 0.5s cubic-bezier(0.34, 1.56, 0.64, 1);
}

@keyframes bounceIn {
  0% { transform: scale(0.3); opacity: 0; }
  70% { transform: scale(1.05); }
  100% { transform: scale(1); opacity: 1; }
}
</style>
