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
      @confirm-merge="handleConfirmMerge"
      @test-current-plugin="handleTestPlugin"
      @clear-history="handleClearHistory"
    />

    <!-- Test Result Dialog -->
    <dialog ref="testResultDialogRef" class="modal">
      <div class="modal-box w-11/12 max-w-3xl">
        <h3 class="font-bold text-base mb-4">
          <i class="fas fa-vial mr-2"></i>{{ $t('plugins.testResult', '插件测试结果') }}
        </h3>
        <div v-if="store.pluginTesting" class="alert alert-info">
          <span class="loading loading-spinner"></span>
          <span>{{ $t('plugins.testing', '正在测试插件...') }}</span>
        </div>
        <div v-else-if="testResult" class="space-y-4">
          <!-- Status Alert -->
          <div class="alert" :class="{ 'alert-success': testResult.success, 'alert-error': !testResult.success }">
            <i :class="testResult.success ? 'fas fa-check-circle' : 'fas fa-times-circle'"></i>
            <span>{{ testResult.success ? $t('plugins.testPassed', '测试通过') : $t('plugins.testFailed', '测试失败') }}</span>
          </div>
          
          <!-- Failed: Show error message only -->
          <div v-if="!testResult.success" class="card bg-base-200">
            <div class="card-body">
              <h4 class="font-semibold mb-2 text-error">{{ $t('plugins.errorInfo', '错误信息') }}</h4>
              <pre class="text-sm whitespace-pre-wrap break-all text-error/80">{{ testResult.error || testResult.message || $t('plugins.unknownError', '未知错误') }}</pre>
            </div>
          </div>
          
          <!-- Success: Show message and findings -->
          <template v-else>
            <div v-if="testResult.message" class="card bg-base-200">
              <div class="card-body">
                <h4 class="font-semibold mb-2">{{ $t('plugins.testMessage', '测试消息') }}</h4>
                <pre class="text-sm whitespace-pre-wrap">{{ testResult.message }}</pre>
              </div>
            </div>
            <div v-if="testResult.findings && testResult.findings.length > 0" class="card bg-base-200">
              <div class="card-body">
                <h4 class="font-semibold mb-2">{{ $t('plugins.findings', '发现') }} ({{ testResult.findings.length }})</h4>
                <div class="space-y-2">
                  <div v-for="(finding, idx) in testResult.findings" :key="idx" class="card bg-base-100">
                    <div class="card-body p-3">
                      <div class="flex justify-between items-start">
                        <span class="font-medium">{{ finding.title }}</span>
                        <span class="badge" :class="getSeverityBadgeClass(finding.severity)">{{ finding.severity }}</span>
                      </div>
                      <p class="text-sm text-base-content/70 mt-1 whitespace-pre-wrap break-all">{{ finding.description }}</p>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </template>
        </div>
        <div class="modal-action">
          <button class="btn btn-primary" @click="handleReferTestResultToAi" :disabled="!testResult">
            <i class="fas fa-robot mr-2"></i>{{ $t('plugins.referToAi', '引用到AI助手') }}
          </button>
          <button class="btn" @click="closeTestResultDialog">{{ $t('common.close', '关闭') }}</button>
        </div>
      </div>
      <form method="dialog" class="modal-backdrop"><button @click="closeTestResultDialog">close</button></form>
    </dialog>

    <!-- Custom Context Menu for Editor - Teleport to body to escape stacking context -->
    <Teleport to="body">
      <div v-if="contextMenu.show" 
        data-context-menu
        class="fixed"
        :style="{
          left: contextMenu.x + 'px',
          top: contextMenu.y + 'px',
          zIndex: 9999999,
          backgroundColor: '#2a2e37',
          color: '#fff',
          boxShadow: '0 10px 30px rgba(0,0,0,0.5)',
          border: '2px solid rgba(99, 102, 241, 0.5)',
          borderRadius: '8px',
          padding: '6px',
          minWidth: '180px',
          pointerEvents: 'auto'
        }"
        v-click-outside="() => contextMenu.show = false"
        @mousedown.stop>
        <button 
          :style="{
            display: 'flex',
            alignItems: 'center',
            gap: '8px',
            width: '100%',
            padding: '10px 16px',
            fontSize: '14px',
            borderRadius: '6px',
            transition: 'all 0.2s',
            cursor: 'pointer',
            border: 'none',
            backgroundColor: 'transparent',
            color: 'inherit',
            textAlign: 'left'
          }"
          @click.stop="addSelectedToAiContext"
          @mouseenter="e => (e.target as HTMLElement).style.backgroundColor = '#6366f1'"
          @mouseleave="e => (e.target as HTMLElement).style.backgroundColor = 'transparent'">
          <i class="fas fa-robot" style="width: 16px"></i>
          <span>{{ $t('plugins.addToAiContext', '添加到 AI 上下文') }}</span>
        </button>
      </div>
    </Teleport>

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
import { MergeView } from '@codemirror/merge'
import { javascript } from '@codemirror/lang-javascript'
import { oneDark } from '@codemirror/theme-one-dark'

// 扩展 HTMLElement 类型以支持自定义属性
declare module '@vue/runtime-core' {
  interface HTMLElement {
    _clickOutsideHandler?: (event: MouseEvent) => void
  }
}

// ===== Diff Format Parser and Applicator =====

interface DiffBlock {
  type: 'search_replace' | 'insert_after' | 'full_code'
  search?: string
  replace?: string
  insertAfter?: string
  insertContent?: string
  fullCode?: string
}

/**
 * Parse LLM response for diff blocks
 */
function parseDiffBlocks(content: string): DiffBlock[] {
  const blocks: DiffBlock[] = []
  
  // Pattern 1: SEARCH/REPLACE blocks
  const searchReplaceRegex = /```diff\s*\n<<<<<<< SEARCH\s*\n([\s\S]*?)\n=======\s*\n([\s\S]*?)\n>>>>>>> REPLACE\s*\n```/g
  let match
  
  while ((match = searchReplaceRegex.exec(content)) !== null) {
    blocks.push({
      type: 'search_replace',
      search: match[1].trim(),
      replace: match[2].trim()
    })
  }
  
  // Pattern 2: INSERT blocks
  const insertRegex = /```diff\s*\n<<<<<<< INSERT_AFTER\s*\n([\s\S]*?)\n=======\s*\n([\s\S]*?)\n>>>>>>> INSERT\s*\n```/g
  
  while ((match = insertRegex.exec(content)) !== null) {
    blocks.push({
      type: 'insert_after',
      insertAfter: match[1].trim(),
      insertContent: match[2].trim()
    })
  }
  
  // Pattern 3: Full code blocks (fallback)
  if (blocks.length === 0) {
    const fullCodeRegex = /```(?:typescript|ts|javascript|js)\s*\n([\s\S]*?)```/g
    while ((match = fullCodeRegex.exec(content)) !== null) {
      blocks.push({
        type: 'full_code',
        fullCode: match[1].trim()
      })
    }
  }
  
  return blocks
}

/**
 * Apply diff blocks to current code
 */
function applyDiffBlocks(currentCode: string, blocks: DiffBlock[]): { success: boolean, code: string, error?: string } {
  let resultCode = currentCode
  
  for (const block of blocks) {
    if (block.type === 'search_replace') {
      const searchText = block.search!
      const replaceText = block.replace!
      
      if (!resultCode.includes(searchText)) {
        return {
          success: false,
          code: currentCode,
          error: `Cannot find search text in code:\n${searchText.substring(0, 100)}...`
        }
      }
      
      const occurrences = resultCode.split(searchText).length - 1
      if (occurrences > 1) {
        return {
          success: false,
          code: currentCode,
          error: `Search text appears ${occurrences} times. Need more context.`
        }
      }
      
      resultCode = resultCode.replace(searchText, replaceText)
      
    } else if (block.type === 'insert_after') {
      const anchor = block.insertAfter!
      const content = block.insertContent!
      
      if (!resultCode.includes(anchor)) {
        return {
          success: false,
          code: currentCode,
          error: `Cannot find insertion anchor:\n${anchor.substring(0, 100)}...`
        }
      }
      
      const occurrences = resultCode.split(anchor).length - 1
      if (occurrences > 1) {
        return {
          success: false,
          code: currentCode,
          error: `Insertion anchor appears ${occurrences} times. Need more specific anchor.`
        }
      }
      
      resultCode = resultCode.replace(anchor, `${anchor}\n${content}`)
      
    } else if (block.type === 'full_code') {
      resultCode = block.fullCode!
    }
  }
  
  return { success: true, code: resultCode }
}

const store = usePluginEditorStore()
const { t } = useI18n()
const dialogRef = ref<InstanceType<typeof PluginCodeEditorDialog>>()
const testResultDialogRef = ref<HTMLDialogElement>()

// 测试结果状态
const testResult = ref<TestResult | null>(null)

// 右键菜单状态
const contextMenu = ref({
  show: false,
  x: 0,
  y: 0,
  selectedText: ''
})

// 追踪菜单打开时间，防止立即被关闭
let contextMenuOpenTime = 0

const addSelectedToAiContext = () => {
  console.log('addSelectedToAiContext called', contextMenu.value.selectedText)
  if (contextMenu.value.selectedText) {
    const lines = contextMenu.value.selectedText.split('\n')
    store.selectedCodeRef = {
      code: contextMenu.value.selectedText,
      preview: lines[0].substring(0, 50) + (lines.length > 1 || lines[0].length > 50 ? '...' : ''),
      startLine: 0, // CodeMirror 选区坐标计算较复杂，这里简化处理
      endLine: 0,
      isFullCode: false
    }
    store.showAiPanel = true
    showToast(t('plugins.addedToContext', '已添加到上下文'), 'success')
  }
  contextMenu.value.show = false
}

// Click outside directive - 使用 mousedown 而不是 click，避免与 contextmenu 冲突
const vClickOutside = {
  mounted(el: any, binding: any) {
    el._clickOutsideHandler = (event: MouseEvent) => {
      // 忽略右键点击
      if (event.button === 2) {
        console.log('Ignoring right click')
        return
      }
      
      // 检查菜单是否刚刚打开（300ms 内不关闭）
      const timeSinceOpen = Date.now() - contextMenuOpenTime
      if (timeSinceOpen < 300) {
        console.log('Menu just opened, ignoring click outside', timeSinceOpen)
        return
      }
      
      if (!(el === event.target || el.contains(event.target as Node))) {
        console.log('Click outside detected, closing context menu')
        binding.value()
      }
    }
    // 立即添加监听器，通过时间戳来过滤
    console.log('Adding click outside listener')
    document.addEventListener('mousedown', el._clickOutsideHandler!, true)
  },
  unmounted(el: any) {
    if (el._clickOutsideHandler) {
      console.log('Removing click outside listener')
      document.removeEventListener('mousedown', el._clickOutsideHandler, true)
      delete el._clickOutsideHandler
    }
  }
}

// 验证状态
const validationState = ref<{
  validating: boolean
  result: any | null
}>({
  validating: false,
  result: null
})

// CodeMirror Instances
let codeEditorView: EditorView | null = null
let fullscreenCodeEditorView: EditorView | null = null
let diffMergeView: MergeView | null = null
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
  toast.className = 'toast toast-top toast-end z-[9999999]'
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
      }),
      EditorView.domEventHandlers({
        contextmenu: (e, view) => {
          const selection = view.state.sliceDoc(
            view.state.selection.main.from,
            view.state.selection.main.to
          )
          console.log('Context menu event triggered, selection:', selection)
          if (selection.trim()) {
            e.preventDefault()
            e.stopPropagation() // 阻止事件冒泡
            
            // 确保坐标正确
            const x = e.clientX
            const y = e.clientY
            console.log('Setting context menu at', x, y)
            
            // 记录菜单打开时间
            contextMenuOpenTime = Date.now()
            
            // 使用 nextTick 确保 Vue 响应性系统正确更新
            nextTick(() => {
              contextMenu.value = {
                show: true,
                x: x,
                y: y,
                selectedText: selection
              }
              
              console.log('Context menu state after nextTick:', contextMenu.value, 'openTime:', contextMenuOpenTime)
              
              // 验证 DOM 元素是否真的存在
              nextTick(() => {
                const menuEl = document.querySelector('[data-context-menu]')
                console.log('Menu DOM element:', menuEl)
              })
            })
            
            return true
          }
          console.log('No selection, showing default menu')
          return false
        }
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
      }),
      EditorView.domEventHandlers({
        contextmenu: (e, view) => {
          const selection = view.state.sliceDoc(
            view.state.selection.main.from,
            view.state.selection.main.to
          )
          console.log('Context menu event triggered, selection:', selection)
          if (selection.trim()) {
            e.preventDefault()
            e.stopPropagation() // 阻止事件冒泡
            
            // 确保坐标正确
            const x = e.clientX
            const y = e.clientY
            console.log('Setting context menu at', x, y)
            
            // 记录菜单打开时间
            contextMenuOpenTime = Date.now()
            
            // 使用 nextTick 确保 Vue 响应性系统正确更新
            nextTick(() => {
              contextMenu.value = {
                show: true,
                x: x,
                y: y,
                selectedText: selection
              }
              
              console.log('Context menu state after nextTick:', contextMenu.value, 'openTime:', contextMenuOpenTime)
              
              // 验证 DOM 元素是否真的存在
              nextTick(() => {
                const menuEl = document.querySelector('[data-context-menu]')
                console.log('Menu DOM element:', menuEl)
              })
            })
            
            return true
          }
          console.log('No selection, showing default menu')
          return false
        }
      })
    ],
    parent: container
  })
}

const initDiffEditor = () => {
  const container = dialogRef.value?.fullscreenDiffEditorContainerRef
  if (!container) return
  
  container.innerHTML = ''
  
  if (diffMergeView) { diffMergeView.destroy(); diffMergeView = null }
  
  diffMergeView = new MergeView({
    a: {
      doc: store.pluginCode,
      extensions: [basicSetup, javascript(), oneDark, EditorView.lineWrapping, EditorView.editable.of(false)]
    },
    b: {
      doc: store.previewCode,
      extensions: [
        basicSetup, javascript(), oneDark, EditorView.lineWrapping,
        EditorView.updateListener.of((update: ViewUpdate) => {
          if (update.docChanged) store.previewCode = update.state.doc.toString()
        })
      ]
    },
    parent: container,
    collapseUnchanged: { margin: 3, minSize: 4 } // 折叠未修改的代码，提升聚焦度
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
    // 使用完整的插件生成 prompt（包含任务说明、示例等）来提取模板代码
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
    // 使用插件编辑专用的接口文档（仅包含接口说明，不包含生成任务说明）
    // 这样 AI 会专注于编辑现有代码，而不是重新生成整个插件
    const isAgentPlugin = store.newPluginMetadata.mainCategory === 'agent'
    const interfaceDoc = await invoke<string>('get_plugin_interface_doc_api', {
      pluginType: isAgentPlugin ? 'agent' : 'traffic'
    })

    // 2. 构建代码上下文区块
    // 如果用户选中了代码片段，只发送选中部分；否则发送全量代码
    let codeContextBlock = ''
    let currentCodeBlock = ''
    
    if (codeContext) {
      // the user selected a code fragment, only send the selected part
      codeContextBlock = `
### the code fragment selected by the user (Focus Context)
\`\`\`typescript
${codeContext}
\`\`\`

**Note**：the user only selected the above code fragment, please only modify this part. The returned code should be able to replace the selected part directly.
`
    } else {
      // the user did not select any code, send the full code
      currentCodeBlock = `
### the full code of the current plugin (Full Code)
\`\`\`typescript
${latestCode}
\`\`\`
`
    }

    const systemPrompt = `You are a senior security researcher and TypeScript expert, helping users with code editing tasks.

${codeContextBlock}
${currentCodeBlock}

## CRITICAL OUTPUT FORMAT

You MUST use one of these formats for code modifications:

### Format 1: SEARCH/REPLACE (Preferred - for modifying existing code)
\`\`\`diff
<<<<<<< SEARCH
[exact code to find - include 3-5 lines before and after for uniqueness]
=======
[new code to replace with]
>>>>>>> REPLACE
\`\`\`

### Format 2: INSERT (for adding new code)
\`\`\`diff
<<<<<<< INSERT_AFTER
[exact line after which to insert]
=======
[new code to insert]
>>>>>>> INSERT
\`\`\`

### Format 3: FULL CODE (ONLY if user explicitly requests or needs complete rewrite)
\`\`\`typescript
[complete code]
\`\`\`

## Rules and Constraints

1. **Minimum Modification Principle (CRITICAL)**:
   - **DEFAULT to SEARCH/REPLACE format** - Never return full code unless absolutely necessary
   - **Only do what user requires**: Strictly follow user's instructions
   - **Keep original code as is**: Preserve parts that don't need modification
   - **No excessive optimization**: Don't refactor code user didn't mention

2. **SEARCH/REPLACE Best Practices**:
   - Include enough context (3-5 lines) to uniquely identify location
   - One change per block - use multiple blocks for multiple changes
   - Verify SEARCH text exists and is unique in the code

3. **Chain of Thought Check**:
   - Before responding, ask: Did I modify parts user didn't mention?
   - Clearly state your modification range

4. **Technical Specifications**:
   - For Agent plugins, update \`get_input_schema\` if modifying parameters
   - Maintain code style and formatting

## Example

**Good (SEARCH/REPLACE):**
I'll add ORDER BY payloads to the array:

\`\`\`diff
<<<<<<< SEARCH
const sqlPayloads = [
    "' OR '1'='1",
    "' UNION SELECT NULL--"
];
=======
const sqlPayloads = [
    "' OR '1'='1",
    "' UNION SELECT NULL--",
    "' ORDER BY 1--",
    "' ORDER BY 10--"
];
>>>>>>> REPLACE
\`\`\`

**Bad (full code):**
\`\`\`typescript
[entire 500 line file]
\`\`\`

---
### Interface Reference Document
${interfaceDoc}

Now, please assist based on the user's message and code context above.`

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
  
  // Parse diff blocks
  const diffBlocks = parseDiffBlocks(content)
  
  // Try to apply diff blocks automatically
  let appliedCode: string | null = null
  let applyError: string | null = null
  
  if (diffBlocks.length > 0) {
    const currentCode = store.pluginCode
    const result = applyDiffBlocks(currentCode, diffBlocks)
    
    if (result.success) {
      appliedCode = result.code
      // Auto-apply to store
      store.pluginCode = result.code
      console.log('✅ Diff applied automatically')
    } else {
      applyError = result.error
      console.warn('⚠️ Could not auto-apply diff:', result.error)
    }
  }
  
  // Fallback: Extract code blocks
  const codeBlocks: string[] = []
  const codeBlockRegex = /```(?:typescript|ts|javascript|js)\s*\n([\s\S]*?)```/g
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
    codeBlock: appliedCode || codeBlocks[0],
    codeBlocks: appliedCode ? [appliedCode] : codeBlocks,
    diffApplied: !!appliedCode,
    diffError: applyError || undefined
  })
}

const handleAiQuickAction = (action: string) => {
  const actions: Record<string, string> = {
    'explain': '请解释这段插件代码的功能和工作原理',
    'optimize': '请优化这段代码，提高性能和可读性',
    'fix': '请检查并修复这段代码中可能存在的问题',
    'refactor': '请重构这段代码，提高代码质量和可维护性',
    'security': '请对这段代码进行安全审查，找出潜在的安全漏洞',
    'document': '请为这段代码添加详细的注释和文档说明',
    'test': '请为这段代码生成测试用例，覆盖主要功能和边界情况'
  }
  handleSendAiMessage(actions[action] || action)
}

const handleApplyAiCode = async (code: string, context?: CodeReference | null) => {
  if (!code) return
  
  let finalCode = store.pluginCode
  let isPartialUpdate = false
  
  // 判断是否为局部更新
  if (context && !context.isFullCode && context.code && store.pluginCode.includes(context.code)) {
    // 局部替换：用AI返回的代码替换选中的代码片段
    finalCode = store.pluginCode.replace(context.code, code)
    isPartialUpdate = true
  } else if (!code.includes('/**') && !code.includes('export interface') && code.trim().startsWith('export')) {
    // 智能检测：如果AI返回的代码是单个函数或小片段（没有元数据注释，没有接口定义）
    // 尝试智能替换
    const functionMatch = code.match(/export\s+(?:async\s+)?function\s+(\w+)/)
    if (functionMatch) {
      const functionName = functionMatch[1]
      // 查找当前代码中的同名函数
      const currentFunctionRegex = new RegExp(
        `export\\s+(?:async\\s+)?function\\s+${functionName}[^{]*\\{[\\s\\S]*?\\n\\}`,
        'g'
      )
      if (currentFunctionRegex.test(store.pluginCode)) {
        finalCode = store.pluginCode.replace(currentFunctionRegex, code.trim())
        isPartialUpdate = true
      } else {
        // 找不到对应函数，可能是新增，插入到文件末尾（globalThis 绑定之前）
        const globalThisMatch = store.pluginCode.match(/(globalThis\.\w+\s*=\s*\w+;?\s*)$/)
        if (globalThisMatch) {
          finalCode = store.pluginCode.replace(
            globalThisMatch[1],
            `\n${code.trim()}\n\n${globalThisMatch[1]}`
          )
        } else {
          finalCode = store.pluginCode + '\n\n' + code.trim()
        }
        isPartialUpdate = true
      }
    }
  }
  
  // 如果不是局部更新，则作为完整代码替换
  if (!isPartialUpdate) {
    finalCode = code
  }
  
  // 验证最终代码
  const validationResult = await validateCode(finalCode)
  
  if (!validationResult.is_valid) {
    const errorMsg = validationResult.errors.join('\n')
    if (!confirm(t('plugins.codeHasErrors', { errors: errorMsg }))) {
      return
    }
  } else if (validationResult.warnings.length > 0) {
    const warningMsg = validationResult.warnings.join('\n')
    showToast(t('plugins.codeHasWarnings', { warnings: warningMsg }), 'warning')
  }
  
  // 以前是直接应用，现在改为进入 Diff 预览模式
  handlePreviewAiCode(finalCode)
  showToast(t('plugins.reviewChanges', '请审查代码变更'), 'info')
}

const validateCode = async (code: string): Promise<any> => {
  try {
    const result = await invoke('validate_plugin_code', { code })
    return result
  } catch (error) {
    console.error('Validation failed:', error)
    return {
      is_valid: true, // 验证失败时不阻止用户
      syntax_valid: true,
      has_required_functions: true,
      security_check_passed: true,
      errors: [],
      warnings: [String(error)]
    }
  }
}

const handlePreviewAiCode = (code: string) => {
  store.previewCode = code
  store.isPreviewMode = true
  nextTick(() => {
    initDiffEditor()
  })
}

const handleConfirmMerge = () => {
  store.pluginCode = store.previewCode
  updateEditorsContent(store.pluginCode)
  
  store.isPreviewMode = false
  store.previewCode = ''
  if (diffMergeView) { diffMergeView.destroy(); diffMergeView = null }
  
  nextTick(() => {
    if (store.isFullscreen) initFullscreenCodeEditor()
    else initCodeEditor()
  })
  
  showToast(t('plugins.codeMerged', '代码已合并'), 'success')
  if (!store.isEditing) handleEnableEditing()
}

const handleExitPreviewMode = () => {
  store.isPreviewMode = false
  store.previewCode = ''
  if (diffMergeView) { diffMergeView.destroy(); diffMergeView = null }
  nextTick(() => {
    if (store.isFullscreen) initFullscreenCodeEditor()
    else initCodeEditor()
  })
}

const handleTestPlugin = async () => {
  if (!store.editingPlugin) return
  
  // 打开测试结果对话框
  testResult.value = null
  testResultDialogRef.value?.showModal()
  
  store.pluginTesting = true
  try {
    const isAgentPlugin = store.editingPlugin.metadata.main_category === 'agent'
    const command = isAgentPlugin ? 'test_agent_plugin' : 'test_plugin'
    const resp = await invoke<CommandResponse<any>>(command, { 
      pluginId: store.editingPlugin.metadata.id,
      inputs: isAgentPlugin ? {} : undefined
    })
    
    if (resp.success && resp.data) {
      // 处理 Agent 插件测试结果
      if (isAgentPlugin) {
        testResult.value = {
          success: resp.data.success,
          message: resp.data.message || (resp.data.success ? `插件执行完成 (${resp.data.execution_time_ms}ms)` : '测试失败'),
          findings: [{
            title: 'Agent工具执行结果',
            description: JSON.stringify(resp.data.output ?? { error: resp.data.error }, null, 2),
            severity: resp.data.success ? 'info' : 'error'
          }],
          error: resp.data.error
        }
      } else {
        // 流量分析插件测试结果
        testResult.value = resp.data
      }
      
      showToast(resp.data.success ? t('plugins.testSuccess', '测试成功') : t('plugins.testFailed', '测试失败'), resp.data.success ? 'success' : 'error')
    } else {
      testResult.value = {
        success: false,
        message: resp.error || t('plugins.testError', '测试错误'),
        error: resp.error
      }
      showToast(resp.error || t('plugins.testError', '测试错误'), 'error')
    }
  } catch (e) {
    const errorMsg = e instanceof Error ? e.message : t('plugins.testError', '测试异常')
    testResult.value = {
      success: false,
      message: errorMsg,
      error: errorMsg
    }
    showToast(errorMsg, 'error')
  } finally {
    store.pluginTesting = false
  }
}

const handleClearHistory = () => {
  if (!store.editingPlugin) return
  
  // 确认对话框
    const pluginId = store.editingPlugin.metadata.id
    // 先清空当前显示的消息
    store.aiChatMessages = []
    // 再清除并保存（这会同时清除内存和 localStorage）
    store.clearChatHistory(pluginId)
    showToast(t('plugins.historyCleared', '对话历史已清除'), 'success')
  
}

const closeTestResultDialog = () => {
  testResultDialogRef.value?.close()
}

const handleReferTestResultToAi = () => {
  if (!testResult.value) return
  
  // 构建测试结果引用
  const resultText = `Test Result:\n${JSON.stringify(testResult.value, null, 2)}`
  store.selectedTestResultRef = {
    result: testResult.value,
    preview: testResult.value.message?.substring(0, 100) || 'Test Result'
  }
  
  store.showAiPanel = true
  closeTestResultDialog()
  showToast(t('plugins.addedToContext', '已添加到上下文'), 'success')
}

const getSeverityBadgeClass = (severity: string): string => {
  const map: Record<string, string> = {
    'critical': 'badge-error',
    'high': 'badge-warning',
    'medium': 'badge-info',
    'low': 'badge-success',
    'info': 'badge-ghost',
    'error': 'badge-error'
  }
  return map[severity.toLowerCase()] || 'badge-ghost'
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

// Global keyboard shortcuts handler
const handleKeyDown = (e: KeyboardEvent) => {
  // ESC: 退出全屏或关闭 AI 面板
  if (e.key === 'Escape') {
    if (store.isFullscreen) {
      e.preventDefault()
      e.stopPropagation()
      handleToggleFullscreen()
    } else if (store.showAiPanel) {
      e.preventDefault()
      store.showAiPanel = false
    }
    return
  }

  const isMac = /Mac|iPod|iPhone|iPad/.test(navigator.platform)
  const ctrlOrCmd = isMac ? e.metaKey : e.ctrlKey

  // Ctrl/Cmd + K: 打开/关闭 AI 助手
  if (ctrlOrCmd && e.key === 'k') {
    e.preventDefault()
    store.showAiPanel = !store.showAiPanel
    return
  }

  // Ctrl/Cmd + S: 保存插件
  if (ctrlOrCmd && e.key === 's') {
    e.preventDefault()
    if (store.editingPlugin && store.isEditing) {
      handleSavePlugin()
    } else if (!store.editingPlugin) {
      handleCreateNewPlugin()
    }
    return
  }

  // Ctrl/Cmd + Shift + F: 格式化代码
  if (ctrlOrCmd && e.shiftKey && e.key === 'F') {
    e.preventDefault()
    handleFormatCode()
    return
  }

  // Ctrl/Cmd + Shift + C: 复制插件代码
  if (ctrlOrCmd && e.shiftKey && e.key === 'C') {
    e.preventDefault()
    handleCopyPlugin()
    return
  }

  // F11: 切换全屏
  if (e.key === 'F11') {
    e.preventDefault()
    handleToggleFullscreen()
    return
  }

  // Ctrl/Cmd + E: 启用编辑模式
  if (ctrlOrCmd && e.key === 'e' && store.editingPlugin && !store.isEditing) {
    e.preventDefault()
    handleEnableEditing()
    return
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
  if (diffMergeView) diffMergeView.destroy()
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
