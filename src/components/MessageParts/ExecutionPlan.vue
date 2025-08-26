<template>
  <div class="mt-3 ">
    <div class="bg-base-200/60 rounded-lg border border-base-300/50 overflow-hidden">
      <div class="p-3 border-b border-base-300/50 bg-base-100/50">
        <div class="flex items-center gap-2">
          <svg class="w-4 h-4 text-accent" fill="currentColor" viewBox="0 0 20 20">
            <path d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z"/>
          </svg>
          <span class="font-medium text-sm">{{ t('aiAssistant.executionPlan', '执行计划') }}</span>
          <div class="badge badge-accent badge-outline badge-xs ml-auto">
            {{ executionPlan.steps?.length || 0 }} 步骤
          </div>
        </div>
        
        <!-- Progress bar -->
        <div v-if="isStreaming || executionProgress !== undefined" 
             class="flex items-center gap-2 mt-2">
          <div class="text-xs text-base-content/60">进度:</div>
          <div class="flex-1 h-1.5 bg-base-300 rounded-full overflow-hidden">
            <div class="h-full bg-gradient-to-r from-primary to-accent transition-all duration-500 ease-out" 
                 :style="{ width: `${executionProgress || 0}%` }"
                 :class="{ 'animate-pulse': isStreaming && (executionProgress || 0) < 100 }"></div>
          </div>
          <div class="text-xs text-base-content/60 min-w-max font-mono">
            {{ Math.round(executionProgress || 0) }}%
          </div>
        </div>
        
        <!-- Current step indicator -->
        <div v-if="isStreaming && currentStep" 
             class="flex items-center gap-2 mt-2 text-xs text-base-content/70 bg-primary/5 px-2 py-1 rounded">
          <svg class="w-3 h-3 text-primary animate-spin" fill="currentColor" viewBox="0 0 20 20">
            <path d="M4 2a2 2 0 00-2 2v12a2 2 0 002 2h12a2 2 0 002-2V4a2 2 0 00-2-2H4z"/>
          </svg>
          <span>正在执行: {{ currentStep }}</span>
        </div>
      </div>
      
      <!-- Steps list -->
      <div class="space-y-1 p-2">
        <ExecutionStep
          v-for="(step, index) in executionPlan.steps" 
          :key="step.id"
          :step="step"
          :index="index"
        />
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import ExecutionStep from './ExecutionStep.vue'

const props = defineProps<{
  executionPlan: {
    steps?: any[]
    name?: string
  }
  executionProgress?: number
  currentStep?: string
  isStreaming?: boolean
}>()

const { t } = useI18n()
</script>