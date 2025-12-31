<template>
  <div class="space-y-4">
    <div class="flex justify-between items-center">
      <div class="alert alert-info flex-1 mr-4">
        <i class="fas fa-info-circle"></i>
        <span>管理 Agent 插件工具，可在创建 Agent 时选择启用的插件工具</span>
      </div>
      <div class="join">
        <button @click="viewMode = 'card'" :class="['join-item', 'btn', 'btn-sm', {'btn-primary': viewMode === 'card'}]">
          <i class="fas fa-th-large"></i>
        </button>
        <button @click="viewMode = 'list'" :class="['join-item', 'btn', 'btn-sm', {'btn-primary': viewMode === 'list'}]">
          <i class="fas fa-list"></i>
        </button>
      </div>
    </div>
    
    <!-- 插件列表 -->
    <div v-if="isLoading" class="text-center p-8">
      <i class="fas fa-spinner fa-spin text-2xl"></i>
      <p class="mt-2">正在加载插件...</p>
    </div>
    
    <div v-else-if="plugins.length > 0" class="space-y-4">
      <div class="flex justify-end mb-4">
        <button @click="$emit('show-upload')" class="btn btn-primary btn-sm">
          <i class="fas fa-upload mr-2"></i>
          上传插件
        </button>
      </div>
      
      <!-- 卡片视图 -->
      <div v-if="viewMode === 'card'" class="grid grid-cols-1 lg:grid-cols-2 gap-4">
        <div 
          v-for="plugin in plugins" 
          :key="plugin.metadata.id"
          class="card bg-base-100 shadow-lg hover:shadow-xl transition-shadow"
        >
          <div class="card-body">
            <div class="flex items-center gap-3">
              <div class="avatar">
                <div class="w-12 h-12 rounded-lg bg-primary/10 flex items-center justify-center">
                  <i class="fas fa-puzzle-piece text-primary text-xl"></i>
                </div>
              </div>
              <div class="flex-1">
                <h3 class="card-title text-lg">{{ plugin.metadata.name }}</h3>
                <span class="badge badge-ghost badge-sm">v{{ plugin.metadata.version }}</span>
              </div>
              <div class="form-control">
                <label class="label cursor-pointer">
                  <input 
                    type="checkbox" 
                    class="toggle toggle-primary toggle-sm" 
                    :checked="plugin.status === 'Enabled'"
                    @change="togglePlugin(plugin)"
                    :disabled="plugin.is_toggling"
                  />
                </label>
              </div>
            </div>

            <p class="text-sm mt-2 h-16">{{ plugin.metadata.description }}</p>

            <div class="flex flex-wrap gap-2 mt-2">
              <span class="badge badge-outline badge-xs">{{ plugin.metadata.author }}</span>
              <span 
                v-for="perm in plugin.metadata.permissions" 
                :key="perm"
                class="badge badge-warning badge-xs"
              >
                {{ perm }}
              </span>
            </div>

            <div class="card-actions justify-between items-center mt-4">
              <div class="flex gap-1">
                <span 
                  :class="['badge badge-sm', plugin.status === 'Enabled' ? 'badge-success' : 'badge-ghost']"
                >
                  {{ plugin.status }}
                </span>
                <span v-if="plugin.last_error" class="badge badge-error badge-sm" :title="plugin.last_error">
                  <i class="fas fa-exclamation-triangle"></i>
                </span>
              </div>
              <div class="flex gap-1">
                <button 
                  v-if="plugin.status === 'Enabled'"
                  @click="openTestModal(plugin)"
                  class="btn btn-xs btn-primary"
                  title="测试插件"
                >
                  <i class="fas fa-play mr-1"></i>
                  测试
                </button>
                <button 
                  @click="editPlugin(plugin)"
                  class="btn btn-xs btn-outline"
                  title="编辑"
                >
                  <i class="fas fa-edit"></i>
                </button>
                <button 
                  @click="viewPluginInfo(plugin)"
                  class="btn btn-xs btn-outline"
                  title="详情"
                >
                  <i class="fas fa-info"></i>
                </button>
              </div>
            </div>
          </div>
        </div>
      </div>
      
      <!-- 列表视图 -->
      <div v-if="viewMode === 'list'" class="overflow-x-auto">
        <table class="table w-full">
          <thead>
            <tr>
              <th class="w-1/12">启用</th>
              <th>名称</th>
              <th>版本</th>
              <th>作者</th>
              <th>描述</th>
              <th>状态</th>
              <th>操作</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="plugin in plugins" :key="plugin.metadata.id">
              <td>
                <input 
                  type="checkbox" 
                  class="toggle toggle-primary toggle-sm" 
                  :checked="plugin.status === 'Enabled'"
                  @change="togglePlugin(plugin)"
                  :disabled="plugin.is_toggling"
                />
              </td>
              <td>
                <div class="flex items-center gap-2">
                  <i class="fas fa-puzzle-piece text-primary"></i>
                  <span class="font-semibold">{{ plugin.metadata.name }}</span>
                </div>
              </td>
              <td><span class="badge badge-ghost badge-sm">v{{ plugin.metadata.version }}</span></td>
              <td><span class="badge badge-outline badge-xs">{{ plugin.metadata.author }}</span></td>
              <td class="text-sm">{{ plugin.metadata.description }}</td>
              <td>
                <div class="flex flex-col gap-1">
                  <span :class="['badge badge-sm', plugin.status === 'Enabled' ? 'badge-success' : 'badge-ghost']">
                    {{ plugin.status }}
                  </span>
                  <span v-if="plugin.last_error" class="badge badge-error badge-sm" :title="plugin.last_error">
                    <i class="fas fa-exclamation-triangle"></i>
                  </span>
                </div>
              </td>
              <td>
                <div class="flex gap-1">
                  <button 
                    v-if="plugin.status === 'Enabled'"
                    @click="openTestModal(plugin)"
                    class="btn btn-xs btn-primary"
                    title="测试插件"
                  >
                    <i class="fas fa-play"></i>
                  </button>
                  <button 
                    @click="editPlugin(plugin)"
                    class="btn btn-xs btn-outline"
                    title="编辑"
                  >
                    <i class="fas fa-edit"></i>
                  </button>
                  <button 
                    @click="viewPluginInfo(plugin)"
                    class="btn btn-xs btn-outline"
                    title="详情"
                  >
                    <i class="fas fa-info"></i>
                  </button>
                </div>
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>
    
    <div v-else class="text-center p-8">
      <i class="fas fa-plug text-4xl text-base-content/30 mb-4"></i>
      <p class="text-lg font-semibold">暂无插件工具</p>
      <p class="text-base-content/70 mt-2">前往插件管理创建 Agent 工具插件</p>
      <button @click="goToPluginManagement" class="btn btn-primary mt-4">
        <i class="fas fa-plus mr-2"></i>
        创建插件工具
      </button>
    </div>



    <!-- 统一测试组件 -->
    <UnifiedToolTest
      v-model="showTestModal"
      tool-type="plugin"
      :tool-name="testingPlugin?.metadata.name || ''"
      :tool-description="testingPlugin?.metadata.description"
      :tool-version="testingPlugin?.metadata.version"
      :tool-category="testingPlugin?.metadata.category"
      :execution-info="{
        type: 'plugin',
        id: testingPlugin?.metadata.id
      }"
    />
    <!-- 插件代码编辑器 -->
    <PluginCodeEditorDialog
      ref="codeEditorDialogRef"
      :editing-plugin="editingPlugin"
      :new-plugin-metadata="editPluginMetadata"
      :is-editing="isEditing"
      :saving="isSavingWait"
      :code-error="codeError"
      :is-fullscreen-editor="isFullscreenEditor"
      :sub-categories="subCategories"
      :show-ai-panel="showAiPanel"
      :ai-messages="aiChatMessages"
      :ai-streaming="aiChatStreaming"
      :ai-streaming-content="aiChatStreamingContent"
      :selected-code-ref="selectedCodeRef"
      :selected-test-result-ref="selectedTestResultRef"
      :plugin-testing="pluginTesting"
      :is-preview-mode="isPreviewMode"
      @update:new-plugin-metadata="val => editPluginMetadata = val"
      @format-code="formatCode"
      @copy-plugin="copyPlugin"
      @toggle-fullscreen="toggleFullscreenEditor"
      @enable-editing="enableEditing"
      @cancel-editing="cancelEditing"
      @save-plugin="savePlugin"
      @close="closeCodeEditorDialog"
      @toggle-ai-panel="showAiPanel = !showAiPanel"
      @send-ai-message="sendAiChatMessage"
      @clear-code-ref="selectedCodeRef = null"
      @clear-test-result-ref="selectedTestResultRef = null"
      @ai-quick-action="handleAiQuickAction"
      @apply-ai-code="(code) => { pluginCode = code; if (codeEditorView) codeEditorView.dispatch({ changes: { from: 0, to: codeEditorView.state.doc.length, insert: code } }) }"
      @exit-preview-mode="isPreviewMode = false"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, nextTick, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { dialog } from '@/composables/useDialog'
import { useI18n } from 'vue-i18n'
import UnifiedToolTest from './UnifiedToolTest.vue'
import PluginCodeEditorDialog from '@/components/PluginManagement/PluginCodeEditorDialog.vue'
import { mainCategories, type SubCategory, type NewPluginMetadata, type AiChatMessage, type CodeReference, type TestResultReference } from '@/components/PluginManagement/types'
import { EditorView } from '@codemirror/view'
import { EditorState, Compartment } from '@codemirror/state'
import { basicSetup } from 'codemirror'
import { javascript } from '@codemirror/lang-javascript'
import { oneDark } from '@codemirror/theme-one-dark'

const { t } = useI18n()

// 类型定义
interface PluginMetadata {
  id: string
  name: string
  version: string
  description: string
  author: string
  main_category: string
  category: string
  permissions: string[]
}

interface PluginRecord {
  metadata: PluginMetadata
  path: string
  status: string
  last_error: string | null
  is_toggling?: boolean
  is_testing?: boolean
}

// 定义事件
defineEmits<{
  (e: 'show-upload'): void
}>()

// 状态
const plugins = ref<PluginRecord[]>([])
const isLoading = ref(false)
const viewMode = ref('list')

// 测试相关状态
const showTestModal = ref(false)
const testingPlugin = ref<PluginRecord | null>(null)
const testParamsJson = ref('{}')
const testResult = ref('')
const isTesting = ref(false)

// 方法
async function fetchPlugins() {
  isLoading.value = true
  try {
    const response = await invoke<any>('list_plugins')
    if (response.success && response.data) {
      plugins.value = response.data.filter((plugin: PluginRecord) => 
        plugin.metadata.main_category === 'agent'
      )
    }
  } catch (error) {
    console.error('Failed to fetch agent tool plugins:', error)
    plugins.value = []
  } finally {
    isLoading.value = false
  }
}

async function refresh() {
  await fetchPlugins()
}

function goToPluginManagement() {
  window.location.hash = '#/plugin-management'
}

async function togglePlugin(plugin: PluginRecord) {
  plugin.is_toggling = true
  try {
    const isEnabled = plugin.status === 'Enabled'
    if (isEnabled) {
      await invoke('disable_plugin', { pluginId: plugin.metadata.id })
      dialog.toast.success(`已禁用插件: ${plugin.metadata.name}`)
    } else {
      await invoke('enable_plugin', { pluginId: plugin.metadata.id })
      dialog.toast.success(`已启用插件: ${plugin.metadata.name}`)
    }
    await fetchPlugins()
  } catch (error: any) {
    console.error(`Failed to toggle plugin ${plugin.metadata.id}:`, error)
    dialog.toast.error(`切换插件状态失败: ${error}`)
  } finally {
    plugin.is_toggling = false
  }
}



// 打开高级测试模态框
function openTestModal(plugin: PluginRecord) {
  testingPlugin.value = { ...plugin }
  testParamsJson.value = '{}'
  testResult.value = ''
  nextTick(() => {
    showTestModal.value = true
  })
}

function closeTestModal() {
  showTestModal.value = false
  setTimeout(() => {
    testingPlugin.value = null
    testParamsJson.value = '{}'
    testResult.value = ''
  }, 300)
}

async function runTest() {
  if (!testingPlugin.value) {
    dialog.toast.error('请选择要测试的插件')
    return
  }

  let inputs: any = {}
  if (testParamsJson.value.trim()) {
    try {
      inputs = JSON.parse(testParamsJson.value)
    } catch (e) {
      dialog.toast.error('参数 JSON 格式错误，请检查')
      return
    }
  }

  isTesting.value = true
  testResult.value = '正在执行测试...'
  
  try {
    const result = await invoke<any>('unified_execute_tool', {
      toolName: `plugin::${testingPlugin.value.metadata.id}`,
      inputs,
      context: null,
      timeout: 120,
    })

    if (result.success) {
      testResult.value = typeof result.output === 'string'
        ? result.output
        : JSON.stringify(result.output, null, 2)
      dialog.toast.success('插件测试完成')
    } else {
      testResult.value = `测试失败: ${result.error || '未知错误'}`
      dialog.toast.error('插件测试失败')
    }
  } catch (error: any) {
    console.error('Failed to test plugin:', error)
    testResult.value = `测试失败: ${error?.message || String(error)}`
    dialog.toast.error('插件测试失败')
  } finally {
    isTesting.value = false
  }
}

async function editPlugin(plugin: PluginRecord) {
  try {
    const response = await invoke<any>('get_plugin_code', { pluginId: plugin.metadata.id })
    if (response.success) {
       pluginCode.value = response.data || ''
       originalCode.value = response.data || ''
       editingPlugin.value = plugin
       
       // Populate metadata for display
       editPluginMetadata.value = {
         id: plugin.metadata.id,
         name: plugin.metadata.name,
         version: plugin.metadata.version,
         author: plugin.metadata.author,
         mainCategory: plugin.metadata.main_category,
         category: plugin.metadata.category,
         default_severity: 'medium',
         description: plugin.metadata.description,
         tagsString: (plugin.metadata.permissions || []).join(', ')
       }
       
       isEditing.value = false
       codeError.value = ''
       codeEditorDialogRef.value?.showDialog()
       await nextTick()
       initCodeEditor()
    } else {
       dialog.toast.error(response.error || '获取代码失败')
    }
  } catch(e: any) {
     dialog.toast.error('获取代码失败: ' + e)
  }
}

// Editor Logic
const codeEditorDialogRef = ref()
const editingPlugin = ref<any>(null)
const editPluginMetadata = ref<NewPluginMetadata>({
  id: '', name: '', version: '1.0.0', author: '',
  mainCategory: 'traffic', category: 'custom',
  default_severity: 'medium', description: '', tagsString: ''
})
const isEditing = ref(false)
const isSavingWait = ref(false)
const codeError = ref('')
const isFullscreenEditor = ref(false)
const pluginCode = ref('')
const originalCode = ref('')

// AI and Test related states for PluginCodeEditorDialog
const showAiPanel = ref(false)
const aiChatMessages = ref<AiChatMessage[]>([])
const aiChatStreaming = ref(false)
const aiChatStreamingContent = ref('')
const selectedCodeRef = ref<CodeReference | null>(null)
const selectedTestResultRef = ref<TestResultReference | null>(null)
const pluginTesting = ref(false)
const isPreviewMode = ref(false)

let codeEditorView: EditorView | null = null
let fullscreenCodeEditorView: EditorView | null = null
const codeEditorReadOnly = new Compartment()

const subCategories = computed<SubCategory[]>(() => {
  if (editPluginMetadata.value.mainCategory === 'traffic') {
    return [
      { value: 'sqli', label: 'SQL注入', icon: 'fas fa-database' },
      { value: 'command_injection', label: '命令注入', icon: 'fas fa-terminal' },
      { value: 'xss', label: '跨站脚本', icon: 'fas fa-code' },
      { value: 'custom', label: '自定义', icon: 'fas fa-wrench' }
    ]
  } else if (editPluginMetadata.value.mainCategory === 'agent') {
    return [
      { value: 'scanner', label: '扫描工具', icon: 'fas fa-radar' },
      { value: 'analyzer', label: '分析工具', icon: 'fas fa-microscope' },
      { value: 'utility', label: '实用工具', icon: 'fas fa-toolbox' },
      { value: 'custom', label: '自定义', icon: 'fas fa-wrench' }
    ]
  }
  return []
})

function initCodeEditor() {
  if (codeEditorView) codeEditorView.destroy()
  const El = codeEditorDialogRef.value?.codeEditorContainerRef
  if (!El) return

  const state = EditorState.create({
    doc: pluginCode.value,
    extensions: [
      basicSetup,
      javascript(),
      oneDark,
      EditorView.theme({
        "&": {
          fontSize: "var(--font-size-base, 14px)"
        }
      }),
      EditorView.updateListener.of((v) => {
        if (v.docChanged) {
          pluginCode.value = v.state.doc.toString()
          if (isFullscreenEditor.value && fullscreenCodeEditorView && fullscreenCodeEditorView.state.doc.toString() !== pluginCode.value) {
            fullscreenCodeEditorView.dispatch({
              changes: { from: 0, to: fullscreenCodeEditorView.state.doc.length, insert: pluginCode.value }
            })
          }
        }
      }),
      codeEditorReadOnly.of(EditorState.readOnly.of(true))
    ]
  })

  codeEditorView = new EditorView({
    state,
    parent: El
  })
}

function enableEditing() {
  isEditing.value = true
  if (codeEditorView) {
    codeEditorView.dispatch({
      effects: codeEditorReadOnly.reconfigure(EditorState.readOnly.of(false))
    })
  }
  if (fullscreenCodeEditorView) {
    // Similarly for fullscreen
  }
}

function cancelEditing() {
  pluginCode.value = originalCode.value
  isEditing.value = false
  if (codeEditorView) {
    codeEditorView.dispatch({
      changes: { from: 0, to: codeEditorView.state.doc.length, insert: originalCode.value },
      effects: codeEditorReadOnly.reconfigure(EditorState.readOnly.of(true))
    })
  }
}

function closeCodeEditorDialog() {
  if (codeEditorView) {
    codeEditorView.destroy()
    codeEditorView = null
  }
  editingPlugin.value = null
  isEditing.value = false
}

function formatCode() {
  // Simple format via string manipulation or use prettier if available (not included here)
  // Just trim for now as in PluginManagement example
  const lines = pluginCode.value.split('\n')
  const formatted = lines.map(line => line.trimEnd()).join('\n')
  pluginCode.value = formatted
  if (codeEditorView) {
    codeEditorView.dispatch({
       changes: { from: 0, to: codeEditorView.state.doc.length, insert: formatted }
    })
  }
}

async function copyPlugin() {
  try {
    const metadata = {
      id: editPluginMetadata.value.id,
      name: editPluginMetadata.value.name,
      version: editPluginMetadata.value.version,
      author: editPluginMetadata.value.author || 'Unknown',
      category: editPluginMetadata.value.category,
      default_severity: editPluginMetadata.value.default_severity,
      description: editPluginMetadata.value.description || '',
      tags: editPluginMetadata.value.tagsString.split(',').map(s => s.trim()).filter(Boolean)
    }

    const metadataComment = `/**
 * @plugin ${metadata.id}
 * @name ${metadata.name}
 * @version ${metadata.version}
 * @author ${metadata.author}
 * @category ${metadata.category}
 * @default_severity ${metadata.default_severity}
 * @tags ${metadata.tags.join(', ')}
 * @description ${metadata.description}
 */
`
    const codeWithoutMetadata = pluginCode.value.replace(/\/\*\*\s*[\s\S]*?\*\/\s*/, '')
    const fullCode = metadataComment + '\n' + codeWithoutMetadata
    await navigator.clipboard.writeText(fullCode)
    dialog.toast.success(t('plugins.copySuccess', '已复制'))
  } catch (error) {
    console.error('Failed to copy plugin:', error)
    dialog.toast.error(t('plugins.copyFailed', '复制失败'))
  }
}

function toggleFullscreenEditor() {
  // Simplification: ignore fullscreen logic for PluginToolsTab to be concise, 
  // or just toggle flag but not implement the teleport view
  isFullscreenEditor.value = !isFullscreenEditor.value
  // Note: Since I didn't verify Fullscreen overlay template code, I actually relying on PluginCodeEditorDialog's internal Fullscreen support?
  // PluginCodeEditorDialog HAS <Teleport> in IT? 
  // YES. PluginCodeEditorDialog has the teleport logic inside it!
  // Wait, no. PluginManagement had the Teleport logic. PluginCodeEditorDialog ONLY has the dialog.
  // PluginCodeEditorDialog.vue lines 149-210 show Teleport IS in PluginCodeEditorDialog.vue!
  // So I don't need to copy template for fullscreen!
  // BUT PluginCodeEditorDialog emits `toggleFullscreen` and expects parent to handle state and init logic.
  // The Teleport content refers `fullscreenCodeEditorContainerRef` which is EXPOSED by PluginCodeEditorDialog.
  
  // To fully support fullscreen:
  const El = codeEditorDialogRef.value?.fullscreenCodeEditorContainerRef
  if (isFullscreenEditor.value) {
      nextTick(() => {
          if (!El) return
          if (fullscreenCodeEditorView) fullscreenCodeEditorView.destroy()
          
          fullscreenCodeEditorView = new EditorView({
            state: EditorState.create({
                doc: pluginCode.value,
                extensions: [
                    basicSetup,
                    javascript(),
                    oneDark,
                    EditorView.theme({
                      "&": {
                        fontSize: "var(--font-size-base, 14px)"
                      }
                    }),
                     EditorView.updateListener.of((v) => {
                        if (v.docChanged) {
                            pluginCode.value = v.state.doc.toString()
                            // Sync back to normal editor if needed
                         }
                     }),
                    EditorState.readOnly.of(!isEditing.value)
                ]
            }),
            parent: El
          })
      })
  } else {
      if (fullscreenCodeEditorView) {
          fullscreenCodeEditorView.destroy()
          fullscreenCodeEditorView = null
      }
  }
}

async function savePlugin() {
  if (!editingPlugin.value) return
  isSavingWait.value = true
  codeError.value = ''
  
  try {
     const metadata = {
        id: editPluginMetadata.value.id,
        name: editPluginMetadata.value.name,
        version: editPluginMetadata.value.version,
        author: editPluginMetadata.value.author,
        main_category: editPluginMetadata.value.mainCategory,
        category: editPluginMetadata.value.category,
        description: editPluginMetadata.value.description,
        default_severity: editPluginMetadata.value.default_severity,
        tags: editPluginMetadata.value.tagsString.split(',').map(s=>s.trim())
     }
     
     const metadataComment = `/**
 * @plugin ${metadata.id}
 * @name ${metadata.name}
 * @version ${metadata.version}
 * @author ${metadata.author}
 * @category ${metadata.category}
 * @default_severity ${metadata.default_severity}
 * @tags ${metadata.tags.join(', ')}
 * @description ${metadata.description}
 */
`
     const codeWithoutMetadata = pluginCode.value.replace(/\/\*\*\s*[\s\S]*?\*\/\s*/, '')
     const fullCode = metadataComment + '\n' + codeWithoutMetadata
     
     const response = await invoke<any>('update_plugin', {
       metadata,
       pluginCode: fullCode
     })
     
     if (response.success) {
        originalCode.value = pluginCode.value
        isEditing.value = false
        dialog.toast.success('保存成功')
        if (codeEditorView) {
             codeEditorView.dispatch({
                effects: codeEditorReadOnly.reconfigure(EditorState.readOnly.of(true))
             })
        }
        await refresh()
     } else {
        codeError.value = response.error || '保存失败'
     }
  } catch(e: any) {
     codeError.value = e.message || '保存失败'
  } finally {
     isSavingWait.value = false
  }
}

// AI related methods
async function handleAiQuickAction(action: string) {
  const actions: Record<string, string> = {
    'explain': '请解释这段插件代码的功能和工作原理',
    'optimize': '请优化这段代码，提高性能和可读性',
    'fix': '请检查并修复这段代码中可能存在的问题'
  }
  const message = actions[action] || action
  await sendAiChatMessage(message)
}

async function sendAiChatMessage(message: string) {
  if (!message.trim() || aiChatStreaming.value) return
  
  // Get current references
  const codeRef = selectedCodeRef.value
  const testResultRef = selectedTestResultRef.value
  
  // Get latest code from editor for default context
  const editorView = isFullscreenEditor.value ? fullscreenCodeEditorView : codeEditorView
  const latestCode = editorView ? editorView.state.doc.toString() : pluginCode.value
  
  // Determine code reference to use (explicit or default to full code)
  const finalCodeRef: CodeReference = codeRef || {
    code: latestCode,
    preview: latestCode.split('\n').slice(0, 5).join('\n') + (latestCode.split('\n').length > 5 ? '\n...' : ''),
    startLine: 1,
    endLine: latestCode.split('\n').length,
    isFullCode: true
  }
  
  // Build history for backend
  const history = aiChatMessages.value.map(msg => ({
    role: msg.role,
    content: msg.content
  }))

  // Add user message with references to UI
  aiChatMessages.value.push({ 
    role: 'user', 
    content: message,
    codeRef: finalCodeRef,
    testResultRef: testResultRef || undefined
  })
  
  aiChatStreaming.value = true
  aiChatStreamingContent.value = ''
  
  const streamId = `plugin_edit_${Date.now()}`
  
  try {
    // Build system prompt
    const isAgentPlugin = editPluginMetadata.value.mainCategory === 'agent'
    const baseSystemPrompt = await invoke<string>('get_combined_plugin_prompt_api', {
      pluginType: isAgentPlugin ? 'agent' : 'traffic',
      vulnType: 'custom',
      severity: 'medium'
    })
    
    const agentInstructions = `
you are a senior code editor Agent, writing a plugin for the "Sentinel AI" security testing platform.
you are goal is to modify the TypeScript code directly and efficiently according to the user's needs.

[Behavior Guidelines]:
1. **Direct Modification**: If the user requires modifying the code, please provide the modified code block directly.
2. **Partial vs Full**:
   - If the user only wants to modify a specific function or add a small logic, you can only return the relevant code block (wrapped in \`\`\`typescript).
   - If the user requires global structure adjustments or explicitly requests, please return the complete code.
3. **Keep the Context**: When modifying, please refer to the user's [full code context] to ensure that the new code is compatible with the existing logic and type definitions.
4. **Security First**: As a security plugin, the code must be robust and avoid injection risks and performance bottlenecks.
5. **Simple Communication**: No need to add too many开场白, directly state what you have modified and then provide the code.
`
    const systemPrompt = `${baseSystemPrompt}\n\n${agentInstructions}`
    
    // Build user prompt with context
    let userPrompt = message
    const contextParts: string[] = []
    
    if (finalCodeRef.isFullCode) {
      contextParts.push(`[Current Full Plugin Code]:\n\`\`\`typescript\n${finalCodeRef.code}\n\`\`\``)
    } else {
      contextParts.push(`[Current Focused Code Block] (Lines ${finalCodeRef.startLine}-${finalCodeRef.endLine}):\n\`\`\`typescript\n${finalCodeRef.code}\n\`\`\``)
      contextParts.push(`[Full Code Context]:\n\`\`\`typescript\n${latestCode}\n\`\`\``)
    }
    
    if (testResultRef) {
      contextParts.push(`[Latest Plugin Test Result]:\n${testResultRef.preview}`)
    }
    
    const instruction = "\n\nPlease modify the code according to the above code context and my needs. Please return the code directly."
    
    if (contextParts.length > 0) {
      userPrompt = `${contextParts.join('\n\n')}\n\n[User Requirement]: ${message}${instruction}`
    } else {
      userPrompt = `${message}${instruction}`
    }
    
    // Clear references after sending
    selectedCodeRef.value = null
    selectedTestResultRef.value = null
    
    let generatedContent = ''
    
    const unlistenDelta = await listen('plugin_gen_delta', (event: any) => {
      if (event.payload.stream_id === streamId) {
        generatedContent += event.payload.delta || ''
        aiChatStreamingContent.value = generatedContent
      }
    })
    
    const unlistenComplete = await listen('plugin_gen_complete', (event: any) => {
      if (event.payload.stream_id === streamId) {
        generatedContent = event.payload.content || generatedContent
        aiChatStreaming.value = false
        aiChatStreamingContent.value = ''
        
        // Simple markdown extraction
        const codeBlocks: string[] = []
        const codeBlockRegex = /```(?:typescript|ts|javascript|js)?\n?([\s\S]*?)```/g
        let match
        while ((match = codeBlockRegex.exec(generatedContent)) !== null) {
          codeBlocks.push(match[1].trim())
        }
        
        aiChatMessages.value.push({ 
          role: 'assistant', 
          content: generatedContent,
          codeBlock: codeBlocks[0],
          codeBlocks: codeBlocks
        })
      }
    })
    
    const unlistenError = await listen('plugin_gen_error', (event: any) => {
      if (event.payload.stream_id === streamId) {
        aiChatMessages.value.push({ role: 'assistant', content: `❌ ${event.payload.error || 'AI 处理失败'}` })
        aiChatStreaming.value = false
        aiChatStreamingContent.value = ''
      }
    })
    
    await invoke('generate_plugin_stream', {
      request: {
        stream_id: streamId,
        message: userPrompt,
        system_prompt: systemPrompt,
        service_name: 'default',
        history: history
      }
    })
    
    setTimeout(() => {
      unlistenDelta()
      unlistenComplete()
      unlistenError()
    }, 180000)
    
  } catch (error) {
    aiChatMessages.value.push({ 
      role: 'assistant', 
      content: `❌ ${error instanceof Error ? error.message : 'AI 处理失败'}` 
    })
    aiChatStreaming.value = false
    aiChatStreamingContent.value = ''
  }
}

function viewPluginInfo(plugin: PluginRecord) {
  const info = `
插件名称: ${plugin.metadata.name}
版本: ${plugin.metadata.version}
作者: ${plugin.metadata.author}
描述: ${plugin.metadata.description}
权限: ${plugin.metadata.permissions.join(', ')}
状态: ${plugin.status}
${plugin.last_error ? `错误: ${plugin.last_error}` : ''}
  `.trim()
  
  dialog.info(info)
}

// 暴露方法供父组件调用
defineExpose({ refresh })

onMounted(() => {
  fetchPlugins()
})
</script>

