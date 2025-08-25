<template>
  <div class="enhanced-ai-chat w-full h-full flex flex-col bg-gradient-to-br from-base-100 to-base-200 overflow-hidden">
    <!-- 消息区域 -->
    <div ref="messagesContainer" class="flex-1 overflow-y-auto p-4 space-y-4 min-h-0 max-w-full">
      <!-- 欢迎消息 -->
      <div v-if="messages.length === 0" class="flex justify-center items-center h-full">
        <div class="text-center">
          <div class="avatar placeholder mb-4">
            <div class="bg-primary text-primary-content rounded-full w-16">
              <i class="fas fa-brain text-2xl"></i>
            </div>
          </div>
          <h3 class="text-lg font-semibold mb-2">{{ t('aiAssistant.welcome.title', 'AI智能助手') }}</h3>
          <p class="text-base-content/70 max-w-md">
            {{ t('aiAssistant.welcome.description', '我是您的AI安全助手，可以帮您执行安全扫描、漏洞分析等任务。请告诉我您需要什么帮助？') }}
          </p>
          <div class="flex flex-wrap gap-2 mt-4 justify-center">
            <button v-for="suggestion in quickSuggestions" :key="suggestion.id" 
                    class="btn btn-sm btn-outline" @click="sendQuickMessage(suggestion.message)">
              <i :class="suggestion.icon"></i>
              {{ suggestion.text }}
            </button>
          </div>
        </div>
      </div>

      <!-- 消息列表 -->
      <div v-for="message in messages" :key="message.id" :class="message.role === 'user' ? 'chat-end ' : 'chat-start'">
        <div class="chat-image avatar">
          <div class="w-12 rounded-full shadow-lg" :class="message.role === 'user' ? 'bg-gradient-to-br from-primary to-primary-focus' : 'bg-gradient-to-br from-secondary to-secondary-focus '">
            <div class="w-full h-full flex items-center justify-center">
              <i :class="message.role === 'user' ? 'fas fa-user text-white text-sm' : 'fas fa-robot text-white text-sm'"></i>
            </div>
          </div>
        </div>
        <div class="chat-header mb-1">
          <span class="font-semibold text-base-content">{{ message.role === 'user' ? t('common.you', '您') : t('common.assistant', 'AI助手') }}</span>
          <time class="text-xs opacity-60 ml-3 bg-base-300 px-2 py-1 rounded-full">{{ formatTime(message.timestamp) }}</time>
        </div>
        <div class="chat-bubble shadow-lg border-0 " :class="message.role === 'user' ? 'bg-gradient-to-br  to-primary-focus text-primary-content' : 'bg-gradient-to-br from-base-100 to-base-200 text-base-content border border-base-300 w-max-600'">
          <!-- 主要消息内容 -->
          <div v-if="message.content" 
               class="prose prose-sm max-w-none leading-relaxed" 
               :class="[
                 message.role === 'user' ? 'prose-invert' : '',
                 message.isStreaming && message.role === 'assistant' ? 'streaming-content' : ''
               ]" 
               v-html="renderMarkdown(message.content)"></div>
          
          <!-- 流式响应打字机效果指示器 -->
          <div v-else-if="message.isStreaming && message.role === 'assistant'" class="flex items-center gap-2 text-base-content/70">
            <span class="loading loading-dots loading-sm"></span>
            <div class="flex flex-col gap-1">
              <span class="text-sm">{{ t('aiAssistant.generating', 'AI正在思考...') }}</span>
              <div class="flex items-center gap-4 text-xs opacity-60">
                <span v-if="streamCharCount > 0">
                  {{ streamCharCount }} 字符
                </span>
                <span v-if="getStreamSpeed() > 0">
                  {{ getStreamSpeed() }} 字符/秒
                </span>
              </div>
            </div>
          </div>
          
          <!-- 执行计划显示 -->
          <div v-if="message.executionPlan" class="mt-4">
            <div class="bg-base-300/50 rounded-xl border border-base-300">
              <div class="p-4 border-b border-base-300">
                <div class="flex items-center gap-2">
                  <i class="fas fa-list-check text-accent"></i>
                  <span class="font-semibold">{{ t('aiAssistant.executionPlan', '执行计划') }}</span>
                  <div class="badge badge-accent badge-outline badge-sm ml-auto">
                    {{ message.executionPlan.steps?.length || 0 }} 步骤
                  </div>
                </div>
                <!-- 整体进度条 -->
                <div v-if="message.isStreaming || message.executionProgress !== undefined" class="flex items-center gap-2 mt-3">
                  <div class="text-xs text-base-content/70">进度:</div>
                  <div class="flex-1 h-2 bg-base-200 rounded-full overflow-hidden">
                    <div class="h-full bg-primary transition-all duration-500 ease-out" 
                         :style="{ width: `${message.executionProgress || 0}%` }"
                         :class="{ 'animate-pulse': message.isStreaming && (message.executionProgress || 0) < 100 }"></div>
                  </div>
                  <div class="text-xs text-base-content/70 min-w-max">{{ Math.round(message.executionProgress || 0) }}%</div>
                </div>
                <!-- 当前执行步骤提示 -->
                <div v-if="message.isStreaming && message.currentStep" class="flex items-center gap-2 mt-2 text-xs text-base-content/60">
                  <i class="fas fa-cog fa-spin text-primary"></i>
                  <span>正在执行: {{ message.currentStep }}</span>
                </div>
              </div>
              
              <!-- 步骤列表 - 每个步骤都是独立的折叠面板 -->
              <div class="space-y-1 p-2">
                <div v-for="(step, index) in message.executionPlan.steps" :key="step.id" 
                     class="collapse collapse-arrow bg-base-100 border border-base-200 rounded-lg"
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
                          <i v-if="step.status === 'executing' || step.status === 'running'" 
                             class="fas fa-spinner fa-spin text-primary text-xs"></i>
                          <i v-else-if="step.status === 'completed'" 
                             class="fas fa-check-circle text-success text-xs"></i>
                          <i v-else-if="step.status === 'failed'" 
                             class="fas fa-times-circle text-error text-xs"></i>
                        </div>
                        <!-- 实时进度条（仅执行中的步骤显示） -->
                        <div v-if="step.status === 'executing' || step.status === 'running'" class="mt-2">
                          <div class="flex items-center gap-2 text-xs">
                            <span class="text-base-content/60">执行中...</span>
                            <div class="flex-1 h-1 bg-base-200 rounded-full">
                              <div class="h-full bg-primary rounded-full transition-all duration-500" 
                                   :style="{ width: '100%' }"
                                   style="animation: progress-pulse 1.5s ease-in-out infinite;"></div>
                            </div>
                          </div>
                        </div>
                        <div v-if="step.description && step.name !== step.description" 
                             class="text-xs text-base-content/60 mt-1 truncate">{{ step.description }}</div>
                      </div>
                      <!-- <div class="badge badge-xs" :class="getStepStatusClass(step.status)"> -->
                        <!-- {{ getStepStatusText(step.status) }} -->
                      <!-- </div> -->
                    </div>
                  </div>
                  <div class="collapse-content px-4 pb-4">
                    <div class="space-y-3">
                      <!-- 基本信息 -->
                      <div class="grid grid-cols-1 md:grid-cols-2 gap-3 text-xs">
                        <div v-if="step.started_at" class="flex items-center gap-2">
                          <i class="fas fa-clock text-base-content/60"></i>
                          <span class="text-base-content/70">开始:</span>
                          <span>{{ formatTimestamp(step.started_at) }}</span>
                        </div>
                        <div v-if="step.completed_at" class="flex items-center gap-2">
                          <i class="fas fa-check-circle text-success"></i>
                          <span class="text-base-content/70">完成:</span>
                          <span>{{ formatTimestamp(step.completed_at) }}</span>
                        </div>
                        <div v-if="step.started_at && step.completed_at" class="flex items-center gap-2 md:col-span-2">
                          <i class="fas fa-stopwatch text-info"></i>
                          <span class="text-base-content/70">耗时:</span>
                          <span class="text-success font-medium">{{ Math.round((step.completed_at - step.started_at) / 1000) }}秒</span>
                        </div>
                      </div>
                      
                      <!-- 步骤描述 -->
                      <div v-if="step.description && step.name !== step.description" class="bg-base-200/50 rounded-lg p-3">
                        <div class="text-xs font-medium text-base-content/70 mb-2 flex items-center gap-1">
                          <i class="fas fa-info-circle"></i>
                          描述
                        </div>
                        <div class="text-sm text-base-content">{{ step.description }}</div>
                      </div>
                      
                      <!-- 执行结果 -->
                      <div v-if="getStepResultData(step)" class="bg-success/10 border border-success/20 rounded-lg p-3">
                        <div class="text-xs font-medium text-success mb-2 flex items-center gap-1">
                          <i class="fas fa-check-circle"></i>
                          执行结果
                        </div>
                        <div class="text-sm text-base-content">
                          <div v-if="getStepReasoningResult(step)" class="space-y-2">
                            <div class="text-xs text-base-content/70">推理结果:</div>
                            <div class="prose prose-sm max-w-none bg-base-100 p-3 rounded border" 
                                 v-html="renderMarkdown(getStepReasoningResult(step))"></div>
                          </div>
                          <div v-else-if="typeof getStepResultData(step) === 'string'" class="bg-base-100 p-3 rounded border">
                            <div class="prose prose-sm max-w-none" 
                                 v-html="renderMarkdown(getStepResultData(step))"></div>
                          </div>
                          <div v-else class="bg-base-100 p-3 rounded border">
                            <pre class="text-xs overflow-x-auto max-h-60">{{ JSON.stringify(getStepResultData(step), null, 2) }}</pre>
                          </div>
                        </div>
                      </div>
                      
                      <!-- 错误信息 -->
                      <div v-if="step.error" class="bg-error/10 border border-error/20 rounded-lg p-3">
                        <div class="text-xs font-medium text-error mb-2 flex items-center gap-1">
                          <i class="fas fa-exclamation-triangle"></i>
                          错误信息
                        </div>
                        <div class="text-sm text-error whitespace-pre-wrap">{{ step.error }}</div>
                      </div>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </div>

          <!-- 工具执行状态 -->
          <div v-if="message.toolExecutions && message.toolExecutions.length > 0" class="mt-4">
            <div class="collapse collapse-arrow bg-base-300/50 rounded-xl border border-base-300">
              <input type="checkbox" class="collapse-checkbox" />
              <div class="collapse-title font-semibold flex items-center gap-2 py-3">
                <i class="fas fa-tools text-info"></i>
                <span>{{ t('aiAssistant.toolExecution', '工具执行') }}</span>
                <div class="badge badge-info badge-outline badge-sm ml-auto">{{ message.toolExecutions.length }} 工具</div>
              </div>
              <div class="collapse-content">
                <div class="space-y-3 pt-2">
                  <div v-for="tool in message.toolExecutions" :key="tool.id" 
                       class="collapse collapse-arrow bg-base-100 rounded-lg shadow-sm">
                    <input type="checkbox" class="collapse-checkbox" />
                    <div class="collapse-title text-sm font-medium flex items-center gap-2 py-3">
                      <i class="fas fa-cog" :class="{
                        'text-warning animate-spin': tool.status === 'running',
                        'text-success': tool.status === 'completed',
                        'text-error': tool.status === 'failed'
                      }"></i>
                      <span class="flex-1">{{ tool.name }}</span>
                      <div class="badge badge-xs" :class="getToolStatusClass(tool.status)">
                        {{ tool.status }}
                      </div>
                    </div>
                    <div class="collapse-content">
                      <div v-if="tool.result" class="bg-base-200 rounded-lg p-3 text-xs font-mono">
                        <pre class="whitespace-pre-wrap"><code>{{ formatToolResult(tool.result) }}</code></pre>
                      </div>
                      <div v-if="tool.error" class="alert alert-error mt-3 text-sm">
                        <i class="fas fa-exclamation-triangle"></i>
                        <span>{{ tool.error }}</span>
                      </div>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </div>

          <!-- 执行结果 -->
          <div v-if="message.executionResult" class="mt-4">
            <div class="collapse collapse-open bg-gradient-to-r from-success/10 to-info/10 border border-success/30 rounded-xl">
              <div class="collapse-title font-semibold flex items-center gap-2 text-success py-3">
                <i class="fas fa-chart-bar"></i>
                <span>{{ t('aiAssistant.executionResult', '执行结果') }}</span>
                <div class="badge badge-success badge-outline badge-sm ml-auto">
                  {{ getResultStatusText(message.executionResult.status) }}
                </div>
              </div>
              <div class="collapse-content">
                <div class="space-y-4 pt-2">
                  <!-- 基础统计信息 -->
                  <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4 text-sm">
                    <div class="bg-base-100 rounded-lg p-3 shadow-sm">
                      <span class="font-medium text-base-content/70">{{ t('aiAssistant.status', '状态') }}</span>
                      <div class="mt-1">
                        <div class="badge badge-lg" :class="getResultStatusClass(message.executionResult.status)">
                          {{ getResultStatusText(message.executionResult.status) }}
                        </div>
                      </div>
                    </div>
                    <div class="bg-base-100 rounded-lg p-3 shadow-sm">
                      <span class="font-medium text-base-content/70">{{ t('aiAssistant.duration', '耗时') }}</span>
                      <div class="mt-1 text-lg font-semibold text-base-content">
                        {{ formatDuration(message.executionResult.duration || 0) }}
                      </div>
                    </div>
                    <div class="bg-base-100 rounded-lg p-3 shadow-sm">
                      <span class="font-medium text-base-content/70">{{ t('aiAssistant.tasksCompleted', '完成任务') }}</span>
                      <div class="mt-1 text-lg font-semibold text-base-content">
                        {{ message.executionResult.tasksCompleted || message.completedSteps || 0 }}/{{ message.executionResult.totalTasks || message.totalSteps || 0 }}
                      </div>
                    </div>
                    <div class="bg-base-100 rounded-lg p-3 shadow-sm">
                      <span class="font-medium text-base-content/70">{{ t('aiAssistant.architecture', '架构') }}</span>
                      <div class="mt-1 text-lg font-semibold text-primary">{{ message.executionResult.architecture || 'Plan-Execute' }}</div>
                    </div>
                  </div>
                  
                  <!-- 详细结果内容 -->
                  <div v-if="getExecutionDetailedResult(message)" class="bg-base-100 rounded-lg p-4 shadow-sm">
                    <h5 class="font-medium text-base-content mb-3 flex items-center gap-2">
                      <i class="fas fa-clipboard-list text-info"></i>
                      详细结果
                    </h5>
                    <div class="text-sm text-base-content/80">
                      <div v-if="typeof getExecutionDetailedResult(message) === 'string'" 
                           class="prose prose-sm max-w-none"
                           v-html="renderMarkdown(getExecutionDetailedResult(message))">
                      </div>
                      <pre v-else class="bg-base-200 p-3 rounded text-xs overflow-x-auto max-h-40 border">{{ JSON.stringify(getExecutionDetailedResult(message), null, 2) }}</pre>
                    </div>
                  </div>
                  
                  <!-- 执行摘要 -->
                  <div v-if="message.executionResult.summary" class="bg-base-100 rounded-lg p-4 shadow-sm">
                    <h5 class="font-medium text-base-content mb-3 flex items-center gap-2">
                      <i class="fas fa-file-alt text-info"></i>
                      执行摘要
                    </h5>
                    <div class="text-sm text-base-content/80">{{ message.executionResult.summary }}</div>
                  </div>
                  
                  <!-- 错误信息 -->
                  <div v-if="message.executionResult.error" class="bg-error/10 border border-error/20 rounded-lg p-4">
                    <h5 class="font-medium text-error mb-3 flex items-center gap-2">
                      <i class="fas fa-exclamation-triangle"></i>
                      错误信息
                    </h5>
                    <div class="text-sm text-error">{{ message.executionResult.error }}</div>
                  </div>
                </div>
              </div>
            </div>
          </div>
          
          <!-- 重试按钮（仅在出错时显示） -->
          <div v-if="message.hasError && message.role === 'assistant'" class="mt-3 flex gap-2">
            <button @click="retryLastMessage" class="btn btn-sm btn-outline btn-primary">
              <i class="fas fa-redo"></i>
              重新发送
            </button>
            <button @click="clearErrorMessage(message)" class="btn btn-sm btn-outline btn-ghost">
              <i class="fas fa-times"></i>
              清除错误
            </button>
          </div>
        </div>
        <!-- 执行状态指示器 -->
      </div>
    </div>

    <!-- 输入区域 -->
    <div class="border-t-2 border-base-300/50 bg-gradient-to-t from-base-200 to-base-100 shadow-lg flex-shrink-0">
      <!-- 配置设置工具栏 -->
      <div class="bg-base-100 border border-base-300 rounded-lg p-3 mb-3 mx-4 mt-3 shadow-sm">
        <!-- 统一工具栏 -->
        <div class="flex items-center justify-between flex-wrap gap-3">
          <!-- 左侧：会话管理 -->
          <div class="flex items-center gap-3">
            <div class="flex items-center gap-2">
              <i class="fas fa-comments text-primary text-sm"></i>
              <span class="text-sm font-medium">{{ t('aiAssistant.sessionManagement', '会话管理') }}</span>
            </div>
            <div class="flex items-center gap-1">
              <button @click="createNewConversation" class="btn btn-primary btn-xs" :disabled="isLoadingConversations" title="新建会话">
                <i class="fas fa-plus"></i>
              </button>
              <button @click="showConversationsList = true" class="btn btn-outline btn-xs" :disabled="conversations.length === 0" title="查看所有会话">
                <i class="fas fa-list"></i>
              </button>
              <button v-if="currentConversationId" @click="clearCurrentConversation" class="btn btn-warning btn-xs" title="清空当前会话">
                <i class="fas fa-broom"></i>
              </button>
              <button v-if="currentConversationId" @click="deleteCurrentConversation" class="btn btn-error btn-xs" title="删除当前会话">
                <i class="fas fa-trash"></i>
              </button>
            </div>
          </div>

          <!-- 右侧：配置设置 -->
          <div class="flex items-center gap-3 flex-wrap">
            <!-- 架构选择器 -->
            <div class="flex items-center gap-1">
              <span class="text-xs text-base-content/70">{{ t('aiAssistant.architecture', '架构') }}:</span>
              <div class="dropdown dropdown-end">
                <div tabindex="0" role="button" class="btn btn-xs btn-outline gap-1">
                  <i class="fas fa-layer-group text-xs"></i>
                  {{ selectedArchitecture }}
                  <i class="fas fa-chevron-down text-xs"></i>
                </div>
                <ul tabindex="0" class="dropdown-content z-[1000] menu p-2 shadow bg-base-100 rounded-box w-72 max-h-96 overflow-y-auto">
                  <li v-for="arch in getAvailableArchitectures()" :key="arch.id">
                    <a @click="selectArchitecture(arch)" 
                       class="hover:bg-primary hover:text-primary-content py-3 px-3"
                       :class="{ 'active bg-primary text-primary-content': selectedArchitecture === arch.name }">
                      <div class="flex flex-col items-start w-full">
                        <div class="flex items-center justify-between w-full mb-1">
                          <span class="font-medium">{{ arch.name }}</span>
                          <div class="badge badge-xs" :class="getArchBadgeClass(arch.status)">
                            {{ getArchBadgeText(arch.status) }}
                          </div>
                        </div>
                        <span class="text-xs opacity-70 text-left" v-if="arch.description">
                          {{ arch.description }}
                        </span>
                      </div>
                    </a>
                  </li>
                  <li v-if="getAvailableArchitectures().length === 0" class="opacity-60">
                    <span class="text-sm px-3 py-2">暂无可用架构</span>
                  </li>
                </ul>
              </div>
            </div>
            
            <!-- 可搜索的模型选择器 -->
            <div class="flex items-center gap-1">
              <span class="text-xs text-base-content/70">{{ t('aiAssistant.model', '模型') }}:</span>
              <div class="dropdown dropdown-top" :class="{ 'dropdown-open': showModelDropdown }">
                <div class="relative">
                  <input 
                    v-model="displayModelText" 
                    @focus="onModelInputFocus"
                    @blur="onModelInputBlur"
                    @input="onModelInput"
                    class="input input-bordered input-xs model-selector-input pr-6"
                    :class="{ 'model-selected': selectedModel && !showModelDropdown }"
                    :style="{ width: getModelInputWidth() }"
                    :placeholder="selectedModel ? '' : '搜索模型...'"
                    :readonly="selectedModel && !showModelDropdown"
                  />
                  <i class="fas fa-chevron-down absolute right-2 top-1/2 transform -translate-y-1/2 text-xs opacity-60"></i>
                </div>
                <ul class="dropdown-content z-[1000] menu p-2 shadow bg-base-100 rounded-box w-80 max-h-80 overflow-y-auto">
                  <template v-for="(models, provider) in filteredModels" :key="provider">
                    <li class="menu-title" v-if="models.length > 0">
                      <span class="text-xs font-semibold opacity-60 uppercase">{{ provider }}</span>
                    </li>
                    <li v-for="model in models" :key="model.id" @click="selectModel(model)">
                      <a class="text-sm hover:bg-primary hover:text-primary-content py-2 px-3" 
                         :class="{ 'active bg-primary text-primary-content': selectedModel === model.id }">
                        <div class="flex flex-col items-start w-full">
                          <span class="font-medium">{{ model.name }}</span>
                          <span class="text-xs opacity-70">{{ model.provider }}</span>
                        </div>
                      </a>
                    </li>
                  </template>
                  <li v-if="Object.keys(filteredModels).length === 0" class="opacity-60">
                    <span class="text-sm px-3 py-2">未找到匹配的模型</span>
                  </li>
                  <li v-if="Object.keys(groupedModels).length === 0" class="opacity-60">
                    <span class="text-sm px-3 py-2">正在加载模型...</span>
                  </li>
                </ul>
              </div>
            </div>
          </div>
        </div>
        
        <!-- 当前会话信息 -->
        <div v-if="currentConversationId" class="flex items-center gap-2 mt-3 pt-3 border-t border-base-300 text-xs text-base-content/70">
          <span>{{ t('aiAssistant.currentSession', '当前会话') }}:</span>
          <div class="badge badge-outline badge-sm">{{ getCurrentConversationTitle() }}</div>
          <span class="text-xs opacity-60">{{ messages.length }} 条消息</span>
        </div>
      </div>

    <!-- 会话列表抽屉 -->
    <div class="fixed inset-0 z-50 mt-16" v-if="showConversationsList">
      <div class="absolute inset-0 bg-black bg-opacity-50" @click="showConversationsList = false"></div>
      <div class="absolute right-0 top-0 h-[calc(100vh-4rem)] w-80 bg-base-200 shadow-xl transform transition-transform duration-300 ease-in-out">
        <div class="h-full p-4 overflow-y-auto">
          <div class="flex items-center justify-between mb-4">
            <h3 class="font-bold text-lg flex items-center gap-2">
              <i class="fas fa-comments text-primary"></i>
              {{ t('aiAssistant.conversationHistory', '会话历史') }}
            </h3>
            <button @click="showConversationsList = false" class="btn btn-ghost btn-sm btn-circle">
              <i class="fas fa-times"></i>
            </button>
          </div>
          
          <!-- 操作按钮 -->
          <div class="flex gap-2 mb-4">
            <button @click="loadConversations" class="btn btn-outline btn-sm flex-1" :disabled="isLoadingConversations">
              <i class="fas fa-sync" :class="{ 'animate-spin': isLoadingConversations }"></i>
              刷新
            </button>
            <button @click="createNewConversation" class="btn btn-primary btn-sm" :disabled="isLoadingConversations">
              <i class="fas fa-plus"></i>
              新建
            </button>
          </div>
          
          <!-- 会话列表 -->
          <div v-if="conversations.length === 0" class="text-center text-base-content/60 py-8">
            <i class="fas fa-comments text-4xl opacity-30 mb-4"></i>
            <p>{{ t('aiAssistant.noConversations', '暂无会话记录') }}</p>
          </div>
          <div v-else class="space-y-3 max-h-[calc(100vh-200px)] overflow-y-auto">
            <div 
              v-for="conv in conversations" 
              :key="conv.id"
              class="card bg-base-100 shadow-sm hover:shadow-md transition-all duration-200 cursor-pointer"
              :class="{ 'ring-2 ring-primary': conv.id === currentConversationId }"
              @click="switchToConversation(conv.id)"
            >
              <div class="card-body p-3">
                <div class="flex items-start justify-between">
                  <div class="flex-1 min-w-0">
                    <h4 class="font-medium text-sm truncate mb-1">
                      {{ conv.title || t('aiAssistant.untitledConversation', '无标题会话') }}
                    </h4>
                    <div class="text-xs text-base-content/60 space-y-1">
                      <div class="flex items-center gap-2">
                        <i class="fas fa-clock"></i>
                        {{ new Date(conv.created_at).toLocaleString() }}
                      </div>
                      <div class="flex items-center gap-2">
                        <i class="fas fa-comment-dots"></i>
                        {{ conv.total_messages }} 条消息
                      </div>
                    </div>
                  </div>
                  <div class="flex flex-col gap-1">
                    <button 
                      v-if="conv.id === currentConversationId" 
                      class="badge badge-primary badge-xs"
                    >
                      当前
                    </button>
                    <button 
                      @click.stop="deleteConversation(conv.id)" 
                      class="btn btn-ghost btn-xs text-error hover:bg-error hover:text-error-content"
                      title="删除会话"
                    >
                      <i class="fas fa-trash"></i>
                    </button>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>

      <!-- 输入框 -->
      <div class="flex gap-3 mx-4 pb-16">
        <div class="flex-1 relative">
          <!-- 输入框增加回车事件，回车发送消息 -->
          <textarea 
            v-model="inputMessage"
            @keydown.enter.ctrl="sendMessage"
            :disabled="isLoading"
            :placeholder="t('aiAssistant.inputPlaceholder', '描述您需要执行的安全任务... (Ctrl+Enter发送)')"
            class="textarea textarea-bordered w-full resize-none shadow-lg border-2 border-base-300 focus:border-primary focus:ring-2 focus:ring-primary/20 transition-all duration-200"
            rows="2"
          ></textarea>

        </div>
        <div class="flex flex-col gap-2">
          <button 
            v-if="!isLoading" 
            @click="sendMessage" 
            :disabled="!inputMessage.trim()" 
            class="btn btn-primary btn-lg shadow-lg hover:shadow-xl transition-shadow duration-200"
            :class="{ 'btn-disabled': !inputMessage.trim() }"
          >
            <i class="fas fa-paper-plane text-lg"></i>
          </button>
          <button 
            v-else 
            @click="stopExecution" 
            class="btn btn-error btn-lg shadow-lg hover:shadow-xl transition-shadow duration-200"
          >
            <i class="fas fa-stop text-lg"></i>
          </button>

        </div>
      </div>
    </div>

    <!-- 步骤详情对话框 -->
    <dialog :class="['modal', { 'modal-open': stepDetailVisible }]">
      <div class="modal-box max-w-4xl max-h-[90vh] overflow-y-auto">
        <div class="flex justify-between items-center mb-4">
          <h3 class="font-bold text-lg">步骤详情</h3>
          <button class="btn btn-sm btn-circle btn-ghost" @click="closeStepDetail">✕</button>
        </div>
        
        <div v-if="selectedStepDetail" class="space-y-4">
          <!-- 基本信息 -->
          <div class="card bg-base-200">
            <div class="card-body">
              <h4 class="card-title text-base mb-3">基本信息</h4>
              <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
                <div>
                  <span class="font-semibold">步骤名称:</span>
                  <span class="ml-2">{{ selectedStepDetail.name || selectedStepDetail.description }}</span>
                </div>
                <div>
                  <span class="font-semibold">状态:</span>
                  <span class="ml-2 badge badge-xs" :class="getStepStatusClass(selectedStepDetail.status)">
                    {{ getStepStatusText(selectedStepDetail.status) }}
                  </span>
                </div>
                <div>
                  <span class="font-semibold">开始时间:</span>
                  <span class="ml-2">{{ formatTimestamp(selectedStepDetail.started_at) }}</span>
                </div>
                <div>
                  <span class="font-semibold">完成时间:</span>
                  <span class="ml-2">{{ formatTimestamp(selectedStepDetail.completed_at) }}</span>
                </div>
                <div v-if="selectedStepDetail.started_at && selectedStepDetail.completed_at" class="col-span-2">
                  <span class="font-semibold">执行耗时:</span>
                  <span class="ml-2">{{ Math.round((selectedStepDetail.completed_at - selectedStepDetail.started_at) / 1000) }}秒</span>
                </div>
              </div>
            </div>
          </div>
          
          <!-- 描述信息 -->
          <div v-if="selectedStepDetail.description" class="card bg-base-200">
            <div class="card-body">
              <h4 class="card-title text-base mb-3">步骤描述</h4>
              <p class="text-sm">{{ selectedStepDetail.description }}</p>
            </div>
          </div>
          
          <!-- 执行结果 -->
          <div v-if="selectedStepDetail.result" class="card bg-base-200">
            <div class="card-body">
              <h4 class="card-title text-base mb-3">执行结果</h4>
              <pre class="bg-base-100 p-4 rounded text-sm overflow-x-auto max-h-60">{{ JSON.stringify(selectedStepDetail.result, null, 2) }}</pre>
            </div>
          </div>
          
          <!-- 错误信息 -->
          <div v-if="selectedStepDetail.error" class="card bg-error/10">
            <div class="card-body">
              <h4 class="card-title text-base mb-3 text-error">错误信息</h4>
              <pre class="bg-base-100 p-4 rounded text-sm overflow-x-auto text-error">{{ selectedStepDetail.error }}</pre>
            </div>
          </div>
        </div>
      </div>
      <form method="dialog" class="modal-backdrop">
        <button @click="closeStepDetail">close</button>
      </form>
    </dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, nextTick, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { marked } from 'marked'

// 定义消息类型
interface ChatMessage {
  id: string
  role: 'user' | 'assistant'
  content: string
  timestamp: Date
  isStreaming?: boolean
  hasError?: boolean
  executionPlan?: any
  toolExecutions?: any[]
  executionResult?: any
  executionProgress?: number
  currentStep?: string
  totalSteps?: number
  completedSteps?: number
}

interface ModelInfo {
  id: string
  name: string
  provider: string
}

interface DispatchResult {
  execution_id: string
  initial_response?: string
  execution_plan?: {
    name?: string
    steps?: any[]
  }
}

interface GroupedModels {
  [provider: string]: ModelInfo[]
}

// Props
const props = defineProps<{
  selectedArchitecture: string
  selectedAgent?: any
  availableArchitectures?: any[]
}>()

// Emits
const emit = defineEmits(['execution-started', 'execution-progress', 'execution-completed', 'architecture-changed'])



const { t } = useI18n()
marked.setOptions({ breaks: true, gfm: true })

// 状态
const messages = ref<ChatMessage[]>([])
const inputMessage = ref('')
const isLoading = ref(false)
const messagesContainer = ref<HTMLElement | null>(null)
const selectedModel = ref('')
const groupedModels = ref<GroupedModels>({})
const currentExecutionId = ref<string | null>(null)
const streamStartTime = ref<number | null>(null)
const streamCharCount = ref(0)

// 会话管理相关状态
const conversations = ref<any[]>([])
const currentConversationId = ref<string | null>(null)
const isLoadingConversations = ref(false)
const showConversationsList = ref(false)

// 模型搜索相关状态
const modelSearchText = ref('')
const showModelDropdown = ref(false)
const displayModelText = computed({
  get: () => {
    if (showModelDropdown.value) {
      return modelSearchText.value
    }
    return selectedModel.value ? getCurrentModelName() : ''
  },
  set: (value: string) => {
    modelSearchText.value = value
  }
})

// 快速建议
const quickSuggestions = ref([
])

// 方法
const sendMessage = async () => {
  if (!inputMessage.value.trim() || isLoading.value) return

  const userMessage: ChatMessage = {
    id: Date.now().toString(),
    role: 'user',
    content: inputMessage.value,
    timestamp: new Date()
  }
  
  messages.value.push(userMessage)
  const userInput = inputMessage.value
  inputMessage.value = ''
  isLoading.value = true

  // 添加助手消息
  const assistantMessage: ChatMessage = {
    id: (Date.now() + 1).toString(),
    role: 'assistant',
    content: '',
    timestamp: new Date(),
    isStreaming: true,
    executionPlan: null,
    toolExecutions: [],
    executionResult: null,
    executionProgress: 0,
    currentStep: undefined,
    totalSteps: 0,
    completedSteps: 0
  }
  messages.value.push(assistantMessage)
  
  scrollToBottom()

  try {
    // 先进行意图分类，智能路由用户请求
    const routeResult = await invoke<any>('smart_route_user_request', {
      userInput: userInput
    })

    console.log('Intent classification result:', routeResult)

    // 根据分类结果处理
    if (routeResult.type === 'chat' || routeResult.type === 'question') {
      // 普通对话或知识问答，使用流式响应
      
      // 确保有当前会话
      if (!currentConversationId.value) {
        await createNewConversation()
      }
      
      // 启动流式聊天
      try {
        // 设置流式开始状态
        streamStartTime.value = Date.now()
        streamCharCount.value = 0
        
        // 使用前端生成的消息ID
        const messageId = assistantMessage.id
        console.log('Starting stream chat with message ID:', messageId)
        
        await invoke<string>('send_ai_stream_message', {
          request: {
            conversation_id: currentConversationId.value,
            message: userInput,
            service_name: 'default',
            provider: selectedModel.value ? selectedModel.value.split('/')[0] : undefined,
            model: selectedModel.value ? (
              selectedModel.value.split('/').length > 1 ? 
                selectedModel.value.split('/').slice(1).join('/') : // 提取provider之后的完整模型名
                selectedModel.value // 如果没有斜杠，使用整个字符串
            ) : undefined,
            temperature: undefined, // 使用默认配置
            max_tokens: undefined,  // 使用默认配置
            system_prompt: undefined,
            message_id: messageId // 传递前端生成的消息ID
          }
        })
        
        console.log('Started stream chat with message ID:', messageId)
        
        // 立即滚动到底部显示流式状态
        scrollToBottom()
        
      } catch (streamError) {
        console.error('Failed to start stream chat:', streamError)
        // 回退到直接显示内容
        assistantMessage.content = routeResult.response?.content || `${t('aiAssistant.error', '错误')}: ${streamError}`
        assistantMessage.isStreaming = false
        assistantMessage.hasError = true
        isLoading.value = false
        streamStartTime.value = null
        streamCharCount.value = 0
      }
      
      // 保存用户消息
      if (currentConversationId.value) {
        try {
          await saveMessagesToConversation([userMessage])
        } catch (error) {
          console.error('Failed to save user message to conversation:', error)
        }
      }
      
    } else if (routeResult.type === 'task' && routeResult.needs_agent_execution) {
      // 确保有当前会话用于任务执行
      if (!currentConversationId.value) {
        await createNewConversation()
      }
      
      // 需要Agent执行的任务，调用原有的智能调度器
      const dispatchResult = await invoke<DispatchResult>('dispatch_intelligent_query', {
        request: {
          query: userInput,
          architecture: mapArchitectureToId(props.selectedArchitecture),
          agent_id: props.selectedAgent?.id
        }
      })

      // 更新消息内容
      assistantMessage.executionPlan = dispatchResult.execution_plan
      assistantMessage.content = dispatchResult.initial_response || t('aiAssistant.planningExecution', '正在规划执行...')
      currentExecutionId.value = dispatchResult.execution_id

      // 通知父组件执行开始
      emit('execution-started', {
        id: dispatchResult.execution_id,
        name: dispatchResult.execution_plan?.name || 'AI任务执行',
        description: userInput,
        progress: 0,
        status: 'running'
      })

      // 注意：执行已在后端dispatch_intelligent_query中自动开始，无需重复调用
    } else {
      // 其他情况，显示分类信息
      assistantMessage.content = `分类结果: ${routeResult.classification.intent} (置信度: ${(routeResult.classification.confidence * 100).toFixed(1)}%)`
      assistantMessage.isStreaming = false
      isLoading.value = false
    }

  } catch (error) {
    console.error('Failed to send message:', error)
    assistantMessage.content = `${t('aiAssistant.error', '错误')}: ${error}`
    assistantMessage.isStreaming = false
    isLoading.value = false
  }
}

const sendQuickMessage = (message: string) => {
  inputMessage.value = message
  sendMessage()
}

// 重试最后一条消息
const retryLastMessage = () => {
  // 找到最后一条用户消息
  const userMessages = messages.value.filter(m => m.role === 'user')
  if (userMessages.length > 0) {
    const lastUserMessage = userMessages[userMessages.length - 1]
    inputMessage.value = lastUserMessage.content
    sendMessage()
  }
}

// 清除错误消息
const clearErrorMessage = (message: ChatMessage) => {
  message.hasError = false
  message.content = '[已清除错误消息]'
}

const stopExecution = async () => {
  // 取消任务执行
  if (currentExecutionId.value) {
    try {
      await invoke('stop_execution', {
        execution_id: currentExecutionId.value
      })
    } catch (error) {
      console.error('Failed to stop execution:', error)
    }
  }
  
  // 取消流式聊天
  if (currentConversationId.value) {
    try {
      await invoke('cancel_ai_stream', {
        conversation_id: currentConversationId.value
      })
    } catch (error) {
      console.error('Failed to cancel stream:', error)
    }
  }
  
  // 停止当前流式消息
  const lastAssistantMessage = messages.value.filter(m => m.role === 'assistant').pop()
  if (lastAssistantMessage && lastAssistantMessage.isStreaming) {
    lastAssistantMessage.isStreaming = false
    lastAssistantMessage.content += '\n\n[用户中断了响应]'
  }
  
  isLoading.value = false
}

const clearConversation = () => {
  messages.value = []
  currentExecutionId.value = null
}

// 会话管理方法
const createNewConversation = async () => {
  isLoadingConversations.value = true
  try {
    const result = await invoke('create_ai_conversation', {
      request: {
        title: `AI会话 ${new Date().toLocaleString()}`,
        service_name: "default"
      }
    })
    
    // 切换到新会话
    currentConversationId.value = result as string
    messages.value = []
    currentExecutionId.value = null
    
    // 刷新会话列表
    await loadConversations()
    
    console.log('New conversation created:', result)
  } catch (error) {
    console.error('Failed to create new conversation:', error)
  } finally {
    isLoadingConversations.value = false
  }
}

const loadConversations = async () => {
  isLoadingConversations.value = true
  try {
    const result = await invoke('get_ai_conversations')
    conversations.value = result as any[]
    console.log('Loaded conversations:', result)
  } catch (error) {
    console.error('Failed to load conversations:', error)
    conversations.value = []
  } finally {
    isLoadingConversations.value = false
  }
}

const switchToConversation = async (conversationId: string) => {
  try {
    currentConversationId.value = conversationId
    showConversationsList.value = false
    
    // 加载会话历史
    const history = await invoke('get_ai_conversation_history', {
      conversation_id: conversationId,
      service_name: "default"
    })
    
    // 将历史消息转换为ChatMessage格式
    const historyMessages = (history as any[]).map((msg: any) => ({
      id: msg.id,
      role: msg.role,
      content: msg.content,
      timestamp: new Date(msg.timestamp),
      isStreaming: false
    }))
    
    messages.value = historyMessages
    scrollToBottom()
    
    console.log('Switched to conversation:', conversationId, 'History:', history)
  } catch (error) {
    console.error('Failed to switch conversation:', error)
  }
}

const clearCurrentConversation = () => {
  messages.value = []
  currentExecutionId.value = null
  
  // 清理本地存储
  if (currentConversationId.value) {
    const sessionKey = `ai_chat_session_${currentConversationId.value}`
    localStorage.removeItem(sessionKey)
  }
}

const deleteCurrentConversation = async () => {
  if (!currentConversationId.value) return
  
  try {
    const conversationToDelete = currentConversationId.value
    
    await invoke('delete_ai_conversation', {
      conversationId: conversationToDelete,
      serviceName: "default"
    })
    
    // 清理本地存储
    const sessionKey = `ai_chat_session_${conversationToDelete}`
    localStorage.removeItem(sessionKey)
    
    // 清空当前会话
    currentConversationId.value = null
    messages.value = []
    currentExecutionId.value = null
    
    // 刷新会话列表
    await loadConversations()
    
    console.log('Current conversation deleted')
  } catch (error) {
    console.error('Failed to delete current conversation:', error)
  }
}

const deleteConversation = async (conversationId: string) => {
  try {
    await invoke('delete_ai_conversation', {
      conversationId: conversationId,
      serviceName: "default"
    })
    
    // 清理本地存储
    const sessionKey = `ai_chat_session_${conversationId}`
    localStorage.removeItem(sessionKey)
    
    // 如果删除的是当前会话，清空
    if (conversationId === currentConversationId.value) {
      currentConversationId.value = null
      messages.value = []
      currentExecutionId.value = null
    }
    
    // 刷新会话列表
    await loadConversations()
    
    console.log('Conversation deleted:', conversationId)
  } catch (error) {
    console.error('Failed to delete conversation:', error)
  }
}

const getCurrentConversationTitle = () => {
  if (!currentConversationId.value) return t('aiAssistant.newConversation', '新会话')
  
  const conv = conversations.value.find(c => c.id === currentConversationId.value)
  return conv?.title || t('aiAssistant.untitledConversation', '无标题会话')
}

const saveMessagesToConversation = async (messagesToSave: ChatMessage[]) => {
  if (!currentConversationId.value) return
  
  try {
    // 逐个保存消息
    for (const message of messagesToSave) {
      await invoke('save_ai_message', {
        request: {
          conversation_id: currentConversationId.value,
          role: message.role,
          content: message.content
        }
      })
    }
    
    console.log('Messages saved to conversation:', currentConversationId.value)
  } catch (error) {
    console.error('Failed to save messages to conversation:', error)
    throw error
  }
}

const scrollToBottom = () => {
  nextTick(() => {
    if (messagesContainer.value) {
      messagesContainer.value.scrollTop = messagesContainer.value.scrollHeight
    }
  })
}

const renderMarkdown = (content: string) => marked(content)

const formatTime = (timestamp: Date) => {
  return timestamp.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })
}

const formatToolResult = (result: any) => {
  if (typeof result === 'string') return result
  return JSON.stringify(result, null, 2)
}

const getStepStatusClass = (status: string) => {
  switch (status) {
    case 'pending': return 'badge-ghost'
    case 'executing':
    case 'running': return 'badge-warning'
    case 'completed': return 'badge-success'
    case 'failed': return 'badge-error'
    default: return 'badge-ghost'
  }
}

const getStepStatusText = (status: string) => {
  switch (status) {
    case 'pending': return '待执行'
    case 'executing': return '执行中'
    case 'running': return '运行中'
    case 'completed': return '已完成'
    case 'failed': return '失败'
    default: return status
  }
}

const getToolStatusClass = (status: string) => {
  switch (status) {
    case 'running': return 'badge-warning'
    case 'completed': return 'badge-success'
    case 'failed': return 'badge-error'
    default: return 'badge-ghost'
  }
}

const getResultStatusClass = (status: string) => {
  switch (status) {
    case 'success':
    case 'completed': return 'badge-success'
    case 'failure':
    case 'failed': return 'badge-error'
    case 'partial':
    case 'running': return 'badge-warning'
    default: return 'badge-ghost'
  }
}

const getResultStatusText = (status: string) => {
  switch (status) {
    case 'success': return '成功'
    case 'completed': return '已完成'
    case 'failure': return '失败'
    case 'failed': return '失败'
    case 'partial': return '部分完成'
    case 'running': return '运行中'
    default: return status
  }
}

const formatDuration = (milliseconds: number) => {
  if (milliseconds < 1000) {
    return `${milliseconds}ms`
  }
  const seconds = Math.floor(milliseconds / 1000)
  if (seconds < 60) {
    return `${seconds}秒`
  }
  const minutes = Math.floor(seconds / 60)
  const remainingSeconds = seconds % 60
  return `${minutes}分${remainingSeconds}秒`
}

const formatTimestamp = (timestamp: number) => {
  if (!timestamp) return '-'
  const date = new Date(timestamp * 1000)
  return date.toLocaleTimeString('zh-CN', { hour: '2-digit', minute: '2-digit', second: '2-digit' })
}

const getStepResultSummary = (result: any) => {
  if (!result) return '-'
  
  // 优先显示 reasoning_result 的摘要
  if (typeof result === 'object' && result.reasoning_result) {
    const reasoning = result.reasoning_result
    if (typeof reasoning === 'string') {
      return reasoning.length > 100 ? reasoning.substring(0, 100) + '...' : reasoning
    }
  }
  
  if (typeof result === 'string') {
    return result.length > 100 ? result.substring(0, 100) + '...' : result
  }
  
  if (typeof result === 'object') {
    if (result.message) {
      return result.message
    }
    if (result.action) {
      return `${result.action}: ${result.message || '执行完成'}`
    }
    const jsonStr = JSON.stringify(result)
    return jsonStr.length > 100 ? jsonStr.substring(0, 100) + '...' : jsonStr
  }
  
  return String(result)
}

// 获取执行结果的详细内容 - 优先显示最后一步的 reasoning_result
const getExecutionDetailedResult = (message: ChatMessage) => {
  // 首先尝试从最后一步的 reasoning_result 获取
  if (message.executionPlan?.steps && message.executionPlan.steps.length > 0) {
    const lastStep = message.executionPlan.steps[message.executionPlan.steps.length - 1]
    const reasoningResult = getStepReasoningResult(lastStep)
    if (reasoningResult) {
      return reasoningResult
    }
  }
  
  // 然后尝试从执行结果获取
  if (message.executionResult) {
    if (message.executionResult.result) {
      return message.executionResult.result
    }
    if (message.executionResult.data) {
      return message.executionResult.data
    }
  }
  
  return null
}

// 获取步骤的结果数据 - 兼容 result_data 和 result 字段
const getStepResultData = (step: any) => {
  return step.result_data || step.result || null
}

// 获取步骤的推理结果 - 从结果数据中提取 reasoning_result
const getStepReasoningResult = (step: any) => {
  const resultData = getStepResultData(step)
  if (!resultData) return null
  
  // 如果结果数据是对象且包含 reasoning_result
  if (typeof resultData === 'object' && resultData.reasoning_result) {
    return resultData.reasoning_result
  }
  
  // 如果结果数据是字符串，尝试解析JSON
  if (typeof resultData === 'string') {
    try {
      const parsed = JSON.parse(resultData)
      if (parsed.reasoning_result) {
        return parsed.reasoning_result
      }
    } catch (e) {
      // 如果解析失败，返回null，让其他逻辑处理
    }
  }
  
  return null
}

// 将后端步骤状态转换为前端状态
const getStepStatusFromBackend = (backendStatus: string) => {
  const statusMap: Record<string, string> = {
    'Pending': 'pending',
    'Running': 'executing',
    'Executing': 'executing',
    'Completed': 'completed',
    'Failed': 'failed',
    'Skipped': 'skipped',
    'Cancelled': 'failed'
  }
  return statusMap[backendStatus] || backendStatus?.toLowerCase()
}

// 步骤详情查看
const stepDetailVisible = ref(false)
const selectedStepDetail = ref<any>(null)

const viewStepDetail = (step: any) => {
  selectedStepDetail.value = step
  stepDetailVisible.value = true
}

const closeStepDetail = () => {
  stepDetailVisible.value = false
  selectedStepDetail.value = null
}

const mapArchitectureToId = (architectureName: string) => {
  const mapping: Record<string, string> = {
    'Plan-and-Execute': 'plan-execute',
    'ReWOO': 'rewoo',
    'LLMCompiler': 'llm-compiler',
    'Intelligent Dispatcher': 'intelligent-dispatcher'
  }
  return mapping[architectureName] || 'plan-execute'
}

// 架构选择相关方法
const selectArchitecture = (architecture: any) => {
  emit('architecture-changed', architecture)
}

const getArchBadgeClass = (status: string) => {
  switch (status) {
    case 'stable': return 'badge-success'
    case 'beta': return 'badge-warning'
    case 'experimental': return 'badge-info'
    case 'ai-powered': return 'badge-accent'
    default: return 'badge-ghost'
  }
}

const getArchBadgeText = (status: string) => {
  switch (status) {
    case 'stable': return 'STABLE'
    case 'beta': return 'BETA'
    case 'experimental': return 'EXPERIMENTAL'
    case 'ai-powered': return 'AI'
    default: return status?.toUpperCase?.() || 'N/A'
  }
}

// 获取可用架构列表，提供默认选项
const getAvailableArchitectures = () => {
  if (props.availableArchitectures && props.availableArchitectures.length > 0) {
    return props.availableArchitectures
  }
  
  // 提供默认架构选项
  return [
    {
      id: 'plan-execute',
      name: 'Plan-and-Execute',
      description: '计划执行架构：先制定计划，再逐步执行',
      status: 'stable'
    },
    {
      id: 'rewoo',
      name: 'ReWOO',
      description: '推理无观察架构：减少工具调用的推理方法',
      status: 'beta'
    },
    {
      id: 'llm-compiler',
      name: 'LLMCompiler',
      description: 'LLM编译器：并行执行任务的先进架构',
      status: 'experimental'
    },
    {
      id: 'intelligent-dispatcher',
      name: 'Intelligent Dispatcher',
      description: '智能调度器：AI驱动的智能任务分发',
      status: 'ai-powered'
    }
  ]
}

const getCurrentModelName = () => {
  if (!selectedModel.value) return '未选择'
  const parts = selectedModel.value.split('/')
  // 处理 provider/modelname 格式，其中 modelname 可能包含斜杠
  // 格式: provider/modelname，所以模型名是第一个斜杠之后的所有内容
  if (parts.length > 1) {
    // 移除第一个部分（provider），剩余部分用斜杠重新连接
    return parts.slice(1).join('/')
  }
  return selectedModel.value
}

// 计算模型输入框的自适应宽度
const getModelInputWidth = () => {
  if (showModelDropdown.value) {
    return '160px' // 搜索时固定宽度
  }
  
  if (!selectedModel.value) {
    return '120px' // 未选择时的默认宽度
  }
  
  // 根据模型名称长度计算宽度
  const modelName = getCurrentModelName()
  const baseWidth = 80 // 基础宽度
  const charWidth = 6.5 // 每个字符大约的宽度（像素）
  const padding = 30 // 左右内边距和图标宽度
  
  const calculatedWidth = baseWidth + (modelName.length * charWidth) + padding
  const maxWidth = 200 // 最大宽度限制
  const minWidth = 100 // 最小宽度限制
  
  return `${Math.min(Math.max(calculatedWidth, minWidth), maxWidth)}px`
}

// 计算流式响应速度
const getStreamSpeed = () => {
  if (!streamStartTime.value || streamCharCount.value === 0) return 0
  const elapsed = (Date.now() - streamStartTime.value) / 1000 // 秒
  return Math.round(streamCharCount.value / elapsed) // 字符/秒
}

// 模型搜索相关方法
const filteredModels = computed(() => {
  if (!modelSearchText.value.trim()) {
    return groupedModels.value
  }
  
  const searchTerm = modelSearchText.value.toLowerCase()
  const filtered: GroupedModels = {}
  
  for (const [provider, models] of Object.entries(groupedModels.value)) {
    const matchedModels = models.filter(model => 
      model.name.toLowerCase().includes(searchTerm) ||
      model.provider.toLowerCase().includes(searchTerm)
    )
    if (matchedModels.length > 0) {
      filtered[provider] = matchedModels
    }
  }
  
  return filtered
})

const onModelInputFocus = () => {
  showModelDropdown.value = true
  modelSearchText.value = ''
}

const onModelInput = () => {
  showModelDropdown.value = true
}

const selectModel = async (model: ModelInfo) => {
  selectedModel.value = model.id
  modelSearchText.value = ''
  showModelDropdown.value = false
  
  // 保存选择的模型到数据库作为默认聊天模型
  try {
    const parts = model.id.split('/')
    if (parts.length >= 2) {
      await invoke('set_default_ai_model', {
        modelType: 'chat',
        provider: parts[0],
        modelName: parts.slice(1).join('/') // 使用完整的模型名（provider之后的所有部分）
      })
      console.log('Default chat model saved:', model.id)
    }
  } catch (error) {
    console.error('Failed to save default model:', error)
  }
}

const onModelInputBlur = () => {
  setTimeout(() => {
    showModelDropdown.value = false
  }, 200)
}

// 监听执行事件
let unlistenCallbacks: (() => void)[] = []

onMounted(async () => {
  // 初始化意图分类器
  try {
    await invoke('initialize_intent_classifier')
    console.log('Intent classifier initialized successfully')
  } catch (error) {
    console.error('Failed to initialize intent classifier:', error)
  }

  // 恢复会话状态
  restoreSessionState()

  // 加载会话列表
  await loadConversations()
  
  // 不再自动创建会话，而是在用户发送第一条消息时创建

  // 加载可用模型
  try {
    const models = await invoke('get_ai_chat_models') as any[]
    console.log('Loaded AI models:', models)
    
    if (Array.isArray(models) && models.length > 0) {
      const grouped: GroupedModels = {}
      for (const model of models) {
        if (model.provider && model.name) {
          if (!grouped[model.provider]) {
            grouped[model.provider] = []
          }
          grouped[model.provider].push({
            id: `${model.provider}/${model.name}`,
            name: model.name,
            provider: model.provider
          })
        }
      }
      groupedModels.value = grouped
      console.log('Grouped models:', grouped)

      // 设置默认模型
      try {
        const defaultModel = await invoke('get_default_ai_model', { modelType: 'chat' }) as any
        if (defaultModel && defaultModel.provider && defaultModel.name) {
          selectedModel.value = `${defaultModel.provider}/${defaultModel.name}`
          console.log('Default model set:', selectedModel.value)
        } else if (Object.keys(grouped).length > 0) {
          // 如果没有默认模型，选择第一个可用模型
          const firstProvider = Object.keys(grouped)[0]
          const firstModel = grouped[firstProvider][0]
          selectedModel.value = firstModel.id
          console.log('Auto-selected first model:', selectedModel.value)
        }
      } catch (defaultError) {
        console.error('Failed to get default model:', defaultError)
        // 选择第一个可用模型作为备用
        if (Object.keys(grouped).length > 0) {
          const firstProvider = Object.keys(grouped)[0]
          const firstModel = grouped[firstProvider][0]
          selectedModel.value = firstModel.id
          console.log('Fallback to first model:', selectedModel.value)
        }
      }
    } else {
      console.warn('No models available or invalid models response')
    }
  } catch (error) {
    console.error('Failed to load models:', error)
    // 提供一个默认的模型选项以便测试
    groupedModels.value = {
      'test': [{
        id: 'test/default',
        name: '测试模型',
        provider: 'test'
      }]
    }
  }

  // 监听执行进度
  const unlistenProgress = await listen('execution_progress', (event) => {
    const data = event.payload as any
    if (data.execution_id === currentExecutionId.value) {
      emit('execution-progress', data.progress || data.percentage || 0)
      
      // 更新消息中的执行进度
      const lastAssistantMessage = messages.value.filter(m => m.role === 'assistant').pop()
      if (lastAssistantMessage) {
        lastAssistantMessage.executionProgress = data.progress || data.percentage || 0
        lastAssistantMessage.currentStep = data.current_step || data.step_name
        lastAssistantMessage.totalSteps = data.total_steps
        lastAssistantMessage.completedSteps = data.completed_steps
        
        // 更新工具执行状态
        if (data.tool_execution) {
          if (!lastAssistantMessage.toolExecutions) {
            lastAssistantMessage.toolExecutions = []
          }
          const existingTool = lastAssistantMessage.toolExecutions.find((t: any) => t.id === data.tool_execution.id)
          if (existingTool) {
            Object.assign(existingTool, data.tool_execution)
          } else {
            lastAssistantMessage.toolExecutions.push(data.tool_execution)
          }
        }
        
        // 更新执行计划中的步骤状态
        if (data.step_update && lastAssistantMessage.executionPlan?.steps) {
          const step = lastAssistantMessage.executionPlan.steps.find((s: any) => s.id === data.step_update.id)
          if (step) {
            Object.assign(step, data.step_update)
            console.log('实时更新步骤状态:', step.name, step.status)
          }
        }
        
        // 实时更新当前执行步骤的状态
        if (data.current_step && lastAssistantMessage.executionPlan?.steps) {
          // 将之前执行中的步骤设为完成状态
          lastAssistantMessage.executionPlan.steps.forEach((step: any) => {
            if (step.status === 'executing' && step.name !== data.current_step) {
              step.status = 'completed'
            }
          })
          
          // 设置当前步骤为执行中
          const currentStep = lastAssistantMessage.executionPlan.steps.find((s: any) => s.name === data.current_step)
          if (currentStep && currentStep.status !== 'executing') {
            currentStep.status = 'executing'
            currentStep.started_at = Date.now() / 1000
            console.log('步骤开始执行:', currentStep.name)
          }
        }
      }
      scrollToBottom()
    }
  })

  // 监听执行完成
  const unlistenComplete = await listen('execution_completed', (event) => {
    const data = event.payload as any
    if (data.execution_id === currentExecutionId.value) {
      const lastAssistantMessage = messages.value.filter(m => m.role === 'assistant').pop()
      if (lastAssistantMessage) {
        lastAssistantMessage.isStreaming = false
        lastAssistantMessage.executionResult = data.result || data
        lastAssistantMessage.content = data.final_response || data.response || lastAssistantMessage.content
        lastAssistantMessage.executionProgress = 100
        
        // 从执行结果中提取步骤结果数据
        if (lastAssistantMessage.executionPlan?.steps && data.result) {
          // 尝试从执行结果中获取步骤结果映射 - 支持多种路径
          const stepResults = data.result.step_results || 
                             data.result.data?.step_results || 
                             (data.result.data && typeof data.result.data === 'object' ? data.result.data.step_results : null) || 
                             {}
          
          console.log('提取到的步骤结果:', stepResults)
          console.log('执行结果数据结构:', data.result)
          
          lastAssistantMessage.executionPlan.steps.forEach((step: any, index: number) => {
            // 查找对应的步骤结果数据
            const stepResult = stepResults[step.id] || stepResults[step.name]
            
            if (stepResult) {
              // 更新步骤状态
              step.status = getStepStatusFromBackend(stepResult.status) || 'completed'
              step.started_at = stepResult.started_at
              step.completed_at = stepResult.completed_at
              step.result_data = stepResult.result_data
              step.result = stepResult.result_data // 兼容性
              step.error = stepResult.error
              console.log(`步骤 ${step.name} 已更新结果:`, stepResult)
            } else {
              // 如果没有找到步骤结果，假设已完成
              step.status = step.status === 'failed' ? 'failed' : 'completed'
              console.log(`步骤 ${step.name} 未找到结果数据，设为默认状态:`, step.status)
            }
          })
          
          lastAssistantMessage.completedSteps = lastAssistantMessage.executionPlan.steps.filter((s: any) => s.status === 'completed').length
        }
        
        // 自动保存完整的执行结果到会话
        if (currentConversationId.value) {
          try {
            saveMessagesToConversation([lastAssistantMessage])
          } catch (error) {
            console.error('Failed to save execution result to conversation:', error)
          }
        }
      }
      
      emit('execution-completed', data.result || data)
      isLoading.value = false
      currentExecutionId.value = null
      scrollToBottom()
    }
  })

  // 监听流式响应
  const unlistenStream = await listen('ai_stream_message', (event) => {
    const data = event.payload as any
    console.log('Received stream message:', {
      conversation_id: data.conversation_id,
      message_id: data.message_id,
      content_length: (data.content || '').length,
      is_complete: data.is_complete,
      content_preview: (data.content || '').substring(0, 50)
    })
    
    // 检查是否是任务执行的流式响应
    if (data.execution_id === currentExecutionId.value) {
      const lastAssistantMessage = messages.value.filter(m => m.role === 'assistant').pop()
      if (lastAssistantMessage) {
        lastAssistantMessage.content = data.content || ''
        if (data.is_complete) {
          lastAssistantMessage.isStreaming = false
          isLoading.value = false
        }
      }
      scrollToBottom()
    }
    
    // 检查是否是普通聊天的流式响应
    if (data.conversation_id === currentConversationId.value && data.message_id) {
      const targetMessage = messages.value.find(m => m.id === data.message_id)
      if (targetMessage) {
        // 更新消息内容 - 确保流式更新
        const newContent = data.content || ''
        if (newContent !== targetMessage.content) {
          targetMessage.content = newContent
          console.log('Stream update - content length:', newContent.length, 'is_complete:', data.is_complete)
        }
        
        // 更新字符计数
        streamCharCount.value = newContent.length
        
        // 确保消息仍处于流式状态（除非明确完成）
        if (!data.is_complete && !targetMessage.isStreaming) {
          targetMessage.isStreaming = true
          console.log('Message set to streaming state')
        }
        
        if (data.is_complete) {
          targetMessage.isStreaming = false
          isLoading.value = false
          console.log('Stream completed for message:', data.message_id)
          
          // 保存完成的助手消息到会话
          if (currentConversationId.value) {
            saveMessagesToConversation([targetMessage]).catch(error => {
              console.error('Failed to save assistant message to conversation:', error)
            })
          }
          
          // 重置计时器
          streamStartTime.value = null
          streamCharCount.value = 0
        }
        
        // 实时滚动到底部
        nextTick(() => {
          scrollToBottom()
        })
      } else {
        console.warn('Target message not found for stream update:', data.message_id)
      }
    }
  })

  // 监听流式响应开始
  const unlistenStreamStart = await listen('ai_stream_start', (event) => {
    const data = event.payload as any
    console.log('Stream started:', data)
    
    // 记录开始时间
    streamStartTime.value = Date.now()
    streamCharCount.value = 0
    
    // 找到对应的消息并确保其处于流式状态
    if (data.conversation_id === currentConversationId.value && data.message_id) {
      const targetMessage = messages.value.find(m => m.id === data.message_id)
      if (targetMessage) {
        targetMessage.isStreaming = true
        targetMessage.content = '' // 重置内容为空，准备接收流式数据
        console.log('Message set to streaming state:', data.message_id)
        // 立即滚动到底部确保用户看到流式状态
        nextTick(() => {
          scrollToBottom()
        })
      } else {
        console.warn('Message not found for stream start:', data.message_id)
      }
    }
  })

  // 监听流式响应错误
  const unlistenStreamError = await listen('ai_stream_error', (event) => {
    const data = event.payload as any
    console.error('Stream error:', data)
    
    if (data.conversation_id === currentConversationId.value || data.message_id) {
      // 尝试根据message_id或conversation_id查找消息
      const targetMessage = data.message_id 
        ? messages.value.find(m => m.id === data.message_id)
        : messages.value.filter(m => m.role === 'assistant').pop()
        
      if (targetMessage) {
        targetMessage.content = `❌ 流式响应出错: ${data.error}\n\n点击下方"重新发送"按钮重试。`
        targetMessage.isStreaming = false
        targetMessage.hasError = true
        isLoading.value = false
        
        // 重置计时器
        streamStartTime.value = null
        streamCharCount.value = 0
        
        scrollToBottom()
      } else {
        console.error('Could not find message to show error for')
      }
    }
  })

  // 监听流式响应完成
  const unlistenStreamComplete = await listen('ai_stream_complete', (event) => {
    const data = event.payload as any
    console.log('Stream completed:', data)
    
    if (data.conversation_id === currentConversationId.value || data.message_id) {
      // 尝试根据message_id或conversation_id查找消息
      const targetMessage = data.message_id 
        ? messages.value.find(m => m.id === data.message_id)
        : messages.value.filter(m => m.role === 'assistant').pop()
        
      if (targetMessage && targetMessage.isStreaming) {
        targetMessage.isStreaming = false
        isLoading.value = false
        
        console.log('Stream completed for message:', targetMessage.id, 'Content length:', targetMessage.content.length)
        
        // 保存完成的助手消息到会话
        if (currentConversationId.value) {
          saveMessagesToConversation([targetMessage]).catch(error => {
            console.error('Failed to save assistant message to conversation:', error)
          })
        }
        
        // 重置计时器
        streamStartTime.value = null
        streamCharCount.value = 0
        
        // 最终滚动到底部
        scrollToBottom()
      }
    }
  })

  // 监听步骤初始化事件
  const unlistenStepsInit = await listen('execution_steps_initialized', (event) => {
    const data = event.payload as any
    if (data.execution_id === currentExecutionId.value) {
      const lastAssistantMessage = messages.value.filter(m => m.role === 'assistant').pop()
      if (lastAssistantMessage && data.steps) {
        if (!lastAssistantMessage.executionPlan) {
          lastAssistantMessage.executionPlan = {}
        }
        lastAssistantMessage.executionPlan.steps = data.steps.map((step: any) => ({
          id: step.id,
          name: step.name || step.description,
          description: step.description,
          status: step.status || 'pending',
          started_at: step.started_at,
          completed_at: step.completed_at,
          result: step.result,
          error: step.error
        }))
        lastAssistantMessage.totalSteps = data.steps.length
      }
      scrollToBottom()
    }
  })

  // 监听步骤开始事件
  const unlistenStepStart = await listen('execution_step_started', (event) => {
    const data = event.payload as any
    if (data.execution_id === currentExecutionId.value) {
      const lastAssistantMessage = messages.value.filter(m => m.role === 'assistant').pop()
      if (lastAssistantMessage && lastAssistantMessage.executionPlan?.steps) {
        // 找到步骤并更新状态
        const step = lastAssistantMessage.executionPlan.steps.find((s: any) => 
          s.id === data.step_id || s.name === data.step_name)
        if (step) {
          step.status = 'executing'
          step.started_at = data.started_at || Date.now() / 1000
          console.log('步骤开始事件 - 实时更新:', step.name, '状态:', step.status)
        }
        
        // 将之前执行中的其他步骤设为完成状态
        lastAssistantMessage.executionPlan.steps.forEach((s: any) => {
          if (s.id !== data.step_id && s.name !== data.step_name && s.status === 'executing') {
            s.status = 'completed'
            if (!s.completed_at) {
              s.completed_at = Date.now() / 1000
            }
          }
        })
        
        lastAssistantMessage.currentStep = data.step_name
      }
      scrollToBottom()
    }
  })

  // 监听步骤完成事件
  const unlistenStepComplete = await listen('execution_step_completed', (event) => {
    const data = event.payload as any
    if (data.execution_id === currentExecutionId.value) {
      const lastAssistantMessage = messages.value.filter(m => m.role === 'assistant').pop()
      if (lastAssistantMessage && lastAssistantMessage.executionPlan?.steps) {
        const step = lastAssistantMessage.executionPlan.steps.find((s: any) => 
          s.id === data.step_id || s.name === data.step_name)
        if (step) {
          step.status = data.status || 'completed'
          step.completed_at = data.completed_at || Date.now() / 1000
          step.result_data = data.result_data || data.result
          step.result = data.result_data || data.result // 兼容性
          step.error = data.error
          console.log('步骤完成事件 - 实时更新:', step.name, '状态:', step.status, '结果:', step.result_data ? '有' : '无')
        }
        
        // 更新完成步骤数和进度
        const completedSteps = lastAssistantMessage.executionPlan.steps.filter((s: any) => s.status === 'completed').length
        lastAssistantMessage.completedSteps = completedSteps
        if (lastAssistantMessage.totalSteps && lastAssistantMessage.totalSteps > 0) {
          lastAssistantMessage.executionProgress = (completedSteps / lastAssistantMessage.totalSteps) * 100
        }
      }
      scrollToBottom()
    }
  })

  unlistenCallbacks = [
    unlistenProgress, 
    unlistenComplete, 
    unlistenStream, 
    unlistenStreamStart,
    unlistenStreamError,
    unlistenStreamComplete,
    unlistenStepsInit, 
    unlistenStepStart, 
    unlistenStepComplete
  ]
})

// 监听架构变化
watch(() => props.selectedArchitecture, () => {
  // 架构变化时可以显示提示信息
})

// 监听模型选择变化，更新搜索文本显示
watch(selectedModel, (newModel) => {
  if (newModel && !modelSearchText.value) {
    // 只有在用户未手动输入时才更新显示
    modelSearchText.value = ''
  }
})

// 监听会话变化，持久化存储当前会话状态
watch(currentConversationId, (newId) => {
  if (newId) {
    localStorage.setItem('ai_chat_current_conversation_id', newId)
  } else {
    localStorage.removeItem('ai_chat_current_conversation_id')
  }
})

// 监听消息变化，持久化存储到本地
watch(messages, (newMessages) => {
  if (currentConversationId.value) {
    const sessionKey = `ai_chat_session_${currentConversationId.value}`
    localStorage.setItem(sessionKey, JSON.stringify(newMessages))
  }
}, { deep: true })

// 恢复会话状态
const restoreSessionState = () => {
  const savedConversationId = localStorage.getItem('ai_chat_current_conversation_id')
  if (savedConversationId) {
    currentConversationId.value = savedConversationId
    
    // 恢复消息
    const sessionKey = `ai_chat_session_${savedConversationId}`
    const savedMessages = localStorage.getItem(sessionKey)
    if (savedMessages) {
      try {
        const parsedMessages = JSON.parse(savedMessages)
        // 确保时间戳是Date对象
        messages.value = parsedMessages.map((msg: any) => ({
          ...msg,
          timestamp: new Date(msg.timestamp)
        }))
      } catch (error) {
        console.error('Failed to restore session messages:', error)
        messages.value = []
      }
    }
  }
}

// 清理
onUnmounted(() => {
  unlistenCallbacks.forEach(callback => callback())
})
</script>

<style scoped>
.enhanced-ai-chat {
  font-family: 'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
  position: relative;
  overflow: hidden;
  max-width: 100vw;
  max-height: 100vh;
  box-sizing: border-box;
}




/* 聊天气泡动画 */
.chat {
  animation: fadeInUp 0.4s ease-out;
}


/* 流式内容打字机动画 */
.streaming-content {
  position: relative;
}

.streaming-content::after {
  content: '▌';
  animation: blink 1.2s infinite;
  color: hsl(var(--p));
  font-weight: bold;
  margin-left: 2px;
}

@keyframes blink {
  0%, 50% {
    opacity: 1;
  }
  51%, 100% {
    opacity: 0;
  }
}

/* 流式消息的平滑过渡效果 */
.chat-bubble .prose {
  transition: opacity 0.1s ease-out;
}

/* 确保流式内容有足够的最小高度 */
.streaming-content:empty::before {
  content: ' ';
  display: inline-block;
  width: 1px;
  height: 1em;
}

/* 加载动画增强 */
.loading-dots {
  animation: loading 1.4s infinite;
}

@keyframes loading {
  0%, 80%, 100% {
    opacity: 0.4;
  }
  40% {
    opacity: 1;
  }
}

/* 悬停效果 */
.chat-bubble {
  transition: all 0.2s ease;
}

.chat:hover .chat-bubble {
  transform: translateY(-1px);
  box-shadow: 0 8px 25px rgba(0, 0, 0, 0.1);
}

/* 收起组件动画增强 */
.collapse-content {
  transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
  overflow: hidden;
}

.collapse:not(.collapse-open) .collapse-content {
  max-height: 0;
  padding-top: 0;
  padding-bottom: 0;
  opacity: 0;
}

.collapse.collapse-open .collapse-content,
.collapse-checkbox:checked ~ .collapse-content {
  max-height: 1000px; /* 调整为合适的最大高度 */
  opacity: 1;
}

/* 折叠面板标题悬停效果 */
.collapse-title {
  transition: all 0.2s ease;
}

.collapse:hover .collapse-title {
  background-color: hsl(var(--b2) / 0.5);
}

/* 执行中的步骤特殊动画 */
.collapse.border-primary {
  animation: pulse-border 2s infinite;
}

@keyframes pulse-border {
  0%, 100% {
    border-color: hsl(var(--p));
    box-shadow: 0 0 0 0 hsl(var(--p) / 0.4);
  }
  50% {
    border-color: hsl(var(--p) / 0.8);
    box-shadow: 0 0 0 4px hsl(var(--p) / 0.1);
  }
}

/* 步骤执行进度条动画 */
@keyframes progress-pulse {
  0% {
    opacity: 0.6;
    transform: scaleX(0.8);
  }
  50% {
    opacity: 1;
    transform: scaleX(1);
  }
  100% {
    opacity: 0.6;
    transform: scaleX(0.8);
  }
}

/* 按钮悬停效果 */
.btn {
  transition: all 0.2s ease;
}

.btn:hover:not(.btn-disabled) {
  transform: translateY(-1px);
}

/* 输入框焦点效果 */
.textarea:focus {
  box-shadow: 0 0 0 3px hsl(var(--p) / 0.1);
}

/* 渐变背景动画 */
.enhanced-ai-chat {
  background: linear-gradient(135deg, hsl(var(--b1)) 0%, hsl(var(--b2)) 100%);
  background-size: 200% 200%;
  animation: gradientShift 20s ease infinite;
}

@keyframes gradientShift {
  0% {
    background-position: 0% 50%;
  }
  50% {
    background-position: 100% 50%;
  }
  100% {
    background-position: 0% 50%;
  }
}

/* 性能优化 */
.chat-bubble,
.collapse,
.btn,
.textarea {
  will-change: transform, box-shadow;
}


/* 抽屉样式优化 */
.drawer-end .drawer-side {
  z-index: 1000;
}

.drawer-end .drawer-side .w-80 {
  box-shadow: -4px 0 15px rgba(0, 0, 0, 0.1);
  border-left: 1px solid hsl(var(--b3));
}

/* 模型搜索下拉样式 */
.dropdown.dropdown-open .dropdown-content {
  display: block;
  opacity: 1;
  transform: translateY(0);
  transition: all 0.2s ease;
}

.dropdown .dropdown-content {
  opacity: 0;
  transform: translateY(-10px);
  transition: all 0.2s ease;
}




.model-selector-input:read-only {
  cursor: pointer;
}

.model-selector-input:read-only:hover {
  background: hsl(var(--p) / 0.15);
  border-color: hsl(var(--p) / 0.5);
}

/* 移动端优化 */
@media (max-width: 768px) {
  .enhanced-ai-chat .chat-bubble {
    max-width: calc(100vw - 8rem);
    word-wrap: break-word;
    overflow-wrap: break-word;
  }
  
  .enhanced-ai-chat .collapse-title {
    font-size: 0.875rem;
  }
  
  .enhanced-ai-chat .stats {
    flex-direction: column;
  }
  
  .enhanced-ai-chat .grid {
    grid-template-columns: 1fr !important;
  }
  
  .enhanced-ai-chat .dropdown-content {
    max-width: calc(100vw - 3rem);
    left: auto !important;
    right: 0 !important;
  }
  
  /* 移动端架构和模型下拉列表优化 */
  .enhanced-ai-chat .dropdown-content.w-72,
  .enhanced-ai-chat .dropdown-content.w-80 {
    width: calc(100vw - 3rem) !important;
    max-width: 300px;
  }
  
  .enhanced-ai-chat .flex-wrap {
    flex-wrap: wrap;
  }
  
  .enhanced-ai-chat .select {
    max-width: 150px;
  }
  
  /* 移动端模型选择器优化 */
  .model-selector-input {
    min-width: 80px !important;
    max-width: 140px !important;
    font-size: 0.75rem;
  }
  
  .model-selector-input:focus {
    width: 120px !important;
  }
  
  .enhanced-ai-chat .btn {
    white-space: nowrap;
  }
  
  /* 移动端抽屉优化 */
  .drawer-end .drawer-side .w-80 {
    width: calc(100vw - 2rem) !important;
    max-width: 320px;
  }
}
</style>
