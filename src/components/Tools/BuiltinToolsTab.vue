<template>
  <div class="space-y-4">
    <div class="flex justify-between items-center">
      <div class="alert alert-info flex-1 mr-4">
        <i class="fas fa-info-circle"></i>
        <span>这些是系统内置的工具，已自动注册并可供AI助手调用。</span>
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
    
    <!-- 分类筛选 -->
    <div class="flex flex-wrap gap-2 mb-4">
      <button 
        @click="selectedCategory = ''"
        :class="['btn btn-sm', selectedCategory === '' ? 'btn-primary' : 'btn-ghost']"
      >
        全部 ({{ tools.length }})
      </button>
      <button 
        v-for="cat in categories" 
        :key="cat.key"
        @click="selectedCategory = cat.key"
        :class="['btn btn-sm', selectedCategory === cat.key ? cat.btnClass : 'btn-ghost']"
      >
        <i :class="cat.icon" class="mr-1"></i>
        {{ cat.label }} ({{ getToolCountByCategory(cat.key) }})
      </button>
    </div>

    <div v-if="isLoading" class="text-center p-8">
      <i class="fas fa-spinner fa-spin text-2xl"></i>
      <p class="mt-2">正在加载内置工具...</p>
    </div>
    
    <!-- 卡片视图 - 按分类分组 -->
    <div v-else-if="tools.length > 0 && viewMode === 'card'" class="space-y-6">
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
                  <!-- Vision Explorer V2 Special Actions -->
                  <button 
                    v-if="tool.name === 'vision_explorer' || tool.name === 'vision_explorer_v2'"
                    @click="openVisionExplorerModal(tool)"
                    class="btn btn-primary btn-sm"
                    title="测试 Vision Explorer"
                  >
                    <i class="fas fa-play mr-1"></i>
                    测试
                  </button>
                  <!-- Regular Tools -->
                  <button 
                    v-if="tool.name !== 'shell' && tool.name !== 'vision_explorer' && tool.name !== 'vision_explorer_v2'"
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
    <div v-else-if="tools.length > 0 && viewMode === 'list'" class="space-y-6">
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
                    <!-- Vision Explorer V2 -->
                    <button 
                      v-if="tool.name === 'vision_explorer' || tool.name === 'vision_explorer_v2'"
                      @click="openVisionExplorerModal(tool)"
                      class="btn btn-primary btn-xs"
                      title="测试 Vision Explorer"
                    >
                      <i class="fas fa-play"></i>
                    </button>
                    <!-- Regular Tools -->
                    <button 
                      v-if="tool.name !== 'shell' && tool.name !== 'vision_explorer' && tool.name !== 'vision_explorer_v2'"
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
    
    <div v-else class="text-center p-8">
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

    <!-- Vision Explorer V2 测试模态框 -->
    <dialog :class="['modal', { 'modal-open': showVisionExplorerModal }]">
      <div class="modal-box w-11/12 max-w-3xl">
        <div class="flex justify-between items-center mb-4">
          <h3 class="font-bold text-lg">
            <i class="fas fa-eye text-primary mr-2"></i>
            Vision Explorer V2 测试
          </h3>
          <button @click="closeVisionExplorerModal" class="btn btn-sm btn-ghost">✕</button>
        </div>

        <div class="space-y-4">
          <div class="alert alert-info">
            <i class="fas fa-info-circle"></i>
            <span>Vision Explorer V2 会自动探索目标网站，识别导航、表单和交互元素。</span>
          </div>

          <!-- 目标 URL -->
          <div class="form-control">
            <label class="label"><span class="label-text">目标 URL</span></label>
            <input 
              v-model="veTargetUrl"
              type="url"
              class="input input-bordered"
              placeholder="https://example.com"
            />
          </div>

          <!-- 最大深度 -->
          <div class="form-control">
            <label class="label"><span class="label-text">最大探索深度</span></label>
            <input 
              v-model.number="veMaxDepth"
              type="number"
              min="1"
              max="10"
              class="input input-bordered w-24"
            />
          </div>

          <!-- 测试结果/状态 -->
          <div v-if="veStatus" class="form-control">
            <label class="label"><span class="label-text">探索状态</span></label>
            <div class="bg-base-200 p-4 rounded-lg">
              <div class="flex items-center gap-2 mb-2">
                <span :class="veStatus.is_running ? 'badge badge-info' : 'badge badge-success'">
                  {{ veStatus.is_running ? '运行中' : '已完成' }}
                </span>
                <span class="text-sm">{{ veStatus.session_id }}</span>
              </div>
              <p class="text-sm">目标: {{ veStatus.target_url }}</p>
            </div>
          </div>

          <div v-if="veResult" class="form-control">
            <label class="label"><span class="label-text">执行结果</span></label>
            <pre class="textarea textarea-bordered font-mono text-xs whitespace-pre-wrap h-40 bg-base-200 overflow-auto">{{ veResult }}</pre>
          </div>
        </div>

        <div class="modal-action">
          <button @click="closeVisionExplorerModal" class="btn">关闭</button>
          <button 
            v-if="veStatus?.is_running"
            @click="stopVisionExplorer"
            class="btn btn-warning"
          >
            <i class="fas fa-stop mr-1"></i>
            停止
          </button>
          <button 
            @click="startVisionExplorer"
            class="btn btn-primary"
            :disabled="isVeTesting || !veTargetUrl"
          >
            <i v-if="isVeTesting" class="fas fa-spinner fa-spin mr-1"></i>
            <i v-else class="fas fa-play mr-1"></i>
            开始探索
          </button>
        </div>
      </div>
    </dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, nextTick } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { dialog } from '@/composables/useDialog'
import ShellConfigModal from './ShellConfigModal.vue'
import ShellTerminal from './ShellTerminal.vue'
import UnifiedToolTest from './UnifiedToolTest.vue'

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
  { key: 'browser', label: '浏览器', icon: 'fas fa-globe', btnClass: 'btn-primary', badgeClass: 'badge-primary', bgClass: 'bg-primary/10', textClass: 'text-primary' },
  { key: 'utility', label: '工具', icon: 'fas fa-tools', btnClass: 'btn-success', badgeClass: 'badge-success', bgClass: 'bg-success/10', textClass: 'text-success' },
]

const getCategoryConfig = (category: string): CategoryConfig => {
  return categoryConfigs.find(c => c.key === category.toLowerCase()) || {
    key: category,
    label: category,
    icon: 'fas fa-tools',
    btnClass: 'btn-ghost',
    badgeClass: 'badge-ghost',
    bgClass: 'bg-base-200',
    textClass: 'text-base-content'
  }
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

// 计算属性：可用的分类
const categories = computed(() => {
  const cats = new Set(tools.value.map(t => t.category?.toLowerCase() || 'utility'))
  return categoryConfigs.filter(c => cats.has(c.key))
})

// 计算属性：按分类分组的工具
const groupedTools = computed(() => {
  let filteredTools = tools.value
  
  // 如果选择了分类，只显示该分类
  if (selectedCategory.value) {
    filteredTools = tools.value.filter(t => 
      (t.category?.toLowerCase() || 'utility') === selectedCategory.value
    )
  }
  
  // 按分类分组
  const groups: { category: string; tools: any[] }[] = []
  const categoryOrder = categoryConfigs.map(c => c.key)
  
  for (const cat of categoryOrder) {
    const categoryTools = filteredTools.filter(t => 
      (t.category?.toLowerCase() || 'utility') === cat
    )
    if (categoryTools.length > 0) {
      groups.push({ category: cat, tools: categoryTools })
    }
  }
  
  // 添加未分类的工具
  const knownCategories = new Set(categoryOrder)
  const otherTools = filteredTools.filter(t => 
    !knownCategories.has(t.category?.toLowerCase() || 'utility')
  )
  if (otherTools.length > 0) {
    groups.push({ category: 'other', tools: otherTools })
  }
  
  return groups
})

// 获取分类工具数量
const getToolCountByCategory = (category: string) => {
  return tools.value.filter(t => 
    (t.category?.toLowerCase() || 'utility') === category
  ).length
}

// Vision Explorer V2 状态
const showVisionExplorerModal = ref(false)
const veTargetUrl = ref('https://example.com')
const veMaxDepth = ref(5)
const veStatus = ref<any>(null)
const veResult = ref('')
const isVeTesting = ref(false)
const veExecutionId = ref('')

// methods

// methods
function getToolIcon(toolName: string) {
  const iconMap: Record<string, string> = {
    'subdomain_scanner': 'fas fa-sitemap',
    'subdomain_brute': 'fas fa-sitemap',
    'port_scanner': 'fas fa-network-wired',
    'port_scan': 'fas fa-network-wired',
    'shell': 'fas fa-terminal',
    'interactive_shell': 'fas fa-terminal',
    'vision_explorer': 'fas fa-eye',
    'vision_explorer_v2': 'fas fa-eye',
    'web_search': 'fas fa-search',
    'http_request': 'fas fa-globe',
    'local_time': 'fas fa-clock',
    'memory_manager': 'fas fa-memory',
    'ocr': 'fas fa-file-image',
    'tenth_man_review': 'fas fa-user-secret',
    'todos': 'fas fa-tasks',
    // Browser tools
    'browser_open': 'fas fa-external-link-alt',
    'browser_snapshot': 'fas fa-camera',
    'browser_click': 'fas fa-mouse-pointer',
    'browser_fill': 'fas fa-keyboard',
    'browser_type': 'fas fa-i-cursor',
    'browser_select': 'fas fa-list',
    'browser_scroll': 'fas fa-arrows-alt-v',
    'browser_wait': 'fas fa-hourglass-half',
    'browser_get_text': 'fas fa-font',
    'browser_screenshot': 'fas fa-camera-retro',
    'browser_back': 'fas fa-arrow-left',
    'browser_press': 'fas fa-keyboard',
    'browser_hover': 'fas fa-hand-pointer',
    'browser_evaluate': 'fas fa-code',
    'browser_get_url': 'fas fa-link',
    'browser_close': 'fas fa-times-circle',
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


// ============================================================================
// Vision Explorer V2 方法
// ============================================================================

function openVisionExplorerModal(tool: any) {
  veStatus.value = null
  veResult.value = ''
  showVisionExplorerModal.value = true
}

function closeVisionExplorerModal() {
  showVisionExplorerModal.value = false
}



async function startVisionExplorer() {
  if (!veTargetUrl.value) {
    dialog.toast.warning('请输入目标 URL')
    return
  }

  isVeTesting.value = true
  veResult.value = '正在启动 Vision Explorer V2...'

  try {
    const executionId = await invoke<string>('start_vision_explorer_v2', {
      config: {
        target_url: veTargetUrl.value,
        max_depth: veMaxDepth.value,
        ai_config: {}
      }
    })

    veExecutionId.value = executionId
    dialog.toast.success(`探索已启动，执行ID: ${executionId.substring(0, 8)}...`)
    
    // Poll status periodically
    await pollVeStatus(executionId)
  } catch (error: any) {
    console.error('Failed to start Vision Explorer V2:', error)
    veResult.value = `启动失败: ${error?.message || error}`
    dialog.toast.error('启动 Vision Explorer V2 失败')
  } finally {
    isVeTesting.value = false
  }
}

async function stopVisionExplorer() {
  if (!veExecutionId.value) return

  try {
    await invoke('stop_vision_explorer_v2', {
      executionId: veExecutionId.value
    })
    dialog.toast.success('已发送停止请求')
    veResult.value = '探索已停止'
    
    // Refresh status
    await pollVeStatus(veExecutionId.value)
  } catch (error: any) {
    console.error('Failed to stop Vision Explorer V2:', error)
    dialog.toast.error(`停止失败: ${error?.message || error}`)
  }
}

async function pollVeStatus(executionId: string) {
  try {
    const status = await invoke<any>('get_vision_explorer_v2_status', { executionId })
    veStatus.value = status
    
    if (status.is_running) {
      veResult.value = `探索进行中...\n会话ID: ${status.session_id}\n目标: ${status.target_url}`
      // Continue polling
      setTimeout(() => pollVeStatus(executionId), 2000)
    } else {
      veResult.value = `探索已完成\n会话ID: ${status.session_id}\n目标: ${status.target_url}`
    }
  } catch (error: any) {
    console.warn('Failed to get V2 status:', error)
    // Session may have ended or not found
    veResult.value = '会话已结束或未找到'
    veStatus.value = null
  }
}

// 暴露刷新方法供父组件调用
defineExpose({ refresh })

onMounted(() => {
  fetchTools()
})
</script>
