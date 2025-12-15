<template>
    <!-- 网络(代理)设置 -->
    <div  class="card bg-base-100 shadow-md mb-6">
        <div class="card-body gap-4">
            <div class="flex items-center gap-3">
                <input type="checkbox" class="toggle toggle-primary" v-model="network.proxy.enabled" :disabled="props.saving"
                    @change="saveProxy" />
                <span class="font-medium">{{ t('settings.network.enableGlobalProxy') }}</span>
            </div>
            <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
                <div>
                    <label class="label">
                        <span class="label-text">{{ t('settings.network.scheme') }}</span>
                    </label>
                    <select v-model="network.proxy.scheme" class="select select-bordered w-full" :disabled="props.saving" @change="saveProxy">
                        <option value="http">{{ t('settings.network.proxySchemes.http') }}</option>
                        <option value="https">{{ t('settings.network.proxySchemes.https') }}</option>
                        <option value="socks5">{{ t('settings.network.proxySchemes.socks5') }}</option>
                        <option value="socks5h">{{ t('settings.network.proxySchemes.socks5h') }}</option>
                    </select>
                    <label class="label">
                        <span class="label-text-alt text-gray-500">
                            {{ t('settings.network.proxySchemeHint') }}
                        </span>
                    </label>
                </div>
                <div>
                    <label class="label"><span class="label-text">{{ t('settings.network.host') }}</span></label>
                    <input v-model.trim="network.proxy.host" class="input input-bordered w-full" :disabled="props.saving" :placeholder="t('settings.network.placeholders.host')"
                        @blur="saveProxy" />
                </div>
                <div>
                    <label class="label"><span class="label-text">{{ t('settings.network.port') }}</span></label>
                    <input v-model.number="network.proxy.port" class="input input-bordered w-full" type="number"
                        :disabled="props.saving" :placeholder="t('settings.network.placeholders.port')" @blur="saveProxy" />
                </div>
                <div>
                    <label class="label"><span class="label-text">{{ t('settings.network.noProxy') }}</span></label>
                    <input v-model.trim="network.proxy.no_proxy" class="input input-bordered w-full"
                        :disabled="props.saving" :placeholder="t('settings.network.placeholders.noProxy')" @blur="saveProxy" />
                </div>
                <div>
                    <label class="label"><span class="label-text">{{ t('settings.network.username') }}</span></label>
                    <input v-model.trim="network.proxy.username" class="input input-bordered w-full"
                        :disabled="props.saving" @blur="saveProxy" />
                </div>
                <div>
                    <label class="label"><span class="label-text">{{ t('settings.network.password') }}</span></label>
                    <input v-model.trim="network.proxy.password" class="input input-bordered w-full" type="password"
                        :disabled="props.saving" @blur="saveProxy" />
                </div>
            </div>
        </div>
    </div>


     <div class="proxy-test-panel">
    <div class="card bg-base-100 shadow-md">
      <div class="card-body">
        <h2 class="card-title flex items-center gap-2">
          <i class="fas fa-network-wired text-primary"></i>
          {{ t('settings.network.proxyTest.title') }}
        </h2>
        
        <div class="space-y-4">
          <!-- 测试说明 -->
          <div class="alert alert-info">
            <i class="fas fa-info-circle"></i>
            <div>
              <p class="font-semibold">{{ t('settings.network.proxyTest.descriptionTitle') }}</p>
              <p>{{ t('settings.network.proxyTest.description') }}</p>
            </div>
          </div>

          <!-- 当前代理配置显示 -->
          <div class="bg-base-200 p-4 rounded-lg">
            <h3 class="font-semibold text-lg mb-2">{{ t('settings.network.proxyTest.currentConfig') }}</h3>
            <div v-if="currentProxy" class="space-y-2">
              <div class="flex justify-between">
                <span>{{ t('settings.network.proxyTest.statusLabel') }}</span>
                <span :class="currentProxy.enabled ? 'text-success' : 'text-error'">
                  {{ currentProxy.enabled ? t('settings.enabled') : t('settings.disabled') }}
                </span>
              </div>
              <div v-if="currentProxy.enabled" class="space-y-1">
                <div class="flex justify-between">
                  <span>{{ t('settings.network.proxyTest.schemeLabel') }}</span>
                  <span>{{ currentProxy.scheme || t('settings.network.defaults.scheme') }}</span>
                </div>
                <div class="flex justify-between">
                  <span>{{ t('settings.network.proxyTest.hostLabel') }}</span>
                  <span>{{ currentProxy.host || t('settings.network.defaults.notAvailable') }}</span>
                </div>
                <div class="flex justify-between">
                  <span>{{ t('settings.network.proxyTest.portLabel') }}</span>
                  <span>{{ currentProxy.port || t('settings.network.defaults.notAvailable') }}</span>
                </div>
                <div v-if="currentProxy.username" class="flex justify-between">
                  <span>{{ t('settings.network.proxyTest.usernameLabel') }}</span>
                  <span>{{ currentProxy.username }}</span>
                </div>
                <div v-if="currentProxy.no_proxy" class="flex justify-between">
                  <span>{{ t('settings.network.proxyTest.noProxyLabel') }}</span>
                  <span class="text-xs">{{ currentProxy.no_proxy }}</span>
                </div>
              </div>
            </div>
            <div v-else class="text-gray-500">
              {{ t('settings.network.proxyTest.noConfig') }}
            </div>
          </div>

          <!-- 测试按钮组 -->
          <div class="grid grid-cols-1 md:grid-cols-3 gap-4">
            <!-- 动态更新测试 -->
            <div class="card bg-base-200 shadow">
              <div class="card-body p-4">
                <h4 class="font-semibold mb-2">{{ t('settings.network.proxyTest.dynamic.title') }}</h4>
                <p class="text-sm text-gray-600 mb-3">
                  {{ t('settings.network.proxyTest.dynamic.description') }}
                </p>
                <button 
                  class="btn btn-primary btn-sm w-full"
                  :disabled="props.saving || testing.dynamic"
                  @click="testDynamicUpdate"
                >
                  <span v-if="testing.dynamic" class="loading loading-spinner loading-xs"></span>
                  {{ testing.dynamic ? t('settings.network.proxyTest.testing') : t('settings.network.proxyTest.startTest') }}
                </button>
              </div>
            </div>

            <!-- 持久化测试 -->
            <div class="card bg-base-200 shadow">
              <div class="card-body p-4">
                <h4 class="font-semibold mb-2">{{ t('settings.network.proxyTest.persistence.title') }}</h4>
                <p class="text-sm text-gray-600 mb-3">
                  {{ t('settings.network.proxyTest.persistence.description') }}
                </p>
                <button 
                  class="btn btn-secondary btn-sm w-full"
                  :disabled="props.saving || testing.persistence"
                  @click="testPersistence"
                >
                  <span v-if="testing.persistence" class="loading loading-spinner loading-xs"></span>
                  {{ testing.persistence ? t('settings.network.proxyTest.testing') : t('settings.network.proxyTest.startTest') }}
                </button>
              </div>
            </div>

            <!-- 客户端更新测试 -->
            <div class="card bg-base-200 shadow">
              <div class="card-body p-4">
                <h4 class="font-semibold mb-2">{{ t('settings.network.proxyTest.client.title') }}</h4>
                <p class="text-sm text-gray-600 mb-3">
                  {{ t('settings.network.proxyTest.client.description') }}
                </p>
                <button 
                  class="btn btn-accent btn-sm w-full"
                  :disabled="props.saving || testing.client"
                  @click="testClientUpdate"
                >
                  <span v-if="testing.client" class="loading loading-spinner loading-xs"></span>
                  {{ testing.client ? t('settings.network.proxyTest.testing') : t('settings.network.proxyTest.startTest') }}
                </button>
              </div>
            </div>
          </div>

          <!-- 测试结果显示 -->
          <div v-if="testResults.length > 0" class="space-y-3">
            <h3 class="font-semibold text-lg">{{ t('settings.network.proxyTest.results') }}</h3>
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
                    {{ t('settings.network.proxyTest.responseTime', { ms: result.response_time_ms }) }}
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
              {{ t('settings.network.proxyTest.clearResults') }}
            </button>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>


<script setup lang="ts">
import { dialog } from '@/composables/useDialog';
import { invoke } from '@tauri-apps/api/core';
import { onMounted, reactive, ref } from 'vue';
import { useI18n } from 'vue-i18n'


const { t } = useI18n()


const props = defineProps({
  saving: {
    type: Boolean,
    default: false
  }
})


const network = reactive({ proxy: { enabled: false, scheme: 'http', host: '', port: 0, username: '', password: '', no_proxy: '' } })



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
      test_name: t('settings.network.proxyTest.dynamic.title')
    })
    
    // 刷新当前代理配置显示
    await loadCurrentProxy()
  } catch (error) {
    testResults.value.unshift({
      test_name: t('settings.network.proxyTest.dynamic.title'),
      success: false,
      message: t('settings.network.proxyTest.testFailed', { error: String(error) })
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
      test_name: t('settings.network.proxyTest.persistence.title')
    })
  } catch (error) {
    testResults.value.unshift({
      test_name: t('settings.network.proxyTest.persistence.title'),
      success: false,
      message: t('settings.network.proxyTest.testFailed', { error: String(error) })
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
      test_name: t('settings.network.proxyTest.client.title')
    })
  } catch (error) {
    testResults.value.unshift({
      test_name: t('settings.network.proxyTest.client.title'),
      success: false,
      message: t('settings.network.proxyTest.testFailed', { error: String(error) })
    })
  } finally {
    testing.client = false
  }
}

const clearResults = () => {
  testResults.value = []
}


// 网络代理
const loadProxy = async () => {
  try {
    const cfg = await invoke('get_global_proxy_config') as any
    network.proxy.enabled = !!cfg.enabled
    network.proxy.scheme = cfg.scheme || 'http'
    network.proxy.host = cfg.host || ''
    network.proxy.port = cfg.port || 0
    network.proxy.username = cfg.username || ''
    network.proxy.password = cfg.password || ''
    network.proxy.no_proxy = cfg.no_proxy || ''
  } catch (e) {
    console.warn('loadProxy failed', e)
  }
}



// 生命周期
onMounted(() => {
    loadProxy()
  loadCurrentProxy()
})

const saveProxy = async () => {
  try {
    const cfg = {
      enabled: network.proxy.enabled,
      scheme: network.proxy.scheme,
      host: network.proxy.host,
      port: Number(network.proxy.port) || null,
      username: network.proxy.username || null,
      password: network.proxy.password || null,
      no_proxy: network.proxy.no_proxy || null,
    }
    await invoke('set_global_proxy_config', { cfg })
    dialog.toast.success(t('settings.network.toast.proxySaved'))
  } catch (e) {
    dialog.toast.error(t('settings.network.toast.proxySaveFailed'))
  }
}


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
