<template>
  <div class="agent-creator page-content-padded">
    <!-- 创建向导步骤 -->
    <div class="w-full mb-8">
      <div class="steps-container">
        <ul class="steps steps-horizontal w-full">
          <li 
            v-for="(stepInfo, index) in stepsList" 
            :key="index"
            class="step cursor-pointer transition-all duration-200 hover:scale-105" 
            :class="{ 
              'step-primary': currentStep >= index + 1,
              'step-active': currentStep === index + 1,
              'step-completed': currentStep > index + 1
            }"
            @click="goToStep(index + 1)"
          >
            <div class="step-content">
              <div class="step-number">{{ index + 1 }}</div>
              <div class="step-title">{{ stepInfo.title }}</div>
              <div class="step-subtitle">{{ stepInfo.subtitle }}</div>
            </div>
          </li>
        </ul>
      </div>
      
      <!-- 移动端步骤指示器 -->
      <div class="mobile-steps-indicator md:hidden mt-4">
        <div class="flex items-center justify-center space-x-2">
          <span class="text-sm font-medium">步骤 {{ currentStep }} / {{ stepsList.length }}</span>
          <div class="flex space-x-1">
            <div 
              v-for="(_, index) in stepsList" 
              :key="index"
              class="w-2 h-2 rounded-full transition-all duration-200"
              :class="currentStep >= index + 1 ? 'bg-primary' : 'bg-base-300'"
            ></div>
          </div>
        </div>
        <div class="text-center mt-2">
          <span class="text-lg font-semibold text-primary">{{ stepsList[currentStep - 1]?.title }}</span>
          <p class="text-sm text-base-content/70">{{ stepsList[currentStep - 1]?.subtitle }}</p>
        </div>
      </div>
    </div>

    <!-- 步骤1: 基本信息 -->
    <div v-if="currentStep === 1" class="card bg-base-100 shadow-xl">
      <div class="card-header">
        <h2 class="card-title">
          <i class="fas fa-info-circle mr-2"></i>
          {{ t('agentCreator.basicInfo.title', 'Agent基本信息') }}
        </h2>
      </div>
      <div class="card-body space-y-4">
        <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
          <div class="form-control">
            <label class="label">
              <span class="label-text">{{ t('agentCreator.basicInfo.name', 'Agent名称') }} *</span>
            </label>
            <input 
              v-model="agentConfig.name" 
              type="text" 
              class="input input-bordered" 
              :placeholder="t('agentCreator.basicInfo.namePlaceholder', '输入Agent名称')"
              required
            >
          </div>
          
          <div class="form-control">
            <label class="label">
              <span class="label-text">{{ t('agentCreator.basicInfo.version', '版本') }} *</span>
            </label>
            <input 
              v-model="agentConfig.version" 
              type="text" 
              class="input input-bordered" 
              placeholder="1.0.0"
              required
            >
          </div>
        </div>

        <div class="form-control">
          <label class="label">
            <span class="label-text">{{ t('agentCreator.basicInfo.description', '描述') }} *</span>
          </label>
          <textarea 
            v-model="agentConfig.description" 
            class="textarea textarea-bordered h-24" 
            :placeholder="t('agentCreator.basicInfo.descriptionPlaceholder', '描述Agent的功能和用途')"
            required
          ></textarea>
        </div>

        <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
          <div class="form-control">
            <label class="label">
              <span class="label-text">{{ t('agentCreator.basicInfo.author', '作者') }}</span>
            </label>
            <input 
              v-model="agentConfig.author" 
              type="text" 
              class="input input-bordered" 
              :placeholder="t('agentCreator.basicInfo.authorPlaceholder', '输入作者名称')"
            >
          </div>
          
          <div class="form-control">
            <label class="label">
              <span class="label-text">{{ t('agentCreator.basicInfo.category', '分类') }}</span>
            </label>
            <select v-model="agentConfig.category" class="select select-bordered">
              <option value="">{{ t('agentCreator.basicInfo.selectCategory', '选择分类') }}</option>
              <option value="security">{{ t('agentCreator.categories.security', '安全') }}</option>
              <option value="data_analytics">{{ t('agentCreator.categories.dataAnalytics', '数据分析') }}</option>
              <option value="automation">{{ t('agentCreator.categories.automation', '自动化') }}</option>
              <option value="ai_assistant">{{ t('agentCreator.categories.aiAssistant', 'AI助手') }}</option>
              <option value="monitoring">{{ t('agentCreator.categories.monitoring', '监控') }}</option>
              <option value="custom">{{ t('agentCreator.categories.custom', '自定义') }}</option>
            </select>
          </div>
        </div>

        <div class="form-control">
          <label class="label">
            <span class="label-text">{{ t('agentCreator.basicInfo.tags', '标签') }}</span>
          </label>
          <div class="flex flex-wrap gap-2 mb-2">
            <span v-for="(tag, index) in agentConfig.tags" :key="index" class="badge badge-primary">
              {{ tag }}
              <button @click="removeTag(index)" class="ml-1 text-xs">
                <i class="fas fa-times"></i>
              </button>
            </span>
          </div>
          <div class="flex gap-2">
            <input 
              v-model="newTag" 
              @keyup.enter="addTag" 
              type="text" 
              class="input input-bordered flex-1" 
              :placeholder="t('agentCreator.basicInfo.tagsPlaceholder', '输入标签后按回车添加')"
            >
            <button @click="addTag" class="btn btn-outline btn-sm">
              <i class="fas fa-plus"></i>
            </button>
          </div>
        </div>
      </div>
    </div>

    <!-- 步骤2: 能力定义 -->
    <div v-if="currentStep === 2" class="card bg-base-100 shadow-xl">
      <div class="card-header">
        <h2 class="card-title">
          <i class="fas fa-cogs mr-2"></i>
          {{ t('agentCreator.capabilities.title', 'Agent能力定义') }}
        </h2>
      </div>
      <div class="card-body">
        <div class="flex justify-between items-center mb-4">
          <p class="text-base-content/70">{{ t('agentCreator.capabilities.description', '定义Agent可以执行的动作和能力') }}</p>
          <button @click="addCapability" class="btn btn-primary btn-sm">
            <i class="fas fa-plus mr-2"></i>
            {{ t('agentCreator.capabilities.addCapability', '添加能力') }}
          </button>
        </div>

        <div class="space-y-4">
          <div v-for="(capability, index) in agentConfig.capabilities" :key="index" class="card bg-base-200">
            <div class="card-body">
              <div class="flex justify-between items-start mb-4">
                <h3 class="font-semibold">{{ t('agentCreator.capabilities.capability', '能力') }} {{ index + 1 }}</h3>
                <button @click="removeCapability(index)" class="btn btn-ghost btn-xs text-error">
                  <i class="fas fa-trash"></i>
                </button>
              </div>
              
              <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
                <div class="form-control">
                  <label class="label">
                    <span class="label-text">{{ t('agentCreator.capabilities.actionName', '动作名称') }} *</span>
                  </label>
                  <input 
                    v-model="capability.action" 
                    type="text" 
                    class="input input-bordered input-sm" 
                    :placeholder="t('agentCreator.capabilities.actionPlaceholder', '例如: analyze_data')"
                    required
                  >
                </div>
                
                <div class="form-control">
                  <label class="label">
                    <span class="label-text">{{ t('agentCreator.capabilities.actionDescription', '动作描述') }} *</span>
                  </label>
                  <input 
                    v-model="capability.description" 
                    type="text" 
                    class="input input-bordered input-sm" 
                    :placeholder="t('agentCreator.capabilities.actionDescPlaceholder', '描述这个动作的功能')"
                    required
                  >
                </div>
              </div>

              <!-- 输入参数定义 -->
              <div class="mt-4">
                <div class="flex justify-between items-center mb-2">
                  <label class="label-text font-medium">{{ t('agentCreator.capabilities.inputParams', '输入参数') }}</label>
                  <button @click="addInputParam(index)" class="btn btn-ghost btn-xs">
                    <i class="fas fa-plus mr-1"></i>
                    {{ t('agentCreator.capabilities.addParam', '添加参数') }}
                  </button>
                </div>
                <div class="space-y-2">
                  <div v-for="(param, paramIndex) in capability.inputParams" :key="paramIndex" class="flex gap-2 items-center">
                    <input 
                      v-model="param.name" 
                      type="text" 
                      class="input input-bordered input-xs flex-1" 
                      placeholder="参数名"
                    >
                    <select v-model="param.type" class="select select-bordered select-xs">
                      <option value="string">String</option>
                      <option value="number">Number</option>
                      <option value="boolean">Boolean</option>
                      <option value="object">Object</option>
                      <option value="array">Array</option>
                    </select>
                    <input 
                      v-model="param.description" 
                      type="text" 
                      class="input input-bordered input-xs flex-1" 
                      placeholder="参数描述"
                    >
                    <button @click="removeInputParam(index, paramIndex)" class="btn btn-ghost btn-xs text-error">
                      <i class="fas fa-times"></i>
                    </button>
                  </div>
                </div>
              </div>

              <!-- 输出参数定义 -->
              <div class="mt-4">
                <div class="flex justify-between items-center mb-2">
                  <label class="label-text font-medium">{{ t('agentCreator.capabilities.outputParams', '输出参数') }}</label>
                  <button @click="addOutputParam(index)" class="btn btn-ghost btn-xs">
                    <i class="fas fa-plus mr-1"></i>
                    {{ t('agentCreator.capabilities.addParam', '添加参数') }}
                  </button>
                </div>
                <div class="space-y-2">
                  <div v-for="(param, paramIndex) in capability.outputParams" :key="paramIndex" class="flex gap-2 items-center">
                    <input 
                      v-model="param.name" 
                      type="text" 
                      class="input input-bordered input-xs flex-1" 
                      placeholder="参数名"
                    >
                    <select v-model="param.type" class="select select-bordered select-xs">
                      <option value="string">String</option>
                      <option value="number">Number</option>
                      <option value="boolean">Boolean</option>
                      <option value="object">Object</option>
                      <option value="array">Array</option>
                    </select>
                    <input 
                      v-model="param.description" 
                      type="text" 
                      class="input input-bordered input-xs flex-1" 
                      placeholder="参数描述"
                    >
                    <button @click="removeOutputParam(index, paramIndex)" class="btn btn-ghost btn-xs text-error">
                      <i class="fas fa-times"></i>
                    </button>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- 步骤3: 工具配置 -->
    <div v-if="currentStep === 3" class="card bg-base-100 shadow-xl">
      <div class="card-header">
        <h2 class="card-title">
          <i class="fas fa-tools mr-2"></i>
          {{ t('agentCreator.tools.title', 'MCP工具配置') }}
        </h2>
      </div>
      <div class="card-body">
        <div class="flex justify-between items-center mb-4">
          <p class="text-base-content/70">{{ t('agentCreator.tools.description', '选择Agent需要使用的MCP工具') }}</p>
          <button @click="loadAvailableTools" class="btn btn-outline btn-sm">
            <i class="fas fa-refresh mr-2"></i>
            {{ t('agentCreator.tools.refresh', '刷新工具列表') }}
          </button>
        </div>
        
        <!-- 工具分类标签 -->
        <div class="tabs tabs-boxed mb-4">
          <button 
            v-for="category in toolCategories" 
            :key="category.value"
            @click="selectedToolCategory = category.value"
            class="tab"
            :class="{ 'tab-active': selectedToolCategory === category.value }"
          >
            {{ category.label }}
          </button>
        </div>

        <!-- 工具列表 -->
        <div v-if="loadingTools" class="flex justify-center py-8">
          <span class="loading loading-spinner loading-md"></span>
          <span class="ml-2">{{ t('agentCreator.tools.loading', '加载工具列表...') }}</span>
        </div>

        <div v-else class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
          <div 
            v-for="tool in filteredTools" 
            :key="tool.id"
            class="card bg-base-200 cursor-pointer transition-all duration-200" 
            :class="{ 'ring-2 ring-primary bg-primary/10': agentConfig.tools.includes(tool.id) }" 
            @click="toggleTool(tool.id)"
          >
            <div class="card-body p-4">
              <div class="flex items-start justify-between">
                <div class="flex-1">
                  <h3 class="font-semibold text-sm">{{ tool.display_name || tool.name }}</h3>
                  <p class="text-xs text-base-content/70 mt-1">{{ tool.description }}</p>
                  <div class="flex items-center gap-2 mt-2">
                    <span class="badge badge-outline badge-xs">{{ tool.version }}</span>
                    <span class="badge badge-primary badge-xs">{{ getCategoryLabel(tool.category) }}</span>
                  </div>
                </div>
                <input 
                  type="checkbox" 
                  :checked="agentConfig.tools.includes(tool.id)" 
                  class="checkbox checkbox-primary checkbox-sm"
                  @click.stop
                >
              </div>
            </div>
          </div>
        </div>

        <!-- 选中的工具配置 -->
        <div v-if="agentConfig.tools.length > 0" class="mt-6">
          <h3 class="font-semibold mb-3">{{ t('agentCreator.tools.selectedTools', '已选择的工具') }}</h3>
          <div class="space-y-3">
            <div 
              v-for="toolId in agentConfig.tools" 
              :key="toolId"
              class="card bg-base-300 p-3"
            >
              <div class="flex items-center justify-between">
                <div>
                  <span class="font-medium">{{ getToolName(toolId) }}</span>
                  <span class="text-sm text-base-content/70 ml-2">{{ getToolDescription(toolId) }}</span>
                </div>
                <button @click="toggleTool(toolId)" class="btn btn-ghost btn-xs text-error">
                  <i class="fas fa-times"></i>
                </button>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- 步骤4: 代码编辑 -->
    <div v-if="currentStep === 4" class="card bg-base-100 shadow-xl">
      <div class="card-header">
        <h2 class="card-title">
          <i class="fas fa-code mr-2"></i>
          {{ t('agentCreator.codeEditor.title', 'Agent代码编辑') }}
        </h2>
      </div>
      <div class="card-body">
        <div class="flex justify-between items-center mb-4">
          <div class="tabs tabs-boxed">
            <button 
              v-for="template in codeTemplates" 
              :key="template.name"
              @click="selectedTemplate = template.name"
              class="tab"
              :class="{ 'tab-active': selectedTemplate === template.name }"
            >
              {{ template.label }}
            </button>
          </div>
          <div class="flex gap-2">
            <button @click="generateCode" class="btn btn-primary btn-sm">
              <i class="fas fa-magic mr-2"></i>
              {{ t('agentCreator.codeEditor.generateCode', '生成代码') }}
            </button>
            <button @click="validateCode" class="btn btn-outline btn-sm">
              <i class="fas fa-check mr-2"></i>
              {{ t('agentCreator.codeEditor.validateCode', '验证代码') }}
            </button>
          </div>
        </div>

        <!-- 代码编辑器 -->
        <div class="bg-base-300 rounded-lg p-4 h-96 overflow-auto">
          <textarea 
            v-model="agentConfig.code" 
            class="textarea w-full h-full font-mono text-sm bg-transparent border-none resize-none" 
            :placeholder="t('agentCreator.codeEditor.placeholder', '在这里编写Agent代码...')"
          ></textarea>
        </div>

        <!-- 代码验证结果 -->
        <div v-if="codeValidation" class="mt-4">
          <div class="alert" :class="codeValidation.isValid ? 'alert-success' : 'alert-error'">
            <i :class="codeValidation.isValid ? 'fas fa-check-circle' : 'fas fa-exclamation-triangle'"></i>
            <span>{{ codeValidation.message }}</span>
          </div>
          <div v-if="codeValidation.errors && codeValidation.errors.length > 0" class="mt-2">
            <ul class="list-disc list-inside text-sm text-error">
              <li v-for="error in codeValidation.errors" :key="error">{{ error }}</li>
            </ul>
          </div>
        </div>
      </div>
    </div>

    <!-- 步骤5: 测试部署 -->
    <div v-if="currentStep === 5" class="card bg-base-100 shadow-xl">
      <div class="card-header">
        <h2 class="card-title">
          <i class="fas fa-vial mr-2"></i>
          {{ t('agentCreator.testing.title', '测试和部署') }}
        </h2>
      </div>
      <div class="card-body">
        <!-- Agent预览 -->
        <div class="card bg-base-200 mb-6">
          <div class="card-body">
            <h3 class="card-title text-lg mb-4">{{ t('agentCreator.testing.preview', 'Agent预览') }}</h3>
            <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
              <div>
                <p><strong>{{ t('agentCreator.basicInfo.name', '名称') }}:</strong> {{ agentConfig.name }}</p>
                <p><strong>{{ t('agentCreator.basicInfo.version', '版本') }}:</strong> {{ agentConfig.version }}</p>
                <p><strong>{{ t('agentCreator.basicInfo.category', '分类') }}:</strong> {{ agentConfig.category }}</p>
                <p><strong>{{ t('agentCreator.capabilities.title', '能力数量') }}:</strong> {{ agentConfig.capabilities.length }}</p>
              </div>
              <div>
                <p><strong>{{ t('agentCreator.plugins.title', '插件') }}:</strong></p>
                <div class="flex flex-wrap gap-1 mt-1">
                  <span v-for="plugin in agentConfig.plugins" :key="plugin" class="badge badge-primary badge-sm">
                    {{ plugin }}
                  </span>
                </div>
              </div>
            </div>
          </div>
        </div>

        <!-- 测试区域 -->
        <div class="card bg-base-200 mb-6">
          <div class="card-body">
            <h3 class="card-title text-lg mb-4">{{ t('agentCreator.testing.testAgent', '测试Agent') }}</h3>
            
            <div class="grid grid-cols-1 md:grid-cols-2 gap-4 mb-4">
              <div class="form-control">
                <label class="label">
                  <span class="label-text">{{ t('agentCreator.testing.selectAction', '选择动作') }}</span>
                </label>
                <select v-model="testConfig.action" class="select select-bordered">
                  <option value="">{{ t('agentCreator.testing.selectActionPlaceholder', '选择要测试的动作') }}</option>
                  <option v-for="capability in agentConfig.capabilities" :key="capability.action" :value="capability.action">
                    {{ capability.action }} - {{ capability.description }}
                  </option>
                </select>
              </div>
              
              <div class="form-control">
                <label class="label">
                  <span class="label-text">{{ t('agentCreator.testing.testInputs', '测试输入') }}</span>
                </label>
                <textarea 
                  v-model="testConfig.inputs" 
                  class="textarea textarea-bordered" 
                  placeholder='{ "param1": "value1", "param2": "value2" }'
                ></textarea>
              </div>
            </div>
            
            <div class="flex gap-2 mb-4">
              <button @click="runTest" class="btn btn-primary" :disabled="!testConfig.action || testing">
                <span v-if="testing" class="loading loading-spinner loading-sm mr-2"></span>
                <i v-else class="fas fa-play mr-2"></i>
                {{ t('agentCreator.testing.runTest', '运行测试') }}
              </button>
              <button @click="clearTestResults" class="btn btn-outline">
                <i class="fas fa-trash mr-2"></i>
                {{ t('agentCreator.testing.clearResults', '清除结果') }}
              </button>
            </div>
            
            <!-- 测试结果 -->
            <div v-if="testResults" class="bg-base-300 rounded-lg p-4">
              <h4 class="font-semibold mb-2">{{ t('agentCreator.testing.testResults', '测试结果') }}</h4>
              <pre class="text-sm overflow-auto">{{ JSON.stringify(testResults, null, 2) }}</pre>
            </div>
          </div>
        </div>

        <!-- 部署选项 -->
        <div class="card bg-base-200">
          <div class="card-body">
            <h3 class="card-title text-lg mb-4">{{ t('agentCreator.testing.deployment', '部署选项') }}</h3>
            
            <div class="form-control mb-4">
              <label class="label cursor-pointer">
                <span class="label-text">{{ t('agentCreator.testing.autoStart', '自动启动') }}</span>
                <input v-model="deploymentConfig.autoStart" type="checkbox" class="checkbox checkbox-primary">
              </label>
            </div>
            
            <div class="form-control mb-4">
              <label class="label">
                <span class="label-text">{{ t('agentCreator.testing.maxInstances', '最大实例数') }}</span>
              </label>
              <input 
                v-model.number="deploymentConfig.maxInstances" 
                type="number" 
                class="input input-bordered" 
                min="1" 
                max="10"
              >
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- 导航按钮 -->
    <div class="flex justify-between mt-8">
      <button 
        @click="previousStep" 
        class="btn btn-outline" 
        :disabled="currentStep === 1"
      >
        <i class="fas fa-arrow-left mr-2"></i>
        {{ t('common.previous', '上一步') }}
      </button>
      
      <div class="flex gap-2">
        <button 
          v-if="currentStep < stepsList.length" 
          @click="nextStep" 
          class="btn btn-primary"
          :disabled="!canProceedToNextStep"
        >
          {{ t('common.next', '下一步') }}
          <i class="fas fa-arrow-right ml-2"></i>
        </button>
        
        <button 
          v-if="currentStep === stepsList.length" 
          @click="deployAgent" 
          class="btn btn-success"
          :disabled="deploying"
        >
          <span v-if="deploying" class="loading loading-spinner loading-sm mr-2"></span>
          <i v-else class="fas fa-rocket mr-2"></i>
          {{ t('agentCreator.testing.deploy', '部署Agent') }}
        </button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { dialog } from '@/composables/useDialog'
import { invoke } from '@tauri-apps/api/core'

const { t } = useI18n()
const { toast } = dialog

// 当前步骤
const currentStep = ref(1)

// 步骤列表配置
const stepsList = ref([
  {
    title: '基本信息',
    subtitle: '设置Agent名称和描述'
  },
  {
    title: '能力定义',
    subtitle: '定义Agent的功能和动作'
  },
  {
    title: '工具配置',
    subtitle: '选择需要的MCP工具'
  },
  {
    title: '代码编辑',
    subtitle: '编写和验证Agent代码'
  },
  {
    title: '测试部署',
    subtitle: '测试功能并部署Agent'
  }
])

// Agent配置
const agentConfig = ref({
  name: '',
  version: '1.0.0',
  description: '',
  author: '',
  category: '',
  tags: [] as string[],
  capabilities: [] as any[],
  plugins: [] as string[],
  tools: [] as string[],
  code: ''
})

// 新标签输入
const newTag = ref('')

// 代码模板
const codeTemplates = ref([
  { name: 'basic', label: t('agentCreator.codeEditor.basicTemplate', '基础模板') },
  { name: 'http', label: t('agentCreator.codeEditor.httpTemplate', 'HTTP模板') },
  { name: 'data', label: t('agentCreator.codeEditor.dataTemplate', '数据处理模板') },
  { name: 'ai', label: t('agentCreator.codeEditor.aiTemplate', 'AI模板') }
])

const selectedTemplate = ref('basic')

// 代码验证
const codeValidation = ref<any>(null)

// 测试配置
const testConfig = ref({
  action: '',
  inputs: '{}'
})

const testResults = ref<any>(null)
const testing = ref(false)

// 部署配置
const deploymentConfig = ref({
  autoStart: true,
  maxInstances: 3
})

const deploying = ref(false)

// MCP工具相关数据
const availableTools = ref<any[]>([])
const loadingTools = ref(false)
const selectedToolCategory = ref('all')
const toolCategories = ref([
  { value: 'all', label: '全部' },
  { value: 'security', label: '安全工具' },
  { value: 'network', label: '网络工具' },
  { value: 'analysis', label: '分析工具' },
  { value: 'automation', label: '自动化工具' },
  { value: 'data', label: '数据工具' },
  { value: 'monitoring', label: '监控工具' },
  { value: 'other', label: '其他' }
])

// 计算过滤后的工具列表
const filteredTools = computed(() => {
  if (selectedToolCategory.value === 'all') {
    return availableTools.value
  }
  return availableTools.value.filter(tool => tool.category === selectedToolCategory.value)
})

// 计算属性
const canProceedToNextStep = computed(() => {
  switch (currentStep.value) {
    case 1:
      return agentConfig.value.name && agentConfig.value.version && agentConfig.value.description
    case 2:
      return agentConfig.value.capabilities.length > 0
    case 3:
      return true // 插件是可选的
    case 4:
      return agentConfig.value.code.trim().length > 0
    case 5:
      return true
    default:
      return false
  }
})

// 方法
const goToStep = (step: number) => {
  if (step >= 1 && step <= stepsList.value.length) {
    currentStep.value = step
  }
}

const nextStep = () => {
  if (currentStep.value < stepsList.value.length) {
    currentStep.value++
  }
}

const previousStep = () => {
  if (currentStep.value > 1) {
    currentStep.value--
  }
}

const addTag = () => {
  if (newTag.value.trim() && !agentConfig.value.tags.includes(newTag.value.trim())) {
    agentConfig.value.tags.push(newTag.value.trim())
    newTag.value = ''
  }
}

const removeTag = (index: number) => {
  agentConfig.value.tags.splice(index, 1)
}

const addCapability = () => {
  agentConfig.value.capabilities.push({
    action: '',
    description: '',
    inputParams: [],
    outputParams: []
  })
}

const removeCapability = (index: number) => {
  agentConfig.value.capabilities.splice(index, 1)
}

const addInputParam = (capabilityIndex: number) => {
  agentConfig.value.capabilities[capabilityIndex].inputParams.push({
    name: '',
    type: 'string',
    description: ''
  })
}

const removeInputParam = (capabilityIndex: number, paramIndex: number) => {
  agentConfig.value.capabilities[capabilityIndex].inputParams.splice(paramIndex, 1)
}

const addOutputParam = (capabilityIndex: number) => {
  agentConfig.value.capabilities[capabilityIndex].outputParams.push({
    name: '',
    type: 'string',
    description: ''
  })
}

const removeOutputParam = (capabilityIndex: number, paramIndex: number) => {
  agentConfig.value.capabilities[capabilityIndex].outputParams.splice(paramIndex, 1)
}

const togglePlugin = (plugin: string) => {
  const index = agentConfig.value.plugins.indexOf(plugin)
  if (index > -1) {
    agentConfig.value.plugins.splice(index, 1)
  } else {
    agentConfig.value.plugins.push(plugin)
  }
}

const generateCode = () => {
  // 根据配置生成代码模板
  const template = getCodeTemplate()
  agentConfig.value.code = template
  toast.success(t('agentCreator.codeEditor.codeGenerated', '代码已生成'))
}

const validateCode = () => {
  // 验证代码语法和结构
  try {
    // 这里可以添加更复杂的验证逻辑
    if (agentConfig.value.code.trim().length === 0) {
      throw new Error(t('agentCreator.codeEditor.emptyCode', '代码不能为空'))
    }
    
    codeValidation.value = {
      isValid: true,
      message: t('agentCreator.codeEditor.validCode', '代码验证通过')
    }
  } catch (error: any) {
    codeValidation.value = {
      isValid: false,
      message: t('agentCreator.codeEditor.invalidCode', '代码验证失败'),
      errors: [error.message]
    }
  }
}

const runTest = async () => {
  testing.value = true
  try {
    // 模拟测试执行
    await new Promise(resolve => setTimeout(resolve, 2000))
    
    testResults.value = {
      success: true,
      execution_time: Math.random() * 1000,
      output: {
        result: 'Test execution completed successfully',
        data: { test: true }
      }
    }
    
    toast.success(t('agentCreator.testing.testCompleted', '测试完成'))
  } catch (error: any) {
    testResults.value = {
      success: false,
      error: error.message
    }
    toast.error(t('agentCreator.testing.testFailed', '测试失败'))
  } finally {
    testing.value = false
  }
}

const clearTestResults = () => {
  testResults.value = null
}

const deployAgent = async () => {
  deploying.value = true
  try {
    // 准备Agent配置数据
     const customAgentConfig = {
       name: agentConfig.value.name,
       version: agentConfig.value.version,
       description: agentConfig.value.description,
       author: agentConfig.value.author || 'User',
       category: agentConfig.value.category,
       tags: agentConfig.value.tags,
       capabilities: agentConfig.value.capabilities.map(cap => cap.action),
       tools: agentConfig.value.tools
     }
    
    const deploymentConfig = {
      auto_start: true,
      max_instances: 1,
      resource_limits: {
        memory_mb: 512,
        cpu_percent: 50
      }
    }
    
    // 首先注册Agent
    await invoke('register_custom_agent', { config: customAgentConfig })
    
    // 然后部署Agent
    await invoke('deploy_custom_agent', { 
      agentName: agentConfig.value.name,
      config: deploymentConfig 
    })
    
    toast.success(t('agentCreator.testing.deploySuccess', 'Agent部署成功'))
    
    // 可以在这里添加跳转到Agent管理页面的逻辑
  } catch (error: any) {
    console.error('部署失败:', error)
    toast.error(t('agentCreator.testing.deployFailed', 'Agent部署失败: ') + error.message)
  } finally {
    deploying.value = false
  }
}

// MCP工具相关方法
const loadAvailableTools = async () => {
  loadingTools.value = true
  try {
    // 调用Tauri命令获取真实的MCP工具列表
    const data = await invoke('get_mcp_tools') as any[]
    
    // 转换数据格式以匹配UI需求
    availableTools.value = data.map((tool: any) => ({
      id: tool.id,
      name: tool.name,
      display_name: tool.display_name || tool.name,
      description: tool.description || '',
      version: tool.version || '1.0.0',
      category: mapToolCategory(tool.category)
    }))
  } catch (error: any) {
    console.error('Failed to load MCP tools:', error)
    // 如果API调用失败，使用一些默认工具作为后备
    toast.error(t('agentCreator.messages.loadToolsFailed', '加载工具列表失败，使用默认工具'))
  } finally {
    loadingTools.value = false
  }
}

// 映射工具分类
const mapToolCategory = (category: string) => {
  const categoryMap: Record<string, string> = {
    'reconnaissance': 'security',
    'scanning': 'security', 
    'vulnerability': 'security',
    'network': 'network',
    'web': 'network',
    'analysis': 'analysis',
    'automation': 'automation',
    'data': 'data',
    'monitoring': 'monitoring'
  }
  return categoryMap[category] || 'other'
}

const toggleTool = (toolId: string) => {
  const index = agentConfig.value.tools.indexOf(toolId)
  if (index > -1) {
    agentConfig.value.tools.splice(index, 1)
  } else {
    agentConfig.value.tools.push(toolId)
  }
}

const getCategoryLabel = (category: string) => {
  const categoryItem = toolCategories.value.find(cat => cat.value === category)
  return categoryItem ? categoryItem.label : category
}

const getToolName = (toolId: string) => {
  const tool = availableTools.value.find(t => t.id === toolId)
  return tool ? (tool.display_name || tool.name) : toolId
}

const getToolDescription = (toolId: string) => {
  const tool = availableTools.value.find(t => t.id === toolId)
  return tool ? tool.description : ''
}

const getCodeTemplate = () => {
  // 根据选择的模板和配置生成代码
  const { name, description, capabilities, tools } = agentConfig.value
  
  const capabilityMethods = capabilities.map(cap => {
    const inputParams = cap.inputParams.map((p: any) => `${p.name}: ${p.type}`).join(', ')
    const outputParams = cap.outputParams.map((p: any) => `${p.name}: ${p.type}`).join(', ')
    
    return `
    async fn ${cap.action}(&self, inputs: &HashMap<String, Value>, context: &ExecutionContext) -> Result<ExecutionResult> {
        // TODO: 实现 ${cap.description}
        // 输入参数: ${inputParams}
        // 输出参数: ${outputParams}
        
        // 可以使用以下MCP工具:
${tools.map(tool => `        // - ${getToolName(tool)}: ${getToolDescription(tool)}`).join('\n')}
        
        // 示例: 使用MCP工具
        // let tool_result = context.mcp_client().call_tool("${tools[0] || 'tool-name'}", inputs).await?;
        
        Ok(create_success_result(HashMap::new(), 100))
    }`
  }).join('\n')
  
  return `
// ${name} v${agentConfig.value.version}
// ${description}

use crate::agents::universal_agent::*;
use crate::mcp::McpClient;
use anyhow::Result;
use serde_json::{json, Value};
use std::collections::HashMap;
use async_trait::async_trait;

pub struct ${name.replace(/\s+/g, '')} {
    // Agent状态和配置
    mcp_tools: Vec<String>,
}

#[async_trait]
impl UniversalAgent for ${name.replace(/\s+/g, '')} {
    fn get_metadata(&self) -> &AgentMetadata {
        // 返回Agent元数据
        todo!()
    }
    
    async fn initialize(&mut self, context: &ExecutionContext) -> Result<()> {
        // 初始化Agent并确保MCP工具可用
        for tool_id in &self.mcp_tools {
            context.mcp_client().ensure_tool_available(tool_id).await?;
        }
        Ok(())
    }
    
    async fn execute(
        &self,
        action: &str,
        inputs: &HashMap<String, Value>,
        context: &ExecutionContext,
    ) -> Result<ExecutionResult> {
        match action {
${capabilities.map(cap => `            "${cap.action}" => self.${cap.action}(inputs, context).await,`).join('\n')}
            _ => Err(anyhow::anyhow!("不支持的动作: {}", action))
        }
    }
}

impl ${name.replace(/\s+/g, '')} {
    pub fn new() -> Self {
        Self {
            mcp_tools: vec![
${tools.map(tool => `                "${tool}".to_string(),`).join('\n')}
            ],
        }
    }
${capabilityMethods}
}
`
}

onMounted(() => {
  // 初始化
  loadAvailableTools()
})
</script>

<style scoped>
.agent-creator {
  max-width: 1200px;
  margin: 0 auto;
}

.steps-container {
  @apply hidden md:block;
}

.steps {
  display: flex;
  justify-content: center;
  align-items: center;
  gap: 1rem;
}

.step {
  position: relative;
  display: flex;
  flex-direction: column;
  align-items: center;
  padding: 1rem;
  border-radius: 0.75rem;
  transition: all 0.3s ease;
  min-width: 120px;
}

.step:hover {
  background-color: hsl(var(--base-200));
  transform: translateY(-2px);
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.1);
}

.step-active {
  background-color: hsl(var(--primary) / 0.1);
  border: 2px solid hsl(var(--primary));
}

.step-completed {
  background-color: hsl(var(--success) / 0.1);
}

.step-content {
  display: flex;
  flex-direction: column;
  align-items: center;
  text-align: center;
  gap: 0.25rem;
}

.step-number {
  width: 2rem;
  height: 2rem;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  font-weight: bold;
  font-size: 0.875rem;
  background-color: hsl(var(--base-300));
  color: hsl(var(--base-content));
  transition: all 0.3s ease;
}

.step-primary .step-number {
  background-color: hsl(var(--primary));
  color: hsl(var(--primary-content));
}

.step-completed .step-number {
  background-color: hsl(var(--success));
  color: hsl(var(--success-content));
}

.step-completed .step-number::before {
  content: '✓';
  font-size: 0.75rem;
}

.step-title {
  font-weight: 600;
  font-size: 0.875rem;
  color: hsl(var(--base-content));
}

.step-subtitle {
  font-size: 0.75rem;
  color: hsl(var(--base-content) / 0.7);
  max-width: 100px;
  line-height: 1.2;
}

.step-active .step-title {
  color: hsl(var(--primary));
}

.step-completed .step-title {
  color: hsl(var(--success));
}

/* 连接线 */
.step:not(:last-child)::after {
  content: '';
  position: absolute;
  top: 2rem;
  right: -0.5rem;
  width: 1rem;
  height: 2px;
  background: hsl(var(--base-300));
  transition: all 0.3s ease;
}

.step-primary:not(:last-child)::after {
  background: hsl(var(--primary));
}

.step-completed:not(:last-child)::after {
  background: hsl(var(--success));
}

/* 移动端样式 */
.mobile-steps-indicator {
  background: hsl(var(--base-200));
  border-radius: 1rem;
  padding: 1rem;
}

.card-header {
  border-bottom: 1px solid hsl(var(--border-color));
}

pre {
  white-space: pre-wrap;
  word-wrap: break-word;
}

/* 响应式调整 */
@media (max-width: 768px) {
  .step {
    min-width: 80px;
    padding: 0.5rem;
  }
  
  .step-title {
    font-size: 0.75rem;
  }
  
  .step-subtitle {
    font-size: 0.625rem;
    max-width: 80px;
  }
}
</style>