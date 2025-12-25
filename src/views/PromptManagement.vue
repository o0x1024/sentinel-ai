  
  <template>
  <div class="page-content-padded safe-top h-full flex gap-4">
    <!-- 第一列：分类选择 -->
    <div class="w-56 card bg-base-100 shadow-md overflow-hidden flex flex-col">
      <div class="card-body p-4 pb-3">
        <!-- Prompt分类选择器 -->
        <div class="mb-4">
          <h4 class="card-title text-xs mb-2">{{ t('promptMgmt.categories.promptCategory') }}</h4>
          <select v-model="selectedCategory" class="select select-sm select-bordered w-full">
            <option v-for="cat in promptCategories" :key="cat.value" :value="cat.value">
              {{ cat.label }}
            </option>
          </select>
          <div class="text-xs opacity-60 mt-1">{{ promptCategories.find(c => c.value === selectedCategory)?.description }}</div>
        </div>
        
        <!-- 系统级提示创建按钮 -->
        <div v-if="selectedCategory === 'System'" class="mt-4 pt-4 border-t">
          <h3 class="card-title text-sm">{{ t('promptMgmt.systemPrompts.title') }}</h3>
          <div class="text-xs opacity-70 mt-1">{{ t('promptMgmt.systemPrompts.description') }}</div>
          <div class="mt-2 flex flex-col gap-1">
            <button class="btn btn-xs btn-outline w-full" @click="createIntentClassifierTemplate">
              {{ t('promptMgmt.systemPrompts.intentClassifier') }}
            </button>
            <button class="btn btn-xs btn-outline w-full" @click="createSystemPromptTemplate">
              {{ t('promptMgmt.systemPrompts.generalSystemPrompt') }}
            </button>
          </div>
        </div>
        
        <!-- 应用级提示 - 仅在应用分类时显示 -->
        <div v-if="selectedCategory === 'Application'">
          <h3 class="card-title text-sm">{{ t('promptMgmt.applicationPrompts.title') }}</h3>
          <div class="text-xs opacity-70 mt-1">{{ t('promptMgmt.applicationPrompts.description') }}</div>
          <div class="mt-2 flex flex-col gap-1">
            <button class="btn btn-xs btn-outline" @click="createPluginGenerationTemplate">
              {{ t('promptMgmt.applicationPrompts.pluginGenPassive') }}
            </button>
            <button class="btn btn-xs btn-outline" @click="createAgentPluginGenerationTemplate">
              {{ t('promptMgmt.applicationPrompts.pluginGenAgent') }}
            </button>
            <button class="btn btn-xs btn-outline" @click="createPluginFixTemplate">
              {{ t('promptMgmt.applicationPrompts.pluginFixPassive') }}
            </button>
            <button class="btn btn-xs btn-outline" @click="createAgentPluginFixTemplate">
              {{ t('promptMgmt.applicationPrompts.pluginFixAgent') }}
            </button>
            <button class="btn btn-xs btn-outline" @click="createVisionExplorerVisionTemplate">
              {{ t('promptMgmt.applicationPrompts.visionMultimodal') }}
            </button>
            <button class="btn btn-xs btn-outline" @click="createVisionExplorerTextTemplate">
              {{ t('promptMgmt.applicationPrompts.visionText') }}
            </button>
          </div>
        </div>
        
        <!-- 用户自定义 - 仅在用户自定义分类时显示 -->
        <div v-if="selectedCategory === 'UserDefined'">
          <h3 class="card-title text-sm">{{ t('promptMgmt.userDefinedPrompts.title') }}</h3>
          <div class="text-xs opacity-70 mt-1">{{ t('promptMgmt.userDefinedPrompts.description') }}</div>
        </div>
        
      </div>
    </div>

    <!-- 第二列：分组管理 + 模板列表 -->
    <div class="w-72 card bg-base-100 shadow-md overflow-hidden flex flex-col">
      <div class="card-body p-4 pb-2">
        <!-- 搜索框 -->
        <input v-model.trim="searchQuery" class="input input-sm input-bordered w-full mb-3" :placeholder="$t('promptMgmt.searchTemplates') as string" />
        
        <!-- 当前激活状态 -->
        <div class="text-xs opacity-60 flex items-center gap-2 mb-2" v-if="selectedCategory === 'System'">
          <span>{{ $t('promptMgmt.active') }}:</span>
          <span v-if="activePromptId" class="badge badge-success badge-xs">#{{ activePromptId }}</span>
          <span v-else class="opacity-50">{{ $t('promptMgmt.none') }}</span>
        </div>
        
        <div class="divider my-1"></div>
        
        <!-- 模板列表标题 -->
        <div class="text-xs font-medium mb-2">{{ t('promptMgmt.templateList.count', { count: filteredTemplates.length }) }}</div>
      </div>

      <!-- 模板列表 -->
      <div class="px-4 pb-4 flex-1 overflow-auto">
        <div class="grid grid-cols-1 gap-2">
          <button
            v-for="tpl in filteredTemplates"
            :key="tpl.id"
            class="btn btn-outline btn-sm justify-start normal-case w-full"
            :class="{
              '!btn-primary text-white': editingTemplate?.id === tpl.id,
            }"
            @click="onLoadWithGuard(tpl)"
          >
            <div class="w-full flex items-center gap-2">
              <div class="truncate flex-1 text-left">
                <div class="font-medium text-xs truncate flex items-center gap-1">
                  <span v-if="tpl.is_active" class="inline-block w-2 h-2 rounded-full bg-success" :title="t('promptMgmt.templateList.enabledTitle')"></span>
                  {{ tpl.name }}
                </div>
                <div class="text-[10px] opacity-70 truncate">
                  #{{ tpl.id }} · {{ tpl.template_type || 'Custom' }}
                </div>
              </div>
              <span v-if="tpl.is_active" class="badge badge-success badge-xs">{{ t('promptMgmt.templateList.enabled') }}</span>
              <span v-else-if="tpl.id === activePromptId" class="badge badge-success badge-xs">{{ $t('promptMgmt.activeBadge') }}</span>
              <span v-else-if="tpl.is_default" class="badge badge-outline badge-xs">{{ $t('promptMgmt.default') }}</span>
            </div>
          </button>
        </div>
        
        <!-- 空状态 -->
        <div v-if="filteredTemplates.length === 0" class="text-center py-8 text-xs opacity-50">
          {{ t('promptMgmt.templateList.empty') }}
        </div>
      </div>
    </div>

    <!-- 第三列：工具栏 + 编辑/预览 -->
    <div class="flex-1 flex flex-col gap-3">
      <!-- 工具栏 -->
      <div class="card bg-base-100 shadow-md">
        <div class="card-body py-3 px-4">
          <div  class="flex flex-wrap items-center gap-3">
            <div class="text-sm opacity-70" v-if="isDirty">
              <span class="ml-2 badge badge-warning badge-sm">{{ $t('promptMgmt.unsavedBadge') }}</span>
            </div>
            <button class="btn btn-success btn-sm hover:brightness-95 active:brightness-90 shadow-sm" @click="onNewWithGuard">{{ $t('common.create') }}</button>
            <button class="btn btn-success btn-sm hover:brightness-95 active:brightness-90 shadow-sm" :disabled="!editingTemplate" @click="saveTemplate">{{ $t('common.save') }}</button>
            <button v-if="selectedCategory === 'System'" class="btn btn-outline btn-sm hover:brightness-95 active:brightness-90 shadow-sm" :disabled="!editingTemplate?.id" @click="activateTemplate">{{ $t('promptMgmt.active') }}</button>
            <button class="btn btn-error btn-sm hover:brightness-95 active:brightness-90 shadow-sm" :disabled="!editingTemplate?.id" @click="removeTemplate">{{ $t('common.delete') }}</button>
            <div class="ml-auto flex items-center gap-2 text-sm opacity-70">
              <span v-if="statusText==='Loading...'" class="loading loading-spinner loading-xs"></span>
              <span>{{ statusText }}</span>
            </div>
          </div>
        </div>
      </div>

      <div class="grid grid-cols-2 gap-4 flex-1 overflow-hidden">
        <!-- 编辑器卡片 -->
        <div class="card bg-base-100 shadow-md h-full overflow-hidden">
          <div class="card-body p-4 h-full overflow-hidden" v-if="editingTemplate">
            <input v-model="editingTemplate.name" class="input input-sm input-bordered mb-2" :placeholder="$t('promptMgmt.namePlaceholder') as string" />
            <textarea v-model="editingTemplate.description" class="textarea textarea-bordered mb-2" rows="2" :placeholder="$t('promptMgmt.descPlaceholder') as string"></textarea>
            
            <!-- 新增字段 -->
            <div class="grid grid-cols-2 gap-2 mb-2">
              <div>
                <label class="label label-text text-xs">{{ t('promptMgmt.editor.templateType') }}</label>
                <select v-model="editingTemplate.template_type" class="select select-xs select-bordered w-full">
                  <option value="SystemPrompt">{{ t('promptMgmt.templateTypes.systemPrompt') }}</option>
                  <option value="IntentClassifier">{{ t('promptMgmt.templateTypes.intentClassifier') }}</option>
                  <option value="Planner">{{ t('promptMgmt.templateTypes.planner') }}</option>
                  <option value="Executor">{{ t('promptMgmt.templateTypes.executor') }}</option>
                  <option value="Replanner">{{ t('promptMgmt.templateTypes.replanner') }}</option>
                  <option value="Evaluator">{{ t('promptMgmt.templateTypes.evaluator') }}</option>
                  <option value="ReportGenerator">{{ t('promptMgmt.templateTypes.reportGenerator') }}</option>
                  <option value="PluginGeneration">{{ t('promptMgmt.templateTypes.pluginGenPassive') }}</option>
                  <option value="AgentPluginGeneration">{{ t('promptMgmt.templateTypes.pluginGenAgent') }}</option>
                  <option value="PluginFix">{{ t('promptMgmt.templateTypes.pluginFixPassive') }}</option>
                  <option value="AgentPluginFix">{{ t('promptMgmt.templateTypes.pluginFixAgent') }}</option>
                  <option value="PluginVulnSpecific">{{ t('promptMgmt.templateTypes.pluginVulnSpecific') }}</option>
                  <option value="VisionExplorerVision">{{ t('promptMgmt.templateTypes.visionExplorerVision') }}</option>
                  <option value="VisionExplorerText">{{ t('promptMgmt.templateTypes.visionExplorerText') }}</option>
                  <option value="Custom">{{ t('promptMgmt.templateTypes.custom') }}</option>
                </select>
              </div>
              <div>
                <label class="label label-text text-xs">{{ t('promptMgmt.editor.priority') }}</label>
                <input v-model.number="editingTemplate.priority" type="number" class="input input-xs input-bordered w-full" min="0" max="100" />
              </div>
            </div>
            
            <div class="flex items-center gap-4 mb-2">
              <label class="cursor-pointer label">
                <input v-model="editingTemplate.is_system" type="checkbox" class="checkbox checkbox-xs" />
                <span class="label-text text-xs ml-2">{{ t('promptMgmt.editor.systemTemplate') }}</span>
              </label>
              <label class="cursor-pointer label">
                <input v-model="editingTemplate.is_active" type="checkbox" class="checkbox checkbox-xs checkbox-success" />
                <span class="label-text text-xs ml-2">{{ t('promptMgmt.editor.enableTemplate') }}</span>
              </label>
            </div>
            
            <!-- Tags 标签管理 -->
            <div class="mb-2">
              <label class="label label-text text-xs">{{ t('promptMgmt.editor.tags') }}</label>
              <div class="flex flex-wrap gap-1 mb-1">
                <span v-for="(tag, index) in editingTemplate.tags || []" :key="index"
                      class="badge badge-outline badge-xs flex items-center gap-1">
                  {{ tag }}
                  <button @click="removeTag(index)" class="btn btn-ghost btn-xs p-0 min-h-0 h-3 w-3">×</button>
                </span>
              </div>
              <div class="flex gap-1">
                <input v-model="newTag" @keyup.enter="addTag" class="input input-xs input-bordered flex-1" :placeholder="t('promptMgmt.editor.addTagPlaceholder')" />
                <button @click="addTag" class="btn btn-xs btn-outline">{{ t('promptMgmt.editor.addTag') }}</button>
              </div>
            </div>
            
            <!-- Variables 变量管理 -->
            <div class="mb-2">
              <label class="label label-text text-xs">{{ t('promptMgmt.editor.variables') }}</label>
              <div class="flex flex-wrap gap-1 mb-1">
                <span v-for="(variable, index) in editingTemplate.variables || []" :key="index"
                      class="badge badge-success badge-xs flex items-center gap-1">
                  {{ variable }}
                  <button @click="removeVariable(index)" class="btn btn-ghost btn-xs p-0 min-h-0 h-3 w-3">×</button>
                </span>
              </div>
              <div class="flex gap-1">
                <input v-model="newVariable" @keyup.enter="addVariable" class="input input-xs input-bordered flex-1" :placeholder="t('promptMgmt.editor.addVariablePlaceholder')" />
                <button @click="addVariable" class="btn btn-xs btn-outline">{{ t('promptMgmt.editor.addVariable') }}</button>
                <button @click="loadDefaultPrompt" class="btn btn-xs btn-outline" :disabled="!editingTemplate" :title="t('promptMgmt.editor.importDefaultTitle')">
                  {{ t('promptMgmt.editor.importDefault') }}
                </button>
              </div>
              <div class="text-xs opacity-60 mt-1">
                {{ t('promptMgmt.editor.importDefaultHint') }}
              </div>
            </div>
            
            <textarea v-model="editingTemplate.content" class="textarea textarea-bordered font-mono text-sm h-full grow" :placeholder="$t('promptMgmt.contentPlaceholder') as string"></textarea>
          </div>
          <div class="card-body p-4 h-full flex items-center justify-center text-sm opacity-60" v-else>
            {{ $t('promptMgmt.noTemplateSelected') }}
          </div>
        </div>

        <!-- 预览卡片 -->
        <div class="card bg-base-100 shadow-md h-full overflow-hidden">
          <div class="card-body p-4 h-full overflow-hidden flex flex-col">
            <div class="flex items-center justify-between mb-2">
              <div class="text-sm font-medium">{{ t('promptMgmt.preview.title') }}</div>
              <div class="flex items-center gap-2">
                <label class="label cursor-pointer">
                  <span class="label-text text-xs mr-2">{{ t('promptMgmt.preview.variableRendering') }}</span>
                  <input v-model="enableVariablePreview" type="checkbox" class="checkbox checkbox-xs" />
                </label>
                <button v-if="enableVariablePreview && editingTemplate?.id" 
                        @click="evaluatePreview" 
                        class="btn btn-xs btn-outline">
                  {{ t('promptMgmt.preview.realTimePreview') }}
                </button>
              </div>
            </div>
            
            <!-- 变量上下文编辑器 -->
            <div v-if="enableVariablePreview" class="mb-2">
              <label class="label label-text text-xs">{{ t('promptMgmt.preview.sampleContext') }}</label>
              <textarea v-model="sampleContext" 
                       class="textarea textarea-bordered text-xs font-mono"
                       rows="3"
                       :placeholder="t('promptMgmt.preview.sampleContextPlaceholder')">
              </textarea>
            </div>
            
            <div class="mockup-code text-xs overflow-auto h-full">
              <pre data-prefix=">"><code>{{ renderedPreview }}</code></pre>
            </div>
            <div class="text-[10px] opacity-60 mt-2">{{ $t('promptMgmt.shortcuts') }}</div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onBeforeUnmount, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { invoke } from '@tauri-apps/api/core'
import { useToast } from '@/composables/useToast'
import { dialog } from '@/composables/useDialog'

// 简化类型定义 - 仅保留必要的分类
type PromptCategory = 'System' | 'Application' | 'UserDefined'
type TemplateType = 'SystemPrompt' | 'IntentClassifier' | 'Planner' | 'Executor' | 'Replanner' | 'Evaluator' | 'ReportGenerator' | 'Domain' | 'Custom' | 'PluginGeneration' | 'AgentPluginGeneration' | 'PluginFix' | 'AgentPluginFix' | 'PluginVulnSpecific' | 'VisionExplorerVision' | 'VisionExplorerText'
type ArchitectureType = 'ReAct'

interface PromptTemplate {
  id?: number
  name: string
  description?: string | null
  content: string
  is_default: boolean
  is_active: boolean
  created_at?: string | null
  updated_at?: string | null
  category?: PromptCategory
  template_type?: TemplateType
  is_system?: boolean
  priority?: number
  tags?: string[]
  variables?: string[]
  version?: string
}

const { t } = useI18n()


// 统一使用系统级提示，不再区分架构/阶段
const promptCategories = computed(() => [
  { value: 'System', label: t('promptMgmt.categories.system'), description: t('promptMgmt.categories.systemDesc') },
  { value: 'Application', label: t('promptMgmt.categories.application'), description: t('promptMgmt.categories.applicationDesc') },
  { value: 'UserDefined', label: t('promptMgmt.categories.userDefined'), description: t('promptMgmt.categories.userDefinedDesc') },
])

const templates = ref<PromptTemplate[]>([])
const editingTemplate = ref<PromptTemplate | null>(null)
const activePromptId = ref<number | null>(null)
const statusText = ref('')
const searchQuery = ref('')
const isDirty = ref(false)
const toast = useToast()
const selectedCategory = ref<PromptCategory>('System')
const ignoreCategoryWatch = ref(false)

// 新增响应式数据
const newTag = ref('')
const newVariable = ref('')
const enableVariablePreview = ref(false)
const sampleContext = ref('{"task_name": "端口扫描", "tools": "nmap, masscan", "target_info": "192.168.1.1"}')
const evaluatedContent = ref('')

// 用于精准判断是否有未保存更改
const originalTemplateHash = ref<string>('')
function calcTemplateHash(t: PromptTemplate | null): string {
  if (!t) return ''
  const normalized = {
    name: t.name || '',
    description: t.description || '',
    content: t.content || '',
    template_type: t.template_type || null,
    priority: typeof t.priority === 'number' ? t.priority : 0,
    is_system: !!t.is_system,
    tags: (t.tags || []).slice().sort(),
    variables: (t.variables || []).slice().sort(),
    category: t.category || null,
    version: t.version || ''
  }
  return JSON.stringify(normalized)
}

const preview = computed(() => editingTemplate.value?.content ?? '')

const renderedPreview = computed(() => {
  if (!enableVariablePreview.value) {
    return preview.value
  }
  return evaluatedContent.value || preview.value
})
const filteredTemplates = computed(() => {
  const q = searchQuery.value.toLowerCase().trim()
  let list = templates.value
  
  // 根据选择的分类过滤
  if (selectedCategory.value === 'System') {
    list = list.filter(t => t.is_system || t.template_type === 'SystemPrompt' || t.template_type === 'IntentClassifier' || t.category === 'System')
  } else if (selectedCategory.value === 'Application') {
    list = list.filter(t => t.category === 'Application')
  } else if (selectedCategory.value === 'UserDefined') {
    list = list.filter(t => t.category === 'UserDefined')
  }
  
  if (q) {
    list = list.filter(t =>
      t.name?.toLowerCase().includes(q) ||
      (t.description ?? '').toLowerCase().includes(q)
    )
  }
  return list
})

// 从后端拿到所有模板后缓存一份
const allTemplates = ref<PromptTemplate[]>([])
let onBeforeUnload: ((e: BeforeUnloadEvent) => void) | null = null

async function refresh() {
  statusText.value = 'Loading...'
  try {
    const list = await invoke<PromptTemplate[]>('list_prompt_templates_api')
    allTemplates.value = list
    templates.value = list
  } catch (e) {
    templates.value = []
  }
  statusText.value = t('promptMgmt.messages.ready')
}

function newTemplate() {
  const baseTemplate = {
    name: `${selectedCategory.value}-${Date.now()}`,
    description: '',
    content: '',
    is_default: false,
    is_active: true,
    category: selectedCategory.value,
    template_type: selectedCategory.value === 'System' ? 'SystemPrompt' as TemplateType : 'Custom' as TemplateType,
    is_system: selectedCategory.value === 'System',
    priority: 50,
    tags: [],
    variables: [],
    version: '1.0.0',
  }
  
  editingTemplate.value = baseTemplate as PromptTemplate
  originalTemplateHash.value = calcTemplateHash(editingTemplate.value)
}

async function onNewWithGuard() {
  if (isDirty.value) {
    const ok = await dialog.confirm(t('promptMgmt.confirmDiscardUnsaved'))
    if (!ok) return
  }
  newTemplate()
  isDirty.value = false
}

function loadTemplate(tpl: PromptTemplate) {
  editingTemplate.value = { ...tpl }
  originalTemplateHash.value = calcTemplateHash(editingTemplate.value)
  isDirty.value = false
}

async function onLoadWithGuard(tpl: PromptTemplate) {
  if (isDirty.value) {
    const ok = await dialog.confirm(t('promptMgmt.confirmDiscardUnsaved'))
    if (!ok) return
  }
  loadTemplate(tpl)
  isDirty.value = false
}

async function saveTemplate() {
  if (!editingTemplate.value) return
  const tpl = editingTemplate.value
  if (!tpl.name || !tpl.content) {
    toast.error(t('promptMgmt.requiredFields') as unknown as string)
    return
  }
  
  // 保存模板（后端会自动处理同类型模板的激活互斥逻辑）
  if (tpl.id) {
    await invoke('update_prompt_template_api', { id: tpl.id, template: tpl })
  } else {
    const id = await invoke<number>('create_prompt_template_api', { template: tpl })
    editingTemplate.value.id = id
  }
  
  await refresh()
  originalTemplateHash.value = calcTemplateHash(editingTemplate.value)
  isDirty.value = false
  
  // 如果激活了模板，提示用户同类型的其他模板已被自动取消激活
  if (tpl.is_active && tpl.template_type) {
    toast.success(t('promptMgmt.messages.templateSavedAndActivated'))
  } else if (selectedCategory.value === 'System' && tpl.is_active) {
    toast.success(t('promptMgmt.messages.templateSavedAndActivatedSimple'))
  } else {
    toast.success(t('promptMgmt.savedToast') as unknown as string)
  }
}

async function removeTemplate() {
  if (!editingTemplate.value?.id) return
  const confirmed = await dialog.confirm(t('promptMgmt.confirmDeleteTemplate'))
  if (!confirmed) return
  await invoke('delete_prompt_template_api', { id: editingTemplate.value.id })
  editingTemplate.value = null
  originalTemplateHash.value = ''
  await refresh()
}

async function activateTemplate() {
  if (!editingTemplate.value?.id) return
  // 激活模板逻辑已在saveTemplate中处理
  toast.success(t('promptMgmt.activatedToast') as unknown as string)
}

// Define variables outside onMounted for cleanup
let onKey: (e: KeyboardEvent) => void
onMounted(async () => {
  await refresh()
  onKey = (e: KeyboardEvent) => {
    const isMac = navigator.platform.toLowerCase().includes('mac')
    const ctrlOrCmd = isMac ? e.metaKey : e.ctrlKey
    if (ctrlOrCmd && e.key.toLowerCase() === 's') {
      e.preventDefault()
      saveTemplate()
    }
    if (ctrlOrCmd && e.key === 'Enter') {
      e.preventDefault()
      if (selectedCategory.value === 'System') activateTemplate()
    }
    if (ctrlOrCmd && (e.key === 'Backspace' || e.key === 'Delete')) {
      e.preventDefault()
      removeTemplate()
    }
  }
  window.addEventListener('keydown', onKey)
  onBeforeUnload = (e: BeforeUnloadEvent) => {
    if (isDirty.value) {
      e.preventDefault()
      e.returnValue = ''
    }
  }
  window.addEventListener('beforeunload', onBeforeUnload)
})

// cleanup - moved outside async onMounted
onBeforeUnmount(() => {
  if (onKey) window.removeEventListener('keydown', onKey)
  if (onBeforeUnload) window.removeEventListener('beforeunload', onBeforeUnload)
})

// 精准监听：根据快照判断是否脏
watch(
  () => [
    editingTemplate.value?.name,
    editingTemplate.value?.description,
    editingTemplate.value?.content,
    editingTemplate.value?.template_type,
    editingTemplate.value?.priority,
    editingTemplate.value?.is_system,
    JSON.stringify((editingTemplate.value?.tags || []).slice().sort()),
    JSON.stringify((editingTemplate.value?.variables || []).slice().sort()),
    editingTemplate.value?.category,
    editingTemplate.value?.version,
  ],
  () => {
    const currentHash = calcTemplateHash(editingTemplate.value || null)
    isDirty.value = !!editingTemplate.value && currentHash !== originalTemplateHash.value
  }
)

// 分类切换：守护未保存并刷新列表
watch(selectedCategory, async (newVal, oldVal) => {
  if (ignoreCategoryWatch.value) { ignoreCategoryWatch.value = false; return }
  if (isDirty.value) {
    const ok = await dialog.confirm(t('promptMgmt.confirmDiscardUnsaved'))
    if (!ok) {
      ignoreCategoryWatch.value = true
      selectedCategory.value = oldVal as PromptCategory
      return
    }
  }
  editingTemplate.value = null
  await refresh()
  isDirty.value = false
})

// 导入默认prompt内容
async function loadDefaultPrompt() {
  if (!editingTemplate.value) {
    toast.error(t('promptMgmt.messages.selectOrCreate'))
    return
  }
  
  try {
    statusText.value = t('promptMgmt.messages.loadingDefault')
    
    const content = await invoke<string>('get_default_prompt_content', {})
    
    // 确认是否覆盖当前内容
    if (editingTemplate.value.content && editingTemplate.value.content.trim()) {
      const confirmed = await dialog.confirm({
        title: t('promptMgmt.messages.confirmImport'),
        message: t('promptMgmt.messages.confirmImportMessage'),
        variant: 'warning'
      })
      
      if (!confirmed) {
        statusText.value = ''
        return
      }
    }
    
    // 设置内容
    editingTemplate.value.content = content
    isDirty.value = true
    
    statusText.value = ''
    toast.success(t('promptMgmt.messages.importSuccess'))
  } catch (error: any) {
    console.error('Failed to load default prompt:', error)
    statusText.value = ''
    toast.error(t('promptMgmt.messages.importFailed', { error: error.message || error }))
  }
}

// 创建意图分析器模板
function createIntentClassifierTemplate() {
  const defaultContent = ``

  editingTemplate.value = {
    name: `意图分析器-${Date.now()}`,
    description: '用于分析用户输入意图的系统提示模板',
    content: defaultContent,
    is_default: false,
    is_active: true,
    category: 'System' as PromptCategory,
    template_type: 'IntentClassifier' as TemplateType,
    is_system: true,
    priority: 90, // 高优先级
    tags: ['system', 'intent'],
    variables: ['user_input'],
    version: '1.0.0',
  }
  isDirty.value = false // 这是新创建的模板，不算脏数据
}

// 创建通用系统提示模板
function createSystemPromptTemplate() {
  const defaultContent = `你是一个安全专家AI助手。

你的职责是：
1. 帮助用户进行安全相关的分析和测试
2. 提供专业的安全建议和指导
3. 执行安全相关的任务

请根据用户的具体需求选择合适的工具和方法，确保操作的安全和有效性。`

  editingTemplate.value = {
    name: `系统提示-${Date.now()}`,
    description: '通用系统提示模板',
    content: defaultContent,
    is_default: false,
    is_active: true,
    category: 'System' as PromptCategory,
    template_type: 'SystemPrompt' as TemplateType,
    is_system: true,
    priority: 80,
    tags: ['system'],
    variables: [],
    version: '1.0.0',
  }
  isDirty.value = false
}

// 创建插件生成模板(被动扫描)
function createPluginGenerationTemplate() {
  const defaultContent = ``

  editingTemplate.value = {
    name: `被动扫描插件生成模板-${Date.now()}`,
    description: '用于生成被动扫描插件的AI提示模板',
    content: defaultContent,
    is_default: false,
    is_active: true,
    category: 'Application' as PromptCategory,
    template_type: 'PluginGeneration' as TemplateType,
    is_system: true,
    priority: 90,
    tags: ['plugin', 'generation', 'security', 'traffic'],
    variables: ['vuln_type', 'analysis', 'endpoints', 'requirements'],
    version: '1.0.0',
  }
  isDirty.value = false
}

// 创建Agent插件生成模板
function createAgentPluginGenerationTemplate() {
  const defaultContent = ``

  editingTemplate.value = {
    name: `Agent插件生成模板-${Date.now()}`,
    description: '用于生成Agent工具插件的AI提示模板',
    content: defaultContent,
    is_default: false,
    is_active: true,
    category: 'Application' as PromptCategory,
    template_type: 'AgentPluginGeneration' as TemplateType,
    is_system: true,
    priority: 90,
    tags: ['agent', 'plugin', 'generation', 'tool'],
    variables: ['tool_type', 'requirements', 'options'],
    version: '1.0.0',
  }
  isDirty.value = false
}

// 创建插件修复模板
function createPluginFixTemplate() {
  const defaultContent = ``

  editingTemplate.value = {
    name: `插件修复模板-${Date.now()}`,
    description: '用于修复失败插件代码的AI提示模板',
    content: defaultContent,
    is_default: false,
    is_active: true,
    category: 'Application' as PromptCategory,
    template_type: 'PluginFix' as TemplateType,
    is_system: true,
    priority: 85,
    tags: ['plugin', 'fix', 'repair', 'traffic'],
    variables: ['original_code', 'error_message', 'error_details', 'vuln_type', 'attempt'],
    version: '1.0.0',
  }
  isDirty.value = false
}

// 创建Agent插件修复模板
function createAgentPluginFixTemplate() {
  const defaultContent = `# Agent Tool Plugin Code Fix Task

You are an expert TypeScript developer. An Agent tool plugin failed execution. Your task is to fix the code.

**Variables**:
- {original_code}: The original plugin code
- {error_message}: Error message from execution
- {error_details}: Detailed error information
- {tool_type}: Tool type
- {attempt}: Fix attempt number

Please analyze the error and provide a fixed version of the plugin code.`

  editingTemplate.value = {
    name: `Agent插件修复模板-${Date.now()}`,
    description: '用于修复失败Agent工具插件代码的AI提示模板',
    content: defaultContent,
    is_default: false,
    is_active: true,
    category: 'Application' as PromptCategory,
    template_type: 'AgentPluginFix' as TemplateType,
    is_system: true,
    priority: 85,
    tags: ['agent', 'plugin', 'fix', 'repair'],
    variables: ['original_code', 'error_message', 'error_details', 'tool_type', 'attempt'],
    version: '1.0.0',
  }
  isDirty.value = false
}

// 创建VisionExplorer多模态提示模板
function createVisionExplorerVisionTemplate() {
  const defaultContent = ``

  editingTemplate.value = {
    name: `VisionExplorer多模态提示-${Date.now()}`,
    description: 'VisionExplorer视觉探索引擎多模态模型专用提示，支持截图分析',
    content: defaultContent,
    is_default: false,
    is_active: true,
    category: 'Application' as PromptCategory,
    template_type: 'VisionExplorerVision' as TemplateType,
    is_system: true,
    priority: 90,
    tags: ['vision', 'explorer', 'multimodal', 'screenshot'],
    variables: ['viewport_width', 'viewport_height'],
    version: '1.0.0',
  }
  isDirty.value = false
}

// 创建VisionExplorer文本模型提示模板
function createVisionExplorerTextTemplate() {
  const defaultContent = ``

  editingTemplate.value = {
    name: `VisionExplorer文本模型提示-${Date.now()}`,
    description: 'VisionExplorer视觉探索引擎文本模型专用提示，基于元素列表分析',
    content: defaultContent,
    is_default: false,
    is_active: true,
    category: 'Application' as PromptCategory,
    template_type: 'VisionExplorerText' as TemplateType,
    is_system: true,
    priority: 90,
    tags: ['vision', 'explorer', 'text', 'element-list'],
    variables: ['viewport_width', 'viewport_height'],
    version: '1.0.0',
  }
  isDirty.value = false
}

// ===== Tags 和 Variables 管理方法 =====
function addTag() {
  if (!newTag.value.trim() || !editingTemplate.value) return
  if (!editingTemplate.value.tags) editingTemplate.value.tags = []
  if (!editingTemplate.value.tags.includes(newTag.value.trim())) {
    editingTemplate.value.tags.push(newTag.value.trim())
    newTag.value = ''
    isDirty.value = true
  }
}

function removeTag(index: number) {
  if (!editingTemplate.value?.tags) return
  editingTemplate.value.tags.splice(index, 1)
  isDirty.value = true
}

function addVariable() {
  if (!newVariable.value.trim() || !editingTemplate.value) return
  if (!editingTemplate.value.variables) editingTemplate.value.variables = []
  const varName = newVariable.value.trim()
  if (!editingTemplate.value.variables.includes(varName)) {
    editingTemplate.value.variables.push(varName)
    newVariable.value = ''
    isDirty.value = true
  }
}

function removeVariable(index: number) {
  if (!editingTemplate.value?.variables) return
  editingTemplate.value.variables.splice(index, 1)
  isDirty.value = true
}

async function evaluatePreview() {
  if (!editingTemplate.value?.id) return
  try {
    let context = {}
    try {
      context = JSON.parse(sampleContext.value)
    } catch (e) {
      toast.error(t('promptMgmt.messages.contextJsonError'))
      return
    }
    
    const result = await invoke<string>('evaluate_prompt_api', {
      templateId: editingTemplate.value.id,
      context
    })
    evaluatedContent.value = result
  } catch (error) {
    console.error('Failed to evaluate prompt:', error)
    toast.error(t('promptMgmt.messages.previewFailed', { error: (error as any).message }))
  }
}
</script>

<style scoped>
.btn { padding: 0.25rem 0.75rem; border: 1px solid #e5e7eb; border-radius: 0.25rem; background: #fff; font-size: calc(var(--font-size-base, 14px) * 0.875); }
.btn:hover { background: #f9fafb; }
.input { width: 100%; border: 1px solid #e5e7eb; border-radius: 0.25rem; padding: 0.25rem 0.5rem; }
</style>


