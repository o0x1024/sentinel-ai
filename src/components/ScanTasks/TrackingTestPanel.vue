<template>
  <div class="card bg-base-100 shadow-xl">
    <div class="card-body">
      <h2 class="card-title">
        <i class="fas fa-flask mr-2"></i>
        工具追踪测试面板
      </h2>
      
      <div class="alert alert-info">
        <i class="fas fa-info-circle"></i>
        <span>此面板用于测试工具执行追踪功能。选择一个任务，或使用临时测试任务ID。</span>
      </div>

      <!-- Task Selection -->
      <div class="form-control">
        <label class="label">
          <span class="label-text">选择任务</span>
        </label>
        <select v-model="selectedTaskId" class="select select-bordered">
          <option value="">-- 选择一个任务 --</option>
          <option value="test-task-temp">🧪 临时测试任务（无需创建真实任务）</option>
          <option v-for="task in tasks" :key="task.id" :value="task.id">
            {{ task.name }} ({{ task.id }})
          </option>
        </select>
      </div>

      <!-- Test Buttons -->
      <div class="grid grid-cols-1 md:grid-cols-2 gap-4 mt-4">
        <!-- Plugin Test -->
        <div class="card bg-base-200">
          <div class="card-body">
            <h3 class="card-title text-sm">插件追踪测试</h3>
            <div class="form-control">
              <input v-model="pluginId" type="text" placeholder="插件ID" class="input input-sm input-bordered" />
            </div>
            <div class="form-control mt-2">
              <input v-model="pluginName" type="text" placeholder="插件名称" class="input input-sm input-bordered" />
            </div>
            <button 
              class="btn btn-sm btn-primary mt-2" 
              :disabled="!selectedTaskId || testing"
              @click="testPlugin"
            >
              <span v-if="testing" class="loading loading-spinner loading-xs"></span>
              <i v-else class="fas fa-play mr-1"></i>
              测试插件追踪
            </button>
          </div>
        </div>

        <!-- MCP Test -->
        <div class="card bg-base-200">
          <div class="card-body">
            <h3 class="card-title text-sm">MCP工具追踪测试</h3>
            <div class="form-control">
              <input v-model="mcpConnectionId" type="text" placeholder="连接ID" class="input input-sm input-bordered" />
            </div>
            <div class="form-control mt-2">
              <input v-model="mcpToolName" type="text" placeholder="工具名称" class="input input-sm input-bordered" />
            </div>
            <button 
              class="btn btn-sm btn-secondary mt-2" 
              :disabled="!selectedTaskId || testing"
              @click="testMcp"
            >
              <span v-if="testing" class="loading loading-spinner loading-xs"></span>
              <i v-else class="fas fa-play mr-1"></i>
              测试MCP追踪
            </button>
          </div>
        </div>

        <!-- Builtin Test -->
        <div class="card bg-base-200">
          <div class="card-body">
            <h3 class="card-title text-sm">内置工具追踪测试</h3>
            <div class="form-control">
              <select v-model="builtinToolName" class="select select-sm select-bordered">
                <option value="port_scan">端口扫描</option>
                <option value="subdomain_brute">子域名爆破</option>
                <option value="http_request">HTTP请求</option>
                <option value="local_time">本地时间</option>
              </select>
            </div>
            <button 
              class="btn btn-sm btn-accent mt-2" 
              :disabled="!selectedTaskId || testing"
              @click="testBuiltin"
            >
              <span v-if="testing" class="loading loading-spinner loading-xs"></span>
              <i v-else class="fas fa-play mr-1"></i>
              测试内置工具追踪
            </button>
          </div>
        </div>

        <!-- Error Test -->
        <div class="card bg-base-200">
          <div class="card-body">
            <h3 class="card-title text-sm">错误追踪测试</h3>
            <div class="form-control">
              <input v-model="errorToolId" type="text" placeholder="工具ID" class="input input-sm input-bordered" />
            </div>
            <div class="form-control mt-2">
              <input v-model="errorToolName" type="text" placeholder="工具名称" class="input input-sm input-bordered" />
            </div>
            <button 
              class="btn btn-sm btn-error mt-2" 
              :disabled="!selectedTaskId || testing"
              @click="testError"
            >
              <span v-if="testing" class="loading loading-spinner loading-xs"></span>
              <i v-else class="fas fa-play mr-1"></i>
              测试错误追踪
            </button>
          </div>
        </div>
      </div>

      <!-- Results -->
      <div v-if="testResults.length > 0" class="mt-4">
        <h3 class="font-bold mb-2">测试结果：</h3>
        <div class="space-y-2">
          <div v-for="(result, index) in testResults" :key="index" 
            class="alert" 
            :class="result.success ? 'alert-success' : 'alert-error'"
          >
            <i :class="result.success ? 'fas fa-check-circle' : 'fas fa-times-circle'"></i>
            <div class="flex-1">
              <div class="font-bold">{{ result.type }}</div>
              <div class="text-sm">{{ result.message }}</div>
              <div v-if="result.logId" class="text-xs font-mono mt-1">Log ID: {{ result.logId }}</div>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'

interface ScanTask {
  id: string
  name: string
  status: string
}

interface TestResult {
  type: string
  success: boolean
  message: string
  logId?: string
}

const tasks = ref<ScanTask[]>([])
const selectedTaskId = ref('')
const testing = ref(false)
const testResults = ref<TestResult[]>([])

// Plugin test
const pluginId = ref('sql_injection_detector')
const pluginName = ref('SQL注入检测器')

// MCP test
const mcpConnectionId = ref('test-server')
const mcpToolName = ref('test_tool')

// Builtin test
const builtinToolName = ref('port_scan')

// Error test
const errorToolId = ref('error_plugin')
const errorToolName = ref('错误测试插件')

onMounted(async () => {
  await loadTasks()
})

const loadTasks = async () => {
  try {
    const response = await invoke<{ success: boolean; data?: ScanTask[] }>('get_scan_tasks', {
      projectId: null
    })
    if (response.success && response.data) {
      tasks.value = response.data
    }
  } catch (error) {
    console.error('Failed to load tasks:', error)
  }
}

const testPlugin = async () => {
  if (!selectedTaskId.value) return
  
  testing.value = true
  try {
    const logId = await invoke<string>('test_plugin_tracking', {
      taskId: selectedTaskId.value,
      pluginId: pluginId.value,
      pluginName: pluginName.value
    })
    
    testResults.value.unshift({
      type: '插件追踪',
      success: true,
      message: `成功追踪插件执行: ${pluginName.value}`,
      logId
    })
  } catch (error) {
    testResults.value.unshift({
      type: '插件追踪',
      success: false,
      message: `失败: ${error}`
    })
  } finally {
    testing.value = false
  }
}

const testMcp = async () => {
  if (!selectedTaskId.value) return
  
  testing.value = true
  try {
    const logId = await invoke<string>('test_mcp_tracking', {
      taskId: selectedTaskId.value,
      connectionId: mcpConnectionId.value,
      toolName: mcpToolName.value
    })
    
    testResults.value.unshift({
      type: 'MCP工具追踪',
      success: true,
      message: `成功追踪MCP工具执行: ${mcpToolName.value}`,
      logId
    })
  } catch (error) {
    testResults.value.unshift({
      type: 'MCP工具追踪',
      success: false,
      message: `失败: ${error}`
    })
  } finally {
    testing.value = false
  }
}

const testBuiltin = async () => {
  if (!selectedTaskId.value) return
  
  testing.value = true
  try {
    const logId = await invoke<string>('test_builtin_tracking', {
      taskId: selectedTaskId.value,
      toolName: builtinToolName.value
    })
    
    testResults.value.unshift({
      type: '内置工具追踪',
      success: true,
      message: `成功追踪内置工具执行: ${builtinToolName.value}`,
      logId
    })
  } catch (error) {
    testResults.value.unshift({
      type: '内置工具追踪',
      success: false,
      message: `失败: ${error}`
    })
  } finally {
    testing.value = false
  }
}

const testError = async () => {
  if (!selectedTaskId.value) return
  
  testing.value = true
  try {
    const logId = await invoke<string>('test_error_tracking', {
      taskId: selectedTaskId.value,
      toolId: errorToolId.value,
      toolName: errorToolName.value
    })
    
    testResults.value.unshift({
      type: '错误追踪',
      success: true,
      message: `成功追踪工具错误: ${errorToolName.value}`,
      logId
    })
  } catch (error) {
    testResults.value.unshift({
      type: '错误追踪',
      success: false,
      message: `失败: ${error}`
    })
  } finally {
    testing.value = false
  }
}
</script>
