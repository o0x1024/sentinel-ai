<template>
  <div class="architecture-config">
    <!-- 配置面板 -->
    <div class="card bg-base-100 shadow-xl">
      <div class="card-body">
        <div class="flex items-center justify-between mb-4">
          <h3 class="card-title">架构配置</h3>
          <div class="badge badge-primary">{{ getArchitectureName(architecture) }}</div>
        </div>
        
        <!-- Plan-and-Execute 配置 -->
        <div v-if="architecture === 'plan_execute'" class="space-y-4">
          <div class="form-control">
            <label class="label">
              <span class="label-text">最大规划步骤</span>
              <span class="label-text-alt">{{ config.plan_execute.max_planning_steps }}</span>
            </label>
            <input 
              type="range" 
              min="3" 
              max="20" 
              v-model.number="config.plan_execute.max_planning_steps"
              class="range range-primary" 
            />
            <div class="w-full flex justify-between text-xs px-2">
              <span>3</span>
              <span>10</span>
              <span>20</span>
            </div>
          </div>
          
          <div class="form-control">
            <label class="label">
              <span class="label-text">重规划阈值</span>
              <span class="label-text-alt">{{ config.plan_execute.replanning_threshold }}%</span>
            </label>
            <input 
              type="range" 
              min="10" 
              max="90" 
              step="10"
              v-model.number="config.plan_execute.replanning_threshold"
              class="range range-secondary" 
            />
          </div>
          
          <div class="form-control">
            <label class="label cursor-pointer">
              <span class="label-text">启用动态重规划</span>
              <input 
                type="checkbox" 
                v-model="config.plan_execute.enable_dynamic_replanning"
                class="checkbox checkbox-primary" 
              />
            </label>
          </div>
          
          <div class="form-control">
            <label class="label cursor-pointer">
              <span class="label-text">启用并行执行</span>
              <input 
                type="checkbox" 
                v-model="config.plan_execute.enable_parallel_execution"
                class="checkbox checkbox-primary" 
              />
            </label>
          </div>
        </div>
        
        <!-- ReWOO 配置 -->
        <div v-if="architecture === 'rewoo'" class="space-y-4">
          <div class="form-control">
            <label class="label">
              <span class="label-text">最大工具数量</span>
              <span class="label-text-alt">{{ config.rewoo.max_tools }}</span>
            </label>
            <input 
              type="range" 
              min="3" 
              max="15" 
              v-model.number="config.rewoo.max_tools"
              class="range range-primary" 
            />
          </div>
          
          <div class="form-control">
            <label class="label">
              <span class="label-text">变量解析深度</span>
              <span class="label-text-alt">{{ config.rewoo.variable_resolution_depth }}</span>
            </label>
            <input 
              type="range" 
              min="1" 
              max="5" 
              v-model.number="config.rewoo.variable_resolution_depth"
              class="range range-secondary" 
            />
          </div>
          
          <div class="form-control">
            <label class="label cursor-pointer">
              <span class="label-text">启用工具并行执行</span>
              <input 
                type="checkbox" 
                v-model="config.rewoo.enable_parallel_tools"
                class="checkbox checkbox-primary" 
              />
            </label>
          </div>
          
          <div class="form-control">
            <label class="label cursor-pointer">
              <span class="label-text">启用变量缓存</span>
              <input 
                type="checkbox" 
                v-model="config.rewoo.enable_variable_caching"
                class="checkbox checkbox-primary" 
              />
            </label>
          </div>
          
          <div class="form-control">
            <label class="label">
              <span class="label-text">求解器模式</span>
            </label>
            <select v-model="config.rewoo.solver_mode" class="select select-bordered">
              <option value="simple">简单模式</option>
              <option value="advanced">高级模式</option>
              <option value="adaptive">自适应模式</option>
            </select>
          </div>
        </div>
        
        <!-- LLMCompiler 配置 -->
        <div v-if="architecture === 'llm_compiler'" class="space-y-4">
          <div class="form-control">
            <label class="label">
              <span class="label-text">最大并发任务</span>
              <span class="label-text-alt">{{ config.llm_compiler.max_concurrent_tasks }}</span>
            </label>
            <input 
              type="range" 
              min="2" 
              max="20" 
              v-model.number="config.llm_compiler.max_concurrent_tasks"
              class="range range-primary" 
            />
          </div>
          
          <div class="form-control">
            <label class="label">
              <span class="label-text">DAG 优化级别</span>
              <span class="label-text-alt">{{ config.llm_compiler.dag_optimization_level }}</span>
            </label>
            <input 
              type="range" 
              min="1" 
              max="3" 
              v-model.number="config.llm_compiler.dag_optimization_level"
              class="range range-secondary" 
            />
            <div class="w-full flex justify-between text-xs px-2">
              <span>基础</span>
              <span>标准</span>
              <span>高级</span>
            </div>
          </div>
          
          <div class="form-control">
            <label class="label">
              <span class="label-text">连接器决策频率</span>
              <span class="label-text-alt">{{ config.llm_compiler.joiner_decision_frequency }}ms</span>
            </label>
            <input 
              type="range" 
              min="100" 
              max="2000" 
              step="100"
              v-model.number="config.llm_compiler.joiner_decision_frequency"
              class="range range-accent" 
            />
          </div>
          
          <div class="form-control">
            <label class="label cursor-pointer">
              <span class="label-text">启用智能调度</span>
              <input 
                type="checkbox" 
                v-model="config.llm_compiler.enable_smart_scheduling"
                class="checkbox checkbox-primary" 
              />
            </label>
          </div>
          
          <div class="form-control">
            <label class="label cursor-pointer">
              <span class="label-text">启用依赖优化</span>
              <input 
                type="checkbox" 
                v-model="config.llm_compiler.enable_dependency_optimization"
                class="checkbox checkbox-primary" 
              />
            </label>
          </div>
          
          <div class="form-control">
            <label class="label cursor-pointer">
              <span class="label-text">启用任务预取</span>
              <input 
                type="checkbox" 
                v-model="config.llm_compiler.enable_task_prefetching"
                class="checkbox checkbox-primary" 
              />
            </label>
          </div>
          
          <div class="form-control">
            <label class="label">
              <span class="label-text">执行策略</span>
            </label>
            <select v-model="config.llm_compiler.execution_strategy" class="select select-bordered">
              <option value="greedy">贪心策略</option>
              <option value="balanced">平衡策略</option>
              <option value="conservative">保守策略</option>
            </select>
          </div>
        </div>
        
        <!-- 通用配置 -->
        <div class="divider">通用配置</div>
        
        <div class="space-y-4">
          <div class="form-control">
            <label class="label">
              <span class="label-text">超时时间 (秒)</span>
              <span class="label-text-alt">{{ config.common.timeout_seconds }}</span>
            </label>
            <input 
              type="range" 
              min="30" 
              max="600" 
              step="30"
              v-model.number="config.common.timeout_seconds"
              class="range range-warning" 
            />
          </div>
          
          <div class="form-control">
            <label class="label">
              <span class="label-text">重试次数</span>
              <span class="label-text-alt">{{ config.common.max_retries }}</span>
            </label>
            <input 
              type="range" 
              min="0" 
              max="5" 
              v-model.number="config.common.max_retries"
              class="range range-error" 
            />
          </div>
          
          <div class="form-control">
            <label class="label cursor-pointer">
              <span class="label-text">启用详细日志</span>
              <input 
                type="checkbox" 
                v-model="config.common.enable_verbose_logging"
                class="checkbox checkbox-primary" 
              />
            </label>
          </div>
          
          <div class="form-control">
            <label class="label cursor-pointer">
              <span class="label-text">启用性能监控</span>
              <input 
                type="checkbox" 
                v-model="config.common.enable_performance_monitoring"
                class="checkbox checkbox-primary" 
              />
            </label>
          </div>
        </div>
        
        <!-- 操作按钮 -->
        <div class="card-actions justify-end mt-6">
          <button class="btn btn-ghost" @click="resetToDefaults">
            重置默认
          </button>
          <button class="btn btn-primary" @click="saveConfig">
            保存配置
          </button>
        </div>
      </div>
    </div>
    
    <!-- 配置预览 -->
    <div class="card bg-base-100 shadow-xl mt-4">
      <div class="card-body">
        <h4 class="card-title text-sm">配置预览</h4>
        <div class="mockup-code text-xs">
          <pre><code>{{ JSON.stringify(config, null, 2) }}</code></pre>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, watch } from 'vue'

// Props
interface Props {
  architecture: string
  initialConfig?: any
}

const props = withDefaults(defineProps<Props>(), {
  architecture: 'plan_execute',
  initialConfig: () => ({})
})

// Emits
interface Emits {
  (e: 'configChange', config: any): void
  (e: 'save', config: any): void
}

const emit = defineEmits<Emits>()

// 默认配置
const defaultConfig = {
  plan_execute: {
    max_planning_steps: 10,
    replanning_threshold: 30,
    enable_dynamic_replanning: true,
    enable_parallel_execution: false
  },
  rewoo: {
    max_tools: 8,
    variable_resolution_depth: 3,
    enable_parallel_tools: true,
    enable_variable_caching: true,
    solver_mode: 'advanced'
  },
  llm_compiler: {
    max_concurrent_tasks: 8,
    dag_optimization_level: 2,
    joiner_decision_frequency: 500,
    enable_smart_scheduling: true,
    enable_dependency_optimization: true,
    enable_task_prefetching: false,
    execution_strategy: 'balanced'
  },
  common: {
    timeout_seconds: 300,
    max_retries: 3,
    enable_verbose_logging: false,
    enable_performance_monitoring: true
  }
}

// 响应式配置
const config = reactive({
  ...defaultConfig,
  ...props.initialConfig
})

// 监听配置变化
watch(config, (newConfig) => {
  emit('configChange', newConfig)
}, { deep: true })

// 方法
const getArchitectureName = (architecture: string): string => {
  const names = {
    plan_execute: 'Plan-and-Execute',
    rewoo: 'ReWOO',
    llm_compiler: 'LLMCompiler'
  }
  return names[architecture as keyof typeof names] || architecture
}

const resetToDefaults = () => {
  Object.assign(config, defaultConfig)
}

const saveConfig = () => {
  emit('save', config)
}
</script>

<style scoped>
.architecture-config {
  @apply w-full;
}

.mockup-code {
  max-height: 200px;
  overflow-y: auto;
}
</style>