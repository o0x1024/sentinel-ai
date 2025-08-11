<template>
  <div class="architecture-comparison">
    <!-- 对比概览 -->
    <div class="card bg-base-100 shadow-xl mb-6">
      <div class="card-body">
        <h3 class="card-title mb-4">架构对比分析</h3>
        
        <!-- 架构选择 -->
        <div class="flex flex-wrap gap-2 mb-6">
          <button 
            v-for="arch in availableArchitectures" 
            :key="arch.id"
            :class="[
              'btn btn-sm',
              selectedArchitectures.includes(arch.id) 
                ? 'btn-primary' 
                : 'btn-outline'
            ]"
            @click="toggleArchitecture(arch.id)"
          >
            {{ arch.name }}
          </button>
        </div>
        
        <!-- 对比表格 -->
        <div class="overflow-x-auto">
          <table class="table table-zebra">
            <thead>
              <tr>
                <th>特性</th>
                <th v-for="archId in selectedArchitectures" :key="archId" class="text-center">
                  {{ getArchitectureName(archId) }}
                </th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="feature in comparisonFeatures" :key="feature.key">
                <td class="font-medium">{{ feature.name }}</td>
                <td v-for="archId in selectedArchitectures" :key="archId" class="text-center">
                  <div v-if="feature.type === 'rating'" class="rating rating-sm">
                    <input 
                      v-for="i in 5" 
                      :key="i"
                      type="radio" 
                      :name="`${feature.key}-${archId}`"
                      class="mask mask-star-2 bg-orange-400" 
                      :checked="i <= getFeatureValue(archId, feature.key)"
                      disabled
                    />
                  </div>
                  <div v-else-if="feature.type === 'boolean'" class="flex justify-center">
                    <input 
                      type="checkbox" 
                      class="checkbox checkbox-sm" 
                      :checked="getFeatureValue(archId, feature.key)"
                      disabled
                    />
                  </div>
                  <div v-else-if="feature.type === 'badge'" class="flex justify-center">
                    <div :class="getFeatureBadge(archId, feature.key)" class="badge badge-sm">
                      {{ getFeatureValue(archId, feature.key) }}
                    </div>
                  </div>
                  <span v-else>{{ getFeatureValue(archId, feature.key) }}</span>
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>
    </div>
    
    <!-- 性能指标对比 -->
    <div class="grid grid-cols-1 lg:grid-cols-2 gap-6 mb-6">
      <!-- Token 效率对比 -->
      <div class="card bg-base-100 shadow-xl">
        <div class="card-body">
          <h4 class="card-title text-lg mb-4">Token 效率对比</h4>
          <div class="space-y-3">
            <div v-for="archId in selectedArchitectures" :key="archId" class="flex items-center gap-3">
              <div class="w-20 text-sm">{{ getArchitectureName(archId) }}</div>
              <div class="flex-1">
                <div class="flex justify-between text-xs mb-1">
                  <span>Token 效率</span>
                  <span>{{ getMetric(archId, 'token_efficiency') }}%</span>
                </div>
                <progress 
                  class="progress w-full h-2" 
                  :class="getProgressClass(archId)"
                  :value="getMetric(archId, 'token_efficiency')" 
                  max="100"
                ></progress>
              </div>
            </div>
          </div>
        </div>
      </div>
      
      <!-- 执行速度对比 -->
      <div class="card bg-base-100 shadow-xl">
        <div class="card-body">
          <h4 class="card-title text-lg mb-4">执行速度对比</h4>
          <div class="space-y-3">
            <div v-for="archId in selectedArchitectures" :key="archId" class="flex items-center gap-3">
              <div class="w-20 text-sm">{{ getArchitectureName(archId) }}</div>
              <div class="flex-1">
                <div class="flex justify-between text-xs mb-1">
                  <span>执行速度</span>
                  <span>{{ getMetric(archId, 'execution_speed') }}%</span>
                </div>
                <progress 
                  class="progress w-full h-2" 
                  :class="getProgressClass(archId)"
                  :value="getMetric(archId, 'execution_speed')" 
                  max="100"
                ></progress>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
    
    <!-- 详细分析 -->
    <div class="grid grid-cols-1 lg:grid-cols-3 gap-6">
      <div v-for="archId in selectedArchitectures" :key="archId" class="card bg-base-100 shadow-xl">
        <div class="card-body">
          <div class="flex items-center gap-2 mb-4">
            <div class="w-4 h-4 rounded" :class="getArchitectureColor(archId)"></div>
            <h4 class="card-title text-lg">{{ getArchitectureName(archId) }}</h4>
          </div>
          
          <!-- 优势 -->
          <div class="mb-4">
            <h5 class="font-semibold text-sm mb-2 text-success">✓ 优势</h5>
            <ul class="text-xs space-y-1 text-base-content/70">
              <li v-for="advantage in getArchitectureAdvantages(archId)" :key="advantage">
                • {{ advantage }}
              </li>
            </ul>
          </div>
          
          <!-- 劣势 -->
          <div class="mb-4">
            <h5 class="font-semibold text-sm mb-2 text-warning">⚠ 注意事项</h5>
            <ul class="text-xs space-y-1 text-base-content/70">
              <li v-for="limitation in getArchitectureLimitations(archId)" :key="limitation">
                • {{ limitation }}
              </li>
            </ul>
          </div>
          
          <!-- 适用场景 -->
          <div>
            <h5 class="font-semibold text-sm mb-2">适用场景</h5>
            <div class="flex flex-wrap gap-1">
              <div 
                v-for="scenario in getArchitectureScenarios(archId)" 
                :key="scenario"
                class="badge badge-ghost badge-sm"
              >
                {{ scenario }}
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
    
    <!-- 推荐建议 -->
    <div class="card bg-base-100 shadow-xl mt-6">
      <div class="card-body">
        <h4 class="card-title mb-4">选择建议</h4>
        <div class="grid grid-cols-1 md:grid-cols-3 gap-4">
          <div class="alert alert-info">
            <svg xmlns="http://www.w3.org/2000/svg" class="stroke-current shrink-0 h-6 w-6" fill="none" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"/>
            </svg>
            <div>
              <h3 class="font-bold">新手推荐</h3>
              <div class="text-xs">Plan-and-Execute 架构稳定可靠，适合初学者</div>
            </div>
          </div>
          
          <div class="alert alert-success">
            <svg xmlns="http://www.w3.org/2000/svg" class="stroke-current shrink-0 h-6 w-6" fill="none" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z"/>
            </svg>
            <div>
              <h3 class="font-bold">效率优先</h3>
              <div class="text-xs">ReWOO 架构 Token 效率高，成本低</div>
            </div>
          </div>
          
          <div class="alert alert-warning">
            <svg xmlns="http://www.w3.org/2000/svg" class="stroke-current shrink-0 h-6 w-6" fill="none" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-2.5L13.732 4c-.77-.833-1.964-.833-2.732 0L3.732 16c-.77.833.192 2.5 1.732 2.5z"/>
            </svg>
            <div>
              <h3 class="font-bold">性能优先</h3>
              <div class="text-xs">LLMCompiler 架构并发能力强，适合复杂任务</div>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'

// 可用架构
const availableArchitectures = [
  { id: 'plan_execute', name: 'Plan-and-Execute' },
  { id: 'rewoo', name: 'ReWOO' },
  { id: 'llm_compiler', name: 'LLMCompiler' }
]

// 选中的架构
const selectedArchitectures = ref(['plan_execute', 'rewoo', 'llm_compiler'])

// 对比特性
const comparisonFeatures = [
  { key: 'complexity', name: '实现复杂度', type: 'rating' },
  { key: 'token_efficiency', name: 'Token 效率', type: 'rating' },
  { key: 'execution_speed', name: '执行速度', type: 'rating' },
  { key: 'parallel_support', name: '并行支持', type: 'boolean' },
  { key: 'dynamic_planning', name: '动态规划', type: 'boolean' },
  { key: 'error_recovery', name: '错误恢复', type: 'boolean' },
  { key: 'resource_usage', name: '资源消耗', type: 'badge' },
  { key: 'learning_curve', name: '学习曲线', type: 'badge' },
  { key: 'maintenance', name: '维护难度', type: 'badge' }
]

// 架构数据
const architectureData = {
  plan_execute: {
    complexity: 2,
    token_efficiency: 3,
    execution_speed: 3,
    parallel_support: false,
    dynamic_planning: true,
    error_recovery: true,
    resource_usage: '低',
    learning_curve: '简单',
    maintenance: '容易',
    metrics: {
      token_efficiency: 70,
      execution_speed: 60,
      resource_usage: 40
    },
    advantages: [
      '成熟稳定的架构模式',
      '支持动态重规划',
      '良好的错误处理机制',
      '资源消耗较低',
      '易于调试和监控'
    ],
    limitations: [
      '执行速度相对较慢',
      '并行能力有限',
      'Token 使用效率一般'
    ],
    scenarios: ['常规任务', '稳定环境', '资源受限', '简单工作流']
  },
  rewoo: {
    complexity: 3,
    token_efficiency: 4,
    execution_speed: 4,
    parallel_support: true,
    dynamic_planning: false,
    error_recovery: false,
    resource_usage: '中',
    learning_curve: '中等',
    maintenance: '中等',
    metrics: {
      token_efficiency: 85,
      execution_speed: 80,
      resource_usage: 50
    },
    advantages: [
      '减少LLM调用次数',
      '支持变量替换和模板化',
      '并行工具执行',
      '高Token效率',
      '适合确定性任务'
    ],
    limitations: [
      '缺乏动态调整能力',
      '错误恢复机制较弱',
      '对任务预规划要求高'
    ],
    scenarios: ['工具链明确', 'API调用密集', '中等复杂度', '批处理任务']
  },
  llm_compiler: {
    complexity: 5,
    token_efficiency: 5,
    execution_speed: 5,
    parallel_support: true,
    dynamic_planning: true,
    error_recovery: true,
    resource_usage: '高',
    learning_curve: '困难',
    maintenance: '困难',
    metrics: {
      token_efficiency: 90,
      execution_speed: 95,
      resource_usage: 80
    },
    advantages: [
      'DAG任务调度',
      '最大化并行执行',
      '智能依赖解析',
      '动态任务获取',
      '高性能执行'
    ],
    limitations: [
      '实现复杂度高',
      '资源消耗大',
      '调试难度大',
      '学习成本高'
    ],
    scenarios: ['复杂多步骤', '高并发需求', '大规模任务', '性能优先']
  }
}

// 方法
const toggleArchitecture = (archId: string) => {
  const index = selectedArchitectures.value.indexOf(archId)
  if (index > -1) {
    if (selectedArchitectures.value.length > 1) {
      selectedArchitectures.value.splice(index, 1)
    }
  } else {
    selectedArchitectures.value.push(archId)
  }
}

const getArchitectureName = (archId: string): string => {
  return availableArchitectures.find(arch => arch.id === archId)?.name || archId
}

const getFeatureValue = (archId: string, featureKey: string): any => {
  return architectureData[archId as keyof typeof architectureData]?.[featureKey as keyof typeof architectureData[keyof typeof architectureData]]
}

const getFeatureBadge = (archId: string, featureKey: string): string => {
  const value = getFeatureValue(archId, featureKey)
  const badgeMap: Record<string, string> = {
    '低': 'badge-success',
    '中': 'badge-warning', 
    '高': 'badge-error',
    '简单': 'badge-success',
    '中等': 'badge-warning',
    '困难': 'badge-error',
    '容易': 'badge-success'
  }
  return badgeMap[value] || 'badge-ghost'
}

const getMetric = (archId: string, metricKey: string): number => {
  return architectureData[archId as keyof typeof architectureData]?.metrics[metricKey as keyof typeof architectureData[keyof typeof architectureData]['metrics']] || 0
}

const getProgressClass = (archId: string): string => {
  const classMap: Record<string, string> = {
    'plan_execute': 'progress-info',
    'rewoo': 'progress-secondary',
    'llm_compiler': 'progress-accent'
  }
  return classMap[archId] || 'progress-primary'
}

const getArchitectureColor = (archId: string): string => {
  const colorMap: Record<string, string> = {
    'plan_execute': 'bg-blue-500',
    'rewoo': 'bg-purple-500',
    'llm_compiler': 'bg-indigo-500'
  }
  return colorMap[archId] || 'bg-gray-500'
}

const getArchitectureAdvantages = (archId: string): string[] => {
  return architectureData[archId as keyof typeof architectureData]?.advantages || []
}

const getArchitectureLimitations = (archId: string): string[] => {
  return architectureData[archId as keyof typeof architectureData]?.limitations || []
}

const getArchitectureScenarios = (archId: string): string[] => {
  return architectureData[archId as keyof typeof architectureData]?.scenarios || []
}
</script>

<style scoped>
.architecture-comparison {
  @apply w-full;
}

.rating input {
  pointer-events: none;
}

.checkbox {
  pointer-events: none;
}
</style>