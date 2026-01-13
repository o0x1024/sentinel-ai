<template>
  <div class="ai-settings">
    <!-- 配置模式切换 -->
    <div class="flex justify-between items-center mb-6">
      <h2 class="text-2xl font-bold">{{ t('settings.ai.title') }}</h2>
      <div class="flex items-center gap-4">
        <div class="form-control">
          <label class="label cursor-pointer gap-2">
            <span class="label-text">{{ t('settings.ai.guiMode') }}</span>
            <input type="checkbox" class="toggle toggle-primary" v-model="useGuiMode" />
            <span class="label-text">{{ t('settings.ai.manualMode') }}</span>
          </label>
        </div>
        <button v-if="!useGuiMode" class="btn btn-primary btn-sm" @click="validateConfig">
          <i class="fas fa-check"></i>
          {{ t('settings.ai.validateConfig') }}
        </button>
      </div>
    </div>

    <!-- 手动编辑模式 -->
    <div v-if="!useGuiMode" class="card bg-base-100 shadow-sm mb-6">
      <div class="card-body">
        <h3 class="card-title mb-4">
          <i class="fas fa-code"></i>
          {{ t('settings.ai.manualEdit') }}
        </h3>

        <!-- JSON 编辑器 -->
        <div class="space-y-4">
          <div class="flex items-center gap-2 mb-2">
            <span class="text-sm text-base-content/70">
              {{ t('settings.ai.manualEditJson') }}
            </span>
            <div class="badge badge-warning badge-sm" v-if="configError">
              {{ t('settings.ai.configError') }}
            </div>
            <div class="badge badge-success badge-sm" v-else-if="configValid">
              {{ t('settings.ai.configValid') }}
            </div>
          </div>

          <div class="relative">
            <div ref="editorContainer" class="editor-container rounded-lg overflow-hidden border" :class="{
              'border-error': configError,
              'border-success': configValid && !configError,
              'border-base-300': !configError && !configValid
            }"></div>
            <button class="fullscreen-btn" @click="toggleFullscreen"
              :title="t('settings.ai.fullscreen')">
              <i class="fas fa-expand"></i>
            </button>
          </div>

          <div v-if="configError" class="alert alert-error">
            <i class="fas fa-exclamation-triangle"></i>
            <span>{{ configError }}</span>
          </div>

          <div class="flex gap-2">
            <button class="btn btn-primary" @click="applyManualConfig" :disabled="!!configError">
              <i class="fas fa-save"></i>
              {{ t('settings.ai.applyManualConfig') }}
            </button>
            <button class="btn btn-outline" @click="formatConfig">
              <i class="fas fa-indent"></i>
              {{ t('settings.ai.formatConfig') }}
            </button>
            <button class="btn btn-outline" @click="resetToDefault">
              <i class="fas fa-undo"></i>
              {{ t('settings.ai.resetToDefault') }}
            </button>
          </div>

    <!-- 全屏编辑器模态框 -->
    <div v-if="isFullscreen" class="fullscreen-editor-overlay">
      <div class="fullscreen-editor-container">
        <div class="fullscreen-editor-header">
          <h3 class="text-lg font-semibold flex items-center gap-2">
            <i class="fas fa-code"></i>
            {{ t('settings.ai.manualEdit') }}
          </h3>
          <div class="flex items-center gap-2">
            <div class="badge badge-warning badge-sm" v-if="configError">
              {{ t('settings.ai.configError') }}
            </div>
            <div class="badge badge-success badge-sm" v-else-if="configValid">
              {{ t('settings.ai.configValid') }}
            </div>
            <button class="btn btn-ghost btn-sm" @click="formatConfig">
              <i class="fas fa-indent"></i>
            </button>
            <button class="btn btn-ghost btn-sm" @click="exitFullscreen">
              <i class="fas fa-compress"></i>
            </button>
          </div>
        </div>
        <div ref="fullscreenEditorContainer" class="fullscreen-editor-content"></div>
        <div class="fullscreen-editor-footer">
          <div v-if="configError" class="text-error text-sm flex items-center gap-2">
            <i class="fas fa-exclamation-triangle"></i>
            <span>{{ configError }}</span>
          </div>
          <div class="flex-1"></div>
          <button class="btn btn-outline btn-sm" @click="exitFullscreen">
            {{ t('common.cancel') }}
          </button>
          <button class="btn btn-primary btn-sm" @click="applyAndExitFullscreen" :disabled="!!configError">
            <i class="fas fa-save"></i>
            {{ t('settings.ai.applyManualConfig') }}
          </button>
        </div>
      </div>
    </div>
        </div>
      </div>
    </div>

    <!-- 图形界面模式 -->
    <div v-if="useGuiMode">
      <!-- AI提供商状态总览 -->
      <div class="space-y-4 mb-6">
        <!-- 默认设置卡片 -->
        <div class="card bg-base-100 shadow-sm border">
          <div class="card-body p-4">
            <div class="flex items-center gap-4 mb-4">
              <i class="fas fa-cog text-primary text-xl"></i>
              <h3 class="font-semibold text-lg">{{ t('settings.ai.defaultConfig') }}</h3>
            </div>

            <!-- 默认Provider和模型选择器 -->
            <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
              <!-- 默认Provider选择器 -->
              <div class="space-y-2">
                <label class="label">
                  <span class="label-text font-medium flex items-center gap-2">
                    <i class="fas fa-star text-warning"></i>
                    {{ t('settings.ai.defaultProvider') }}
                  </span>
                </label>
                <SearchableSelect v-model="defaultProviderLocal" :options="providerOptions"
                  :placeholder="t('settings.ai.selectProvider', 'Select Provider')"
                  :search-placeholder="t('settings.ai.searchProvider')" @change="onChangeDefaultProvider" />
              </div>

              <!-- Default Chat Model Selector -->
              <div class="space-y-2">
                <label class="label">
                  <span class="label-text font-medium flex items-center gap-2">
                    <i class="fas fa-comment-dots text-primary"></i>
                    {{ t('settings.ai.defaultChatModel') }}
                    <span class="badge badge-sm badge-ghost">{{ t('settings.ai.fastModel') }}</span>
                  </span>
                </label>
                <EditableSelect v-model="defaultChatModelLocal" :options="chatModelOptions"
                  :placeholder="t('settings.ai.selectOrInputModel')" 
                  :custom-value-text="t('settings.ai.useCustomModel')"
                  :disabled="!defaultProviderLocal"
                  @change="onChangeDefaultChatModel" />
              </div>

              <!-- Default VLM Provider Selector -->
              <div class="space-y-2">
                <label class="label">
                  <span class="label-text font-medium flex items-center gap-2">
                    <i class="fas fa-sitemap text-secondary"></i>
                    {{ t('settings.ai.defaultVlmProvider') }}
                  </span>
                </label>
                <SearchableSelect v-model="defaultVlmProviderLocal" :options="providerOptions"
                  :placeholder="t('settings.ai.selectProvider')" :search-placeholder="t('settings.ai.searchProvider')"
                  @change="onChangeDefaultVlmProvider" />
              </div>

              <!-- Default VLM Model Selector -->
              <div class="space-y-2">
                <label class="label">
                  <span class="label-text font-medium flex items-center gap-2">
                    <i class="fas fa-eye text-accent"></i>
                    {{ t('settings.ai.defaultVlmModel') }}
                    <span class="badge badge-sm badge-ghost">{{ t('settings.ai.smartModel') }}</span>
                  </span>
                </label>
                <EditableSelect v-model="defaultVlmModelLocal" :options="vlmModelOptions"
                  :placeholder="t('settings.ai.selectOrInputModel')" 
                  :custom-value-text="t('settings.ai.useCustomModel')"
                  :disabled="!defaultVlmProviderLocal"
                  @change="onChangeDefaultVisionModel" />
                <label class="label">
                  <span class="label-text-alt text-base-content/60">
                    {{ t('settings.ai.visionModelDescription') }}
                  </span>
                </label>
              </div>
            </div>



            <!-- 提示信息 -->
            <div class="flex items-center gap-2 mt-3 text-sm text-base-content/70">
              <i class="fas fa-info-circle"></i>
              <span>{{ t('settings.ai.aiAssistantWillUseThisConfig') }}</span>
            </div>
          </div>
        </div>
        <div v-for="status in aiServiceStatus" :key="status.provider" class="card bg-base-100 shadow-sm border">
          <div class="card-body p-4">
            <div class="flex items-center gap-4">
              <div class="text-2xl">
                <i :class="getProviderIcon(status.provider)"></i>
              </div>
              <div class="flex-1">
                <h3 class="font-semibold text-lg">{{ getProviderName(status.provider) }}</h3>
                <div class="flex items-center gap-2 mt-1">
                  <div class="badge" :class="status.is_available ? 'badge-success' : 'badge-error'">
                    {{ status.is_available ? t('settings.ai.connected') : t('settings.ai.disconnected') }}
                  </div>
                  <span class="text-sm text-base-content/70">{{ status.models_loaded }} {{ t('settings.ai.modelsCount')
                  }}</span>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- AI提供商配置选项卡 - 垂直布局 -->
      <div class="flex flex-col lg:flex-row gap-6 mb-6">
        <!-- 左侧：提供商选择 -->
        <div class="w-full lg:w-64 flex-shrink-0">
          <h3 class="text-lg font-semibold mb-3">{{ t('settings.ai.aiProviders') }}</h3>
          <div class="menu bg-base-200 rounded-box p-2 space-y-1">
            <li v-for="provider in sortedProviderKeys" :key="provider">
              <a class="flex items-center gap-3 p-3 rounded-lg transition-all duration-200"
                :class="{ 'bg-primary text-primary-content': selectedAiProvider === provider }"
                @click="selectedAiProvider = provider">
                <div class="text-xl">
                  <i :class="getProviderIcon(provider)"></i>
                </div>
                <span class="font-medium">{{ getProviderName(provider) }}</span>
              </a>
            </li>
          </div>
        </div>

        <!-- 右侧：配置内容 -->
        <div class="flex-1">

          <!-- 当前选中的AI提供商配置 -->
          <div v-if="selectedProviderConfig" class="grid grid-cols-1 xl:grid-cols-2 gap-6">
            <!-- 左侧：基本配置 -->
            <div class="space-y-4">
              <h3 class="text-lg font-semibold border-b pb-2">{{ t('settings.ai.basicConfig') }}</h3>

              <!-- 启用/禁用 -->
              <div class="form-control">
                <label class="label cursor-pointer">
                  <span class="label-text">{{ t('settings.ai.enable') }} {{ getProviderName(selectedAiProvider)
                  }}</span>
                  <input type="checkbox" class="toggle toggle-primary" v-model="selectedProviderConfig.enabled"
                    @change="saveAiConfig">
                </label>
              </div>

              <!-- Rig 提供商类型 -->
              <div class="form-control">
                <label class="label">
                  <span class="label-text">{{ t('settings.ai.rigProvider') }}</span>
                  <span class="label-text-alt text-info">{{ t('settings.ai.rigProviderDescription') }}</span>
                </label>
                <SearchableSelect v-model="rigProviderLocal" :options="rigProviderOptions"
                  :placeholder="t('settings.ai.selectProviderType')"
                  :search-placeholder="t('settings.ai.searchProviderType')" @change="saveAiConfig" />
                <label class="label">
                  <span class="label-text-alt">{{ t('settings.ai.decideBackendApiFormat') }}</span>
                </label>
              </div>

              <!-- API密钥配置 -->
              <div class="form-control" v-if="needsApiKey(selectedAiProvider)">
                <label class="label">
                  <span class="label-text">{{ t('settings.ai.apiKey') }}</span>
                </label>
                <div class="input-group">
                  <input type="password" :placeholder="t('settings.apiKeyPlaceholder')"
                    class="input input-bordered flex-1" v-model="selectedProviderConfig.api_key" @blur="saveAiConfig">
                  <button class="btn btn-outline" @click="testConnection(selectedAiProvider)">
                    <i class="fas fa-plug"></i>
                    {{ t('settings.testConnection') }}
                  </button>
                  <button class="btn btn-outline" @click="refreshModels(selectedAiProvider)">
                    <i class="fas fa-sync-alt"></i>
                    {{ t('settings.ai.refreshModels') }}
                  </button>
                </div>
              </div>

              <!-- API Base URL -->
              <div class="form-control" >
                <label class="label">
                  <span class="label-text">{{ t('settings.ai.apiBaseUrl') }}</span>
                </label>
                <div class="input-group">
                  <input type="url" :placeholder="t('settings.ai.apiBaseUrl')" class="input input-bordered" style="width:300px"
                    v-model="selectedProviderConfig.api_base" @blur="saveAiConfig">
                  <!-- 为Ollama等不需要API密钥但需要测试连接的提供商添加按钮 -->
                  <button v-if="!needsApiKey(selectedAiProvider)" class="btn btn-outline"
                    @click="testConnection(selectedAiProvider)">
                    <i class="fas fa-plug"></i>
                    {{ t('settings.testConnection') }}
                  </button>
                  <button v-if="!needsApiKey(selectedAiProvider)" class="btn btn-outline"
                    @click="refreshModels(selectedAiProvider)">
                    <i class="fas fa-sync-alt"></i>
                    {{ t('settings.ai.refreshModels') }}
                  </button>
                </div>
              </div>

              <!-- 组织ID (OpenAI) -->
              <div class="form-control"
                v-if="selectedAiProvider === 'OpenAI' && selectedProviderConfig && 'organization' in selectedProviderConfig">
                <label class="label">
                  <span class="label-text">{{ t('settings.ai.organizationId') }}</span>
                </label>
                <input type="text" :placeholder="t('settings.ai.organizationId')" class="input input-bordered"
                  v-model="(selectedProviderConfig as any).organization" @blur="saveAiConfig">
              </div>

              <!-- OpenRouter特定配置 -->
              <div v-if="selectedAiProvider === 'OpenRouter'" class="space-y-4">
                <!-- HTTP Referer -->
                <div class="form-control">
                  <label class="label">
                    <span class="label-text">{{ t('settings.ai.httpReferer') }}</span>
                  </label>
                  <input type="url" placeholder="https://yoursite.com" class="input input-bordered"
                    v-model="selectedProviderConfig.http_referer" @blur="saveAiConfig">
                  <label class="label">
                    <span class="label-text-alt">{{ t('settings.ai.httpRefererDescription') }}</span>
                  </label>
                </div>

                <!-- X-Title -->
                <div class="form-control">
                  <label class="label">
                    <span class="label-text">{{ t('settings.ai.appName') }}</span>
                  </label>
                  <input type="text" :placeholder="t('settings.ai.appNamePlaceholder')" class="input input-bordered"
                    v-model="selectedProviderConfig.x_title" @blur="saveAiConfig">
                  <label class="label">
                    <span class="label-text-alt">{{ t('settings.ai.appNameDescription') }}</span>
                  </label>
                </div>
              </div>

              <!-- 默认模型选择 -->
              <div class="form-control">
                <label class="label">
                  <span class="label-text">{{ t('settings.ai.defaultModel') }}</span>
                </label>
                <EditableSelect v-model="selectedProviderDefaultModel" :options="selectedProviderModelOptions"
                  :placeholder="t('settings.ai.selectOrInputModel')" 
                  :custom-value-text="t('settings.ai.useCustomModel')"
                  :allow-custom="true"
                  @change="onSelectedProviderModelChange" />
                <label class="label">
                  <span class="label-text-alt text-base-content/60">
                    {{ t('settings.ai.canInputCustomModel') }}
                  </span>
                </label>
              </div>

              <!-- 最大上下文长度设置 -->
              <div class="form-control">
                <label class="label">
                  <span class="label-text">{{ t('settings.ai.maxContextLength') }}</span>
                </label>
                <input 
                  type="number" 
                  class="input input-bordered" 
                  v-model.number="selectedProviderMaxContextLength"
                  @blur="saveAiConfig"
                  min="1000"
                  max="1000000"
                  step="1000"
                  placeholder="128000"
                />
                <label class="label">
                  <span class="label-text-alt text-base-content/60">
                    {{ t('settings.ai.maxContextLengthHint') }}
                  </span>
                </label>
              </div>

              <!-- 自定义请求头 -->
              <div class="form-control">
                <label class="label">
                  <span class="label-text flex items-center gap-2">
                    {{ t('settings.ai.customHeaders') }}
                    <span class="badge badge-sm badge-info">{{ t('settings.ai.optional') }}</span>
                  </span>
                </label>
                <textarea 
                  class="textarea textarea-bordered font-mono text-sm h-24"
                  :placeholder="t('settings.ai.customHeadersPlaceholder')"
                  v-model="customHeadersJson"
                  @blur="saveCustomHeaders"
                ></textarea>
                <label class="label">
                  <span class="label-text-alt text-base-content/60">
                    {{ t('settings.ai.customHeadersDescription') }}
                  </span>
                </label>
                <div v-if="customHeadersError" class="alert alert-error mt-2">
                  <i class="fas fa-exclamation-triangle"></i>
                  <span>{{ customHeadersError }}</span>
                </div>
              </div>
            </div>

            <!-- 右侧：高级配置 -->
            <div class="space-y-4">
              <h3 class="text-lg font-semibold border-b pb-2">{{ t('settings.ai.advancedConfig') }}</h3>

              <!-- 温度设置 -->
              <div class="form-control">
                <label class="label">
                  <span class="label-text">{{ t('settings.ai.temperature') }}</span>
                </label>
                <div class="flex items-center gap-4">
                  <input v-model.number="settings.ai.temperature" type="range" min="0" max="1" step="0.1"
                    class="range range-primary flex-1" @change="saveAiConfig" />
                  <span class="text-sm min-w-[60px]">{{ settings.ai.temperature }}</span>
                </div>
                <label class="label">
                  <span class="label-text-alt">{{ t('settings.ai.temperatureHint') }}</span>
                </label>
              </div>

              <!-- 最大Token设置 -->
              <div class="form-control">
                <label class="label">
                  <span class="label-text">{{ t('settings.ai.maxTokens') }} (Max Generation)</span>
                </label>
                <div class="flex items-center gap-4">
                  <input v-model.number="settings.ai.maxTokens" type="range" min="500" max="8000" step="500"
                    class="range range-primary flex-1" @change="saveAiConfig" />
                  <span class="text-sm min-w-[60px]">{{ settings.ai.maxTokens }}</span>
                </div>
                <label class="label">
                  <span class="label-text-alt">{{ t('settings.ai.maxTokensHint') }}</span>
                </label>
              </div>

              <!-- 工具输出限制设置 -->
              <!-- 输出存储阈值 -->
              <div class="form-control">
                <label class="label">
                  <span class="label-text">{{ t('settings.ai.outputStorageThreshold') }}</span>
                  <span class="badge badge-info badge-sm">Dynamic Context</span>
                </label>
                <div class="flex items-center gap-4">
                  <input v-model.number="settings.ai.outputStorageThreshold" type="range" min="5000" max="200000" step="5000"
                    class="range range-info flex-1" @change="saveAiConfig" />
                  <span class="text-sm min-w-[60px]">{{ (settings.ai.outputStorageThreshold || 10000) / 1000 }}K</span>
                </div>
                <label class="label">
                  <span class="label-text-alt">{{ t('settings.ai.outputStorageThresholdHint') }}</span>
                </label>
              </div>

              <!-- 最大对话轮数设置 -->
              <div class="form-control">
                <label class="label">
                  <span class="label-text">{{ t('settings.ai.maxTurns') }}</span>
                </label>
                <div class="flex items-center gap-4">
                  <input v-model.number="settings.ai.maxTurns" type="range" min="10" max="200" step="10"
                    class="range range-accent flex-1" @change="saveAiConfig" />
                  <span class="text-sm min-w-[60px]">{{ settings.ai.maxTurns || 100 }}</span>
                </div>
                <label class="label">
                  <span class="label-text-alt">{{ t('settings.ai.maxTurnsHint') }}</span>
                </label>
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- AI 工作目录设置 -->
      <div class="card bg-base-100 shadow-sm mt-6">
        <div class="card-body p-4">
          <div class="flex items-center gap-3 mb-2">
            <i class="fas fa-folder text-primary text-lg"></i>
            <h3 class="font-semibold">{{ t('settings.ai.workingDirectory') }}</h3>
          </div>
          <div class="form-control">
            <label class="label">
              <span class="label-text">{{ t('settings.ai.workingDirectory') }}</span>
            </label>
            <div class="input-group">
              <input 
                v-model="workingDirectoryLocal" 
                type="text" 
                class="input input-bordered flex-1" 
                :placeholder="t('settings.ai.workingDirectoryPlaceholder')" 
              />
              <button class="btn btn-outline" @click="selectWorkingDirectory">
                <i class="fas fa-folder-open mr-1"></i>
                {{ t('settings.ai.selectDirectory') }}
              </button>
            </div>
            <label class="label">
              <span class="label-text-alt">{{ t('settings.ai.workingDirectoryHint') }}</span>
            </label>
          </div>
          <div class="flex justify-end mt-3">
            <button class="btn btn-primary btn-sm" @click="saveAiConfig">
              <i class="fas fa-save mr-1"></i>
              {{ t('settings.ai.save') }}
            </button>
          </div>
        </div>
      </div>

      <!-- Tavily Search 设置 -->
      <div class="card bg-base-100 shadow-sm mt-6">
        <div class="card-body p-4">
          <div class="flex items-center gap-3 mb-2">
            <i class="fas fa-search text-primary text-lg"></i>
            <h3 class="font-semibold">Tavily Search</h3>
          </div>
          <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
            <div class="form-control">
              <label class="label">
                <span class="label-text">Tavily API Key</span>
              </label>
              <input v-model="tavilyApiKeyLocal" type="password" class="input input-bordered" placeholder="tvly-..." />
              <label class="label">
                <span class="label-text-alt">{{ t('settings.ai.tavilyApiKeyDescription') }}</span>
              </label>
            </div>
            <div class="form-control">
              <label class="label">
                <span class="label-text">{{ t('settings.ai.defaultMaxResults') }}</span>
              </label>
              <input v-model.number="tavilyMaxResultsLocal" type="number" min="1" max="20"
                class="input input-bordered w-40" />
            </div>
          </div>
          <div class="flex justify-end mt-3">
            <button class="btn btn-primary btn-sm" @click="saveAiConfig">
              <i class="fas fa-save mr-1"></i>
              {{ t('settings.ai.save') }}
            </button>
          </div>
        </div>
      </div>

      <!-- 阿里云 OSS 配置（用于 DashScope 文件上传） -->
      <div class="card bg-base-100 shadow-sm mt-6">
        <div class="card-body p-4">
          <div class="flex items-center gap-3 mb-2">
            <i class="fas fa-cloud-upload-alt text-primary text-lg"></i>
            <h3 class="font-semibold">{{ t('settings.ai.aliyunDashScope') }}</h3>
          </div>
          <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
            <div class="form-control">
              <label class="label">
                <span class="label-text">{{ t('settings.ai.dashscopeApiKey') }}</span>
              </label>
              <input v-model="aliyunApiKeyLocal" type="password" class="input input-bordered" placeholder="sk-..." />
              <label class="label">
                <span class="label-text-alt">{{ t('settings.ai.dashscopeApiKeyDescription') }}</span>
              </label>
            </div>
            <div class="form-control">
              <label class="label">
                <span class="label-text">{{ t('settings.ai.defaultModel') }}</span>
              </label>
              <input v-model="aliyunDefaultModelLocal" type="text" class="input input-bordered"
                placeholder="qwen-vl-plus" />
              <label class="label">
                <span class="label-text-alt">{{ t('settings.ai.dashscopeDefaultModelDescription') }}</span>
              </label>
            </div>
          </div>
          <div class="flex justify-end mt-3 gap-2">
            <button class="btn btn-outline btn-sm" @click="testAliyunConnection" :disabled="testingAliyun">
              <span v-if="testingAliyun" class="loading loading-spinner loading-sm"></span>
              <i v-else class="fas fa-plug mr-1"></i>
              {{ t('settings.ai.testConnection') }}
            </button>
            <button class="btn btn-primary btn-sm" @click="saveAiConfig">
              <i class="fas fa-save mr-1"></i>
              {{ t('settings.ai.save') }}
            </button>
          </div>
        </div>
      </div>

      <!-- 可用模型列表 - 重构为卡片布局 -->
      <div class="mt-6">
        <h3 class="text-lg font-semibold border-b pb-2 mb-4">
          {{ t('settings.ai.availableModels') }} ({{ selectedProviderConfig?.models?.length || 0 }})
        </h3>

        <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
          <div v-for="model in selectedProviderConfig?.models" :key="model.id"
            class="card bg-base-100 shadow-sm border">
            <div class="card-body p-4">
              <div class="flex justify-between items-start mb-2">
                <h4 class="card-title text-sm">{{ model.name }}</h4>
                <div :class="model.is_available ? 'badge badge-success badge-sm' : 'badge badge-error badge-sm'">
                  {{ model.is_available ? t('settings.ai.available') : t('settings.ai.unavailable') }}
                </div>
              </div>

              <p class="text-xs text-base-content/70 mb-3">{{ model.description }}</p>

              <div class="space-y-2">
                <div class="flex justify-between text-xs">
                  <span class="text-base-content/60">{{ t('settings.ai.contextLength') }}:</span>
                  <span>{{ model.context_length?.toLocaleString() || 'N/A' }}</span>
                </div>

                <div class="flex flex-wrap gap-1">
                  <div v-if="model.supports_streaming" class="badge badge-primary badge-xs">
                    {{ t('settings.ai.streaming') }}
                  </div>
                  <div v-if="model.supports_tools" class="badge badge-secondary badge-xs">
                    {{ t('settings.ai.tools') }}
                  </div>
                  <div v-if="model.supports_vision" class="badge badge-accent badge-xs">
                    {{ t('settings.ai.vision') }}
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- 自定义AI提供商 -->
      <div class="card bg-base-100 shadow-sm mt-6">
        <div class="card-body">
          <h3 class="card-title">
            <i class="fas fa-plus-circle"></i>
            {{ t('settings.ai.customProvider') }}
          </h3>

          <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
            <!-- 提供商唯一名称（ID） -->
            <div class="form-control">
              <label class="label">
                <span class="label-text">{{ t('settings.ai.providerName') }}</span>
                <span class="label-text-alt text-warning">{{ t('settings.ai.providerNameDescription') }}</span>
              </label>
              <input type="text" placeholder="MyCustomProvider" class="input input-bordered"
                v-model="customProvider.name">
            </div>

            <!-- Rig 提供商选择 -->
            <div class="form-control">
              <label class="label">
                <span class="label-text">{{ t('settings.ai.rigProvider') }}</span>
                <span class="label-text-alt text-warning">{{ t('settings.ai.rigProviderDescription') }}</span>
              </label>
              <SearchableSelect v-model="customProvider.rig_provider" :options="rigProviderOptions"
                :placeholder="t('settings.ai.rigProviderPlaceholder')"
                :search-placeholder="t('settings.ai.rigProviderSearchPlaceholder')" />
              <label class="label">
                <span class="label-text-alt">
                  {{ t('settings.ai.rigProviderDescription') }}
                </span>
              </label>
            </div>

            <!-- API密钥 -->
            <div class="form-control">
              <label class="label">
                <span class="label-text">{{ t('settings.ai.apiKey') }}</span>
                <span class="label-text-alt text-warning" v-if="customProvider.compat_mode !== 'ollama'">{{
                  t('settings.ai.apiKeyDescription') }}</span>
              </label>
              <input type="password" :placeholder="t('settings.apiKeyPlaceholder')" class="input input-bordered"
                v-model="customProvider.api_key">
            </div>

            <!-- API Base URL -->
            <div class="form-control">
              <label class="label">
                <span class="label-text">{{ t('settings.ai.apiBaseUrl') }}</span>
                <span class="label-text-alt text-warning" v-if="customProvider.compat_mode !== 'ollama'">{{
                  t('settings.ai.apiBaseUrlDescription') }}</span>
              </label>
              <input type="url" placeholder="https://api.example.com/v1" class="input input-bordered"
                v-model="customProvider.api_base">
              <label class="label">
                <span class="label-text-alt">
                  {{ t('settings.ai.apiBaseUrlExample') }}
                </span>
              </label>
            </div>

            <!-- 模型ID -->
            <div class="form-control">
              <label class="label">
                <span class="label-text">{{ t('settings.ai.modelId') }}</span>
                <span class="label-text-alt text-warning" v-if="customProvider.compat_mode !== 'ollama'">{{
                  t('settings.ai.modelIdDescription') }}</span>
              </label>
              <input type="text" placeholder="gpt-4o-mini" class="input input-bordered"
                v-model="customProvider.model_id">
            </div>

            <!-- 显示名称 -->
            <div class="form-control">
              <label class="label">
                <span class="label-text">{{ t('settings.ai.displayName') }}</span>
                <span class="label-text-alt">{{ t('settings.ai.displayNameDescription') }}</span>
              </label>
              <input type="text" :placeholder="t('settings.ai.displayNamePlaceholder')" class="input input-bordered"
                v-model="customProvider.display_name">
            </div>
          </div>

          <!-- 高级选项折叠 -->
          <div class="collapse collapse-arrow bg-base-200 mt-4">
            <input type="checkbox" />
            <div class="collapse-title font-medium">
              <i class="fas fa-cogs mr-2"></i>{{ t('settings.ai.advancedOptions') }}
            </div>
            <div class="collapse-content">
              <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
                <!-- 额外请求头 -->
                <div class="form-control md:col-span-2">
                  <label class="label">
                    <span class="label-text">{{ t('settings.ai.extraHeaders') }}</span>
                  </label>
                  <textarea class="textarea textarea-bordered font-mono text-sm h-24"
                    placeholder='{"X-Custom-Header": "value"}' v-model="customProvider.extra_headers_json"></textarea>
                </div>

                <!-- 超时设置 -->
                <div class="form-control">
                  <label class="label">
                    <span class="label-text">{{ t('settings.ai.timeout') }}</span>
                  </label>
                  <input type="number" class="input input-bordered" v-model.number="customProvider.timeout" min="10"
                    max="600" placeholder="120">
                </div>

                <!-- 最大重试次数 -->
                <div class="form-control">
                  <label class="label">
                    <span class="label-text">{{ t('settings.ai.maxRetries') }}</span>
                  </label>
                  <input type="number" class="input input-bordered" v-model.number="customProvider.max_retries" min="0"
                    max="5" placeholder="3">
                </div>
              </div>
            </div>
          </div>

          <!-- 验证提示 -->
          <div v-if="customProviderValidationError" class="alert alert-error mt-4">
            <i class="fas fa-exclamation-triangle"></i>
            <span>{{ customProviderValidationError }}</span>
          </div>

          <div class="card-actions justify-end mt-4">
            <button class="btn btn-outline" @click="testCustomProvider"
              :disabled="!!customProviderValidationError || testingCustomProvider">
              <span v-if="testingCustomProvider" class="loading loading-spinner loading-sm"></span>
              <i v-else class="fas fa-vial"></i>
              {{ t('settings.ai.testCustomProvider') }}
            </button>
            <button class="btn btn-primary" @click="addCustomProvider"
              :disabled="!!customProviderValidationError || addingCustomProvider">
              <span v-if="addingCustomProvider" class="loading loading-spinner loading-sm"></span>
              <i v-else class="fas fa-plus"></i>
              {{ t('settings.ai.addCustomProvider') }}
            </button>
          </div>
        </div>
      </div>

      <!-- AI使用统计 -->
      <div class="card bg-base-100 shadow-sm mt-6">
        <div class="card-body">
          <div class="flex justify-between items-center mb-4">
            <h3 class="card-title">
              <i class="fas fa-chart-bar"></i>
              {{ t('settings.ai.usageStats') }}
            </h3>
            <div class="flex gap-2">
              <button class="btn btn-ghost btn-sm" @click="loadDetailedStats">
                <i class="fas fa-sync-alt mr-1"></i>
                {{ t('settings.ai.refresh') }}
              </button>
              <button class="btn btn-ghost btn-sm text-error" @click="clearUsageStats">
                <i class="fas fa-trash-alt mr-1"></i>
                {{ t('settings.ai.clearStats', 'Clear Stats') }}
              </button>
            </div>
          </div>

          <!-- 统计概览 -->
          <div class="grid grid-cols-1 md:grid-cols-4 gap-4 mb-6">
            <div class="stat bg-base-200 rounded-lg">
              <div class="stat-title">{{ t('settings.ai.totalRequests') }}</div>
              <div class="stat-value text-primary">{{ totalRequests }}</div>
            </div>
            <div class="stat bg-base-200 rounded-lg">
              <div class="stat-title">{{ t('settings.ai.totalTokens') }}</div>
              <div class="stat-value text-secondary">{{ totalTokensFormatted }}</div>
            </div>
            <div class="stat bg-base-200 rounded-lg">
              <div class="stat-title">{{ t('settings.ai.totalCost') }}</div>
              <div class="stat-value text-accent">${{ totalCost.toFixed(2) }}</div>
            </div>
            <div class="stat bg-base-200 rounded-lg">
              <div class="stat-title">{{ t('settings.ai.avgCostPerRequest') }}</div>
              <div class="stat-value text-info">${{ avgCostPerRequest }}</div>
            </div>
          </div>

          <!-- 切换视图：按提供商 / 按模型 -->
          <div class="tabs tabs-boxed mb-4">
            <a class="tab" :class="{ 'tab-active': statsView === 'provider' }" @click="statsView = 'provider'">
              <i class="fas fa-server mr-2"></i>
              {{ t('settings.ai.byProvider') }}
            </a>
            <a class="tab" :class="{ 'tab-active': statsView === 'model' }" @click="statsView = 'model'">
              <i class="fas fa-brain mr-2"></i>
              {{ t('settings.ai.byModel') }}
            </a>
          </div>

          <!-- 按提供商统计 -->
          <div v-if="statsView === 'provider'" class="overflow-x-auto">
            <table class="table table-compact w-full">
              <thead>
                <tr>
                  <th>{{ t('settings.providers') }}</th>
                  <th class="text-right">{{ t('settings.ai.inputTokens') }}</th>
                  <th class="text-right">{{ t('settings.ai.outputTokens') }}</th>
                  <th class="text-right">{{ t('settings.ai.totalTokens') }}</th>
                  <th class="text-right">{{ t('settings.ai.estimatedCost') }}</th>
                  <th class="text-right">{{ t('settings.ai.percentage') }}</th>
                </tr>
              </thead>
              <tbody>
                <tr v-for="(usage, provider) in aiUsageStats" :key="provider" class="hover">
                  <td>
                    <div class="flex items-center gap-2">
                      <i :class="getProviderIcon(String(provider))"></i>
                      <span class="font-semibold">{{ getProviderName(String(provider)) }}</span>
                    </div>
                  </td>
                  <td class="text-right">{{ usage.input_tokens?.toLocaleString() }}</td>
                  <td class="text-right">{{ usage.output_tokens?.toLocaleString() }}</td>
                  <td class="text-right font-semibold">{{ usage.total_tokens?.toLocaleString() }}</td>
                  <td class="text-right">${{ (usage.cost || 0).toFixed(4) }}</td>
                  <td class="text-right">
                    <div class="flex items-center gap-2 justify-end">
                      <progress class="progress progress-primary w-20" :value="usage.total_tokens" :max="maxTokens"></progress>
                      <span class="text-sm">{{ ((usage.total_tokens / maxTokens) * 100).toFixed(1) }}%</span>
                    </div>
                  </td>
                </tr>
              </tbody>
              <tfoot v-if="Object.keys(aiUsageStats).length > 0">
                <tr class="font-bold">
                  <td>{{ t('settings.ai.total') }}</td>
                  <td class="text-right">{{ totalInputTokens.toLocaleString() }}</td>
                  <td class="text-right">{{ totalOutputTokens.toLocaleString() }}</td>
                  <td class="text-right">{{ (totalInputTokens + totalOutputTokens).toLocaleString() }}</td>
                  <td class="text-right">${{ totalCost.toFixed(4) }}</td>
                  <td></td>
                </tr>
              </tfoot>
            </table>
          </div>

          <!-- 按模型统计 -->
          <div v-if="statsView === 'model'" class="overflow-x-auto">
            <table class="table table-compact w-full">
              <thead>
                <tr>
                  <th>{{ t('settings.ai.provider') }}</th>
                  <th>{{ t('settings.ai.model') }}</th>
                  <th class="text-right">{{ t('settings.ai.inputTokens') }}</th>
                  <th class="text-right">{{ t('settings.ai.outputTokens') }}</th>
                  <th class="text-right">{{ t('settings.ai.totalTokens') }}</th>
                  <th class="text-right">{{ t('settings.ai.estimatedCost') }}</th>
                  <th class="text-right">{{ t('settings.ai.lastUsed') }}</th>
                </tr>
              </thead>
              <tbody>
                <tr v-for="stat in detailedStats" :key="`${stat.provider}-${stat.model}`" class="hover">
                  <td>
                    <div class="flex items-center gap-2">
                      <i :class="getProviderIcon(stat.provider)"></i>
                      <span>{{ getProviderName(stat.provider) }}</span>
                    </div>
                  </td>
                  <td>
                    <span class="badge badge-ghost">{{ stat.model }}</span>
                  </td>
                  <td class="text-right">{{ stat.input_tokens?.toLocaleString() }}</td>
                  <td class="text-right">{{ stat.output_tokens?.toLocaleString() }}</td>
                  <td class="text-right font-semibold">{{ stat.total_tokens?.toLocaleString() }}</td>
                  <td class="text-right">${{ (stat.cost || 0).toFixed(4) }}</td>
                  <td class="text-right text-sm">{{ formatLastUsed(stat.last_used) }}</td>
                </tr>
              </tbody>
            </table>
          </div>

          <!-- 空状态 -->
          <div v-if="Object.keys(aiUsageStats).length === 0 && detailedStats.length === 0" class="text-center py-8 text-base-content/60">
            <i class="fas fa-chart-line text-4xl mb-4"></i>
            <p>{{ t('settings.ai.noUsageData') }}</p>
          </div>
        </div>
      </div>

    </div>

  </div>
</template>

<script setup lang="ts">
import { computed, ref, watch, onMounted, onUnmounted, nextTick } from 'vue'
import { useI18n } from 'vue-i18n'
import { invoke } from '@tauri-apps/api/core'
import SearchableSelect from '@/components/SearchableSelect.vue'
import EditableSelect from '@/components/EditableSelect.vue'
import { EditorView, basicSetup } from 'codemirror'
import { EditorState } from '@codemirror/state'
import { json } from '@codemirror/lang-json'
import { oneDark } from '@codemirror/theme-one-dark'
import { keymap } from '@codemirror/view'
import { defaultKeymap, indentWithTab } from '@codemirror/commands'

const { t } = useI18n()

// 手动编辑模式相关状态
const useGuiMode = ref(true)
const manualConfigText = ref('')
const configError = ref('')
const configValid = ref(false)

// 统计视图状态
const statsView = ref<'provider' | 'model'>('provider')
const detailedStats = ref<any[]>([])

// 加载详细统计
const loadDetailedStats = async () => {
  try {
    const stats = await invoke('get_detailed_ai_usage_stats') as any[]
    detailedStats.value = stats || []
  } catch (e) {
    console.warn('Failed to load detailed AI usage stats', e)
  }
}

// CodeMirror 相关
const editorContainer = ref<HTMLDivElement | null>(null)
const fullscreenEditorContainer = ref<HTMLDivElement | null>(null)
let editorView: EditorView | null = null
let fullscreenEditorView: EditorView | null = null
const isFullscreen = ref(false)

// Props
interface Props {
  aiServiceStatus: any[]
  aiConfig: any
  selectedAiProvider: string
  settings: any
  customProvider: any
  aiUsageStats: any
  saving: boolean
  testingCustomProvider?: boolean
  addingCustomProvider?: boolean
}

const props = withDefaults(defineProps<Props>(), {
  testingCustomProvider: false,
  addingCustomProvider: false,
})

// Emits
interface Emits {
  'update:selectedAiProvider': [value: string]
  'update:settings': [value: any]
  'update:customProvider': [value: any]
  'update:aiConfig': [value: any]
  'testConnection': [provider: string]
  'testCustomProvider': []
  'addCustomProvider': []
  'saveAiConfig': []
  'refreshModels': [provider: string]
  'applyManualConfig': [config: any]
  'setDefaultProvider': [provider: string]
  'setDefaultChatModel': [model: string]
  'setDefaultVisionModel': [model: string]
  'setDefaultVlmProvider': [provider: string]
  'clearUsageStats': []
}

const emit = defineEmits<Emits>()

// Computed
const selectedAiProvider = computed({
  get: () => props.selectedAiProvider,
  set: (value: string) => {
    emit('update:selectedAiProvider', value)
  }
})

const sortedProviderKeys = computed(() => {
  const providers = props.aiConfig?.providers
  if (!providers || typeof providers !== 'object') return []
  return Object.keys(providers).sort((a, b) =>
    a.localeCompare(b, 'en', { sensitivity: 'base', numeric: true })
  )
})

const settings = computed({
  get: () => props.settings ?? { ai: { temperature: 0.7, maxTokens: 2000 } },
  set: (value) => emit('update:settings', value)
})

const customProvider = computed({
  get: () => props.customProvider,
  set: (value) => emit('update:customProvider', value)
})

// 是否正在测试/添加自定义提供商
const testingCustomProvider = computed(() => props.testingCustomProvider)
const addingCustomProvider = computed(() => props.addingCustomProvider)

// 自定义提供商验证错误
const customProviderValidationError = computed(() => {
  const p = props.customProvider
  if (!p.name || !p.name.trim()) {
    return t('settings.ai.providerNameDescription')
  }
  // 检查名称是否与现有提供商冲突
  const existingProviders = Object.keys(props.aiConfig?.providers || {})
  const nameLower = p.name.trim().toLowerCase()
  if (existingProviders.some(k => k.toLowerCase() === nameLower)) {
    return `提供商名称 "${p.name}" 已存在，请使用其他名称`
  }
  if (!p.rig_provider || !p.rig_provider.trim()) {
    return '请选择 Rig 提供商类型'
  }
  if (!p.api_base || !p.api_base.trim()) {
    return '请输入 API Base URL'
  }
  if (!p.model_id || !p.model_id.trim()) {
    return '请输入默认模型 ID'
  }
  // Ollama 不需要 API Key
  const noApiKeyProviders = ['ollama']
  if (!noApiKeyProviders.includes(p.rig_provider) && (!p.api_key || !p.api_key.trim())) {
    return '请输入 API Key'
  }
  // 验证 extra_headers_json 是否为有效 JSON
  if (p.extra_headers_json && p.extra_headers_json.trim()) {
    try {
      JSON.parse(p.extra_headers_json)
    } catch {
      return '额外请求头 JSON 格式无效'
    }
  }
  return ''
})

const selectedProviderConfig = computed(() => {
  return props.aiConfig.providers[props.selectedAiProvider]
})

const rigProviderLocal = computed({
  get: () => selectedProviderConfig.value?.rig_provider || '',
  set: (value: string) => {
    const providerKey = selectedAiProvider.value
    if (providerKey && props.aiConfig.providers && props.aiConfig.providers[providerKey]) {
      // 直接修改对象属性，Vue 3 的响应式系统会检测到变化
      props.aiConfig.providers[providerKey].rig_provider = value
    }
  }
})

// 自定义请求头
const customHeadersJson = ref('')
const customHeadersError = ref('')

// 加载自定义 headers
const loadCustomHeaders = () => {
  customHeadersError.value = ''
  const provider = selectedProviderConfig.value
  if (provider && provider.extra_headers) {
    try {
      customHeadersJson.value = JSON.stringify(provider.extra_headers, null, 2)
    } catch (e) {
      customHeadersJson.value = ''
    }
  } else {
    customHeadersJson.value = ''
  }
}

// 保存自定义 headers
const saveCustomHeaders = () => {
  customHeadersError.value = ''
  const providerKey = selectedAiProvider.value
  
  if (!providerKey || !props.aiConfig.providers || !props.aiConfig.providers[providerKey]) {
    return
  }

  // 如果为空，清除 extra_headers
  if (!customHeadersJson.value.trim()) {
    props.aiConfig.providers[providerKey].extra_headers = undefined
    saveAiConfig()
    return
  }

  // 验证 JSON 格式
  try {
    const headers = JSON.parse(customHeadersJson.value)
    
    // 验证是否为对象
    if (typeof headers !== 'object' || headers === null || Array.isArray(headers)) {
      customHeadersError.value = t('settings.ai.customHeadersMustBeObject')
      return
    }

    // 验证所有值都是字符串
    for (const [key, value] of Object.entries(headers)) {
      if (typeof value !== 'string') {
        customHeadersError.value = `${t('settings.ai.customHeadersValueMustBeString').replace('{key}', key)}`
        return
      }
    }

    // 保存到配置
    props.aiConfig.providers[providerKey].extra_headers = headers
    saveAiConfig()
    
  } catch (e) {
    customHeadersError.value = t('settings.ai.invalidJsonFormat') + ': ' + (e as Error).message
  }
}

// 监听选中的提供商变化，加载其自定义 headers
watch(() => props.selectedAiProvider, () => {
  loadCustomHeaders()
}, { immediate: true })

// 默认 Provider 选择
const defaultProviderLocal = ref('')
// 默认 Chat 模型选择
const defaultChatModelLocal = ref('')
// 默认 VLM Provider 选择
const defaultVlmProviderLocal = ref('')
// 默认 VLM 模型选择
const defaultVlmModelLocal = ref('')

watch(() => props.aiConfig, (cfg: any) => {

  const dp = (cfg && (cfg as any).default_llm_provider) || 'modelscope'
  // 查找匹配的提供商名称（不区分大小写）
  const matchedProvider = Object.keys(cfg?.providers || {}).find(key =>
    key.toLowerCase() === String(dp).toLowerCase()
  )
  defaultProviderLocal.value = matchedProvider || String(dp)

  // 初始化默认 LLM 模型
  const dcm = (cfg && (cfg as any).default_llm_model) || ''
  if (dcm && dcm.includes('/')) {
    const slashIndex = dcm.indexOf('/')
    const modelName = slashIndex !== -1 ? dcm.substring(slashIndex + 1) : dcm
    defaultChatModelLocal.value = modelName || ''
  } else {
    defaultChatModelLocal.value = String(dcm)
  }

  // 初始化默认 VLM 配置
  const dvm = (cfg && (cfg as any).default_vlm_model) || ''
  let dvmProvider = ''
  let dvmModel = ''
  if (dvm && dvm.includes('/')) {
    const slashIndex = dvm.indexOf('/')
    dvmProvider = slashIndex !== -1 ? dvm.substring(0, slashIndex) : ''
    dvmModel = slashIndex !== -1 ? dvm.substring(slashIndex + 1) : dvm
  } else {
    dvmModel = String(dvm)
  }

  const dvp = (cfg && (cfg as any).default_vlm_provider) || dvmProvider || dp
  const matchedVlmProvider = Object.keys(cfg?.providers || {}).find(key =>
    key.toLowerCase() === String(dvp).toLowerCase()
  )
  defaultVlmProviderLocal.value = matchedVlmProvider || String(dvp || '')
  defaultVlmModelLocal.value = dvmModel || ''
}, { immediate: true, deep: true })

const onChangeDefaultProvider = async () => {
  try {
    const provider = defaultProviderLocal.value
    // 发送小写格式的提供商名称给后端
    emit('setDefaultProvider', provider.toLowerCase())

    // 当提供商变化时，清空默认模型选择
    defaultChatModelLocal.value = ''
    emit('setDefaultChatModel', '')
  } catch (e) {
    console.error('Failed to set default provider', e)
  }
}

const onChangeDefaultVlmProvider = async () => {
  try {
    const provider = defaultVlmProviderLocal.value
    emit('setDefaultVlmProvider', provider.toLowerCase())

    // 当VLM提供商变化时，清空默认模型选择
    defaultVlmModelLocal.value = ''
    emit('setDefaultVisionModel', '')
  } catch (e) {
    console.error('Failed to set default VLM provider', e)
  }
}

const onChangeDefaultChatModel = async () => {
  try {
    const model = defaultChatModelLocal.value
    emit('setDefaultChatModel', model)
  } catch (e) {
    console.error('Failed to set default chat model', e)
  }
}

const onChangeDefaultVisionModel = async () => {
  try {
    const model = defaultVlmModelLocal.value
    if (!model) {
      emit('setDefaultVisionModel', '')
      return
    }
    const provider = defaultVlmProviderLocal.value
    const providerValue = provider ? provider.toLowerCase() : ''
    const modelValue = providerValue ? `${providerValue}/${model}` : model
    emit('setDefaultVisionModel', modelValue)
  } catch (e) {
    console.error('Failed to set default vision model', e)
  }
}

// 获取已启用的提供商列表
const getEnabledProviders = () => {
  if (!props.aiConfig.providers) {
    return []
  }

  return Object.keys(props.aiConfig.providers).filter(providerKey => {
    const provider = props.aiConfig.providers[providerKey]
    return provider && provider.enabled === true
  })
}

// Provider 选项（用于可搜索下拉）
const providerOptions = computed(() => {
  return getEnabledProviders().map(provider => ({
    value: provider,
    label: getProviderName(provider),
    description: ''
  }))
})

// Chat 模型选项（用于可搜索下拉）
const chatModelOptions = computed(() => {
  const models = getProviderModels(defaultProviderLocal.value)
  return models.map((model: any) => ({
    value: model.id,
    label: model.name,
    description: model.description || ''
  }))
})

// VLM 模型选项（用于可搜索下拉）
const vlmModelOptions = computed(() => {
  const models = getProviderModels(defaultVlmProviderLocal.value)
  return models.map((model: any) => ({
    value: model.id,
    label: model.supports_vision ? `👁️ ${model.name}` : model.name,
    description: model.description || ''
  }))
})

// 选中提供商的模型选项（用于可搜索下拉）
const selectedProviderModelOptions = computed(() => {
  const models = selectedProviderConfig.value?.models || []
  return models.map((model: any) => ({
    value: model.id,
    label: model.name,
    description: model.description || ''
  }))
})

// 选中提供商的默认模型（双向绑定）
const selectedProviderDefaultModel = computed({
  get: () => selectedProviderConfig.value?.default_model || '',
  set: (value: string) => {
    if (selectedProviderConfig.value) {
      selectedProviderConfig.value.default_model = value
    }
  }
})

// 选中提供商的最大上下文长度（双向绑定）
const selectedProviderMaxContextLength = computed({
  get: () => selectedProviderConfig.value?.max_context_length || 128000,
  set: (value: number) => {
    if (selectedProviderConfig.value) {
      selectedProviderConfig.value.max_context_length = value
    }
  }
})

// 提供商默认模型变更处理
const onSelectedProviderModelChange = () => {
  saveAiConfig()
}

// 获取指定提供商的模型列表
const getProviderModels = (providerKey: string) => {
  if (!providerKey || !props.aiConfig.providers) {
    return []
  }

  // 查找匹配的提供商（不区分大小写）
  const provider = Object.keys(props.aiConfig.providers).find(key =>
    key.toLowerCase() === providerKey.toLowerCase()
  )

  if (!provider) {
    return []
  }

  const models = props.aiConfig.providers[provider]?.models || []
  return models
}

// Methods
const getProviderIcon = (provider: string) => {
  const icons: Record<string, string> = {
    'OpenAI': 'fas fa-brain',
    'Anthropic': 'fas fa-robot',
    'Google': 'fab fa-google',
    'Gemini': 'fab fa-google',
    'Ollama': 'fas fa-server',
    'DeepSeek': 'fas fa-eye',
    'Moonshot': 'fas fa-moon',
    'OpenRouter': 'fas fa-route',
    'ModelScope': 'fas fa-cog',
    'Groq': 'fas fa-bolt',
    'Perplexity': 'fas fa-search',
    'TogetherAI': 'fas fa-users',
    'xAI': 'fas fa-atom',
    'Cohere': 'fas fa-comments',
    'Hyperbolic': 'fas fa-infinity',
  }
  return icons[provider] || 'fas fa-cog'
}

const getProviderName = (provider: string) => {
  const names: Record<string, string> = {
    'OpenAI': 'OpenAI',
    'Anthropic': 'Anthropic',
    'Google': 'Google',
    'Gemini': 'Gemini',
    'Ollama': 'Ollama',
    'DeepSeek': 'DeepSeek',
    'Moonshot': 'Moonshot',
    'OpenRouter': 'OpenRouter',
    'ModelScope': 'ModelScope',
    'Groq': 'Groq',
    'Perplexity': 'Perplexity',
    'TogetherAI': 'TogetherAI',
    'xAI': 'xAI',
    'Cohere': 'Cohere',
    'Hyperbolic': 'Hyperbolic',
  }
  return names[provider] || provider
}

// rig 库支持的提供商列表
const rigProviderOptions = [
  { value: 'openai', label: 'OpenAI', description: 'OpenAI 及兼容 API' },
  { value: 'anthropic', label: 'Anthropic', description: 'Claude 系列模型' },
  { value: 'gemini', label: 'Google Gemini', description: 'Google Gemini 模型' },
  { value: 'openrouter', label: 'OpenRouter', description: '多模型路由服务' },
  { value: 'ollama', label: 'Ollama', description: '本地模型服务' },
  { value: 'deepseek', label: 'DeepSeek', description: 'DeepSeek 模型' },
  { value: 'groq', label: 'Groq', description: 'Groq 高速推理' },
  { value: 'perplexity', label: 'Perplexity', description: 'Perplexity 搜索增强' },
  { value: 'togetherai', label: 'TogetherAI', description: '开源模型托管' },
  { value: 'xai', label: 'xAI', description: 'xAI Grok 模型' },
  { value: 'cohere', label: 'Cohere', description: 'Cohere 模型' },
  { value: 'hyperbolic', label: 'Hyperbolic', description: 'Hyperbolic 模型' },
  { value: 'moonshot', label: 'Moonshot', description: 'Moonshot Kimi 模型' },
  { value: 'azure', label: 'Azure OpenAI', description: 'Azure 托管的 OpenAI' },
]

const needsApiKey = (provider: string) => {
  // Ollama never needs API key
  if (['Ollama'].includes(provider)) {
    return false
  }
  
  // Check if this is a local OpenAI-compatible service (like LM Studio)
  const providerConfig = selectedProviderConfig.value
  if (providerConfig && providerConfig.rig_provider === 'openai') {
    const apiBase = providerConfig.api_base || ''
    if (apiBase.startsWith('http://localhost') || 
        apiBase.startsWith('http://127.0.0.1') ||
        apiBase.startsWith('http://0.0.0.0')) {
      return false
    }
  }
  
  return true
}

const testConnection = (provider: string) => {
  emit('testConnection', provider)
}

const refreshModels = (provider: string) => {
  emit('refreshModels', provider)
}

const testCustomProvider = () => {
  emit('testCustomProvider')
}

const addCustomProvider = () => {
  emit('addCustomProvider')
}

const clearUsageStats = () => {
  emit('clearUsageStats')
}

const saveAiConfig = async () => {
  await saveWorkingDirectory()
  await saveTavilyConfig()
  await saveAliyunConfig()
  emit('saveAiConfig')
}

// --- AI Working Directory Settings ---
const workingDirectoryLocal = ref('')

const loadWorkingDirectory = async () => {
  try {
    const items = await invoke('get_config', { request: { category: 'ai', key: null } }) as Array<{ key: string, value: string }>
    const map = new Map(items.map(i => [i.key, i.value]))
    workingDirectoryLocal.value = String(map.get('working_directory') || '')
  } catch (e) {
    console.warn('Failed to load working directory config', e)
  }
}

const saveWorkingDirectory = async () => {
  try {
    const configs = [
      { category: 'ai', key: 'working_directory', value: workingDirectoryLocal.value || '', description: 'AI assistant working directory', is_encrypted: false },
    ]
    await invoke('save_config_batch', { configs })
  } catch (e) {
    console.error('Failed to save working directory config', e)
  }
}

const selectWorkingDirectory = async () => {
  try {
    const { open } = await import('@tauri-apps/plugin-dialog')
    const selected = await open({
      directory: true,
      multiple: false,
      title: t('settings.ai.selectDirectory'),
    })
    if (selected) {
      workingDirectoryLocal.value = selected as string
    }
  } catch (e) {
    console.error('Failed to select directory', e)
  }
}

// --- Tavily Search Settings ---
const tavilyApiKeyLocal = ref('')
const tavilyMaxResultsLocal = ref<number>(5)

const loadTavilyConfig = async () => {
  try {
    const items = await invoke('get_config', { request: { category: 'ai', key: null } }) as Array<{ key: string, value: string }>
    const map = new Map(items.map(i => [i.key, i.value]))
    tavilyApiKeyLocal.value = String(map.get('tavily_api_key') || '')
    const mr = Number(map.get('tavily_max_results') || 5)
    tavilyMaxResultsLocal.value = isNaN(mr) ? 5 : Math.min(Math.max(mr, 1), 20)
  } catch (e) {
    console.warn('Failed to load Tavily config', e)
  }
}

const saveTavilyConfig = async () => {
  try {
    const configs = [
      { category: 'ai', key: 'tavily_api_key', value: tavilyApiKeyLocal.value || '', description: 'Tavily API key for web search', is_encrypted: true },
      { category: 'ai', key: 'tavily_max_results', value: String(tavilyMaxResultsLocal.value || 5), description: 'Default max results for Tavily', is_encrypted: false },
    ]
    await invoke('save_config_batch', { configs })
  } catch (e) {
    console.error('Failed to save Tavily config', e)
  }
}

// --- Aliyun DashScope Settings ---
const aliyunApiKeyLocal = ref('')
const aliyunDefaultModelLocal = ref('qwen-vl-plus')
const testingAliyun = ref(false)

const loadAliyunConfig = async () => {
  try {
    const items = await invoke('get_config', { request: { category: 'ai', key: null } }) as Array<{ key: string, value: string }>
    const map = new Map(items.map(i => [i.key, i.value]))
    aliyunApiKeyLocal.value = String(map.get('aliyun_dashscope_api_key') || '')
    aliyunDefaultModelLocal.value = String(map.get('aliyun_dashscope_model') || 'qwen-vl-plus')
  } catch (e) {
    console.warn('Failed to load Aliyun config', e)
  }
}

const saveAliyunConfig = async () => {
  try {
    const configs = [
      { category: 'ai', key: 'aliyun_dashscope_api_key', value: aliyunApiKeyLocal.value || '', description: 'Aliyun DashScope API key for file upload', is_encrypted: true },
      { category: 'ai', key: 'aliyun_dashscope_model', value: aliyunDefaultModelLocal.value || 'qwen-vl-plus', description: 'Default model for DashScope upload', is_encrypted: false },
    ]
    await invoke('save_config_batch', { configs })
  } catch (e) {
    console.error('Failed to save Aliyun config', e)
  }
}

const testAliyunConnection = async () => {
  if (!aliyunApiKeyLocal.value) {
    alert('请先输入 DashScope API Key')
    return
  }
  testingAliyun.value = true
  try {
    const result = await invoke('test_aliyun_dashscope_connection', {
      apiKey: aliyunApiKeyLocal.value,
      model: aliyunDefaultModelLocal.value || 'qwen-vl-plus',
    })
    if (result) {
      alert('连接成功！')
    } else {
      alert('连接失败，请检查 API Key')
    }
  } catch (e: any) {
    alert('连接测试失败: ' + (e?.message || e))
  } finally {
    testingAliyun.value = false
  }
}

// CodeMirror 初始化
const initCodeMirror = () => {
  if (!editorContainer.value) return
  
  if (editorView) {
    editorView.destroy()
    editorView = null
  }
  
  editorContainer.value.innerHTML = ''
  
  const state = EditorState.create({
    doc: manualConfigText.value,
    extensions: [
      basicSetup,
      json(),
      oneDark,
      keymap.of([...defaultKeymap, indentWithTab]),
      EditorView.lineWrapping,
      EditorView.updateListener.of((update) => {
        if (update.docChanged) {
          manualConfigText.value = update.state.doc.toString()
          validateConfigText()
        }
      }),
    ],
  })
  
  editorView = new EditorView({
    state,
    parent: editorContainer.value,
  })
}

// 更新编辑器内容
const updateEditorContent = (content: string) => {
  if (!editorView) return
  const currentContent = editorView.state.doc.toString()
  if (currentContent !== content) {
    editorView.dispatch({
      changes: {
        from: 0,
        to: currentContent.length,
        insert: content
      }
    })
  }
  // 同步更新全屏编辑器
  if (fullscreenEditorView) {
    const fsContent = fullscreenEditorView.state.doc.toString()
    if (fsContent !== content) {
      fullscreenEditorView.dispatch({
        changes: {
          from: 0,
          to: fsContent.length,
          insert: content
        }
      })
    }
  }
}

// 全屏编辑器初始化
const initFullscreenEditor = () => {
  if (!fullscreenEditorContainer.value) return
  
  if (fullscreenEditorView) {
    fullscreenEditorView.destroy()
    fullscreenEditorView = null
  }
  
  fullscreenEditorContainer.value.innerHTML = ''
  
  const state = EditorState.create({
    doc: manualConfigText.value,
    extensions: [
      basicSetup,
      json(),
      oneDark,
      keymap.of([...defaultKeymap, indentWithTab]),
      EditorView.lineWrapping,
      EditorView.updateListener.of((update) => {
        if (update.docChanged) {
          manualConfigText.value = update.state.doc.toString()
          validateConfigText()
          // 同步到普通编辑器
          if (editorView) {
            const normalContent = editorView.state.doc.toString()
            const newContent = update.state.doc.toString()
            if (normalContent !== newContent) {
              editorView.dispatch({
                changes: {
                  from: 0,
                  to: normalContent.length,
                  insert: newContent
                }
              })
            }
          }
        }
      }),
    ],
  })
  
  fullscreenEditorView = new EditorView({
    state,
    parent: fullscreenEditorContainer.value,
  })
  
  fullscreenEditorView.focus()
}

// 切换全屏
const toggleFullscreen = async () => {
  isFullscreen.value = true
  await nextTick()
  initFullscreenEditor()
}

// 退出全屏
const exitFullscreen = () => {
  if (fullscreenEditorView) {
    fullscreenEditorView.destroy()
    fullscreenEditorView = null
  }
  isFullscreen.value = false
}

// 应用并退出全屏
const applyAndExitFullscreen = () => {
  applyManualConfig()
  exitFullscreen()
}

// 统计计算属性
const totalInputTokens = computed<number>(() => {
  return Object.values(props.aiUsageStats).reduce((sum: number, usage: any) => sum + (usage.input_tokens || 0), 0)
})

const totalOutputTokens = computed<number>(() => {
  return Object.values(props.aiUsageStats).reduce((sum: number, usage: any) => sum + (usage.output_tokens || 0), 0)
})

const totalCost = computed<number>(() => {
  return Object.values(props.aiUsageStats).reduce((sum: number, usage: any) => sum + (usage.cost || 0), 0)
})

const maxTokens = computed<number>(() => {
  return Math.max(...Object.values(props.aiUsageStats).map((usage: any) => usage.total_tokens || 0), 1)
})

const totalRequests = computed<number>(() => {
  return detailedStats.value.length
})

const totalTokensFormatted = computed<string>(() => {
  const total = totalInputTokens.value + totalOutputTokens.value
  if (total >= 1_000_000) {
    return `${(total / 1_000_000).toFixed(2)}M`
  } else if (total >= 1_000) {
    return `${(total / 1_000).toFixed(2)}K`
  }
  return total.toLocaleString()
})

const avgCostPerRequest = computed<string>(() => {
  if (totalRequests.value === 0) return '0.0000'
  return (totalCost.value / totalRequests.value).toFixed(4)
})

// 格式化最后使用时间
const formatLastUsed = (timestamp: string | null) => {
  if (!timestamp) return '-'
  const date = new Date(timestamp)
  const now = new Date()
  const diff = now.getTime() - date.getTime()
  
  if (diff < 60000) return t('settings.ai.justNow')
  if (diff < 3600000) return t('settings.ai.minutesAgo', { n: Math.floor(diff / 60000) })
  if (diff < 86400000) return t('settings.ai.hoursAgo', { n: Math.floor(diff / 3600000) })
  if (diff < 604800000) return t('settings.ai.daysAgo', { n: Math.floor(diff / 86400000) })
  return date.toLocaleDateString()
}

onMounted(() => {
  loadWorkingDirectory()
  loadTavilyConfig()
  loadAliyunConfig()
  loadDetailedStats()
})

onUnmounted(() => {
  if (editorView) {
    editorView.destroy()
    editorView = null
  }
  if (fullscreenEditorView) {
    fullscreenEditorView.destroy()
    fullscreenEditorView = null
  }
})

// 手动编辑相关方法
const validateConfigText = () => {
  configError.value = ''
  configValid.value = false

  if (!manualConfigText.value.trim()) {
    return
  }

  try {
    const parsed = JSON.parse(manualConfigText.value)

    // 基本验证：确保是对象且包含 providers
    if (typeof parsed !== 'object' || parsed === null) {
      configError.value = '配置必须是有效的 JSON 对象'
      return
    }

    if (!parsed.providers || typeof parsed.providers !== 'object') {
      configError.value = '配置必须包含 providers 对象'
      return
    }

    // 验证每个 provider 的基本结构
    for (const [providerName, providerConfig] of Object.entries(parsed.providers)) {
      if (typeof providerConfig !== 'object' || providerConfig === null) {
        configError.value = `Provider "${providerName}" 必须是对象`
        return
      }

      const config = providerConfig as any
      if (typeof config.enabled !== 'boolean') {
        configError.value = `Provider "${providerName}" 缺少必需的 enabled 字段（布尔值）`
        return
      }
    }

    configValid.value = true
  } catch (error) {
    configError.value = `JSON 解析错误: ${(error as Error).message}`
  }
}

const validateConfig = () => {
  validateConfigText()
}

const applyManualConfig = () => {
  if (configError.value) {
    return
  }

  try {
    const parsed = JSON.parse(manualConfigText.value)
    emit('applyManualConfig', parsed)
  } catch (error) {
    configError.value = `应用配置失败: ${(error as Error).message}`
  }
}

const formatConfig = () => {
  if (!manualConfigText.value.trim()) {
    return
  }

  try {
    const parsed = JSON.parse(manualConfigText.value)
    const formatted = JSON.stringify(parsed, null, 2)
    manualConfigText.value = formatted
    updateEditorContent(formatted)
    validateConfigText()
  } catch (error) {
    // 保持原始文本，不格式化无效的 JSON
  }
}

const resetToDefault = () => {
  const defaultConfig = {
    providers: {
      OpenAI: {
        enabled: false,
        rig_provider: 'openai',
        api_key: '',
        api_base: 'https://api.openai.com/v1',
        default_model: '',
        models: []
      },
      Anthropic: {
        enabled: false,
        rig_provider: 'anthropic',
        api_key: '',
        api_base: 'https://api.anthropic.com',
        default_model: '',
        models: []
      },
      Gemini: {
        enabled: false,
        rig_provider: 'gemini',
        api_key: '',
        api_base: 'https://generativelanguage.googleapis.com/v1beta',
        default_model: '',
        models: []
      },
      Ollama: {
        enabled: false,
        rig_provider: 'ollama',
        api_base: 'http://localhost:11434',
        default_model: '',
        models: []
      },
      DeepSeek: {
        enabled: false,
        rig_provider: 'deepseek',
        api_key: '',
        api_base: 'https://api.deepseek.com/v1',
        default_model: '',
        models: []
      },
      Moonshot: {
        enabled: false,
        rig_provider: 'moonshot',
        api_key: '',
        api_base: 'https://api.moonshot.cn/v1',
        default_model: '',
        models: []
      },
      OpenRouter: {
        enabled: false,
        rig_provider: 'openrouter',
        api_key: '',
        api_base: 'https://openrouter.ai/api/v1',
        default_model: '',
        http_referer: '',
        x_title: '',
        models: []
      },
      Groq: {
        enabled: false,
        rig_provider: 'groq',
        api_key: '',
        api_base: 'https://api.groq.com/openai/v1',
        default_model: '',
        models: []
      },
      Perplexity: {
        enabled: false,
        rig_provider: 'perplexity',
        api_key: '',
        api_base: 'https://api.perplexity.ai',
        default_model: '',
        models: []
      },
      xAI: {
        enabled: false,
        rig_provider: 'xai',
        api_key: '',
        api_base: 'https://api.x.ai/v1',
        default_model: '',
        models: []
      }
    }
  }

  const formatted = JSON.stringify(defaultConfig, null, 2)
  manualConfigText.value = formatted
  updateEditorContent(formatted)
  validateConfigText()
}

// 监听 aiConfig 变化，同步到手动编辑文本
watch(() => props.aiConfig, (newConfig) => {
  if (newConfig && !useGuiMode.value) {
    const newText = JSON.stringify(newConfig, null, 2)
    manualConfigText.value = newText
    updateEditorContent(newText)
    validateConfigText()
  }
}, { immediate: true, deep: true })

// 初始化手动编辑文本
watch(useGuiMode, async (isGuiMode) => {
  if (!isGuiMode && props.aiConfig) {
    manualConfigText.value = JSON.stringify(props.aiConfig, null, 2)
    validateConfigText()
    await nextTick()
    initCodeMirror()
  } else if (isGuiMode && editorView) {
    editorView.destroy()
    editorView = null
  }
})

</script>

<style scoped>
.ai-settings {
  @apply space-y-6;
}

.card {
  @apply transition-all duration-200 hover:shadow-md;
}

.stat {
  @apply transition-all duration-200 hover:scale-105;
}

.tab {
  @apply transition-all duration-200;
}

.tab:hover {
  @apply bg-base-300;
}

.tab-active {
  @apply bg-primary text-primary-content;
}

.editor-container {
  height: 24rem;
}

.editor-container :deep(.cm-editor) {
  height: 100%;
  font-size: 0.875rem;
}

.editor-container :deep(.cm-scroller) {
  overflow: auto;
}

/* 全屏按钮样式 */
.fullscreen-btn {
  position: absolute;
  top: 0.5rem;
  right: 0.5rem;
  z-index: 10;
  padding: 0.5rem;
  border-radius: 0.375rem;
  background: rgba(255, 255, 255, 0.1);
  color: #9ca3af;
  border: 1px solid rgba(255, 255, 255, 0.1);
  cursor: pointer;
  transition: all 0.2s;
}

.fullscreen-btn:hover {
  background: rgba(255, 255, 255, 0.2);
  color: #fff;
}

/* 全屏编辑器样式 */
.fullscreen-editor-overlay {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: rgba(0, 0, 0, 0.8);
  z-index: 9999;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 2rem;
}

.fullscreen-editor-container {
  width: 100%;
  height: 100%;
  max-width: 1400px;
  background: var(--fallback-b1, oklch(var(--b1)));
  border-radius: 0.75rem;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.fullscreen-editor-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 1rem 1.5rem;
  border-bottom: 1px solid var(--fallback-b3, oklch(var(--b3)));
}

.fullscreen-editor-content {
  flex: 1;
  overflow: hidden;
}

.fullscreen-editor-content :deep(.cm-editor) {
  height: 100%;
  font-size: 0.875rem;
}

.fullscreen-editor-content :deep(.cm-scroller) {
  overflow: auto;
}

.fullscreen-editor-footer {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  padding: 1rem 1.5rem;
  border-top: 1px solid var(--fallback-b3, oklch(var(--b3)));
}
</style>
