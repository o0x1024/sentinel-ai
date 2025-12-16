<template>
  <div class="vision-panel border border-base-300 rounded-lg bg-base-200 flex flex-col h-full min-h-0 overflow-hidden" v-if="isActive">
    <!-- Header -->
    <div class="vision-header p-3 border-b border-base-300 flex justify-between items-center bg-base-100">
      <div class="flex items-center gap-2">
        <span class="text-lg">👁️</span>
        <span class="font-bold text-sm">Vision Explorer</span>
      </div>
      <div class="flex gap-2 items-center">
         <div class="badge badge-sm badge-info gap-1" v-if="discoveredApis.length > 0">
            API: {{ discoveredApis.length }}
         </div>
         <button @click="$emit('close')" class="btn btn-ghost btn-xs btn-circle" title="Close Panel">
            ✕
         </button>
      </div>
    </div>

    <!-- Login Takeover Form - only show if not yet submitted in this session -->
    <div v-if="showTakeoverForm && !credentialsSubmitted" class="p-3 bg-warning/10 border-b border-warning/30">
      <div class="flex items-center gap-2 text-sm text-warning font-medium mb-2">
        <i class="fas fa-key"></i>
        <span>{{ takeoverMessage || t('agent.loginPageDetected') }}</span>
      </div>
      
      <div class="space-y-2">
          <!-- Dynamic Fields -->
          <template v-if="takeoverFields && takeoverFields.length > 0">
              <div v-for="field in takeoverFields" :key="field.id">
                  <input
                    v-model="credentials[field.id]"
                    :type="field.field_type"
                    :placeholder="field.placeholder || field.label"
                    class="input input-sm input-bordered w-full text-xs"
                    @keyup.enter="submitCredentials"
                  />
              </div>
          </template>
          
          <!-- Fallback Fields (if no dynamic fields provided) -->
          <template v-else>
              <input
                v-model="credentials.username"
                type="text"
                :placeholder="t('agent.usernameAccount')"
                class="input input-sm input-bordered w-full text-xs"
                @keyup.enter="submitCredentials"
              />
              <input
                v-model="credentials.password"
                type="password"
                :placeholder="t('agent.password')"
                class="input input-sm input-bordered w-full text-xs"
                @keyup.enter="submitCredentials"
              />
              <input
                  v-model="credentials.verificationCode"
                  type="text"
                  :placeholder="t('agent.verificationCodeOptional')"
                  class="input input-sm input-bordered w-full text-xs"
                  @keyup.enter="submitCredentials"
              />
          </template>

          <!-- Actions -->
          <div class="flex gap-2 mt-2">
            <button
              class="btn btn-sm btn-warning flex-1"
              :disabled="!canSubmit || isSubmittingCredentials || isSkippingLogin"
              @click="submitCredentials"
            >
              <span v-if="isSubmittingCredentials" class="loading loading-spinner loading-xs"></span>
              <span v-else>{{ t('agent.continueExploration') }}</span>
            </button>

            <button
              class="btn btn-sm btn-ghost flex-1"
              :disabled="isSubmittingCredentials || isSkippingLogin"
              @click="skipLogin"
            >
              <span v-if="isSkippingLogin" class="loading loading-spinner loading-xs"></span>
              <span v-else>{{ t('agent.skipLogin') }}</span>
            </button>
          </div>
        </div>
    </div>

    <!-- Plan & Progress Section -->
    <div class="p-3 bg-base-100/50 text-xs border-b border-base-300 flex flex-col gap-3" v-if="currentPlan || currentProgress">
      <!-- Current Plan -->
      <div v-if="currentPlan" class="plan-section">
        <div class="flex items-center gap-2 mb-2">
          <span class="text-primary font-bold">📋</span>
          <span class="font-bold text-primary">{{ currentPlan.phase_name || currentPlan.phase }}</span>
        </div>
        <div class="pl-5 space-y-1 text-base-content/80">
          <div><span class="opacity-60">{{ t('agent.visionGoal') }}:</span> {{ currentPlan.goal }}</div>
          <div v-if="currentPlan.steps && currentPlan.steps.length > 0">
            <span class="opacity-60">{{ t('agent.visionSteps') }}:</span>
            <ul class="list-disc list-inside pl-2 mt-1 space-y-0.5">
              <li v-for="(step, idx) in currentPlan.steps" :key="idx" class="truncate" :title="step">{{ step }}</li>
            </ul>
          </div>
          <div v-if="currentPlan.completion_criteria">
            <span class="opacity-60">{{ t('agent.visionCompletion') }}:</span> {{ currentPlan.completion_criteria }}
          </div>
        </div>
      </div>

      <!-- Current Progress -->
      <div v-if="currentProgress" class="progress-section">
        <div class="flex items-center gap-2 mb-2">
          <span class="text-secondary font-bold">📊</span>
          <span class="font-bold text-secondary">{{ t('agent.visionProgress') }}</span>
          <span class="ml-auto badge badge-sm badge-secondary">{{ currentProgress.iteration }}/{{ currentProgress.max_iterations }}</span>
        </div>
        <div class="grid grid-cols-3 gap-2 text-center">
          <div class="bg-base-200 rounded p-2">
            <div class="text-lg font-bold text-primary">{{ currentProgress.pages_visited }}</div>
            <div class="text-[10px] opacity-60">{{ t('agent.visionPages') }}</div>
          </div>
          <div class="bg-base-200 rounded p-2">
            <div class="text-lg font-bold text-accent">{{ currentProgress.apis_discovered }}</div>
            <div class="text-[10px] opacity-60">APIs</div>
          </div>
          <div class="bg-base-200 rounded p-2">
            <div class="text-lg font-bold text-info">{{ currentProgress.elements_interacted }}</div>
            <div class="text-[10px] opacity-60">{{ t('agent.visionElements') }}</div>
          </div>
        </div>
      </div>
    </div>

    <!-- Coverage Stats -->
    <div class="p-3 bg-base-100/50 text-xs border-b border-base-300 flex flex-col gap-2" v-if="coverage">
      <div class="flex justify-between items-center">
         <span class="opacity-70">Target:</span>
         <span class="font-mono truncate max-w-[200px]" :title="currentUrl">{{ currentUrl }}</span>
      </div>
      
      <!-- Route Coverage -->
      <div class="flex items-center gap-2" title="Route Coverage">
        <span class="w-16 opacity-70">Routes</span>
        <progress class="progress progress-primary w-full" :value="coverage.route_coverage" max="100"></progress>
        <span class="w-8 text-right">{{ coverage.route_coverage.toFixed(0) }}%</span>
      </div>

      <!-- Element Coverage -->
      <div class="flex items-center gap-2" title="Element Coverage">
        <span class="w-16 opacity-70">Elements</span>
        <progress class="progress progress-secondary w-full" :value="coverage.element_coverage" max="100"></progress>
        <span class="w-8 text-right">{{ coverage.element_coverage.toFixed(0) }}%</span>
      </div>
    </div>

    <!-- Timeline Steps -->
    <div class="steps-container flex-1 overflow-y-auto p-3 flex flex-col gap-3 scroll-smooth" ref="stepsContainer">
       <div v-if="steps.length === 0" class="text-center text-xs opacity-50 py-4">
         Waiting for events...
       </div>

       <div v-for="(step, idx) in steps" :key="idx" class="vision-step flex gap-3 text-xs">
          <!-- Icon Column -->
          <div class="flex flex-col items-center">
             <div class="w-6 h-6 rounded-full flex items-center justify-center text-xs border bg-base-100 z-10"
                :class="{
                    'border-primary text-primary': step.phase === 'action',
                    'border-secondary text-secondary': step.phase === 'analyze',
                    'border-accent text-accent': step.phase === 'screenshot',
                    'border-error text-error': step.error
                }">
                <span v-if="step.error">❌</span>
                <span v-else-if="step.phase === 'screenshot'">📸</span>
                <span v-else-if="step.phase === 'analyze'">🧠</span>
                <span v-else-if="step.phase === 'action'">⚡</span>
             </div>
             <div class="w-0.5 h-full bg-base-300 -mt-1" v-if="idx < steps.length - 1"></div>
          </div>

          <!-- Content Column -->
          <div class="flex-1 pb-4 min-w-0">
             <div class="flex justify-between mb-1 opacity-60 text-[10px]">
                <span>Iteration {{ step.iteration }}</span>
                <span class="uppercase">{{ step.phase }}</span>
             </div>

             <!-- Screenshot -->
             <div v-if="step.screenshot" class="mb-2">
                <img :src="'data:image/png;base64,' + step.screenshot" class="rounded border border-base-300 shadow-sm w-full max-h-[150px] object-cover hover:object-contain transition-all bg-base-300" />
                <div class="text-[10px] mt-1 opacity-70 truncate">{{ step.title }}</div>
             </div>

             <!-- Analysis -->
             <div v-if="step.analysis" class="bg-base-100 p-2 rounded border border-base-300">
                <p class="mb-1">{{ step.analysis.page_analysis }}</p>
                <div v-if="step.analysis.estimated_apis && step.analysis.estimated_apis.length" class="mt-1 pt-1 border-t border-base-200">
                    <div class="text-[10px] opacity-70">Estimated APIs:</div>
                    <div class="flex flex-wrap gap-1 mt-0.5">
                        <span v-for="api in step.analysis.estimated_apis" :key="api" class="badge badge-xs badge-ghost max-w-full truncate">
                            {{ api }}
                        </span>
                    </div>
                </div>
             </div>

             <!-- Action -->
             <div v-if="step.action" class="bg-base-100 p-2 rounded border border-base-300" :class="{'border-error/50 bg-error/5': !step.action.success}">
                 <div class="font-bold mb-1">{{ step.action.action_type }}</div>
                 <div v-if="step.action.element_index !== undefined" class="mb-0.5 opacity-80">
                    Target: Index [{{ step.action.element_index }}]
                 </div>
                 <div v-if="step.action.value" class="mb-0.5 break-all">
                    Value: <span class="font-mono bg-base-200 px-1 rounded">{{ step.action.value }}</span>
                 </div>
                 <div class="text-[10px] italic opacity-60 mt-1">
                    "{{ step.action.reason }}"
                 </div>
             </div>

             <!-- Error -->
             <div v-if="step.error" class="bg-error/10 text-error p-2 rounded border border-error/20">
                {{ step.error }}
             </div>
          </div>
       </div>
    </div>

    <!-- User Message Input (sticky bottom) -->
    <div class="mt-auto p-3 border-t border-base-300 bg-base-100/70">
      <div class="relative">
        <textarea
          v-model="userMessage"
          class="textarea textarea-bordered w-full text-xs leading-5 min-h-[2.75rem] max-h-28 resize-none pr-20"
          :placeholder="t('agent.visionMessagePlaceholder')"
          @keydown="onUserMessageKeydown"
        />

        <!-- Inline buttons inside textarea -->
        <div class="absolute right-2 bottom-2 flex items-center gap-1">
          <!-- <button
            class="btn btn-xs btn-ghost"
            :title="t('agent.stop')"
            :disabled="isStopping"
            @click="stopVision"
          >
            <span v-if="isStopping" class="loading loading-spinner loading-xs"></span>
            <i v-else class="fas fa-stop"></i>
          </button> -->

          <button
            class="btn btn-xs btn-primary"
            :title="t('agent.send')"
            :disabled="!canSendMessage || isSendingMessage"
            @click="sendUserMessage"
          >
            <span v-if="isSendingMessage" class="loading loading-spinner loading-xs"></span>
            <i v-else class="fas fa-paper-plane"></i>
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, watch, nextTick, computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { invoke } from '@tauri-apps/api/core'
import type { VisionStep, VisionCoverage, VisionPlan, VisionProgress } from '@/composables/useVisionEvents'

const { t } = useI18n()

const props = defineProps<{
  steps: VisionStep[]
  coverage: VisionCoverage | null
  discoveredApis: { method: string; url: string }[]
  isActive: boolean
  currentUrl: string
  // Plan & Progress props
  currentPlan?: VisionPlan | null
  currentProgress?: VisionProgress | null
  // Takeover props
  showTakeoverForm?: boolean
  takeoverMessage?: string
  takeoverFields?: any[]
  executionId?: string | null
}>()

defineEmits<{
  (e: 'close'): void
}>()

const stepsContainer = ref<HTMLElement | null>(null)

// Takeover form state
const isSubmittingCredentials = ref(false)
const isSkippingLogin = ref(false)
const credentials = ref<Record<string, string>>({})
const credentialsSubmitted = ref(false) // Track if credentials were submitted in this session

// User message state
const userMessage = ref('')
const isSendingMessage = ref(false)
const isStopping = ref(false)

// Reset credentialsSubmitted when a new takeover request comes in.
// Note: `showTakeoverForm` may stay true across retries; we also reset when message/fields change.
watch(
  () => [props.showTakeoverForm, props.takeoverMessage, props.takeoverFields?.length],
  ([show]) => {
    if (show) credentialsSubmitted.value = false
  }
)

// Initialize credentials when fields change
watch(() => props.takeoverFields, (fields) => {
    const newCreds: Record<string, string> = {}
    if (fields) {
        fields.forEach(f => {
            newCreds[f.id] = ''
        })
    } else {
        // Fallback init
        newCreds.username = ''
        newCreds.password = ''
        newCreds.verificationCode = ''
    }
    credentials.value = newCreds
}, { immediate: true })

const canSubmit = computed(() => {
    if (props.takeoverFields && props.takeoverFields.length > 0) {
        // Check required fields
        return props.takeoverFields.every(f => !f.required || !!credentials.value[f.id])
    }
    return !!credentials.value.username && !!credentials.value.password
})

const canSendMessage = computed(() => {
  return !!(props.executionId && userMessage.value.trim().length > 0)
})

const onUserMessageKeydown = async (e: KeyboardEvent) => {
  if (e.key !== 'Enter') return
  if (e.shiftKey) return
  e.preventDefault()
  await sendUserMessage()
}

const sendUserMessage = async () => {
  const message = userMessage.value.trim()
  if (!message) return
  const eid = props.executionId
  if (!eid) {
    console.warn('[VisionPanel] Missing executionId for sending message')
    return
  }

  isSendingMessage.value = true
  try {
    await invoke('vision_explorer_send_user_message', {
      executionId: eid,
      message
    })
    userMessage.value = ''
    console.log('[VisionPanel] User message sent')
  } catch (error) {
    console.error('[VisionPanel] Failed to send user message:', error)
  } finally {
    isSendingMessage.value = false
  }
}

const stopVision = async () => {
  const eid = props.executionId
  if (!eid) {
    console.warn('[VisionPanel] Missing executionId for stop')
    return
  }
  isStopping.value = true
  try {
    await invoke('cancel_ai_stream', { conversationId: eid })
    console.log('[VisionPanel] Stop command sent')
  } catch (error) {
    console.error('[VisionPanel] Failed to stop:', error)
  } finally {
    isStopping.value = false
  }
}

// Skip login and continue exploration without credentials
const skipLogin = async () => {
  const eid = props.executionId || (window as any).__visionExecutionId
  if (!eid) {
    console.warn('[VisionPanel] Missing executionId for skip login')
    return
  }

  isSkippingLogin.value = true
  try {
    await invoke('vision_explorer_skip_login', { executionId: eid })
    credentialsSubmitted.value = true
    console.log('[VisionPanel] Skip login requested')
  } catch (error) {
    console.error('[VisionPanel] Failed to skip login:', error)
  } finally {
    isSkippingLogin.value = false
  }
}

// Submit credentials to backend
const submitCredentials = async () => {
  if (!canSubmit.value) return
  
  isSubmittingCredentials.value = true
  try {
    const eid = props.executionId || (window as any).__visionExecutionId
    
    if (!eid) {
      console.warn('No execution ID available for credential submission')
      return
    }
    
    // Map credentials to backend expected format
    let username = ''
    let password = ''
    let verificationCode: string | null = null
    let extraFields: Record<string, string> | null = null
    
    if (props.takeoverFields && props.takeoverFields.length > 0) {
        // Dynamic mapping
        const creds = credentials.value
        const extras: Record<string, string> = {}
        
        // Find standard fields by ID convention or fallback
        // We expect backend to send ids: "username", "password", "verification_code" for standard ones
        
        if (creds['username']) username = creds['username']
        if (creds['password']) password = creds['password']
        if (creds['verification_code']) verificationCode = creds['verification_code']
        
        // Put everything else or duplicates into extraFields
        Object.entries(creds).forEach(([key, val]) => {
            if (key !== 'username' && key !== 'password' && key !== 'verification_code') {
                extras[key] = val
            }
        })
        
        if (Object.keys(extras).length > 0) {
            extraFields = extras
        }
    } else {
        // Fallback mapping
        username = credentials.value.username
        password = credentials.value.password
        verificationCode = credentials.value.verificationCode || null
    }
    
    await invoke('vision_explorer_receive_credentials', {
      executionId: eid,
      username,
      password,
      verificationCode,
      extraFields
    })
    console.log('[VisionPanel] Credentials submitted successfully')
    
    // Mark as submitted to immediately hide the form
    credentialsSubmitted.value = true
    
    // Reset
    const newCreds: Record<string, string> = {}
    if (props.takeoverFields) {
        props.takeoverFields.forEach(f => newCreds[f.id] = '')
    } else {
        newCreds.username = ''
        newCreds.password = ''
    }
    credentials.value = newCreds
    
  } catch (error) {
    console.error('Failed to submit credentials:', error)
  } finally {
    isSubmittingCredentials.value = false
  }
}

// Auto-scroll to bottom when steps change
watch(() => props.steps.length, async () => {
    await nextTick()
    if (stepsContainer.value) {
        stepsContainer.value.scrollTop = stepsContainer.value.scrollHeight
    }
})
</script>

<style scoped>
.vision-step:last-child .w-0\.5 {
  display: none;
}
</style>
