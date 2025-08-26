<template>
  <div class="collapse collapse-arrow bg-base-100 border border-base-200 rounded-lg"
       :class="{ 
         'border-primary border-2': step.status === 'executing' || step.status === 'running'
       }">
    <input type="checkbox" class="collapse-checkbox" :id="`step-${step.id}`" />
    <div class="collapse-title font-medium text-sm py-3 px-4 hover:bg-base-200/50 transition-colors">
      <div class="flex items-center gap-3">
        <div class="badge badge-primary badge-sm">{{ index + 1 }}</div>
        <div class="flex-1 min-w-0">
          <div class="flex items-center gap-2">
            <span class="truncate">{{ step.name || step.description }}</span>
            <StatusIcon :status="step.status" />
          </div>
          
          <!-- Progress indicator for executing steps -->
          <div v-if="step.status === 'executing' || step.status === 'running'" class="mt-2">
            <div class="flex items-center gap-2 text-xs">
              <span class="text-base-content/60">执行中...</span>
              <div class="flex-1 h-1 bg-base-200 rounded-full">
                <div class="h-full bg-primary rounded-full transition-all duration-500" 
                     style="width: 100%; animation: progress-pulse 1.5s ease-in-out infinite;"></div>
              </div>
            </div>
          </div>
          
          <div v-if="step.description && step.name !== step.description" 
               class="text-xs text-base-content/60 mt-1 truncate">{{ step.description }}</div>
        </div>
      </div>
    </div>
    
    <div class="collapse-content px-4 pb-4">
      <StepDetails :step="step" />
    </div>
  </div>
</template>

<script setup lang="ts">
import StatusIcon from './StatusIcon.vue'
import StepDetails from './StepDetails.vue'

const props = defineProps<{
  step: any
  index: number
}>()
</script>