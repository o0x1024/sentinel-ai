<template>
  <div class="space-y-4">
    <!-- Basic information -->
    <div class="card bg-base-200">
      <div class="card-body">
        <h4 class="card-title text-base mb-3">基本信息</h4>
        <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
          <div>
            <span class="font-semibold">步骤名称:</span>
            <span class="ml-2">{{ step.name || step.description }}</span>
          </div>
          <div>
            <span class="font-semibold">状态:</span>
            <span class="ml-2 badge badge-xs" :class="getStepStatusClass(step.status)">
              {{ getStepStatusText(step.status) }}
            </span>
          </div>
          <div>
            <span class="font-semibold">开始时间:</span>
            <span class="ml-2">{{ formatTimestamp(step.started_at) }}</span>
          </div>
          <div>
            <span class="font-semibold">完成时间:</span>
            <span class="ml-2">{{ formatTimestamp(step.completed_at) }}</span>
          </div>
          <div v-if="step.started_at && step.completed_at" class="col-span-2">
            <span class="font-semibold">执行耗时:</span>
            <span class="ml-2">{{ Math.round((step.completed_at - step.started_at) / 1000) }}秒</span>
          </div>
        </div>
      </div>
    </div>
    
    <!-- Description -->
    <div v-if="step.description" class="card bg-base-200">
      <div class="card-body">
        <h4 class="card-title text-base mb-3">步骤描述</h4>
        <p class="text-sm">{{ step.description }}</p>
      </div>
    </div>
    
    <!-- Execution result -->
    <div v-if="step.result || step.result_data" class="card bg-base-200">
      <div class="card-body">
        <h4 class="card-title text-base mb-3">执行结果</h4>
        <pre class="bg-base-100 p-4 rounded text-sm overflow-x-auto max-h-60">{{ JSON.stringify(step.result_data || step.result, null, 2) }}</pre>
      </div>
    </div>
    
    <!-- Error information -->
    <div v-if="step.error" class="card bg-error/10">
      <div class="card-body">
        <h4 class="card-title text-base mb-3 text-error">错误信息</h4>
        <pre class="bg-base-100 p-4 rounded text-sm overflow-x-auto text-error">{{ step.error }}</pre>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { useMessageUtils } from '../../composables/useMessageUtils'

const props = defineProps<{
  step: any
}>()

const { getStepStatusClass, getStepStatusText, formatTimestamp } = useMessageUtils()
</script>