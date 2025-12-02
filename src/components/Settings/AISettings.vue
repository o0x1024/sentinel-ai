<template>
  <div class="ai-settings">
    <!-- 配置模式切换 -->
    <div class="flex justify-between items-center mb-6">
      <h2 class="text-2xl font-bold">AI 配置</h2>
      <div class="flex items-center gap-4">
        <div class="form-control">
          <label class="label cursor-pointer gap-2">
            <span class="label-text">图形界面</span>
            <input type="checkbox" class="toggle toggle-primary" 
                   v-model="useGuiMode" />
            <span class="label-text">手动编辑</span>
          </label>
        </div>
        <button v-if="!useGuiMode" class="btn btn-primary btn-sm" @click="validateConfig">
          <i class="fas fa-check"></i>
          验证配置
        </button>
      </div>
    </div>

    <!-- 手动编辑模式 -->
    <div v-if="!useGuiMode" class="card bg-base-100 shadow-sm mb-6">
      <div class="card-body">
        <h3 class="card-title mb-4">
          <i class="fas fa-code"></i>
          手动编辑 Providers 配置
        </h3>
        
        <!-- JSON 编辑器 -->
        <div class="space-y-4">
          <div class="flex items-center gap-2 mb-2">
            <span class="text-sm text-base-content/70">
              直接编辑 AI 提供商配置 JSON
            </span>
            <div class="badge badge-warning badge-sm" v-if="configError">
              配置有误
            </div>
            <div class="badge badge-success badge-sm" v-else-if="configValid">
              配置有效
            </div>
          </div>
          
          <textarea 
            class="textarea textarea-bordered font-mono text-sm h-96 w-full"
            :class="{
              'textarea-error': configError,
              'textarea-success': configValid && !configError
            }"
            v-model="manualConfigText"
            @input="onManualConfigChange"
            placeholder="输入 providers 配置的 JSON 格式..."
          ></textarea>
          
          <div v-if="configError" class="alert alert-error">
            <i class="fas fa-exclamation-triangle"></i>
            <span>{{ configError }}</span>
          </div>
          
          <div class="flex gap-2">
            <button class="btn btn-primary" @click="applyManualConfig" :disabled="!!configError">
              <i class="fas fa-save"></i>
              应用配置
            </button>
            <button class="btn btn-outline" @click="formatConfig">
              <i class="fas fa-indent"></i>
              格式化
            </button>
            <button class="btn btn-outline" @click="resetToDefault">
              <i class="fas fa-undo"></i>
              重置为默认
            </button>
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
              <h3 class="font-semibold text-lg">默认配置</h3>
            </div>
            
            <!-- 默认Provider和模型选择器 -->
            <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
              <!-- 默认Provider选择器 -->
              <div class="space-y-2">
                <label class="label">
                  <span class="label-text font-medium flex items-center gap-2">
                    <i class="fas fa-star text-warning"></i>
                    默认 Provider
                  </span>
                </label>
                <SearchableSelect
                  v-model="defaultProviderLocal"
                  :options="providerOptions"
                  :placeholder="t('settings.ai.selectProvider', '选择提供商')"
                  search-placeholder="搜索提供商..."
                  @change="onChangeDefaultProvider"
                />
              </div>
              
              <!-- 默认模型选择器 -->
              <div class="space-y-2">
                <label class="label">
                  <span class="label-text font-medium flex items-center gap-2">
                    <i class="fas fa-robot text-primary"></i>
                    默认 Chat 模型
                  </span>
                </label>
                <SearchableSelect
                  v-model="defaultChatModelLocal"
                  :options="chatModelOptions"
                  :placeholder="t('settings.ai.selectModel')"
                  search-placeholder="搜索模型..."
                  :disabled="!defaultProviderLocal || !getProviderModels(defaultProviderLocal).length"
                  @change="onChangeDefaultChatModel"
                />
              </div>
            </div>
            
            <!-- 提示信息 -->
            <div class="flex items-center gap-2 mt-3 text-sm text-base-content/70">
              <i class="fas fa-info-circle"></i>
              <span>AI助手将使用此配置进行对话</span>
            </div>
          </div>
        </div>
        <div v-for="status in aiServiceStatus" :key="status.provider" 
             class="card bg-base-100 shadow-sm border">
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
                  <span class="text-sm text-base-content/70">{{ status.models_loaded }} {{ t('settings.ai.modelsCount') }}</span>
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
        <h3 class="text-lg font-semibold mb-3">AI 提供商</h3>
        <div class="menu bg-base-200 rounded-box p-2 space-y-1">
          <li v-for="provider in Object.keys(aiConfig.providers)" :key="provider">
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
            <span class="label-text">{{ t('settings.ai.enable') }} {{ getProviderName(selectedAiProvider) }}</span>
            <input type="checkbox" class="toggle toggle-primary" 
                   v-model="selectedProviderConfig.enabled"
                   @change="saveAiConfig">
          </label>
        </div>

        <!-- Rig 提供商类型 -->
        <div class="form-control">
          <label class="label">
            <span class="label-text">Rig 提供商类型</span>
            <span class="label-text-alt text-info">后端 API 调用方式</span>
          </label>
          <SearchableSelect
            v-model="selectedProviderConfig.rig_provider"
            :options="rigProviderOptions"
            placeholder="选择提供商类型..."
            search-placeholder="搜索提供商..."
            @change="saveAiConfig"
          />
          <label class="label">
            <span class="label-text-alt">决定后端使用哪种 API 格式发送请求</span>
          </label>
        </div>

        <!-- API密钥配置 -->
        <div class="form-control" v-if="needsApiKey(selectedAiProvider)">
          <label class="label">
            <span class="label-text">{{ t('settings.ai.apiKey') }}</span>
          </label>
          <div class="input-group">
            <input type="password" :placeholder="t('settings.apiKeyPlaceholder')" 
                   class="input input-bordered flex-1"
                   v-model="selectedProviderConfig.api_key"
                   @blur="saveAiConfig">
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
        <div class="form-control" v-if="selectedProviderConfig.api_base">
          <label class="label">
            <span class="label-text">{{ t('settings.ai.apiBaseUrl') }}</span>
          </label>
          <div class="input-group">
            <input type="url" :placeholder="t('settings.ai.apiBaseUrl')" 
                   class="input input-bordered flex-1"
                   v-model="selectedProviderConfig.api_base"
                   @blur="saveAiConfig">
            <!-- 为Ollama等不需要API密钥但需要测试连接的提供商添加按钮 -->
            <button v-if="!needsApiKey(selectedAiProvider)" class="btn btn-outline" @click="testConnection(selectedAiProvider)">
              <i class="fas fa-plug"></i>
              {{ t('settings.testConnection') }}
            </button>
            <button v-if="!needsApiKey(selectedAiProvider)" class="btn btn-outline" @click="refreshModels(selectedAiProvider)">
              <i class="fas fa-sync-alt"></i>
              {{ t('settings.ai.refreshModels') }}
            </button>
          </div>
        </div>

        <!-- 组织ID (OpenAI) -->
        <div class="form-control" v-if="selectedAiProvider === 'OpenAI' && selectedProviderConfig && 'organization' in selectedProviderConfig">
          <label class="label">
            <span class="label-text">{{ t('settings.ai.organizationId') }}</span>
          </label>
          <input type="text" :placeholder="t('settings.ai.organizationId')" 
                 class="input input-bordered"
                 v-model="(selectedProviderConfig as any).organization"
                 @blur="saveAiConfig">
        </div>

        <!-- OpenRouter特定配置 -->
        <div v-if="selectedAiProvider === 'OpenRouter'" class="space-y-4">
          <!-- HTTP Referer -->
          <div class="form-control">
            <label class="label">
              <span class="label-text">HTTP Referer (可选)</span>
            </label>
            <input type="url" placeholder="https://yoursite.com" 
                   class="input input-bordered"
                   v-model="selectedProviderConfig.http_referer"
                   @blur="saveAiConfig">
            <label class="label">
              <span class="label-text-alt">用于在 OpenRouter 上进行排名统计</span>
            </label>
          </div>

          <!-- X-Title -->
          <div class="form-control">
            <label class="label">
              <span class="label-text">应用名称 (可选)</span>
            </label>
            <input type="text" placeholder="我的AI应用" 
                   class="input input-bordered"
                   v-model="selectedProviderConfig.x_title"
                   @blur="saveAiConfig">
            <label class="label">
              <span class="label-text-alt">用于在 OpenRouter 上显示站点标题</span>
            </label>
          </div>
        </div>

        <!-- 默认模型选择 -->
        <div class="form-control">
          <label class="label">
            <span class="label-text">{{ t('settings.ai.defaultModel') }}</span>
          </label>
          <SearchableSelect
            v-model="selectedProviderDefaultModel"
            :options="selectedProviderModelOptions"
            :placeholder="t('settings.ai.selectModel')"
            search-placeholder="搜索模型..."
            @change="onSelectedProviderModelChange"
          />
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
            <input 
              v-model.number="settings.ai.temperature"
              type="range"
              min="0"
              max="1"
              step="0.1"
              class="range range-primary flex-1"
              @change="saveAiConfig"
            />
            <span class="text-sm min-w-[60px]">{{ settings.ai.temperature }}</span>
          </div>
          <label class="label">
            <span class="label-text-alt">{{ t('settings.ai.temperatureHint') }}</span>
          </label>
        </div>
        
        <!-- 最大Token设置 -->
        <div class="form-control">
          <label class="label">
            <span class="label-text">{{ t('settings.ai.maxTokens') }}</span>
          </label>
          <div class="flex items-center gap-4">
            <input 
              v-model.number="settings.ai.maxTokens"
              type="range"
              min="500"
              max="8000"
              step="500"
              class="range range-primary flex-1"
              @change="saveAiConfig"
            />
            <span class="text-sm min-w-[60px]">{{ settings.ai.maxTokens }}</span>
          </div>
          <label class="label">
            <span class="label-text-alt">{{ t('settings.ai.maxTokensHint') }}</span>
          </label>
        </div>
      </div>
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
              <span class="label-text-alt">后端将从数据库读取用于联网搜索</span>
            </label>
          </div>
          <div class="form-control">
            <label class="label">
              <span class="label-text">默认最大结果数</span>
            </label>
            <input v-model.number="tavilyMaxResultsLocal" type="number" min="1" max="20" class="input input-bordered w-40" />
          </div>
        </div>
        <div class="flex justify-end mt-3">
          <button class="btn btn-primary btn-sm" @click="saveAiConfig">
            <i class="fas fa-save mr-1"></i>
            保存设置
          </button>
        </div>
      </div>
    </div>

    <!-- 阿里云 OSS 配置（用于 DashScope 文件上传） -->
    <div class="card bg-base-100 shadow-sm mt-6">
      <div class="card-body p-4">
        <div class="flex items-center gap-3 mb-2">
          <i class="fas fa-cloud-upload-alt text-primary text-lg"></i>
          <h3 class="font-semibold">阿里云 DashScope</h3>
        </div>
        <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
          <div class="form-control">
            <label class="label">
              <span class="label-text">DashScope API Key</span>
            </label>
            <input v-model="aliyunApiKeyLocal" type="password" class="input input-bordered" placeholder="sk-..." />
            <label class="label">
              <span class="label-text-alt">用于文件上传和多模态模型调用</span>
            </label>
          </div>
          <div class="form-control">
            <label class="label">
              <span class="label-text">默认模型</span>
            </label>
            <input v-model="aliyunDefaultModelLocal" type="text" class="input input-bordered" placeholder="qwen-vl-plus" />
            <label class="label">
              <span class="label-text-alt">上传文件时使用的模型名称</span>
            </label>
          </div>
        </div>
        <div class="flex justify-end mt-3 gap-2">
          <button class="btn btn-outline btn-sm" @click="testAliyunConnection" :disabled="testingAliyun">
            <span v-if="testingAliyun" class="loading loading-spinner loading-sm"></span>
            <i v-else class="fas fa-plug mr-1"></i>
            测试连接
          </button>
          <button class="btn btn-primary btn-sm" @click="saveAiConfig">
            <i class="fas fa-save mr-1"></i>
            保存设置
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
              <span class="label-text-alt text-warning">*必填，唯一标识</span>
            </label>
            <input type="text" placeholder="MyCustomProvider" 
                   class="input input-bordered"
                   v-model="customProvider.name">
          </div>
          
          <!-- Rig 提供商选择 -->
          <div class="form-control">
            <label class="label">
              <span class="label-text">Rig 提供商</span>
              <span class="label-text-alt text-warning">*必填，决定 API 调用方式</span>
            </label>
            <SearchableSelect
              v-model="customProvider.rig_provider"
              :options="rigProviderOptions"
              placeholder="选择提供商类型..."
              search-placeholder="搜索提供商..."
            />
            <label class="label">
              <span class="label-text-alt">
                选择与你的 API 服务兼容的提供商类型，后端将使用对应的调用方式
              </span>
            </label>
          </div>
          
          <!-- API密钥 -->
          <div class="form-control">
            <label class="label">
              <span class="label-text">{{ t('settings.ai.apiKey') }}</span>
              <span class="label-text-alt text-warning" v-if="customProvider.compat_mode !== 'ollama'">*必填</span>
            </label>
            <input type="password" :placeholder="t('settings.apiKeyPlaceholder')" 
                   class="input input-bordered"
                   v-model="customProvider.api_key">
          </div>
          
          <!-- API Base URL -->
          <div class="form-control">
            <label class="label">
              <span class="label-text">{{ t('settings.ai.apiBaseUrl') }}</span>
              <span class="label-text-alt text-warning">*必填</span>
            </label>
            <input type="url" placeholder="https://api.example.com/v1" 
                   class="input input-bordered"
                   v-model="customProvider.api_base">
            <label class="label">
              <span class="label-text-alt">
                示例：https://api.openai.com/v1
              </span>
            </label>
          </div>
          
          <!-- 模型ID -->
          <div class="form-control">
            <label class="label">
              <span class="label-text">{{ t('settings.ai.modelId') }}</span>
              <span class="label-text-alt text-warning">*必填，默认模型</span>
            </label>
            <input type="text" placeholder="gpt-4o-mini" 
                   class="input input-bordered"
                   v-model="customProvider.model_id">
          </div>
          
          <!-- 显示名称 -->
          <div class="form-control">
            <label class="label">
              <span class="label-text">显示名称</span>
              <span class="label-text-alt">可选，用于界面显示</span>
            </label>
            <input type="text" placeholder="我的自定义提供商" 
                   class="input input-bordered"
                   v-model="customProvider.display_name">
          </div>
        </div>
        
        <!-- 高级选项折叠 -->
        <div class="collapse collapse-arrow bg-base-200 mt-4">
          <input type="checkbox" /> 
          <div class="collapse-title font-medium">
            <i class="fas fa-cogs mr-2"></i>高级选项
          </div>
          <div class="collapse-content">
            <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
              <!-- 额外请求头 -->
              <div class="form-control md:col-span-2">
                <label class="label">
                  <span class="label-text">额外请求头 (JSON)</span>
                </label>
                <textarea 
                  class="textarea textarea-bordered font-mono text-sm h-24"
                  placeholder='{"X-Custom-Header": "value"}'
                  v-model="customProvider.extra_headers_json"
                ></textarea>
              </div>
              
              <!-- 超时设置 -->
              <div class="form-control">
                <label class="label">
                  <span class="label-text">请求超时（秒）</span>
                </label>
                <input type="number" class="input input-bordered" 
                       v-model.number="customProvider.timeout" 
                       min="10" max="600" placeholder="120">
              </div>
              
              <!-- 最大重试次数 -->
              <div class="form-control">
                <label class="label">
                  <span class="label-text">最大重试次数</span>
                </label>
                <input type="number" class="input input-bordered" 
                       v-model.number="customProvider.max_retries" 
                       min="0" max="5" placeholder="3">
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
          <button class="btn btn-outline" @click="testCustomProvider" :disabled="!!customProviderValidationError || testingCustomProvider">
            <span v-if="testingCustomProvider" class="loading loading-spinner loading-sm"></span>
            <i v-else class="fas fa-vial"></i>
            {{ t('settings.ai.testCustomProvider') }}
          </button>
          <button class="btn btn-primary" @click="addCustomProvider" :disabled="!!customProviderValidationError || addingCustomProvider">
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
        <h3 class="card-title">
          <i class="fas fa-chart-bar"></i>
          {{ t('settings.ai.usageStats') }}
        </h3>
        
        <div class="overflow-x-auto">
          <table class="table table-compact w-full">
            <thead>
              <tr>
                <th>{{ t('settings.providers') }}</th>
                <th>{{ t('settings.ai.inputTokens') }}</th>
                <th>{{ t('settings.ai.outputTokens') }}</th>
                <th>{{ t('settings.ai.totalTokens') }}</th>
                <th>{{ t('settings.ai.estimatedCost') }}</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="(usage, provider) in aiUsageStats" :key="provider">
                <td>{{ getProviderName(String(provider)) }}</td>
                <td>{{ usage.input_tokens?.toLocaleString() }}</td>
                <td>{{ usage.output_tokens?.toLocaleString() }}</td>
                <td>{{ usage.total_tokens?.toLocaleString() }}</td>
                <td>${{ (usage.cost || 0).toFixed(4) }}</td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>
    </div>

    </div>

  </div>
</template>

<script setup lang="ts">
import { computed, ref, watch, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { invoke } from '@tauri-apps/api/core'
import SearchableSelect from '@/components/SearchableSelect.vue'

const { t } = useI18n()

// 手动编辑模式相关状态
const useGuiMode = ref(true)
const manualConfigText = ref('')
const configError = ref('')
const configValid = ref(false)

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
}

const emit = defineEmits<Emits>()

// Computed
const selectedAiProvider = computed({
  get: () => props.selectedAiProvider,
  set: (value: string) => {
    emit('update:selectedAiProvider', value)
  }
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
    return '请输入提供商名称（唯一标识）'
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

// 默认 Provider 选择
const defaultProviderLocal = ref('')
// 默认 Chat 模型选择
const defaultChatModelLocal = ref('')

watch(() => props.aiConfig, (cfg: any) => {
  
  const dp = (cfg && (cfg as any).default_provider) || 'modelscope'
  // 查找匹配的提供商名称（不区分大小写）
  const matchedProvider = Object.keys(cfg?.providers || {}).find(key => 
    key.toLowerCase() === String(dp).toLowerCase()
  )
  defaultProviderLocal.value = matchedProvider || String(dp)
  
  // 初始化默认 Chat 模型
  const dcm = (cfg && (cfg as any).default_chat_model) || ''
  
  // 解析 default_chat_model 格式：provider/model_name
  if (dcm && dcm.includes('/')) {
    // 处理复杂的模型ID，如 "modelscope/Qwen/Qwen2-VL-7B-Instruct"
    // 提取 provider/ 之后的所有内容作为模型名
    const slashIndex = dcm.indexOf('/')
    const modelName = slashIndex !== -1 ? dcm.substring(slashIndex + 1) : dcm
    defaultChatModelLocal.value = modelName || ''
  } else {
    defaultChatModelLocal.value = String(dcm)
  }
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

const onChangeDefaultChatModel = async () => {
  try {
    const model = defaultChatModelLocal.value
    emit('setDefaultChatModel', model)
  } catch (e) {
    console.error('Failed to set default chat model', e)
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
  return !['Ollama'].includes(provider)
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

const saveAiConfig = async () => {
  await saveTavilyConfig()
  await saveAliyunConfig()
  emit('saveAiConfig')
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

onMounted(() => {
  loadTavilyConfig()
  loadAliyunConfig()
})

// 手动编辑相关方法
const onManualConfigChange = () => {
  validateConfigText()
}

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
    manualConfigText.value = JSON.stringify(parsed, null, 2)
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
  
  manualConfigText.value = JSON.stringify(defaultConfig, null, 2)
  validateConfigText()
}

// 监听 aiConfig 变化，同步到手动编辑文本
watch(() => props.aiConfig, (newConfig) => {
  if (newConfig && !useGuiMode.value) {
    manualConfigText.value = JSON.stringify(newConfig, null, 2)
    validateConfigText()
  }
}, { immediate: true, deep: true })

// 初始化手动编辑文本
watch(useGuiMode, (isGuiMode) => {
  if (!isGuiMode && props.aiConfig) {
    manualConfigText.value = JSON.stringify(props.aiConfig, null, 2)
    validateConfigText()
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
</style>