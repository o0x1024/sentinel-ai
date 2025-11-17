<template>
  <div class="page-content-padded safe-top h-full flex gap-4">
    <!-- 左侧：分类选择 + 架构/阶段 + 搜索 + 模板列表 -->
    <div class="w-80 card bg-base-100 shadow-md overflow-hidden flex flex-col">
      <div class="card-body p-4 pb-3">
        <!-- Prompt分类选择器 -->
        <div class="mb-4">
          <h4 class="card-title text-xs mb-2">Prompt分类</h4>
          <select v-model="selectedCategory" class="select select-sm select-bordered w-full">
            <option v-for="cat in promptCategories" :key="cat.value" :value="cat.value">
              {{ cat.label }} - {{ cat.description }}
            </option>
          </select>
        </div>
        
        <!-- 架构/阶段选择 - 仅在LLM架构分类时显示 -->
        <div v-if="selectedCategory === 'LlmArchitecture'">
          <h3 class="card-title text-sm">{{ $t('promptMgmt.archStage') }}</h3>
        <ul class="menu menu-sm rounded-box mt-2">
          <li v-for="group in groups" :key="group.value">
            <h2 class="menu-title">{{ group.label }}</h2>
            <ul>
              <li v-for="st in group.stages" :key="st.value">
                <a
                  class="justify-start"
                  :class="{ active: selected.architecture===group.value && selected.stage===st.value }"
                  @click="onSelectWithGuard(group.value as ArchitectureType, st.value as StageType)"
                >
                  {{ st.label }}
                </a>
              </li>
            </ul>
          </li>
        </ul>
        </div>
        
        <!-- 系统级提示 - 仅在系统分类时显示 -->
        <div v-if="selectedCategory === 'System'">
          <h3 class="card-title text-sm">系统级提示模板</h3>
          <div class="text-xs opacity-70 mt-1">管理跨架构通用的系统提示</div>
          <div class="mt-2">
            <button class="btn btn-xs btn-outline" @click="createIntentClassifierTemplate">
              创建意图分析器模板
            </button>
          </div>
        </div>
        
        <!-- 应用级提示 - 仅在应用分类时显示 -->
        <div v-if="selectedCategory === 'Application'">
          <h3 class="card-title text-sm">应用级提示模板</h3>
          <div class="text-xs opacity-70 mt-1">管理应用特定的提示模板</div>
          <div class="mt-2 flex flex-col gap-1">
            <button class="btn btn-xs btn-outline" @click="createPluginGenerationTemplate">
              创建插件生成模板(被动扫描)
            </button>
            <button class="btn btn-xs btn-outline" @click="createAgentPluginGenerationTemplate">
              创建插件生成模板(Agent工具)
            </button>
            <button class="btn btn-xs btn-outline" @click="createPluginFixTemplate">
              创建插件修复模板(被动扫描)
            </button>
            <button class="btn btn-xs btn-outline" @click="createAgentPluginFixTemplate">
              创建插件修复模板(Agent工具)
            </button>
            <button class="btn btn-xs btn-outline" @click="createPluginInterfaceTemplate">
              创建插件接口模板(被动扫描)
            </button>
            <button class="btn btn-xs btn-outline" @click="createAgentPluginInterfaceTemplate">
              创建插件接口模板(Agent工具)
            </button>
            <button class="btn btn-xs btn-outline" @click="createPluginOutputFormatTemplate">
              创建插件输出格式模板(被动扫描)
            </button>
            <button class="btn btn-xs btn-outline" @click="createAgentPluginOutputFormatTemplate">
              创建插件输出格式模板(Agent工具)
            </button>
          </div>
        </div>
        
        <!-- 用户自定义 - 仅在用户自定义分类时显示 -->
        <div v-if="selectedCategory === 'UserDefined'">
          <h3 class="card-title text-sm">用户自定义模板</h3>
          <div class="text-xs opacity-70 mt-1">管理用户创建的自定义模板</div>
        </div>
      </div>
      <div class="px-4 pb-2">
        <input v-model.trim="searchQuery" class="input input-sm input-bordered w-full" :placeholder="$t('promptMgmt.searchTemplates') as string" />
      </div>
      <div class="px-4 pb-3 text-xs opacity-60 flex items-center gap-2" v-if="selectedCategory === 'LlmArchitecture'">
        <span>{{ $t('promptMgmt.active') }}</span>
        <span v-if="activePromptId">#{{ activePromptId }}</span>
        <span v-else>{{ $t('promptMgmt.none') }}</span>
      </div>
      <!-- 分组管理 -->
      <div class="px-4 pb-2" v-if="selectedCategory === 'LlmArchitecture'">
        <div class="flex items-center justify-between mb-1">
          <div class="text-xs opacity-70">{{ $t('promptMgmt.groups') }}</div>
          <div class="flex items-center gap-1">
            <button class="btn btn-xs" @click="createGroup">{{ $t('promptMgmt.new') }}</button>
            <button class="btn btn-xs" :disabled="!selectedGroupId" @click="setDefaultGroup">{{ $t('promptMgmt.setDefault') }}</button>
            <button class="btn btn-xs btn-error" :disabled="!selectedGroupId || selectedGroup?.is_default" @click="deleteGroup">{{ $t('common.delete') }}</button>
          </div>
        </div>
        <div class="flex flex-col gap-2 max-h-40 overflow-auto">
          <button
            v-for="g in promptGroups"
            :key="g.id"
            class="btn btn-outline btn-xs justify-start normal-case w-full"
            :class="{ '!btn-primary text-white': selectedGroupId === g.id }"
            @click="selectGroup(g.id!)"
          >
            <div class="w-full flex items-center gap-2">
              <div class="truncate flex-1 text-left">
                <div class="font-medium text-[11px] truncate">{{ g.name }}</div>
                <div class="text-[10px] opacity-70 truncate">{{ g.description }}</div>
              </div>
              <span v-if="g.is_default" class="badge badge-success badge-xs">{{ $t('promptMgmt.default') }}</span>
            </div>
          </button>
        </div>
      </div>
      <div class="divider"></div>

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
                  <span v-if="t.is_active" class="inline-block w-2 h-2 rounded-full bg-success" title="已激活"></span>
                  {{ t.name }}
                </div>
                <div class="text-[10px] opacity-70 truncate">
                  #{{ t.id }} · 
                  <span v-if="t.category === 'Application' || t.category === 'System' || t.category === 'UserDefined'">
                    {{ t.template_type || 'Custom' }}
                  </span>
                  <span v-else>
                    {{ t.architecture }} / {{ t.stage }}
                  </span>
                </div>
              </div>
              <span v-if="t.is_active" class="badge badge-success badge-xs">激活</span>
              <span v-else-if="t.id === activePromptId" class="badge badge-success badge-xs">{{ $t('promptMgmt.activeBadge') }}</span>
              <span v-else-if="t.is_default" class="badge badge-outline badge-xs">{{ $t('promptMgmt.default') }}</span>
            </div>
          </button>
        </div>
      </div>
    </div>

    <!-- 中右：工具栏 + 编辑/预览 -->
    <div class="flex-1 flex flex-col gap-3">
      <!-- 工具栏 -->
      <div class="card bg-base-100 shadow-md">
        <div class="card-body py-3 px-4">
          <div  class="flex flex-wrap items-center gap-3">
            <div v-if="selectedCategory === 'LlmArchitecture'"  class="text-sm opacity-70">
              {{ $t('promptMgmt.toolbarContext', { architecture: selected.architecture, stage: selected.stage }) }}
              <span v-if="isDirty" class="ml-2 badge badge-warning badge-sm">{{ $t('promptMgmt.unsavedBadge') }}</span>
            </div>
            <div v-if="selectedCategory === 'LlmArchitecture'" class="divider divider-horizontal m-0"></div>
            <button class="btn btn-success btn-sm hover:brightness-95 active:brightness-90 shadow-sm" @click="onNewWithGuard">{{ $t('common.create') }}</button>
            <button class="btn btn-success btn-sm hover:brightness-95 active:brightness-90 shadow-sm" :disabled="!editingTemplate" @click="saveTemplate">{{ $t('common.save') }}</button>
            <button v-if="selectedCategory === 'LlmArchitecture'" class="btn btn-outline btn-sm hover:brightness-95 active:brightness-90 shadow-sm" :disabled="!editingTemplate?.id" @click="activateTemplate">{{ $t('promptMgmt.active') }}</button>
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
                  <option value="PluginInterface">插件接口(被动扫描)</option>
                  <option value="PluginOutputFormat">插件输出格式(被动扫描)</option>
                  <option value="AgentPluginOutputFormat">插件输出格式(Agent工具)</option>
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
                <span class="label-text text-xs ml-2">激活此模板</span>
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

      <!-- 组阶段映射管理 -->
      <div class="card bg-base-100 shadow-md" v-if="selectedCategory === 'LlmArchitecture'">
        <div class="card-body p-4">
          <div class="flex items-center justify-between mb-3">
            <h4 class="card-title text-sm">{{ $t('promptMgmt.groupMapping') }}</h4>
            <div class="text-xs opacity-70">{{ $t('promptMgmt.currentGroup') }}：<span class="font-medium">{{ selectedGroup?.name || $t('promptMgmt.notSelected') }}</span></div>
          </div>
          <div class="grid grid-cols-3 gap-3">
            <div v-for="st in stagesOfGroupArch" :key="st" class="flex flex-col gap-1">
              <div class="text-xs opacity-70">{{ st }}</div>
              <select class="select select-bordered select-xs" :disabled="!selectedGroupId" v-model="groupMappingDraft[st]" @change="onChangeGroupItem(st)">
                <option :value="null">{{ $t('promptMgmt.notSet') }}</option>
                <option v-for="t in allTemplatesForGroupByStage[st] || []" :key="t.id" :value="t.id">#{{ t.id }} · {{ t.name }}</option>
              </select>
            </div>
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

type ArchitectureType = 'ReWOO' | 'LLMCompiler' | 'PlanExecute' | 'ReAct'
type StageType = 'Planner' | 'Worker' | 'Solver' | 'Planning' | 'Execution' | 'Evaluation' | 'Replan'
type PromptCategory = 'System' | 'LlmArchitecture' | 'Application' | 'UserDefined'
type TemplateType = 'SystemPrompt' | 'IntentClassifier' | 'Planner' | 'Executor' | 'Replanner' | 'Evaluator' | 'ReportGenerator' | 'Domain' | 'Custom' | 'PluginGeneration' | 'AgentPluginGeneration' | 'PluginFix' | 'AgentPluginFix' | 'PluginVulnSpecific' | 'PluginInterface' | 'PluginOutputFormat' | 'AgentPluginOutputFormat'

interface PromptTemplate {
  id?: number
  name: string
  description?: string | null
  architecture: ArchitectureType
  stage: StageType
  content: string
  is_default: boolean
  is_active: boolean
  created_at?: string | null
  updated_at?: string | null
  // 新增字段
  category?: PromptCategory
  template_type?: TemplateType
  target_architecture?: ArchitectureType
  is_system?: boolean
  priority?: number
  tags?: string[]
  variables?: string[]
  version?: string
}

interface PromptGroup {
  id?: number
  architecture: ArchitectureType
  name: string
  description?: string | null
  is_default: boolean
  created_at?: string | null
  updated_at?: string | null
}

interface PromptGroupItem {
  id?: number
  group_id: number
  stage: StageType
  template_id: number
  created_at?: string | null
  updated_at?: string | null
}

const promptCategories = [
  { value: 'System', label: '系统级', description: '跨架构通用的系统提示' },
  { value: 'LlmArchitecture', label: 'LLM架构', description: '特定架构的提示模板' },
  { value: 'Application', label: '应用级', description: '应用特定的提示模板' },
  { value: 'UserDefined', label: '用户自定义', description: '用户创建的自定义模板' },
]

const groups = [
  { value: 'ReWOO', label: 'ReWOO', stages: [
    { value: 'Planner', label: 'Planner (规划器)' },
    { value: 'Solver', label: 'Solver (求解器)' },
  ]},
  { value: 'LLMCompiler', label: 'LLMCompiler', stages: [
    { value: 'Planning', label: 'Planning (规划)' },
    { value: 'Execution', label: 'Execution (执行)' },
    { value: 'Evaluation', label: 'Evaluation (评估)' },
    { value: 'Replan', label: 'Replan (重规划)' },
  ]},
  { value: 'PlanExecute', label: 'Plan&Execute', stages: [
    { value: 'Planning', label: 'Planning (规划)' },
    { value: 'Execution', label: 'Execution (执行)' },
    { value: 'Replan', label: 'Replan (重规划)' },
  ]},
  { value: 'ReAct', label: 'ReAct', stages: [
    { value: 'Planning', label: 'Planning (规划)' },
    { value: 'Execution', label: 'Execution (执行)' },
  ]},
]

const selected = ref<{ architecture: ArchitectureType, stage: StageType }>({ architecture: 'ReWOO', stage: 'Planner' })
const templates = ref<PromptTemplate[]>([])
const editingTemplate = ref<PromptTemplate | null>(null)
const activePromptId = ref<number | null>(null)
const statusText = ref('')
const searchQuery = ref('')
const isDirty = ref(false)
const toast = useToast()
const { t } = useI18n()
const selectedCategory = ref<PromptCategory>('LlmArchitecture')
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
    architecture: t.architecture,
    stage: t.stage,
    target_architecture: t.target_architecture || null,
    version: t.version || ''
  }
  return JSON.stringify(normalized)
}

// 组相关
const promptGroups = ref<PromptGroup[]>([])
const selectedGroupId = ref<number | null>(null)
const groupItems = ref<Record<StageType, number | undefined>>({} as any)
const groupMappingDraft = ref<Record<string, number | undefined>>({})
const defaultGroupId = computed(() => promptGroups.value.find(g => g.is_default)?.id || null)

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
    list = list.filter(t => t.is_system || t.template_type === 'SystemPrompt' || t.template_type === 'IntentClassifier')
  } else if (selectedCategory.value === 'LlmArchitecture') {
    list = list.filter(t => t.category === 'LlmArchitecture' || (!t.category && t.architecture && t.stage))
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

const stagesOfSelectedArch = computed<StageType[]>(() => {
  if (selected.value.architecture === 'ReWOO') return ['Planner','Solver'] as StageType[]
  if (selected.value.architecture === 'LLMCompiler') return ['Planning','Execution','Evaluation','Replan'] as StageType[]
  if (selected.value.architecture === 'ReAct') return ['Planning','Execution'] as StageType[]
  return ['Planning','Execution','Replan'] as StageType[]
})

// 按当前选中分组的架构计算阶段（用于分组映射区）
const stagesOfGroupArch = computed<StageType[]>(() => {
  const arch = selectedGroup.value?.architecture || selected.value.architecture
  if (arch === 'ReWOO') return ['Planner','Solver'] as StageType[]
  if (arch === 'LLMCompiler') return ['Planning','Execution','Evaluation','Replan'] as StageType[]
  if (arch === 'ReAct') return ['Planning','Execution'] as StageType[]
  return ['Planning','Execution','Replan'] as StageType[]
})

const allTemplatesByStage = computed<Record<string, PromptTemplate[]>>(() => {
  const map: Record<string, PromptTemplate[]> = {}
  for (const st of stagesOfSelectedArch.value) {
    map[st] = allTemplates.value.filter(t => t.stage === st)
  }
  return map
})

// 分组映射区可选模板：按分组架构过滤
const allTemplatesForGroupByStage = computed<Record<string, PromptTemplate[]>>(() => {
  const map: Record<string, PromptTemplate[]> = {}
  const arch = selectedGroup.value?.architecture || selected.value.architecture
  const list = allTemplates.value.filter(t => t.architecture === arch)
  for (const st of stagesOfGroupArch.value) {
    map[st] = list.filter(t => t.stage === st)
  }
  return map
})

// 从后端拿到所有模板后缓存一份，便于分组映射下拉使用
const allTemplates = ref<PromptTemplate[]>([])

async function select(architecture: ArchitectureType, stage: StageType) {
  console.log('[select] Selecting:', { architecture, stage })
  selected.value = { architecture, stage }
  selectedGroupId.value = null
  console.log('[select] Calling refresh...')
  await refresh()
  console.log('[select] Refresh complete, calling loadDefaultStagePrompt...')
  // 切换阶段后，自动加载并显示默认分组的默认提示词
  await loadDefaultStagePrompt()
  console.log('[select] loadDefaultStagePrompt complete')
}

async function onSelectWithGuard(architecture: ArchitectureType, stage: StageType) {
  if (isDirty.value) {
    const ok = await dialog.confirm(t('promptMgmt.confirmDiscardUnsaved'))
    if (!ok) return
  }
  select(architecture, stage)
  originalTemplateHash.value = calcTemplateHash(editingTemplate.value)
  isDirty.value = false
}

async function refresh() {
  statusText.value = 'Loading...'
  try {
    const list = await invoke<PromptTemplate[]>('list_prompt_templates_api')
    // 缓存所有模板供组映射区域下拉使用
    allTemplates.value = list
    // 根据分类填充左侧模板列表
    if (selectedCategory.value === 'LlmArchitecture') {
      templates.value = allTemplates.value.filter(t => 
        t.architecture === selected.value.architecture && t.stage === selected.value.stage
      )
    } else {
      // 非架构类分类展示全量，交由 filteredTemplates 再做二次过滤
      templates.value = list
    }
  } catch (e) {
    // Fallback: 使用旧命令（仅返回ID），构造占位模板以避免前端报错
    try {
      const ids = await invoke<string[]>('list_prompt_templates')
      templates.value = ids.map((id, idx) => ({
        id: idx as unknown as number,
        name: id,
        description: '',
        architecture: selected.value.architecture,
        stage: selected.value.stage,
        content: '',
        is_default: false,
        is_active: true,
      }))
    } catch (_) {
      templates.value = []
    }
  }
  await loadGroups()
  await loadActiveId()
  statusText.value = 'Ready'
}

async function loadActiveId() {
  try {
    const configs = await invoke<Array<{ architecture: ArchitectureType; stage: StageType; template_id: number }>>('get_user_prompt_configs_api')
    const c = configs.find(c => c.architecture === selected.value.architecture && c.stage === selected.value.stage)
    if (c) {
      activePromptId.value = c.template_id as unknown as number
      // 若当前编辑模板与激活不同，不应误判为脏
      if (editingTemplate.value && editingTemplate.value.id === activePromptId.value) {
        originalTemplateHash.value = calcTemplateHash(editingTemplate.value)
        isDirty.value = false
      }
      return
    }
    // fallback: 默认组
    const defId = defaultGroupId.value
    if (defId) {
      await loadGroupItems(defId)
      const tid = groupItems.value[selected.value.stage]
      activePromptId.value = tid ?? null
    } else {
      activePromptId.value = null
    }
  } catch (_) {
    activePromptId.value = null
  }
}

// 加载默认分组中当前阶段的默认提示词并显示在编辑器中
async function loadDefaultStagePrompt() {
  try {
    console.log('[loadDefaultStagePrompt] Starting...', {
      architecture: selected.value.architecture,
      stage: selected.value.stage,
      category: selectedCategory.value
    })
    
    // 优先使用默认分组，如果没有默认分组则使用第一个分组
    let defId = defaultGroupId.value
    console.log('[loadDefaultStagePrompt] Default group ID:', defId)
    console.log('[loadDefaultStagePrompt] All groups:', promptGroups.value)
    
    if (!defId && promptGroups.value.length > 0) {
      defId = promptGroups.value[0].id || null
      console.log('[loadDefaultStagePrompt] No default group, using first group:', defId)
    }
    
    if (!defId) {
      console.log('[loadDefaultStagePrompt] No group found')
      editingTemplate.value = null
      return
    }
    
    // 加载默认分组的阶段映射
    await loadGroupItems(defId)
    console.log('[loadDefaultStagePrompt] Group items loaded:', groupItems.value)
    
    // 获取当前阶段对应的模板ID
    const templateId = groupItems.value[selected.value.stage]
    console.log('[loadDefaultStagePrompt] Template ID for stage:', templateId)
    
    if (!templateId) {
      console.log('[loadDefaultStagePrompt] No template ID found for stage:', selected.value.stage)
      editingTemplate.value = null
      return
    }
    
    // 从缓存的所有模板中查找对应的模板
    console.log('[loadDefaultStagePrompt] All templates count:', allTemplates.value.length)
    console.log('[loadDefaultStagePrompt] Looking for template with ID:', templateId)
    const template = allTemplates.value.find(t => t.id === templateId)
    console.log('[loadDefaultStagePrompt] Found template:', template)
    
    if (template) {
      console.log('[loadDefaultStagePrompt] Loading template:', template.name)
      loadTemplate(template)
    } else {
      console.log('[loadDefaultStagePrompt] Template not found in cache')
      editingTemplate.value = null
    }
  } catch (error) {
    console.error('[loadDefaultStagePrompt] Error:', error)
    editingTemplate.value = null
  }
}

function newTemplate() {
  const baseTemplate = {
    name: selectedCategory.value === 'LlmArchitecture' 
      ? `${selected.value.architecture}-${selected.value.stage}-${Date.now()}`
      : `${selectedCategory.value}-${Date.now()}`,
    description: '',
    content: '',
    is_default: false,
    is_active: true,
    // 新增字段
    category: selectedCategory.value,
    template_type: selectedCategory.value === 'System' ? 'SystemPrompt' as TemplateType : 'Custom' as TemplateType,
    is_system: selectedCategory.value === 'System',
    priority: 50,
    tags: [],
    variables: [],
    version: '1.0.0',
  }
  
  // 根据分类设置不同的字段
  if (selectedCategory.value === 'LlmArchitecture') {
    editingTemplate.value = {
      ...baseTemplate,
      architecture: selected.value.architecture,
      stage: selected.value.stage,
      target_architecture: selected.value.architecture,
    }
  } else {
    editingTemplate.value = {
      ...baseTemplate,
      architecture: 'ReWOO' as ArchitectureType, // 默认值，可能不会使用
      stage: 'Planner' as StageType, // 默认值，可能不会使用
    }
  }
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
  await invoke('update_user_prompt_config_api', {
    architecture: selected.value.architecture,
    stage: selected.value.stage,
    template_id: editingTemplate.value.id,
  } as any)
  activePromptId.value = editingTemplate.value.id ?? null
  originalTemplateHash.value = calcTemplateHash(editingTemplate.value)
  toast.success(t('promptMgmt.activatedToast') as unknown as string)
}

onMounted(async () => {
  await refresh()
  // 初始加载时也显示默认分组的默认提示词
  if (selectedCategory.value === 'LlmArchitecture') {
    await loadDefaultStagePrompt()
  }
  const onKey = (e: KeyboardEvent) => {
    const isMac = navigator.platform.toLowerCase().includes('mac')
    const ctrlOrCmd = isMac ? e.metaKey : e.ctrlKey
    if (ctrlOrCmd && e.key.toLowerCase() === 's') {
      e.preventDefault()
      saveTemplate()
    }
    if (ctrlOrCmd && e.key === 'Enter') {
      e.preventDefault()
      if (selectedCategory.value === 'LlmArchitecture') activateTemplate()
    }
    if (ctrlOrCmd && (e.key === 'Backspace' || e.key === 'Delete')) {
      e.preventDefault()
      removeTemplate()
    }
  }
  window.addEventListener('keydown', onKey)
  const onBeforeUnload = (e: BeforeUnloadEvent) => {
    if (isDirty.value) {
      e.preventDefault()
      e.returnValue = ''
    }
  }
  window.addEventListener('beforeunload', onBeforeUnload)
  // cleanup
  onBeforeUnmount(() => {
    window.removeEventListener('keydown', onKey)
    window.removeEventListener('beforeunload', onBeforeUnload)
  })
})

watch(editingTemplate, () => {
  // 外层对象切换时不设为脏
}, { deep: false })

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
    editingTemplate.value?.architecture,
    editingTemplate.value?.stage,
    editingTemplate.value?.target_architecture,
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
  selectedGroupId.value = null
  await refresh()
  isDirty.value = false
})

// ===== Prompt Group helpers =====
const selectedGroup = computed(() => promptGroups.value.find(g => g.id === selectedGroupId.value) || null)

async function loadGroups() {
  try {
    console.log('[loadGroups] Loading groups for architecture:', selected.value.architecture)
    const list = await invoke<PromptGroup[]>('list_prompt_groups_api', { architecture: selected.value.architecture })
    console.log('[loadGroups] Loaded groups:', list)
    promptGroups.value = list
    if (!selectedGroupId.value && list.length) {
      selectedGroupId.value = (list.find(g => g.is_default)?.id ?? list[0].id) || null
      console.log('[loadGroups] Selected group ID:', selectedGroupId.value)
      if (selectedGroupId.value) await loadGroupItems(selectedGroupId.value)
    }
  } catch (e) {
    console.error('[loadGroups] Error loading groups:', e)
    promptGroups.value = []
  }
}

function selectGroup(id: number) {
  selectedGroupId.value = id
  loadGroupItems(id)
}

async function createGroup() {
  const name = await dialog.input({
    title: t('promptMgmt.groups') as unknown as string,
    message: t('promptMgmt.groupNamePrompt') as unknown as string,
    placeholder: t('promptMgmt.groupNamePlaceholder') as unknown as string,
    variant: 'primary'
  })
  if (!name || !name.trim()) return
  const group: PromptGroup = { name: name.trim(), description: '', architecture: selected.value.architecture, is_default: false }
  const id = await invoke<number>('create_prompt_group_api', { group })
  await loadGroups()
  selectedGroupId.value = id as number
  toast.success(t('promptMgmt.groupCreateSuccess') as unknown as string)
}

async function setDefaultGroup() {
  if (!selectedGroupId.value) return
  await invoke('set_arch_default_group_api', { architecture: selected.value.architecture, groupId: selectedGroupId.value } as any)
  await loadGroups()
  toast.success(t('promptMgmt.defaultGroupSet') as unknown as string)
}

async function deleteGroup() {
  if (!selectedGroupId.value) return
  
  // 防止删除默认分组
  const group = promptGroups.value.find(g => g.id === selectedGroupId.value)
  if (group?.is_default) {
    toast.error('不能删除默认分组')
    return
  }
  
  const confirmed = await dialog.confirm({
    title: t('promptMgmt.groups') as unknown as string,
    message: `确定要删除分组"${group?.name}"吗？此操作将删除该分组的所有阶段映射。`,
    variant: 'error'
  })
  
  if (!confirmed) return
  
  try {
    await invoke('delete_prompt_group_api', { id: selectedGroupId.value } as any)
    selectedGroupId.value = null
    await loadGroups()
    toast.success('分组已删除')
  } catch (error) {
    console.error('Failed to delete group:', error)
    toast.error('删除分组失败: ' + (error as any).message)
  }
}

async function loadGroupItems(groupId: number) {
  try {
    const items = await invoke<PromptGroupItem[]>('list_prompt_group_items_api', { groupId: groupId } as any)
    console.log('Loaded group items:', items)
    const map: Record<StageType, number | undefined> = {} as any
    for (const it of items) { map[it.stage] = it.template_id }
    groupItems.value = map
    console.log('Group items map:', map)
    
    // 更新草稿 - 使用分组架构的阶段而不是当前选中架构的阶段
    const draft: Record<string, number | undefined> = {}
    const stages = stagesOfGroupArch.value
    console.log('Group arch stages:', stages)
    for (const st of stages) draft[st] = map[st as StageType]
    groupMappingDraft.value = draft
    console.log('Updated draft mapping:', draft)
  } catch (error) {
    console.error('Failed to load group items:', error)
    groupItems.value = {} as any
    groupMappingDraft.value = {}
  }
}

async function onChangeGroupItem(stage: string) {
  if (!selectedGroupId.value) return
  const templateId = groupMappingDraft.value[stage]
  console.log(`Changing group item for stage ${stage}, templateId: ${templateId}`)
  
  if (templateId == null) {
    // 选择"未设置"时移除该映射
    console.log('Removing group item mapping')
    await invoke('remove_prompt_group_item_api', { groupId: selectedGroupId.value, stage } as any)
  } else {
    // 设置新的映射
    console.log('Setting group item mapping')
    await invoke('upsert_prompt_group_item_api', { groupId: selectedGroupId.value, stage, templateId: templateId } as any)
  }
  
  // 重新加载分组项以刷新UI
  await loadGroupItems(selectedGroupId.value)
  
  if (!activePromptId.value && defaultGroupId.value === selectedGroupId.value && stage === selected.value.stage) {
    activePromptId.value = templateId as number
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
    architecture: 'ReWOO' as ArchitectureType,
    stage: 'Planner' as StageType,
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
    architecture: 'ReWOO' as ArchitectureType,
    stage: 'Planner' as StageType,
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
    architecture: 'ReWOO' as ArchitectureType,
    stage: 'Planner' as StageType,
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
    architecture: 'ReWOO' as ArchitectureType,
    stage: 'Planner' as StageType,
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

// 创建插件接口模板(被动扫描)
function createPluginInterfaceTemplate() {
  const defaultContent = `## Plugin Interface (Required Structure)

Your generated plugin MUST implement the following TypeScript interface:

**Variables**: None (this is a static interface definition)

Please ensure all generated plugins follow this exact interface structure.`

  editingTemplate.value = {
    name: `被动扫描插件接口模板-${Date.now()}`,
    description: '定义被动扫描插件接口和API的模板',
    architecture: 'ReWOO' as ArchitectureType,
    stage: 'Planner' as StageType,
    content: defaultContent,
    is_default: false,
    is_active: true,
    category: 'Application' as PromptCategory,
    template_type: 'PluginInterface' as TemplateType,
    is_system: true,
    priority: 80,
    tags: ['plugin', 'interface', 'api', 'passive'],
    variables: [],
    version: '1.0.0',
  }
  isDirty.value = false
}

// 创建插件输出格式模板(被动扫描)
function createPluginOutputFormatTemplate() {
  const defaultContent = `## Output Format

Return ONLY the TypeScript plugin code wrapped in a markdown code block.

**Variables**: None (this is a static format requirement)

Ensure the output follows the exact format specified.`

  editingTemplate.value = {
    name: `被动扫描插件输出格式模板-${Date.now()}`,
    description: '定义被动扫描插件输出格式要求的模板',
    architecture: 'ReWOO' as ArchitectureType,
    stage: 'Planner' as StageType,
    content: defaultContent,
    is_default: false,
    is_active: true,
    category: 'Application' as PromptCategory,
    template_type: 'PluginOutputFormat' as TemplateType,
    is_system: true,
    priority: 75,
    tags: ['plugin', 'output', 'format', 'passive'],
    variables: [],
    version: '1.0.0',
  }
  isDirty.value = false
}

// 创建Agent插件接口模板
function createAgentPluginInterfaceTemplate() {
  const defaultContent = `## Agent Tool Plugin Interface (Required Structure)

Your generated Agent tool plugin MUST implement the following TypeScript interface:

**Variables**: None (this is a static interface definition)

Please ensure all generated Agent tool plugins follow this exact interface structure.`

  editingTemplate.value = {
    name: `Agent工具插件接口模板-${Date.now()}`,
    description: '定义Agent工具插件接口和API的模板',
    architecture: 'ReWOO' as ArchitectureType,
    stage: 'Planner' as StageType,
    content: defaultContent,
    is_default: false,
    is_active: true,
    category: 'Application' as PromptCategory,
    template_type: 'PluginInterface' as TemplateType,
    is_system: true,
    priority: 80,
    tags: ['agent', 'plugin', 'interface', 'api'],
    variables: [],
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
    architecture: 'ReWOO' as ArchitectureType,
    stage: 'Planner' as StageType,
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

// 创建Agent插件输出格式模板
function createAgentPluginOutputFormatTemplate() {
  const defaultContent = `## Agent Tool Plugin Output Format

Return ONLY the TypeScript plugin code wrapped in a markdown code block.

**Variables**: None (this is a static format requirement)

Ensure the output follows the exact Agent tool plugin format.`

  editingTemplate.value = {
    name: `Agent插件输出格式模板-${Date.now()}`,
    description: '定义Agent工具插件输出格式要求的模板',
    architecture: 'ReWOO' as ArchitectureType,
    stage: 'Planner' as StageType,
    content: defaultContent,
    is_default: false,
    is_active: true,
    category: 'Application' as PromptCategory,
    template_type: 'AgentPluginOutputFormat' as TemplateType,
    is_system: true,
    priority: 75,
    tags: ['agent', 'plugin', 'output', 'format'],
    variables: [],
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
      template_id: editingTemplate.value.id,
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
.btn { padding: 0.25rem 0.75rem; border: 1px solid #e5e7eb; border-radius: 0.25rem; background: #fff; font-size: 0.875rem; }
.btn:hover { background: #f9fafb; }
.input { width: 100%; border: 1px solid #e5e7eb; border-radius: 0.25rem; padding: 0.25rem 0.5rem; }
</style>


