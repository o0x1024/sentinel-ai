<template>
  <div class="vision-panel border border-base-300 rounded-lg bg-base-200 flex flex-col h-full max-h-[600px] overflow-hidden" v-if="isActive">
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
        <span>{{ takeoverMessage || '检测到登录页面，请输入凭证' }}</span>
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
              <input
                  v-model="credentials.verificationCode"
                  type="text"
                  placeholder="验证码（可选）"
                  class="input input-sm input-bordered w-full text-xs"
                  @keyup.enter="submitCredentials"
              />
          </template>

          <button
              class="btn btn-sm btn-warning w-full mt-2"
              :disabled="!canSubmit || isSubmittingCredentials"
              @click="submitCredentials"
            >
              <span v-if="isSubmittingCredentials" class="loading loading-spinner loading-xs"></span>
              <span v-else>继续探索</span>
          </button>
        </div>
    </div>

    <!-- Coverage Stats -->
    <div class="p-3 bg-base-100/50 text-xs border-b border-base-300 flex flex-col gap-2" v-if="coverage">
      <div class="flex justify-between items-center">
         <span class="opacity-70">Target:</span>
         <span class="font-mono truncate max-w-[150px]" :title="currentUrl">{{ currentUrl }}</span>
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
  </div>
</template>

<script setup lang="ts">
import { ref, watch, nextTick, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { VisionStep, VisionCoverage } from '@/composables/useVisionEvents'

const props = defineProps<{
  steps: VisionStep[]
  coverage: VisionCoverage | null
  discoveredApis: { method: string; url: string }[]
  isActive: boolean
  currentUrl: string
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
const credentials = ref<Record<string, string>>({})
const credentialsSubmitted = ref(false) // Track if credentials were submitted in this session

// Reset credentialsSubmitted when a new takeover request comes in
watch(() => props.showTakeoverForm, (newVal) => {
    if (newVal) {
        // New takeover request, reset submitted state
        credentialsSubmitted.value = false
    }
})

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
