<template>
  <div class="page-content-padded safe-top">
    <div class="mb-6">
      <h1 class="text-3xl font-bold text-gray-800 mb-2">工作流监控</h1>
      <p class="text-gray-600">查看和管理工作流执行状态</p>
    </div>

    <!-- 统计信息卡片 -->
    <div class="grid grid-cols-1 md:grid-cols-4 gap-4 mb-6">
      <div class="stat bg-base-100 shadow rounded-lg">
        <div class="stat-title">总工作流</div>
        <div class="stat-value text-primary">{{ statistics.total_workflows || 0 }}</div>
      </div>
      <div class="stat bg-base-100 shadow rounded-lg">
        <div class="stat-title">总执行次数</div>
        <div class="stat-value text-secondary">{{ statistics.total_executions || 0 }}</div>
      </div>
      <div class="stat bg-base-100 shadow rounded-lg">
        <div class="stat-title">成功执行</div>
        <div class="stat-value text-success">{{ statistics.successful_executions || 0 }}</div>
      </div>
      <div class="stat bg-base-100 shadow rounded-lg">
        <div class="stat-title">正在运行</div>
        <div class="stat-value text-warning">{{ statistics.running_executions || 0 }}</div>
      </div>
    </div>

    <!-- 操作按钮 -->
    <div class="flex gap-4 mb-6">
      <button class="btn btn-primary" @click="refreshExecutions">
        <svg class="w-4 h-4 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
        </svg>
        刷新
      </button>
      <button class="btn btn-outline" @click="refreshStatistics">
        <svg class="w-4 h-4 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 19v-6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2a2 2 0 002-2zm0 0V9a2 2 0 012-2h2a2 2 0 012 2v10m-6 0a2 2 0 002 2h2a2 2 0 002-2m0 0V5a2 2 0 012-2h2a2 2 0 012 2v14a2 2 0 01-2 2h-2a2 2 0 01-2-2z" />
        </svg>
        更新统计
      </button>
    </div>

    <!-- 执行列表 -->
    <div class="card bg-base-100 shadow-xl">
      <div class="card-header">
        <h2 class="card-title text-xl font-semibold">工作流执行列表</h2>
      </div>
      <div class="card-body">
        <div v-if="loading" class="flex justify-center py-8">
          <span class="loading loading-spinner loading-lg"></span>
        </div>
        
        <div v-else-if="executions.length === 0" class="text-center py-8 text-gray-500">
          <svg class="w-16 h-16 mx-auto mb-4 text-gray-300" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5H7a2 2 0 00-2 2v10a2 2 0 002 2h8a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2" />
          </svg>
          <p>暂无工作流执行记录</p>
        </div>

        <div v-else class="overflow-x-auto">
          <table class="table table-zebra w-full">
            <thead>
              <tr>
                <th>执行ID</th>
                <th>工作流ID</th>
                <th>状态</th>
                <th>进度</th>
                <th>当前步骤</th>
                <th>开始时间</th>
                <th>操作</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="execution in executions" :key="execution.execution_id">
                <td>
                  <code class="text-sm bg-gray-100 px-2 py-1 rounded">{{ execution.execution_id.substring(0, 8) }}...</code>
                </td>
                <td>
                  <div class="flex flex-col gap-1">
                    <span class="font-medium">{{ execution.workflow_id }}</span>
                    <div v-if="execution.result" class="text-xs text-gray-500">
                      {{ getExecutionResultSummary(execution.result) }}
                    </div>
                  </div>
                </td>
                <td>
                  <div class="badge" :class="getStatusBadgeClass(execution.status)">{{ getStatusText(execution.status) }}</div>
                </td>
                <td>
                  <div class="flex items-center gap-2">
                    <progress class="progress w-20" :class="getProgressClass(execution.status)" :value="execution.progress" max="100"></progress>
                    <span class="text-sm">{{ Math.round(execution.progress) }}%</span>
                  </div>
                  <div class="text-xs text-gray-500 mt-1">
                    {{ execution.completed_steps }}/{{ execution.total_steps }} 步骤
                  </div>
                </td>
                <td>
                  <div class="flex flex-col gap-1">
                    <span v-if="execution.current_step" class="text-sm font-medium">{{ formatStepTitle(execution.current_step) }}</span>
                    <span v-else class="text-gray-400 text-sm">-</span>
                  </div>
                </td>
                <td>
                  <div class="flex flex-col gap-1 text-sm">
                    <span>{{ formatTime(execution.started_at) }}</span>
                    <span v-if="execution.completed_at" class="text-green-600 text-xs">
                      完成: {{ formatTime(execution.completed_at) }}
                    </span>
                    <span v-else-if="execution.status === 'Running'" class="text-blue-600 text-xs">
                      运行中: {{ calculateRunningTime(execution.started_at) }}
                    </span>
                  </div>
                </td>
                <td>
                  <div class="flex gap-2">
                    <button class="btn btn-sm btn-outline" @click="viewExecution(execution.execution_id)">
                      查看
                    </button>
                    <button class="btn btn-sm btn-info" @click="viewExecutionFlowchart(execution.execution_id)">
                      流程图
                    </button>
                    <button 
                      v-if="execution.status === 'Running'" 
                      class="btn btn-sm btn-error" 
                      @click="cancelExecution(execution.execution_id)"
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

    <!-- 执行详情模态框 -->
    <dialog ref="executionModal" class="modal">
      <div class="modal-box w-11/12 max-w-4xl">
        <h3 class="font-bold text-lg mb-4">执行详情</h3>
        <div v-if="selectedExecution" class="space-y-4">
          <div class="grid grid-cols-2 gap-4">
            <div>
              <label class="label">执行ID</label>
              <code class="block bg-gray-100 p-2 rounded text-sm">{{ selectedExecution.execution_id }}</code>
            </div>
            <div>
              <label class="label">工作流ID</label>
              <div class="bg-gray-100 p-2 rounded text-sm">{{ selectedExecution.workflow_id }}</div>
            </div>
            <div>
              <label class="label">状态</label>
              <div class="badge" :class="getStatusBadgeClass(selectedExecution.status)">{{ getStatusText(selectedExecution.status) }}</div>
            </div>
            <div>
              <label class="label">进度</label>
              <div class="flex items-center gap-2">
                <progress class="progress flex-1" :class="getProgressClass(selectedExecution.status)" :value="selectedExecution.progress" max="100"></progress>
                <span class="text-sm">{{ Math.round(selectedExecution.progress) }}%</span>
              </div>
            </div>
            <div>
              <label class="label">开始时间</label>
              <div class="bg-gray-100 p-2 rounded text-sm">{{ formatTime(selectedExecution.started_at) }}</div>
            </div>
            <div>
              <label class="label">完成时间</label>
              <div class="bg-gray-100 p-2 rounded text-sm">{{ selectedExecution.completed_at ? formatTime(selectedExecution.completed_at) : '-' }}</div>
            </div>
          </div>
          
          <div v-if="selectedExecution.error" class="alert alert-error">
            <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
            </svg>
            <div>
              <h4 class="font-bold">执行错误</h4>
              <p>{{ selectedExecution.error }}</p>
            </div>
          </div>
        </div>
        
        <div class="modal-action">
          <button class="btn" @click="closeModal">关闭</button>
        </div>
      </div>
    </dialog>

    <!-- 工作流程图模态框 -->
    <dialog ref="flowchartModal" class="modal">
      <div class="modal-box w-11/12 max-w-7xl max-h-[90vh]">
        <div class="flex justify-between items-center mb-4">
          <h3 class="font-bold text-lg">工作流程图</h3>
          <div class="flex gap-2">
            <button class="btn btn-sm btn-outline" @click="refreshFlowchart">
              <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
              </svg>
              刷新
            </button>
          </div>
        </div>

        <div v-if="loadingFlowchart" class="flex justify-center py-12">
          <span class="loading loading-spinner loading-lg"></span>
        </div>

        <div v-else-if="workflowPlan" class="space-y-6">
          <!-- 工作流信息 -->
          <div class="workflow-info-card">
            <div class="flex items-start justify-between mb-4">
              <div>
                <h4 class="font-semibold text-xl text-gray-800 mb-1">{{ workflowPlan.name }}</h4>
                <p v-if="workflowPlan.description" class="text-gray-600">{{ workflowPlan.description }}</p>
              </div>
              <div class="workflow-progress-circle">
                <div class="progress-ring">
                  <svg class="progress-ring-svg" width="80" height="80">
                    <circle
                      class="progress-ring-circle-bg"
                      cx="40"
                      cy="40"
                      r="30"
                      stroke="#e5e7eb"
                      stroke-width="6"
                      fill="transparent"
                    />
                    <circle
                      class="progress-ring-circle"
                      cx="40"
                      cy="40"
                      r="30"
                      stroke="#3b82f6"
                      stroke-width="6"
                      fill="transparent"
                      :stroke-dasharray="188.4"
                      :stroke-dashoffset="188.4 * (1 - (workflowPlan.completed_steps / workflowPlan.total_steps))"
                    />
                  </svg>
                  <div class="progress-text">
                    {{ Math.round((workflowPlan.completed_steps / workflowPlan.total_steps) * 100) }}%
                  </div>
                </div>
              </div>
            </div>
            
            <div class="grid grid-cols-4 gap-4">
              <div class="stats-item">
                <div class="stats-icon bg-blue-100 text-blue-600">
                  <svg class="w-5 h-5" fill="currentColor" viewBox="0 0 20 20">
                    <path d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z"/>
                  </svg>
                </div>
                <div class="stats-content">
                  <div class="stats-number text-blue-600">{{ workflowPlan.total_steps }}</div>
                  <div class="stats-label">总步骤</div>
                </div>
              </div>
              <div class="stats-item">
                <div class="stats-icon bg-green-100 text-green-600">
                  <svg class="w-5 h-5" fill="currentColor" viewBox="0 0 20 20">
                    <path fill-rule="evenodd" d="M16.707 5.293a1 1 0 010 1.414l-8 8a1 1 0 01-1.414 0l-4-4a1 1 0 011.414-1.414L8 12.586l7.293-7.293a1 1 0 011.414 0z" clip-rule="evenodd"/>
                  </svg>
                </div>
                <div class="stats-content">
                  <div class="stats-number text-green-600">{{ workflowPlan.completed_steps }}</div>
                  <div class="stats-label">已完成</div>
                </div>
              </div>
              <div class="stats-item">
                <div class="stats-icon bg-red-100 text-red-600">
                  <svg class="w-5 h-5" fill="currentColor" viewBox="0 0 20 20">
                    <path fill-rule="evenodd" d="M4.293 4.293a1 1 0 011.414 0L10 8.586l4.293-4.293a1 1 0 111.414 1.414L11.414 10l4.293 4.293a1 1 0 01-1.414 1.414L10 11.414l-4.293 4.293a1 1 0 01-1.414-1.414L8.586 10 4.293 5.707a1 1 0 010-1.414z" clip-rule="evenodd"/>
                  </svg>
                </div>
                <div class="stats-content">
                  <div class="stats-number text-red-600">{{ workflowPlan.failed_steps }}</div>
                  <div class="stats-label">失败</div>
                </div>
              </div>
              <div class="stats-item">
                <div class="stats-icon bg-yellow-100 text-yellow-600">
                  <svg class="w-5 h-5" fill="currentColor" viewBox="0 0 20 20">
                    <path fill-rule="evenodd" d="M3 10a1 1 0 011-1h12a1 1 0 110 2H4a1 1 0 01-1-1z" clip-rule="evenodd"/>
                  </svg>
                </div>
                <div class="stats-content">
                  <div class="stats-number text-yellow-600">{{ workflowPlan.skipped_steps }}</div>
                  <div class="stats-label">跳过</div>
                </div>
              </div>
            </div>
          </div>

          <!-- 流程图 -->
          <div class="bg-white border rounded-lg p-6 overflow-auto" style="min-height: 500px;">
            <div class="workflow-flowchart">
              <div class="flex flex-col items-center gap-6">
                <!-- 开始节点 -->
                <div class="flowchart-node start-node">
                  <div class="node-circle bg-blue-500 text-white">
                    <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 3v18m0-18l4 4m-4-4l-4 4"/>
                    </svg>
                  </div>
                  <div class="node-label">开始</div>
                </div>

                <!-- 连接线 -->
                <div class="connector-line"></div>

                <!-- 判断节点 -->
                <div class="flowchart-node decision-node" @click="showWorkflowCondition">
                  <div class="decision-diamond bg-purple-100 border-2 border-purple-300">
                    <span class="text-purple-700 font-semibold">是否满足条件</span>
                  </div>
                </div>

                <!-- 分支连接线 -->
                <div class="branch-container">
                  <div class="branch-line left-branch">
                    <span class="branch-label">是</span>
                  </div>
                  <div class="branch-line right-branch">
                    <span class="branch-label">否</span>
                  </div>
                </div>

                <!-- 执行步骤区域 -->
                <div class="execution-area">
                  <div class="steps-container">
                    <!-- 左侧执行路径 -->
                    <div class="execution-path left-path">
                      <div 
                        v-for="(step, index) in workflowPlan.steps" 
                        :key="step.step_id"
                        class="execution-step"
                      >
                        <!-- 步骤节点 -->
                        <div 
                          class="flowchart-node step-node cursor-pointer transition-all hover:shadow-lg"
                          :class="getStepNodeClass(step.status)"
                          @click="viewStepDetails(step)"
                        >
                          <div class="node-circle" :class="getStepIndicatorClass(step.status)">
                            <span v-if="step.status === 'Completed'" class="text-white">✓</span>
                            <span v-else-if="step.status === 'Failed'" class="text-white">✗</span>
                            <span v-else-if="step.status === 'Running'" class="text-white">⋯</span>
                            <span v-else-if="step.status === 'Skipped'" class="text-white">⊘</span>
                            <span v-else class="text-white">{{ index + 1 }}</span>
                          </div>
                          <div class="node-label">{{ formatStepTitle(step.step_name) }}</div>
                          <div class="node-sublabel">{{ step.step_id.substring(0, 8) }}...</div>
                          
                          <!-- 步骤摘要信息 -->
                          <div class="step-summary">
                            <div v-if="getStepSummary(step)" class="summary-text" :title="getStepSummary(step)">
                              {{ truncateText(getStepSummary(step), 50) }}
                            </div>
                          </div>
                          
                          <!-- 状态信息 -->
                          <div class="status-info">
                            <div class="badge badge-xs" :class="getStepStatusBadgeClass(step.status)">
                              {{ getStepStatusText(step.status) }}
                            </div>
                            <div v-if="step.duration_ms" class="text-xs text-gray-500 mt-1">
                              {{ formatDuration(step.duration_ms) }}
                            </div>
                          </div>

                          <!-- 数据指示器 -->
                          <div class="data-indicators">
                            <div v-if="step.result_data" class="indicator-item" title="有执行结果">
                              <svg class="w-3 h-3 text-green-500" fill="currentColor" viewBox="0 0 20 20">
                                <path d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z"/>
                              </svg>
                              <span class="text-xs">结果</span>
                            </div>
                            <div v-if="step.tool_result" class="indicator-item" title="有工具执行结果">
                              <svg class="w-3 h-3 text-blue-500" fill="currentColor" viewBox="0 0 20 20">
                                <path d="M13.586 3.586a2 2 0 112.828 2.828l-.793.793-2.828-2.828.793-.793zM11.379 5.793L3 14.172V17h2.828l8.38-8.379-2.83-2.828z"/>
                              </svg>
                              <span class="text-xs">工具</span>
                            </div>
                            <div v-if="step.dependencies && step.dependencies.length > 0" class="indicator-item" :title="`依赖: ${step.dependencies.join(', ')}`">
                              <svg class="w-3 h-3 text-orange-500" fill="currentColor" viewBox="0 0 20 20">
                                <path fill-rule="evenodd" d="M12.586 4.586a2 2 0 112.828 2.828l-3 3a2 2 0 01-2.828 0 1 1 0 00-1.414 1.414 4 4 0 005.656 0l3-3a4 4 0 00-5.656-5.656l-1.5 1.5a1 1 0 101.414 1.414l1.5-1.5zm-5 5a2 2 0 012.828 0 1 1 0 101.414-1.414 4 4 0 00-5.656 0l-3 3a4 4 0 105.656 5.656l1.5-1.5a1 1 0 10-1.414-1.414l-1.5 1.5a2 2 0 11-2.828-2.828l3-3z" clip-rule="evenodd"/>
                              </svg>
                              <span class="text-xs">{{ step.dependencies.length }}</span>
                            </div>
                          </div>

                          <!-- 错误提示 -->
                          <div v-if="step.error" class="error-indicator">
                            <svg class="w-4 h-4 text-red-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"/>
                            </svg>
                          </div>
                        </div>

                        <!-- 步骤间连接线 -->
                        <div v-if="index < workflowPlan.steps.length - 1" class="step-connector"></div>
                      </div>
                    </div>

                    <!-- 右侧执行路径（备用路径） -->
                    <div class="execution-path right-path">
                      <div class="flowchart-node step-node alternative-step">
                        <div class="node-circle bg-gray-400">
                          <svg class="w-6 h-6 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/>
                          </svg>
                        </div>
                        <div class="node-label">执行操作</div>
                        <div class="node-sublabel">备用路径</div>
                      </div>
                    </div>
                  </div>
                </div>

                <!-- 汇聚连接线 -->
                <div class="merge-lines">
                  <div class="merge-line left-merge"></div>
                  <div class="merge-line right-merge"></div>
                </div>

                <!-- 结束节点 -->
                <div class="flowchart-node end-node">
                  <div class="node-circle bg-green-500 text-white">
                    <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7"/>
                    </svg>
                  </div>
                  <div class="node-label">结束</div>
                </div>
              </div>
            </div>
          </div>
        </div>

        <div class="modal-action">
          <button class="btn" @click="closeFlowchartModal">关闭</button>
        </div>
      </div>
    </dialog>

    <!-- 步骤详情模态框 -->
    <dialog ref="stepDetailsModal" class="modal">
      <div class="modal-box w-11/12 max-w-4xl">
        <h3 class="font-bold text-lg mb-4">步骤详情</h3>
        <div v-if="selectedStep" class="space-y-4">
          <!-- 步骤概览卡片 -->
          <div class="step-overview-card mb-6">
            <div class="flex items-start gap-4">
              <div class="step-status-icon" :class="getStepIndicatorClass(selectedStep.status)">
                <span v-if="selectedStep.status === 'Completed'" class="text-white">✓</span>
                <span v-else-if="selectedStep.status === 'Failed'" class="text-white">✗</span>
                <span v-else-if="selectedStep.status === 'Running'" class="text-white">⋯</span>
                <span v-else-if="selectedStep.status === 'Skipped'" class="text-white">⊘</span>
                <span v-else class="text-white">?</span>
              </div>
              <div class="flex-1">
                <h3 class="text-lg font-semibold text-gray-800">{{ formatStepTitle(selectedStep.step_name) }}</h3>
                <p class="text-sm text-gray-600 mt-1">{{ getStepSummary(selectedStep) || '暂无描述' }}</p>
                <div class="flex items-center gap-4 mt-3">
                  <div class="badge" :class="getStepStatusBadgeClass(selectedStep.status)">{{ getStepStatusText(selectedStep.status) }}</div>
                  <span class="text-sm text-gray-500">执行时长: {{ formatDuration(selectedStep.duration_ms) }}</span>
                  <span v-if="selectedStep.retry_count > 0" class="text-sm text-yellow-600">重试 {{ selectedStep.retry_count }} 次</span>
                </div>
              </div>
            </div>
          </div>

          <div class="grid grid-cols-2 gap-4">
            <div>
              <label class="label">步骤ID</label>
              <code class="block bg-gray-100 p-2 rounded text-sm">{{ selectedStep.step_id }}</code>
            </div>
            <div>
              <label class="label">步骤类型</label>
              <div class="bg-gray-100 p-2 rounded text-sm">{{ getStepType(selectedStep) }}</div>
            </div>
            <div>
              <label class="label">开始时间</label>
              <div class="bg-gray-100 p-2 rounded text-sm">{{ selectedStep.started_at ? formatTime(selectedStep.started_at) : '-' }}</div>
            </div>
            <div>
              <label class="label">完成时间</label>
              <div class="bg-gray-100 p-2 rounded text-sm">{{ selectedStep.completed_at ? formatTime(selectedStep.completed_at) : '-' }}</div>
            </div>
            <div>
              <label class="label">数据类型</label>
              <div class="bg-gray-100 p-2 rounded text-sm">
                <div class="flex flex-wrap gap-1">
                  <span v-if="selectedStep.result_data" class="badge badge-sm badge-success">执行结果</span>
                  <span v-if="selectedStep.tool_result" class="badge badge-sm badge-info">工具结果</span>
                  <span v-if="selectedStep.error" class="badge badge-sm badge-error">错误信息</span>
                  <span v-if="!selectedStep.result_data && !selectedStep.tool_result && !selectedStep.error" class="text-gray-500">无数据</span>
                </div>
              </div>
            </div>
            <div>
              <label class="label">依赖关系</label>
              <div class="bg-gray-100 p-2 rounded text-sm">
                <div v-if="selectedStep.dependencies && selectedStep.dependencies.length > 0" class="space-y-1">
                  <div v-for="dep in selectedStep.dependencies" :key="dep" class="badge badge-outline badge-sm">{{ dep.substring(0, 8) }}...</div>
                </div>
                <span v-else class="text-gray-500">无依赖</span>
              </div>
            </div>
          </div>
          
          <!-- 结果数据 -->
          <div v-if="selectedStep.result_data">
            <label class="label">执行结果</label>
            <div class="formatted-data-container">
              <div class="data-tabs">
                <button 
                  class="tab-button" 
                  :class="{ active: resultDataTab === 'formatted' }"
                  @click="resultDataTab = 'formatted'"
                >
                  格式化显示
                </button>
                <button 
                  class="tab-button" 
                  :class="{ active: resultDataTab === 'raw' }"
                  @click="resultDataTab = 'raw'"
                >
                  原始数据
                </button>
              </div>
              <div class="data-content">
                <div v-if="resultDataTab === 'formatted'" class="formatted-content">
                  <div v-html="formatJsonData(selectedStep.result_data)"></div>
                </div>
                <div v-else class="raw-content">
                  <pre class="json-code">{{ JSON.stringify(selectedStep.result_data, null, 2) }}</pre>
                </div>
              </div>
            </div>
          </div>
          
          <!-- 工具执行结果 -->
          <div v-if="selectedStep.tool_result">
            <label class="label">工具执行结果</label>
            <div class="formatted-data-container">
              <div class="data-tabs">
                <button 
                  class="tab-button" 
                  :class="{ active: toolResultTab === 'formatted' }"
                  @click="toolResultTab = 'formatted'"
                >
                  格式化显示
                </button>
                <button 
                  class="tab-button" 
                  :class="{ active: toolResultTab === 'raw' }"
                  @click="toolResultTab = 'raw'"
                >
                  原始数据
                </button>
              </div>
              <div class="data-content">
                <div v-if="toolResultTab === 'formatted'" class="formatted-content">
                  <div v-html="formatJsonData(selectedStep.tool_result)"></div>
                </div>
                <div v-else class="raw-content">
                  <pre class="json-code">{{ JSON.stringify(selectedStep.tool_result, null, 2) }}</pre>
                </div>
              </div>
            </div>
          </div>
          
          <!-- 错误信息 -->
          <div v-if="selectedStep.error" class="alert alert-error">
            <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
            </svg>
            <div>
              <h4 class="font-bold">执行错误</h4>
              <p>{{ selectedStep.error }}</p>
            </div>
          </div>
        </div>
        
        <div class="modal-action">
          <button class="btn" @click="closeStepDetailsModal">关闭</button>
        </div>
      </div>
    </dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'

interface WorkflowExecution {
  execution_id: string
  workflow_id: string
  status: string
  started_at: string
  completed_at?: string
  current_step?: string
  total_steps: number
  completed_steps: number
  progress: number
  error?: string
  result?: any
}

interface WorkflowStatistics {
  total_workflows: number
  total_executions: number
  successful_executions: number
  failed_executions: number
  running_executions: number
}

interface WorkflowStepDetail {
  step_id: string
  step_name: string
  status: string
  started_at?: string
  completed_at?: string
  duration_ms: number
  result_data?: any
  error?: string
  retry_count: number
  dependencies: string[]
  tool_result?: any
}

interface WorkflowExecutionPlan {
  plan_id: string
  name: string
  description?: string
  steps: WorkflowStepDetail[]
  total_steps: number
  completed_steps: number
  failed_steps: number
  skipped_steps: number
}

const executions = ref<WorkflowExecution[]>([])
const statistics = ref<WorkflowStatistics>({
  total_workflows: 0,
  total_executions: 0,
  successful_executions: 0,
  failed_executions: 0,
  running_executions: 0
})
const loading = ref(false)
const loadingFlowchart = ref(false)
const selectedExecution = ref<WorkflowExecution | null>(null)
const workflowPlan = ref<WorkflowExecutionPlan | null>(null)
const selectedStep = ref<WorkflowStepDetail | null>(null)
const executionModal = ref<HTMLDialogElement>()
const flowchartModal = ref<HTMLDialogElement>()
const stepDetailsModal = ref<HTMLDialogElement>()

// 数据显示选项卡状态
const resultDataTab = ref<'formatted' | 'raw'>('formatted')
const toolResultTab = ref<'formatted' | 'raw'>('formatted')

// 确保Agent管理器已初始化
const ensureAgentManagerInitialized = async () => {
  try {
    // 尝试初始化Agent管理器（如果已初始化会返回成功信息）
    await invoke('initialize_agent_manager')
  } catch (error) {
    console.warn('Agent manager initialization warning:', error)
  }
}

// 获取执行列表
const refreshExecutions = async () => {
  loading.value = true
  try {
    // 确保Agent管理器已初始化
    await ensureAgentManagerInitialized()
    
    const result = await invoke<WorkflowExecution[]>('list_workflow_executions')
    executions.value = result
  } catch (error) {
    console.error('获取执行列表失败:', error)
  } finally {
    loading.value = false
  }
}

// 获取统计信息
const refreshStatistics = async () => {
  try {
    // 确保Agent管理器已初始化
    await ensureAgentManagerInitialized()
    
    const result = await invoke<WorkflowStatistics>('get_workflow_statistics')
    statistics.value = result
  } catch (error) {
    console.error('获取统计信息失败:', error)
  }
}

// 查看执行详情
const viewExecution = async (executionId: string) => {
  try {
    // 确保Agent管理器已初始化
    await ensureAgentManagerInitialized()
    
    const result = await invoke<WorkflowExecution>('get_workflow_execution', { executionId })
    if (result) {
      selectedExecution.value = result
      executionModal.value?.showModal()
    }
  } catch (error) {
    console.error('获取执行详情失败:', error)
  }
}

// 查看执行流程图
const viewExecutionFlowchart = async (executionId: string) => {
  try {
    // 确保Agent管理器已初始化
    await ensureAgentManagerInitialized()
    
    loadingFlowchart.value = true
    const result = await invoke<WorkflowExecutionPlan>('get_workflow_execution_details', { executionId })
    
    if (result) {
      workflowPlan.value = result
      flowchartModal.value?.showModal()
    } else {
      console.error('未找到工作流详情')
    }
  } catch (error) {
    console.error('获取工作流详情失败:', error)
  } finally {
    loadingFlowchart.value = false
  }
}

// 刷新流程图
const refreshFlowchart = async () => {
  if (workflowPlan.value) {
    await viewExecutionFlowchart(workflowPlan.value.plan_id)
  }
}

// 查看步骤详情
const viewStepDetails = (step: WorkflowStepDetail) => {
  selectedStep.value = step
  stepDetailsModal.value?.showModal()
}

// 取消执行
const cancelExecution = async (executionId: string) => {
  try {
    // 确保Agent管理器已初始化
    await ensureAgentManagerInitialized()
    
    await invoke('cancel_workflow_execution', { executionId })
    await refreshExecutions()
  } catch (error) {
    console.error('取消执行失败:', error)
  }
}

// 关闭模态框
const closeModal = () => {
  executionModal.value?.close()
  selectedExecution.value = null
}

const closeFlowchartModal = () => {
  flowchartModal.value?.close()
  workflowPlan.value = null
}

const closeStepDetailsModal = () => {
  stepDetailsModal.value?.close()
  selectedStep.value = null
  // 重置选项卡状态
  resultDataTab.value = 'formatted'
  toolResultTab.value = 'formatted'
}

// 获取状态徽章样式
const getStatusBadgeClass = (status: string) => {
  const statusMap: Record<string, string> = {
    'Pending': 'badge-ghost',
    'Running': 'badge-primary',
    'Completed': 'badge-success',
    'Failed': 'badge-error',
    'Cancelled': 'badge-warning',
    'Paused': 'badge-info'
  }
  return statusMap[status] || 'badge-ghost'
}

// 获取状态文本
const getStatusText = (status: string) => {
  const statusMap: Record<string, string> = {
    'Pending': '等待中',
    'Running': '运行中',
    'Completed': '已完成',
    'Failed': '失败',
    'Cancelled': '已取消',
    'Paused': '已暂停'
  }
  return statusMap[status] || status
}

// 获取进度条样式
const getProgressClass = (status: string) => {
  const progressMap: Record<string, string> = {
    'Pending': 'progress-ghost',
    'Running': 'progress-primary',
    'Completed': 'progress-success',
    'Failed': 'progress-error',
    'Cancelled': 'progress-warning',
    'Paused': 'progress-info'
  }
  return progressMap[status] || 'progress-ghost'
}

// 获取步骤卡片样式
const getStepCardClass = (status: string) => {
  const classMap: Record<string, string> = {
    'Completed': 'border-green-200 bg-green-50',
    'Failed': 'border-red-200 bg-red-50',
    'Running': 'border-blue-200 bg-blue-50',
    'Skipped': 'border-yellow-200 bg-yellow-50',
    'Pending': 'border-gray-200 bg-gray-50'
  }
  return classMap[status] || 'border-gray-200 bg-gray-50'
}

// 获取步骤节点样式
const getStepNodeClass = (status: string) => {
  const classMap: Record<string, string> = {
    'Completed': 'completed-node',
    'Failed': 'failed-node',
    'Running': 'running-node',
    'Skipped': 'skipped-node',
    'Pending': 'pending-node'
  }
  return classMap[status] || 'pending-node'
}

// 显示工作流条件信息
const showWorkflowCondition = () => {
  // 这里可以添加显示条件逻辑的功能
  console.log('显示工作流条件')
}

// 获取步骤指示器样式
const getStepIndicatorClass = (status: string) => {
  const classMap: Record<string, string> = {
    'Completed': 'bg-green-500',
    'Failed': 'bg-red-500',
    'Running': 'bg-blue-500',
    'Skipped': 'bg-yellow-500',
    'Pending': 'bg-gray-400'
  }
  return classMap[status] || 'bg-gray-400'
}

// 获取步骤状态徽章样式
const getStepStatusBadgeClass = (status: string) => {
  const statusMap: Record<string, string> = {
    'Pending': 'badge-ghost',
    'Running': 'badge-primary',
    'Completed': 'badge-success',
    'Failed': 'badge-error',
    'Cancelled': 'badge-warning',
    'Paused': 'badge-info',
    'Skipped': 'badge-warning',
    'Retrying': 'badge-warning'
  }
  return statusMap[status] || 'badge-ghost'
}

// 获取步骤状态文本
const getStepStatusText = (status: string) => {
  const statusMap: Record<string, string> = {
    'Pending': '等待中',
    'Running': '运行中',
    'Completed': '已完成',
    'Failed': '失败',
    'Cancelled': '已取消',
    'Paused': '已暂停',
    'Skipped': '已跳过',
    'Retrying': '重试中'
  }
  return statusMap[status] || status
}

// 格式化时间
const formatTime = (timeStr: string) => {
  return new Date(parseInt(timeStr) * 1000).toLocaleString('zh-CN')
}

// 格式化持续时间
const formatDuration = (ms: number) => {
  if (ms < 1000) {
    return `${ms}ms`
  } else if (ms < 60000) {
    return `${(ms / 1000).toFixed(1)}s`
  } else {
    return `${(ms / 60000).toFixed(1)}min`
  }
}

// 格式化JSON数据为HTML
const formatJsonData = (data: any): string => {
  if (!data) return ''
  
  try {
    let jsonData = data
    
    // 处理复杂的嵌套JSON字符串
    if (typeof data === 'string') {
      try {
        jsonData = JSON.parse(data)
      } catch {
        // 如果字符串不是有效JSON，尝试处理特殊格式
        jsonData = parseComplexJsonString(data)
      }
    }
    
    // 检查是否是工具执行结果格式
    if (jsonData && typeof jsonData === 'object' && jsonData.output) {
      return formatToolOutputData(jsonData)
    }
    
    // 根据数据类型进行格式化
    return `<div class="formatted-json">${formatDataStructure(jsonData, 0)}</div>`
  } catch (error) {
    // 如果解析失败，返回原始字符串的HTML转义版本
    return `<div class="error-content">数据解析错误: ${escapeHtml(String(data))}</div>`
  }
}

// 处理复杂的JSON字符串
const parseComplexJsonString = (str: string): any => {
  try {
    // 尝试处理像日志中看到的复杂转义字符串
    if (str.includes('Annotated {') && str.includes('raw: Text(RawTextContent')) {
      // 提取实际的JSON内容
      const match = str.match(/text: \\"(.+?)\\"/s)
      if (match) {
        // 处理转义字符
        const jsonStr = match[1]
          .replace(/\\\\n/g, '\n')
          .replace(/\\\\"/g, '"')
          .replace(/\\\\/g, '\\')
        
        return JSON.parse(jsonStr)
      }
    }
    
    // 其他格式处理...
    return str
  } catch {
    return str
  }
}

// 格式化工具输出数据
const formatToolOutputData = (data: any): string => {
  if (!data.output) return formatDataStructure(data, 0)
  
  try {
    let outputData = data.output
    
    // 检查是否是字符串格式的JSON
    if (typeof outputData === 'string') {
      // 处理Annotated格式
      if (outputData.includes('Annotated {') && outputData.includes('raw: Text(RawTextContent')) {
        const match = outputData.match(/text: \\"(.+?)\\"/s)
        if (match) {
          const jsonStr = match[1]
            .replace(/\\\\n/g, '\n')
            .replace(/\\\\"/g, '"')
            .replace(/\\\\/g, '\\')
          
          const parsedData = JSON.parse(jsonStr)
          
          // 如果是数组数据，进行特殊格式化
          if (Array.isArray(parsedData)) {
            return formatArrayData(parsedData)
          }
          
          outputData = parsedData
        }
      }
    }
    
    const result = `
      <div class="tool-output-container">
        <div class="tool-output-header">
          <span class="tool-output-label">工具执行结果</span>
          <span class="tool-success-badge">${data.success ? '✓ 成功' : '✗ 失败'}</span>
        </div>
        <div class="tool-output-content">
          ${typeof outputData === 'object' ? formatDataStructure(outputData, 0) : escapeHtml(String(outputData))}
        </div>
      </div>
    `
    
    return result
  } catch (error) {
    return formatDataStructure(data, 0)
  }
}

// 格式化数组数据（特别是视频列表等）
const formatArrayData = (data: any[]): string => {
  if (!Array.isArray(data) || data.length === 0) {
    return '<div class="empty-array">空数组</div>'
  }
  
  // 检查是否是视频列表格式
  if (data[0] && typeof data[0] === 'object' && data[0].title) {
    return formatVideoList(data)
  }
  
  // 其他数组格式
  return `<div class="array-container">
    ${data.map((item, index) => `
      <div class="array-item">
        <div class="array-index">[${index}]</div>
        <div class="array-value">${formatDataStructure(item, 1)}</div>
      </div>
    `).join('')}
  </div>`
}

// 格式化视频列表
const formatVideoList = (videos: any[]): string => {
  return `<div class="video-list-container">
    <div class="video-list-header">
      <h4>搜索结果 (${videos.length} 个视频)</h4>
    </div>
    <div class="video-list">
      ${videos.map((video, index) => `
        <div class="video-item">
          <div class="video-rank">#${index + 1}</div>
          <div class="video-content">
            <div class="video-title">
              <a href="${video.url || '#'}" target="_blank" class="video-link">
                ${escapeHtml(video.title || '无标题')}
              </a>
            </div>
            <div class="video-meta">
              <span class="video-author">UP主: ${escapeHtml(video.author || '未知')}</span>
              <span class="video-plays">播放量: ${formatNumber(video.play_count || 0)}</span>
              <span class="video-duration">时长: ${video.duration || '未知'}</span>
              <span class="video-date">发布: ${video.publish_date || '未知'}</span>
            </div>
            <div class="video-ids">
              <span class="video-bvid">BV号: ${video.bvid || '未知'}</span>
            </div>
          </div>
        </div>
      `).join('')}
    </div>
  </div>`
}

// 格式化数字（播放量等）
const formatNumber = (num: number): string => {
  if (num >= 100000000) {
    return `${(num / 100000000).toFixed(1)}亿`
  } else if (num >= 10000) {
    return `${(num / 10000).toFixed(1)}万`
  } else {
    return num.toString()
  }
}

// 递归格式化数据结构
const formatDataStructure = (data: any, depth: number): string => {
  const indent = '  '.repeat(depth)
  const nextIndent = '  '.repeat(depth + 1)
  
  if (data === null) {
    return '<span class="json-null">null</span>'
  }
  
  if (typeof data === 'boolean') {
    return `<span class="json-boolean">${data}</span>`
  }
  
  if (typeof data === 'number') {
    return `<span class="json-number">${data}</span>`
  }
  
  if (typeof data === 'string') {
    // 检查是否是特殊的字符串格式
    if (data.startsWith('http://') || data.startsWith('https://')) {
      return `<span class="json-string"><a href="${data}" target="_blank" class="json-link">"${escapeHtml(data)}"</a></span>`
    }
    if (data.match(/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}/)) {
      return `<span class="json-string json-date">"${escapeHtml(data)}"</span>`
    }
    if (data.length > 100) {
      return `<details class="json-long-string">
        <summary class="json-string">"${escapeHtml(data.substring(0, 100))}..." (点击展开)</summary>
        <div class="json-string-full">"${escapeHtml(data)}"</div>
      </details>`
    }
    return `<span class="json-string">"${escapeHtml(data)}"</span>`
  }
  
  if (Array.isArray(data)) {
    if (data.length === 0) {
      return '<span class="json-bracket">[]</span>'
    }
    
    const items = data.map((item, index) => {
      return `${nextIndent}<span class="json-array-index">${index}:</span> ${formatDataStructure(item, depth + 1)}`
    }).join(',\n')
    
    return `<span class="json-bracket">[</span>\n${items}\n${indent}<span class="json-bracket">]</span>`
  }
  
  if (typeof data === 'object') {
    const keys = Object.keys(data)
    if (keys.length === 0) {
      return '<span class="json-bracket">{}</span>'
    }
    
    const items = keys.map(key => {
      const value = formatDataStructure(data[key], depth + 1)
      return `${nextIndent}<span class="json-key">"${escapeHtml(key)}"</span><span class="json-colon">:</span> ${value}`
    }).join(',\n')
    
    return `<span class="json-bracket">{</span>\n${items}\n${indent}<span class="json-bracket">}</span>`
  }
  
  return escapeHtml(String(data))
}

// HTML转义函数
const escapeHtml = (text: string): string => {
  const div = document.createElement('div')
  div.textContent = text
  return div.innerHTML
}

// 格式化步骤标题
const formatStepTitle = (title: string): string => {
  if (!title) return '未知步骤'
  
  // 处理常见的步骤名称格式
  if (title.includes('_')) {
    return title.split('_').map(word => 
      word.charAt(0).toUpperCase() + word.slice(1).toLowerCase()
    ).join(' ')
  }
  
  return title
}

// 获取步骤摘要信息
const getStepSummary = (step: WorkflowStepDetail): string => {
  // 优先从结果数据中提取摘要
  if (step.result_data) {
    try {
      const resultSummary = extractDataSummary(step.result_data)
      if (resultSummary) return resultSummary
    } catch (error) {
      // 忽略解析错误
    }
  }
  
  // 从工具结果中提取摘要
  if (step.tool_result) {
    try {
      const toolSummary = extractDataSummary(step.tool_result)
      if (toolSummary) return toolSummary
    } catch (error) {
      // 忽略解析错误
    }
  }
  
  // 如果有错误，显示错误信息的简要版本
  if (step.error) {
    return `错误: ${step.error.substring(0, 30)}...`
  }
  
  return ''
}

// 提取数据摘要
const extractDataSummary = (data: any): string => {
  if (!data) return ''
  
  try {
    let parsedData = data
    
    // 如果是字符串，尝试解析为JSON
    if (typeof data === 'string') {
      try {
        parsedData = JSON.parse(data)
      } catch {
        // 处理复杂格式的字符串数据
        if (data.includes('Annotated {') && data.includes('raw: Text(RawTextContent')) {
          const match = data.match(/text: \\"(.+?)\\"/s)
          if (match) {
            const jsonStr = match[1]
              .replace(/\\\\n/g, '\n')
              .replace(/\\\\"/g, '"')
              .replace(/\\\\/g, '\\')
            parsedData = JSON.parse(jsonStr)
          }
        } else {
          return data.length > 50 ? data.substring(0, 50) + '...' : data
        }
      }
    }
    
    // 检查是否是工具执行结果
    if (parsedData && typeof parsedData === 'object' && parsedData.output) {
      const output = parsedData.output
      
      // 处理视频搜索结果
      if (typeof output === 'string' && output.includes('Annotated {')) {
        const match = output.match(/text: \\"(.+?)\\"/s)
        if (match) {
          const jsonStr = match[1]
            .replace(/\\\\n/g, '\n')
            .replace(/\\\\"/g, '"')
            .replace(/\\\\/g, '\\')
          const videoData = JSON.parse(jsonStr)
          
          if (Array.isArray(videoData) && videoData.length > 0) {
            return `找到 ${videoData.length} 个视频结果`
          }
        }
      }
      
      if (Array.isArray(output)) {
        return `数组数据 (${output.length} 项)`
      } else if (typeof output === 'object') {
        const keys = Object.keys(output)
        return `对象数据 (${keys.length} 个字段)`
      } else {
        return String(output).substring(0, 50)
      }
    }
    
    // 处理数组数据
    if (Array.isArray(parsedData)) {
      if (parsedData.length === 0) return '空数组'
      if (parsedData[0] && typeof parsedData[0] === 'object' && parsedData[0].title) {
        return `${parsedData.length} 个视频结果`
      }
      return `数组数据 (${parsedData.length} 项)`
    }
    
    // 处理对象数据
    if (typeof parsedData === 'object') {
      const keys = Object.keys(parsedData)
      if (keys.length === 0) return '空对象'
      
      // 检查特殊字段
      if (parsedData.title) return `标题: ${parsedData.title}`
      if (parsedData.name) return `名称: ${parsedData.name}`
      if (parsedData.message) return `消息: ${parsedData.message}`
      if (parsedData.status) return `状态: ${parsedData.status}`
      
      return `对象 (${keys.length} 个字段)`
    }
    
    // 其他类型的数据
    const str = String(parsedData)
    return str.length > 50 ? str.substring(0, 50) + '...' : str
    
  } catch (error) {
    return '数据解析错误'
  }
}

// 截断文本
const truncateText = (text: string, maxLength: number): string => {
  if (!text) return ''
  return text.length > maxLength ? text.substring(0, maxLength) + '...' : text
}

// 获取执行结果摘要
const getExecutionResultSummary = (result: any): string => {
  if (!result) return ''
  
  try {
    return extractDataSummary(result)
  } catch (error) {
    return '结果数据解析错误'
  }
}

// 计算运行时间
const calculateRunningTime = (startedAt: string): string => {
  const startTime = new Date(parseInt(startedAt) * 1000)
  const now = new Date()
  const diffMs = now.getTime() - startTime.getTime()
  
  if (diffMs < 60000) {
    return `${Math.floor(diffMs / 1000)}秒`
  } else if (diffMs < 3600000) {
    return `${Math.floor(diffMs / 60000)}分钟`
  } else {
    return `${Math.floor(diffMs / 3600000)}小时`
  }
}

// 获取步骤类型
const getStepType = (step: WorkflowStepDetail): string => {
  if (step.tool_result) {
    return '工具执行步骤'
  } else if (step.result_data) {
    return '数据处理步骤'
  } else if (step.dependencies && step.dependencies.length > 0) {
    return '依赖步骤'
  } else if (step.error) {
    return '失败步骤'
  } else {
    return '普通步骤'
  }
}

// 组件挂载时获取数据
onMounted(() => {
  refreshExecutions()
  refreshStatistics()
})
</script>

<style scoped>
.card-header {
  @apply p-6 pb-0;
}

.workflow-flowchart {
  @apply min-w-full;
}

.step-card {
  @apply min-w-0;
  min-width: 500px;
}

.step-indicator {
  @apply flex-shrink-0;
}

/* 自定义滚动条 */
.overflow-auto::-webkit-scrollbar {
  width: 8px;
  height: 8px;
}

.overflow-auto::-webkit-scrollbar-track {
  @apply bg-gray-100 rounded;
}

.overflow-auto::-webkit-scrollbar-thumb {
  @apply bg-gray-400 rounded;
}

.overflow-auto::-webkit-scrollbar-thumb:hover {
  @apply bg-gray-500;
}

/* 流程图样式 */
.flowchart-node {
  @apply flex flex-col items-center gap-2 relative;
}

.node-circle {
  @apply w-12 h-12 rounded-full flex items-center justify-center font-bold text-sm;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
}

.node-label {
  @apply text-sm font-semibold text-gray-700 text-center;
  max-width: 120px;
}

.node-sublabel {
  @apply text-xs text-gray-500 text-center;
}

.decision-diamond {
  @apply w-32 h-20 flex items-center justify-center;
  transform: rotate(45deg);
  position: relative;
}

.decision-diamond span {
  transform: rotate(-45deg);
  font-size: 12px;
  text-align: center;
  line-height: 1.2;
}

.connector-line {
  @apply w-px h-8 bg-gray-300;
}

.branch-container {
  @apply relative w-64 h-16;
}

.branch-line {
  @apply absolute w-24 h-px bg-gray-300;
  top: 50%;
  transform: translateY(-50%);
}

.left-branch {
  left: 20%;
  transform: translateY(-50%) rotate(-45deg);
  transform-origin: right center;
}

.right-branch {
  right: 20%;
  transform: translateY(-50%) rotate(45deg);
  transform-origin: left center;
}

.branch-label {
  @apply absolute text-xs text-gray-600 font-medium;
  top: -16px;
  left: 50%;
  transform: translateX(-50%);
}

.execution-area {
  @apply w-full max-w-4xl;
}

.steps-container {
  @apply flex justify-between items-start gap-8;
}

.execution-path {
  @apply flex-1 flex flex-col items-center gap-6;
}

.execution-step {
  @apply flex flex-col items-center gap-4;
}

.step-node {
  @apply bg-white border-2 rounded-lg p-4 min-w-[200px] text-center;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
}

.step-connector {
  @apply w-px h-6 bg-gray-300;
}

.status-info {
  @apply mt-2 flex flex-col items-center gap-1;
}

.error-indicator {
  @apply absolute -top-2 -right-2 bg-white rounded-full p-1 border border-red-200;
}

.merge-lines {
  @apply relative w-64 h-16;
}

.merge-line {
  @apply absolute w-24 h-px bg-gray-300;
  top: 50%;
}

.left-merge {
  left: 20%;
  transform: translateY(-50%) rotate(45deg);
  transform-origin: right center;
}

.right-merge {
  right: 20%;
  transform: translateY(-50%) rotate(-45deg);
  transform-origin: left center;
}

/* 节点状态样式 */
.completed-node {
  @apply border-green-300 bg-green-50;
}

.failed-node {
  @apply border-red-300 bg-red-50;
}

.running-node {
  @apply border-blue-300 bg-blue-50;
  animation: pulse 2s infinite;
}

.skipped-node {
  @apply border-yellow-300 bg-yellow-50;
}

.pending-node {
  @apply border-gray-300 bg-gray-50;
}

.alternative-step {
  @apply opacity-60;
}

@keyframes pulse {
  0%, 100% {
    box-shadow: 0 2px 8px rgba(59, 130, 246, 0.2);
  }
  50% {
    box-shadow: 0 2px 16px rgba(59, 130, 246, 0.4);
  }
}

/* 格式化数据显示样式 */
.formatted-data-container {
  @apply border border-gray-200 rounded-lg overflow-hidden;
}

.data-tabs {
  @apply flex bg-gray-50 border-b border-gray-200;
}

.tab-button {
  @apply px-4 py-2 text-sm font-medium text-gray-600 hover:text-gray-900 hover:bg-gray-100 transition-colors;
  border-bottom: 2px solid transparent;
}

.tab-button.active {
  @apply text-blue-600 bg-white;
  border-bottom-color: #3b82f6;
}

.data-content {
  @apply max-h-96 overflow-auto;
}

.formatted-content {
  @apply p-4 font-mono text-sm leading-relaxed;
  background: #f8fafc;
}

.raw-content {
  @apply p-0;
}

.json-code {
  @apply p-4 text-sm leading-relaxed bg-gray-900 text-gray-100 whitespace-pre-wrap overflow-auto;
  margin: 0;
  font-family: 'Monaco', 'Menlo', 'Ubuntu Mono', monospace;
}

/* JSON语法高亮样式 */
.json-key {
  @apply text-blue-600 font-medium;
}

.json-string {
  @apply text-green-600;
}

.json-number {
  @apply text-purple-600 font-medium;
}

.json-boolean {
  @apply text-orange-600 font-medium;
}

.json-null {
  @apply text-gray-500 font-medium;
}

.json-bracket {
  @apply text-gray-700 font-bold;
}

.json-colon {
  @apply text-gray-700 mx-1;
}

.json-array-index {
  @apply text-blue-500 font-medium mr-2;
}

.json-link {
  @apply text-blue-500 hover:text-blue-700 underline;
}

.json-date {
  @apply text-indigo-600;
}

.json-long-string summary {
  @apply cursor-pointer hover:bg-gray-100 p-1 rounded;
}

.json-string-full {
  @apply mt-2 p-2 bg-gray-50 rounded border-l-4 border-green-500;
}

.error-content {
  @apply text-red-600 bg-red-50 p-2 rounded border border-red-200;
}

/* 工具输出格式化样式 */
.tool-output-container {
  @apply border border-gray-200 rounded-lg overflow-hidden mb-4;
}

.tool-output-header {
  @apply bg-gray-50 px-4 py-2 flex justify-between items-center border-b border-gray-200;
}

.tool-output-label {
  @apply font-medium text-gray-700;
}

.tool-success-badge {
  @apply text-sm px-2 py-1 rounded;
  color: #059669;
  background-color: #d1fae5;
}

.tool-output-content {
  @apply p-4;
}

/* 视频列表样式 */
.video-list-container {
  @apply bg-white rounded-lg border border-gray-200;
}

.video-list-header {
  @apply bg-gray-50 px-4 py-3 border-b border-gray-200;
}

.video-list-header h4 {
  @apply text-lg font-semibold text-gray-800 m-0;
}

.video-list {
  @apply divide-y divide-gray-100;
}

.video-item {
  @apply p-4 flex items-start gap-3 hover:bg-gray-50 transition-colors;
}

.video-rank {
  @apply flex-shrink-0 w-8 h-8 bg-blue-500 text-white rounded-full flex items-center justify-center text-sm font-bold;
}

.video-content {
  @apply flex-1 min-w-0;
}

.video-title {
  @apply mb-2;
}

.video-link {
  @apply text-blue-600 hover:text-blue-800 font-medium text-base leading-tight block;
  text-decoration: none;
}

.video-link:hover {
  text-decoration: underline;
}

.video-meta {
  @apply flex flex-wrap gap-4 text-sm text-gray-600 mb-2;
}

.video-meta span {
  @apply inline-flex items-center;
}

.video-author {
  @apply font-medium text-gray-700;
}

.video-plays {
  @apply text-red-600 font-medium;
}

.video-duration {
  @apply text-green-600;
}

.video-date {
  @apply text-blue-600;
}

.video-ids {
  @apply text-xs text-gray-500;
}

.video-bvid {
  @apply font-mono bg-gray-100 px-2 py-1 rounded;
}

/* 数组容器样式 */
.array-container {
  @apply space-y-2;
}

.array-item {
  @apply flex items-start gap-3 p-2 bg-gray-50 rounded border border-gray-200;
}

.array-index {
  @apply text-sm font-mono text-blue-600 font-bold flex-shrink-0;
}

.array-value {
  @apply flex-1 min-w-0;
}

.empty-array {
  @apply text-center py-4 text-gray-500 italic;
}

/* 格式化JSON容器 */
.formatted-json {
  @apply font-mono text-sm leading-relaxed;
}

/* 步骤摘要样式 */
.step-summary {
  @apply mt-2 mb-1 px-2;
}

.summary-text {
  @apply text-xs text-gray-600 leading-relaxed break-words;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
  min-height: 24px;
  max-width: 180px;
}

/* 数据指示器样式 */
.data-indicators {
  @apply flex flex-wrap justify-center gap-1 mt-2;
}

.indicator-item {
  @apply flex items-center gap-1 px-2 py-1 bg-gray-100 rounded-full text-xs;
  transition: background-color 0.2s ease;
}

.indicator-item:hover {
  @apply bg-gray-200;
}

.indicator-item svg {
  @apply flex-shrink-0;
}

/* 改进的错误指示器 */
.error-indicator {
  @apply absolute -top-2 -right-2 bg-white rounded-full p-1 border border-red-200 shadow-sm;
  animation: pulse-error 2s infinite;
}

@keyframes pulse-error {
  0%, 100% {
    box-shadow: 0 0 0 0 rgba(239, 68, 68, 0.7);
  }
  70% {
    box-shadow: 0 0 0 4px rgba(239, 68, 68, 0);
  }
}

/* 步骤概览卡片样式 */
.step-overview-card {
  @apply bg-gradient-to-r from-blue-50 to-indigo-50 border border-blue-200 rounded-lg p-6;
}

.step-status-icon {
  @apply w-16 h-16 rounded-full flex items-center justify-center font-bold text-lg flex-shrink-0;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.1);
}

/* 工作流信息卡片样式 */
.workflow-info-card {
  @apply bg-gradient-to-br from-white via-blue-50 to-indigo-100 border border-indigo-200 rounded-xl p-6;
  box-shadow: 0 8px 25px rgba(59, 130, 246, 0.1);
}

.workflow-progress-circle {
  @apply flex-shrink-0;
}

.progress-ring {
  @apply relative;
}

.progress-ring-svg {
  @apply transform -rotate-90;
}

.progress-ring-circle {
  transition: stroke-dashoffset 0.5s ease-in-out;
}

.progress-text {
  @apply absolute inset-0 flex items-center justify-center text-sm font-bold text-blue-600;
}

/* 统计项样式 */
.stats-item {
  @apply flex items-center gap-3 p-3 bg-white/80 backdrop-blur-sm rounded-lg border border-gray-200;
  transition: transform 0.2s ease, box-shadow 0.2s ease;
}

.stats-item:hover {
  @apply transform scale-105;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.1);
}

.stats-icon {
  @apply w-10 h-10 rounded-full flex items-center justify-center flex-shrink-0;
}

.stats-content {
  @apply flex-1;
}

.stats-number {
  @apply text-xl font-bold leading-none;
}

.stats-label {
  @apply text-xs text-gray-600 font-medium mt-1;
}
</style>