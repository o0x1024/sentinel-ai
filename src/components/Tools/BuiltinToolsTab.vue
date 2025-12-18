<template>
  <div class="space-y-4">
    <div class="flex justify-between items-center">
      <div class="alert alert-info flex-1 mr-4">
        <i class="fas fa-info-circle"></i>
        <span>这些是系统内置的MCP工具，已自动注册并可供AI助手调用。</span>
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
    
    <div v-if="isLoading" class="text-center p-8">
      <i class="fas fa-spinner fa-spin text-2xl"></i>
      <p class="mt-2">正在加载内置工具...</p>
    </div>
    
    <!-- 卡片视图 -->
    <div v-else-if="tools.length > 0 && viewMode === 'card'" class="grid grid-cols-1 lg:grid-cols-2 xl:grid-cols-3 gap-4">
      <div 
        v-for="tool in tools" 
        :key="tool.id"
        class="card bg-base-100 shadow-lg hover:shadow-xl transition-shadow"
      >
        <div class="card-body">
          <div class="flex items-center gap-3">
            <div class="avatar">
              <div class="w-12 h-12 rounded-lg bg-success/10 flex items-center justify-center">
                <i :class="getToolIcon(tool.name)" class="text-success text-xl"></i>
              </div>
            </div>
            <div class="flex-1">
              <h3 class="card-title text-lg">{{ tool.name }}</h3>
              <span class="badge badge-success badge-sm">{{ tool.category }}</span>
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
              <button 
                v-if="tool.name === 'shell'"
                @click="showShellConfigModal = true"
                class="btn btn-warning btn-sm"
                title="安全配置"
              >
                <i class="fas fa-shield-alt"></i>
              </button>
              <button 
                @click="quickTest(tool)"
                class="btn btn-success btn-sm"
                :disabled="tool.is_testing"
                title="快速测试（使用默认参数）"
              >
                <i v-if="tool.is_testing" class="fas fa-spinner fa-spin mr-1"></i>
                <i v-else class="fas fa-play mr-1"></i>
                测试
              </button>
              <button 
                @click="openTestModal(tool)"
                class="btn btn-outline btn-info btn-sm"
                title="高级测试（自定义参数）"
              >
                <i class="fas fa-cog"></i>
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>
    
    <!-- 列表视图 -->
    <div v-else-if="tools.length > 0 && viewMode === 'list'" class="overflow-x-auto">
      <table class="table w-full">
        <thead>
          <tr>
            <th class="w-1/12">启用</th>
            <th>名称</th>
            <th>分类</th>
            <th>描述</th>
            <th>版本</th>
            <th>操作</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="tool in tools" :key="tool.id">
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
                <i :class="getToolIcon(tool.name)" class="text-success"></i>
                <span class="font-semibold">{{ tool.name }}</span>
              </div>
            </td>
            <td><span class="badge badge-success badge-sm">{{ tool.category }}</span></td>
            <td class="text-sm">{{ tool.description }}</td>
            <td class="text-xs text-base-content/60">v{{ tool.version }}</td>
            <td>
              <div class="flex gap-1">
                <button 
                  v-if="tool.name === 'shell'"
                  @click="showShellConfigModal = true"
                  class="btn btn-warning btn-xs"
                  title="安全配置"
                >
                  <i class="fas fa-shield-alt"></i>
                </button>
                <button 
                  @click="quickTest(tool)"
                  class="btn btn-success btn-xs"
                  :disabled="tool.is_testing"
                  title="快速测试（使用默认参数）"
                >
                  <i v-if="tool.is_testing" class="fas fa-spinner fa-spin mr-1"></i>
                  <i v-else class="fas fa-play"></i>
                </button>
                <button 
                  @click="openTestModal(tool)"
                  class="btn btn-outline btn-info btn-xs"
                  title="高级测试（自定义参数）"
                >
                  <i class="fas fa-cog"></i>
                </button>
              </div>
            </td>
          </tr>
        </tbody>
      </table>
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

    <!-- 测试模态框 -->
    <dialog :class="['modal', { 'modal-open': showTestModal }]">
      <div v-if="testingTool" class="modal-box w-11/12 max-w-5xl">
        <div class="flex justify-between items-center mb-4">
          <h3 class="font-bold text-lg">
            测试内置工具: {{ testingTool.name }}
          </h3>
          <button @click="closeTestModal" class="btn btn-sm btn-ghost">✕</button>
        </div>

        <div class="space-y-4 max-h-[70vh] overflow-y-auto pr-2">
          <div class="alert alert-info">
            <i class="fas fa-info-circle"></i>
            <span>输入测试参数后点击运行测试，可以验证工具是否正常工作。</span>
          </div>

          <!-- 工具描述 -->
          <div class="bg-base-200 p-4 rounded-lg">
            <p class="text-sm">{{ testingTool.description }}</p>
            <div class="flex gap-2 mt-2">
              <span class="badge badge-success badge-sm">{{ testingTool.category }}</span>
              <span class="badge badge-ghost badge-sm">v{{ testingTool.version }}</span>
            </div>
          </div>

          <!-- 参数说明 -->
          <div v-if="testingTool.input_schema && testingTool.input_schema.properties && Object.keys(testingTool.input_schema.properties).length > 0" class="collapse collapse-arrow border border-base-300 bg-base-100">
            <input type="checkbox" checked />
            <div class="collapse-title text-md font-medium">
              输入参数说明
            </div>
            <div class="collapse-content">
              <div class="overflow-x-auto">
                <table class="table table-sm w-full">
                  <thead>
                    <tr>
                      <th>参数名</th>
                      <th>类型</th>
                      <th>必填</th>
                      <th>描述</th>
                      <th>约束</th>
                    </tr>
                  </thead>
                  <tbody>
                    <tr v-for="prop in getToolProperties(testingTool.input_schema)" :key="prop.name">
                      <td class="font-mono text-primary">{{ prop.name }}</td>
                      <td><span class="badge badge-outline">{{ prop.type }}</span></td>
                      <td>
                        <span v-if="prop.required" class="badge badge-error badge-sm">必填</span>
                      </td>
                      <td>{{ prop.description }}</td>
                      <td class="font-mono text-xs">{{ prop.constraints }}</td>
                    </tr>
                  </tbody>
                </table>
              </div>
            </div>
          </div>

          <!-- 无参数工具提示 -->
          <div v-else class="alert alert-warning">
            <i class="fas fa-exclamation-triangle"></i>
            <span>此工具没有输入参数或参数信息暂未提供，可直接运行测试。</span>
          </div>

          <!-- 测试参数输入 -->
          <div class="form-control">
            <label class="label"><span class="label-text">测试参数 (JSON)</span></label>
            <textarea
              v-model="testParamsJson"
              class="textarea textarea-bordered font-mono text-sm"
              placeholder="输入 JSON 格式的测试参数，例如: {}"
              rows="6"
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

    <!-- Shell 配置模态框 -->
    <ShellConfigModal v-model="showShellConfigModal" />
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, nextTick } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { dialog } from '@/composables/useDialog'
import ShellConfigModal from './ShellConfigModal.vue'

// 状态
const tools = ref<any[]>([])
const isLoading = ref(false)
const viewMode = ref('list')
const showTestModal = ref(false)
const showShellConfigModal = ref(false)
const testingTool = ref<any>(null)
const testParamsJson = ref('')
const testResult = ref('')
const isTesting = ref(false)

// 方法
function getToolIcon(toolName: string) {
  switch (toolName) {
    case 'subdomain_scanner': return 'fas fa-sitemap'
    case 'port_scanner': return 'fas fa-network-wired'
    case 'shell': return 'fas fa-terminal'
    default: return 'fas fa-tools'
  }
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

async function quickTest(tool: any) {
  tool.is_testing = true
  try {
    const defaultParams = tool.input_schema ? JSON.parse(generateDefaultParams(tool.input_schema)) : {}
    const result = await invoke<any>('unified_execute_tool', {
      toolName: tool.name,
      inputs: defaultParams,
      context: null,
      timeout: 60,
    })
    if (result.success) {
      dialog.toast.success(`工具 ${tool.name} 测试成功`)
    } else {
      dialog.toast.error(`工具 ${tool.name} 测试失败：${result.error || '未知错误'}`)
    }
  } catch (error: any) {
    console.error(`Failed to test tool ${tool.name}:`, error)
    dialog.toast.error(`工具 ${tool.name} 测试失败：${error?.message || error}`)
  } finally {
    tool.is_testing = false
  }
}

function openTestModal(tool: any) {
  testingTool.value = { ...tool }
  testResult.value = ''
  testParamsJson.value = tool.input_schema ? generateDefaultParams(tool.input_schema) : '{}'
  nextTick(() => {
    showTestModal.value = true
  })
}

function closeTestModal() {
  showTestModal.value = false
  setTimeout(() => {
    testingTool.value = null
    testParamsJson.value = ''
    testResult.value = ''
  }, 350)
}

async function runTest() {
  if (!testingTool.value) {
    dialog.toast.error('请选择要测试的工具')
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
      toolName: testingTool.value.name,
      inputs,
      context: null,
      timeout: 60,
    })

    if (result.success) {
      testResult.value = typeof result.output === 'string'
        ? result.output
        : JSON.stringify(result.output, null, 2)
      dialog.toast.success('工具测试完成')
    } else {
      testResult.value = `测试失败: ${result.error || '未知错误'}`
      dialog.toast.error('工具测试失败')
    }
  } catch (error: any) {
    console.error('Failed to test builtin tool:', error)
    testResult.value = `测试失败: ${error?.message || String(error)}`
    dialog.toast.error('工具测试失败')
  } finally {
    isTesting.value = false
  }
}

// 暴露刷新方法供父组件调用
defineExpose({ refresh })

onMounted(() => {
  fetchTools()
})
</script>
