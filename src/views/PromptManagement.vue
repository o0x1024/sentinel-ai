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
      <div class="px-4 pb-3 text-xs opacity-60 flex items-center gap-2">
        <span>{{ $t('promptMgmt.active') }}</span>
        <span v-if="activePromptId">#{{ activePromptId }}</span>
        <span v-else>{{ $t('promptMgmt.none') }}</span>
      </div>
      <!-- 分组管理 -->
      <div class="px-4 pb-2">
        <div class="flex items-center justify-between mb-1">
          <div class="text-xs opacity-70">{{ $t('promptMgmt.groups') }}</div>
          <div class="flex items-center gap-2">
            <button class="btn btn-xs" @click="createGroup">{{ $t('promptMgmt.new') }}</button>
            <button class="btn btn-xs" :disabled="!selectedGroupId" @click="setDefaultGroup">{{ $t('promptMgmt.setDefault') }}</button>
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
                <div class="font-medium text-xs truncate">{{ t.name }}</div>
                <div class="text-[10px] opacity-70 truncate">#{{ t.id }} · {{ t.architecture }} / {{ t.stage }}</div>
              </div>
              <span v-if="t.id === activePromptId" class="badge badge-success badge-xs">{{ $t('promptMgmt.activeBadge') }}</span>
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
          <div class="flex flex-wrap items-center gap-3">
            <div class="text-sm opacity-70">
              {{ $t('promptMgmt.toolbarContext', { architecture: selected.architecture, stage: selected.stage }) }}
              <span v-if="isDirty" class="ml-2 badge badge-warning badge-sm">{{ $t('promptMgmt.unsavedBadge') }}</span>
            </div>
            <div class="divider divider-horizontal m-0"></div>
            <button class="btn btn-success btn-sm hover:brightness-95 active:brightness-90 shadow-sm" @click="onNewWithGuard">{{ $t('common.create') }}</button>
            <button class="btn btn-success btn-sm hover:brightness-95 active:brightness-90 shadow-sm" :disabled="!editingTemplate" @click="saveTemplate">{{ $t('common.save') }}</button>
            <button class="btn btn-outline btn-sm hover:brightness-95 active:brightness-90 shadow-sm" :disabled="!editingTemplate?.id" @click="activateTemplate">{{ $t('promptMgmt.active') }}</button>
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
                  <option value="ReportGenerator">报告生成器</option>
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
            </div>
            
            <textarea v-model="editingTemplate.content" class="textarea textarea-bordered font-mono text-sm h-full grow" :placeholder="$t('promptMgmt.contentPlaceholder') as string"></textarea>
          </div>
          <div class="card-body p-4 h-full flex items-center justify-center text-sm opacity-60" v-else>
            {{ $t('promptMgmt.noTemplateSelected') }}
          </div>
        </div>

        <!-- 预览卡片 -->
        <div class="card bg-base-100 shadow-md h-full overflow-hidden">
          <div class="card-body p-4 h-full overflow-hidden">
            <div class="text-sm font-medium mb-2">{{ $t('promptMgmt.preview') }}</div>
            <div class="mockup-code text-xs overflow-auto h-full">
              <pre data-prefix=">"><code>{{ preview }}</code></pre>
            </div>
            <div class="text-[10px] opacity-60 mt-2">{{ $t('promptMgmt.shortcuts') }}</div>
          </div>
        </div>
      </div>

      <!-- 组阶段映射管理 -->
      <div class="card bg-base-100 shadow-md">
        <div class="card-body p-4">
          <div class="flex items-center justify-between mb-3">
            <h4 class="card-title text-sm">{{ $t('promptMgmt.groupMapping') }}</h4>
            <div class="text-xs opacity-70">{{ $t('promptMgmt.currentGroup') }}：<span class="font-medium">{{ selectedGroup?.name || $t('promptMgmt.notSelected') }}</span></div>
          </div>
          <div class="grid grid-cols-3 gap-3">
            <div v-for="st in stagesOfSelectedArch" :key="st" class="flex flex-col gap-1">
              <div class="text-xs opacity-70">{{ st }}</div>
              <select class="select select-bordered select-xs" :disabled="!selectedGroupId" v-model.number="groupMappingDraft[st]" @change="onChangeGroupItem(st)">
                <option :value="undefined">{{ $t('promptMgmt.notSet') }}</option>
                <option v-for="t in allTemplatesByStage[st] || []" :key="t.id" :value="t.id">#{{ t.id }} · {{ t.name }}</option>
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

type ArchitectureType = 'ReWOO' | 'LLMCompiler' | 'PlanExecute'
type StageType = 'Planner' | 'Worker' | 'Solver' | 'Planning' | 'Execution' | 'Replan'
type PromptCategory = 'System' | 'LlmArchitecture' | 'Application' | 'UserDefined'
type TemplateType = 'SystemPrompt' | 'IntentClassifier' | 'Planner' | 'Executor' | 'Replanner' | 'ReportGenerator' | 'Domain' | 'Custom'

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
    { value: 'Planner', label: 'Planner' },
    { value: 'Worker', label: 'Worker' },
    { value: 'Solver', label: 'Solver' },
  ]},
  { value: 'LLMCompiler', label: 'LLMCompiler', stages: [
    { value: 'Planning', label: 'Planning' },
    { value: 'Execution', label: 'Execution' },
    { value: 'Replan', label: 'Replan' },
  ]},
  { value: 'PlanExecute', label: 'Plan&Execute', stages: [
    { value: 'Planning', label: 'Planning' },
    { value: 'Execution', label: 'Execution' },
    { value: 'Replan', label: 'Replan' },
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

// 组相关
const promptGroups = ref<PromptGroup[]>([])
const selectedGroupId = ref<number | null>(null)
const groupItems = ref<Record<StageType, number | undefined>>({} as any)
const groupMappingDraft = ref<Record<string, number | undefined>>({})
const defaultGroupId = computed(() => promptGroups.value.find(g => g.is_default)?.id || null)

const preview = computed(() => editingTemplate.value?.content ?? '')
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
  if (selected.value.architecture === 'ReWOO') return ['Planner','Worker','Solver'] as StageType[]
  if (selected.value.architecture === 'LLMCompiler') return ['Planning','Execution','Replan'] as StageType[]
  return ['Planning','Execution','Replan'] as StageType[]
})

const allTemplatesByStage = computed<Record<string, PromptTemplate[]>>(() => {
  const map: Record<string, PromptTemplate[]> = {}
  for (const st of stagesOfSelectedArch.value) {
    map[st] = allTemplates.value.filter(t => t.stage === st)
  }
  return map
})

// 从后端拿到所有模板后缓存一份，便于分组映射下拉使用
const allTemplates = ref<PromptTemplate[]>([])

function select(architecture: ArchitectureType, stage: StageType) {
  selected.value = { architecture, stage }
  refresh()
}

async function onSelectWithGuard(architecture: ArchitectureType, stage: StageType) {
  if (isDirty.value) {
    const ok = await dialog.confirm(t('promptMgmt.confirmDiscardUnsaved'))
    if (!ok) return
  }
  select(architecture, stage)
  isDirty.value = false
}

async function refresh() {
  statusText.value = 'Loading...'
  try {
    const list = await invoke<PromptTemplate[]>('list_prompt_templates_api')
    allTemplates.value = list.filter(t => t.architecture === selected.value.architecture)
    templates.value = allTemplates.value.filter(t => t.stage === selected.value.stage)
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
  await Promise.all([loadGroups(), loadActiveId()])
  statusText.value = 'Ready'
}

async function loadActiveId() {
  try {
    const configs = await invoke<Array<{ architecture: ArchitectureType; stage: StageType; template_id: number }>>('get_user_prompt_configs_api')
    const c = configs.find(c => c.architecture === selected.value.architecture && c.stage === selected.value.stage)
    if (c) {
      activePromptId.value = c.template_id as unknown as number
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
  if (tpl.id) {
    await invoke('update_prompt_template_api', { id: tpl.id, template: tpl })
  } else {
    const id = await invoke<number>('create_prompt_template_api', { template: tpl })
    editingTemplate.value.id = id
  }
  await refresh()
  isDirty.value = false
  toast.success(t('promptMgmt.savedToast') as unknown as string)
}

async function removeTemplate() {
  if (!editingTemplate.value?.id) return
  const confirmed = await dialog.confirm(t('promptMgmt.confirmDeleteTemplate'))
  if (!confirmed) return
  await invoke('delete_prompt_template_api', { id: editingTemplate.value.id })
  editingTemplate.value = null
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
  toast.success(t('promptMgmt.activatedToast') as unknown as string)
}

onMounted(() => {
  refresh()
  const onKey = (e: KeyboardEvent) => {
    const isMac = navigator.platform.toLowerCase().includes('mac')
    const ctrlOrCmd = isMac ? e.metaKey : e.ctrlKey
    if (ctrlOrCmd && e.key.toLowerCase() === 's') {
      e.preventDefault()
      saveTemplate()
    }
    if (ctrlOrCmd && e.key === 'Enter') {
      e.preventDefault()
      activateTemplate()
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

watch(() => [editingTemplate.value?.name, editingTemplate.value?.description, editingTemplate.value?.content], () => {
  if (editingTemplate.value?.id) {
    isDirty.value = true
  } else if (editingTemplate.value) {
    // 新建也认为脏
    isDirty.value = true
  }
})

// ===== Prompt Group helpers =====
const selectedGroup = computed(() => promptGroups.value.find(g => g.id === selectedGroupId.value) || null)

async function loadGroups() {
  try {
    const list = await invoke<PromptGroup[]>('list_prompt_groups_api', { architecture: selected.value.architecture })
    promptGroups.value = list
    if (!selectedGroupId.value && list.length) {
      selectedGroupId.value = (list.find(g => g.is_default)?.id ?? list[0].id) || null
      if (selectedGroupId.value) await loadGroupItems(selectedGroupId.value)
    }
  } catch (e) {
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

async function loadGroupItems(groupId: number) {
  try {
    const items = await invoke<PromptGroupItem[]>('list_prompt_group_items_api', { groupId: groupId } as any)
    const map: Record<StageType, number | undefined> = {} as any
    for (const it of items) { map[it.stage] = it.template_id }
    groupItems.value = map
    // 更新草稿
    const draft: Record<string, number | undefined> = {}
    for (const st of stagesOfSelectedArch.value) draft[st] = map[st as StageType]
    groupMappingDraft.value = draft
  } catch (_) {
    groupItems.value = {} as any
    groupMappingDraft.value = {}
  }
}

async function onChangeGroupItem(stage: string) {
  if (!selectedGroupId.value) return
  const templateId = groupMappingDraft.value[stage]
  if (templateId === undefined) return
  await invoke('upsert_prompt_group_item_api', { groupId: selectedGroupId.value, stage, templateId: templateId } as any)
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
  }
  isDirty.value = false // 这是新创建的模板，不算脏数据
}
</script>

<style scoped>
.btn { padding: 0.25rem 0.75rem; border: 1px solid #e5e7eb; border-radius: 0.25rem; background: #fff; font-size: 0.875rem; }
.btn:hover { background: #f9fafb; }
.input { width: 100%; border: 1px solid #e5e7eb; border-radius: 0.25rem; padding: 0.25rem 0.5rem; }
</style>


