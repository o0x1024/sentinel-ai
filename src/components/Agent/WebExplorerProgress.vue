<template>
  <div class="web-explorer-progress mt-3 mb-2 border border-base-300 rounded-lg bg-base-100 overflow-hidden shadow-sm">
    <!-- Header Summary -->
    <div 
      class="flex items-center justify-between px-3 py-2 bg-base-200/50 cursor-pointer hover:bg-base-200 transition-colors select-none"
      @click="isExpanded = !isExpanded"
    >
      <div class="flex items-center gap-2">
        <div v-if="isRunning" class="loading loading-spinner loading-xs text-primary"></div>
        <div v-else :class="['w-2 h-2 rounded-full', isComplete ? 'bg-success' : 'bg-base-content/30']"></div>
        
        <span class="text-xs font-semibold opacity-80">Web Explorer</span>
        
        <div v-if="currentStep" class="badge badge-sm badge-ghost text-[10px] font-mono h-5">
           {{ currentStep }}
        </div>
        
        <!-- Coverage Badge -->
        <div v-if="coverage.overall > 0" class="badge badge-sm text-[10px] h-5" :class="getCoverageBadgeClass(coverage.overall)">
           {{ coverage.overall.toFixed(0) }}%
        </div>
      </div>
      
      <div class="flex items-center gap-2">
        <span class="text-[10px] opacity-50 font-mono">{{ logs.length }} logs</span>
        <i :class="['fas fa-chevron-down text-xs opacity-50 transition-transform duration-200', isExpanded ? 'rotate-180' : '']"></i>
      </div>
    </div>

    <!-- Expanded Content -->
    <div v-show="isExpanded" class="border-t border-base-300 transition-all duration-300 ease-in-out">
      <!-- Multi-Agent Workers Dashboard -->
      <div v-if="isMultiAgentMode && workers.size > 0" class="p-2 bg-gradient-to-r from-primary/5 to-secondary/5 border-b border-base-300">
        <div class="flex items-center justify-between mb-2">
          <div class="flex items-center gap-2 text-xs">
            <span class="font-bold text-primary">🤖 Multi-Agent</span>
            <span class="badge badge-xs badge-primary">{{ multiAgentMode }}</span>
          </div>
          <span class="text-[10px] opacity-70">{{ completedWorkers }}/{{ totalWorkers }} workers</span>
        </div>
        <div class="grid grid-cols-2 gap-1.5">
          <div
            v-for="[taskId, worker] in workers"
            :key="taskId"
            class="flex items-center gap-2 bg-base-100 rounded px-2 py-1 text-[10px]"
            :class="{
              'ring-1 ring-primary/50': worker.status === 'running',
              'opacity-60': worker.status === 'pending'
            }"
          >
            <span v-if="worker.status === 'completed'" class="text-success text-[8px]">✓</span>
            <span v-else-if="worker.status === 'running'" class="loading loading-spinner loading-xs text-primary"></span>
            <span v-else class="opacity-30 text-[8px]">○</span>
            <span class="truncate flex-1 font-medium">{{ worker.scope_name }}</span>
            <span class="opacity-60">{{ worker.pages_visited }}p/{{ worker.apis_discovered }}a</span>
          </div>
        </div>
      </div>

      <!-- Coverage Dashboard -->
      <div v-if="showCoverage" class="grid grid-cols-4 gap-2 p-2 bg-base-200/30 border-b border-base-300">
        <div class="flex flex-col items-center">
          <div class="radial-progress text-primary text-[10px]" :style="`--value:${coverage.route}; --size:2.5rem; --thickness:3px`">
            {{ coverage.route.toFixed(0) }}%
          </div>
          <span class="text-[9px] opacity-60 mt-1">{{ t('agent.route') }}</span>
        </div>
        <div class="flex flex-col items-center">
          <div class="radial-progress text-secondary text-[10px]" :style="`--value:${coverage.element}; --size:2.5rem; --thickness:3px`">
            {{ coverage.element.toFixed(0) }}%
          </div>
          <span class="text-[9px] opacity-60 mt-1">{{ t('agent.element') }}</span>
        </div>
        <div class="flex flex-col items-center">
          <div class="radial-progress text-accent text-[10px]" :style="`--value:${coverage.component}; --size:2.5rem; --thickness:3px`">
            {{ coverage.component.toFixed(0) }}%
          </div>
          <span class="text-[9px] opacity-60 mt-1">{{ t('agent.component') }}</span>
        </div>
        <div class="flex flex-col items-center">
          <div class="text-lg font-bold text-info">{{ stats.apis }}</div>
          <span class="text-[9px] opacity-60">APIs</span>
        </div>
      </div>
      
      <!-- Pending Routes -->
      <div v-if="pendingRoutes.length > 0" class="p-2 bg-warning/10 border-b border-base-300">
        <div class="flex items-center gap-1 text-[10px] text-warning font-medium mb-1">
          <i class="fas fa-route"></i>
          {{ t('agent.pendingRoutesCount', { count: pendingRoutes.length }) }}
        </div>
        <div class="text-[9px] opacity-70 max-h-[40px] overflow-y-auto">
          {{ pendingRoutes.slice(0, 3).join(' • ') }}
          <span v-if="pendingRoutes.length > 3">... +{{ pendingRoutes.length - 3 }} more</span>
        </div>
      </div>
      
      <!-- Screenshot Preview -->
      <div v-if="lastScreenshot" class="relative group bg-base-300 min-h-[120px] max-h-[240px] overflow-hidden flex items-center justify-center p-2">
        <img :src="lastScreenshot.path" class="max-w-full max-h-[220px] object-contain rounded shadow-lg" />
        <div class="absolute bottom-0 left-0 right-0 bg-gradient-to-t from-black/80 to-transparent text-white text-[10px] p-2 pt-4 backdrop-blur-[1px]">
           <div class="font-bold truncate px-1">{{ lastScreenshot.title || 'Page Screenshot' }}</div>
        </div>
      </div>

      <!-- Logs -->
      <div class="max-h-[160px] overflow-y-auto p-2 space-y-1 bg-base-100 text-[11px] font-mono scrollbar-thin" ref="logsContainer">
        <div v-for="(log, i) in logs" :key="i" class="flex gap-2 group hover:bg-base-200/50 rounded px-1 transition-colors">
          <span class="opacity-30 shrink-0 select-none w-14 text-[10px] pt-[1px]">{{ log.time }}</span>
          <span :class="['break-all', getLogClass(log.type)]">{{ log.message }}</span>
        </div>
        <div v-if="logs.length === 0" class="text-center opacity-30 py-4 flex flex-col items-center gap-2">
            <i class="fas fa-satellite-dish animate-pulse"></i>
            <span>Waiting for events...</span>
        </div>
      </div>
      
      <!-- Login Takeover Form -->
      <div v-if="showTakeoverForm" class="p-3 bg-warning/10 border-b border-warning/30">
        <div class="flex items-center gap-2 text-sm text-warning font-medium mb-2">
          <i class="fas fa-key"></i>
          <span>{{ takeoverMessage || 'Login page detected. Please enter credentials below or click "Skip Login" to continue without authentication.' }}</span>
        </div>
        
        <div class="space-y-2">
          <!-- 动态渲染登录字段 -->
          <template v-if="loginFields.length > 0">
            <div v-for="field in loginFields" :key="field.id" class="w-full">
              <input
                v-model="dynamicCredentials[field.id]"
                :type="field.field_type === 'password' ? 'password' : 'text'"
                :placeholder="field.placeholder || field.label"
                class="input input-sm input-bordered w-full text-xs"
                @keyup.enter="submitCredentials"
              />
            </div>
          </template>
          
          <!-- 回退：如果没有检测到字段，使用默认的账号密码 -->
          <template v-else>
            <input
              v-model="dynamicCredentials.username"
              type="text"
              placeholder="请输入账号"
              class="input input-sm input-bordered w-full text-xs"
              @keyup.enter="submitCredentials"
            />
            <input
              v-model="dynamicCredentials.password"
              type="password"
              placeholder="请输入密码"
              class="input input-sm input-bordered w-full text-xs"
              @keyup.enter="submitCredentials"
            />
          </template>
          
          <div class="flex gap-2">
            <button
              class="btn btn-sm btn-warning flex-1"
              :disabled="!canSubmitCredentials || isSubmittingCredentials"
              @click="submitCredentials"
            >
              <span v-if="isSubmittingCredentials" class="loading loading-spinner loading-xs"></span>
              <span v-else>继续探索</span>
            </button>
            <button
              class="btn btn-sm btn-ghost"
              :disabled="isSubmittingCredentials"
              @click="skipLogin"
            >
              跳过登录
            </button>
          </div>
        </div>
      </div>
      
      <!-- Footer Stats -->
      <div class="flex gap-4 p-2 bg-base-200/30 text-[10px] border-t border-base-300 font-medium">
         <div class="flex items-center gap-1 opacity-70" title="Pages Visited">
            <i class="fas fa-globe w-3 text-center"></i> {{ stats.pages }} pages
         </div>
         <div class="flex items-center gap-1 opacity-70" title="APIs Discovered">
            <i class="fas fa-network-wired w-3 text-center"></i> {{ stats.apis }} APIs
         </div>
         <div class="flex items-center gap-1 opacity-70" title="Elements Interacted">
            <i class="fas fa-mouse-pointer w-3 text-center"></i> {{ stats.elements }} interactions
         </div>
         <div v-if="stableRounds > 0" class="flex items-center gap-1 opacity-70 ml-auto" title="Stable Rounds">
            <i class="fas fa-check-double w-3 text-center"></i> {{ stableRounds }}/5 stable
         </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, nextTick } from 'vue'
import { useI18n } from 'vue-i18n'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { convertFileSrc, invoke } from '@tauri-apps/api/core'

const { t } = useI18n()

const props = defineProps<{
  executionId: string
}>()

const isExpanded = ref(true)
const isRunning = ref(false)
const currentStep = ref<string>('')
const logs = ref<Array<{ time: string, message: string, type: 'info' | 'error' | 'success' | 'action' }>>([])
const lastScreenshot = ref<{ path: string, title: string } | null>(null)
const logsContainer = ref<HTMLElement | null>(null)
const stats = ref({ pages: 0, apis: 0, elements: 0 })
const isComplete = ref(false)

// Coverage data
const coverage = ref({ route: 0, element: 0, component: 100, overall: 0 })
const pendingRoutes = ref<string[]>([])
const stableRounds = ref(0)
const showCoverage = ref(false)

// Takeover form state
const showTakeoverForm = ref(false)
const takeoverMessage = ref('')
const isSubmittingCredentials = ref(false)

// Multi-Agent state
interface WorkerInfo {
  task_id: string
  scope_name: string
  status: 'pending' | 'running' | 'completed' | 'failed'
  pages_visited: number
  apis_discovered: number
  progress: number
}
const isMultiAgentMode = ref(false)
const multiAgentMode = ref<string>('Sequential')
const workers = ref<Map<string, WorkerInfo>>(new Map())
const totalWorkers = ref(0)
const completedWorkers = ref(0)

// 登录字段定义接口
interface LoginField {
  id: string
  label: string
  field_type: string
  required: boolean
  placeholder?: string
}

// 动态登录字段列表（从后端检测获取）
const loginFields = ref<LoginField[]>([])

// 动态凭据输入（key 为字段 id）
const dynamicCredentials = ref<Record<string, string>>({})

// 旧的固定格式凭据（保留作为向后兼容）
const credentials = ref({
  username: '',
  password: '',
  verificationCode: ''
})

// 计算是否可以提交凭据
const canSubmitCredentials = computed(() => {
  if (loginFields.value.length > 0) {
    // 检查所有必填字段是否已填写
    return loginFields.value
      .filter(f => f.required)
      .every(f => dynamicCredentials.value[f.id]?.trim())
  } else {
    // 回退模式：检查账号密码
    return dynamicCredentials.value.username?.trim() && dynamicCredentials.value.password?.trim()
  }
})

const unlisteners: UnlistenFn[] = []

// Get coverage badge color based on percentage
const getCoverageBadgeClass = (pct: number) => {
  if (pct >= 95) return 'badge-success'
  if (pct >= 70) return 'badge-warning'
  if (pct >= 40) return 'badge-info'
  return 'badge-ghost'
}

const pushLog = (message: string, type: 'info' | 'error' | 'success' | 'action' = 'info') => {
  logs.value.push({
    time: new Date().toLocaleTimeString('en-US', { hour12: false, hour: '2-digit', minute:'2-digit', second:'2-digit' }),
    message,
    type
  })
  
  if (isExpanded.value) {
    nextTick(() => {
      if (logsContainer.value) {
        logsContainer.value.scrollTop = logsContainer.value.scrollHeight
      }
    })
  }
}

const getLogClass = (type: string) => {
  switch (type) {
    case 'error': return 'text-error font-medium'
    case 'success': return 'text-success font-medium'
    case 'action': return 'text-warning font-medium'
    default: return 'text-base-content/80'
  }
}

// Submit credentials to backend
const submitCredentials = async () => {
  isSubmittingCredentials.value = true
  try {
    // 构建凭据对象
    let username = ''
    let password = ''
    let verificationCode: string | null = null
    const extraFields: Record<string, string> = {}
    
    if (loginFields.value.length > 0) {
      // 从动态字段获取凭据
      for (const field of loginFields.value) {
        const value = dynamicCredentials.value[field.id] || ''
        if (field.id === 'username' || field.field_type === 'email' || field.id.includes('user') || field.id.includes('account')) {
          username = value
        } else if (field.id === 'password' || field.field_type === 'password') {
          password = value
        } else if (field.id === 'verification_code' || field.id.includes('code') || field.id.includes('captcha')) {
          verificationCode = value || null
        } else {
          extraFields[field.id] = value
        }
      }
    } else {
      // 使用回退模式的固定字段
      username = dynamicCredentials.value.username || ''
      password = dynamicCredentials.value.password || ''
    }
    
    await invoke('web_explorer_receive_credentials', {
      executionId: props.executionId,
      username,
      password,
      verificationCode,
      extraFields: Object.keys(extraFields).length > 0 ? extraFields : null
    })
    pushLog('Credentials submitted, resuming exploration...', 'success')
    showTakeoverForm.value = false
    // 重置凭据
    dynamicCredentials.value = {}
    loginFields.value = []
  } catch (error) {
    console.error('Failed to submit credentials:', error)
    pushLog(`Failed to submit credentials: ${error}`, 'error')
  } finally {
    isSubmittingCredentials.value = false
  }
}

// Skip login and continue without credentials
const skipLogin = async () => {
  try {
    await invoke('web_explorer_skip_login', {
      executionId: props.executionId
    })
    pushLog('Login skipped, continuing exploration...', 'info')
    showTakeoverForm.value = false
    dynamicCredentials.value = {}
    loginFields.value = []
  } catch (error) {
    console.error('Failed to skip login:', error)
    pushLog(`Failed to skip login: ${error}`, 'error')
  }
}

onMounted(async () => {
  // Listen for Web Explorer events

  // Multi-Agent events
  unlisteners.push(await listen<any>('web_explorer:multi_agent', (e) => {
    const payload = e.payload
    if (payload.execution_id !== props.executionId) return

    switch (payload.type) {
      case 'multi_agent_start':
        isMultiAgentMode.value = true
        multiAgentMode.value = payload.mode || 'Sequential'
        totalWorkers.value = payload.total_workers || 0
        completedWorkers.value = 0
        workers.value.clear()
        pushLog(`Multi-Agent mode: ${payload.mode} with ${payload.total_workers} workers`, 'info')
        break

      case 'worker_tasks':
        if (payload.tasks) {
          for (const task of payload.tasks) {
            const taskId = task.task_id || task.id
            workers.value.set(taskId, {
              task_id: taskId,
              scope_name: task.scope_name || task.scope?.name || '',
              status: 'pending',
              pages_visited: 0,
              apis_discovered: 0,
              progress: 0
            })
          }
          pushLog(`Manager assigned ${payload.tasks.length} scopes to explore`, 'info')
        }
        break

      case 'worker_progress':
        if (payload.worker) {
          const w = payload.worker
          workers.value.set(w.task_id, {
            task_id: w.task_id,
            scope_name: w.scope_name,
            status: w.status || 'running',
            pages_visited: w.pages_visited || 0,
            apis_discovered: w.apis_discovered || 0,
            progress: w.progress || 0
          })
          currentStep.value = `${w.scope_name}: ${w.status}`
        }
        break

      case 'worker_complete':
        if (payload.task_id) {
          const existing = workers.value.get(payload.task_id)
          if (existing) {
            existing.status = 'completed'
            existing.progress = 100
            if (payload.stats) {
              existing.pages_visited = payload.stats.pages_visited || existing.pages_visited
              existing.apis_discovered = payload.stats.apis_discovered || existing.apis_discovered
            }
          }
          completedWorkers.value += 1
          pushLog(`Worker completed: ${payload.scope_name} (${payload.stats?.pages_visited || 0} pages, ${payload.stats?.apis_discovered || 0} APIs)`, 'success')
        }
        break

      case 'multi_agent_stats':
        if (payload.global_stats) {
          stats.value = {
            pages: payload.global_stats.total_urls_visited || 0,
            apis: payload.global_stats.total_apis_discovered || 0,
            elements: payload.global_stats.total_elements_interacted || 0
          }
        }
        if (payload.mode_info) {
          completedWorkers.value = payload.mode_info.completed_workers || 0
        }
        break
    }
  }))
  
  unlisteners.push(await listen<any>('web_explorer:start', (e) => {
    if (e.payload.execution_id !== props.executionId) return
    isRunning.value = true
    isComplete.value = false
    pushLog(`Started exploration of ${e.payload.target_url}`, 'info')
    currentStep.value = 'Starting...'
  }))

  unlisteners.push(await listen<any>('web_explorer:screenshot', (e) => {
    if (e.payload.execution_id !== props.executionId) return
    const src = convertFileSrc(e.payload.path)
    lastScreenshot.value = {
        path: src,
        title: e.payload.title
    }
    pushLog(`Captured page: ${e.payload.title}`, 'info')
    currentStep.value = `Analyzing Page ${e.payload.iteration}`
    stats.value.pages = e.payload.iteration 
  }))

  unlisteners.push(await listen<any>('web_explorer:analysis', (e) => {
    if (e.payload.execution_id !== props.executionId) return
    const analysis = e.payload.analysis
    
    // Summary of analysis
    const items = []
    if (analysis.estimated_apis && analysis.estimated_apis.length > 0) {
        items.push(`${analysis.estimated_apis.length} APIs`)
    }
    
    pushLog(`Analyzed page.${items.length > 0 ? ' Found: ' + items.join(', ') : ''}`, 'info')
  }))

  unlisteners.push(await listen<any>('web_explorer:action', (e) => {
    if (e.payload.execution_id !== props.executionId) return
    const action = e.payload.action
    const actionStr = `${action.action_type} ${action.value ? `"${action.value}"` : ''}`
    pushLog(`Action: ${actionStr} (${action.success ? 'OK' : 'Failed'})`, action.success ? 'action' : 'error')
    currentStep.value = action.action_type
    stats.value.elements++
  }))
  
   unlisteners.push(await listen<any>('web_explorer:complete', (e) => {
    if (e.payload.execution_id !== props.executionId) return
    isRunning.value = false
    isComplete.value = true
    currentStep.value = 'Completed'
    const s = e.payload.stats
    pushLog(`Exploration completed: ${s.status}`, 'success')
    pushLog(`Summary: ${s.pages_visited} pages, ${s.apis_discovered} APIs, ${s.total_iterations} steps`, 'success')
    
    stats.value = {
        pages: s.pages_visited,
        apis: s.apis_discovered,
        elements: s.elements_interacted
    }
  }))

  // Listen for coverage updates
  unlisteners.push(await listen<any>('web_explorer:coverage_update', (e) => {
    if (e.payload.execution_id !== props.executionId) return
    
    showCoverage.value = true
    
    const cov = e.payload.coverage
    coverage.value = {
      route: cov.route_coverage || 0,
      element: cov.element_coverage || 0,
      component: cov.component_coverage || 100,
      overall: cov.overall_coverage || 0
    }
    
    pendingRoutes.value = e.payload.pending_routes || []
    stableRounds.value = e.payload.stable_rounds || 0
    stats.value.apis = e.payload.api_count || stats.value.apis
    
    pushLog(`Coverage: ${coverage.value.overall.toFixed(1)}% (Routes: ${coverage.value.route.toFixed(0)}%, Elements: ${coverage.value.element.toFixed(0)}%)`, 'info')
  }))

  unlisteners.push(await listen<any>('web_explorer:error', (e) => {
    if (e.payload.execution_id !== props.executionId) return
    isRunning.value = false
    pushLog(`Error: ${e.payload.error}`, 'error')
    currentStep.value = 'Error'
  }))

  // Listen for takeover requests (login page detected)
  unlisteners.push(await listen<any>('web_explorer:takeover_request', (e) => {
    if (e.payload.execution_id !== props.executionId) return
    showTakeoverForm.value = true
    takeoverMessage.value = e.payload.message || 'Login page detected. Please enter credentials below or click "Skip Login" to continue without authentication.'
    
    // 解析登录字段
    if (e.payload.fields && Array.isArray(e.payload.fields)) {
      loginFields.value = e.payload.fields.map((f: any) => ({
        id: f.id || '',
        label: f.label || f.id || '',
        field_type: f.field_type || 'text',
        required: f.required !== false,
        placeholder: f.placeholder || undefined
      }))
      // 初始化动态凭据对象
      dynamicCredentials.value = {}
      for (const field of loginFields.value) {
        dynamicCredentials.value[field.id] = ''
      }
      pushLog(`Login page detected with ${loginFields.value.length} field(s) - waiting for credentials`, 'action')
    } else {
      // 没有字段信息，使用默认的账号密码
      loginFields.value = []
      dynamicCredentials.value = { username: '', password: '' }
      pushLog('Login page detected - waiting for credentials', 'action')
    }
    
    currentStep.value = 'Waiting for credentials'
  }))

  // Listen for credentials received confirmation
  unlisteners.push(await listen<any>('web_explorer:credentials_received', (e) => {
    if (e.payload.execution_id !== props.executionId) return
    showTakeoverForm.value = false
    pushLog('Credentials received, continuing exploration...', 'success')
    currentStep.value = 'Logging in...'
  }))
})

onUnmounted(() => {
  unlisteners.forEach(u => u())
})
</script>

<style scoped>
.scrollbar-thin::-webkit-scrollbar {
  width: 6px;
}
.scrollbar-thin::-webkit-scrollbar-track {
  background: transparent;
}
.scrollbar-thin::-webkit-scrollbar-thumb {
  background-color: currentColor;
  opacity: 0.1;
  border-radius: 20px;
}
.scrollbar-thin {
  scrollbar-width: thin;
}
</style>
