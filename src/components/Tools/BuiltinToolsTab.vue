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

    <div v-if="sourceFilter === 'builtin'" class="card bg-base-100 border border-base-300 shadow-sm">
      <div class="card-body gap-3">
        <div
          v-if="focusedMemoryId"
          class="rounded-lg border border-info/30 bg-info/10 px-3 py-2"
        >
          <div class="flex flex-col gap-2 lg:flex-row lg:items-center lg:justify-between">
            <div class="text-sm text-base-content/75 break-all">
              当前已定位到 memory:
              <code class="ml-1 text-info">{{ focusedMemoryId }}</code>
            </div>
            <div class="flex flex-wrap gap-2">
              <button class="btn btn-xs btn-outline" @click="copyMemoryId(focusedMemoryId)">
                <i class="fas fa-copy mr-1"></i>
                复制 ID
              </button>
              <button
                v-if="focusedMemoryDiagnostics && canOpenOriginConversation(focusedMemoryDiagnostics.record)"
                class="btn btn-xs btn-outline btn-secondary"
                @click="openOriginConversation(focusedMemoryDiagnostics.record)"
              >
                <i class="fas fa-crosshairs mr-1"></i>
                定位到对话消息
              </button>
              <button class="btn btn-xs btn-outline btn-info" @click="clearFocusedMemory">
                清除定位
              </button>
            </div>
          </div>
        </div>

        <div class="flex flex-col gap-3 lg:flex-row lg:items-center lg:justify-between">
          <div>
            <h3 class="card-title text-base">
              <i class="fas fa-memory text-info mr-2"></i>
              Memory Diagnostics
            </h3>
            <p class="text-sm text-base-content/70">
              查看 durable memory 的 canonical 记录和 projection health。
            </p>
          </div>
          <div class="flex flex-wrap items-center gap-2">
            <label class="label cursor-pointer gap-2 py-0">
              <span class="label-text text-sm">仅问题</span>
              <input v-model="memoryDiagnosticsOnlyIssues" type="checkbox" class="toggle toggle-warning toggle-sm" />
            </label>
            <button
              class="btn btn-sm btn-outline"
              :class="{ 'btn-disabled': memoryDiagnosticsLoading }"
              @click="refreshMemoryDiagnostics"
            >
              <i :class="['fas', memoryDiagnosticsLoading ? 'fa-spinner fa-spin' : 'fa-sync-alt']"></i>
              刷新
            </button>
          </div>
        </div>

        <div class="flex flex-wrap gap-2 text-xs">
          <span class="badge badge-ghost">记录 {{ memoryDiagnostics.length }}</span>
          <span class="badge badge-success">可检索 {{ memoryDiagnosticsReadyCount }}</span>
          <span class="badge badge-warning">异常 {{ memoryDiagnosticsIssueCount }}</span>
        </div>

        <div v-if="memoryDiagnosticsLoading" class="flex items-center gap-2 text-sm text-base-content/70">
          <span class="loading loading-spinner loading-sm"></span>
          <span>正在加载 memory diagnostics...</span>
        </div>
        <div v-else-if="memoryDiagnostics.length === 0" class="text-sm text-base-content/60">
          当前没有可显示的 durable memory diagnostics。
        </div>
        <div v-else class="overflow-x-auto">
          <table class="table table-sm">
            <thead>
              <tr>
                <th>Memory</th>
                <th>类型</th>
                <th>来源</th>
                <th>Projection</th>
                <th>状态</th>
                <th class="w-24">操作</th>
              </tr>
            </thead>
            <tbody>
              <tr
                v-for="item in memoryDiagnostics"
                :id="`memory-diagnostic-${item.record.id}`"
                :key="item.record.id"
                :class="item.record.id === focusedMemoryId ? 'bg-info/10' : ''"
              >
                <td class="max-w-md">
                  <div class="font-medium truncate">{{ item.record.title || item.record.id }}</div>
                  <div class="text-xs text-base-content/60 line-clamp-2 break-all">
                    {{ item.record.text }}
                  </div>
                </td>
                <td>
                  <div class="flex flex-wrap gap-1">
                    <span class="badge badge-ghost badge-sm">{{ item.record.kind }}</span>
                    <span class="badge badge-ghost badge-sm">{{ item.record.scope }}</span>
                  </div>
                </td>
                <td>
                  <div class="text-xs">{{ item.record.source }}</div>
                  <div class="text-[11px] text-base-content/50">
                    {{ formatTimestamp(item.record.updated_at_ms) }}
                  </div>
                  <div
                    v-if="item.record.origin_execution_id"
                    class="mt-1 text-[11px] text-base-content/50 break-all"
                  >
                    execution:
                    <code>{{ item.record.origin_execution_id }}</code>
                  </div>
                </td>
                <td>
                  <div class="flex flex-wrap gap-1">
                    <span :class="projectionBadgeClass(item.projection?.lexical_indexed)">lexical</span>
                    <span :class="projectionBadgeClass(item.projection?.vector_indexed)">vector</span>
                    <span :class="projectionBadgeClass(item.projection?.skill_projected)">skill</span>
                  </div>
                </td>
                <td class="max-w-xs">
                  <div class="flex flex-wrap gap-1 mb-1">
                    <span :class="item.retrievable_projection_ready ? 'badge badge-success badge-sm' : 'badge badge-warning badge-sm'">
                      {{ item.retrievable_projection_ready ? 'retrievable' : 'degraded' }}
                    </span>
                    <span v-if="item.projection_issue" class="badge badge-warning badge-sm">issue</span>
                  </div>
                  <div v-if="item.projection?.last_error" class="text-[11px] text-warning break-all">
                    {{ item.projection.last_error }}
                  </div>
                </td>
                <td>
                  <div class="flex items-center gap-1">
                    <button
                      class="btn btn-ghost btn-xs"
                      :title="`复制 ${item.record.id}`"
                      @click="copyMemoryId(item.record.id)"
                    >
                      <i class="fas fa-copy"></i>
                    </button>
                    <button
                      v-if="canOpenOriginConversation(item.record)"
                      class="btn btn-ghost btn-xs"
                      title="定位到来源会话中的消息"
                      @click="openOriginConversation(item.record)"
                    >
                      <i class="fas fa-crosshairs"></i>
                    </button>
                  </div>
                </td>
              </tr>
            </tbody>
          </table>
        </div>
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
                  <button
                    v-if="tool.name === 'search_exploit'"
                    @click="showExploitDbModal = true"
                    class="btn btn-secondary btn-sm"
                    :title="$t('Tools.exploitdb.openManager')"
                  >
                    <i class="fas fa-database mr-1"></i>
                    {{ $t('Tools.exploitdb.openManager') }}
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
                    <button
                      v-if="tool.name === 'search_exploit'"
                      @click="showExploitDbModal = true"
                      class="btn btn-secondary btn-xs"
                      :title="$t('Tools.exploitdb.openManager')"
                    >
                      <i class="fas fa-database"></i>
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
      :initial-params="testingToolInitialParams"
      :execution-info="{
        type: 'unified', // or 'builtin', handled as direct toolName call
        name: testingTool?.name
      }"
    />

    <!-- Shell 配置模态框 -->
    <ShellConfigModal v-model="showShellConfigModal" />

    <ExploitDbManagerModal v-model="showExploitDbModal" @open-tool-test="openExploitDbToolTest" />

    <!-- Shell 终端模态框 -->
    <ShellTerminal v-model="showShellTerminal" />

  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { invoke } from '@tauri-apps/api/core'
import { dialog } from '@/composables/useDialog'
import ShellConfigModal from './ShellConfigModal.vue'
import ShellTerminal from './ShellTerminal.vue'
import UnifiedToolTest from './UnifiedToolTest.vue'
import ExploitDbManagerModal from './ExploitDbManagerModal.vue'

type BuiltinSourceFilter = 'builtin' | 'workflow' | 'plugin'

const props = withDefaults(defineProps<{
  sourceFilter?: BuiltinSourceFilter
  showContent?: boolean
  workflowCount?: number
  pluginCount?: number
  focusedMemoryId?: string | null
}>(), {
  sourceFilter: 'builtin',
  showContent: true,
  workflowCount: 0,
  pluginCount: 0,
  focusedMemoryId: null,
})

const emit = defineEmits<{
  (e: 'source-filter-change', filter: BuiltinSourceFilter): void
}>()
const route = useRoute()
const router = useRouter()

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

interface DurableMemoryProjectionState {
  memory_id: string
  lexical_indexed: boolean
  vector_indexed: boolean
  skill_projected: boolean
  last_error?: string | null
  updated_at_ms: number
}

interface DurableMemoryRecord {
  id: string
  title?: string | null
  text: string
  kind: string
  tier: string
  scope: string
  stability: string
  source: string
  confidence: number
  importance: number
  origin_execution_id?: string | null
  updated_at_ms: number
}

interface DurableMemoryDiagnosticsItem {
  record: DurableMemoryRecord
  projection?: DurableMemoryProjectionState | null
  retrievable_projection_ready: boolean
  projection_issue: boolean
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
const showExploitDbModal = ref(false)
const testingTool = ref<any>(null)
const testingToolInitialParams = ref<Record<string, unknown> | null>(null)
const selectedCategory = ref('')
const memoryDiagnostics = ref<DurableMemoryDiagnosticsItem[]>([])
const memoryDiagnosticsLoading = ref(false)
const memoryDiagnosticsOnlyIssues = ref(true)
const sourceFilter = computed(() => props.sourceFilter)
const showContent = computed(() => props.showContent)
const workflowCount = computed(() => props.workflowCount ?? 0)
const pluginCount = computed(() => props.pluginCount ?? 0)
const focusedMemoryId = computed(() => {
  const value = String(props.focusedMemoryId || '').trim()
  return value || ''
})
const infoText = computed(() => {
  if (sourceFilter.value === 'workflow') {
    return '这些是在工作流工作室中标记为工具的工作流，可供AI助手调用执行。'
  }
  if (sourceFilter.value === 'plugin') {
    return '管理 Agent 插件工具，可在创建 Agent 时选择启用的插件工具。'
  }
  return '这些是系统内置的工具，已自动注册并可供AI助手调用。'
})

const memoryDiagnosticsReadyCount = computed(() =>
  memoryDiagnostics.value.filter(item => item.retrievable_projection_ready).length,
)

const memoryDiagnosticsIssueCount = computed(() =>
  memoryDiagnostics.value.filter(item => item.projection_issue).length,
)

const focusedMemoryDiagnostics = computed(() =>
  memoryDiagnostics.value.find(item => item.record.id === focusedMemoryId.value) ?? null,
)

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

async function fetchMemoryDiagnostics() {
  if (sourceFilter.value !== 'builtin') return
  memoryDiagnosticsLoading.value = true
  try {
    const diagnostics = await invoke<DurableMemoryDiagnosticsItem[]>('list_durable_memory_diagnostics', {
      limit: 20,
      onlyIssues: focusedMemoryId.value ? false : memoryDiagnosticsOnlyIssues.value,
    })
    const result = Array.isArray(diagnostics) ? diagnostics : []
    if (focusedMemoryId.value) {
      const exact = await invoke<DurableMemoryDiagnosticsItem[]>('get_durable_memory_diagnostics_by_ids', {
        memoryIds: [focusedMemoryId.value],
      })
      const merged = new Map<string, DurableMemoryDiagnosticsItem>()
      for (const item of result) merged.set(item.record.id, item)
      for (const item of Array.isArray(exact) ? exact : []) merged.set(item.record.id, item)
      memoryDiagnostics.value = Array.from(merged.values())
      await scrollToFocusedMemory()
      return
    }
    memoryDiagnostics.value = result
  } catch (error) {
    console.error('Failed to fetch memory diagnostics:', error)
    memoryDiagnostics.value = []
    dialog.toast.error(`加载 memory diagnostics 失败：${String(error)}`)
  } finally {
    memoryDiagnosticsLoading.value = false
  }
}

async function refreshMemoryDiagnostics() {
  await fetchMemoryDiagnostics()
}

async function refresh() {
  await fetchTools()
  await fetchMemoryDiagnostics()
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



function openTestModal(tool: any, initialParams: Record<string, unknown> | null = null) {
  testingTool.value = { ...tool }
  testingToolInitialParams.value = initialParams
  showTestModal.value = true
}

function openExploitDbToolTest(payload: { action: 'get'; edb_id: number }) {
  const exploitTool = tools.value.find(tool => tool.name === 'search_exploit')
  if (!exploitTool) {
    dialog.toast.error('未找到 search_exploit 工具')
    return
  }

  openTestModal(exploitTool, payload)
}

function projectionBadgeClass(ok: boolean | undefined) {
  return ok === true
    ? 'badge badge-success badge-sm'
    : 'badge badge-warning badge-sm'
}

function formatTimestamp(timestamp: number) {
  if (!Number.isFinite(timestamp) || timestamp <= 0) return '-'
  return new Date(timestamp).toLocaleString()
}

async function copyMemoryId(memoryId: string) {
  const normalized = String(memoryId || '').trim()
  if (!normalized) return
  try {
    await navigator.clipboard.writeText(normalized)
    dialog.toast.success(`已复制 memory_id: ${normalized}`)
  } catch (error) {
    console.error('Failed to copy memory id:', error)
    dialog.toast.error('复制 memory_id 失败')
  }
}

function normalizeOriginExecutionId(record: DurableMemoryRecord): string {
  return String(record.origin_execution_id || '').trim()
}

function canOpenOriginConversation(record: DurableMemoryRecord): boolean {
  const executionId = normalizeOriginExecutionId(record)
  return !!executionId && executionId !== 'memory_tool'
}

function openOriginConversation(record: DurableMemoryRecord) {
  const executionId = normalizeOriginExecutionId(record)
  if (!executionId || executionId === 'memory_tool') {
    dialog.toast.error('该 memory 没有可跳转的来源会话')
    return
  }
  void router.push({
    name: 'AIAssistant',
    query: {
      conversationId: executionId,
      memoryId: record.id,
    },
  })
}

async function scrollToFocusedMemory() {
  if (!focusedMemoryId.value) return
  await new Promise(resolve => requestAnimationFrame(() => resolve(undefined)))
  const target = document.getElementById(`memory-diagnostic-${focusedMemoryId.value}`)
  target?.scrollIntoView({ behavior: 'smooth', block: 'center' })
}

function clearFocusedMemory() {
  const nextQuery = { ...route.query }
  delete nextQuery.memoryId
  void router.replace({ query: nextQuery })
}

// 暴露刷新方法供父组件调用
defineExpose({ refresh })

onMounted(() => {
  void fetchTools()
  void fetchMemoryDiagnostics()
})

watch(memoryDiagnosticsOnlyIssues, () => {
  void fetchMemoryDiagnostics()
})

watch(sourceFilter, (next) => {
  if (next === 'builtin') {
    void fetchMemoryDiagnostics()
  }
})

watch(focusedMemoryId, (next) => {
  if (next && memoryDiagnosticsOnlyIssues.value) {
    memoryDiagnosticsOnlyIssues.value = false
    return
  }
  if (sourceFilter.value === 'builtin') {
    void fetchMemoryDiagnostics()
  }
})
</script>
