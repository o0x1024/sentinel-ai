<template>
  <div class="space-y-4">
    <div class="flex justify-between items-center">
      <div class="alert alert-info flex-1 mr-4">
        <i class="fas fa-info-circle"></i>
        <span>这些是在工作流工作室中标记为工具的工作流，可供AI助手调用执行。</span>
      </div>
      <div class="flex gap-2">
        <button @click="openAiGenerateInStudio" class="btn btn-outline btn-secondary btn-sm">
          <i class="fas fa-magic mr-1"></i>
          AI生成
        </button>
        <button @click="refresh" class="btn btn-outline btn-sm">
          <i class="fas fa-sync-alt mr-1"></i>
          刷新
        </button>
        <div class="join">
          <button @click="viewMode = 'card'" :class="['join-item', 'btn', 'btn-sm', {'btn-primary': viewMode === 'card'}]">
            <i class="fas fa-th-large"></i>
          </button>
          <button @click="viewMode = 'list'" :class="['join-item', 'btn', 'btn-sm', {'btn-primary': viewMode === 'list'}]">
            <i class="fas fa-list"></i>
          </button>
        </div>
      </div>
    </div>
    
    <div v-if="isLoading" class="text-center p-8">
      <i class="fas fa-spinner fa-spin text-2xl"></i>
      <p class="mt-2">正在加载工作流工具...</p>
    </div>
    
    <!-- 卡片视图 -->
    <div v-else-if="workflows.length > 0 && viewMode === 'card'" class="grid grid-cols-1 lg:grid-cols-2 xl:grid-cols-3 gap-4">
      <div 
        v-for="workflow in workflows" 
        :key="workflow.id"
        class="card bg-base-100 shadow-lg hover:shadow-xl transition-shadow"
      >
        <div class="card-body">
          <div class="flex items-center gap-3">
            <div class="avatar">
              <div class="w-12 h-12 rounded-lg bg-secondary/10 flex items-center justify-center">
                <i class="fas fa-project-diagram text-secondary text-xl"></i>
              </div>
            </div>
            <div class="flex-1">
              <h3 class="card-title text-lg">{{ workflow.name }}</h3>
              <span class="badge badge-secondary badge-sm">v{{ workflow.version }}</span>
            </div>
          </div>

          <p class="text-sm mt-2 h-16 overflow-hidden">{{ workflow.description || '暂无描述' }}</p>

          <div class="flex flex-wrap gap-1 mt-2" v-if="workflow.tags">
            <span 
              v-for="tag in parseTags(workflow.tags)" 
              :key="tag"
              class="badge badge-outline badge-xs"
            >
              {{ tag }}
            </span>
          </div>

          <div class="card-actions justify-between items-center mt-4">
            <span class="text-xs text-base-content/60">{{ formatDate(workflow.updated_at) }}</span>
            <div class="flex gap-2">
                <button 
                @click="openTestModal(workflow)"
                class="btn btn-outline btn-info btn-sm"
                title="测试工作流工具"
              >
                <i class="fas fa-play mr-1"></i>
                测试
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>
    
    <!-- 列表视图 -->
    <div v-else-if="workflows.length > 0 && viewMode === 'list'" class="overflow-x-auto">
      <table class="table w-full">
        <thead>
          <tr>
            <th>名称</th>
            <th>版本</th>
            <th>描述</th>
            <th>标签</th>
            <th>更新时间</th>
            <th>操作</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="workflow in workflows" :key="workflow.id">
            <td>
              <div class="flex items-center gap-2">
                <i class="fas fa-project-diagram text-secondary"></i>
                <span class="font-semibold">{{ workflow.name }}</span>
              </div>
            </td>
            <td><span class="badge badge-secondary badge-sm">v{{ workflow.version }}</span></td>
            <td class="text-sm max-w-xs truncate">{{ workflow.description || '暂无描述' }}</td>
            <td>
              <div class="flex flex-wrap gap-1">
                <span 
                  v-for="tag in parseTags(workflow.tags)" 
                  :key="tag"
                  class="badge badge-outline badge-xs"
                >
                  {{ tag }}
                </span>
              </div>
            </td>
            <td class="text-xs text-base-content/60">{{ formatDate(workflow.updated_at) }}</td>
            <td>
              <div class="flex gap-1">
                <button 
                  @click="openTestModal(workflow)"
                  class="btn btn-outline btn-info btn-xs"
                title="测试工作流工具"
                >
                  <i class="fas fa-play"></i>
                </button>
              </div>
            </td>
          </tr>
        </tbody>
      </table>
    </div>
    
    <div v-else class="text-center p-8">
      <i class="fas fa-project-diagram text-4xl text-base-content/30 mb-4"></i>
      <p class="text-lg font-semibold">暂无工作流工具</p>
      <p class="text-base-content/70 mt-2">在工作流中创建工作流并将其设置为工具</p>
      <button @click="goToWorkflowStudio" class="btn btn-primary mt-4">
        <i class="fas fa-plus mr-2"></i>
        前往工作流
      </button>
    </div>

    <!-- 统一测试组件 -->
    <UnifiedToolTest
      v-model="showTestModal"
      tool-type="workflow"
      :tool-name="testingWorkflow?.name || ''"
      :tool-description="testingWorkflow?.description"
      :tool-version="testingWorkflow?.version"
      :tool-category="'Workflow'"
      :input-schema="testingInputSchema"
      :execution-info="{
        type: 'workflow',
        id: testingWorkflow?.id
      }"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, nextTick } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { dialog } from '@/composables/useDialog'
import UnifiedToolTest from './UnifiedToolTest.vue'

// 状态
const workflows = ref<any[]>([])
const isLoading = ref(false)
const viewMode = ref('list')
const showTestModal = ref(false)
const testingWorkflow = ref<any>(null)
const testingInputSchema = ref<any>({})

// 方法
function parseTags(tags: string | null): string[] {
  if (!tags) return []
  try {
    const parsed = JSON.parse(tags)
    return Array.isArray(parsed) ? parsed : []
  } catch {
    return tags.split(',').map(t => t.trim()).filter(t => t)
  }
}

function formatDate(dateStr: string | null): string {
  if (!dateStr) return '-'
  try {
    const date = new Date(dateStr)
    return date.toLocaleDateString('zh-CN', {
      year: 'numeric',
      month: '2-digit',
      day: '2-digit',
      hour: '2-digit',
      minute: '2-digit'
    })
  } catch {
    return dateStr
  }
}

async function fetchWorkflows() {
  isLoading.value = true
  try {
    const result = await invoke<any[]>('list_workflow_tools')
    workflows.value = result || []
  } catch (error) {
    console.error('Failed to fetch workflow tools:', error)
    workflows.value = []
  } finally {
    isLoading.value = false
  }
}

async function refresh() {
  await fetchWorkflows()
}

function goToWorkflowStudio() {
  window.location.hash = '#/workflow-studio'
}

function openAiGenerateInStudio() {
  localStorage.setItem('open_ai_generate_workflow', '1')
  goToWorkflowStudio()
}

function viewInStudio(workflow: any) {
  window.location.hash = `#/workflow-studio?id=${workflow.id}`
}

async function openTestModal(workflow: any) {
  testingWorkflow.value = { ...workflow }
  testingInputSchema.value = {}
  
  // 加载完整的工作流定义以获取输入参数
  try {
    const fullDef = await invoke<any>('get_workflow_definition', { id: workflow.id })
    if (fullDef && fullDef.graph) {
      const defaultParams = extractInputParams(fullDef.graph)
      // 构造 JSON Schema
      testingInputSchema.value = {
        type: 'object',
        properties: Object.keys(defaultParams).reduce((acc: any, key) => {
          const val = defaultParams[key]
          acc[key] = {
            type: typeof val,
            default: val,
            description: `工作流输入参数: ${key}`
          }
          if (Array.isArray(val)) acc[key].type = 'array'
          return acc
        }, {})
      }
    }
  } catch (e) {
    console.error('Failed to load workflow definition:', e)
  }
  
  showTestModal.value = true
}



// 从工作流图中提取输入参数定义
function extractInputParams(graph: any): Record<string, any> {
  const params: Record<string, any> = {}
  
  if (!graph || !graph.nodes || graph.nodes.length === 0) return params
  
  // 查找入口节点：优先 start 节点，否则使用第一个节点
  let entryNode = graph.nodes.find((n: any) => n.node_type === 'start')
  
  if (!entryNode) {
    // 没有 start 节点，找拓扑排序的第一个节点（没有入边的节点）
    const targetNodes = new Set((graph.edges || []).map((e: any) => e.to_node))
    entryNode = graph.nodes.find((n: any) => !targetNodes.has(n.id))
    
    // 如果所有节点都有入边，使用第一个节点
    if (!entryNode) {
      entryNode = graph.nodes[0]
    }
  }
  
  if (!entryNode) return params
  
  // 优先从节点的 params 提取参数（包含实际配置值）
  if (entryNode.params && typeof entryNode.params === 'object') {
    for (const [key, value] of Object.entries(entryNode.params)) {
      // 直接使用节点配置的参数值
      params[key] = value !== null && value !== undefined ? value : ''
    }
  }
  
  // 如果 params 为空，尝试从 input_ports 提取参数定义
  if (Object.keys(params).length === 0 && entryNode.input_ports && Array.isArray(entryNode.input_ports)) {
    for (const port of entryNode.input_ports) {
      const name = port.name || port.id
      // 跳过通用的 "输入" 端口
      if (name === '输入' || name === 'inputs') continue
      params[name] = getDefaultValueForType(port.port_type)
    }
  }
  
  // 如果还是没有参数，添加一个默认的 input
  if (Object.keys(params).length === 0) {
    params.input = ''
  }
  
  return params
}

// 根据类型返回默认值
function getDefaultValueForType(portType: string | any): any {
  if (typeof portType === 'object') {
    // 可能是嵌套类型如 { Array: "String" }
    if (portType.Array) return []
    if (portType.Object) return {}
    return ''
  }
  
  const typeStr = String(portType).toLowerCase()
  switch (typeStr) {
    case 'string': return ''
    case 'integer':
    case 'int':
    case 'number':
    case 'float':
    case 'double': return 0
    case 'boolean':
    case 'bool': return false
    case 'array': return []
    case 'object':
    case 'json': return {}
    default: return ''
  }
}

// 暴露刷新方法供父组件调用
defineExpose({ refresh })

onMounted(() => {
  fetchWorkflows()
})
</script>
