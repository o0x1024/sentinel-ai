<template>
  <div class="proxy-test-panel">
    <div class="card bg-base-100 shadow-md">
      <div class="card-body">
        <h2 class="card-title flex items-center gap-2">
          <i class="fas fa-network-wired text-primary"></i>
          代理配置动态更新测试
        </h2>
        
        <div class="space-y-4">
          <!-- 测试说明 -->
          <div class="alert alert-info">
            <i class="fas fa-info-circle"></i>
            <div>
              <p class="font-semibold">测试功能说明：</p>
              <p>此测试验证网络设置更改后，三大AI架构能够动态更新代理配置，防止请求出错。</p>
            </div>
          </div>

          <!-- 当前代理配置显示 -->
          <div class="bg-base-200 p-4 rounded-lg">
            <h3 class="font-semibold text-lg mb-2">当前代理配置</h3>
            <div v-if="currentProxy" class="space-y-2">
              <div class="flex justify-between">
                <span>状态:</span>
                <span :class="currentProxy.enabled ? 'text-success' : 'text-error'">
                  {{ currentProxy.enabled ? '启用' : '禁用' }}
                </span>
              </div>
              <div v-if="currentProxy.enabled" class="space-y-1">
                <div class="flex justify-between">
                  <span>协议:</span>
                  <span>{{ currentProxy.scheme || 'http' }}</span>
                </div>
                <div class="flex justify-between">
                  <span>主机:</span>
                  <span>{{ currentProxy.host || 'N/A' }}</span>
                </div>
                <div class="flex justify-between">
                  <span>端口:</span>
                  <span>{{ currentProxy.port || 'N/A' }}</span>
                </div>
                <div v-if="currentProxy.username" class="flex justify-between">
                  <span>用户名:</span>
                  <span>{{ currentProxy.username }}</span>
                </div>
                <div v-if="currentProxy.no_proxy" class="flex justify-between">
                  <span>不代理:</span>
                  <span class="text-xs">{{ currentProxy.no_proxy }}</span>
                </div>
              </div>
            </div>
            <div v-else class="text-gray-500">
              没有代理配置
            </div>
          </div>

          <!-- 测试按钮组 -->
          <div class="grid grid-cols-1 md:grid-cols-3 gap-4">
            <!-- 动态更新测试 -->
            <div class="card bg-base-200 shadow">
              <div class="card-body p-4">
                <h4 class="font-semibold mb-2">动态更新测试</h4>
                <p class="text-sm text-gray-600 mb-3">
                  测试HTTP客户端能否在代理配置变更后自动应用新配置
                </p>
                <button 
                  class="btn btn-primary btn-sm w-full"
                  :disabled="testing.dynamic"
                  @click="testDynamicUpdate"
                >
                  <span v-if="testing.dynamic" class="loading loading-spinner loading-xs"></span>
                  {{ testing.dynamic ? '测试中...' : '开始测试' }}
                </button>
              </div>
            </div>

            <!-- 持久化测试 -->
            <div class="card bg-base-200 shadow">
              <div class="card-body p-4">
                <h4 class="font-semibold mb-2">持久化测试</h4>
                <p class="text-sm text-gray-600 mb-3">
                  测试代理配置能否正确保存到数据库并加载
                </p>
                <button 
                  class="btn btn-secondary btn-sm w-full"
                  :disabled="testing.persistence"
                  @click="testPersistence"
                >
                  <span v-if="testing.persistence" class="loading loading-spinner loading-xs"></span>
                  {{ testing.persistence ? '测试中...' : '开始测试' }}
                </button>
              </div>
            </div>

            <!-- 客户端更新测试 -->
            <div class="card bg-base-200 shadow">
              <div class="card-body p-4">
                <h4 class="font-semibold mb-2">客户端更新测试</h4>
                <p class="text-sm text-gray-600 mb-3">
                  测试HTTP客户端的代理自动更新机制
                </p>
                <button 
                  class="btn btn-accent btn-sm w-full"
                  :disabled="testing.client"
                  @click="testClientUpdate"
                >
                  <span v-if="testing.client" class="loading loading-spinner loading-xs"></span>
                  {{ testing.client ? '测试中...' : '开始测试' }}
                </button>
              </div>
            </div>
          </div>

          <!-- 测试结果显示 -->
          <div v-if="testResults.length > 0" class="space-y-3">
            <h3 class="font-semibold text-lg">测试结果</h3>
            <div v-for="(result, index) in testResults" :key="index" 
                 :class="[
                   'alert',
                   result.success ? 'alert-success' : 'alert-error'
                 ]">
              <div class="flex items-start gap-3">
                <i :class="[
                  'fas',
                  result.success ? 'fa-check-circle' : 'fa-times-circle'
                ]"></i>
                <div class="flex-1">
                  <div class="font-semibold">{{ result.test_name }}</div>
                  <div class="text-sm">{{ result.message }}</div>
                  <div v-if="result.response_time_ms" class="text-xs mt-1">
                    响应时间: {{ result.response_time_ms }}ms
                  </div>
                  <div v-if="result.proxy_config" class="text-xs mt-2 font-mono bg-base-100 p-2 rounded">
                    {{ JSON.stringify(result.proxy_config, null, 2) }}
                  </div>
                </div>
              </div>
            </div>
          </div>

          <!-- 清空结果 -->
          <div v-if="testResults.length > 0" class="flex justify-end">
            <button class="btn btn-ghost btn-sm" @click="clearResults">
              <i class="fas fa-trash mr-2"></i>
              清空结果
            </button>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'

interface ProxyConfig {
  enabled: boolean
  scheme?: string
  host?: string
  port?: number
  username?: string
  password?: string
  no_proxy?: string
}

interface TestResult {
  test_name: string
  success: boolean
  message: string
  proxy_config?: ProxyConfig
  response_time_ms?: number
}

// 响应式数据
const currentProxy = ref<ProxyConfig | null>(null)
const testing = reactive({
  dynamic: false,
  persistence: false,
  client: false
})
const testResults = ref<TestResult[]>([])

// 方法
const loadCurrentProxy = async () => {
  try {
    const proxy = await invoke('get_global_proxy_config') as ProxyConfig
    currentProxy.value = proxy
  } catch (error) {
    console.error('Failed to load current proxy config:', error)
  }
}

const testDynamicUpdate = async () => {
  testing.dynamic = true
  try {
    const result = await invoke('test_proxy_dynamic_update') as TestResult
    testResults.value.unshift({
      ...result,
      test_name: '动态更新测试'
    })
    
    // 刷新当前代理配置显示
    await loadCurrentProxy()
  } catch (error) {
    testResults.value.unshift({
      test_name: '动态更新测试',
      success: false,
      message: `测试失败: ${error}`
    })
  } finally {
    testing.dynamic = false
  }
}

const testPersistence = async () => {
  testing.persistence = true
  try {
    const result = await invoke('test_proxy_persistence') as TestResult
    testResults.value.unshift({
      ...result,
      test_name: '持久化测试'
    })
  } catch (error) {
    testResults.value.unshift({
      test_name: '持久化测试',
      success: false,
      message: `测试失败: ${error}`
    })
  } finally {
    testing.persistence = false
  }
}

const testClientUpdate = async () => {
  testing.client = true
  try {
    const result = await invoke('test_http_client_proxy_update') as TestResult
    testResults.value.unshift({
      ...result,
      test_name: '客户端更新测试'
    })
  } catch (error) {
    testResults.value.unshift({
      test_name: '客户端更新测试',
      success: false,
      message: `测试失败: ${error}`
    })
  } finally {
    testing.client = false
  }
}

const clearResults = () => {
  testResults.value = []
}

// 生命周期
onMounted(() => {
  loadCurrentProxy()
})
</script>

<style scoped>
.proxy-test-panel {
  @apply p-4;
}

.font-mono {
  font-family: 'Courier New', Courier, monospace;
}

.alert {
  @apply p-3 rounded-lg;
}

.alert-info {
  @apply bg-info bg-opacity-20 border border-info text-info-content;
}

.alert-success {
  @apply bg-success bg-opacity-20 border border-success text-success-content;
}

.alert-error {
  @apply bg-error bg-opacity-20 border border-error text-error-content;
}
</style>
