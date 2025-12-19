import { ref, computed, onMounted, onUnmounted, type Ref } from 'vue'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'

export interface VisionStep {
  iteration: number
  phase: string
  status: string
  url?: string
  title?: string
  screenshot?: string
  analysis?: {
    page_analysis: string
    estimated_apis?: string[]
    exploration_progress: number
  }
  action?: {
    action_type: string
    element_index?: number
    value?: string
    reason: string
    success: boolean
    duration_ms?: number
  }
  error?: string
}

export interface VisionPlan {
  phase: string
  phase_name: string
  goal: string
  steps: string[]
  completion_criteria: string
  reason: string
}

export interface VisionProgress {
  phase: string
  iteration: number
  max_iterations: number
  pages_visited: number
  apis_discovered: number
  elements_interacted: number
}

export interface VisionCoverage {
  route_coverage: number
  element_coverage: number
  component_coverage: number
  overall_coverage: number
  api_count: number
  pending_routes: string[]
  stable_rounds: number
}

export interface WorkerTask {
  task_id: string
  scope_name: string
  entry_url: string
  url_patterns: string[]
  max_iterations: number
  priority: number
}

export interface WorkerProgress {
  task_id: string
  scope_name: string
  status: 'pending' | 'running' | 'completed' | 'failed'
  pages_visited: number
  apis_discovered: number
  elements_interacted: number
  iterations_used: number
  progress: number
  completion_reason?: string
}

export interface MultiAgentMode {
  is_multi_agent: boolean
  mode: 'Sequential' | 'Parallel' | 'Adaptive'
  total_workers: number
  completed_workers: number
}

export interface MultiAgentState {
  mode: MultiAgentMode | null
  tasks: WorkerTask[]
  workers: Map<string, WorkerProgress>
  globalStats: {
    total_urls_visited: number
    total_apis_discovered: number
    workers_completed: number
    total_elements_interacted: number
  } | null
}

export type VisionActivityEvent =
  | { type: 'multi_agent_start'; ts: number; mode: string; total_workers: number }
  | { type: 'worker_tasks'; ts: number; count: number }
  | { type: 'worker_progress'; ts: number; task_id: string; scope_name: string; status: string; pages: number; apis: number; progress: number }
  | { type: 'worker_complete'; ts: number; task_id: string; scope_name: string; pages: number; apis: number; completion_reason?: string }
  | { type: 'worker_decision'; ts: number; task_id: string; scope_name: string; iteration: number; page_analysis: string; action_type: string; element_index?: number; value?: string; reason: string; progress: number; estimated_apis?: string[] }
  | { type: 'worker_action'; ts: number; task_id: string; scope_name: string; iteration: number; action_type: string; element_index?: number; value?: string; success: boolean; duration_ms?: number; reason: string }
  | { type: 'vision_plan'; ts: number; phase_name: string; phase: string }
  | { type: 'vision_progress'; ts: number; phase: string; iteration: number; max_iterations: number }
  | { type: 'api_discovered'; ts: number; method: string; url: string }
  | { type: 'takeover_request'; ts: number; request_type: string }
  | { type: 'complete'; ts: number; status: string }

interface VisionV2Envelope {
  execution_id: string
  type: string
  ts: number
  data: any
}

export function useVisionEvents(executionId?: Ref<string | null>) {
  const steps = ref<VisionStep[]>([])
  const coverage = ref<VisionCoverage | null>(null)
  const discoveredApis = ref<{ method: string; url: string }[]>([])
  const isVisionActive = ref(false)
  const currentUrl = ref('')

  const currentPlan = ref<VisionPlan | null>(null)
  const currentProgress = ref<VisionProgress | null>(null)

  const showTakeoverForm = ref(false)
  const takeoverMessage = ref('')
  const takeoverFields = ref<any[] | null>(null)
  const loginTimeoutSeconds = ref<number | null>(null)  // Timeout for manual login
  const currentExecutionId = ref<string | null>(null)

  const multiAgent = ref<MultiAgentState>({
    mode: null,
    tasks: [],
    workers: new Map(),
    globalStats: null
  })
  const isMultiAgentMode = computed(() => multiAgent.value.mode?.is_multi_agent ?? false)

  const activity = ref<VisionActivityEvent[]>([])

  const unlisteners: UnlistenFn[] = []

  const resetstate = () => {
    steps.value = []
    coverage.value = null
    discoveredApis.value = []
    isVisionActive.value = false
    currentUrl.value = ''
    currentPlan.value = null
    currentProgress.value = null
    showTakeoverForm.value = false
    takeoverMessage.value = ''
    takeoverFields.value = null
    loginTimeoutSeconds.value = null
    currentExecutionId.value = null
    activity.value = []
    multiAgent.value = {
      mode: null,
      tasks: [],
      workers: new Map(),
      globalStats: null
    }
  }

  const pushActivity = (evt: VisionActivityEvent) => {
    activity.value.push(evt)
    if (activity.value.length > 200) {
      activity.value.splice(0, activity.value.length - 200)
    }
  }

  const handleV2 = (payload: VisionV2Envelope) => {
    if (executionId?.value && payload.execution_id !== executionId.value) return

    isVisionActive.value = true
    currentExecutionId.value = payload.execution_id

    const now = payload.ts || Date.now()
    const data = payload.data

    switch (payload.type) {
      case 'start': {
        resetstate()
        isVisionActive.value = true
        currentExecutionId.value = payload.execution_id
        if (data?.target_url) currentUrl.value = data.target_url
        return
      }

      case 'vision_step': {
        const step = data?.step as VisionStep | undefined
        if (step) {
          steps.value.push(step)
          if (step.url) currentUrl.value = step.url
        }
        return
      }

      case 'coverage_update': {
        if (!data) return
        coverage.value = {
          route_coverage: data.coverage?.route_coverage ?? 0,
          element_coverage: data.coverage?.element_coverage ?? 0,
          component_coverage: data.coverage?.component_coverage ?? 0,
          overall_coverage: data.coverage?.overall_coverage ?? 0,
          api_count: data.api_count ?? 0,
          pending_routes: data.pending_routes ?? [],
          stable_rounds: data.stable_rounds ?? 0
        }
        return
      }

      case 'api_discovered': {
        if (!data) return
        const api = { method: data.method, url: data.api }
        if (!discoveredApis.value.some(a => a.method === api.method && a.url === api.url)) {
          discoveredApis.value.push(api)
        }
        pushActivity({ type: 'api_discovered', ts: now, method: data.method, url: data.api })
        return
      }

      case 'takeover_request': {
        showTakeoverForm.value = true
        takeoverMessage.value = data?.message || ''
        takeoverFields.value = data?.fields || null
        loginTimeoutSeconds.value = data?.timeout_seconds ?? null
        pushActivity({ type: 'takeover_request', ts: now, request_type: data?.request_type || 'login' })
        return
      }

      case 'credentials_received': {
        showTakeoverForm.value = false
        takeoverMessage.value = ''
        takeoverFields.value = null
        loginTimeoutSeconds.value = null
        return
      }

      case 'login_wait_status': {
        // Update timeout remaining if provided
        if (data?.waiting === false) {
          showTakeoverForm.value = false
          loginTimeoutSeconds.value = null
        } else if (data?.remaining_seconds !== undefined) {
          loginTimeoutSeconds.value = data.remaining_seconds
        }
        return
      }

      case 'vision_plan': {
        currentPlan.value = {
          phase: data?.phase || '',
          phase_name: data?.phase_name || '',
          goal: data?.goal || '',
          steps: data?.steps || [],
          completion_criteria: data?.completion_criteria || '',
          reason: data?.reason || ''
        }
        pushActivity({ type: 'vision_plan', ts: now, phase_name: data?.phase_name || '', phase: data?.phase || '' })
        return
      }

      case 'vision_progress': {
        currentProgress.value = {
          phase: data?.phase || '',
          iteration: data?.iteration || 0,
          max_iterations: data?.max_iterations || 100,
          pages_visited: data?.pages_visited || 0,
          apis_discovered: data?.apis_discovered || 0,
          elements_interacted: data?.elements_interacted || 0
        }
        pushActivity({
          type: 'vision_progress',
          ts: now,
          phase: data?.phase || '',
          iteration: data?.iteration || 0,
          max_iterations: data?.max_iterations || 100
        })
        return
      }

      case 'multi_agent_start': {
        multiAgent.value.mode = {
          is_multi_agent: true,
          mode: data?.mode || 'Sequential',
          total_workers: data?.total_workers || 0,
          completed_workers: 0
        }
        pushActivity({ type: 'multi_agent_start', ts: now, mode: data?.mode || 'Sequential', total_workers: data?.total_workers || 0 })
        return
      }

      case 'worker_tasks': {
        const tasks = (data?.tasks || []) as any[]
        multiAgent.value.tasks = tasks.map(t => ({
          task_id: t.task_id || t.id,
          scope_name: t.scope_name || t.scope?.name || '',
          entry_url: t.entry_url || t.scope?.entry_url || '',
          url_patterns: t.url_patterns || t.scope?.url_patterns || [],
          max_iterations: t.max_iterations || 30,
          priority: t.priority || t.scope?.priority || 1
        }))

        for (const task of multiAgent.value.tasks) {
          if (!multiAgent.value.workers.has(task.task_id)) {
            multiAgent.value.workers.set(task.task_id, {
              task_id: task.task_id,
              scope_name: task.scope_name,
              status: 'pending',
              pages_visited: 0,
              apis_discovered: 0,
              elements_interacted: 0,
              iterations_used: 0,
              progress: 0
            })
          }
        }

        pushActivity({ type: 'worker_tasks', ts: now, count: tasks.length })
        return
      }

      case 'worker_progress': {
        const w = data?.worker
        if (!w) return
        multiAgent.value.workers.set(w.task_id, {
          task_id: w.task_id,
          scope_name: w.scope_name,
          status: (w.status || 'running') as any,
          pages_visited: w.pages_visited || 0,
          apis_discovered: w.apis_discovered || 0,
          elements_interacted: w.elements_interacted || 0,
          iterations_used: w.iterations_used || 0,
          progress: w.progress || 0,
          completion_reason: w.completion_reason
        })

        pushActivity({
          type: 'worker_progress',
          ts: now,
          task_id: w.task_id,
          scope_name: w.scope_name,
          status: w.status || 'running',
          pages: w.pages_visited || 0,
          apis: w.apis_discovered || 0,
          progress: w.progress || 0
        })
        return
      }

      case 'worker_complete': {
        pushActivity({
          type: 'worker_complete',
          ts: now,
          task_id: data?.task_id,
          scope_name: data?.scope_name,
          pages: data?.stats?.pages_visited || 0,
          apis: data?.stats?.apis_discovered || 0,
          completion_reason: data?.stats?.completion_reason
        })
        return
      }

      case 'worker_decision': {
        const d = data?.decision
        if (!d) return
        pushActivity({
          type: 'worker_decision',
          ts: now,
          task_id: d.task_id,
          scope_name: d.scope_name,
          iteration: d.iteration,
          page_analysis: d.page_analysis,
          action_type: d.action_type,
          element_index: d.element_index,
          value: d.value,
          reason: d.reason,
          progress: d.progress,
          estimated_apis: d.estimated_apis
        })
        return
      }

      case 'worker_action': {
        const a = data?.action
        if (!a) return
        pushActivity({
          type: 'worker_action',
          ts: now,
          task_id: a.task_id,
          scope_name: a.scope_name,
          iteration: a.iteration,
          action_type: a.action_type,
          element_index: a.element_index,
          value: a.value,
          success: a.success,
          duration_ms: a.duration_ms,
          reason: a.reason
        })
        return
      }

      case 'complete': {
        isVisionActive.value = false
        pushActivity({ type: 'complete', ts: now, status: data?.statistics?.status || 'completed' })
        return
      }

      default:
        return
    }
  }

  const startListening = async () => {
    const unlistenV2 = await listen<VisionV2Envelope>('vision:v2', evt => {
      handleV2(evt.payload)
    })
    unlisteners.push(unlistenV2)
  }

  const stopListening = () => {
    unlisteners.forEach(fn => fn())
    unlisteners.length = 0
  }

  onMounted(() => {
    startListening()
  })

  onUnmounted(() => {
    stopListening()
  })

  const close = () => {
    isVisionActive.value = false
  }

  const hasHistory = computed(() => steps.value.length > 0)

  const open = () => {
    if (hasHistory.value) {
      isVisionActive.value = true
    }
  }

  const activeWorkers = computed(() => Array.from(multiAgent.value.workers.values()).filter(w => w.status === 'running'))

  const completedWorkers = computed(() => Array.from(multiAgent.value.workers.values()).filter(w => w.status === 'completed'))

  const multiAgentProgress = computed(() => {
    if (!multiAgent.value.mode || multiAgent.value.mode.total_workers === 0) return 0
    return (multiAgent.value.mode.completed_workers / multiAgent.value.mode.total_workers) * 100
  })

  return {
    steps,
    coverage,
    discoveredApis,
    isVisionActive,
    currentUrl,
    currentPlan,
    currentProgress,
    showTakeoverForm,
    takeoverMessage,
    takeoverFields,
    loginTimeoutSeconds,
    currentExecutionId,
    hasHistory,
    resetstate,
    close,
    open,
    activity,
    multiAgent,
    isMultiAgentMode,
    activeWorkers,
    completedWorkers,
    multiAgentProgress
  }
}
