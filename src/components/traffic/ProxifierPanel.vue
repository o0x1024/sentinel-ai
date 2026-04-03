<template>
  <div class="flex flex-col h-full bg-base-200">
    <!-- 顶部工具栏 -->
    <div class="navbar bg-base-100 min-h-12 px-4 border-b border-base-300">
      <div class="flex-1 flex items-center gap-4">
        <div class="flex items-center gap-2">
          <span class="text-sm font-semibold">Proxifier</span>
          <div class="badge badge-sm" :class="isEnabled ? 'badge-success' : 'badge-ghost'">
            {{ isEnabled ? $t('trafficAnalysis.proxifierPanel.statusRunning') : $t('trafficAnalysis.proxifierPanel.statusStopped') }}
          </div>
        </div>
        <div class="divider divider-horizontal mx-0"></div>
        <button 
          class="btn btn-sm"
          :class="isEnabled ? 'btn-error' : 'btn-success'"
          @click="toggleProxifier"
          :disabled="isToggling"
        >
          <i :class="['fas', isToggling ? 'fa-spinner fa-spin' : (isEnabled ? 'fa-stop' : 'fa-play'), 'mr-1']"></i>
          {{ isEnabled ? $t('trafficAnalysis.proxifierPanel.stop') : $t('trafficAnalysis.proxifierPanel.start') }}
        </button>
      </div>
      <div class="flex-none flex items-center gap-2">
        <!-- 子标签页切换 -->
        <div class="tabs tabs-boxed bg-base-200">
          <button 
            class="tab tab-sm" 
            :class="{ 'tab-active': activeSubTab === 'proxies' }"
            @click="activeSubTab = 'proxies'"
          >
            <i class="fas fa-server mr-1"></i>
            Proxies
          </button>
          <button 
            class="tab tab-sm" 
            :class="{ 'tab-active': activeSubTab === 'rules' }"
            @click="activeSubTab = 'rules'"
          >
            <i class="fas fa-filter mr-1"></i>
            Rules
          </button>
        </div>
      </div>
    </div>

    <!-- 主内容区 -->
    <div class="flex flex-1 min-h-0">
      <!-- 左侧：连接列表 -->
      <div class="flex-1 flex flex-col border-r border-base-300">
        <!-- 连接表格 -->
        <div class="flex-1 overflow-auto">
          <table class="table table-xs table-pin-rows">
            <thead>
              <tr class="bg-base-200">
                <th class="w-32">{{ $t('trafficAnalysis.proxifierPanel.application') }}</th>
                <th class="w-64">{{ $t('trafficAnalysis.proxifierPanel.target') }}</th>
                <th class="w-28">{{ $t('trafficAnalysis.proxifierPanel.timeOrStatus') }}</th>
                <th class="w-40">{{ $t('trafficAnalysis.proxifierPanel.ruleProxy') }}</th>
                <th class="w-20 text-right">{{ $t('trafficAnalysis.proxifierPanel.sent') }}</th>
                <th class="w-20 text-right">{{ $t('trafficAnalysis.proxifierPanel.received') }}</th>
              </tr>
            </thead>
            <tbody>
              <tr v-if="connections.length === 0">
                <td colspan="6" class="text-center text-base-content/50 py-8">
                  <i class="fas fa-plug text-2xl mb-2 block"></i>
                  {{ $t('trafficAnalysis.proxifierPanel.noConnections') }}
                  <p class="text-xs mt-2">{{ $t('trafficAnalysis.proxifierPanel.startProxifierToShow') }}</p>
                </td>
              </tr>
              <tr 
                v-for="conn in connections" 
                :key="conn.id"
                class="hover:bg-base-200/50 cursor-pointer"
                :class="{ 'bg-base-200': selectedConnection === conn.id }"
                @click="selectedConnection = conn.id"
              >
                <td class="font-mono text-xs">
                  <div class="flex items-center gap-1">
                    <i class="fas fa-window-maximize text-base-content/50"></i>
                    {{ conn.application }}
                  </div>
                </td>
                <td class="font-mono text-xs truncate max-w-64" :title="conn.target">
                  {{ conn.target }}
                </td>
                <td class="text-xs">
                  <span :class="getStatusClass(conn.status)">{{ conn.timeOrStatus }}</span>
                </td>
                <td class="text-xs">
                  <span class="text-info">{{ conn.rule }}</span> : 
                  <span class="text-warning">{{ conn.proxy }}</span>
                </td>
                <td class="text-right font-mono text-xs">{{ formatBytes(conn.sent) }}</td>
                <td class="text-right font-mono text-xs">{{ formatBytes(conn.received) }}</td>
              </tr>
            </tbody>
          </table>
        </div>

        <!-- 底部标签栏 -->
        <div class="border-t border-base-300 bg-base-100">
          <div class="tabs tabs-boxed bg-transparent p-1">
            <button 
              class="tab tab-sm" 
              :class="{ 'tab-active': bottomTab === 'connections' }"
              @click="bottomTab = 'connections'"
            >
              Connections
            </button>
            <button 
              class="tab tab-sm" 
              :class="{ 'tab-active': bottomTab === 'traffic' }"
              @click="bottomTab = 'traffic'"
            >
              Traffic
            </button>
            <button 
              class="tab tab-sm" 
              :class="{ 'tab-active': bottomTab === 'statistics' }"
              @click="bottomTab = 'statistics'"
            >
              Statistics
            </button>
          </div>
        </div>

        <!-- 日志区域 -->
        <div class="h-40 overflow-auto bg-base-300/30 border-t border-base-300 p-2">
          <div 
            v-for="(log, index) in logs" 
            :key="index"
            class="text-xs font-mono py-0.5"
          >
            <span class="text-base-content/50">[{{ log.time }}]</span>
            <span :class="getLogClass(log.type)">{{ log.message }}</span>
          </div>
          <div v-if="logs.length === 0" class="text-center text-base-content/50 py-4">
            {{ $t('trafficAnalysis.proxifierPanel.noLogs') }}
          </div>
        </div>
      </div>

      <!-- 右侧：配置面板 -->
      <div class="w-96 flex flex-col bg-base-100">
        <!-- Proxies 子面板 -->
        <ProxifierProxies 
          v-if="activeSubTab === 'proxies'"
          v-model:proxies="proxies"
          @update:proxies="saveProxies"
        />
        
        <!-- Rules 子面板 -->
        <ProxifierRules 
          v-if="activeSubTab === 'rules'"
          v-model:rules="rules"
          :proxies="proxies"
          @update:rules="saveRules"
        />
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen, UnlistenFn } from '@tauri-apps/api/event'
import ProxifierProxies from './ProxifierProxies.vue'
import ProxifierRules from './ProxifierRules.vue'

// 代理请求类型（从 proxy:request 事件）
interface ProxyRequest {
  id: number
  url: string
  host: string
  protocol: string
  method: string
  status_code: number
  response_size: number
  response_time: number
  timestamp: string
}

// Types
interface ProxyServer {
  id: string
  name: string
  host: string
  port: number
  type: 'HTTP' | 'HTTPS' | 'SOCKS5'
  username?: string
  password?: string
  enabled: boolean
}

interface ProxifierRule {
  id: string
  name: string
  enabled: boolean
  applications: string
  targetHosts: string
  targetPorts: string
  action: 'Direct' | string
}

interface Connection {
  id: string
  application: string
  target: string
  timeOrStatus: string
  status: 'open' | 'closed' | 'error'
  rule: string
  proxy: string
  sent: number
  received: number
}

interface LogEntry {
  time: string
  type: 'info' | 'warning' | 'error'
  message: string
}

// State
const isEnabled = ref(false)
const isToggling = ref(false)
const activeSubTab = ref<'proxies' | 'rules'>('proxies')
const bottomTab = ref<'connections' | 'traffic' | 'statistics'>('connections')
const selectedConnection = ref<string | null>(null)

// 代理服务器列表（从数据库加载）
const proxies = ref<ProxyServer[]>([])

// 规则列表（从数据库加载）
const rules = ref<ProxifierRule[]>([])

// 连接列表
const connections = ref<Connection[]>([])

// 日志
const logs = ref<LogEntry[]>([
  { time: formatTime(new Date()), type: 'info', message: 'Welcome to Proxifier v1.0' },
])

// 事件监听器取消函数
let unlistenProxyRequest: UnlistenFn | null = null
let unlistenConnection: UnlistenFn | null = null
let unlistenLog: UnlistenFn | null = null
const connectionIdCounter = 0

// Methods
function formatTime(date: Date): string {
  const month = String(date.getMonth() + 1).padStart(2, '0')
  const day = String(date.getDate()).padStart(2, '0')
  const hours = String(date.getHours()).padStart(2, '0')
  const minutes = String(date.getMinutes()).padStart(2, '0')
  const seconds = String(date.getSeconds()).padStart(2, '0')
  return `${month}.${day} ${hours}:${minutes}:${seconds}`
}

function formatBytes(bytes: number): string {
  if (bytes === 0) return '0 B'
  const k = 1024
  const sizes = ['B', 'KB', 'MB', 'GB']
  const i = Math.floor(Math.log(bytes) / Math.log(k))
  return `${(bytes / Math.pow(k, i)).toFixed(1)} ${sizes[i]}`
}

function getStatusClass(status: string): string {
  switch (status) {
    case 'open': return 'text-success'
    case 'closed': return 'text-base-content/50'
    case 'error': return 'text-error'
    default: return ''
  }
}

function getLogClass(type: string): string {
  switch (type) {
    case 'info': return 'text-info'
    case 'warning': return 'text-warning'
    case 'error': return 'text-error'
    default: return ''
  }
}

function addLog(type: 'info' | 'warning' | 'error', message: string) {
  logs.value.push({
    time: formatTime(new Date()),
    type,
    message
  })
  if (logs.value.length > 100) {
    logs.value.shift()
  }
}

function upsertConnection(conn: Connection) {
  const existing = connections.value.findIndex(c => c.id === conn.id)
  if (existing >= 0) {
    connections.value[existing] = conn
    return
  }

  connections.value.unshift(conn)
  if (connections.value.length > 1000) {
    connections.value.pop()
  }
}

async function toggleProxifier() {
  isToggling.value = true
  try {
    if (isEnabled.value) {
      const result = await invoke<any>('stop_proxifier')
      if (result.success) {
        isEnabled.value = false
        connections.value = []
        selectedConnection.value = null
        addLog('info', 'Proxifier stopped, system proxy cleared')
      } else {
        addLog('error', `Failed to stop: ${result.error}`)
      }
    } else {
      const result = await invoke<any>('start_proxifier', { 
        proxies: proxies.value,
        rules: rules.value
      })
      if (result.success) {
        isEnabled.value = true
        addLog('info', 'Proxifier started, system proxy configured')
      } else {
        addLog('error', `Failed to start: ${result.error}`)
      }
    }
  } catch (error: any) {
    console.error('Failed to toggle proxifier:', error)
    addLog('error', `Failed to toggle: ${error}`)
  } finally {
    isToggling.value = false
  }
}

async function saveProxies(nextProxies?: ProxyServer[]) {
  try {
    if (nextProxies) {
      proxies.value = [...nextProxies]
    }
    // 保存到内存
    await invoke('save_proxifier_proxies', { proxies: proxies.value })
    // 保存到数据库
    await invoke('save_proxifier_proxies_to_db', { proxies: proxies.value })
    addLog('info', '代理服务器配置已保存')
  } catch (error: any) {
    console.error('Failed to save proxies:', error)
    addLog('error', `保存代理服务器失败: ${error}`)
  }
}

async function saveRules(nextRules?: ProxifierRule[]) {
  try {
    if (nextRules) {
      rules.value = [...nextRules]
    }
    // 保存到内存
    await invoke('save_proxifier_rules', { rules: rules.value })
    // 保存到数据库
    await invoke('save_proxifier_rules_to_db', { rules: rules.value })
    addLog('info', '规则配置已保存')
  } catch (error: any) {
    console.error('Failed to save rules:', error)
    addLog('error', `保存规则失败: ${error}`)
  }
}

async function loadConfig() {
  try {
    const result = await invoke<any>('get_proxifier_config')
    if (result.success && result.data) {
      if (result.data.proxies?.length) {
        proxies.value = result.data.proxies
      }
      if (result.data.rules?.length) {
        rules.value = result.data.rules
      }
      isEnabled.value = result.data.enabled ?? false
    }
  } catch (error) {
    console.error('Failed to load proxifier config:', error)
  }
}

// 从数据库加载代理服务器
async function loadProxiesFromDb() {
  try {
    const result = await invoke<any>('load_proxifier_proxies_from_db')
    if (result.success && result.data) {
      proxies.value = result.data
      addLog('info', `从数据库加载了 ${result.data.length} 个代理服务器`)
    }
  } catch (error) {
    console.error('Failed to load proxies from database:', error)
  }
}

// 从数据库加载规则
async function loadRulesFromDb() {
  try {
    const result = await invoke<any>('load_proxifier_rules_from_db')
    if (result.success && result.data) {
      rules.value = result.data
      addLog('info', `从数据库加载了 ${result.data.length} 条规则`)
    }
  } catch (error) {
    console.error('Failed to load rules from database:', error)
  }
}

// 将 ProxyRequest 转换为 Connection
function proxyRequestToConnection(req: ProxyRequest): Connection {
  const url = new URL(req.url)
  const target = `${url.hostname}:${url.port || (url.protocol === 'https:' ? 443 : 80)}`
  
  return {
    id: `proxy-${req.id}`,
    application: 'HTTP',  // 通过 HTTP 代理的请求
    target: target,
    timeOrStatus: req.status_code > 0 ? `${req.response_time}ms` : 'pending',
    status: req.status_code > 0 ? 'closed' : 'open',
    rule: 'Default',
    proxy: isEnabled.value ? 'Proxifier' : 'Direct',
    sent: 0,  // 暂时没有发送字节数
    received: req.response_size || 0,
  }
}

// Lifecycle
onMounted(async () => {
  // 从数据库加载配置
  await loadProxiesFromDb()
  await loadRulesFromDb()
  await loadConfig()
  
  // 监听代理请求事件（从 TrafficProxy 发送）
  unlistenProxyRequest = await listen<ProxyRequest>('proxy:request', (event) => {
    if (!isEnabled.value) {
      return
    }
    const conn = proxyRequestToConnection(event.payload)
    upsertConnection(conn)
  })
  
  // 监听连接事件（保留兼容）
  unlistenConnection = await listen<Connection>('proxifier:connection', (event) => {
    if (!isEnabled.value) {
      return
    }
    upsertConnection(event.payload)
  })
  
  // 监听日志事件
  unlistenLog = await listen<LogEntry>('proxifier:log', (event) => {
    addLog(event.payload.type, event.payload.message)
  })
})

// 清理事件监听器
onUnmounted(() => {
  if (unlistenProxyRequest) unlistenProxyRequest()
  if (unlistenConnection) unlistenConnection()
  if (unlistenLog) unlistenLog()
})
</script>

<style scoped>
.table-xs th,
.table-xs td {
  padding: 0.5rem 0.75rem;
}

.tabs-boxed .tab-active {
  background-color: hsl(var(--p));
  color: hsl(var(--pc));
}
</style>
