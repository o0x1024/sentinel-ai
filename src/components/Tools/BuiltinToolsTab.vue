<template>
  <div class="space-y-4">
    <div class="flex justify-between items-center">
      <div class="alert alert-info flex-1 mr-4">
        <i class="fas fa-info-circle"></i>
        <span>{{ infoText }}</span>
      </div>
      <div v-if="sourceFilter === 'builtin'" class="join">
        <button @click="viewMode = 'card'" :class="['join-item', 'btn', 'btn-sm', {'btn-primary': viewMode === 'card'}]">
          <i class="fas fa-th-large"></i>
        </button>
        <button @click="viewMode = 'list'" :class="['join-item', 'btn', 'btn-sm', {'btn-primary': viewMode === 'list'}]">
          <i class="fas fa-list"></i>
        </button>
      </div>
    </div>
    
    <!-- 分类筛选 -->
    <div class="flex flex-wrap gap-2 mb-4">
      <button 
        @click="selectBuiltinCategory('')"
        :class="['btn btn-sm', sourceFilter === 'builtin' && selectedCategory === '' ? 'btn-primary' : 'btn-ghost']"
      >
        全部 ({{ tools.length }})
      </button>
      <button 
        v-for="cat in categories" 
        :key="cat.key"
        @click="selectBuiltinCategory(cat.key)"
        :class="['btn btn-sm', sourceFilter === 'builtin' && selectedCategory === cat.key ? cat.btnClass : 'btn-ghost']"
      >
        <i :class="cat.icon" class="mr-1"></i>
        {{ cat.label }} ({{ getToolCountByCategory(cat.key) }})
      </button>
      <button
        @click="selectSourceFilter('workflow')"
        :class="['btn btn-sm', sourceFilter === 'workflow' ? 'btn-secondary' : 'btn-ghost']"
      >
        <i class="fas fa-project-diagram mr-1"></i>
        工作流工具 ({{ workflowCount }})
      </button>
      <button
        @click="selectSourceFilter('plugin')"
        :class="['btn btn-sm', sourceFilter === 'plugin' ? 'btn-accent' : 'btn-ghost']"
      >
        <i class="fas fa-plug mr-1"></i>
        插件工具 ({{ pluginCount }})
      </button>
    </div>

    <div v-if="showContent && isLoading" class="text-center p-8">
      <i class="fas fa-spinner fa-spin text-2xl"></i>
      <p class="mt-2">正在加载内置工具...</p>
    </div>
    
    <!-- 卡片视图 - 按分类分组 -->
    <div v-else-if="showContent && tools.length > 0 && viewMode === 'card'" class="space-y-6">
      <div v-for="group in groupedTools" :key="group.category" class="space-y-3">
        <!-- 分类标题 -->
        <div class="flex items-center gap-2 border-b border-base-300 pb-2">
          <i :class="[getCategoryConfig(group.category).icon, getCategoryConfig(group.category).textClass]"></i>
          <h3 class="font-semibold text-lg">{{ getCategoryConfig(group.category).label }}</h3>
          <span class="badge badge-ghost badge-sm">{{ group.tools.length }} 个工具</span>
        </div>
        
        <!-- 工具卡片 -->
        <div class="grid grid-cols-1 lg:grid-cols-2 xl:grid-cols-3 gap-4">
          <div 
            v-for="tool in group.tools" 
            :key="tool.id"
            class="card bg-base-100 shadow-lg hover:shadow-xl transition-shadow"
          >
            <div class="card-body">
              <div class="flex items-center gap-3">
                <div class="avatar">
                  <div :class="['w-12 h-12 rounded-lg flex items-center justify-center', getCategoryConfig(tool.category).bgClass]">
                    <i :class="[getToolIcon(tool.name), getCategoryConfig(tool.category).textClass, 'text-xl']"></i>
                  </div>
                </div>
                <div class="flex-1">
                  <h3 class="card-title text-lg">{{ tool.name }}</h3>
                  <span :class="['badge badge-sm', getCategoryConfig(tool.category).badgeClass]">{{ getCategoryConfig(tool.category).label }}</span>
                </div>
                <div class="form-control">
                  <label class="label cursor-pointer">
                    <input 
                      type="checkbox" 
                      class="toggle toggle-success toggle-sm" 
                      :checked="tool.enabled !== false"
                      @change="toggleTool(tool)"
                      :disabled="tool.is_toggling"
                    />
                  </label>
                </div>
              </div>

              <p class="text-sm mt-2 h-16">{{ tool.description }}</p>

              <div class="card-actions justify-between items-center mt-4">
                <span class="text-xs text-base-content/60">v{{ tool.version }}</span>
                <div class="flex gap-2">
                  <!-- Shell Tool Special Actions -->
                  <button 
                    v-if="tool.name === 'shell'"
                    @click="showShellTerminal = true"
                    class="btn btn-primary btn-sm"
                    title="打开终端"
                  >
                    <i class="fas fa-terminal mr-1"></i>
                    终端
                  </button>
                  <button 
                    v-if="tool.name === 'shell'"
                    @click="showShellConfigModal = true"
                    class="btn btn-warning btn-sm"
                    title="安全配置"
                  >
                    <i class="fas fa-shield-alt"></i>
                  </button>
                  <!-- Regular Tools -->
                  <button 
                    v-if="tool.name !== 'shell'"
                    @click="openTestModal(tool)"
                    class="btn btn-primary btn-sm"
                    title="测试工具"
                  >
                    <i class="fas fa-play mr-1"></i>
                    测试
                  </button>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
    
    <!-- 列表视图 - 按分类分组 -->
    <div v-else-if="showContent && tools.length > 0 && viewMode === 'list'" class="space-y-6">
      <div v-for="group in groupedTools" :key="group.category" class="space-y-2">
        <!-- 分类标题 -->
        <div class="flex items-center gap-2 border-b border-base-300 pb-2">
          <i :class="[getCategoryConfig(group.category).icon, getCategoryConfig(group.category).textClass]"></i>
          <h3 class="font-semibold">{{ getCategoryConfig(group.category).label }}</h3>
          <span class="badge badge-ghost badge-sm">{{ group.tools.length }}</span>
        </div>
        
        <div class="overflow-x-auto">
          <table class="table w-full">
            <thead>
              <tr>
                <th class="w-1/12">启用</th>
                <th>名称</th>
                <th>描述</th>
                <th>版本</th>
                <th>操作</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="tool in group.tools" :key="tool.id">
                <td>
                  <input 
                    type="checkbox" 
                    class="toggle toggle-success toggle-sm" 
                    :checked="tool.enabled !== false"
                    @change="toggleTool(tool)"
                    :disabled="tool.is_toggling"
                  />
                </td>
                <td>
                  <div class="flex items-center gap-2">
                    <i :class="[getToolIcon(tool.name), getCategoryConfig(tool.category).textClass]"></i>
                    <span class="font-semibold">{{ tool.name }}</span>
                  </div>
                </td>
                <td class="text-sm">{{ tool.description }}</td>
                <td class="text-xs text-base-content/60">v{{ tool.version }}</td>
                <td>
                  <div class="flex gap-1">
                    <!-- Shell Tool -->
                    <button 
                      v-if="tool.name === 'shell'"
                      @click="showShellTerminal = true"
                      class="btn btn-primary btn-xs"
                      title="打开终端"
                    >
                      <i class="fas fa-terminal"></i>
                    </button>
                    <button 
                      v-if="tool.name === 'shell'"
                      @click="showShellConfigModal = true"
                      class="btn btn-warning btn-xs"
                      title="安全配置"
                    >
                      <i class="fas fa-shield-alt"></i>
                    </button>
                    <!-- Regular Tools -->
                    <button 
                      v-if="tool.name !== 'shell'"
                      @click="openTestModal(tool)"
                      class="btn btn-primary btn-xs"
                      title="测试工具"
                    >
                      <i class="fas fa-play"></i>
                    </button>
                  </div>
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>
    </div>
    
    <div v-else-if="showContent" class="text-center p-8">
      <i class="fas fa-exclamation-triangle text-4xl text-warning mb-4"></i>
      <p class="text-lg font-semibold">未找到内置工具</p>
      <p class="text-base-content/70">请检查MCP服务是否正常运行</p>
      <button @click="refresh" class="btn btn-primary mt-4">
        <i class="fas fa-sync-alt mr-2"></i>
        重新加载
      </button>
    </div>

    <!-- 统一测试组件 -->
    <UnifiedToolTest
      v-model="showTestModal"
      tool-type="builtin"
      :tool-name="testingTool?.name || ''"
      :tool-description="testingTool?.description"
      :tool-version="testingTool?.version"
      :tool-category="testingTool?.category"
      :input-schema="testingTool?.input_schema"
      :execution-info="{
        type: 'unified', // or 'builtin', handled as direct toolName call
        name: testingTool?.name
      }"
    />

    <!-- Shell 配置模态框 -->
    <ShellConfigModal v-model="showShellConfigModal" />

    <!-- Shell 终端模态框 -->
    <ShellTerminal v-model="showShellTerminal" />

  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { dialog } from '@/composables/useDialog'
import ShellConfigModal from './ShellConfigModal.vue'
import ShellTerminal from './ShellTerminal.vue'
import UnifiedToolTest from './UnifiedToolTest.vue'

type BuiltinSourceFilter = 'builtin' | 'workflow' | 'plugin'

const props = withDefaults(defineProps<{
  sourceFilter?: BuiltinSourceFilter
  showContent?: boolean
  workflowCount?: number
  pluginCount?: number
}>(), {
  sourceFilter: 'builtin',
  showContent: true,
  workflowCount: 0,
  pluginCount: 0,
})

const emit = defineEmits<{
  (e: 'source-filter-change', filter: BuiltinSourceFilter): void
}>()

// 分类配置
interface CategoryConfig {
  key: string
  label: string
  icon: string
  btnClass: string
  badgeClass: string
  bgClass: string
  textClass: string
}

const categoryConfigs: CategoryConfig[] = [
  { key: 'network', label: '网络', icon: 'fas fa-network-wired', btnClass: 'btn-info', badgeClass: 'badge-info', bgClass: 'bg-info/10', textClass: 'text-info' },
  { key: 'system', label: '系统', icon: 'fas fa-cog', btnClass: 'btn-neutral', badgeClass: 'badge-neutral', bgClass: 'bg-neutral/10', textClass: 'text-neutral' },
  { key: 'ai', label: 'AI', icon: 'fas fa-brain', btnClass: 'btn-warning', badgeClass: 'badge-warning', bgClass: 'bg-warning/10', textClass: 'text-warning' },
  { key: 'utility', label: '工具', icon: 'fas fa-tools', btnClass: 'btn-success', badgeClass: 'badge-success', bgClass: 'bg-success/10', textClass: 'text-success' },
  { key: 'other', label: '其他', icon: 'fas fa-tools', btnClass: 'btn-ghost', badgeClass: 'badge-ghost', bgClass: 'bg-base-200', textClass: 'text-base-content' },
]

const knownCategoryKeys = new Set(categoryConfigs.map(c => c.key))

const normalizeCategory = (category: unknown): string => {
  const raw = String(category || '').toLowerCase().trim()
  if (!raw) return 'utility'
  return knownCategoryKeys.has(raw) ? raw : 'other'
}

const getCategoryConfig = (category: string): CategoryConfig => {
  const normalized = normalizeCategory(category)
  return categoryConfigs.find(c => c.key === normalized) || categoryConfigs[categoryConfigs.length - 1]
}

// 状态
const tools = ref<any[]>([])
const isLoading = ref(false)
const viewMode = ref('list')
const showTestModal = ref(false)
const showShellConfigModal = ref(false)
const showShellTerminal = ref(false)
const testingTool = ref<any>(null)
const selectedCategory = ref('')
const sourceFilter = computed(() => props.sourceFilter)
const showContent = computed(() => props.showContent)
const workflowCount = computed(() => props.workflowCount ?? 0)
const pluginCount = computed(() => props.pluginCount ?? 0)
const infoText = computed(() => {
  if (sourceFilter.value === 'workflow') {
    return '这些是在工作流工作室中标记为工具的工作流，可供AI助手调用执行。'
  }
  if (sourceFilter.value === 'plugin') {
    return '管理 Agent 插件工具，可在创建 Agent 时选择启用的插件工具。'
  }
  return '这些是系统内置的工具，已自动注册并可供AI助手调用。'
})

// 计算属性：可用的分类
const categories = computed(() => {
  const cats = new Set(tools.value.map(t => normalizeCategory(t.category)))
  return categoryConfigs.filter(c => cats.has(c.key))
})

// 计算属性：按分类分组的工具
const groupedTools = computed(() => {
  let filteredTools = tools.value
  
  // 如果选择了分类，只显示该分类
  if (selectedCategory.value) {
    filteredTools = tools.value.filter(t => 
      normalizeCategory(t.category) === selectedCategory.value
    )
  }
  
  // 按分类分组
  const groups: { category: string; tools: any[] }[] = []
  const categoryOrder = categoryConfigs.map(c => c.key)
  
  for (const cat of categoryOrder) {
    const categoryTools = filteredTools.filter(t => 
      normalizeCategory(t.category) === cat
    )
    if (categoryTools.length > 0) {
      groups.push({ category: cat, tools: categoryTools })
    }
  }

  return groups
})

// 获取分类工具数量
const getToolCountByCategory = (category: string) => {
  return tools.value.filter(t => 
    normalizeCategory(t.category) === category
  ).length
}

function selectBuiltinCategory(category: string) {
  selectedCategory.value = category
  emit('source-filter-change', 'builtin')
}

function selectSourceFilter(filter: BuiltinSourceFilter) {
  emit('source-filter-change', filter)
}

function getToolIcon(toolName: string) {
  const iconMap: Record<string, string> = {
    'ask_user_question': 'fas fa-list-check',
    'shell': 'fas fa-terminal',
    'interactive_shell': 'fas fa-terminal',
    'web_search': 'fas fa-search',
    'http_request': 'fas fa-globe',
    'memory': 'fas fa-memory',
    'ocr': 'fas fa-file-image',
    'tenth_man_review': 'fas fa-user-secret',
    'todos': 'fas fa-tasks',
    'search_exploit': 'fas fa-bug',
  }
  return iconMap[toolName] || 'fas fa-tools'
}

function generateDefaultParams(schema: any): string {
  if (!schema || !schema.properties) return '{}'

  const params: any = {}
  for (const name in schema.properties) {
    const prop = schema.properties[name]
    if (prop.default !== undefined) {
      params[name] = prop.default
    } else {
      switch (prop.type) {
        case 'string': params[name] = ''; break
        case 'number':
        case 'integer': params[name] = prop.minimum !== undefined ? prop.minimum : 0; break
        case 'boolean': params[name] = false; break
        case 'array': params[name] = []; break
        case 'object': params[name] = {}; break
        default: params[name] = null
      }
    }
  }
  return JSON.stringify(params, null, 2)
}

function getToolProperties(schema: any) {
  if (!schema || !schema.properties) return []
  const requiredParams = new Set(schema.required || [])
  const properties = []
  for (const name in schema.properties) {
    const details = schema.properties[name]
    const constraints = []
    if (details.minimum !== undefined) constraints.push(`min: ${details.minimum}`)
    if (details.maximum !== undefined) constraints.push(`max: ${details.maximum}`)
    properties.push({
      name,
      type: details.type,
      required: requiredParams.has(name),
      description: details.description || '',
      constraints: constraints.join(', '),
    })
  }
  return properties
}

async function fetchTools() {
  isLoading.value = true
  try {
    const result: any[] = await invoke('get_builtin_tools_with_status')
    tools.value = result
  } catch (error) {
    console.error('Failed to fetch builtin tools:', error)
    tools.value = []
  } finally {
    isLoading.value = false
  }
}

async function refresh() {
  await fetchTools()
}

async function toggleTool(tool: any) {
  tool.is_toggling = true
  try {
    const newState = tool.enabled === false
    await invoke('toggle_builtin_tool', { toolName: tool.name, enabled: newState })
    tool.enabled = newState
    dialog.toast.success(`工具 ${tool.name} 已${newState ? '启用' : '禁用'}`)
  } catch (error: any) {
    console.error(`Failed to toggle tool ${tool.name}:`, error)
    dialog.toast.error(`切换工具 ${tool.name} 状态失败：${error?.message || error}`)
  } finally {
    tool.is_toggling = false
  }
}



function openTestModal(tool: any) {
  testingTool.value = { ...tool }
  showTestModal.value = true
}

// 暴露刷新方法供父组件调用
defineExpose({ refresh })

onMounted(() => {
  fetchTools()
})
</script>
