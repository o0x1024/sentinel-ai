import { ref, onMounted, onUnmounted, type Ref } from 'vue'
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

    const unlisteners: UnlistenFn[] = []

    const resetstate = () => {
        steps.value = []
        coverage.value = null
        discoveredApis.value = []
        isVisionActive.value = false
        currentUrl.value = ''
    }

    const startListening = async () => {
        // Listen for coverage updates (Global event)
        const unlistenCoverage = await listen<any>('vision:coverage_update', (event) => {
            const payload = event.payload
            if (executionId?.value && payload.execution_id !== executionId.value) return

            coverage.value = payload.coverage
            // Also update api count if needed, though we track specific APIs too
        })
        unlisteners.push(unlistenCoverage)

        // Listen for message chunks (Meta events for steps)
        const unlistenChunk = await listen<OrderedMessageChunk>('message_chunk', (event) => {
            const chunk = event.payload
            if (executionId?.value && chunk.execution_id !== executionId.value) return

            if (chunk.chunk_type === 'Meta') {
                const data = chunk.structured_data

                if (data?.type === 'start') {
                    isVisionActive.value = true
                    resetstate() // New session
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

                if (data?.type === 'api_discovered') {
                    const api = { method: data.method, url: data.api }
                    // Avoid duplicates
                    if (!discoveredApis.value.some(a => a.method === api.method && a.url === api.url)) {
                        discoveredApis.value.push(api)
                    }
                }

                if (data?.type === 'complete') {
                    // Keep active for viewing results
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

    return {
        steps,
        coverage,
        discoveredApis,
        isVisionActive,
        currentUrl,
        resetstate,
        close
    }
}
