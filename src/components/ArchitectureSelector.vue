<template>
  <div class="architecture-selector">
    <!-- 架构选择卡片 -->
    <div class="card bg-base-100 shadow-xl">
      <div class="card-body">
        <h3 class="card-title mb-4">选择执行架构</h3>
        
        <div class="grid grid-cols-1 md:grid-cols-3 gap-4">
          <!-- Plan-and-Execute -->
          <div 
            :class="[
              'card cursor-pointer transition-all duration-200 border-2',
              selectedArchitecture === 'plan_execute' 
                ? 'border-primary bg-primary/10' 
                : 'border-base-300 hover:border-primary/50'
            ]"
            @click="selectArchitecture('plan_execute')"
          >
            <div class="card-body p-4">
              <div class="flex items-center gap-2 mb-2">
                <div class="w-3 h-3 bg-blue-500 rounded"></div>
                <h4 class="font-semibold">Plan-and-Execute</h4>
              </div>
              <p class="text-sm text-base-content/70 mb-3">
                传统的规划执行模式，适合大多数常规任务
              </p>
              <div class="space-y-1 text-xs">
                <div class="flex justify-between">
                  <span>复杂度:</span>
                  <span class="text-green-600">低</span>
                </div>
                <div class="flex justify-between">
                  <span>执行速度:</span>
                  <span class="text-yellow-600">中等</span>
                </div>
                <div class="flex justify-between">
                  <span>资源消耗:</span>
                  <span class="text-green-600">低</span>
                </div>
              </div>
              <div class="mt-3">
                <div class="badge badge-outline badge-sm">稳定可靠</div>
              </div>
            </div>
          </div>
          
          <!-- ReWOO -->
          <div 
            :class="[
              'card cursor-pointer transition-all duration-200 border-2',
              selectedArchitecture === 'rewoo' 
                ? 'border-primary bg-primary/10' 
                : 'border-base-300 hover:border-primary/50'
            ]"
            @click="selectArchitecture('rewoo')"
          >
            <div class="card-body p-4">
              <div class="flex items-center gap-2 mb-2">
                <div class="w-3 h-3 bg-purple-500 rounded"></div>
                <h4 class="font-semibold">ReWOO</h4>
              </div>
              <p class="text-sm text-base-content/70 mb-3">
                推理无观察架构，适合工具链明确的任务
              </p>
              <div class="space-y-1 text-xs">
                <div class="flex justify-between">
                  <span>复杂度:</span>
                  <span class="text-yellow-600">中等</span>
                </div>
                <div class="flex justify-between">
                  <span>执行速度:</span>
                  <span class="text-green-600">快</span>
                </div>
                <div class="flex justify-between">
                  <span>资源消耗:</span>
                  <span class="text-green-600">低</span>
                </div>
              </div>
              <div class="mt-3">
                <div class="badge badge-outline badge-sm">高效节能</div>
              </div>
            </div>
          </div>
          
          <!-- LLMCompiler -->
          <div 
            :class="[
              'card cursor-pointer transition-all duration-200 border-2',
              selectedArchitecture === 'llm_compiler' 
                ? 'border-primary bg-primary/10' 
                : 'border-base-300 hover:border-primary/50'
            ]"
            @click="selectArchitecture('llm_compiler')"
          >
            <div class="card-body p-4">
              <div class="flex items-center gap-2 mb-2">
                <div class="w-3 h-3 bg-indigo-500 rounded"></div>
                <h4 class="font-semibold">LLMCompiler</h4>
              </div>
              <p class="text-sm text-base-content/70 mb-3">
                并发执行架构，适合复杂多步骤任务
              </p>
              <div class="space-y-1 text-xs">
                <div class="flex justify-between">
                  <span>复杂度:</span>
                  <span class="text-red-600">高</span>
                </div>
                <div class="flex justify-between">
                  <span>执行速度:</span>
                  <span class="text-green-600">很快</span>
                </div>
                <div class="flex justify-between">
                  <span>资源消耗:</span>
                  <span class="text-red-600">高</span>
                </div>
              </div>
              <div class="mt-3">
                <div class="badge badge-outline badge-sm">高性能</div>
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

// 架构信息
const architectureInfo = {
  plan_execute: {
    title: 'Plan-and-Execute 架构',
    description: '传统的规划执行模式，通过规划器生成执行计划，然后由执行器逐步执行。支持动态重规划和错误恢复。',
    scenarios: ['常规任务', '稳定环境', '资源受限', '简单工作流'],
    features: [
      '成熟稳定的架构模式',
      '支持动态重规划',
      '良好的错误处理机制',
      '资源消耗较低',
      '易于调试和监控'
    ],
    metrics: {
      token_efficiency: 70,
      execution_speed: 60,
      resource_usage: 40
    }
  },
  rewoo: {
    title: 'ReWOO 架构',
    description: '推理无观察架构，通过一次性规划生成完整的工具调用链，然后并行执行所有工具，最后由求解器生成最终答案。',
    scenarios: ['工具链明确', 'API调用密集', '中等复杂度', '批处理任务'],
    features: [
      '减少LLM调用次数',
      '支持变量替换和模板化',
      '并行工具执行',
      '高Token效率',
      '适合确定性任务'
    ],
    metrics: {
      token_efficiency: 85,
      execution_speed: 80,
      resource_usage: 50
    }
  },
  llm_compiler: {
    title: 'LLMCompiler 架构',
    description: 'DAG并发执行架构，通过构建任务依赖图实现最大化并行执行。智能连接器负责决策和重规划。',
    scenarios: ['复杂多步骤', '高并发需求', '大规模任务', '性能优先'],
    features: [
      'DAG任务调度',
      '最大化并行执行',
      '智能依赖解析',
      '动态任务获取',
      '高性能执行'
    ],
    metrics: {
      token_efficiency: 90,
      execution_speed: 95,
      resource_usage: 80
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