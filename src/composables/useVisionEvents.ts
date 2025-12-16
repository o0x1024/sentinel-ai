import { ref, computed, onMounted, onUnmounted, type Ref } from 'vue'
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

interface OrderedMessageChunk {
    execution_id: string
    chunk_type: string
    stage?: string
    structured_data?: any
}

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
            takeoverMessage.value = payload.message || '检测到登录页面，请输入凭证'
            takeoverFields.value = payload.fields || null
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
                    isVisionActive.value = true
                    resetstate() // New session
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
                    takeoverMessage.value = data?.message || '检测到登录页面，请输入凭证'
                    takeoverFields.value = data?.fields || null
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
                }

                if (data?.type === 'complete') {
                    // Auto close on completion as requested
                    isVisionActive.value = false
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
        open
    }
}
