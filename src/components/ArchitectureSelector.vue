<template>
  <div class="architecture-selector">
    <!-- 架构信息卡片（统一使用 ReAct 泛化引擎） -->
    <div class="card bg-base-100 shadow-xl">
      <div class="card-body">
        <h3 class="card-title mb-4">执行引擎</h3>
        
        <div class="max-w-lg">
          <!-- ReAct 泛化引擎 -->
          <div class="card border-2 border-primary bg-primary/10">
            <div class="card-body p-4">
              <div class="flex items-center gap-2 mb-2">
                <div class="w-3 h-3 bg-green-500 rounded"></div>
                <h4 class="font-semibold">ReAct 泛化引擎</h4>
                <div class="badge badge-primary badge-xs">统一引擎</div>
              </div>
              <p class="text-sm text-base-content/70 mb-3">
                统一的推理与行动架构，支持所有任务类型。任务特性通过 Prompt 配置实现。
              </p>
              <div class="space-y-1 text-xs">
                <div class="flex justify-between">
                  <span>适用性:</span>
                  <span class="text-green-600">所有任务</span>
                </div>
                <div class="flex justify-between">
                  <span>执行速度:</span>
                  <span class="text-green-600">快</span>
                </div>
                <div class="flex justify-between">
                  <span>资源消耗:</span>
                  <span class="text-yellow-600">中等</span>
                </div>
              </div>
              <div class="mt-3 flex flex-wrap gap-1">
                <div class="badge badge-outline badge-xs">思考链</div>
                <div class="badge badge-outline badge-xs">并行执行</div>
                <div class="badge badge-outline badge-xs">工具调用</div>
                <div class="badge badge-outline badge-xs">Prompt 驱动</div>
              </div>
              <div class="mt-3 p-2 bg-base-200 rounded text-xs text-base-content/70">
                <i class="fas fa-info-circle mr-1"></i>
                任务类型（通用、安全测试、数据分析等）通过 Prompt 配置，而非代码写死流程
              </div>
            </div>
          </div>
        </div>
        
        <!-- 架构详细信息 -->
        <div class="mt-6 p-4 bg-base-200 rounded-lg">
          <h4 class="font-semibold mb-2">{{ getArchitectureTitle(selectedArchitecture) }}</h4>
          <p class="text-sm text-base-content/80 mb-3">{{ getArchitectureDescription(selectedArchitecture) }}</p>
          
          <!-- 适用场景 -->
          <div class="mb-3">
            <h5 class="font-medium text-sm mb-1">适用场景:</h5>
            <div class="flex flex-wrap gap-1">
              <div 
                v-for="scenario in getArchitectureScenarios(selectedArchitecture)" 
                :key="scenario"
                class="badge badge-ghost badge-sm"
              >
                {{ scenario }}
              </div>
            </div>
          </div>
          
          <!-- 技术特点 -->
          <div>
            <h5 class="font-medium text-sm mb-1">技术特点:</h5>
            <ul class="text-xs text-base-content/70 space-y-1">
              <li v-for="feature in getArchitectureFeatures(selectedArchitecture)" :key="feature">
                • {{ feature }}
              </li>
            </ul>
          </div>
        </div>
        
        <!-- 性能对比图表 -->
        <div class="mt-6">
          <h4 class="font-semibold mb-3">性能对比</h4>
          <div class="grid grid-cols-3 gap-4">
            <div class="text-center">
              <div class="text-xs text-base-content/60 mb-1">Token效率</div>
              <div class="radial-progress text-primary" :style="`--value:${getMetric('token_efficiency')}`">
                {{ getMetric('token_efficiency') }}%
              </div>
            </div>
            <div class="text-center">
              <div class="text-xs text-base-content/60 mb-1">执行速度</div>
              <div class="radial-progress text-success" :style="`--value:${getMetric('execution_speed')}`">
                {{ getMetric('execution_speed') }}%
              </div>
            </div>
            <div class="text-center">
              <div class="text-xs text-base-content/60 mb-1">资源使用</div>
              <div class="radial-progress text-warning" :style="`--value:${100 - getMetric('resource_usage')}`">
                {{ 100 - getMetric('resource_usage') }}%
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'

// Props
interface Props {
  modelValue?: string
  disabled?: boolean
}

const props = withDefaults(defineProps<Props>(), {
  modelValue: 'plan_execute',
  disabled: false
})

// Emits
interface Emits {
  (e: 'update:modelValue', value: string): void
  (e: 'change', value: string): void
}

const emit = defineEmits<Emits>()

// 响应式数据
const selectedArchitecture = computed({
  get: () => props.modelValue,
  set: (value) => {
    emit('update:modelValue', value)
    emit('change', value)
  }
})

// 架构信息（统一为泛化 ReAct）
// 所有任务类型通过 Prompt 配置，而非代码写死流程
const architectureInfo = {
  react: {
    title: 'ReAct 泛化引擎',
    description: '统一的推理与行动架构，支持所有任务类型。任务特性（如安全测试、数据分析等）通过 Prompt 配置实现，而非代码写死流程。',
    scenarios: ['通用任务', '安全测试', '数据分析', '自动化流程', '复杂工作流'],
    features: [
      '交替推理和执行',
      '支持并行工具调用',
      '支持推理链执行',
      '通过 Prompt 定制任务流程',
      '良好的可解释性'
    ],
    metrics: {
      token_efficiency: 85,
      execution_speed: 85,
      resource_usage: 50
    }
  }
}

// 方法
const selectArchitecture = (architecture: string) => {
  if (!props.disabled) {
    selectedArchitecture.value = architecture
  }
}

const getArchitectureTitle = (architecture: string): string => {
  return architectureInfo[architecture as keyof typeof architectureInfo]?.title || ''
}

const getArchitectureDescription = (architecture: string): string => {
  return architectureInfo[architecture as keyof typeof architectureInfo]?.description || ''
}

const getArchitectureScenarios = (architecture: string): string[] => {
  return architectureInfo[architecture as keyof typeof architectureInfo]?.scenarios || []
}

const getArchitectureFeatures = (architecture: string): string[] => {
  return architectureInfo[architecture as keyof typeof architectureInfo]?.features || []
}

const getMetric = (metric: string): number => {
  const info = architectureInfo[selectedArchitecture.value as keyof typeof architectureInfo]
  return info?.metrics[metric as keyof typeof info.metrics] || 0
}
</script>

<style scoped>
.architecture-selector {
  @apply w-full;
}

.radial-progress {
  --size: 3rem;
  --thickness: 3px;
}
</style>