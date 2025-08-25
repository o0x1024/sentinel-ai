<template>
  <div class="scenario-selector">
    <div class="card bg-base-100 shadow-xl">
      <div class="card-body">
        <h3 class="card-title mb-4">
          <i class="fas fa-magic mr-2"></i>
          选择使用场景
        </h3>
        <p class="text-base-content/70 mb-6">
          基于您的具体需求，我们会智能推荐最适合的Agent架构和工作流配置
        </p>
        
        <!-- 场景卡片网格 -->
        <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
          <!-- 安全扫描场景 -->
          <div 
            :class="[
              'card cursor-pointer transition-all duration-200 border-2',
              selectedScenario === 'security_scan' 
                ? 'border-primary bg-primary/10' 
                : 'border-base-300 hover:border-primary/50'
            ]"
            @click="selectScenario('security_scan')"
          >
            <div class="card-body p-4">
              <div class="flex items-center gap-3 mb-3">
                <div class="w-12 h-12 bg-red-500/20 rounded-lg flex items-center justify-center">
                  <i class="fas fa-shield-alt text-red-500 text-xl"></i>
                </div>
                <div>
                  <h4 class="font-semibold">安全扫描</h4>
                  <p class="text-xs text-base-content/60">Security Scanning</p>
                </div>
              </div>
              <p class="text-sm text-base-content/70 mb-3">
                对目标系统进行全面的安全漏洞扫描和评估
              </p>
              <div class="space-y-1 text-xs">
                <div class="flex items-center gap-2">
                  <i class="fas fa-check text-green-500"></i>
                  <span>端口扫描和服务识别</span>
                </div>
                <div class="flex items-center gap-2">
                  <i class="fas fa-check text-green-500"></i>
                  <span>漏洞检测和分析</span>
                </div>
                <div class="flex items-center gap-2">
                  <i class="fas fa-check text-green-500"></i>
                  <span>安全报告生成</span>
                </div>
              </div>
              <div class="mt-3 flex flex-wrap gap-1">
                <div class="badge badge-outline badge-xs">可靠稳定</div>
                <div class="badge badge-outline badge-xs">系统性</div>
              </div>
            </div>
          </div>
          
          <!-- 批量分析场景 -->
          <div 
            :class="[
              'card cursor-pointer transition-all duration-200 border-2',
              selectedScenario === 'batch_analysis' 
                ? 'border-primary bg-primary/10' 
                : 'border-base-300 hover:border-primary/50'
            ]"
            @click="selectScenario('batch_analysis')"
          >
            <div class="card-body p-4">
              <div class="flex items-center gap-3 mb-3">
                <div class="w-12 h-12 bg-blue-500/20 rounded-lg flex items-center justify-center">
                  <i class="fas fa-layer-group text-blue-500 text-xl"></i>
                </div>
                <div>
                  <h4 class="font-semibold">批量分析</h4>
                  <p class="text-xs text-base-content/60">Batch Analysis</p>
                </div>
              </div>
              <p class="text-sm text-base-content/70 mb-3">
                对多个目标进行并行分析处理，提高效率
              </p>
              <div class="space-y-1 text-xs">
                <div class="flex items-center gap-2">
                  <i class="fas fa-check text-green-500"></i>
                  <span>多目标并行处理</span>
                </div>
                <div class="flex items-center gap-2">
                  <i class="fas fa-check text-green-500"></i>
                  <span>智能负载均衡</span>
                </div>
                <div class="flex items-center gap-2">
                  <i class="fas fa-check text-green-500"></i>
                  <span>结果统一汇总</span>
                </div>
              </div>
              <div class="mt-3 flex flex-wrap gap-1">
                <div class="badge badge-outline badge-xs">高效率</div>
                <div class="badge badge-outline badge-xs">并行化</div>
              </div>
            </div>
          </div>
          
          <!-- API集成场景 -->
          <div 
            :class="[
              'card cursor-pointer transition-all duration-200 border-2',
              selectedScenario === 'api_integration' 
                ? 'border-primary bg-primary/10' 
                : 'border-base-300 hover:border-primary/50'
            ]"
            @click="selectScenario('api_integration')"
          >
            <div class="card-body p-4">
              <div class="flex items-center gap-3 mb-3">
                <div class="w-12 h-12 bg-purple-500/20 rounded-lg flex items-center justify-center">
                  <i class="fas fa-plug text-purple-500 text-xl"></i>
                </div>
                <div>
                  <h4 class="font-semibold">API集成</h4>
                  <p class="text-xs text-base-content/60">API Integration</p>
                </div>
              </div>
              <p class="text-sm text-base-content/70 mb-3">
                与第三方API和服务进行集成调用
              </p>
              <div class="space-y-1 text-xs">
                <div class="flex items-center gap-2">
                  <i class="fas fa-check text-green-500"></i>
                  <span>API调用和数据获取</span>
                </div>
                <div class="flex items-center gap-2">
                  <i class="fas fa-check text-green-500"></i>
                  <span>数据格式转换</span>
                </div>
                <div class="flex items-center gap-2">
                  <i class="fas fa-check text-green-500"></i>
                  <span>错误处理和重试</span>
                </div>
              </div>
              <div class="mt-3 flex flex-wrap gap-1">
                <div class="badge badge-outline badge-xs">节能高效</div>
                <div class="badge badge-outline badge-xs">工具链</div>
              </div>
            </div>
          </div>
          
          <!-- 复杂任务场景 -->
          <div 
            :class="[
              'card cursor-pointer transition-all duration-200 border-2',
              selectedScenario === 'complex_task' 
                ? 'border-primary bg-primary/10' 
                : 'border-base-300 hover:border-primary/50'
            ]"
            @click="selectScenario('complex_task')"
          >
            <div class="card-body p-4">
              <div class="flex items-center gap-3 mb-3">
                <div class="w-12 h-12 bg-orange-500/20 rounded-lg flex items-center justify-center">
                  <i class="fas fa-sitemap text-orange-500 text-xl"></i>
                </div>
                <div>
                  <h4 class="font-semibold">复杂任务</h4>
                  <p class="text-xs text-base-content/60">Complex Tasks</p>
                </div>
              </div>
              <p class="text-sm text-base-content/70 mb-3">
                处理多步骤、高并发的复杂业务逻辑
              </p>
              <div class="space-y-1 text-xs">
                <div class="flex items-center gap-2">
                  <i class="fas fa-check text-green-500"></i>
                  <span>复杂依赖管理</span>
                </div>
                <div class="flex items-center gap-2">
                  <i class="fas fa-check text-green-500"></i>
                  <span>最大化并行执行</span>
                </div>
                <div class="flex items-center gap-2">
                  <i class="fas fa-check text-green-500"></i>
                  <span>智能错误恢复</span>
                </div>
              </div>
              <div class="mt-3 flex flex-wrap gap-1">
                <div class="badge badge-outline badge-xs">高性能</div>
                <div class="badge badge-outline badge-xs">复杂度</div>
              </div>
            </div>
          </div>
          
          <!-- 数据分析场景 -->
          <div 
            :class="[
              'card cursor-pointer transition-all duration-200 border-2',
              selectedScenario === 'data_analysis' 
                ? 'border-primary bg-primary/10' 
                : 'border-base-300 hover:border-primary/50'
            ]"
            @click="selectScenario('data_analysis')"
          >
            <div class="card-body p-4">
              <div class="flex items-center gap-3 mb-3">
                <div class="w-12 h-12 bg-green-500/20 rounded-lg flex items-center justify-center">
                  <i class="fas fa-chart-line text-green-500 text-xl"></i>
                </div>
                <div>
                  <h4 class="font-semibold">数据分析</h4>
                  <p class="text-xs text-base-content/60">Data Analysis</p>
                </div>
              </div>
              <p class="text-sm text-base-content/70 mb-3">
                对收集的数据进行深度分析和洞察
              </p>
              <div class="space-y-1 text-xs">
                <div class="flex items-center gap-2">
                  <i class="fas fa-check text-green-500"></i>
                  <span>数据清洗和预处理</span>
                </div>
                <div class="flex items-center gap-2">
                  <i class="fas fa-check text-green-500"></i>
                  <span>统计分析和可视化</span>
                </div>
                <div class="flex items-center gap-2">
                  <i class="fas fa-check text-green-500"></i>
                  <span>报告生成和导出</span>
                </div>
              </div>
              <div class="mt-3 flex flex-wrap gap-1">
                <div class="badge badge-outline badge-xs">智能化</div>
                <div class="badge badge-outline badge-xs">洞察力</div>
              </div>
            </div>
          </div>
          
          <!-- 自定义场景 -->
          <div 
            :class="[
              'card cursor-pointer transition-all duration-200 border-2 border-dashed',
              selectedScenario === 'custom' 
                ? 'border-primary bg-primary/10' 
                : 'border-base-300 hover:border-primary/50'
            ]"
            @click="selectScenario('custom')"
          >
            <div class="card-body p-4">
              <div class="flex items-center gap-3 mb-3">
                <div class="w-12 h-12 bg-gray-500/20 rounded-lg flex items-center justify-center">
                  <i class="fas fa-cog text-gray-500 text-xl"></i>
                </div>
                <div>
                  <h4 class="font-semibold">自定义场景</h4>
                  <p class="text-xs text-base-content/60">Custom Scenario</p>
                </div>
              </div>
              <p class="text-sm text-base-content/70 mb-3">
                根据特定需求自定义配置和流程
              </p>
              <div class="mt-6 text-center">
                <i class="fas fa-plus text-3xl text-base-content/30"></i>
              </div>
              <div class="mt-3 flex flex-wrap gap-1 justify-center">
                <div class="badge badge-outline badge-xs">灵活性</div>
                <div class="badge badge-outline badge-xs">可定制</div>
              </div>
            </div>
          </div>
        </div>
        
        <!-- 选中场景的详细信息 -->
        <div v-if="selectedScenario && selectedScenario !== 'custom'" class="mt-6 p-4 bg-base-200 rounded-lg">
          <h4 class="font-semibold mb-2 flex items-center gap-2">
            <i class="fas fa-info-circle text-primary"></i>
            推荐配置
          </h4>
          <div class="grid grid-cols-1 md:grid-cols-3 gap-4 text-sm">
            <div>
              <span class="font-medium text-base-content/80">推荐架构:</span>
              <div class="mt-1 p-2 bg-base-100 rounded border">
                <div class="flex items-center gap-2">
                  <div :class="`w-3 h-3 rounded ${getArchitectureColor(getRecommendedArchitecture(selectedScenario))}`"></div>
                  <span>{{ getArchitectureName(getRecommendedArchitecture(selectedScenario)) }}</span>
                </div>
              </div>
            </div>
            <div>
              <span class="font-medium text-base-content/80">预计执行时间:</span>
              <div class="mt-1 p-2 bg-base-100 rounded border">
                {{ getEstimatedTime(selectedScenario) }}
              </div>
            </div>
            <div>
              <span class="font-medium text-base-content/80">资源需求:</span>
              <div class="mt-1 p-2 bg-base-100 rounded border">
                {{ getResourceRequirement(selectedScenario) }}
              </div>
            </div>
          </div>
        </div>
        
        <!-- 自定义配置区域 -->
        <div v-if="selectedScenario === 'custom'" class="mt-6 p-4 bg-base-200 rounded-lg">
          <h4 class="font-semibold mb-4">自定义配置</h4>
          <div class="space-y-4">
            <div class="form-control">
              <label class="label">
                <span class="label-text">任务描述</span>
              </label>
              <textarea 
                v-model="customConfig.description" 
                class="textarea textarea-bordered h-20" 
                placeholder="请详细描述您的任务需求..."
              ></textarea>
            </div>
            <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
              <div class="form-control">
                <label class="label">
                  <span class="label-text">复杂度等级</span>
                </label>
                <select v-model="customConfig.complexity" class="select select-bordered">
                  <option value="simple">简单</option>
                  <option value="medium">中等</option>
                  <option value="complex">复杂</option>
                </select>
              </div>
              <div class="form-control">
                <label class="label">
                  <span class="label-text">并发需求</span>
                </label>
                <select v-model="customConfig.concurrency" class="select select-bordered">
                  <option value="low">低并发</option>
                  <option value="medium">中等并发</option>
                  <option value="high">高并发</option>
                </select>
              </div>
            </div>
          </div>
        </div>
        
        <!-- 继续按钮 -->
        <div class="mt-6 flex justify-end">
          <button 
            @click="confirmSelection" 
            :disabled="!selectedScenario"
            class="btn btn-primary"
          >
            <i class="fas fa-arrow-right mr-2"></i>
            继续配置
          </button>
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
}

const props = withDefaults(defineProps<Props>(), {
  modelValue: ''
})

// Emits
interface Emits {
  (e: 'update:modelValue', value: string): void
  (e: 'confirm', config: any): void
}

const emit = defineEmits<Emits>()

// 响应式数据
const selectedScenario = ref(props.modelValue)
const customConfig = ref({
  description: '',
  complexity: 'medium',
  concurrency: 'medium'
})

// 场景配置映射
const scenarioConfigs = {
  security_scan: {
    architecture: 'plan_execute',
    estimatedTime: '5-15分钟',
    resourceRequirement: '中等',
    description: '系统化的安全扫描流程'
  },
  batch_analysis: {
    architecture: 'llm_compiler',
    estimatedTime: '2-8分钟',
    resourceRequirement: '高',
    description: '高并发批量处理'
  },
  api_integration: {
    architecture: 'rewoo',
    estimatedTime: '1-5分钟',
    resourceRequirement: '低',
    description: '高效的API调用链'
  },
  complex_task: {
    architecture: 'llm_compiler',
    estimatedTime: '10-30分钟',
    resourceRequirement: '很高',
    description: '复杂任务分解和并行执行'
  },
  data_analysis: {
    architecture: 'plan_execute',
    estimatedTime: '8-20分钟',
    resourceRequirement: '中等',
    description: '系统化数据处理流程'
  }
}

// 方法
const selectScenario = (scenario: string) => {
  selectedScenario.value = scenario
  emit('update:modelValue', scenario)
}

const getRecommendedArchitecture = (scenario: string) => {
  return scenarioConfigs[scenario as keyof typeof scenarioConfigs]?.architecture || 'plan_execute'
}

const getArchitectureName = (architecture: string) => {
  const names = {
    'plan_execute': 'Plan-and-Execute',
    'rewoo': 'ReWOO',
    'llm_compiler': 'LLMCompiler'
  }
  return names[architecture as keyof typeof names] || architecture
}

const getArchitectureColor = (architecture: string) => {
  const colors = {
    'plan_execute': 'bg-blue-500',
    'rewoo': 'bg-purple-500',
    'llm_compiler': 'bg-indigo-500'
  }
  return colors[architecture as keyof typeof colors] || 'bg-gray-500'
}

const getEstimatedTime = (scenario: string) => {
  return scenarioConfigs[scenario as keyof typeof scenarioConfigs]?.estimatedTime || '未知'
}

const getResourceRequirement = (scenario: string) => {
  return scenarioConfigs[scenario as keyof typeof scenarioConfigs]?.resourceRequirement || '未知'
}

const confirmSelection = () => {
  if (!selectedScenario.value) return
  
  let config
  
  if (selectedScenario.value === 'custom') {
    // 根据自定义配置推荐架构
    let recommendedArchitecture = 'plan_execute'
    
    if (customConfig.value.concurrency === 'high') {
      recommendedArchitecture = 'llm_compiler'
    } else if (customConfig.value.complexity === 'simple') {
      recommendedArchitecture = 'rewoo'
    }
    
    config = {
      scenario: selectedScenario.value,
      architecture: recommendedArchitecture,
      customConfig: customConfig.value
    }
  } else {
    config = {
      scenario: selectedScenario.value,
      architecture: getRecommendedArchitecture(selectedScenario.value),
      ...scenarioConfigs[selectedScenario.value as keyof typeof scenarioConfigs]
    }
  }
  
  emit('confirm', config)
}
</script>

<style scoped>
.scenario-selector {
  @apply w-full;
}

.card:hover {
  transform: translateY(-2px);
}

.card.border-primary {
  box-shadow: 0 0 0 1px hsl(var(--p)), 0 4px 16px hsl(var(--p) / 0.1);
}
</style>
