<template>
  <div class="vision-progress mt-3 mb-2 border border-base-300 rounded-lg bg-base-100 overflow-hidden shadow-sm">
    <!-- Header Summary -->
    <div 
      class="flex items-center justify-between px-3 py-2 bg-base-200/50 cursor-pointer hover:bg-base-200 transition-colors select-none"
      @click="isExpanded = !isExpanded"
    >
      <div class="flex items-center gap-2">
        <div v-if="isRunning" class="loading loading-spinner loading-xs text-primary"></div>
        <div v-else :class="['w-2 h-2 rounded-full', isComplete ? 'bg-success' : 'bg-base-content/30']"></div>
        
        <span class="text-xs font-semibold opacity-80">Vision Explorer</span>
        
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
          <span>{{ takeoverMessage || '检测到登录页面，请输入凭证' }}</span>
        </div>
        
        <div class="space-y-2">
          <input
            v-model="credentials.username"
            type="text"
            placeholder="用户名/账号"
            class="input input-sm input-bordered w-full text-xs"
            @keyup.enter="submitCredentials"
          />
          <input
            v-model="credentials.password"
            type="password"
            placeholder="密码"
            class="input input-sm input-bordered w-full text-xs"
            @keyup.enter="submitCredentials"
          />
          <div class="flex gap-2">
            <input
              v-model="credentials.verificationCode"
              type="text"
              placeholder="验证码（可选）"
              class="input input-sm input-bordered flex-1 text-xs"
              @keyup.enter="submitCredentials"
            />
            <button
              class="btn btn-sm btn-warning shrink-0"
              :disabled="!credentials.username || !credentials.password || isSubmittingCredentials"
              @click="submitCredentials"
            >
              <span v-if="isSubmittingCredentials" class="loading loading-spinner loading-xs"></span>
              <span v-else>继续探索</span>
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
import { ref, onMounted, onUnmounted, nextTick } from 'vue'
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
const credentials = ref({
  username: '',
  password: '',
  verificationCode: ''
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
    await invoke('vision_explorer_receive_credentials', {
      executionId: props.executionId,
      username: credentials.value.username,
      password: credentials.value.password,
      verificationCode: credentials.value.verificationCode || null,
      extraFields: null
    })
    pushLog('Credentials submitted, resuming exploration...', 'success')
    showTakeoverForm.value = false
    // Reset credentials for security
    credentials.value = { username: '', password: '', verificationCode: '' }
  } catch (error) {
    console.error('Failed to submit credentials:', error)
    pushLog(`Failed to submit credentials: ${error}`, 'error')
  } finally {
    isSubmittingCredentials.value = false
  }
}

onMounted(async () => {
  // Listen for Vision Explorer events sent by VisionExplorerMessageEmitter
  
  unlisteners.push(await listen<any>('vision:start', (e) => {
    if (e.payload.execution_id !== props.executionId) return
    isRunning.value = true
    isComplete.value = false
    pushLog(`Started exploration of ${e.payload.target_url}`, 'info')
    currentStep.value = 'Starting...'
  }))

  unlisteners.push(await listen<any>('vision:screenshot', (e) => {
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

  unlisteners.push(await listen<any>('vision:analysis', (e) => {
    if (e.payload.execution_id !== props.executionId) return
    const analysis = e.payload.analysis
    
    // Summary of analysis
    const items = []
    if (analysis.estimated_apis && analysis.estimated_apis.length > 0) {
        items.push(`${analysis.estimated_apis.length} APIs`)
    }
    
    pushLog(`Analyzed page.${items.length > 0 ? ' Found: ' + items.join(', ') : ''}`, 'info')
  }))

  unlisteners.push(await listen<any>('vision:action', (e) => {
    if (e.payload.execution_id !== props.executionId) return
    const action = e.payload.action
    const actionStr = `${action.action_type} ${action.value ? `"${action.value}"` : ''}`
    pushLog(`Action: ${actionStr} (${action.success ? 'OK' : 'Failed'})`, action.success ? 'action' : 'error')
    currentStep.value = action.action_type
    stats.value.elements++
  }))
  
   unlisteners.push(await listen<any>('vision:complete', (e) => {
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
  unlisteners.push(await listen<any>('vision:coverage_update', (e) => {
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

  unlisteners.push(await listen<any>('vision:error', (e) => {
    if (e.payload.execution_id !== props.executionId) return
    isRunning.value = false
    pushLog(`Error: ${e.payload.error}`, 'error')
    currentStep.value = 'Error'
  }))

  // Listen for takeover requests (login page detected)
  unlisteners.push(await listen<any>('vision:takeover_request', (e) => {
    if (e.payload.execution_id !== props.executionId) return
    showTakeoverForm.value = true
    takeoverMessage.value = e.payload.message || 'Login page detected. Please enter your credentials.'
    pushLog('Login page detected - waiting for credentials', 'action')
    currentStep.value = 'Waiting for credentials'
  }))

  // Listen for credentials received confirmation
  unlisteners.push(await listen<any>('vision:credentials_received', (e) => {
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
