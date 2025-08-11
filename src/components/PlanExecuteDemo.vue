<template>
  <div class="plan-execute-demo">
    <!-- 页面标题 -->
    <div class="demo-header">
      <h1 class="text-4xl font-bold mb-4">Plan-and-Execute 引擎测试</h1>
      <p class="demo-description text-lg text-base-content/70 max-w-4xl mx-auto">
        专门测试 Plan-and-Execute 引擎的功能，包括任务规划、执行、重新规划和状态管理。
        该引擎采用经典的规划-执行模式，适合需要明确步骤分解的复杂任务。
      </p>
    </div>

    <!-- 引擎状态 -->
    <div class="service-status mb-8">
      <div class="card bg-base-100 shadow-xl">
        <div class="card-body">
          <div class="flex justify-between items-center mb-6">
            <h3 class="card-title">Plan-and-Execute 引擎状态</h3>
            <button 
              class="btn btn-primary"
              @click="initializeEngine"
              :disabled="initializing || engineStatus.initialized"
            >
              <span v-if="initializing" class="loading loading-spinner loading-sm"></span>
              {{ engineStatus.initialized ? '引擎已就绪' : '启动引擎' }}
            </button>
          </div>
          
          <div class="grid grid-cols-1 md:grid-cols-4 gap-4">
            <div class="stat">
              <div class="stat-title">引擎状态</div>
              <div class="stat-value text-lg">
                <div :class="['badge', engineStatus.initialized ? 'badge-success' : 'badge-warning']">
                  {{ engineStatus.initialized ? '运行中' : '未启动' }}
                </div>
              </div>
            </div>
            <div class="stat">
              <div class="stat-title">活跃任务</div>
              <div class="stat-value text-lg">{{ engineStatus.activeSessions || 0 }}</div>
            </div>
            <div class="stat">
              <div class="stat-title">引擎版本</div>
              <div class="stat-value text-lg">{{ engineStatus.version }}</div>
            </div>
            <div class="stat">
              <div class="stat-title">运行时间</div>
              <div class="stat-value text-lg">{{ formatUptime(engineStatus.uptime) }}</div>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- 执行统计 -->
    <div class="health-status mb-8" v-if="engineStatus.initialized">
      <div class="card bg-base-100 shadow-xl">
        <div class="card-body">
          <div class="flex justify-between items-center mb-6">
            <h3 class="card-title">引擎统计</h3>
            <button 
              class="btn btn-outline btn-sm"
              @click="loadStatistics"
              :disabled="loadingStatistics"
            >
              <span v-if="loadingStatistics" class="loading loading-spinner loading-sm"></span>
              刷新统计
            </button>
          </div>
          
          <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-5 gap-4">
            <div class="stat bg-base-200 rounded-lg">
              <div class="stat-title">总任务数</div>
              <div class="stat-value text-primary">{{ statistics.totalSessions || 0 }}</div>
            </div>
            <div class="stat bg-base-200 rounded-lg">
              <div class="stat-title">完成任务</div>
              <div class="stat-value text-success">{{ statistics.completedSessions || 0 }}</div>
            </div>
            <div class="stat bg-base-200 rounded-lg">
              <div class="stat-title">失败任务</div>
              <div class="stat-value text-error">{{ statistics.failedSessions || 0 }}</div>
            </div>
            <div class="stat bg-base-200 rounded-lg">
              <div class="stat-title">重规划次数</div>
              <div class="stat-value text-warning">{{ statistics.replanCount || 0 }}</div>
            </div>
            <div class="stat bg-base-200 rounded-lg">
              <div class="stat-title">平均耗时</div>
              <div class="stat-value text-info">{{ formatDuration(statistics.averageExecutionTime || 0) }}</div>
            </div>
          </div>
          
          <!-- 引擎组件状态 -->
          <div class="mt-6">
            <h4 class="font-semibold mb-3">引擎组件状态</h4>
            <div class="grid grid-cols-1 md:grid-cols-4 gap-3">
              <div class="flex justify-between items-center p-3 bg-base-200 rounded">
                <span class="font-medium">规划器</span>
                <div class="badge badge-success">正常</div>
              </div>
              <div class="flex justify-between items-center p-3 bg-base-200 rounded">
                <span class="font-medium">执行器</span>
                <div class="badge badge-success">正常</div>
              </div>
              <div class="flex justify-between items-center p-3 bg-base-200 rounded">
                <span class="font-medium">重规划器</span>
                <div class="badge badge-success">正常</div>
              </div>
              <div class="flex justify-between items-center p-3 bg-base-200 rounded">
                <span class="font-medium">内存管理器</span>
                <div class="badge badge-success">正常</div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- Plan-and-Execute 流程可视化 -->
    <div class="flowchart-container mb-8" v-if="engineStatus.initialized">
      <div class="card bg-base-100 shadow-xl">
        <div class="card-body">
          <h3 class="card-title mb-6">Plan-and-Execute 执行流程</h3>
          <div class="bg-base-200 rounded-lg p-6 min-h-[400px]">
            <!-- 流程步骤图 -->
            <div class="flex flex-col lg:flex-row items-center justify-center space-y-4 lg:space-y-0 lg:space-x-8">
              <!-- 规划阶段 -->
              <div class="flex flex-col items-center">
                <div class="w-20 h-20 bg-primary rounded-full flex items-center justify-center text-primary-content font-bold text-lg mb-2">
                  1
                </div>
                <div class="text-center">
                  <div class="font-semibold">任务规划</div>
                  <div class="text-sm text-base-content/70">分析任务并制定执行计划</div>
                </div>
              </div>
              
              <!-- 箭头 -->
              <div class="hidden lg:block text-2xl text-base-content/50">→</div>
              
              <!-- 执行阶段 -->
              <div class="flex flex-col items-center">
                <div class="w-20 h-20 bg-secondary rounded-full flex items-center justify-center text-secondary-content font-bold text-lg mb-2">
                  2
                </div>
                <div class="text-center">
                  <div class="font-semibold">步骤执行</div>
                  <div class="text-sm text-base-content/70">按计划逐步执行任务</div>
                </div>
              </div>
              
              <!-- 箭头 -->
              <div class="hidden lg:block text-2xl text-base-content/50">→</div>
              
              <!-- 评估阶段 -->
              <div class="flex flex-col items-center">
                <div class="w-20 h-20 bg-accent rounded-full flex items-center justify-center text-accent-content font-bold text-lg mb-2">
                  3
                </div>
                <div class="text-center">
                  <div class="font-semibold">结果评估</div>
                  <div class="text-sm text-base-content/70">检查执行结果</div>
                </div>
              </div>
              
              <!-- 箭头 -->
              <div class="hidden lg:block text-2xl text-base-content/50">→</div>
              
              <!-- 重规划阶段 -->
              <div class="flex flex-col items-center">
                <div class="w-20 h-20 bg-warning rounded-full flex items-center justify-center text-warning-content font-bold text-lg mb-2">
                  4
                </div>
                <div class="text-center">
                  <div class="font-semibold">重新规划</div>
                  <div class="text-sm text-base-content/70">必要时调整计划</div>
                </div>
              </div>
            </div>
            
            <!-- 当前执行状态 -->
            <div v-if="currentTaskDetail" class="mt-8 p-4 bg-base-100 rounded-lg">
              <h4 class="font-semibold mb-2">当前执行状态</h4>
              <div class="text-sm">
                <div>任务ID: {{ currentTaskDetail.task_id }}</div>
                <div>当前步骤: {{ currentTaskDetail.current_step || '等待中' }}</div>
                <div>进度: {{ Math.round(currentTaskDetail.progress || 0) }}%</div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- 引擎配置 -->
    <div class="llm-config mb-8" v-if="engineStatus.initialized">
      <div class="card bg-base-100 shadow-xl">
        <div class="card-body">
          <div class="flex justify-between items-center mb-6">
            <h3 class="card-title">引擎配置</h3>
            <button 
              class="btn btn-outline btn-sm"
              @click="showEngineConfig = !showEngineConfig"
            >
              {{ showEngineConfig ? '隐藏配置' : '显示配置' }}
            </button>
          </div>
          
          <div v-if="showEngineConfig" class="space-y-4">
            <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
              <div class="form-control">
                <label class="label">
                  <span class="label-text">规划策略</span>
                </label>
                <select class="select select-bordered" v-model="engineConfig.planningStrategy">
                  <option value="default">默认规划</option>
                  <option value="detailed">详细规划</option>
                  <option value="adaptive">自适应规划</option>
                  <option value="conservative">保守规划</option>
                </select>
              </div>
              
              <div class="form-control">
                <label class="label">
                  <span class="label-text">执行模式</span>
                </label>
                <select class="select select-bordered" v-model="engineConfig.executionMode">
                  <option value="sequential">顺序执行</option>
                  <option value="parallel">并行执行</option>
                  <option value="mixed">混合执行</option>
                </select>
              </div>
            </div>
            
            <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
              <div class="form-control">
                <label class="label">
                  <span class="label-text">重规划阈值</span>
                </label>
                <input 
                  type="range" 
                  min="0" 
                  max="100" 
                  v-model="engineConfig.replanThreshold" 
                  class="range range-primary"
                >
                <div class="w-full flex justify-between text-xs px-2">
                  <span>0%</span>
                  <span>{{ engineConfig.replanThreshold }}%</span>
                  <span>100%</span>
                </div>
              </div>
              
              <div class="form-control">
                <label class="label">
                  <span class="label-text">最大重试次数</span>
                </label>
                <input 
                  type="number" 
                  class="input input-bordered" 
                  min="1" 
                  max="10"
                  v-model.number="engineConfig.maxRetries"
                >
              </div>
            </div>
            
            <div class="form-control">
              <label class="label">
                <span class="label-text">引擎选项</span>
              </label>
              <div class="flex gap-4">
                <label class="label cursor-pointer">
                  <input 
                    type="checkbox" 
                    class="checkbox checkbox-primary" 
                    v-model="engineConfig.enableRealTimeMonitoring"
                  >
                  <span class="label-text ml-2">实时监控</span>
                </label>
                <label class="label cursor-pointer">
                  <input 
                    type="checkbox" 
                    class="checkbox checkbox-primary"
                    v-model="engineConfig.enableAutoReplan"
                  >
                  <span class="label-text ml-2">自动重规划</span>
                </label>
                <label class="label cursor-pointer">
                  <input 
                    type="checkbox" 
                    class="checkbox checkbox-primary"
                    v-model="engineConfig.enableDetailedLogging"
                  >
                  <span class="label-text ml-2">详细日志</span>
                </label>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- 任务调度表单 -->
    <div class="execute-form mb-8" v-if="engineStatus.initialized">
      <div class="card bg-base-100 shadow-xl">
        <div class="card-body">
          <h3 class="card-title mb-6">Plan-and-Execute 任务</h3>
          
          <div class="grid grid-cols-1 lg:grid-cols-2 gap-6">
            <!-- 左侧：基本信息 -->
            <div class="space-y-4">
              <div class="form-control">
                <label class="label">
                  <span class="label-text">任务目标 *</span>
                </label>
                <textarea 
                  class="textarea textarea-bordered h-32" 
                  placeholder="请详细描述您希望完成的目标，例如：分析某个文档、解决特定问题、完成研究任务等..."
                  v-model="dispatchForm.task_description"
                ></textarea>
              </div>
              
              <div class="form-control">
                <label class="label">
                  <span class="label-text">用户ID *</span>
                </label>
                <input 
                  type="text" 
                  class="input input-bordered" 
                  placeholder="输入用户ID"
                  v-model="dispatchForm.user_id"
                >
              </div>
              
              <div class="form-control">
                <label class="label">
                  <span class="label-text">任务类型</span>
                </label>
                <select class="select select-bordered" v-model="dispatchForm.task_type">
                  <option value="">自动检测</option>
                  <option value="research">研究分析</option>
                  <option value="problem_solving">问题解决</option>
                  <option value="data_processing">数据处理</option>
                  <option value="content_creation">内容创作</option>
                  <option value="automation">自动化任务</option>
                </select>
              </div>
              
              <div class="form-control">
                <label class="label">
                  <span class="label-text">规划策略</span>
                </label>
                <select class="select select-bordered" v-model="engineConfig.planningStrategy">
                  <option value="default">默认规划</option>
                  <option value="detailed">详细规划</option>
                  <option value="adaptive">自适应规划</option>
                  <option value="conservative">保守规划</option>
                </select>
                <label class="label">
                  <span class="label-text-alt text-xs">
                    选择适合任务复杂度的规划策略
                  </span>
                </label>
              </div>
            </div>
            
            <!-- 右侧：高级选项 -->
            <div class="space-y-4">
              <div class="form-control">
                <label class="label">
                  <span class="label-text">优先级</span>
                </label>
                <select class="select select-bordered" v-model="dispatchForm.priority">
                  <option value="low">低</option>
                  <option value="normal">普通</option>
                  <option value="high">高</option>
                  <option value="critical">紧急</option>
                </select>
              </div>
              
              <div class="form-control">
                <label class="label">
                  <span class="label-text">最大执行时间（分钟）</span>
                </label>
                <input 
                  type="number" 
                  class="input input-bordered" 
                  min="1" 
                  max="120"
                  v-model.number="dispatchForm.max_execution_time"
                >
              </div>
              
              <div class="form-control">
                <label class="label">
                  <span class="label-text">最大规划步骤</span>
                </label>
                <input 
                  type="number" 
                  class="input input-bordered" 
                  min="1" 
                  max="20"
                  value="10"
                >
              </div>
              
              <div class="form-control">
                <label class="label">
                  <span class="label-text">上下文信息（可选）</span>
                </label>
                <textarea 
                  class="textarea textarea-bordered h-20" 
                  placeholder="提供相关的背景信息或约束条件..."
                  v-model="customParametersJson"
                ></textarea>
              </div>
            </div>
          </div>
          
          <div class="form-control mt-4">
            <label class="label">
              <span class="label-text">执行选项</span>
            </label>
            <div class="flex flex-wrap gap-4">
              <label class="label cursor-pointer">
                <input 
                  type="checkbox" 
                  class="checkbox checkbox-primary" 
                  v-model="engineConfig.enableAutoReplan"
                >
                <span class="label-text ml-2">允许自动重规划</span>
              </label>
              <label class="label cursor-pointer">
                <input 
                  type="checkbox" 
                  class="checkbox checkbox-primary"
                  v-model="engineConfig.enableRealTimeMonitoring"
                >
                <span class="label-text ml-2">实时监控</span>
              </label>
              <label class="label cursor-pointer">
                <input 
                  type="checkbox" 
                  class="checkbox checkbox-primary"
                  v-model="engineConfig.enableDetailedLogging"
                >
                <span class="label-text ml-2">详细日志</span>
              </label>
            </div>
          </div>
          
          <div class="card-actions justify-end mt-6">
            <button class="btn btn-outline" @click="resetForm">重置</button>
            <button 
              class="btn btn-primary"
              @click="dispatchTask"
              :disabled="dispatching"
            >
              <span v-if="dispatching" class="loading loading-spinner loading-sm"></span>
              {{ dispatching ? '执行中...' : '开始执行' }}
            </button>
          </div>
        </div>
      </div>
    </div>

    <!-- 任务列表 -->
    <div class="sessions mb-8" v-if="engineStatus.initialized">
      <div class="card bg-base-100 shadow-xl">
        <div class="card-body">
          <div class="flex justify-between items-center mb-6">
            <h3 class="card-title">Plan-and-Execute 任务列表</h3>
            <div class="flex gap-2">
              <select class="select select-bordered select-sm" v-model="taskFilter.status">
                <option value="">全部状态</option>
                <option value="pending">等待中</option>
                <option value="planning">规划中</option>
                <option value="executing">执行中</option>
                <option value="replanning">重规划中</option>
                <option value="completed">已完成</option>
                <option value="failed">失败</option>
                <option value="cancelled">已取消</option>
              </select>
              <button 
                class="btn btn-outline btn-sm"
                @click="loadTasks"
                :disabled="loadingTasks"
              >
                <span v-if="loadingTasks" class="loading loading-spinner loading-sm"></span>
                刷新
              </button>
            </div>
          </div>
          
          <div v-if="taskList.length === 0" class="text-center py-8 text-base-content/60">
            暂无任务记录
          </div>
          
          <div v-else class="overflow-x-auto">
            <table class="table table-zebra">
              <thead>
                <tr>
                  <th>任务ID</th>
                  <th>目标描述</th>
                  <th>状态</th>
                  <th>进度</th>
                  <th>当前阶段</th>
                  <th>重规划次数</th>
                  <th>开始时间</th>
                  <th>操作</th>
                </tr>
              </thead>
              <tbody>
                <tr v-for="task in taskList" :key="task.task_id">
                  <td>
                    <span class="font-mono text-sm">{{ task.task_id.substring(0, 8) }}...</span>
                  </td>
                  <td>
                    <div class="text-sm max-w-32 truncate" :title="task.description">
                      {{ task.description || task.goal || '-' }}
                    </div>
                  </td>
                  <td>
                    <div :class="['badge', getStatusBadgeClass(task.status)]">
                      {{ getStatusDisplayText(task.status) }}
                    </div>
                  </td>
                  <td>
                    <div class="flex items-center gap-2">
                      <progress 
                        class="progress progress-primary w-16" 
                        :value="Math.round(task.progress)" 
                        max="100"
                      ></progress>
                      <span class="text-sm">{{ Math.round(task.progress) }}%</span>
                    </div>
                  </td>
                  <td>
                    <div class="text-sm">
                      <div class="badge badge-sm" :class="{
                        'badge-info': task.current_phase === 'planning',
                        'badge-primary': task.current_phase === 'executing', 
                        'badge-warning': task.current_phase === 'replanning',
                        'badge-success': task.current_phase === 'completed'
                      }">
                        {{ task.current_phase === 'planning' ? '规划' : 
                           task.current_phase === 'executing' ? '执行' :
                           task.current_phase === 'replanning' ? '重规划' :
                           task.current_phase === 'completed' ? '完成' : task.current_phase || '-' }}
                      </div>
                    </div>
                  </td>
                  <td>
                    <div class="text-center">
                      <span class="badge badge-outline badge-sm">{{ task.replan_count || 0 }}</span>
                    </div>
                  </td>
                  <td>
                    <span class="text-sm">
                      {{ task.started_at ? formatDateTime(task.started_at) : '-' }}
                    </span>
                  </td>
                  <td>
                    <div class="flex gap-1">
                      <button 
                        class="btn btn-ghost btn-xs"
                        @click="viewTaskDetail(task.task_id)"
                      >
                        详情
                      </button>
                      <button 
                        v-if="task.status === 'running' || task.status === 'planning' || task.status === 'executing'"
                        class="btn btn-error btn-xs"
                        @click="cancelTask(task.task_id)"
                      >
                        取消
                      </button>
                    </div>
                  </td>
                </tr>
              </tbody>
            </table>
          </div>
        </div>
      </div>
    </div>

    <!-- 可用架构 -->
    <div class="available-tools mb-8" v-if="engineStatus.initialized">
      <div class="card bg-base-100 shadow-xl">
        <div class="card-body">
          <div class="flex justify-between items-center mb-6">
            <h3 class="card-title">可用架构</h3>
            <button 
              class="btn btn-outline btn-sm"
              @click="loadAvailableArchitectures"
              :disabled="loadingArchitectures"
            >
              <span v-if="loadingArchitectures" class="loading loading-spinner loading-sm"></span>
              刷新
            </button>
          </div>
          
          <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
            <div 
              v-for="arch in availableArchitectures" 
              :key="arch.name"
              class="card bg-base-200 shadow-md"
            >
              <div class="card-body p-4">
                <h4 class="card-title text-base">{{ arch.name }}</h4>
                <p class="text-sm text-base-content/70 mb-3">{{ arch.description }}</p>
                <div class="space-y-2">
                  <div>
                    <h5 class="font-semibold text-sm mb-1">适用场景:</h5>
                    <div class="flex flex-wrap gap-1">
                      <div 
                        v-for="scenario in arch.suitable_for" 
                        :key="scenario"
                        class="badge badge-outline badge-xs"
                      >
                        {{ scenario }}
                      </div>
                    </div>
                  </div>
                  <div>
                    <span class="text-xs text-base-content/60">
                      复杂度范围: {{ arch.complexity_range }}
                    </span>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- 任务详情模态框 -->
    <dialog :class="['modal', { 'modal-open': taskDetailVisible }]">
      <div class="modal-box w-11/12 max-w-5xl max-h-[80vh] overflow-y-auto">
        <div class="flex justify-between items-center mb-6">
          <h3 class="font-bold text-lg">Plan-and-Execute 任务详情</h3>
          <button class="btn btn-sm btn-circle btn-ghost" @click="closeTaskDetail">✕</button>
        </div>
        
        <div v-if="currentTaskDetail" class="space-y-6">
          <!-- 任务基本信息 -->
          <div class="card bg-base-200">
            <div class="card-body">
              <h4 class="card-title text-base mb-4">基本信息</h4>
              <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
                <div>
                  <span class="font-semibold">任务ID:</span>
                  <span class="font-mono text-sm ml-2">{{ currentTaskDetail.task_id }}</span>
                </div>
                <div>
                  <span class="font-semibold">任务类型:</span>
                  <span class="badge badge-outline ml-2">{{ currentTaskDetail.task_type || 'general' }}</span>
                </div>
                <div>
                  <span class="font-semibold">状态:</span>
                  <div :class="['badge ml-2', getStatusBadgeClass(currentTaskDetail.status)]">
                    {{ getStatusDisplayText(currentTaskDetail.status) }}
                  </div>
                </div>
                <div>
                  <span class="font-semibold">当前阶段:</span>
                  <span class="badge badge-sm ml-2" :class="{
                    'badge-info': currentTaskDetail.current_phase === 'planning',
                    'badge-primary': currentTaskDetail.current_phase === 'executing', 
                    'badge-warning': currentTaskDetail.current_phase === 'replanning',
                    'badge-success': currentTaskDetail.current_phase === 'completed'
                  }">
                    {{ currentTaskDetail.current_phase === 'planning' ? '规划' : 
                       currentTaskDetail.current_phase === 'executing' ? '执行' :
                       currentTaskDetail.current_phase === 'replanning' ? '重规划' :
                       currentTaskDetail.current_phase === 'completed' ? '完成' : currentTaskDetail.current_phase || '-' }}
                  </span>
                </div>
                <div>
                  <span class="font-semibold">进度:</span>
                  <div class="flex items-center gap-2 ml-2">
                    <progress 
                      class="progress progress-primary w-24" 
                      :value="Math.round(currentTaskDetail.progress)" 
                      max="100"
                    ></progress>
                    <span class="text-sm">{{ Math.round(currentTaskDetail.progress) }}%</span>
                  </div>
                </div>
                <div>
                  <span class="font-semibold">重规划次数:</span>
                  <span class="badge badge-outline badge-sm ml-2">{{ currentTaskDetail.replan_count || 0 }}</span>
                </div>
                <div>
                  <span class="font-semibold">当前步骤:</span>
                  <span class="ml-2">{{ currentTaskDetail.current_step || '-' }}</span>
                </div>
                <div>
                  <span class="font-semibold">优先级:</span>
                  <span class="badge badge-outline ml-2">{{ currentTaskDetail.priority || 'normal' }}</span>
                </div>
                <div>
                  <span class="font-semibold">开始时间:</span>
                  <span class="ml-2">
                    {{ currentTaskDetail.started_at ? formatDateTime(currentTaskDetail.started_at) : '-' }}
                  </span>
                </div>
                <div>
                  <span class="font-semibold">完成时间:</span>
                  <span class="ml-2">
                    {{ currentTaskDetail.completed_at ? formatDateTime(currentTaskDetail.completed_at) : '-' }}
                  </span>
                </div>
              </div>
            </div>
          </div>

          <!-- 任务目标 -->
          <div class="card bg-base-200">
            <div class="card-body">
              <h4 class="card-title text-base mb-4">任务目标</h4>
              <p class="text-sm">{{ currentTaskDetail.goal || currentTaskDetail.description || '-' }}</p>
            </div>
          </div>

          <!-- 执行计划 -->
          <div v-if="currentTaskDetail.plan" class="card bg-base-200">
            <div class="card-body">
              <h4 class="card-title text-base mb-4">执行计划</h4>
              <div class="space-y-2">
                <div v-for="(step, index) in currentTaskDetail.plan.steps" :key="index" class="flex items-center gap-2">
                  <div class="badge badge-sm">{{ index + 1 }}</div>
                  <span class="text-sm">{{ step.description || step.action }}</span>
                  <div v-if="step.status" class="badge badge-xs" :class="{
                    'badge-success': step.status === 'completed',
                    'badge-primary': step.status === 'executing',
                    'badge-ghost': step.status === 'pending'
                  }">
                    {{ step.status === 'completed' ? '完成' : 
                       step.status === 'executing' ? '执行中' : '待执行' }}
                  </div>
                </div>
              </div>
            </div>
          </div>

          <!-- 执行结果 -->
          <div v-if="currentTaskDetail.result" class="card bg-base-200">
            <div class="card-body">
              <h4 class="card-title text-base mb-4">执行结果</h4>
              <pre class="bg-base-100 p-4 rounded text-sm overflow-x-auto max-h-60">{{ JSON.stringify(currentTaskDetail.result, null, 2) }}</pre>
            </div>
          </div>

          <!-- 错误信息 -->
          <div v-if="currentTaskDetail.error" class="card bg-error/10">
            <div class="card-body">
              <h4 class="card-title text-base mb-4 text-error">错误信息</h4>
              <pre class="bg-base-100 p-4 rounded text-sm overflow-x-auto text-error">{{ currentTaskDetail.error }}</pre>
            </div>
          </div>
        </div>
      </div>
      <form method="dialog" class="modal-backdrop">
        <button>close</button>
      </form>
    </dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted, onUnmounted, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useToast } from '@/composables/useToast'
import { dialog } from '@/composables/useDialog'
import FlowchartVisualization from './FlowchartVisualization.vue'
import { EnhancedStateMachine, StateMachineFactory } from '../utils/enhanced-state-machine'
import {
  DetailedExecutionStatus,
  type EnhancedExecuteRequest,
  type EnhancedExecutionResponse,
  type EnhancedExecutionContext,
  type ExecutionStrategy,
  type StateTransitionEvent,
  type RealTimeUpdate
} from '../types/enhanced-execution'

// Plan-and-Execute 引擎相关类型定义
interface MultiAgentDispatchRequest {
  task_description: string
  user_id: string
  task_type?: string
  priority?: string
  max_execution_time?: number
  custom_parameters?: Record<string, any>
}

interface MultiAgentDispatchResponse {
  task_id: string
  architecture: string
  estimated_duration: number
  confidence: number
  reasoning: string
}

interface DispatchStatus {
  task_id: string
  status: string
  progress: number
  current_step?: string
  current_phase?: string
  task_type?: string
  priority?: string
  goal?: string
  description?: string
  plan?: {
    steps: Array<{
      description?: string
      action?: string
      status?: string
    }>
  }
  replan_count?: number
  result?: any
  error?: string
  started_at?: string
  completed_at?: string
}

interface DispatchStatistics {
  total_dispatches: number
  successful_dispatches: number
  failed_dispatches: number
  average_duration: number
  architecture_usage?: Record<string, number>
}

interface AgentArchitecture {
  name: string
  description: string
  suitable_for: string[]
  complexity_range: string
}

// API 响应类型定义
interface ApiResponse<T = any> {
  success: boolean
  data?: T
  error?: string
}

interface EngineStartResponse {
  success: boolean
  error?: string
}

interface StatisticsResponse {
  total_sessions: number
  completed_sessions: number
  failed_sessions: number
  replan_count: number
  average_execution_time: number
  success_rate: number
}

interface TaskExecuteResponse {
  success: boolean
  data?: {
    task_id: string
    status: string
  }
  error?: string
}

// 组件引用
const flowchartRef = ref<InstanceType<typeof FlowchartVisualization>>()

// 响应式数据
const engineStatus = ref({
  initialized: false,
  running: false,
  activeSessions: 0,
  version: '2.0.0',
  uptime: 0,
  components: {
    planner: { status: 'idle', last_activity: null },
    executor: { status: 'idle', last_activity: null },
    replanner: { status: 'idle', last_activity: null },
    memory_manager: { status: 'idle', last_activity: null }
  }
})

const statistics = ref({
  totalSessions: 0,
  completedSessions: 0,
  failedSessions: 0,
  replanCount: 0,
  averageExecutionTime: 0,
  successRate: 0
})

const taskList = ref<DispatchStatus[]>([])
const availableArchitectures = ref<AgentArchitecture[]>([])
const currentTaskDetail = ref<DispatchStatus | null>(null)
const currentPlanData = ref<any>(null)

// 加载状态
const initializing = ref(false)
const loadingStatistics = ref(false)
const dispatching = ref(false)
const loadingTasks = ref(false)
const loadingArchitectures = ref(false)
const taskDetailVisible = ref(false)
const showArchitectureConfig = ref(false)
const showSessionDetail = ref(false)
const realTimeConnection = ref<WebSocket | null>(null)

// 表单数据
const dispatchForm = reactive<MultiAgentDispatchRequest>({
  task_description: '',
  user_id: 'demo-user',
  task_type: '',
  priority: 'normal',
  max_execution_time: 30,
  custom_parameters: {}
})

// 引擎配置
const engineConfig = reactive({
  planningStrategy: 'default',
  executionMode: 'sequential',
  replanThreshold: 50,
  maxRetries: 3,
  enableRealTimeMonitoring: true,
  enableAutoReplan: true,
  enableDetailedLogging: false
})

const showEngineConfig = ref(false)

// 任务筛选
const taskFilter = reactive({
  status: '',
  priority: ''
})

// 当前执行状态
const currentExecution = ref({
  taskId: null,
  currentStep: null,
  progress: 0,
  phase: 'idle'
})

// 自定义参数JSON字符串
const customParametersJson = ref('')

// Toast实例
const toast = useToast()

// 计算属性
const parsedCustomParameters = computed(() => {
  try {
    return customParametersJson.value ? JSON.parse(customParametersJson.value) : {}
  } catch {
    return {}
  }
})

const getStatusClass = computed(() => (status: string) => {
  switch (status) {
    case 'running':
    case 'healthy':
    case DetailedExecutionStatus.STEP_EXECUTING:
    case DetailedExecutionStatus.TOOL_CALLING:
      return 'text-success'
    case 'error':
    case 'failed':
    case DetailedExecutionStatus.FAILED:
    case DetailedExecutionStatus.STEP_FAILED:
      return 'text-error'
    case 'warning':
    case DetailedExecutionStatus.REQUIRES_INTERVENTION:
    case DetailedExecutionStatus.PAUSED:
      return 'text-warning'
    case 'initializing':
    case DetailedExecutionStatus.INITIALIZED:
    case DetailedExecutionStatus.PLANNING_STARTED:
      return 'text-info'
    default:
      return 'text-base-content'
  }
})

const getStatusText = (status: DetailedExecutionStatus): string => {
  const statusMap: Record<string, string> = {
    'initialized': '已初始化',
    'planning_started': '规划中',
    'step_executing': '执行中',
    'completed': '已完成',
    'failed': '失败',
    'paused': '已暂停',
    'requires_intervention': '需要干预'
  }
  return statusMap[status] || status
}

// 方法
const initializeEngine = async () => {
  console.log('[PlanExecuteDemo] 开始启动Plan-Execute引擎')
  
  initializing.value = true
  try {
    // 启动Plan-Execute引擎
    const response = await invoke('start_plan_execute_engine') as EngineStartResponse
    console.log('[PlanExecuteDemo] 引擎启动响应:', response)
    
    if (response && response.success) {
      engineStatus.value.initialized = true
      engineStatus.value.running = true
      engineStatus.value.uptime = Date.now()
      
      // 更新组件状态
      engineStatus.value.components.planner.status = 'ready'
      engineStatus.value.components.executor.status = 'ready'
      engineStatus.value.components.replanner.status = 'ready'
      engineStatus.value.components.memory_manager.status = 'ready'
      
      toast.success('Plan-Execute引擎启动成功')
    } else {
      throw new Error(response?.error || '引擎启动失败')
    }
    
    // 加载初始数据
    await Promise.all([
      loadStatistics(),
      loadAvailableArchitectures(),
      loadTasks()
    ])
    
    // 建立实时连接
    setupRealTimeConnection()
  } catch (error) {
    console.error('[PlanExecuteDemo] 引擎启动失败:', error)
    toast.error(`引擎启动失败: ${error}`)
    engineStatus.value.running = false
  } finally {
    initializing.value = false
  }
}

const setupRealTimeConnection = () => {
  // 模拟实时连接设置
  console.log('[MultiAgentDemo] 设置实时连接')
}

const onStateTransition = (event: StateTransitionEvent) => {
  console.log('[MultiAgentDemo] 状态转换:', event)
  // 更新UI状态
}

const onRealTimeUpdate = (update: RealTimeUpdate) => {
  console.log('[MultiAgentDemo] 实时更新:', update)
  // 更新流程图和状态
}

const loadStatistics = async () => {
  console.log('[PlanExecuteDemo] 开始加载执行统计')
  
  loadingStatistics.value = true
  try {
    const response = await invoke('get_plan_execute_statistics') as StatisticsResponse
    console.log('[PlanExecuteDemo] 获取执行统计响应:', response)
    
    if (response) {
      statistics.value = {
        totalSessions: response.total_sessions || 0,
        completedSessions: response.completed_sessions || 0,
        failedSessions: response.failed_sessions || 0,
        replanCount: response.replan_count || 0,
        averageExecutionTime: response.average_execution_time || 0,
        successRate: response.success_rate || 0
      }
      console.log('[PlanExecuteDemo] 执行统计已更新:', statistics.value)
    }
  } catch (error) {
    console.error('[PlanExecuteDemo] 获取执行统计异常:', error)
    toast.error(`获取执行统计失败: ${error}`)
  } finally {
    loadingStatistics.value = false
  }
}

const loadAvailableArchitectures = async () => {
  console.log('[PlanExecuteDemo] 开始加载可用架构')
  
  loadingArchitectures.value = true
  try {
    const response = await invoke('list_plan_execute_architectures') as AgentArchitecture[]
    console.log('[PlanExecuteDemo] 获取可用架构响应:', response)
    
    if (response) {
      availableArchitectures.value = response
      console.log('[PlanExecuteDemo] 可用架构已更新:', availableArchitectures.value)
    }
  } catch (error) {
    console.error('[PlanExecuteDemo] 获取可用架构异常:', error)
    toast.error(`获取可用架构失败: ${error}`)
  } finally {
    loadingArchitectures.value = false
  }
}

const loadTasks = async () => {
  console.log('[PlanExecuteDemo] 开始加载任务列表')
  
  loadingTasks.value = true
  try {
    const response = await invoke('get_plan_execute_sessions', { filter: taskFilter }) as DispatchStatus[]
    console.log('[PlanExecuteDemo] 获取任务列表响应:', response)
    
    if (response) {
      taskList.value = response
    } else {
      taskList.value = []
    }
    console.log('[PlanExecuteDemo] 任务列表已更新')
  } catch (error) {
    console.error('[PlanExecuteDemo] 获取任务列表异常:', error)
    toast.error(`获取任务列表失败: ${error}`)
    taskList.value = []
  } finally {
    loadingTasks.value = false
  }
}

const dispatchTask = async () => {
  console.log('[PlanExecuteDemo] 开始执行Plan-and-Execute任务')
  console.log('[PlanExecuteDemo] 表单数据:', dispatchForm)
  
  // 表单验证
  if (!dispatchForm.task_description.trim() || dispatchForm.task_description.length < 10) {
    console.warn('[PlanExecuteDemo] 表单验证失败: 任务描述不足10个字符')
    toast.error('请输入至少10个字符的任务描述')
    return
  }
  
  if (!dispatchForm.user_id.trim()) {
    console.warn('[PlanExecuteDemo] 表单验证失败: 用户ID为空')
    toast.error('请输入用户ID')
    return
  }
  
  console.log('[PlanExecuteDemo] 表单验证通过，开始执行任务')
  dispatching.value = true
  
  try {
    const requestData = {
      goal: dispatchForm.task_description,
      task_type: dispatchForm.task_type || 'general',
      priority: dispatchForm.priority,
      max_execution_time: (dispatchForm.max_execution_time || 30) * 60, // 转换为秒
      context: customParametersJson.value || '',
      config: {
        planning_strategy: engineConfig.planningStrategy,
        execution_mode: engineConfig.executionMode,
        replan_threshold: engineConfig.replanThreshold / 100,
        max_retries: engineConfig.maxRetries,
        enable_real_time_monitoring: engineConfig.enableRealTimeMonitoring,
        enable_auto_replan: engineConfig.enableAutoReplan,
        enable_detailed_logging: engineConfig.enableDetailedLogging
      },
      metadata: {
        user_id: dispatchForm.user_id,
        session_id: `session-${Date.now()}`
      }
    }
    
    console.log('[PlanExecuteDemo] 调用execute_plan_and_execute_task API，参数:', requestData)
    const response = await invoke('execute_plan_and_execute_task', { taskData: requestData }) as TaskExecuteResponse
    console.log('[PlanExecuteDemo] 任务执行响应:', response)
    
    if (response && response.success) {
      const taskResponse = response.data
      console.log('[PlanExecuteDemo] 任务执行成功，响应数据:', taskResponse)
      if (taskResponse) {
        toast.success(`任务开始执行！任务ID: ${taskResponse.task_id}`)
      } else {
        toast.success('任务开始执行！')
      }
      
      console.log('[PlanExecuteDemo] 重新加载任务列表')
      await loadTasks()
      
      console.log('[PlanExecuteDemo] 重置表单')
      resetForm()
    } else {
      throw new Error(response?.error || '任务执行失败')
    }
  } catch (error) {
    console.error('[PlanExecuteDemo] 任务执行异常:', error)
    toast.error('任务执行失败: ' + error)
  } finally {
    console.log('[PlanExecuteDemo] 任务执行流程结束')
    dispatching.value = false
  }
}

const viewTaskDetail = async (taskId: string) => {
  console.log('[PlanExecuteDemo] 查看任务详情，taskId:', taskId)
  
  try {
    const response = await invoke('get_plan_execute_session_detail', { session_id: taskId }) as DispatchStatus
    console.log('[PlanExecuteDemo] 任务详情响应:', response)
    
    if (response) {
      currentTaskDetail.value = response
      taskDetailVisible.value = true
    }
  } catch (error) {
    console.error('[PlanExecuteDemo] 获取任务详情异常:', error)
    toast.error('获取任务详情失败: ' + error)
  }
}

const cancelTask = async (taskId: string) => {
  console.log('[PlanExecuteDemo] 取消任务，taskId:', taskId)
  
  try {
    const confirmed = await dialog.confirm('确定要取消此任务吗？')
    if (!confirmed) return
    
    const response = await invoke('cancel_plan_execute_session', { session_id: taskId }) as ApiResponse
    console.log('[PlanExecuteDemo] 取消任务响应:', response)
    
    toast.success('任务已取消')
    await loadTasks()
  } catch (error) {
    console.error('[PlanExecuteDemo] 取消任务异常:', error)
    toast.error('取消任务失败: ' + error)
  }
}

const closeTaskDetail = () => {
  taskDetailVisible.value = false
  currentTaskDetail.value = null
}

const resetForm = () => {
  console.log('[PlanExecuteDemo] 重置表单')
  
  Object.assign(dispatchForm, {
    task_description: '',
    user_id: 'demo-user',
    task_type: '',
    priority: 'normal',
    max_execution_time: 30,
    custom_parameters: {}
  })
  
  customParametersJson.value = ''
}

// 工具函数
const formatUptime = (seconds: number): string => {
  const hours = Math.floor(seconds / 3600)
  const minutes = Math.floor((seconds % 3600) / 60)
  return `${hours}小时${minutes}分钟`
}

const formatDuration = (seconds: number): string => {
  if (seconds < 60) return `${seconds}秒`
  if (seconds < 3600) return `${Math.floor(seconds / 60)}分钟`
  return `${Math.floor(seconds / 3600)}小时${Math.floor((seconds % 3600) / 60)}分钟`
}

const formatDateTime = (dateStr: string): string => {
  return new Date(dateStr).toLocaleString('zh-CN')
}

const getStatusBadgeClass = (status: string) => {
  switch (status) {
    case 'completed': return 'badge-success'
    case 'running': case 'pending': return 'badge-primary'
    case 'failed': case 'cancelled': return 'badge-error'
    default: return 'badge-info'
  }
}

const getStatusDisplayText = (status: string): string => {
  const statusMap: Record<string, string> = {
    'pending': '等待中',
    'running': '运行中',
    'completed': '已完成',
    'failed': '失败',
    'cancelled': '已取消'
  }
  return statusMap[status] || status
}

// 生命周期
onMounted(async () => {
  console.log('[PlanExecuteDemo] 组件挂载开始，执行初始化流程')
  
  try {
    // 检查引擎状态，默认未初始化
    engineStatus.value.initialized = false
    
    console.log('[PlanExecuteDemo] 组件初始化完成，等待用户手动初始化引擎')
  } catch (error) {
    console.error('[PlanExecuteDemo] 组件初始化过程中发生异常:', error)
  }
})

onUnmounted(() => {
  if (realTimeConnection.value) {
    realTimeConnection.value.close()
  }
})
</script>

<style scoped>
.plan-execute-demo {
  @apply p-6 max-w-7xl mx-auto;
}

.demo-header {
  @apply text-center mb-8;
}

.demo-description {
  @apply leading-relaxed;
}

@media (max-width: 768px) {
  .plan-execute-demo {
    @apply p-4;
  }
}
</style>