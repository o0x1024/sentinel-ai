  
  <template>
  <div class="page-content-padded safe-top h-full flex gap-4">
    <!-- 第一列：分类选择 -->
    <div class="w-56 card bg-base-100 shadow-md overflow-hidden flex flex-col">
      <div class="card-body p-4 pb-3">
        <!-- Prompt分类选择器 -->
        <div class="mb-4">
          <h4 class="card-title text-xs mb-2">Prompt分类</h4>
          <select v-model="selectedCategory" class="select select-sm select-bordered w-full">
            <option v-for="cat in promptCategories" :key="cat.value" :value="cat.value">
              {{ cat.label }}
            </option>
          </select>
          <div class="text-xs opacity-60 mt-1">{{ promptCategories.find(c => c.value === selectedCategory)?.description }}</div>
        </div>
        
        <!-- 系统级提示创建按钮 -->
        <div v-if="selectedCategory === 'System'" class="mt-4 pt-4 border-t">
          <h3 class="card-title text-sm">创建系统提示</h3>
          <div class="text-xs opacity-70 mt-1">添加新的系统提示模板</div>
          <div class="mt-2 flex flex-col gap-1">
            <button class="btn btn-xs btn-outline w-full" @click="createIntentClassifierTemplate">
              意图分析器
            </button>
            <button class="btn btn-xs btn-outline w-full" @click="createSystemPromptTemplate">
              通用系统提示
            </button>
          </div>
        </div>
        
        <!-- 应用级提示 - 仅在应用分类时显示 -->
        <div v-if="selectedCategory === 'Application'">
          <h3 class="card-title text-sm">应用级提示模板</h3>
          <div class="text-xs opacity-70 mt-1">管理应用特定的提示模板</div>
          <div class="mt-2 flex flex-col gap-1">
            <button class="btn btn-xs btn-outline" @click="createPluginGenerationTemplate">
              插件生成(被动扫描)
            </button>
            <button class="btn btn-xs btn-outline" @click="createAgentPluginGenerationTemplate">
              插件生成(Agent工具)
            </button>
            <button class="btn btn-xs btn-outline" @click="createPluginFixTemplate">
              插件修复(被动扫描)
            </button>
            <button class="btn btn-xs btn-outline" @click="createAgentPluginFixTemplate">
              插件修复(Agent工具)
            </button>
            <button class="btn btn-xs btn-outline" @click="createVisionExplorerSystemTemplate">
              VisionExplorer系统提示
            </button>
          </div>
        </div>
        
        <!-- 用户自定义 - 仅在用户自定义分类时显示 -->
        <div v-if="selectedCategory === 'UserDefined'">
          <h3 class="card-title text-sm">用户自定义模板</h3>
          <div class="text-xs opacity-70 mt-1">管理用户创建的自定义模板</div>
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
        <div class="text-xs font-medium mb-2">模板列表 ({{ filteredTemplates.length }})</div>
      </div>

      <!-- 模板列表 -->
      <div class="px-4 pb-4 flex-1 overflow-auto">
        <div class="grid grid-cols-1 gap-2">
          <button
            v-for="t in filteredTemplates"
            :key="t.id"
            class="btn btn-outline btn-sm justify-start normal-case w-full"
            :class="{
              '!btn-primary text-white': editingTemplate?.id === t.id,
            }"
            @click="onLoadWithGuard(t)"
          >
            <div class="w-full flex items-center gap-2">
              <div class="truncate flex-1 text-left">
                <div class="font-medium text-xs truncate flex items-center gap-1">
                  <span v-if="t.is_active" class="inline-block w-2 h-2 rounded-full bg-success" title="已启用"></span>
                  {{ t.name }}
                </div>
                <div class="text-[10px] opacity-70 truncate">
                  #{{ t.id }} · {{ t.template_type || 'Custom' }}
                </div>
              </div>
              <span v-if="t.is_active" class="badge badge-success badge-xs">启用</span>
              <span v-else-if="t.id === activePromptId" class="badge badge-success badge-xs">{{ $t('promptMgmt.activeBadge') }}</span>
              <span v-else-if="t.is_default" class="badge badge-outline badge-xs">{{ $t('promptMgmt.default') }}</span>
            </div>
          </button>
        </div>
        
        <!-- 空状态 -->
        <div v-if="filteredTemplates.length === 0" class="text-center py-8 text-xs opacity-50">
          暂无模板，点击"新建"创建
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
                <label class="label label-text text-xs">模板类型</label>
                <select v-model="editingTemplate.template_type" class="select select-xs select-bordered w-full">
                  <option value="SystemPrompt">系统提示</option>
                  <option value="IntentClassifier">意图分析器</option>
                  <option value="Planner">规划器</option>
                  <option value="Executor">执行器</option>
                  <option value="Replanner">重规划器</option>
                  <option value="Evaluator">评估器</option>
                  <option value="ReportGenerator">报告生成器</option>
                  <option value="PluginGeneration">插件生成(被动扫描)</option>
                  <option value="AgentPluginGeneration">插件生成(Agent工具)</option>
                  <option value="PluginFix">插件修复(被动扫描)</option>
                  <option value="AgentPluginFix">插件修复(Agent工具)</option>
                  <option value="PluginVulnSpecific">插件漏洞专用</option>
                  <option value="VisionExplorerSystem">VisionExplorer系统提示</option>
                  <option value="Custom">自定义</option>
                </select>
              </div>
              <div>
                <label class="label label-text text-xs">优先级</label>
                <input v-model.number="editingTemplate.priority" type="number" class="input input-xs input-bordered w-full" min="0" max="100" />
              </div>
            </div>
            
            <div class="flex items-center gap-4 mb-2">
              <label class="cursor-pointer label">
                <input v-model="editingTemplate.is_system" type="checkbox" class="checkbox checkbox-xs" />
                <span class="label-text text-xs ml-2">系统级模板</span>
              </label>
              <label class="cursor-pointer label">
                <input v-model="editingTemplate.is_active" type="checkbox" class="checkbox checkbox-xs checkbox-success" />
                <span class="label-text text-xs ml-2">启用此模板</span>
              </label>
            </div>
            
            <!-- Tags 标签管理 -->
            <div class="mb-2">
              <label class="label label-text text-xs">标签</label>
              <div class="flex flex-wrap gap-1 mb-1">
                <span v-for="(tag, index) in editingTemplate.tags || []" :key="index"
                      class="badge badge-outline badge-xs flex items-center gap-1">
                  {{ tag }}
                  <button @click="removeTag(index)" class="btn btn-ghost btn-xs p-0 min-h-0 h-3 w-3">×</button>
                </span>
              </div>
              <div class="flex gap-1">
                <input v-model="newTag" @keyup.enter="addTag" class="input input-xs input-bordered flex-1" placeholder="添加标签..." />
                <button @click="addTag" class="btn btn-xs btn-outline">添加</button>
              </div>
            </div>
            
            <!-- Variables 变量管理 -->
            <div class="mb-2">
              <label class="label label-text text-xs">变量</label>
              <div class="flex flex-wrap gap-1 mb-1">
                <span v-for="(variable, index) in editingTemplate.variables || []" :key="index"
                      class="badge badge-success badge-xs flex items-center gap-1">
                  {{ variable }}
                  <button @click="removeVariable(index)" class="btn btn-ghost btn-xs p-0 min-h-0 h-3 w-3">×</button>
                </span>
              </div>
              <div class="flex gap-1">
                <input v-model="newVariable" @keyup.enter="addVariable" class="input input-xs input-bordered flex-1" placeholder="变量名 (如: task_name)" />
                <button @click="addVariable" class="btn btn-xs btn-outline">添加</button>
                <button @click="loadDefaultPrompt" class="btn btn-xs btn-outline" :disabled="!editingTemplate" title="从应用数据目录的prompts文件夹导入默认内容">
                  📥 导入默认prompt
                </button>
              </div>
              <div class="text-xs opacity-60 mt-1">
                提示：默认prompt存储在应用数据目录的prompts文件夹中，可以手动编辑
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
              <div class="text-sm font-medium">{{ $t('promptMgmt.preview') }}</div>
              <div class="flex items-center gap-2">
                <label class="label cursor-pointer">
                  <span class="label-text text-xs mr-2">变量渲染</span>
                  <input v-model="enableVariablePreview" type="checkbox" class="checkbox checkbox-xs" />
                </label>
                <button v-if="enableVariablePreview && editingTemplate?.id" 
                        @click="evaluatePreview" 
                        class="btn btn-xs btn-outline">
                  实时预览
                </button>
              </div>
            </div>
            
            <!-- 变量上下文编辑器 -->
            <div v-if="enableVariablePreview" class="mb-2">
              <label class="label label-text text-xs">示例上下文 (JSON)</label>
              <textarea v-model="sampleContext" 
                       class="textarea textarea-bordered text-xs font-mono"
                       rows="3"
                       placeholder='{"task_name": "端口扫描", "tools": "nmap, masscan", "target_info": "192.168.1.1"}'>
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
type TemplateType = 'SystemPrompt' | 'IntentClassifier' | 'Planner' | 'Executor' | 'Replanner' | 'Evaluator' | 'ReportGenerator' | 'Domain' | 'Custom' | 'PluginGeneration' | 'AgentPluginGeneration' | 'PluginFix' | 'AgentPluginFix' | 'PluginVulnSpecific' | 'VisionExplorerSystem'
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

// 统一使用系统级提示，不再区分架构/阶段
const promptCategories = [
  { value: 'System', label: '系统级', description: '系统提示模板' },
  { value: 'Application', label: '应用级', description: '应用特定的提示模板' },
  { value: 'UserDefined', label: '用户自定义', description: '用户创建的自定义模板' },
]

const templates = ref<PromptTemplate[]>([])
const editingTemplate = ref<PromptTemplate | null>(null)
const activePromptId = ref<number | null>(null)
const statusText = ref('')
const searchQuery = ref('')
const isDirty = ref(false)
const toast = useToast()
const { t } = useI18n()
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
  statusText.value = 'Ready'
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
    toast.success('模板已保存并激活，同类型的其他模板已自动取消激活')
  } else if (selectedCategory.value === 'System' && tpl.is_active) {
    toast.success('模板已保存并激活')
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
    toast.error('请先选择或创建一个模板')
    return
  }
  
  try {
    statusText.value = '正在加载默认prompt...'
    
    const content = await invoke<string>('get_default_prompt_content', {})
    
    // 确认是否覆盖当前内容
    if (editingTemplate.value.content && editingTemplate.value.content.trim()) {
      const confirmed = await dialog.confirm({
        title: '确认导入',
        message: '当前模板已有内容，是否覆盖？',
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
    toast.success('已导入默认prompt')
  } catch (error: any) {
    console.error('Failed to load default prompt:', error)
    statusText.value = ''
    toast.error(`导入失败: ${error.message || error}`)
  }
}

// 创建意图分析器模板
function createIntentClassifierTemplate() {
  const defaultContent = `作为一个AI意图分类器，请分析用户输入并判断意图类型。

请判断用户输入属于以下哪种类型：
1. Chat - 普通对话（问候、闲聊、简单交流）
2. Question - 知识性问答（询问概念、原理等，不需要实际执行）  
3. Task - 任务执行（需要AI助手执行具体的安全扫描、分析等操作）

判断标准：
- Chat: 问候语、感谢、简单交流等
- Question: 以"什么是"、"如何理解"等开头的概念性问题
- Task: 包含"扫描"、"检测"、"分析"、"帮我执行"等行动指令

请以JSON格式回复：
{
    "intent": "Chat|Question|Task",
    "confidence": 0.0-1.0,
    "reasoning": "分类理由",
    "requires_agent": true/false,
    "extracted_info": {"key": "value"}
}`

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
  const defaultContent = `# Security Plugin Generation Task

You are an expert security researcher and TypeScript developer. Your task is to generate a high-quality security testing plugin for a passive scanning system.

## Environment and Context

### Available APIs
- **Finding Emission**: Use \`Deno.core.ops.op_emit_finding(finding)\` to report vulnerabilities
- **Logging**: Use \`console.log()\`, \`console.warn()\`, \`console.error()\` for debugging
- **HTTP Analysis**: Access request/response data through the provided context objects

### Plugin Interface (Required)
Your plugin MUST implement these functions:

\`\`\`typescript
interface PluginMetadata {
  id: string;                    // Unique plugin identifier
  name: string;                  // Human-readable name
  version: string;               // Semantic version (e.g., "1.0.0")
  author: string;                // Author name
  main_category: "passive";      // Must be "passive" for passive scan plugins
  category: string;              // Vulnerability category (e.g., "sqli", "xss")
  description: string;           // Brief description
  default_severity: "critical" | "high" | "medium" | "low";
  tags: string[];                // Descriptive tags
}

interface RequestContext {
  id: string;                    // Request ID
  url: string;                   // Full URL
  method: string;                // HTTP method (GET, POST, etc.)
  headers: Record<string, string>;
  query_params: Record<string, string>;  // Parsed query parameters
  body: number[] | Uint8Array;   // Request body as bytes
  content_type?: string;         // Content-Type header
  is_https: boolean;             // Whether using HTTPS
  timestamp: string;             // ISO 8601 timestamp
}

interface ResponseContext {
  id: string;                    // Response ID (matches request)
  status: number;                // HTTP status code
  headers: Record<string, string>;
  body: number[] | Uint8Array;   // Response body as bytes
  timestamp: string;             // ISO 8601 timestamp
}

// Required functions:
export function get_metadata(): PluginMetadata;
export function scan_request(ctx: RequestContext): void;   // Optional
export function scan_response(ctx: ResponseContext): void; // Optional
\`\`\`

### Body Handling
Request/response bodies are provided as \`number[]\` or \`Uint8Array\`. Use this helper:

\`\`\`typescript
function bodyToString(body: number[] | Uint8Array): string {
  try {
    if (body instanceof Uint8Array) {
      return new TextDecoder().decode(body);
    } else if (Array.isArray(body)) {
      return new TextDecoder().decode(new Uint8Array(body));
    }
    return "";
  } catch (e) {
    return "";
  }
}
\`\`\`

### Iterating Over Objects
Use \`Object.entries()\` to iterate over plain JavaScript objects:

\`\`\`typescript
// ✅ Correct
for (const [key, value] of Object.entries(query_params)) {
  // ...
}

// ❌ Wrong (objects don't have .entries() method)
for (const [key, value] of query_params.entries()) {
  // ...
}
\`\`\`

### Emitting Findings
\`\`\`typescript
Deno.core.ops.op_emit_finding({
  title: "SQL Injection Detected",
  description: "Potential SQL injection in parameter 'id'",
  severity: "high",
  confidence: 0.85,
  request_id: ctx.id,
  evidence: {
    parameter: "id",
    value: "1' OR '1'='1",
    pattern: "SQL_INJECTION"
  }
});
\`\`\`

## Task Requirements

**Variables**: 
- {vuln_type}: Vulnerability type to detect (e.g., "sqli", "xss", "idor")
- {analysis}: Website analysis data (technologies, endpoints, patterns)
- {endpoints}: Target endpoints to focus on
- {requirements}: Additional specific requirements

## Output Format

Return ONLY the complete TypeScript plugin code wrapped in a markdown code block:

\`\`\`typescript
// Your plugin code here
\`\`\`

Do NOT include explanations or comments outside the code block.

## Important Constraints

1. **Use \`Object.entries()\`** for iterating over objects (query_params, headers, etc.)
2. **Convert body to string** using the \`bodyToString()\` helper function
3. **Check for null/undefined** before accessing properties
4. **Use try-catch blocks** to handle errors gracefully
5. **Emit findings** only when confident (confidence >= 0.7)
6. **Include proper TypeScript types** for all variables and functions

Please generate a complete, production-ready TypeScript plugin that follows all the above guidelines.`

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
    tags: ['plugin', 'generation', 'security', 'passive'],
    variables: ['vuln_type', 'analysis', 'endpoints', 'requirements'],
    version: '1.0.0',
  }
  isDirty.value = false
}

// 创建Agent插件生成模板
function createAgentPluginGenerationTemplate() {
  const defaultContent = `# Agent Tool Plugin Generation Task

You are an expert security researcher and TypeScript developer. Your task is to generate a high-quality Agent tool plugin for an AI-powered security testing system.

The plugin should:
1. Be written in TypeScript
2. Implement specific security testing or analysis functionality
3. Follow the Agent tool plugin interface
4. Include proper error handling and validation
5. Return structured results using the ToolOutput interface

**Variables**: 
- {tool_type}: Type of tool to implement
- {requirements}: Specific requirements
- {options}: Additional options

Please generate a complete TypeScript Agent tool plugin that follows the standard interface.`

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
  const defaultContent = `# Plugin Code Fix Task

You are an expert TypeScript developer and security researcher. A security plugin was generated but failed execution testing. Your task is to fix the code so it executes correctly.

## Error Information

**Fix Attempt**: {attempt}

**Error Message**: {error_message}

**Detailed Error**:
\`\`\`
{error_details}
\`\`\`

## Original Plugin Code

\`\`\`typescript
{original_code}
\`\`\`

## Fix Instructions

Please fix the code to resolve the error. The fixed plugin must:

1. **Fix the specific error** mentioned above
2. **Maintain the plugin interface**:
   - \`function get_metadata()\` - returns plugin metadata with id, name, version, etc.
   - \`function scan_response(ctx)\` - scans HTTP response for vulnerabilities
   - Optionally \`function scan_request(ctx)\` - scans HTTP request
3. **Detect {vuln_type} vulnerabilities** correctly
4. **Use proper TypeScript syntax** - no syntax errors
5. **Emit findings** using \`Deno.core.ops.op_emit_finding()\`
6. **Include error handling** - use try-catch blocks
7. **Be executable** - the code must run without errors

## Common Issues to Check

- **Missing or incorrect function signatures**: Ensure \`get_metadata()\`, \`scan_request()\`, \`scan_response()\` are properly defined
- **Undefined variables or functions**: Check all variable declarations and function calls
- **Incorrect API usage**: Use \`Deno.core.ops.op_emit_finding()\` (not \`Sentinel.emitFinding()\`)
- **Missing metadata fields**: Ensure all required fields (id, name, version, category, etc.) are present
- **Syntax errors**: Check for missing brackets, semicolons, parentheses
- **Type errors in TypeScript**: Ensure proper type annotations
- **Accessing undefined properties**: Use optional chaining (\`?.\`) or null checks
- **Object iteration**: Use \`Object.entries()\` not \`.entries()\` for plain objects
- **Body handling**: Use \`bodyToString()\` helper to convert \`number[]\` or \`Uint8Array\` to string

## Body Handling Helper

\`\`\`typescript
function bodyToString(body: number[] | Uint8Array): string {
  try {
    if (body instanceof Uint8Array) {
      return new TextDecoder().decode(body);
    } else if (Array.isArray(body)) {
      return new TextDecoder().decode(new Uint8Array(body));
    }
    return "";
  } catch (e) {
    return "";
  }
}
\`\`\`

## Correct Object Iteration

\`\`\`typescript
// ✅ Correct
for (const [key, value] of Object.entries(query_params)) {
  // ...
}

// ❌ Wrong
for (const [key, value] of query_params.entries()) {
  // ...
}
\`\`\`

## Output Format

Return ONLY the fixed TypeScript code, wrapped in a code block:

\`\`\`typescript
// Fixed plugin code here
\`\`\`

Do NOT include explanations, comments about the fix, or any other text outside the code block.

## Important Reminders

- Focus on fixing the SPECIFIC error mentioned
- Maintain all existing functionality
- Ensure the plugin is production-ready
- Test edge cases in your mind before outputting`

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
    tags: ['plugin', 'fix', 'repair', 'passive'],
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

// 创建VisionExplorer系统提示模板
function createVisionExplorerSystemTemplate() {
  const defaultContent = `# Vision Explorer System Prompt

You are **VisionExplorer**, a highly-reliable AI agent operating a web browser to discover all API endpoints and functionality of a website. The browser display measures {viewport_width} x {viewport_height} pixels.

────────────────────────
CORE WORKING PRINCIPLES
────────────────────────

1. **Observe First** - *Always* invoke \`computer_screenshot\` before your first action **and** whenever the UI may have changed. Never act blindly.

2. **Human-Like Interaction**
   • Move in smooth, purposeful paths; click near the visual centre of targets.
   • Type realistic, context-appropriate text for form fields.
   • Wait for page loads and animations to complete.

3. **Systematic Exploration**
   • Explore ALL interactive elements: buttons, links, forms, menus.
   • Click on every button, fill every form, navigate every link.
   • Track what you've explored to avoid repetition.

4. **Verify Every Step** - After each action:
   a. Take another screenshot.
   b. Confirm the expected state before continuing.
   c. If it failed, retry sensibly (try 2 different methods) before calling \`set_exploration_status\` with \`"status":"needs_help"\`.

5. **API Discovery Focus**
   • Your main goal is to trigger as many API calls as possible.
   • Forms, search boxes, and data operations typically trigger APIs.
   • Pay attention to AJAX requests, form submissions, and navigation.

────────────────────────
EXPLORATION STRATEGY
────────────────────────

1. **Initial Scan**
   - Take a screenshot to understand the page structure
   - Identify all visible interactive elements
   - Plan a systematic exploration order

2. **Navigation Menu First**
   - Click through all navigation menu items
   - Each page may have unique forms and functionalities

3. **Forms and Inputs**
   - Fill forms with realistic test data
   - Submit forms to trigger API calls
   - Test both valid and edge case inputs

4. **Interactive Elements**
   - Click all buttons (except dangerous ones like "Delete All")
   - Test dropdown menus and selections
   - Explore modal dialogs and popups

5. **Scroll and Discover**
   - Scroll through pages to load lazy content
   - Look for infinite scroll or pagination
   - Check for elements revealed after scrolling

────────────────────────
AVAILABLE TOOLS
────────────────────────

**Observation:**
- \`computer_screenshot\` - Capture current page state (ALWAYS use before acting)

**Mouse Actions:**
- \`computer_click_mouse\` - Click at coordinates
- \`computer_scroll\` - Scroll in a direction

**Keyboard Actions:**
- \`computer_type_text\` - Type text into focused element
- \`computer_type_keys\` - Press keyboard keys (Enter, Tab, etc.)

**Navigation:**
- \`computer_navigate\` - Navigate to a URL
- \`computer_wait\` - Wait for page to settle

**Task Management:**
- \`set_exploration_status\` - Mark exploration as completed or needs_help

────────────────────────
TASK LIFECYCLE
────────────────────────

1. **Start** - Screenshot → analyze page → plan exploration
2. **Loop** - For each unexplored element: Screenshot → Click/Fill → Verify → Record API
3. **Navigate** - When current page is fully explored, go to next unvisited page
4. **Complete** - When all pages and elements are explored, call set_exploration_status with completed

────────────────────────
IMPORTANT NOTES
────────────────────────

- Do NOT click on logout buttons or destructive actions
- Do NOT submit sensitive forms without user consent
- Always take a screenshot BEFORE and AFTER each action
- If you encounter a login page and have credentials, log in first
- If you encounter a CAPTCHA, call \`set_exploration_status\` with \`needs_help\`

────────────────────────
OUTPUT FORMAT
────────────────────────

You MUST respond with a valid JSON object in the following format:

\`\`\`json
{
  "page_analysis": "Brief description of what you see on the page and current state",
  "next_action": {
    "type": "click|scroll|type|navigate|screenshot|completed|needs_help",
    "element_id": "100,200",
    "value": "text to type if applicable",
    "reason": "Why you chose this action"
  },
  "estimated_apis": ["list of API endpoints you estimate might be triggered"],
  "exploration_progress": 0.5,
  "is_exploration_complete": false
}
\`\`\`

**Variables**:
- {viewport_width}: Browser viewport width in pixels
- {viewport_height}: Browser viewport height in pixels

Remember: **accuracy over speed, systematic over random**. Explore every element to maximize API discovery.`

  editingTemplate.value = {
    name: `VisionExplorer系统提示-${Date.now()}`,
    description: 'VisionExplorer视觉探索引擎的系统提示模板，定义AI代理如何操作浏览器发现API',
    content: defaultContent,
    is_default: false,
    is_active: true,
    category: 'Application' as PromptCategory,
    template_type: 'VisionExplorerSystem' as TemplateType,
    is_system: true,
    priority: 90,
    tags: ['vision', 'explorer', 'browser', 'api-discovery'],
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
      toast.error('上下文JSON格式不正确')
      return
    }
    
    const result = await invoke<string>('evaluate_prompt_api', {
      templateId: editingTemplate.value.id,
      context
    })
    evaluatedContent.value = result
  } catch (error) {
    console.error('Failed to evaluate prompt:', error)
    toast.error('预览失败: ' + (error as any).message)
  }
}
</script>

<style scoped>
.btn { padding: 0.25rem 0.75rem; border: 1px solid #e5e7eb; border-radius: 0.25rem; background: #fff; font-size: calc(var(--font-size-base, 14px) * 0.875); }
.btn:hover { background: #f9fafb; }
.input { width: 100%; border: 1px solid #e5e7eb; border-radius: 0.25rem; padding: 0.25rem 0.5rem; }
</style>


