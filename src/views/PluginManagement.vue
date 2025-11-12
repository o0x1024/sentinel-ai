<template>
  <div class="container mx-auto p-6">
    <div class="mb-6">
      <h1 class="text-3xl font-bold">{{ $t('plugins.title', '插件管理') }}</h1>
      <p class="text-base-content/70 mt-2">{{ $t('plugins.description', '管理和配置安全测试插件') }}</p>
    </div>
    
    <!-- Operation Bar -->
    <div class="flex gap-2 mb-6 flex-wrap">
      <button class="btn btn-primary" @click="openCreateDialog">
        <i class="fas fa-plus mr-2"></i>
        {{ $t('plugins.newPlugin', '新增插件') }}
      </button>
      <button class="btn btn-secondary" @click="openUploadDialog">
        <i class="fas fa-upload mr-2"></i>
        {{ $t('plugins.uploadPlugin', '上传插件') }}
      </button>
      <button class="btn btn-accent" @click="openAIGenerateDialog">
        <i class="fas fa-magic mr-2"></i>
        {{ $t('plugins.aiGenerate', 'AI生成插件') }}
      </button>
      <button class="btn btn-info" @click="refreshPlugins">
        <i class="fas fa-sync-alt mr-2"></i>
        {{ $t('common.refresh', '刷新列表') }}
      </button>
      <!-- <button class="btn btn-outline" @click="scanPluginDirectory">
        <i class="fas fa-folder-open mr-2"></i>
        {{ $t('plugins.scanDirectory', '扫描目录') }}
      </button> -->
    </div>
    
    <!-- Category Filter -->
    <div class="tabs tabs-boxed mb-6 flex-wrap gap-2">
      <button 
        v-for="cat in categories" 
        :key="cat.value"
        class="tab"
        :class="{ 'tab-active': selectedCategory === cat.value }"
        @click="selectedCategory = cat.value"
      >
        <i :class="cat.icon" class="mr-2"></i>
        {{ cat.label }}
        <span v-if="getCategoryCount(cat.value) > 0" class="ml-2 badge badge-sm">
          {{ getCategoryCount(cat.value) }}
        </span>
      </button>
    </div>
    
    <!-- Plugin Manager Content (Merged) -->
    <div class="card bg-base-100 shadow-xl">
      <div class="card-body">
        
        <!-- Plugin List -->
        <div v-if="filteredPlugins.length === 0" class="alert alert-info">
          <i class="fas fa-info-circle"></i>
          <span>{{ $t('plugins.noPlugins', '暂无插件，请上传或扫描插件目录') }}</span>
        </div>
        
        <div v-else class="overflow-x-auto">
          <table class="table table-zebra w-full">
            <thead>
              <tr>
                <th class="w-12">{{ $t('common.status', '状态') }}</th>
                <th>{{ $t('plugins.pluginName', '插件名称') }}</th>
                <th class="w-24">{{ $t('plugins.version', '版本') }}</th>
                <th class="w-16 text-center">{{ $t('plugins.category', '分类') }}</th>
                <th class="w-32">{{ $t('plugins.author', '作者') }}</th>
                <th class="w-48">{{ $t('plugins.tags', '标签') }}</th>
                <th class="w-64">{{ $t('common.actions', '操作') }}</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="plugin in filteredPlugins" :key="plugin.metadata.id">
                <!-- Status Indicator -->
                <td>
                  <div class="flex items-center gap-2">
                    <div 
                      class="tooltip" 
                      :data-tip="getStatusText(plugin.status)"
                    >
                      <div
                        class="w-3 h-3 rounded-full"
                        :class="{
                          'bg-success': plugin.status === 'Enabled',
                          'bg-warning': plugin.status === 'Disabled',
                          'bg-error': plugin.status === 'Error'
                        }"
                      ></div>
                    </div>
                  </div>
                </td>
                
                <!-- Plugin Name -->
                <td>
                  <div class="font-bold">{{ plugin.metadata.name }}</div>
                  <div class="text-sm text-gray-500">{{ plugin.metadata.id }}</div>
                  <div v-if="plugin.metadata.description" class="text-xs text-gray-400 mt-1">
                    {{ plugin.metadata.description }}
                  </div>
                </td>
                
                <!-- Version -->
                <td>
                  <span class="badge badge-outline">{{ plugin.metadata.version }}</span>
                </td>
                
                <!-- Category -->
                <td class="text-center">
                  <div class="tooltip" :data-tip="getCategoryLabel(plugin.metadata.category)">
                    <i 
                      :class="getCategoryIcon(plugin.metadata.category)"
                      class="text-primary text-lg"
                    ></i>
                  </div>
                </td>
                
                <!-- Author -->
                <td>{{ plugin.metadata.author || '-' }}</td>
                
                <!-- Tags -->
                <td>
                  <div class="flex flex-wrap gap-1 max-w-xs">
                    <span 
                      v-for="(tag, idx) in plugin.metadata.tags.slice(0, 3)" 
                      :key="tag" 
                      class="badge badge-sm badge-ghost whitespace-nowrap"
                    >
                      {{ tag }}
                    </span>
                    <span 
                      v-if="plugin.metadata.tags.length > 3"
                      class="badge badge-sm badge-outline tooltip"
                      :data-tip="plugin.metadata.tags.slice(3).join(', ')"
                    >
                      +{{ plugin.metadata.tags.length - 3 }}
                    </span>
                  </div>
                </td>
                
                <!-- Action Buttons -->
                <td>
                  <div class="flex gap-1 flex-wrap">
                    <!-- Test Plugin - 显示不同的提示信息 -->
                    <div class="tooltip" :data-tip="isAgentPluginType(plugin) ? '测试 Agent 工具 (analyze)' : '测试被动扫描 (scan_request/scan_response)'">
                      <button
                        class="btn btn-sm btn-outline"
                        @click="testPlugin(plugin)"
                        :disabled="plugin.status !== 'Enabled'"
                      >
                        <i class="fas fa-vial mr-1"></i>
                        {{ $t('plugins.test', '测试') }}
                      </button>
                    </div>

                    <!-- Advanced Test - 仅对被动扫描插件显示 -->
                    <div 
                      v-if="isPassiveScanPluginType(plugin)"
                      class="tooltip" 
                      data-tip="高级并发测试 (仅被动扫描)"
                    >
                      <button
                        class="btn btn-sm btn-outline"
                        @click="openAdvancedDialog(plugin)"
                        :disabled="plugin.status !== 'Enabled'"
                      >
                        <i class="fas fa-gauge-high mr-1"></i>
                        {{ $t('plugins.advancedTest', '高级') }}
                      </button>
                    </div>
                    
                    <!-- Enable/Disable Toggle -->
                    <button
                      class="btn btn-sm"
                      :class="plugin.status === 'Enabled' ? 'btn-warning' : 'btn-success'"
                      @click="togglePlugin(plugin)"
                    >
                      <i 
                        :class="plugin.status === 'Enabled' ? 'fas fa-pause' : 'fas fa-play'"
                        class="mr-1"
                      ></i>
                      {{ plugin.status === 'Enabled' ? $t('plugins.disable', '禁用') : $t('plugins.enable', '启用') }}
                    </button>
                    
                    <!-- View/Edit Code -->
                    <button 
                      class="btn btn-sm btn-info"
                      @click="viewPluginCode(plugin)"
                    >
                      <i class="fas fa-code mr-1"></i>
                      {{ $t('plugins.code', '代码') }}
                    </button>
                    
                    <!-- Delete -->
                    <button 
                      class="btn btn-sm btn-error"
                      @click="confirmDeletePlugin(plugin)"
                    >
                      <i class="fas fa-trash"></i>
                    </button>
                  </div>
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>
    </div>
    
    <!-- Upload Plugin Dialog -->
    <dialog ref="uploadDialog" class="modal">
      <div class="modal-box">
        <h3 class="font-bold text-base mb-4">{{ $t('plugins.uploadPlugin', '上传插件') }}</h3>
        
        <div class="form-control w-full">
          <label class="label">
            <span class="label-text">{{ $t('plugins.selectFile', '选择插件文件 (.ts / .js)') }}</span>
          </label>
          <input 
            type="file" 
            class="file-input file-input-bordered w-full"
            accept=".ts,.js"
            @change="handleFileSelect"
            ref="fileInput"
          />
        </div>
        
        <div v-if="uploadError" class="alert alert-error mt-4">
          <i class="fas fa-exclamation-circle"></i>
          <span>{{ uploadError }}</span>
        </div>
        
        <div class="modal-action">
          <button class="btn" @click="closeUploadDialog">{{ $t('common.cancel', '取消') }}</button>
          <button 
            class="btn btn-primary" 
            :disabled="!selectedFile || uploading"
            @click="uploadPlugin"
          >
            <span v-if="uploading" class="loading loading-spinner"></span>
            {{ uploading ? $t('plugins.uploading', '上传中...') : $t('plugins.upload', '上传') }}
          </button>
        </div>
      </div>
      <form method="dialog" class="modal-backdrop">
        <button @click="closeUploadDialog">close</button>
      </form>
    </dialog>
    
    <!-- Code Editor Dialog -->
    <dialog ref="codeEditorDialog" class="modal">
      <div class="modal-box w-11/12 max-w-5xl">
        <h3 class="font-bold text-base mb-4">
          {{ editingPlugin ? $t('plugins.codeEditor', '插件代码编辑器') : $t('plugins.newPlugin', '新增插件') }}
          <span v-if="editingPlugin" class="text-sm font-normal text-gray-500 ml-2">
            {{ editingPlugin.metadata.name }} ({{ editingPlugin.metadata.id }})
          </span>
        </h3>
        
        <!-- Plugin Metadata Form (for both new and editing) -->
        <div class="grid grid-cols-2 gap-4 mb-4">
          <div class="form-control">
            <label class="label">
              <span class="label-text">{{ $t('plugins.pluginId', '插件ID') }} <span class="text-error">*</span></span>
            </label>
            <input 
              v-model="newPluginMetadata.id"
              type="text" 
              :placeholder="$t('plugins.pluginIdPlaceholder', '例如: sql_injection_scanner')"
              class="input input-bordered input-sm"
              :disabled="!!editingPlugin"
            />
          </div>
          
          <div class="form-control">
            <label class="label">
              <span class="label-text">{{ $t('plugins.pluginName', '插件名称') }} <span class="text-error">*</span></span>
            </label>
            <input 
              v-model="newPluginMetadata.name"
              type="text" 
              :placeholder="$t('plugins.pluginNamePlaceholder', '例如: SQL注入扫描器')"
              class="input input-bordered input-sm"
              :disabled="editingPlugin && !isEditing"
            />
          </div>
          
          <div class="form-control">
            <label class="label">
              <span class="label-text">{{ $t('plugins.version', '版本') }}</span>
            </label>
            <input 
              v-model="newPluginMetadata.version"
              type="text" 
              placeholder="1.0.0"
              class="input input-bordered input-sm"
              :disabled="editingPlugin && !isEditing"
            />
          </div>
          
          <div class="form-control">
            <label class="label">
              <span class="label-text">{{ $t('plugins.author', '作者') }}</span>
            </label>
            <input 
              v-model="newPluginMetadata.author"
              type="text" 
              :placeholder="$t('plugins.authorPlaceholder', '作者名称')"
              class="input input-bordered input-sm"
              :disabled="editingPlugin && !isEditing"
            />
          </div>
          
          <div class="form-control">
            <label class="label">
              <span class="label-text">{{ $t('plugins.mainCategory', '主分类') }} <span class="text-error">*</span></span>
            </label>
            <select 
              v-model="newPluginMetadata.mainCategory" 
              class="select select-bordered select-sm"
              :disabled="editingPlugin && !isEditing"
            >
              <option 
                v-for="cat in mainCategories" 
                :key="cat.value" 
                :value="cat.value"
              >
                {{ cat.label }}
              </option>
            </select>
          </div>
          
          <div class="form-control">
            <label class="label">
              <span class="label-text">{{ $t('plugins.subCategory', '子分类') }} <span class="text-error">*</span></span>
            </label>
            <select 
              v-model="newPluginMetadata.category" 
              class="select select-bordered select-sm"
              :disabled="editingPlugin && !isEditing"
            >
              <option 
                v-for="cat in subCategories" 
                :key="cat.value" 
                :value="cat.value"
              >
                {{ cat.label }}
              </option>
            </select>
          </div>
          
          <div class="form-control">
            <label class="label">
              <span class="label-text">{{ $t('plugins.defaultSeverity', '默认严重程度') }}</span>
            </label>
            <select 
              v-model="newPluginMetadata.default_severity" 
              class="select select-bordered select-sm"
              :disabled="editingPlugin && !isEditing"
            >
              <option value="info">{{ $t('common.info', '信息') }}</option>
              <option value="low">{{ $t('common.low', '低危') }}</option>
              <option value="medium">{{ $t('common.medium', '中危') }}</option>
              <option value="high">{{ $t('common.high', '高危') }}</option>
              <option value="critical">{{ $t('common.critical', '严重') }}</option>
            </select>
          </div>
          
          <div class="form-control col-span-2">
            <label class="label">
              <span class="label-text">{{ $t('common.description', '描述') }}</span>
            </label>
            <input 
              v-model="newPluginMetadata.description"
              type="text" 
              :placeholder="$t('plugins.descriptionPlaceholder', '插件功能描述')"
              class="input input-bordered input-sm"
              :disabled="editingPlugin && !isEditing"
            />
          </div>
          
          <div class="form-control col-span-2">
            <label class="label">
              <span class="label-text">{{ $t('plugins.tags', '标签') }} ({{ $t('plugins.commaSeparated', '逗号分隔') }})</span>
            </label>
            <input 
              v-model="newPluginMetadata.tagsString"
              type="text" 
              :placeholder="$t('plugins.tagsPlaceholder', '例如: security, scanner, sql')"
              class="input input-bordered input-sm"
              :disabled="editingPlugin && !isEditing"
            />
          </div>
        </div>
        
        <!-- Code Editor -->
        <div class="form-control w-full">
          <div class="flex justify-between items-center mb-2">
            <label class="label">
              <span class="label-text">{{ $t('plugins.pluginCode', '插件代码') }}</span>
            </label>
            <div class="flex gap-2">
              <button 
                v-if="!editingPlugin"
                class="btn btn-xs btn-outline"
                @click="insertTemplate"
              >
                <i class="fas fa-file-code mr-1"></i>
                {{ $t('plugins.insertTemplate', '插入模板') }}
              </button>
              <button 
                class="btn btn-xs btn-outline"
                @click="formatCode"
              >
                <i class="fas fa-indent mr-1"></i>
                {{ $t('plugins.format', '格式化') }}
              </button>
            </div>
          </div>
          <textarea 
            v-model="pluginCode"
            class="textarea textarea-bordered font-mono text-sm h-96 w-full"
            :readonly="editingPlugin && !isEditing"
            spellcheck="false"
            :placeholder="$t('plugins.codePlaceholder', '在此输入或粘贴插件代码...')"
          ></textarea>
        </div>
        
        <div v-if="codeError" class="alert alert-error mt-4">
          <i class="fas fa-exclamation-circle"></i>
          <span>{{ codeError }}</span>
        </div>
        
        <div class="modal-action">
          <button class="btn" @click="closeCodeEditorDialog">{{ $t('common.close', '关闭') }}</button>
          
          <!-- Edit Mode Buttons -->
          <template v-if="editingPlugin">
            <button 
              v-if="!isEditing"
              class="btn btn-primary"
              @click="enableEditing"
            >
              <i class="fas fa-edit mr-2"></i>
              {{ $t('common.edit', '编辑') }}
            </button>
            <template v-else>
              <button class="btn btn-warning" @click="cancelEditing">{{ $t('plugins.cancelEdit', '取消编辑') }}</button>
              <button 
                class="btn btn-success"
                :disabled="saving"
                @click="savePluginCode"
              >
                <span v-if="saving" class="loading loading-spinner"></span>
                {{ saving ? $t('common.saving', '保存中...') : $t('common.save', '保存') }}
              </button>
            </template>
          </template>
          
          <!-- New Plugin Mode Buttons -->
          <template v-else>
            <button 
              class="btn btn-success"
              :disabled="saving || !isNewPluginValid"
              @click="createNewPlugin"
            >
              <span v-if="saving" class="loading loading-spinner"></span>
              {{ saving ? $t('plugins.creating', '创建中...') : $t('plugins.createPlugin', '创建插件') }}
            </button>
          </template>
        </div>
      </div>
      <form method="dialog" class="modal-backdrop">
        <button @click="closeCodeEditorDialog">close</button>
      </form>
    </dialog>
    
    <!-- Delete Confirmation Dialog -->
    <dialog ref="deleteDialog" class="modal">
      <div class="modal-box">
        <h3 class="font-bold text-base">{{ $t('plugins.confirmDelete', '确认删除') }}</h3>
        <p class="py-4">
          {{ $t('plugins.deleteConfirmText', '确定要删除插件') }} <strong>{{ deletingPlugin?.metadata.name }}</strong> {{ $t('plugins.deleteWarning', '吗？此操作不可撤销。') }}
        </p>
        
        <div class="modal-action">
          <button class="btn" @click="closeDeleteDialog">{{ $t('common.cancel', '取消') }}</button>
          <button 
            class="btn btn-error"
            :disabled="deleting"
            @click="deletePlugin"
          >
            <span v-if="deleting" class="loading loading-spinner"></span>
            {{ deleting ? $t('plugins.deleting', '删除中...') : $t('common.delete', '删除') }}
          </button>
        </div>
      </div>
      <form method="dialog" class="modal-backdrop">
        <button @click="closeDeleteDialog">close</button>
      </form>
    </dialog>
    
    <!-- AI Generate Plugin Dialog -->
    <dialog ref="aiGenerateDialog" class="modal">
      <div class="modal-box w-11/12 max-w-3xl">
        <h3 class="font-bold text-base mb-4">
          <i class="fas fa-magic mr-2"></i>
          {{ $t('plugins.aiGenerate', 'AI生成插件') }}
        </h3>
        
        <div class="space-y-4">
          <div class="form-control">
            <label class="label">
              <span class="label-text">{{ $t('plugins.aiPrompt', '描述你想要的插件功能') }}</span>
            </label>
            <textarea 
              v-model="aiPrompt"
              class="textarea textarea-bordered h-32"
              :placeholder="$t('plugins.aiPromptPlaceholder', '例如：我需要一个检测SQL注入漏洞的插件，能够分析HTTP请求中的参数，识别潜在的SQL注入模式...')"
            ></textarea>
          </div>
          
          <div class="grid grid-cols-2 gap-4">
            <div class="form-control">
              <label class="label">
                <span class="label-text">{{ $t('plugins.pluginType', '插件类型') }}</span>
              </label>
              <select v-model="aiPluginType" class="select select-bordered select-sm">
                <option value="passiveScan">{{ $t('plugins.categories.passiveScan', '被动扫描插件') }}</option>
                <option value="agentTools">{{ $t('plugins.categories.agentTools', 'Agent工具插件') }}</option>
                <!-- <option value="builtinTools">{{ $t('plugins.categories.builtinTools', '内置工具插件') }}</option>
                <option value="mcpTools">{{ $t('plugins.categories.mcpTools', 'MCP工具插件') }}</option>
                <option value="vulnerability">{{ $t('plugins.categories.vulnerability', '漏洞扫描') }}</option>
                <option value="injection">{{ $t('plugins.categories.injection', '注入检测') }}</option>
                <option value="xss">{{ $t('plugins.categories.xss', '跨站脚本') }}</option> -->
                <option value="custom">{{ $t('plugins.categories.custom', '自定义') }}</option>
              </select>
            </div>
            
            <div class="form-control">
              <label class="label">
                <span class="label-text">{{ $t('plugins.severity', '严重程度') }}</span>
              </label>
              <select v-model="aiSeverity" class="select select-bordered select-sm">
                <option value="info">{{ $t('common.info', '信息') }}</option>
                <option value="low">{{ $t('common.low', '低危') }}</option>
                <option value="medium">{{ $t('common.medium', '中危') }}</option>
                <option value="high">{{ $t('common.high', '高危') }}</option>
                <option value="critical">{{ $t('common.critical', '严重') }}</option>
              </select>
            </div>
          </div>
          
          <div v-if="aiGenerating" class="alert alert-info">
            <span class="loading loading-spinner"></span>
            <span>{{ $t('plugins.aiGenerating', 'AI正在生成插件代码，请稍候...') }}</span>
          </div>
          
          <div v-if="aiGenerateError" class="alert alert-error">
            <i class="fas fa-exclamation-circle"></i>
            <span>{{ aiGenerateError }}</span>
          </div>
        </div>
        
        <div class="modal-action">
          <button class="btn" @click="closeAIGenerateDialog">{{ $t('common.cancel', '取消') }}</button>
          <button 
            class="btn btn-primary"
            :disabled="!aiPrompt.trim() || aiGenerating"
            @click="generatePluginWithAI"
          >
            <i class="fas fa-magic mr-2"></i>
            {{ $t('plugins.generatePlugin', '生成插件') }}
          </button>
        </div>
      </div>
      <form method="dialog" class="modal-backdrop">
        <button @click="closeAIGenerateDialog">close</button>
      </form>
    </dialog>
    
    <!-- Test Result Dialog -->
    <dialog ref="testResultDialog" class="modal">
      <div class="modal-box w-11/12 max-w-3xl">
        <h3 class="font-bold text-base mb-4">
          <i class="fas fa-vial mr-2"></i>
          {{ $t('plugins.testResult', '插件测试结果') }}
          <span v-if="testResult" class="text-sm font-normal text-gray-500 ml-2">
            ({{ getTestTypeLabel() }})
          </span>
        </h3>
        
        <div v-if="testing" class="alert alert-info">
          <span class="loading loading-spinner"></span>
          <span>{{ $t('plugins.testing', '正在测试插件...') }}</span>
        </div>
        
        <div v-else-if="testResult" class="space-y-4">
          <div class="alert" :class="{
            'alert-success': testResult.success,
            'alert-error': !testResult.success
          }">
            <i :class="testResult.success ? 'fas fa-check-circle' : 'fas fa-times-circle'"></i>
            <span>{{ testResult.success ? $t('plugins.testPassed', '测试通过') : $t('plugins.testFailed', '测试失败') }}</span>
          </div>
          
          <div v-if="testResult.message" class="card bg-base-200">
            <div class="card-body">
              <h4 class="font-semibold mb-2">{{ $t('plugins.testMessage', '测试消息') }}</h4>
              <pre class="text-sm whitespace-pre-wrap">{{ testResult.message }}</pre>
            </div>
          </div>
          
          <div v-if="testResult.findings && testResult.findings.length > 0" class="card bg-base-200">
            <div class="card-body">
              <h4 class="font-semibold mb-2">{{ $t('plugins.findings', '发现的问题') }} ({{ testResult.findings.length }})</h4>
              <div class="space-y-2">
                <div v-for="(finding, idx) in testResult.findings" :key="idx" class="card bg-base-100">
                  <div class="card-body p-3">
                    <div class="flex justify-between items-start">
                      <span class="font-medium">{{ finding.title }}</span>
                      <span class="badge" :class="{
                        'badge-error': finding.severity === 'critical' || finding.severity === 'high',
                        'badge-warning': finding.severity === 'medium',
                        'badge-info': finding.severity === 'low' || finding.severity === 'info'
                      }">{{ finding.severity }}</span>
                    </div>
                    <p class="text-sm text-base-content/70 mt-1">{{ finding.description }}</p>
                  </div>
                </div>
              </div>
            </div>
          </div>
          
          <div v-if="testResult.error" class="alert alert-error">
            <i class="fas fa-exclamation-circle"></i>
            <span>{{ testResult.error }}</span>
          </div>
        </div>
        
        <div class="modal-action">
          <button class="btn" @click="closeTestResultDialog">{{ $t('common.close', '关闭') }}</button>
        </div>
      </div>
      <form method="dialog" class="modal-backdrop">
        <button @click="closeTestResultDialog">close</button>
      </form>
    </dialog>

    <!-- Advanced Test Dialog -->
    <dialog ref="advancedDialog" class="modal">
      <div class="modal-box w-11/12 max-w-5xl">
        <h3 class="font-bold text-base mb-4">
          <i class="fas fa-gauge-high mr-2"></i>
          {{ $t('plugins.advancedTest', '高级测试') }}
          <span v-if="advancedPlugin" class="text-sm font-normal text-gray-500 ml-2">
            {{ advancedPlugin.metadata.name }} ({{ advancedPlugin.metadata.id }})
          </span>
        </h3>

        <div class="grid grid-cols-2 gap-4">
          <div class="form-control col-span-2">
            <label class="label">
              <span class="label-text">{{ $t('plugins.requestUrl', '请求 URL') }}</span>
            </label>
            <input v-model="advancedForm.url" type="text" class="input input-bordered input-sm w-full" placeholder="https://example.com/test" />
          </div>

          <div class="form-control">
            <label class="label">
              <span class="label-text">{{ $t('plugins.httpMethod', 'HTTP 方法') }}</span>
            </label>
            <select v-model="advancedForm.method" class="select select-bordered select-sm w-full">
              <option>GET</option>
              <option>POST</option>
              <option>PUT</option>
              <option>DELETE</option>
              <option>PATCH</option>
              <option>HEAD</option>
              <option>OPTIONS</option>
            </select>
          </div>

          <div class="form-control">
            <label class="label">
              <span class="label-text">{{ $t('plugins.runs', '运行次数') }}</span>
            </label>
            <input v-model.number="advancedForm.runs" type="number" min="1" class="input input-bordered input-sm w-full" />
          </div>

          <div class="form-control">
            <label class="label">
              <span class="label-text">{{ $t('plugins.concurrency', '并发数') }}</span>
            </label>
            <input v-model.number="advancedForm.concurrency" type="number" min="1" class="input input-bordered input-sm w-full" />
          </div>

          <div class="form-control col-span-2">
            <label class="label">
              <span class="label-text">{{ $t('plugins.headersJson', '请求头 (JSON)') }}</span>
            </label>
            <textarea v-model="advancedForm.headersText" class="textarea textarea-bordered font-mono text-xs h-24" placeholder='{"User-Agent":"Sentinel-AdvTest/1.0"}'></textarea>
            <label class="label">
              <span class="label-text-alt">{{ $t('plugins.headersHint', '留空则使用默认 UA。必须是有效 JSON。') }}</span>
            </label>
          </div>

          <div class="form-control col-span-2">
            <label class="label">
              <span class="label-text">{{ $t('plugins.body', '请求体') }}</span>
            </label>
            <textarea v-model="advancedForm.bodyText" class="textarea textarea-bordered font-mono text-xs h-24" placeholder=""></textarea>
          </div>
        </div>

        <div v-if="advancedError" class="alert alert-error mt-4">
          <i class="fas fa-exclamation-circle"></i>
          <span>{{ advancedError }}</span>
        </div>

        <div class="mt-4">
          <button class="btn btn-primary" :disabled="advancedTesting" @click="runAdvancedTest">
            <span v-if="advancedTesting" class="loading loading-spinner"></span>
            {{ advancedTesting ? $t('plugins.testing', '正在测试...') : $t('plugins.startTest', '开始测试') }}
          </button>
        </div>

        <div v-if="advancedTesting" class="alert alert-info mt-4">
          <span class="loading loading-spinner"></span>
          <span class="ml-2">{{ $t('plugins.testing', '正在测试插件...') }}</span>
        </div>

        <div v-else-if="advancedResult" class="space-y-4 mt-4">
          <!-- Summary -->
          <div class="stats shadow w-full">
            <div class="stat">
              <div class="stat-title">{{ $t('plugins.totalRuns', '总运行') }}</div>
              <div class="stat-value text-primary">{{ advancedResult.total_runs }}</div>
              <div class="stat-desc">{{ $t('plugins.concurrency', '并发') }}: {{ advancedResult.concurrency }}</div>
            </div>
            <div class="stat">
              <div class="stat-title">{{ $t('plugins.totalDuration', '总耗时(ms)') }}</div>
              <div class="stat-value">{{ advancedResult.total_duration_ms }}</div>
              <div class="stat-desc">{{ $t('plugins.avgPerRun', '平均/次(ms)') }}: {{ advancedResult.avg_duration_ms.toFixed(1) }}</div>
            </div>
            <div class="stat">
              <div class="stat-title">{{ $t('plugins.findingsTotal', '发现数') }}</div>
              <div class="stat-value text-secondary">{{ advancedResult.total_findings }}</div>
              <div class="stat-desc">{{ $t('plugins.unique', '唯一') }}: {{ advancedResult.unique_findings }}</div>
            </div>
          </div>

          <!-- Per-run table -->
          <div class="card bg-base-200">
            <div class="card-body">
              <h4 class="font-semibold mb-2">{{ $t('plugins.runDetails', '运行详情') }}</h4>
              <div class="overflow-x-auto">
                <table class="table table-zebra w-full">
                  <thead>
                    <tr>
                      <th>#</th>
                      <th>{{ $t('plugins.duration', '耗时(ms)') }}</th>
                      <th>{{ $t('plugins.findings', '发现') }}</th>
                      <th>{{ $t('plugins.error', '错误') }}</th>
                    </tr>
                  </thead>
                  <tbody>
                    <tr v-for="r in sortedRuns" :key="r.run_index">
                      <td>{{ r.run_index }}</td>
                      <td>{{ r.duration_ms }}</td>
                      <td>{{ r.findings }}</td>
                      <td>
                        <span v-if="r.error" class="badge badge-error">{{ r.error }}</span>
                        <span v-else class="badge badge-success">OK</span>
                      </td>
                    </tr>
                  </tbody>
                </table>
              </div>
            </div>
          </div>

          <!-- Unique Findings List -->
          <div v-if="advancedResult.findings && advancedResult.findings.length > 0" class="card bg-base-200">
            <div class="card-body">
              <h4 class="font-semibold mb-2">{{ $t('plugins.uniqueFindings', '唯一发现') }} ({{ advancedResult.findings.length }})</h4>
              <div class="space-y-2">
                <div v-for="(finding, idx) in advancedResult.findings" :key="idx" class="card bg-base-100">
                  <div class="card-body p-3">
                    <div class="flex justify-between items-start">
                      <span class="font-medium">{{ finding.title }}</span>
                      <span class="badge" :class="{
                        'badge-error': finding.severity === 'critical' || finding.severity === 'high',
                        'badge-warning': finding.severity === 'medium',
                        'badge-info': finding.severity === 'low' || finding.severity === 'info'
                      }">{{ finding.severity }}</span>
                    </div>
                    <p class="text-sm text-base-content/70 mt-1">{{ finding.description }}</p>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>

        <div class="modal-action">
          <button class="btn" @click="closeAdvancedDialog">{{ $t('common.close', '关闭') }}</button>
        </div>
      </div>
      <form method="dialog" class="modal-backdrop">
        <button @click="closeAdvancedDialog">close</button>
      </form>
    </dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { readFile } from '@tauri-apps/plugin-fs'

const { t } = useI18n()

// Plugin Record Types
interface PluginMetadata {
  id: string
  name: string
  version: string
  author?: string
  category: string
  default_severity: string
  tags: string[]
  description?: string
}

interface PluginRecord {
  metadata: PluginMetadata
  path: string
  status: 'Enabled' | 'Disabled' | 'Error'
  last_error?: string
}

interface CommandResponse<T> {
  success: boolean
  data?: T
  error?: string
}

interface TestResult {
  success: boolean
  message?: string
  findings?: Array<{
    title: string
    description: string
    severity: string
  }>
  error?: string
}

interface AdvancedRunStat {
  run_index: number
  duration_ms: number
  findings: number
  error?: string | null
}

interface AdvancedTestResult {
  plugin_id: string
  success: boolean
  total_runs: number
  concurrency: number
  total_duration_ms: number
  avg_duration_ms: number
  total_findings: number
  unique_findings: number
  findings: Array<{ title: string; description: string; severity: string }>
  runs: AdvancedRunStat[]
  message?: string
  error?: string
}

// Component State
const selectedCategory = ref('all')
const plugins = ref<PluginRecord[]>([])
const uploadDialog = ref<HTMLDialogElement>()
const codeEditorDialog = ref<HTMLDialogElement>()
const deleteDialog = ref<HTMLDialogElement>()
const aiGenerateDialog = ref<HTMLDialogElement>()
const testResultDialog = ref<HTMLDialogElement>()
const advancedDialog = ref<HTMLDialogElement>()
const fileInput = ref<HTMLInputElement>()

const selectedFile = ref<File | null>(null)
const uploading = ref(false)
const uploadError = ref('')

const editingPlugin = ref<PluginRecord | null>(null)
const pluginCode = ref('')
const originalCode = ref('')
const isEditing = ref(false)
const saving = ref(false)
const codeError = ref('')

const deletingPlugin = ref<PluginRecord | null>(null)
const deleting = ref(false)

// New Plugin Related State
const newPluginMetadata = ref({
  id: '',
  name: '',
  version: '1.0.0',
  author: '',
  mainCategory: 'passive', // 大分类: passive (被动扫描) 或 agent (Agent插件)
  category: 'vulnerability', // 子分类
  default_severity: 'medium',
  description: '',
  tagsString: ''
})

// 大分类定义
const mainCategories = [
  { value: 'passive', label: '被动扫描插件', icon: 'fas fa-shield-alt' },
  { value: 'agent', label: 'Agent插件', icon: 'fas fa-robot' }
]

// 被动扫描插件的所有子分类
const passiveScanCategories = [
  'vulnerability', 'injection', 'xss', 'sensitiveInfo', 'authBypass', 'custom'
]

// Agent 插件的所有子分类
const agentToolsCategories = [
  'scanner', 'analyzer', 'reporter', 'recon', 'exploit', 'utility', 'custom', 'agentTools'
]

// 子分类定义（根据大分类动态变化）
const subCategories = computed(() => {
  if (newPluginMetadata.value.mainCategory === 'passive') {
    return [
      { value: 'vulnerability', label: '漏洞检测', icon: 'fas fa-bug' },
      { value: 'injection', label: '注入检测', icon: 'fas fa-syringe' },
      { value: 'xss', label: '跨站脚本', icon: 'fas fa-code' },
      { value: 'sensitiveInfo', label: '敏感信息', icon: 'fas fa-eye-slash' },
      { value: 'authBypass', label: '认证绕过', icon: 'fas fa-unlock' },
      { value: 'custom', label: '自定义', icon: 'fas fa-wrench' }
    ]
  } else if (newPluginMetadata.value.mainCategory === 'agent') {
    return [
      { value: 'scanner', label: '扫描工具', icon: 'fas fa-radar' },
      { value: 'analyzer', label: '分析工具', icon: 'fas fa-microscope' },
      { value: 'reporter', label: '报告工具', icon: 'fas fa-file-alt' },
      { value: 'recon', label: '信息收集', icon: 'fas fa-search' },
      { value: 'exploit', label: '漏洞利用', icon: 'fas fa-bomb' },
      { value: 'utility', label: '实用工具', icon: 'fas fa-toolbox' },
      { value: 'custom', label: '自定义', icon: 'fas fa-wrench' }
    ]
  }
  return []
})

// 根据旧的category值推断mainCategory（用于向后兼容）
const inferMainCategory = (category: string): string => {
  // agentTools 相关
  if (category === 'agentTools' || category === 'scanner' || category === 'analyzer' || 
      category === 'reporter' || category === 'recon' || category === 'exploit' || category === 'utility') {
    return 'agent'
  }
  // 被动扫描相关（默认）
  return 'passive'
}

// 将旧的category值转换为新的子分类
const convertToSubCategory = (category: string): string => {
  // 如果是agentTools，转为scanner
  if (category === 'agentTools') return 'scanner'
  // 如果是passiveScan，转为vulnerability  
  if (category === 'passiveScan') return 'vulnerability'
  // 其他保持不变
  return category
}

// AI Generation Related State
const aiPrompt = ref('')
const aiPluginType = ref('vulnerability')
const aiSeverity = ref('medium')
const aiGenerating = ref(false)
const aiGenerateError = ref('')

// Test Related State
const testing = ref(false)
const testResult = ref<TestResult | null>(null)

// Advanced Test State
const advancedPlugin = ref<PluginRecord | null>(null)
const advancedTesting = ref(false)
const advancedError = ref('')
const advancedResult = ref<AdvancedTestResult | null>(null)
const advancedForm = ref({
  url: 'https://example.com/test',
  method: 'GET',
  headersText: '{"User-Agent":"Sentinel-AdvTest/1.0"}',
  bodyText: '',
  runs: 3,
  concurrency: 2,
})

let pluginChangedUnlisten: UnlistenFn | null = null

// Categories Definition
const categories = computed(() => [
  {
    value: 'all',
    label: t('plugins.categories.all', '全部'),
    icon: 'fas fa-th'
  },
  {
    value: 'passiveScan',
    label: t('plugins.categories.passiveScan', '被动扫描插件'),
    icon: 'fas fa-shield-alt'
  },
  {
    value: 'agentTools',
    label: t('plugins.categories.agentTools', 'Agent工具插件'),
    icon: 'fas fa-robot'
  },
//   {
//     value: 'builtinTools',
//     label: t('plugins.categories.builtinTools', '内置工具插件'),
//     icon: 'fas fa-toolbox'
//   },
//   {
//     value: 'mcpTools',
//     label: t('plugins.categories.mcpTools', 'MCP工具插件'),
//     icon: 'fas fa-plug'
//   },
//   {
//     value: 'vulnerability',
//     label: t('plugins.categories.vulnerability', '漏洞扫描'),
//     icon: 'fas fa-bug'
//   },
//   {
//     value: 'injection',
//     label: t('plugins.categories.injection', '注入检测'),
//     icon: 'fas fa-syringe'
//   },
//   {
//     value: 'xss',
//     label: t('plugins.categories.xss', '跨站脚本'),
//     icon: 'fas fa-code'
//   },
  {
    value: 'custom',
    label: t('plugins.categories.custom', '自定义'),
    icon: 'fas fa-wrench'
  }
])

// Filtered Plugins
const filteredPlugins = computed(() => {
  if (selectedCategory.value === 'all') {
    return plugins.value
  }
  // 被动扫描插件：显示所有被动扫描子分类
  if (selectedCategory.value === 'passiveScan') {
    return plugins.value.filter(p => passiveScanCategories.includes(p.metadata.category))
  }
  // Agent 工具插件：显示所有 Agent 子分类
  if (selectedCategory.value === 'agentTools') {
    return plugins.value.filter(p => agentToolsCategories.includes(p.metadata.category))
  }
  // 其他：精确匹配
  return plugins.value.filter(p => p.metadata.category === selectedCategory.value)
})

// Get Category Count
const getCategoryCount = (category: string) => {
  if (category === 'all') return plugins.value.length
  // 被动扫描插件：统计所有被动扫描子分类
  if (category === 'passiveScan') {
    return plugins.value.filter(p => passiveScanCategories.includes(p.metadata.category)).length
  }
  // Agent 工具插件：统计所有 Agent 子分类
  if (category === 'agentTools') {
    return plugins.value.filter(p => agentToolsCategories.includes(p.metadata.category)).length
  }
  // 其他：精确匹配
  return plugins.value.filter(p => p.metadata.category === category).length
}

// Sorted runs by index for stable display
const sortedRuns = computed(() => {
  if (!advancedResult.value) return [] as AdvancedRunStat[]
  return [...advancedResult.value.runs].sort((a, b) => a.run_index - b.run_index)
})

// Get Category Label
const getCategoryLabel = (category: string) => {
  const cat = categories.value.find(c => c.value === category)
  return cat ? cat.label : category
}

// Get Category Icon
const getCategoryIcon = (category: string) => {
  const cat = categories.value.find(c => c.value === category)
  return cat ? cat.icon : 'fas fa-wrench'
}

// Check if plugin is Agent type
const isAgentPluginType = (plugin: PluginRecord): boolean => {
  return plugin.metadata.category === 'agentTools' || 
         ['scanner', 'analyzer', 'reporter', 'recon', 'exploit', 'utility'].includes(plugin.metadata.category)
}

// Check if plugin is Passive Scan type
const isPassiveScanPluginType = (plugin: PluginRecord): boolean => {
  return passiveScanCategories.includes(plugin.metadata.category)
}

// Validate New Plugin
const isNewPluginValid = computed(() => {
  return newPluginMetadata.value.id.trim() !== '' &&
         newPluginMetadata.value.name.trim() !== '' &&
         newPluginMetadata.value.category.trim() !== '' &&
         pluginCode.value.trim() !== ''
})

// Get Status Text
const getStatusText = (status: string): string => {
  const statusMap: Record<string, string> = {
    'Enabled': t('plugins.enabled', '已启用'),
    'Disabled': t('plugins.disabled', '已禁用'),
    'Error': t('plugins.error', '错误')
  }
  return statusMap[status] || status
}

// Show Toast
const showToast = (message: string, type: 'success' | 'error' | 'info' | 'warning' = 'success') => {
  const toast = document.createElement('div')
  toast.className = 'toast toast-top toast-end z-50'
  toast.style.top = '5rem'
  
  const alertClass = {
    success: 'alert-success',
    error: 'alert-error',
    info: 'alert-info',
    warning: 'alert-warning'
  }[type]
  
  const icon = {
    success: 'fa-check-circle',
    error: 'fa-times-circle',
    info: 'fa-info-circle',
    warning: 'fa-exclamation-triangle'
  }[type]
  
  toast.innerHTML = `
    <div class="alert ${alertClass} shadow-lg">
      <i class="fas ${icon}"></i>
      <span>${message}</span>
    </div>
  `
  document.body.appendChild(toast)
  setTimeout(() => toast.remove(), 3000)
}

// Refresh Plugins List
const refreshPlugins = async () => {
  try {
    const response = await invoke<CommandResponse<PluginRecord[]>>('list_plugins')
    if (response.success && response.data) {
      plugins.value = response.data
    } else {
      console.error('Failed to refresh plugins:', response.error)
    }
  } catch (error) {
    console.error('Error refreshing plugins:', error)
  }
}

// Scan Plugin Directory
// const scanPluginDirectory = async () => {
//   try {
//     const response = await invoke<CommandResponse<string[]>>('scan_plugin_directory')
//     if (response.success) {
//       await refreshPlugins()
//       const count = response.data?.length || 0
//       showToast(t('plugins.scanComplete', `扫描完成，发现 ${count} 个插件`), 'success')
//     } else {
//       showToast(t('plugins.scanFailed', `扫描失败: ${response.error || '未知错误'}`), 'error')
//     }
//   } catch (error) {
//     console.error('Error scanning plugin directory:', error)
//     showToast(t('plugins.scanError', '扫描插件目录时出错'), 'error')
//   }
// }

// Toggle Plugin Enable/Disable
const togglePlugin = async (plugin: PluginRecord) => {
  try {
    const command = plugin.status === 'Enabled' ? 'disable_plugin' : 'enable_plugin'
    const actionText = plugin.status === 'Enabled' ? t('plugins.disable', '禁用') : t('plugins.enable', '启用')
    
    const response = await invoke<CommandResponse<void>>(command, {
      pluginId: plugin.metadata.id
    })
    
    if (response.success) {
      await refreshPlugins()
      showToast(t('plugins.toggleSuccess', `插件已${actionText}: ${plugin.metadata.name}`), 'success')
    } else {
      showToast(t('plugins.toggleFailed', `${actionText}失败: ${response.error || '未知错误'}`), 'error')
    }
  } catch (error) {
    console.error('Error toggling plugin:', error)
    showToast(t('plugins.toggleError', '操作失败'), 'error')
  }
}

// Open Create Plugin Dialog
const openCreateDialog = () => {
  newPluginMetadata.value = {
    id: '',
    name: '',
    version: '1.0.0',
    author: '',
    mainCategory: 'passive',
    category: 'vulnerability',
    default_severity: 'medium',
    description: '',
    tagsString: ''
  }
  pluginCode.value = ''
  editingPlugin.value = null
  isEditing.value = false
  codeError.value = ''
  codeEditorDialog.value?.showModal()
}

// Open Upload Dialog
const openUploadDialog = () => {
  uploadError.value = ''
  selectedFile.value = null
  if (fileInput.value) {
    fileInput.value.value = ''
  }
  uploadDialog.value?.showModal()
}

// Advanced Test Dialog Controls
const openAdvancedDialog = (plugin: PluginRecord) => {
  advancedPlugin.value = plugin
  advancedError.value = ''
  advancedResult.value = null
  
  // 检查是否为 Agent 插件
  const isAgentPlugin = plugin.metadata.category === 'agentTools' || 
                        ['scanner', 'analyzer', 'reporter', 'recon', 'exploit', 'utility'].includes(plugin.metadata.category)
  
  if (isAgentPlugin) {
    // Agent 插件显示提示信息
    advancedError.value = 'Agent 插件暂不支持高级并发测试功能。Agent 插件测试通过调用 analyze 函数进行，请使用普通测试按钮。'
  } else {
    // 被动扫描插件初始化测试参数
    if (!advancedForm.value.url) advancedForm.value.url = 'https://example.com/test'
    if (!advancedForm.value.method) advancedForm.value.method = 'GET'
    if (!advancedForm.value.runs || advancedForm.value.runs < 1) advancedForm.value.runs = 3
    if (!advancedForm.value.concurrency || advancedForm.value.concurrency < 1) advancedForm.value.concurrency = 2
  }
  
  advancedDialog.value?.showModal()
}

const closeAdvancedDialog = () => {
  advancedDialog.value?.close()
  advancedTesting.value = false
  // do not reset form to preserve inputs between opens
}

// Close Upload Dialog
const closeUploadDialog = () => {
  uploadDialog.value?.close()
  selectedFile.value = null
  uploadError.value = ''
}

// Handle File Select
const handleFileSelect = (event: Event) => {
  const target = event.target as HTMLInputElement
  if (target.files && target.files.length > 0) {
    selectedFile.value = target.files[0]
    uploadError.value = ''
  }
}

// Upload Plugin
const uploadPlugin = async () => {
  if (!selectedFile.value) return
  
  uploading.value = true
  uploadError.value = ''
  
  try {
    const fileContent = await selectedFile.value.text()
    
    // Parse plugin metadata from comments
    const metadataMatch = fileContent.match(/\/\*\*\s*([\s\S]*?)\s*\*\//)
    if (!metadataMatch) {
      uploadError.value = t('plugins.parseMetadataError', '无法从文件中解析插件元数据')
      uploading.value = false
      return
    }
    
    const metadataText = metadataMatch[1]
    const extractField = (field: string): string => {
      const match = metadataText.match(new RegExp(`@${field}\\s+(.+)`, 'i'))
      return match ? match[1].trim() : ''
    }
    
    const id = extractField('plugin')
    const name = extractField('name')
    const version = extractField('version')
    const author = extractField('author') || 'Unknown'
    const category = extractField('category')
    const defaultSeverity = extractField('default_severity')
    const tagsStr = extractField('tags')
    const description = extractField('description')
    
    if (!id || !name || !version || !category || !defaultSeverity) {
      uploadError.value = t('plugins.incompleteMetadata', '插件元数据不完整')
      uploading.value = false
      return
    }
    
    const tags = tagsStr.split(',').map(t => t.trim()).filter(t => t.length > 0)
    
    const metadata = {
      id, name, version, author, category, description,
      default_severity: defaultSeverity,
      tags
    }
    
    const response = await invoke<CommandResponse<string>>('create_plugin_in_db', {
      metadata,
      pluginCode: fileContent
    })
    
    if (response.success) {
      closeUploadDialog()
      await refreshPlugins()
      showToast(t('plugins.uploadSuccess', `插件上传成功: ${name}`), 'success')
    } else {
      uploadError.value = response.error || t('plugins.uploadFailed', '上传失败')
    }
  } catch (error) {
    uploadError.value = error instanceof Error ? error.message : t('plugins.uploadFailed', '上传失败')
    console.error('Error uploading plugin:', error)
  } finally {
    uploading.value = false
  }
}

// View Plugin Code
const viewPluginCode = async (plugin: PluginRecord) => {
  try {
    const response = await invoke<CommandResponse<string | null>>('get_plugin_code', {
      pluginId: plugin.metadata.id
    })
    
    if (response.success && response.data) {
      // Successfully got code from backend
      pluginCode.value = response.data
      originalCode.value = pluginCode.value
      editingPlugin.value = plugin
      
      // Load plugin metadata into form
      const oldCategory = plugin.metadata.category
      newPluginMetadata.value = {
        id: plugin.metadata.id,
        name: plugin.metadata.name,
        version: plugin.metadata.version,
        author: plugin.metadata.author || '',
        mainCategory: inferMainCategory(oldCategory),
        category: convertToSubCategory(oldCategory),
        default_severity: plugin.metadata.default_severity,
        description: plugin.metadata.description || '',
        tagsString: plugin.metadata.tags.join(', ')
      }
      
      isEditing.value = false
      codeError.value = ''
      codeEditorDialog.value?.showModal()
    } else if (response.success && response.data === null && plugin.path) {
      // Fallback to reading from file only if path exists
      try {
        const content = await readFile(plugin.path)
        const decoder = new TextDecoder('utf-8')
        pluginCode.value = decoder.decode(content)
        originalCode.value = pluginCode.value
        editingPlugin.value = plugin
        
        // Load plugin metadata into form
        const oldCategory = plugin.metadata.category
        newPluginMetadata.value = {
          id: plugin.metadata.id,
          name: plugin.metadata.name,
          version: plugin.metadata.version,
          author: plugin.metadata.author || '',
          mainCategory: inferMainCategory(oldCategory),
          category: convertToSubCategory(oldCategory),
          default_severity: plugin.metadata.default_severity,
          description: plugin.metadata.description || '',
          tagsString: plugin.metadata.tags.join(', ')
        }
        
        isEditing.value = false
        codeError.value = ''
        codeEditorDialog.value?.showModal()
      } catch (fileError) {
        console.error('Error reading plugin file:', fileError)
        codeError.value = t('plugins.readCodeError', '读取代码失败')
        showToast(codeError.value, 'error')
      }
    } else {
      codeError.value = response.error || t('plugins.readCodeError', '读取代码失败')
      showToast(codeError.value, 'error')
    }
  } catch (error) {
    console.error('Error reading plugin code:', error)
    codeError.value = error instanceof Error ? error.message : t('plugins.readCodeError', '读取代码失败')
    showToast(codeError.value, 'error')
  }
}

// Close Code Editor Dialog
const closeCodeEditorDialog = () => {
  codeEditorDialog.value?.close()
  editingPlugin.value = null
  pluginCode.value = ''
  originalCode.value = ''
  isEditing.value = false
  codeError.value = ''
}

// Enable Editing
const enableEditing = () => {
  isEditing.value = true
}

// Cancel Editing
const cancelEditing = () => {
  pluginCode.value = originalCode.value
  isEditing.value = false
  codeError.value = ''
  
  // Restore original metadata if editing existing plugin
  if (editingPlugin.value) {
    const oldCategory = editingPlugin.value.metadata.category
    newPluginMetadata.value = {
      id: editingPlugin.value.metadata.id,
      name: editingPlugin.value.metadata.name,
      version: editingPlugin.value.metadata.version,
      author: editingPlugin.value.metadata.author || '',
      mainCategory: inferMainCategory(oldCategory),
      category: convertToSubCategory(oldCategory),
      default_severity: editingPlugin.value.metadata.default_severity,
      description: editingPlugin.value.metadata.description || '',
      tagsString: editingPlugin.value.metadata.tags.join(', ')
    }
  }
}

// Save Plugin Code
const savePluginCode = async () => {
  if (!editingPlugin.value) return
  
  saving.value = true
  codeError.value = ''
  
  try {
    // Parse tags
    const tags = newPluginMetadata.value.tagsString
      .split(',')
      .map(t => t.trim())
      .filter(t => t.length > 0)
    
    // Map hierarchical category to backend category
    // Agent插件统一映射为 agentTools，被动扫描插件保留子分类
    const backendCategory = newPluginMetadata.value.mainCategory === 'agent' 
      ? 'agentTools' 
      : newPluginMetadata.value.category
    
    // Build metadata comment
    const metadataComment = `/**
 * @plugin ${newPluginMetadata.value.id}
 * @name ${newPluginMetadata.value.name}
 * @version ${newPluginMetadata.value.version}
 * @author ${newPluginMetadata.value.author || 'Unknown'}
 * @category ${backendCategory}
 * @default_severity ${newPluginMetadata.value.default_severity}
 * @tags ${tags.join(', ')}
 * @description ${newPluginMetadata.value.description || ''}
 */
`
    
    // Remove old metadata comment if exists
    let codeWithoutMetadata = pluginCode.value
    const metadataMatch = codeWithoutMetadata.match(/\/\*\*\s*[\s\S]*?\*\/\s*/)
    if (metadataMatch) {
      codeWithoutMetadata = codeWithoutMetadata.substring(metadataMatch[0].length)
    }
    
    // Combine new metadata with code
    const fullCode = metadataComment + '\n' + codeWithoutMetadata
    
    // Update metadata object
    const metadata = {
      id: newPluginMetadata.value.id,
      name: newPluginMetadata.value.name,
      version: newPluginMetadata.value.version,
      author: newPluginMetadata.value.author || 'Unknown',
      category: backendCategory,
      description: newPluginMetadata.value.description || '',
      default_severity: newPluginMetadata.value.default_severity,
      tags: tags
    }
    
    // Use create_plugin_in_db for both create and update (it uses INSERT OR REPLACE)
    const response = await invoke<CommandResponse<string>>('create_plugin_in_db', {
      metadata: metadata,
      pluginCode: fullCode
    })
    
    if (response.success) {
      originalCode.value = fullCode
      pluginCode.value = fullCode
      isEditing.value = false
      await refreshPlugins()
      showToast(t('plugins.saveSuccess', '插件保存成功'), 'success')
      
      // 如果被动扫描正在运行，重载插件
      try {
        const reloadResponse = await invoke<CommandResponse<string>>('reload_plugin_in_pipeline', {
          pluginId: newPluginMetadata.value.id
        })
        if (reloadResponse.success) {
          console.log('Plugin reloaded in pipeline:', reloadResponse.data)
          showToast('插件已在被动扫描中热更新', 'info')
        } else {
          console.warn('Failed to reload plugin:', reloadResponse.error)
        }
      } catch (reloadError) {
        console.warn('Plugin reload failed (passive scan may not be running):', reloadError)
      }
    } else {
      codeError.value = response.error || t('plugins.saveFailed', '保存失败')
    }
  } catch (error) {
    codeError.value = error instanceof Error ? error.message : t('plugins.saveFailed', '保存失败')
    console.error('Error saving plugin code:', error)
  } finally {
    saving.value = false
  }
}

// Create New Plugin
const createNewPlugin = async () => {
  saving.value = true
  codeError.value = ''
  
  try {
    const tags = newPluginMetadata.value.tagsString
      .split(',')
      .map(t => t.trim())
      .filter(t => t.length > 0)
    
    // Map hierarchical category to backend category
    // Agent插件统一映射为 agentTools，被动扫描插件保留子分类
    const backendCategory = newPluginMetadata.value.mainCategory === 'agent' 
      ? 'agentTools' 
      : newPluginMetadata.value.category
    
    const metadataComment = `/**
 * @plugin ${newPluginMetadata.value.id}
 * @name ${newPluginMetadata.value.name}
 * @version ${newPluginMetadata.value.version}
 * @author ${newPluginMetadata.value.author || 'Unknown'}
 * @category ${backendCategory}
 * @default_severity ${newPluginMetadata.value.default_severity}
 * @tags ${tags.join(', ')}
 * @description ${newPluginMetadata.value.description || ''}
 */
`
    
    const fullCode = metadataComment + '\n' + pluginCode.value
    
    const metadata = {
      id: newPluginMetadata.value.id,
      name: newPluginMetadata.value.name,
      version: newPluginMetadata.value.version,
      author: newPluginMetadata.value.author || 'Unknown',
      category: backendCategory,
      description: newPluginMetadata.value.description || '',
      default_severity: newPluginMetadata.value.default_severity,
      tags: tags
    }
    
    const response = await invoke<CommandResponse<string>>('create_plugin_in_db', {
      metadata,
      pluginCode: fullCode
    })
    
    if (response.success) {
      closeCodeEditorDialog()
      await refreshPlugins()
      showToast(t('plugins.createSuccess', `插件创建成功: ${newPluginMetadata.value.name}`), 'success')
    } else {
      codeError.value = response.error || t('plugins.createFailed', '创建插件失败')
    }
  } catch (error) {
    codeError.value = error instanceof Error ? error.message : t('plugins.createFailed', '创建插件失败')
    console.error('Error creating plugin:', error)
  } finally {
    saving.value = false
  }
}

// Insert Code Template
const insertTemplate = () => {
  const isAgentPlugin = newPluginMetadata.value.mainCategory === 'agent'
  
  if (isAgentPlugin) {
    // Agent插件模板 - 自由参数格式
    pluginCode.value = `export interface ToolInput {
  [key: string]: any;
}

export interface ToolOutput {
  success: boolean;
  data?: any;
  error?: string;
}

/**
 * Agent工具插件
 * @param input - 工具输入参数（根据需要自定义）
 * @returns 工具执行结果
 */
export async function analyze(input: ToolInput): Promise<ToolOutput> {
  try {
    // TODO: 实现你的Agent工具逻辑
    // 例如：
    // - 数据分析工具
    // - 扫描器工具
    // - 报告生成工具
    
    return {
      success: true,
      data: {}
    };
  } catch (error) {
    return {
      success: false,
      error: error instanceof Error ? error.message : 'Unknown error'
    };
  }
}
`
  } else {
    // 被动扫描插件模板 - HTTP请求/响应分析
    pluginCode.value = `export interface HttpRequest {
  method: string;
  url: string;
  headers: Record<string, string>;
  body?: string;
}

export interface HttpResponse {
  status: number;
  headers: Record<string, string>;
  body?: string;
}

export interface PluginContext {
  request: HttpRequest;
  response: HttpResponse;
}

export interface Finding {
  title: string;
  description: string;
  severity: 'info' | 'low' | 'medium' | 'high' | 'critical';
  evidence?: string;
  recommendation?: string;
}

/**
 * 被动扫描插件 - 分析HTTP流量
 * @param context - HTTP请求和响应上下文
 * @returns 检测发现列表
 */
export async function analyze(context: PluginContext): Promise<Finding[]> {
  const findings: Finding[] = [];
  
  // TODO: 实现你的被动扫描逻辑
  // 例如：
  // - 漏洞检测
  // - 注入检测
  // - 敏感信息泄露检测
  
  return findings;
}
`
  }
}


// Format Code
const formatCode = () => {
  try {
    const lines = pluginCode.value.split('\n')
    let indentLevel = 0
    const formatted = lines.map(line => {
      const trimmed = line.trim()
      
      if (trimmed.startsWith('}') || trimmed.startsWith(']')) {
        indentLevel = Math.max(0, indentLevel - 1)
      }
      
      const indented = '  '.repeat(indentLevel) + trimmed
      
      if (trimmed.endsWith('{') || trimmed.endsWith('[')) {
        indentLevel++
      }
      
      return indented
    })
    
    pluginCode.value = formatted.join('\n')
  } catch (error) {
    console.error('Error formatting code:', error)
  }
}

// Open AI Generate Dialog
const openAIGenerateDialog = () => {
  aiPrompt.value = ''
  aiPluginType.value = 'vulnerability'
  aiSeverity.value = 'medium'
  aiGenerateError.value = ''
  aiGenerateDialog.value?.showModal()
}

// Close AI Generate Dialog
const closeAIGenerateDialog = () => {
  aiGenerateDialog.value?.close()
  aiPrompt.value = ''
  aiGenerateError.value = ''
}

// Generate Plugin with AI
const generatePluginWithAI = async () => {
  aiGenerating.value = true
  aiGenerateError.value = ''
  
  try {
    // 判断插件类型
    const isAgentPlugin = aiPluginType.value === 'agentTools' || 
                          ['scanner', 'analyzer', 'reporter', 'recon', 'exploit', 'utility'].includes(aiPluginType.value)
    
    let systemPrompt = ''
    
    if (isAgentPlugin) {
      // Agent 插件提示词
      systemPrompt = `你是一个 Agent 工具插件开发专家。请根据用户的需求生成一个 TypeScript Agent 工具插件代码。

Agent 工具插件应该包含以下接口：
- ToolInput: 自定义输入参数接口 { [key: string]: any }
- ToolOutput: 输出结果接口 { success: boolean; data?: any; error?: string }
- analyze 函数: 接收 ToolInput，返回 Promise<ToolOutput>

插件类型: ${aiPluginType.value}
严重程度: ${aiSeverity.value}

注意：
1. Agent 工具插件不分析 HTTP 请求/响应，而是执行特定的工具任务
2. analyze 函数应该实现具体的工具逻辑（如扫描、分析、报告生成等）
3. 输入参数应该根据工具功能自定义设计
4. 返回结果应该包含工具执行的详细信息

请只返回完整的 TypeScript 代码，不要包含任何解释或 markdown 标记。代码应该可以直接使用。`
    } else {
      // 被动扫描插件提示词
      systemPrompt = `你是一个安全插件开发专家。请根据用户的需求生成一个 TypeScript 被动扫描插件代码。

被动扫描插件应该包含以下接口：
- HttpRequest: 包含 method, url, headers, body
- HttpResponse: 包含 status, headers, body
- PluginContext: 包含 request 和 response
- Finding: 包含 title, description, severity, evidence, recommendation
- analyze 函数: 接收 PluginContext，返回 Finding[]

插件类型: ${aiPluginType.value}
严重程度: ${aiSeverity.value}

注意：
1. 被动扫描插件分析 HTTP 请求和响应，检测安全问题
2. analyze 函数应该检查请求/响应中的漏洞、注入、敏感信息等
3. 返回的 Finding 应该包含详细的问题描述和修复建议

请只返回完整的 TypeScript 代码，不要包含任何解释或 markdown 标记。代码应该可以直接使用。`
    }

    const tempConversationId = `plugin_gen_${Date.now()}`
    const userPrompt = `${systemPrompt}\n\n用户需求：${aiPrompt.value}`
    
    let generatedCode = ''
    let streamCompleted = false
    let streamError: string | null = null
    
    const unlisten = await listen('message_chunk', (event: any) => {
      const payload = event.payload
      if (payload.conversation_id === tempConversationId) {
        if (payload.chunk_type === 'Content' && payload.content) {
          generatedCode += payload.content
        }
        if (payload.is_final) {
          streamCompleted = true
        }
      }
    })
    
    const unlistenError = await listen('ai_stream_error', (event: any) => {
      const payload = event.payload
      if (payload.conversation_id === tempConversationId) {
        streamError = payload.error || payload.message || t('plugins.aiGenerateFailed', 'AI生成失败')
        streamCompleted = true
      }
    })
    
    try {
      await invoke('send_ai_stream_message', {
        request: {
          conversation_id: tempConversationId,
          message: userPrompt,
          service_name: 'default',
        }
      })
      
      const maxWaitTime = 60000
      const startTime = Date.now()
      while (!streamCompleted && (Date.now() - startTime < maxWaitTime)) {
        await new Promise(resolve => setTimeout(resolve, 100))
      }
      
      if (streamError) {
        throw new Error(streamError)
      }
      
      if (!streamCompleted) {
        throw new Error(t('plugins.aiTimeout', 'AI响应超时'))
      }
      
      // Clean AI response
      generatedCode = generatedCode.trim()
      generatedCode = generatedCode.replace(/```typescript\n?/g, '')
      generatedCode = generatedCode.replace(/```ts\n?/g, '')
      generatedCode = generatedCode.replace(/```javascript\n?/g, '')
      generatedCode = generatedCode.replace(/```js\n?/g, '')
      generatedCode = generatedCode.replace(/```\n?/g, '')
      generatedCode = generatedCode.trim()
      
      if (!generatedCode) {
        throw new Error(t('plugins.aiNoCode', 'AI未返回任何代码'))
      }
      
      const pluginId = aiPrompt.value
        .toLowerCase()
        .replace(/[^a-z0-9]+/g, '_')
        .replace(/^_+|_+$/g, '')
        .substring(0, 50) || 'ai_generated_plugin'
      
      // 根据插件类型设置元数据
      const mainCategory = isAgentPlugin ? 'agent' : 'passive'
      const category = isAgentPlugin ? convertToSubCategory(aiPluginType.value) : aiPluginType.value
      
      newPluginMetadata.value = {
        id: pluginId,
        name: aiPrompt.value.substring(0, 50),
        version: '1.0.0',
        author: 'AI Generated',
        mainCategory: mainCategory,
        category: category,
        default_severity: aiSeverity.value,
        description: aiPrompt.value,
        tagsString: `ai-generated, ${aiPluginType.value}`
      }
      
      pluginCode.value = generatedCode
      editingPlugin.value = null
      
      closeAIGenerateDialog()
      codeEditorDialog.value?.showModal()
    } finally {
      unlisten()
      unlistenError()
      
      try {
        await invoke('delete_ai_conversation', { conversationId: tempConversationId })
      } catch (e) {
        console.warn('Failed to cleanup temporary conversation:', e)
      }
    }
  } catch (error) {
    aiGenerateError.value = error instanceof Error ? error.message : t('plugins.aiGenerateFailed', 'AI生成失败')
    console.error('Error generating plugin with AI:', error)
  } finally {
    aiGenerating.value = false
  }
}

// Test Plugin - 根据插件类型选择不同的测试方式
const testPlugin = async (plugin: PluginRecord) => {
  if (!plugin || !plugin.metadata?.id) return
  
  // 判断插件类型
  const isAgentPlugin = plugin.metadata.category === 'agentTools' || 
                        ['scanner', 'analyzer', 'reporter', 'recon', 'exploit', 'utility'].includes(plugin.metadata.category)
  
  testing.value = true
  testResult.value = null
  
  try {
    if (isAgentPlugin) {
      // Agent 插件测试：调用 analyze 函数
      await testAgentPlugin(plugin)
    } else {
      // 被动扫描插件测试：调用 scan_request/scan_response
      await testPassiveScanPlugin(plugin)
    }
  } catch (e) {
    console.error('Error testing plugin:', e)
    const msg = e instanceof Error ? e.message : '测试失败'
    showToast(t('plugins.testError', msg), 'error')
    testResult.value = { success: false, message: msg, error: msg }
    testResultDialog.value?.showModal()
  } finally {
    testing.value = false
  }
}

// 测试被动扫描插件（HTTP请求/响应分析）
const testPassiveScanPlugin = async (plugin: PluginRecord) => {
  const resp = await invoke<CommandResponse<TestResult>>('test_plugin', {
    pluginId: plugin.metadata.id
  })
  
  if (resp.success && resp.data) {
    testResult.value = resp.data
    testResultDialog.value?.showModal()
    if (resp.data.success) {
      showToast(t('plugins.testSuccess', `测试完成，发现 ${resp.data.findings?.length || 0} 条结果`), 'success')
    } else {
      showToast(t('plugins.testFailed', resp.data.message || '测试失败'), 'error')
      testResultDialog.value?.showModal()
    }
  } else {
    const msg = resp.error || '测试命令执行失败'
    showToast(t('plugins.testError', msg), 'error')
    testResult.value = { success: false, message: msg, error: msg }
    testResultDialog.value?.showModal()
  }
}

// 测试 Agent 插件（调用 analyze 函数）
const testAgentPlugin = async (plugin: PluginRecord) => {
  // 构造测试输入数据
  const testInput = {
    plugin_id: plugin.metadata.id,
    target: 'https://example.com/test',
    context: {
      test_mode: true,
      description: 'Plugin test execution'
    },
    data: {
      sample_key: 'sample_value'
    }
  }
  
  const resp = await invoke<CommandResponse<any>>('test_execute_plugin_tool', {
    request: testInput
  })
  
  if (resp.success && resp.data) {
    const result = resp.data
    testResult.value = {
      success: result.success,
      message: `插件 '${result.tool_name}' 执行完成\n执行时间: ${result.execution_time_ms}ms`,
      findings: result.output ? [{
        title: 'Agent 工具执行结果',
        description: JSON.stringify(result.output, null, 2),
        severity: result.success ? 'info' : 'error'
      }] : undefined,
      error: result.error
    }
    testResultDialog.value?.showModal()
    
    if (result.success) {
      showToast(t('plugins.testSuccess', `Agent 插件测试完成 (${result.execution_time_ms}ms)`), 'success')
    } else {
      showToast(t('plugins.testFailed', result.error || '测试失败'), 'error')
    }
  } else {
    const msg = resp.error || 'Agent 插件测试命令执行失败'
    showToast(t('plugins.testError', msg), 'error')
    testResult.value = { success: false, message: msg, error: msg }
    testResultDialog.value?.showModal()
  }
}

// Run Advanced Test - 根据插件类型选择不同的测试方式
const runAdvancedTest = async () => {
  if (!advancedPlugin.value) return
  
  // 判断插件类型
  const isAgentPlugin = advancedPlugin.value.metadata.category === 'agentTools' || 
                        ['scanner', 'analyzer', 'reporter', 'recon', 'exploit', 'utility'].includes(advancedPlugin.value.metadata.category)
  
  advancedTesting.value = true
  advancedError.value = ''
  advancedResult.value = null
  
  try {
    if (isAgentPlugin) {
      // Agent 插件不支持高级测试（多次并发测试），使用单次测试
      advancedError.value = 'Agent 插件暂不支持高级并发测试，请使用普通测试功能'
      showToast('Agent 插件暂不支持高级并发测试', 'warning')
      return
    }
    
    // 被动扫描插件的高级测试
    const headersStr = (advancedForm.value.headersText || '').trim()
    if (headersStr.length > 0) {
      try { JSON.parse(headersStr) } catch (e) {
        console.warn('Invalid headers JSON, proceeding with backend defaults:', e)
      }
    }

    const resp = await invoke<CommandResponse<AdvancedTestResult>>('test_plugin_advanced', {
      pluginId: advancedPlugin.value.metadata.id,
      url: advancedForm.value.url || undefined,
      method: advancedForm.value.method || undefined,
      headers: headersStr || undefined,
      body: advancedForm.value.bodyText || undefined,
      runs: Number(advancedForm.value.runs) || 1,
      concurrency: Number(advancedForm.value.concurrency) || 1,
    } as any)

    if (resp.success && resp.data) {
      advancedResult.value = resp.data
      if (resp.data.success) {
        showToast(t('plugins.testSuccess', `测试完成，唯一发现 ${resp.data.unique_findings} 条`), 'success')
      } else {
        showToast(t('plugins.testFailed', resp.data.message || '测试失败'), 'error')
      }
    } else {
      const msg = resp.error || '高级测试命令执行失败'
      advancedError.value = msg
      showToast(t('plugins.testError', msg), 'error')
    }
  } catch (e) {
    console.error('Error advanced testing plugin:', e)
    const msg = e instanceof Error ? e.message : '高级测试失败'
    advancedError.value = msg
    showToast(t('plugins.testError', msg), 'error')
  } finally {
    advancedTesting.value = false
  }
}

// Close Test Result Dialog
const closeTestResultDialog = () => {
  testResultDialog.value?.close()
  testResult.value = null
}

// Get test type label for current testing plugin
const getTestTypeLabel = (): string => {
  // 从 testResult 中推断插件类型（如果有 findings）
  if (testResult.value?.findings && testResult.value.findings.length > 0) {
    const firstFinding = testResult.value.findings[0]
    if (firstFinding.title === 'Agent 工具执行结果') {
      return 'Agent 工具测试'
    }
  }
  return '被动扫描测试'
}

// Confirm Delete Plugin
const confirmDeletePlugin = (plugin: PluginRecord) => {
  deletingPlugin.value = plugin
  deleteDialog.value?.showModal()
}

// Close Delete Dialog
const closeDeleteDialog = () => {
  deleteDialog.value?.close()
  deletingPlugin.value = null
}

// Delete Plugin
const deletePlugin = async () => {
  if (!deletingPlugin.value) return
  
  deleting.value = true
  
  try {
    // 调用后端删除插件命令
    const response = await invoke<CommandResponse<void>>('delete_plugin', {
      pluginId: deletingPlugin.value.metadata.id
    })
    
    if (response.success) {
      const pluginName = deletingPlugin.value.metadata.name
      closeDeleteDialog()
      await refreshPlugins()
      showToast(t('plugins.deleteSuccess', `插件已删除: ${pluginName}`), 'success')
    } else {
      throw new Error(response.error || t('plugins.deleteFailed', '删除失败'))
    }
  } catch (error) {
    console.error('Error deleting plugin:', error)
    showToast(t('plugins.deleteFailed', `删除失败: ${error instanceof Error ? error.message : '未知错误'}`), 'error')
  } finally {
    deleting.value = false
  }
}

// Setup Event Listeners
const setupEventListeners = async () => {
  pluginChangedUnlisten = await listen('plugin:changed', () => {
    refreshPlugins()
  })
}

// Component Lifecycle
onMounted(async () => {
  await refreshPlugins()
  await setupEventListeners()
})

onUnmounted(() => {
  if (pluginChangedUnlisten) {
    pluginChangedUnlisten()
  }
})
</script>

<style scoped>
.tabs {
  max-width: 100%;
}

.tab {
  flex-shrink: 0;
}

.table th {
  background-color: hsl(var(--b2));
}

textarea.textarea {
  font-family: 'Consolas', 'Monaco', 'Courier New', monospace;
  line-height: 1.5;
  tab-size: 2;
}

.badge {
  font-weight: 600;
}
</style>
