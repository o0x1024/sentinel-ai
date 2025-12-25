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
          <button 
            class="tab tab-sm" 
            :class="{ 'tab-active': activeSubTab === 'system' }"
            @click="activeSubTab = 'system'"
          >
            <i class="fas fa-cog mr-1"></i>
            System
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

        <!-- System 子面板 - pf 透明代理 -->
        <div v-if="activeSubTab === 'system'" class="flex flex-col h-full overflow-auto">
          <div class="p-3 border-b border-base-300">
            <h3 class="font-semibold text-sm flex items-center gap-2">
              <i class="fas fa-shield-alt text-primary"></i>
              {{ $t('trafficAnalysis.proxifierPanel.transparentProxy') }}
            </h3>
          </div>

          <div class="p-4 space-y-4">

            <div class="bg-base-200 rounded-lg p-3">
              <h4 class="text-xs font-semibold text-base-content/70 mb-2">
                {{ $t('trafficAnalysis.proxifierPanel.transparentProxyStatus') }}
              </h4>
              <div class="space-y-2 text-sm">
                <div class="flex justify-between items-center">
                  <span>{{ $t('trafficAnalysis.proxifierPanel.status') }}</span>
                  <span :class="transparentProxy.enabled ? 'text-success' : 'text-base-content/50'">
                    {{ transparentProxy.enabled ? $t('trafficAnalysis.proxifierPanel.running') : $t('trafficAnalysis.proxifierPanel.stopped') }}
                  </span>
                </div>
                <div class="flex justify-between items-center">
                  <span>{{ $t('trafficAnalysis.proxifierPanel.pfFirewall') }}</span>
                  <span :class="transparentProxy.pfEnabled ? 'text-success' : 'text-base-content/50'">
                    {{ transparentProxy.pfEnabled ? $t('trafficAnalysis.proxifierPanel.enabled') : $t('trafficAnalysis.proxifierPanel.disabled') }}
                  </span>
                </div>
                <div v-if="transparentProxy.enabled" class="flex justify-between items-center">
                  <span>{{ $t('trafficAnalysis.proxifierPanel.proxyPort') }}</span>
                  <span class="text-info">{{ transparentProxy.proxyPort }}</span>
                </div>
                <div v-if="transparentProxy.enabled && transparentProxy.redirectPorts.length > 0" class="flex justify-between items-center">
                  <span>{{ $t('trafficAnalysis.proxifierPanel.redirectPorts') }}</span>
                  <span class="text-info">{{ transparentProxy.redirectPorts.join(', ') }}</span>
                </div>
              </div>
            </div>

            <!-- 透明代理配置 -->
            <div class="space-y-3">
              <div class="form-control">
                <label class="label py-1">
                  <span class="label-text text-sm">{{ $t('trafficAnalysis.proxifierPanel.proxyPort') }}</span>
                </label>
                <input 
                  type="number" 
                  v-model.number="transparentProxyConfig.proxyPort"
                  class="input input-bordered input-sm"
                  placeholder="8080"
                  :disabled="transparentProxy.enabled"
                />
              </div>

              <div class="form-control">
                <label class="label py-1">
                  <span class="label-text text-sm">{{ $t('trafficAnalysis.proxifierPanel.redirectPorts') }}</span>
                </label>
                <input 
                  type="text" 
                  v-model="transparentProxyConfig.redirectPortsStr"
                  class="input input-bordered input-sm"
                  placeholder="80, 443"
                  :disabled="transparentProxy.enabled"
                />
              </div>

              <div class="flex gap-2">
                <button 
                  v-if="!transparentProxy.enabled"
                  class="btn btn-sm btn-primary flex-1"
                  @click="startTransparentProxy"
                  :disabled="isTransparentProxyStarting"
                >
                  <i :class="['fas mr-1', isTransparentProxyStarting ? 'fa-spinner fa-spin' : 'fa-play']"></i>
                  {{ $t('trafficAnalysis.proxifierPanel.startTransparentProxy') }}
                </button>
                <button 
                  v-else
                  class="btn btn-sm btn-warning flex-1"
                  @click="stopTransparentProxy"
                  :disabled="isTransparentProxyStarting"
                >
                  <i :class="['fas mr-1', isTransparentProxyStarting ? 'fa-spinner fa-spin' : 'fa-stop']"></i>
                  {{ $t('trafficAnalysis.proxifierPanel.stopTransparentProxy') }}
                </button>
              </div>
            </div>

            <!-- 透明代理说明 -->
            <div class="alert alert-info text-xs">
              <i class="fas fa-info-circle"></i>
              <div>
                <p><strong>{{ $t('trafficAnalysis.proxifierPanel.transparentProxy') }}</strong> {{ $t('trafficAnalysis.proxifierPanel.transparentProxyDesc') }}</p>
                <p class="mt-1">{{ $t('trafficAnalysis.proxifierPanel.startTransparentProxyDesc') }}</p>
                <p class="mt-1">{{ $t('trafficAnalysis.proxifierPanel.stopTransparentProxyDesc') }}</p>
              </div>
            </div>

            <!-- Electron/Node.js 应用说明 -->
            <div v-if="transparentProxy.enabled" class="bg-base-200 rounded-lg p-3">
              <h4 class="text-xs font-semibold text-base-content/70 mb-2 flex items-center gap-2">
                <i class="fab fa-node-js text-success"></i>
                {{ $t('trafficAnalysis.proxifierPanel.electronNodeJsApp') }}
              </h4>
              <p class="text-xs text-base-content/70 mb-2">
                {{ $t('trafficAnalysis.proxifierPanel.electronNodeJsAppDesc') }}
              </p>
              <div class="bg-base-300 rounded p-2 font-mono text-xs mb-2 break-all">
                export HTTP_PROXY=http://127.0.0.1:{{ transparentProxy.proxyPort }}<br>
                export HTTPS_PROXY=http://127.0.0.1:{{ transparentProxy.proxyPort }}
              </div>
              <button 
                class="btn btn-xs btn-ghost"
                @click="copyProxyEnvCommand"
              >
                <i class="fas fa-copy mr-1"></i>
                {{ $t('trafficAnalysis.proxifierPanel.copyEnvCommand') }}
              </button>
              
              <div class="divider my-2"></div>
              
              <p class="text-xs text-base-content/70 mb-2">
                {{ $t('trafficAnalysis.proxifierPanel.startElectronAppDesc') }}
              </p>
              <div class="bg-base-300 rounded p-2 font-mono text-xs break-all">
                /path/to/app --proxy-server=http://127.0.0.1:{{ transparentProxy.proxyPort }}
              </div>
            </div>
          </div>
        </div>
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
const activeSubTab = ref<'proxies' | 'rules' | 'system'>('proxies')
const bottomTab = ref<'connections' | 'traffic' | 'statistics'>('connections')
const selectedConnection = ref<string | null>(null)

// pf 透明代理状态
interface TransparentProxyState {
  enabled: boolean
  proxyPort: number
  redirectPorts: number[]
  pfEnabled: boolean
}

const transparentProxy = ref<TransparentProxyState>({
  enabled: false,
  proxyPort: 8080,
  redirectPorts: [],
  pfEnabled: false
})

const transparentProxyConfig = ref({
  proxyPort: 8080,
  redirectPortsStr: '80, 443'
})

const isTransparentProxyStarting = ref(false)

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

// 复制代理环境变量命令
async function copyProxyEnvCommand() {
  const port = transparentProxy.value.proxyPort
  const command = `export HTTP_PROXY=http://127.0.0.1:${port}\nexport HTTPS_PROXY=http://127.0.0.1:${port}`
  try {
    await navigator.clipboard.writeText(command)
    addLog('info', '已复制代理环境变量命令到剪贴板')
  } catch (error) {
    addLog('error', '复制失败，请手动复制')
  }
}

async function toggleProxifier() {
  isToggling.value = true
  try {
    if (isEnabled.value) {
      const result = await invoke<any>('stop_proxifier')
      if (result.success) {
        isEnabled.value = false
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

async function saveProxies() {
  try {
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

async function saveRules() {
  try {
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

// pf 透明代理相关方法
async function refreshTransparentProxyStatus() {
  try {
    const result = await invoke<any>('get_transparent_proxy_status')
    if (result.success && result.data) {
      transparentProxy.value = {
        enabled: result.data.enabled,
        proxyPort: result.data.proxy_port,
        redirectPorts: result.data.redirect_ports || [],
        pfEnabled: result.data.pf_enabled
      }
    }
  } catch (error) {
    console.error('Failed to get transparent proxy status:', error)
  }
}

async function startTransparentProxy() {
  isTransparentProxyStarting.value = true
  try {
    // 解析重定向端口
    const redirectPorts = transparentProxyConfig.value.redirectPortsStr
      .split(',')
      .map(s => parseInt(s.trim()))
      .filter(n => !isNaN(n) && n > 0 && n < 65536)
    
    const proxyPort = transparentProxyConfig.value.proxyPort
    
    // 1. 先启动代理服务器在对应端口
    addLog('info', `正在启动代理服务器在端口 ${proxyPort}...`)
    const proxyConfig = {
      start_port: proxyPort,
      max_port_attempts: 1,  // 只尝试指定端口
      mitm_enabled: true,
      max_request_body_size: 2 * 1024 * 1024,
      max_response_body_size: 2 * 1024 * 1024,
    }
    
    const proxyResult = await invoke<any>('start_passive_scan', { config: proxyConfig })
    if (proxyResult.success && proxyResult.data) {
      addLog('info', `代理服务器已启动，监听端口: ${proxyResult.data}`)
    } else if (proxyResult.error && proxyResult.error.includes('already running')) {
      addLog('info', '代理服务器已在运行中')
    } else if (proxyResult.error) {
      addLog('error', `启动代理服务器失败: ${proxyResult.error}`)
      return
    }
    
    // 2. 再启动 pf 透明代理
    const result = await invoke<any>('start_transparent_proxy', {
      proxyPort,
      redirectPorts
    })
    
    if (result.success) {
      addLog('info', `pf 透明代理已启动，重定向端口 ${redirectPorts.join(', ')} 到 ${proxyPort}`)
    } else {
      addLog('error', `启动透明代理失败: ${result.error}`)
    }
    await refreshTransparentProxyStatus()
  } catch (error: any) {
    addLog('error', `启动透明代理失败: ${error}`)
  } finally {
    isTransparentProxyStarting.value = false
  }
}

async function stopTransparentProxy() {
  isTransparentProxyStarting.value = true
  try {
    // 1. 先停止 pf 透明代理
    const result = await invoke<any>('stop_transparent_proxy')
    if (result.success) {
      addLog('info', 'pf 透明代理已停止')
    } else {
      addLog('error', `停止透明代理失败: ${result.error}`)
    }
    
    // 2. 停止代理监听器
    addLog('info', '正在停止代理监听器...')
    const stopResult = await invoke<any>('stop_passive_scan')
    if (stopResult.success) {
      addLog('info', '代理监听器已停止')
    } else if (stopResult.error) {
      addLog('warning', `停止代理监听器: ${stopResult.error}`)
    }
    
    await refreshTransparentProxyStatus()
  } catch (error: any) {
    addLog('error', `停止透明代理失败: ${error}`)
  } finally {
    isTransparentProxyStarting.value = false
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
    proxy: transparentProxy.value.enabled ? `pf:${transparentProxy.value.proxyPort}` : 'Direct',
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
  await refreshTransparentProxyStatus()
  
  // 如果数据库没有数据，添加默认配置
  if (proxies.value.length === 0) {
    proxies.value = [
      { id: '1', name: '127.0.0.1', host: '127.0.0.1', port: 8080, type: 'HTTP', enabled: true },
    ]
  }
  if (rules.value.length === 0) {
    rules.value = [
      { id: '1', name: 'Localhost', enabled: false, applications: 'Any', targetHosts: 'localhost; 127.0.0.1; ::1', targetPorts: 'Any', action: 'Direct' },
      { id: '2', name: 'Default', enabled: true, applications: 'Any', targetHosts: 'Any', targetPorts: 'Any', action: 'Direct' },
    ]
  }
  
  // 监听代理请求事件（从 PassiveProxy 发送）
  unlistenProxyRequest = await listen<ProxyRequest>('proxy:request', (event) => {
    const conn = proxyRequestToConnection(event.payload)
    const existing = connections.value.findIndex(c => c.id === conn.id)
    if (existing >= 0) {
      connections.value[existing] = conn
    } else {
      connections.value.unshift(conn)
      if (connections.value.length > 1000) {
        connections.value.pop()
      }
    }
  })
  
  // 监听连接事件（保留兼容）
  unlistenConnection = await listen<Connection>('proxifier:connection', (event) => {
    const conn = event.payload
    const existing = connections.value.findIndex(c => c.id === conn.id)
    if (existing >= 0) {
      connections.value[existing] = conn
    } else {
      connections.value.unshift(conn)
      if (connections.value.length > 1000) {
        connections.value.pop()
      }
    }
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
