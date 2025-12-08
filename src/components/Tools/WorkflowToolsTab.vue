<template>
  <div class="space-y-4">
    <div class="flex justify-between items-center">
      <div class="alert alert-info flex-1 mr-4">
        <i class="fas fa-info-circle"></i>
        <span>这些是在工作流工作室中标记为工具的工作流，可供AI助手调用执行。</span>
      </div>
      <div class="flex gap-2">
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
                @click="viewInStudio(workflow)"
                class="btn btn-outline btn-secondary btn-sm"
                title="在工作流工作室中查看"
              >
                <i class="fas fa-external-link-alt mr-1"></i>
                查看
              </button>
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
                  @click="viewInStudio(workflow)"
                  class="btn btn-outline btn-secondary btn-xs"
                  title="在工作流工作室中查看"
                >
                  <i class="fas fa-external-link-alt"></i>
                </button>
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
      <p class="text-base-content/70 mt-2">在工作流工作室中创建工作流并将其设置为工具</p>
      <button @click="goToWorkflowStudio" class="btn btn-primary mt-4">
        <i class="fas fa-plus mr-2"></i>
        前往工作流工作室
      </button>
    </div>

    <!-- 测试模态框 -->
    <dialog :class="['modal', { 'modal-open': showTestModal }]">
      <div v-if="testingWorkflow" class="modal-box w-11/12 max-w-3xl">
        <div class="flex justify-between items-center mb-4">
          <h3 class="font-bold text-lg">
            测试工作流工具: {{ testingWorkflow.name }}
          </h3>
          <button @click="closeTestModal" class="btn btn-sm btn-ghost">✕</button>
        </div>

        <div class="space-y-4 max-h-[70vh] overflow-y-auto pr-2">
          <div class="alert alert-info">
            <i class="fas fa-info-circle"></i>
            <span>输入测试参数后点击运行测试，验证工作流是否正常执行。</span>
          </div>

          <!-- 工作流描述 -->
          <div class="bg-base-200 p-4 rounded-lg">
            <p class="text-sm">{{ testingWorkflow.description || '暂无描述' }}</p>
            <div class="flex gap-2 mt-2">
              <span class="badge badge-secondary badge-sm">v{{ testingWorkflow.version }}</span>
              <span class="text-xs text-base-content/60">更新于 {{ formatDate(testingWorkflow.updated_at) }}</span>
            </div>
          </div>

          <!-- 测试参数输入 -->
          <div class="form-control">
            <label class="label">
              <span class="label-text">测试参数 (JSON, 可选)</span>
              <span class="label-text-alt text-xs opacity-60">参数已从 Start 节点自动提取</span>
            </label>
            <textarea
              v-model="testParamsJson"
              class="textarea textarea-bordered font-mono text-sm"
              placeholder='输入 JSON 格式的测试参数'
              rows="8"
              spellcheck="false"
            ></textarea>
          </div>

          <!-- 测试结果 -->
          <div class="form-control">
            <label class="label"><span class="label-text">测试结果</span></label>
            <pre class="textarea textarea-bordered font-mono text-xs whitespace-pre-wrap h-40 bg-base-200 overflow-auto">{{ testResult || '点击"运行测试"查看结果' }}</pre>
          </div>
        </div>

        <div class="modal-action">
          <button @click="closeTestModal" class="btn">取消</button>
          <button 
            class="btn btn-primary"
            :disabled="isTesting"
            @click="runTest"
          >
            <i v-if="isTesting" class="fas fa-spinner fa-spin mr-1"></i>
            <i v-else class="fas fa-play mr-1"></i>
            运行测试
          </button>
        </div>
      </div>
    </dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, nextTick } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { dialog } from '@/composables/useDialog'

// 状态
const workflows = ref<any[]>([])
const isLoading = ref(false)
const viewMode = ref('list')
const showTestModal = ref(false)
const testingWorkflow = ref<any>(null)
const testParamsJson = ref('')
const testResult = ref('')
const isTesting = ref(false)

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

function viewInStudio(workflow: any) {
  window.location.hash = `#/workflow-studio?id=${workflow.id}`
}

async function openTestModal(workflow: any) {
  testingWorkflow.value = { ...workflow }
  testResult.value = ''
  
  // 加载完整的工作流定义以获取输入参数
  try {
    const fullDef = await invoke<any>('get_workflow_definition', { id: workflow.id })
    if (fullDef && fullDef.graph) {
      const defaultParams = extractInputParams(fullDef.graph)
      testParamsJson.value = JSON.stringify(defaultParams, null, 2)
    } else {
      testParamsJson.value = '{}'
    }
  } catch (e) {
    console.error('Failed to load workflow definition:', e)
    testParamsJson.value = '{}'
  }
  
  nextTick(() => {
    showTestModal.value = true
  })
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

function closeTestModal() {
  showTestModal.value = false
  setTimeout(() => {
    testingWorkflow.value = null
    testParamsJson.value = ''
    testResult.value = ''
  }, 350)
}

async function runTest() {
  if (!testingWorkflow.value) {
    dialog.toast.error('请选择要测试的工作流')
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
  testResult.value = '正在执行工作流测试...'
  
  try {
    const result = await invoke<any>('unified_execute_tool', {
      toolName: `workflow::${testingWorkflow.value.id}`,
      inputs,
      context: null,
      timeout: 120,
    })

    if (result.success) {
      testResult.value = typeof result.output === 'string'
        ? result.output
        : JSON.stringify(result.output, null, 2)
      dialog.toast.success('工作流测试完成')
    } else {
      testResult.value = `测试失败: ${result.error || '未知错误'}`
      dialog.toast.error('工作流测试失败')
    }
  } catch (error: any) {
    console.error('Failed to test workflow tool:', error)
    testResult.value = `测试失败: ${error?.message || String(error)}`
    dialog.toast.error('工作流测试失败')
  } finally {
    isTesting.value = false
  }
}

// 暴露刷新方法供父组件调用
defineExpose({ refresh })

onMounted(() => {
  fetchWorkflows()
})
</script>

