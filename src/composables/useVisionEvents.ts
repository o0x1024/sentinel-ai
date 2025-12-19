import { ref, computed, onMounted, onUnmounted, type Ref, reactive } from 'vue'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'

export interface VisionStep {
    iteration: number
    phase: string // screenshot, analyze, action, verify
    status: string // running, completed, failed
    url?: string
    title?: string
    screenshot?: string // base64
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

// ============================================================================
// Multi-Agent Types
// ============================================================================

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

interface OrderedMessageChunk {
    execution_id: string
    chunk_type: string
    stage?: string
    structured_data?: any
}

// ============================================================================
// Activity Feed (for multi-agent / meta-only runs)
// ============================================================================

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

export function useVisionEvents(executionId?: Ref<string | null>) {
    const steps = ref<VisionStep[]>([])
    const coverage = ref<VisionCoverage | null>(null)
    const discoveredApis = ref<{ method: string; url: string }[]>([])
    const isVisionActive = ref(false)
    const currentUrl = ref('')

    // Planning & Progress State
    const currentPlan = ref<VisionPlan | null>(null)
    const currentProgress = ref<VisionProgress | null>(null)

    // Takeover State
    const showTakeoverForm = ref(false)
    const takeoverMessage = ref('')
    const takeoverFields = ref<any[] | null>(null)
    const currentExecutionId = ref<string | null>(null)

    // Multi-Agent State
    const multiAgent = ref<MultiAgentState>({
        mode: null,
        tasks: [],
        workers: new Map(),
        globalStats: null
    })
    const isMultiAgentMode = computed(() => multiAgent.value.mode?.is_multi_agent ?? false)

    // Activity feed (works even when no vision_step timeline is emitted)
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
        currentExecutionId.value = null
        activity.value = []
        // Reset multi-agent state
        multiAgent.value = {
            mode: null,
            tasks: [],
            workers: new Map(),
            globalStats: null
        }
    }

    const pushActivity = (evt: VisionActivityEvent) => {
        activity.value.push(evt)
        // prevent unbounded growth
        if (activity.value.length > 200) {
            activity.value.splice(0, activity.value.length - 200)
        }
    }

    const startListening = async () => {
        // Listen for coverage updates (Global event)
        const unlistenCoverage = await listen<any>('vision:coverage_update', (event) => {
            const payload = event.payload
            if (executionId?.value && payload.execution_id !== executionId.value) return

            // Auto-activate vision panel on updates
            isVisionActive.value = true
            currentExecutionId.value = payload.execution_id

            coverage.value = payload.coverage
            // Also update api count if needed, though we track specific APIs too
        })
        unlisteners.push(unlistenCoverage)

        // Listen for takeover requests (Global event)
        const unlistenTakeover = await listen<any>('vision:takeover_request', (event) => {
            const payload = event.payload
            if (executionId?.value && payload.execution_id !== executionId.value) return

            console.log('[VisionEvents] Takeover requested:', payload)
            isVisionActive.value = true
            currentExecutionId.value = payload.execution_id
            showTakeoverForm.value = true
            takeoverMessage.value = payload.message || ''
            takeoverFields.value = payload.fields || null
            pushActivity({ type: 'takeover_request', ts: Date.now(), request_type: payload.request_type || 'login' })
        })
        unlisteners.push(unlistenTakeover)

        // Listen for credentials received (Global event)
        const unlistenCreds = await listen<any>('vision:credentials_received', (event) => {
            const payload = event.payload
            console.log('[VisionEvents] Credentials received event:', payload)

            // Less strict filtering - if we have a showTakeoverForm open, close it
            if (executionId?.value && payload.execution_id !== executionId.value) {
                console.log('[VisionEvents] Ignoring credentials event for different execution')
                return
            }

            console.log('[VisionEvents] Hiding takeover form after credentials received')
            showTakeoverForm.value = false
            takeoverMessage.value = ''
            takeoverFields.value = null
        })
        unlisteners.push(unlistenCreds)

        // Listen for multi-agent events
        const unlistenMultiAgent = await listen<any>('vision:multi_agent', (event) => {
            const payload = event.payload
            if (executionId?.value && payload.execution_id !== executionId.value) return

            console.log('[VisionEvents] Multi-agent event:', payload.type, payload)

            switch (payload.type) {
                case 'multi_agent_start':
                    isVisionActive.value = true
                    currentExecutionId.value = payload.execution_id
                    multiAgent.value.mode = {
                        is_multi_agent: true,
                        mode: payload.mode || 'Sequential',
                        total_workers: payload.total_workers || 0,
                        completed_workers: 0
                    }
                    pushActivity({
                        type: 'multi_agent_start',
                        ts: Date.now(),
                        mode: payload.mode || 'Sequential',
                        total_workers: payload.total_workers || 0
                    })
                    break

                case 'worker_tasks':
                    if (payload.tasks && Array.isArray(payload.tasks)) {
                        multiAgent.value.tasks = payload.tasks.map((t: any) => ({
                            task_id: t.task_id || t.id,
                            scope_name: t.scope_name || t.scope?.name || '',
                            entry_url: t.entry_url || t.scope?.entry_url || '',
                            url_patterns: t.url_patterns || t.scope?.url_patterns || [],
                            max_iterations: t.max_iterations || 30,
                            priority: t.priority || t.scope?.priority || 1
                        }))
                        
                        // Don't clear existing workers map to preserve their stats for global summary.
                        // Instead, just ensure the new tasks are in the map.
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
                        pushActivity({ type: 'worker_tasks', ts: Date.now(), count: payload.tasks.length })
                    }
                    break

                case 'worker_progress':
                    if (payload.worker) {
                        const w = payload.worker
                        multiAgent.value.workers.set(w.task_id, {
                            task_id: w.task_id,
                            scope_name: w.scope_name,
                            status: w.status || 'running',
                            pages_visited: w.pages_visited || 0,
                            apis_discovered: w.apis_discovered || 0,
                            elements_interacted: w.elements_interacted || 0,
                            iterations_used: w.iterations_used || 0,
                            progress: w.progress || 0,
                            completion_reason: w.completion_reason
                        })

                        // Update global stats and the Progress tiles during multi-agent runs
                        const workers = Array.from(multiAgent.value.workers.values())
                        const totalPages = workers.reduce((sum, x) => sum + (x.pages_visited || 0), 0)
                        const totalApis = workers.reduce((sum, x) => sum + (x.apis_discovered || 0), 0)
                        const totalElements = workers.reduce((sum, x) => sum + (x.elements_interacted || 0), 0)
                        multiAgent.value.globalStats = {
                            total_urls_visited: totalPages,
                            total_apis_discovered: totalApis,
                            workers_completed: workers.filter(x => x.status === 'completed').length,
                            total_elements_interacted: totalElements
                        }

                        const task = multiAgent.value.tasks.find(t => t.task_id === w.task_id)
                        currentProgress.value = {
                            phase: w.scope_name || '',
                            iteration: w.iterations_used || 0,
                            max_iterations: task?.max_iterations || 100,
                            pages_visited: w.pages_visited || 0,
                            apis_discovered: w.apis_discovered || 0,
                            elements_interacted: w.elements_interacted || 0
                        }

                        pushActivity({
                            type: 'worker_progress',
                            ts: Date.now(),
                            task_id: w.task_id,
                            scope_name: w.scope_name,
                            status: w.status || 'running',
                            pages: w.pages_visited || 0,
                            apis: w.apis_discovered || 0,
                            progress: w.progress || 0
                        })
                    }
                    break

                case 'worker_complete':
                    if (payload.task_id) {
                        const existing = multiAgent.value.workers.get(payload.task_id)
                        if (existing) {
                            existing.status = 'completed'
                            existing.completion_reason = payload.stats?.completion_reason
                            if (payload.stats) {
                                existing.pages_visited = payload.stats.pages_visited || existing.pages_visited
                                existing.apis_discovered = payload.stats.apis_discovered || existing.apis_discovered
                                existing.elements_interacted = payload.stats.elements_interacted || existing.elements_interacted
                            }
                            existing.progress = 100
                        }
                        // Update completed count
                        if (multiAgent.value.mode) {
                            multiAgent.value.mode.completed_workers += 1
                        }
                        pushActivity({
                            type: 'worker_complete',
                            ts: Date.now(),
                            task_id: payload.task_id,
                            scope_name: payload.scope_name || existing?.scope_name || '',
                            pages: payload.stats?.pages_visited || existing?.pages_visited || 0,
                            apis: payload.stats?.apis_discovered || existing?.apis_discovered || 0,
                            completion_reason: payload.stats?.completion_reason
                        })
                    }
                    break

                case 'worker_decision':
                    if (payload.decision) {
                        const d = payload.decision
                        pushActivity({
                            type: 'worker_decision',
                            ts: Date.now(),
                            task_id: d.task_id,
                            scope_name: d.scope_name,
                            iteration: d.iteration || 0,
                            page_analysis: d.page_analysis || '',
                            action_type: d.action_type || '',
                            element_index: d.element_index,
                            value: d.value,
                            reason: d.reason || '',
                            progress: d.progress || 0,
                            estimated_apis: d.estimated_apis
                        })
                    }
                    break

                case 'worker_action':
                    if (payload.action) {
                        const a = payload.action
                        pushActivity({
                            type: 'worker_action',
                            ts: Date.now(),
                            task_id: a.task_id,
                            scope_name: a.scope_name,
                            iteration: a.iteration || 0,
                            action_type: a.action_type || '',
                            element_index: a.element_index,
                            value: a.value,
                            success: !!a.success,
                            duration_ms: a.duration_ms,
                            reason: a.reason || ''
                        })
                    }
                    break

                case 'multi_agent_stats':
                    if (payload.mode_info) {
                        multiAgent.value.mode = {
                            is_multi_agent: payload.mode_info.is_multi_agent ?? true,
                            mode: payload.mode_info.mode || 'Sequential',
                            total_workers: payload.mode_info.total_workers || 0,
                            completed_workers: payload.mode_info.completed_workers || 0
                        }
                    }
                    if (payload.global_stats) {
                        multiAgent.value.globalStats = {
                            total_urls_visited: payload.global_stats.total_urls_visited || 0,
                            total_apis_discovered: payload.global_stats.total_apis_discovered || 0,
                            workers_completed: payload.global_stats.workers_completed || 0,
                            total_elements_interacted: payload.global_stats.total_elements_interacted || 0
                        }
                    }
                    break
            }
        })
        unlisteners.push(unlistenMultiAgent)

        // Listen for message chunks (Meta events for steps)
        const unlistenChunk = await listen<OrderedMessageChunk>('message_chunk', (event) => {
            const chunk = event.payload
            if (executionId?.value && chunk.execution_id !== executionId.value) return

            if (chunk.chunk_type === 'Meta' || chunk.chunk_type === 'StreamComplete') {
                const data = chunk.structured_data

                // Track execution ID from meta events
                if (chunk.execution_id) {
                    currentExecutionId.value = chunk.execution_id
                }

                if (data?.type === 'start') {
                    resetstate() // New session
                    isVisionActive.value = true
                    currentExecutionId.value = chunk.execution_id
                    if (data.target_url) currentUrl.value = data.target_url
                }

                if (data?.type === 'vision_step') {
                    isVisionActive.value = true
                    const step = data.step as VisionStep

                    // Check if we should update an existing step (same iteration & phase) or add new
                    // Usually steps come in sequence: screenshot -> analyze -> action
                    // We can just append them as a timeline
                    steps.value.push(step)

                    if (step.url) currentUrl.value = step.url
                }

                // Handle takeover/credentials events from Meta channel (fallback/redundancy)
                // Some environments may miss the direct tauri event, so we also support:
                // - data.type === 'takeover_request' / 'credentials_received'
                // - chunk.stage === 'takeover_request' / 'credentials_received'
                const metaType = data?.type || chunk.stage
                if (metaType === 'takeover_request') {
                    console.log('[VisionEvents] Takeover requested (Meta):', data)
                    showTakeoverForm.value = true
                    takeoverMessage.value = data?.message || ''
                    takeoverFields.value = data?.fields || null
                    pushActivity({ type: 'takeover_request', ts: Date.now(), request_type: data?.request_type || 'login' })
                }
                if (metaType === 'credentials_received') {
                    console.log('[VisionEvents] Credentials received (Meta):', data)
                    showTakeoverForm.value = false
                    takeoverMessage.value = ''
                    takeoverFields.value = null
                }

                if (data?.type === 'api_discovered') {
                    const api = { method: data.method, url: data.api }
                    // Avoid duplicates
                    if (!discoveredApis.value.some(a => a.method === api.method && a.url === api.url)) {
                        discoveredApis.value.push(api)
                    }
                    pushActivity({ type: 'api_discovered', ts: Date.now(), method: data.method, url: data.api })
                }

                // Handle vision_plan events
                if (data?.type === 'vision_plan') {
                    isVisionActive.value = true
                    currentPlan.value = {
                        phase: data.phase || '',
                        phase_name: data.phase_name || '',
                        goal: data.goal || '',
                        steps: data.steps || [],
                        completion_criteria: data.completion_criteria || '',
                        reason: data.reason || ''
                    }
                    pushActivity({ type: 'vision_plan', ts: Date.now(), phase_name: data.phase_name || '', phase: data.phase || '' })
                }

                // Handle vision_progress events
                if (data?.type === 'vision_progress') {
                    isVisionActive.value = true
                    currentProgress.value = {
                        phase: data.phase || '',
                        iteration: data.iteration || 0,
                        max_iterations: data.max_iterations || 100,
                        pages_visited: data.pages_visited || 0,
                        apis_discovered: data.apis_discovered || 0,
                        elements_interacted: data.elements_interacted || 0
                    }
                    pushActivity({
                        type: 'vision_progress',
                        ts: Date.now(),
                        phase: data.phase || '',
                        iteration: data.iteration || 0,
                        max_iterations: data.max_iterations || 100
                    })
                }

                if (data?.type === 'complete') {
                    // Auto close on completion as requested
                    isVisionActive.value = false
                    pushActivity({ type: 'complete', ts: Date.now(), status: data.statistics?.status || 'completed' })
                }
            }
        })
        unlisteners.push(unlistenChunk)
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

    // Check if there's history to view
    const hasHistory = computed(() => steps.value.length > 0)

    // Open/reopen the panel (for viewing history)
    const open = () => {
        if (hasHistory.value) {
            isVisionActive.value = true
        }
    }

    // Computed helpers for multi-agent
    const activeWorkers = computed(() => 
        Array.from(multiAgent.value.workers.values()).filter(w => w.status === 'running')
    )
    
    const completedWorkers = computed(() => 
        Array.from(multiAgent.value.workers.values()).filter(w => w.status === 'completed')
    )

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
        currentExecutionId,
        hasHistory,
        resetstate,
        close,
        open,
        activity,
        // Multi-Agent exports
        multiAgent,
        isMultiAgentMode,
        activeWorkers,
        completedWorkers,
        multiAgentProgress
    }
}
