<template>
  <!-- Review Plugin Detail Dialog -->
  <dialog ref="reviewDetailDialogRef" class="modal">
    <div class="modal-box w-11/12 max-w-6xl max-h-[90vh] overflow-y-auto">
      <div class="flex justify-between items-start mb-4 sticky top-0 bg-base-100 z-10 pb-2">
        <h3 class="font-bold text-lg">
          <i class="fas fa-eye mr-2"></i>
          {{ $t('plugins.pluginDetail', '插件详情') }}
        </h3>
        <button @click="closeReviewDetailDialog" class="btn btn-sm btn-circle btn-ghost">✕</button>
      </div>

      <div v-if="selectedReviewPlugin" class="space-y-4">
        <!-- Basic Info -->
        <div class="card bg-base-200">
          <div class="card-body p-4">
            <h4 class="font-semibold mb-3">{{ $t('plugins.basicInfo', '基本信息') }}</h4>
            <div class="grid grid-cols-2 gap-4 text-sm">
              <div>
                <span class="text-gray-500">{{ $t('plugins.pluginId', '插件ID') }}:</span>
                <span class="ml-2 font-mono">{{ selectedReviewPlugin.plugin_id }}</span>
              </div>
              <div>
                <span class="text-gray-500">{{ $t('plugins.pluginName', '插件名称') }}:</span>
                <span class="ml-2">{{ selectedReviewPlugin.plugin_name }}</span>
              </div>
              <div>
                <span class="text-gray-500">{{ $t('plugins.vulnType', '漏洞类型') }}:</span>
                <span class="ml-2 badge badge-sm" :class="getVulnTypeBadgeClass(selectedReviewPlugin.vuln_type)">
                  {{ selectedReviewPlugin.vuln_type.toUpperCase() }}
                </span>
              </div>
              <div>
                <span class="text-gray-500">{{ $t('plugins.model', '生成模型') }}:</span>
                <span class="ml-2">{{ selectedReviewPlugin.model }}</span>
              </div>
              <div class="col-span-2">
                <span class="text-gray-500">{{ $t('plugins.qualityScore', '质量评分') }}:</span>
                <div class="flex items-center gap-2 mt-1">
                  <progress class="progress w-full" :class="getProgressClass(selectedReviewPlugin.quality_score)"
                    :value="selectedReviewPlugin.quality_score" max="100"></progress>
                  <span class="font-semibold">{{ selectedReviewPlugin.quality_score }}%</span>
                </div>
              </div>
            </div>
          </div>
        </div>

        <!-- Quality Breakdown -->
        <div v-if="selectedReviewPlugin.quality_breakdown" class="card bg-base-200">
          <div class="card-body p-4">
            <h4 class="font-semibold mb-3">{{ $t('plugins.qualityBreakdown', '质量评分细分') }}</h4>
            <div class="grid grid-cols-2 lg:grid-cols-4 gap-4">
              <div v-for="(item, key) in qualityBreakdownItems" :key="key" class="text-center">
                <div class="text-xs text-gray-500 mb-2">{{ item.label }}</div>
                <div class="radial-progress" :style="`--value:${item.score}; --size:4rem;`" :class="getScoreTextClass(item.score)">
                  {{ item.score }}%
                </div>
              </div>
            </div>
          </div>
        </div>

        <!-- Validation Result -->
        <div v-if="selectedReviewPlugin.validation" class="card bg-base-200">
          <div class="card-body p-4">
            <h4 class="font-semibold mb-3">{{ $t('plugins.validationResult', '验证结果') }}</h4>
            <div class="alert" :class="selectedReviewPlugin.validation.is_valid ? 'alert-success' : 'alert-error'">
              <i :class="selectedReviewPlugin.validation.is_valid ? 'fas fa-check-circle' : 'fas fa-exclamation-circle'"></i>
              <div>
                <div class="font-semibold">
                  {{ selectedReviewPlugin.validation.is_valid ? $t('plugins.validationPassed', '验证通过') : $t('plugins.validationFailed', '验证失败') }}
                </div>
                <div v-if="selectedReviewPlugin.validation.errors.length > 0" class="mt-2">
                  <strong>{{ $t('plugins.errors', '错误') }}:</strong>
                  <ul class="list-disc list-inside mt-1">
                    <li v-for="(error, index) in selectedReviewPlugin.validation.errors" :key="index" class="text-sm">{{ error }}</li>
                  </ul>
                </div>
                <div v-if="selectedReviewPlugin.validation.warnings.length > 0" class="mt-2">
                  <strong>{{ $t('plugins.warnings', '警告') }}:</strong>
                  <ul class="list-disc list-inside mt-1">
                    <li v-for="(warning, index) in selectedReviewPlugin.validation.warnings" :key="index" class="text-sm">{{ warning }}</li>
                  </ul>
                </div>
              </div>
            </div>
          </div>
        </div>

        <!-- Code Editor -->
        <div class="card bg-base-200">
          <div class="card-body p-4">
            <div class="flex justify-between items-center mb-3">
              <h4 class="font-semibold">{{ $t('plugins.pluginCode', '插件代码') }}</h4>
              <div class="flex gap-2">
                <button class="btn btn-sm btn-outline" @click="$emit('copyReviewCode')">
                  <i class="fas fa-copy mr-1"></i>{{ $t('plugins.copy', '复制') }}
                </button>
                <button class="btn btn-sm btn-outline" @click="$emit('toggleReviewEditMode')">
                  <i class="fas fa-edit mr-1"></i>
                  {{ reviewEditMode ? $t('plugins.readonly', '只读') : $t('common.edit', '编辑') }}
                </button>
              </div>
            </div>
            <div ref="reviewCodeEditorContainerRef" class="border border-base-300 rounded-lg overflow-hidden min-h-96"></div>
          </div>
        </div>
      </div>

      <div class="modal-action sticky bottom-0 bg-base-100 pt-4">
        <button class="btn btn-sm" @click="closeReviewDetailDialog">{{ $t('common.close', '关闭') }}</button>
        <button v-if="reviewEditMode" class="btn btn-primary btn-sm" @click="$emit('saveReviewEdit')" :disabled="savingReview">
          <span v-if="savingReview" class="loading loading-spinner"></span>
          {{ savingReview ? $t('common.saving', '保存中...') : $t('common.save', '保存') }}
        </button>
        <button class="btn btn-success btn-sm" @click="$emit('approveReviewPlugin')" :disabled="selectedReviewPlugin?.status === 'Approved'">
          <i class="fas fa-check mr-1"></i>{{ $t('plugins.approve', '批准') }}
        </button>
        <button class="btn btn-error btn-sm" @click="$emit('rejectReviewPlugin')" :disabled="selectedReviewPlugin?.status === 'Rejected'">
          <i class="fas fa-times mr-1"></i>{{ $t('plugins.reject', '拒绝') }}
        </button>
      </div>
    </div>
    <form method="dialog" class="modal-backdrop"><button @click="closeReviewDetailDialog">close</button></form>
  </dialog>

  <!-- Upload Plugin Dialog -->
  <dialog ref="uploadDialogRef" class="modal">
    <div class="modal-box max-h-[90vh] overflow-y-auto">
      <div class="flex justify-between items-start mb-4 sticky top-0 bg-base-100 z-10 pb-2">
        <h3 class="font-bold text-lg">{{ $t('plugins.uploadPlugin', '上传插件') }}</h3>
        <button @click="closeUploadDialog" class="btn btn-sm btn-circle btn-ghost">✕</button>
      </div>
      <div class="form-control w-full">
        <label class="label"><span class="label-text">{{ $t('plugins.selectFile', '选择插件文件 (.ts / .js)') }}</span></label>
        <input type="file" class="file-input file-input-bordered w-full" accept=".ts,.js" @change="$emit('handleFileSelect', $event)" ref="fileInputRef" />
      </div>
      <div v-if="uploadError" class="alert alert-error mt-4">
        <i class="fas fa-exclamation-circle"></i><span>{{ uploadError }}</span>
      </div>
      <div class="modal-action sticky bottom-0 bg-base-100 pt-4">
        <button class="btn btn-sm" @click="closeUploadDialog">{{ $t('common.cancel', '取消') }}</button>
        <button class="btn btn-primary btn-sm" :disabled="!selectedFile || uploading" @click="$emit('uploadPlugin')">
          <span v-if="uploading" class="loading loading-spinner"></span>
          {{ uploading ? $t('plugins.uploading', '上传中...') : $t('plugins.upload', '上传') }}
        </button>
      </div>
    </div>
    <form method="dialog" class="modal-backdrop"><button @click="closeUploadDialog">close</button></form>
  </dialog>

  <!-- Delete Confirmation Dialog -->
  <dialog ref="deleteDialogRef" class="modal">
    <div class="modal-box max-h-[90vh] overflow-y-auto">
      <div class="flex justify-between items-start mb-4 sticky top-0 bg-base-100 z-10 pb-2">
        <h3 class="font-bold text-lg">{{ $t('plugins.confirmDelete', '确认删除') }}</h3>
        <button @click="closeDeleteDialog" class="btn btn-sm btn-circle btn-ghost">✕</button>
      </div>
      <p class="py-4">
        {{ $t('plugins.deleteConfirmText', '确定要删除插件') }} <strong>{{ deletingPlugin?.metadata.name }}</strong>
        {{ $t('plugins.deleteWarning', '吗？此操作不可撤销。') }}
      </p>
      <div class="modal-action sticky bottom-0 bg-base-100 pt-4">
        <button class="btn btn-sm" @click="closeDeleteDialog">{{ $t('common.cancel', '取消') }}</button>
        <button class="btn btn-error btn-sm" :disabled="deleting" @click="$emit('deletePlugin')">
          <span v-if="deleting" class="loading loading-spinner"></span>
          {{ deleting ? $t('plugins.deleting', '删除中...') : $t('common.delete', '删除') }}
        </button>
      </div>
    </div>
    <form method="dialog" class="modal-backdrop"><button @click="closeDeleteDialog">close</button></form>
  </dialog>

  <!-- AI Generate Plugin Dialog -->
  <dialog ref="aiGenerateDialogRef" class="modal">
    <div class="modal-box w-11/12 max-w-3xl">
      <h3 class="font-bold text-base mb-4">
        <i class="fas fa-magic mr-2"></i>{{ $t('plugins.aiGenerate', 'AI生成插件') }}
      </h3>
      <div class="space-y-4">
        <div class="form-control">
          <label class="label"><span class="label-text">{{ $t('plugins.aiPrompt', '描述你想要的插件功能') }}</span></label>
          <textarea :value="aiPrompt" @input="$emit('update:aiPrompt', ($event.target as HTMLTextAreaElement).value)" class="textarea textarea-bordered h-32"
            :placeholder="$t('plugins.aiPromptPlaceholder', '例如：我需要一个检测SQL注入漏洞的插件...')"></textarea>
        </div>
        <div class="grid grid-cols-2 gap-4">
          <div class="form-control">
            <label class="label"><span class="label-text">{{ $t('plugins.pluginType', '插件类型') }}</span></label>
            <select :value="aiPluginType" @change="$emit('update:aiPluginType', ($event.target as HTMLSelectElement).value)" class="select select-bordered select-sm">
              <option value="traffic">{{ $t('plugins.categories.trafficAnalysis', '流量分析插件') }}</option>
              <option value="agent">{{ $t('plugins.categories.agents', 'Agent工具插件') }}</option>
            </select>
          </div>
          <div class="form-control">
            <label class="label"><span class="label-text">{{ $t('plugins.severity', '严重程度') }}</span></label>
            <select :value="aiSeverity" @change="$emit('update:aiSeverity', ($event.target as HTMLSelectElement).value)" class="select select-bordered select-sm">
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
          <i class="fas fa-exclamation-circle"></i><span>{{ aiGenerateError }}</span>
        </div>
      </div>
      <div class="modal-action">
        <button class="btn" @click="closeAIGenerateDialog">{{ $t('common.cancel', '取消') }}</button>
        <button class="btn btn-primary" :disabled="!aiPrompt.trim() || aiGenerating" @click="$emit('generatePluginWithAi')">
          <i class="fas fa-magic mr-2"></i>{{ $t('plugins.generatePlugin', '生成插件') }}
        </button>
      </div>
    </div>
    <form method="dialog" class="modal-backdrop"><button @click="closeAIGenerateDialog">close</button></form>
  </dialog>

  <!-- Test Result Dialog -->
  <dialog ref="testResultDialogRef" class="modal">
    <div class="modal-box w-11/12 max-w-3xl">
      <h3 class="font-bold text-base mb-4">
        <i class="fas fa-vial mr-2"></i>{{ $t('plugins.testResult', '插件测试结果') }}
      </h3>
      <div v-if="testing" class="alert alert-info">
        <span class="loading loading-spinner"></span>
        <span>{{ $t('plugins.testing', '正在测试插件...') }}</span>
      </div>
      <div v-else-if="testResult" class="space-y-4">
        <!-- Status Alert -->
        <div class="alert" :class="{ 'alert-success': testResult.success, 'alert-error': !testResult.success }">
          <i :class="testResult.success ? 'fas fa-check-circle' : 'fas fa-times-circle'"></i>
          <span>{{ testResult.success ? $t('plugins.testPassed', '测试通过') : $t('plugins.testFailed', '测试失败') }}</span>
        </div>
        
        <!-- Failed: Show error message only -->
        <div v-if="!testResult.success" class="card bg-base-200">
          <div class="card-body">
            <h4 class="font-semibold mb-2 text-error">{{ $t('plugins.errorInfo', '错误信息') }}</h4>
            <pre class="text-sm whitespace-pre-wrap break-all text-error/80">{{ testResult.error || testResult.message || $t('plugins.unknownError', '未知错误') }}</pre>
          </div>
        </div>
        
        <!-- Success: Show message and findings -->
        <template v-else>
          <div v-if="testResult.message" class="card bg-base-200">
            <div class="card-body">
              <h4 class="font-semibold mb-2">{{ $t('plugins.testMessage', '测试消息') }}</h4>
              <pre class="text-sm whitespace-pre-wrap">{{ testResult.message }}</pre>
            </div>
          </div>
          <div v-if="testResult.findings && testResult.findings.length > 0" class="card bg-base-200">
            <div class="card-body">
              <h4 class="font-semibold mb-2">{{ $t('plugins.findings', '发现') }} ({{ testResult.findings.length }})</h4>
              <div class="space-y-2">
                <div v-for="(finding, idx) in testResult.findings" :key="idx" class="card bg-base-100">
                  <div class="card-body p-3">
                    <div class="flex justify-between items-start">
                      <span class="font-medium">{{ finding.title }}</span>
                      <span class="badge" :class="getSeverityBadgeClass(finding.severity)">{{ finding.severity }}</span>
                    </div>
                    <p class="text-sm text-base-content/70 mt-1 whitespace-pre-wrap break-all">{{ finding.description }}</p>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </template>
      </div>
      <div class="modal-action">
        <button v-if="isFullscreenEditorMode" class="btn btn-primary" @click="handleReferToAi">
          <i class="fas fa-robot mr-2"></i>{{ $t('plugins.referToAi', '引用到AI助手') }}
        </button>
        <button class="btn" @click="closeTestResultDialog">{{ $t('common.close', '关闭') }}</button>
      </div>
    </div>
    <form method="dialog" class="modal-backdrop"><button @click="closeTestResultDialog">close</button></form>
  </dialog>

  <!-- Advanced Test Dialog -->
  <dialog ref="advancedDialogRef" class="modal">
    <div class="modal-box w-11/12 max-w-5xl">
      <h3 class="font-bold text-base mb-4">
        <i class="fas fa-gauge-high mr-2"></i>{{ $t('plugins.advancedTest', '高级测试') }}
        <span v-if="advancedPlugin" class="text-sm font-normal text-gray-500 ml-2">
          {{ advancedPlugin.metadata.name }} ({{ advancedPlugin.metadata.id }})
        </span>
      </h3>
      <div class="grid grid-cols-2 gap-4">
        <!-- Agent plugin inputs -->
        <div v-if="isAdvancedAgent" class="form-control col-span-2">
          <label class="label"><span class="label-text">{{ $t('plugins.agentInputs', '插件入参 (JSON)') }}</span></label>
          <textarea :value="advancedForm.agent_inputs_text" @input="$emit('update:advancedForm', { ...advancedForm, agent_inputs_text: ($event.target as HTMLTextAreaElement).value })"
            class="textarea textarea-bordered font-mono text-xs h-32" placeholder='{"target":"https://example.com"}'></textarea>
        </div>
        <!-- Traffic analysis inputs -->
        <template v-else>
          <div class="form-control col-span-2">
            <label class="label"><span class="label-text">{{ $t('plugins.requestUrl', '请求 URL') }}</span></label>
            <input :value="advancedForm.url" @input="$emit('update:advancedForm', { ...advancedForm, url: ($event.target as HTMLInputElement).value })"
              type="text" class="input input-bordered input-sm w-full" placeholder="https://example.com/test" />
          </div>
          <div class="form-control">
            <label class="label"><span class="label-text">{{ $t('plugins.httpMethod', 'HTTP 方法') }}</span></label>
            <select :value="advancedForm.method" @change="$emit('update:advancedForm', { ...advancedForm, method: ($event.target as HTMLSelectElement).value })"
              class="select select-bordered select-sm w-full">
              <option>GET</option><option>POST</option><option>PUT</option><option>DELETE</option><option>PATCH</option>
            </select>
          </div>
          <div class="form-control col-span-2">
            <label class="label"><span class="label-text">{{ $t('plugins.headersJson', '请求头 (JSON)') }}</span></label>
            <textarea :value="advancedForm.headersText" @input="$emit('update:advancedForm', { ...advancedForm, headersText: ($event.target as HTMLTextAreaElement).value })"
              class="textarea textarea-bordered font-mono text-xs h-24" placeholder='{"User-Agent":"Test"}'></textarea>
          </div>
          <div class="form-control col-span-2">
            <label class="label"><span class="label-text">{{ $t('plugins.body', '请求体') }}</span></label>
            <textarea :value="advancedForm.bodyText" @input="$emit('update:advancedForm', { ...advancedForm, bodyText: ($event.target as HTMLTextAreaElement).value })"
              class="textarea textarea-bordered font-mono text-xs h-24"></textarea>
          </div>
        </template>
        <!-- Common inputs -->
        <div class="form-control">
          <label class="label"><span class="label-text">{{ $t('plugins.runs', '运行次数') }}</span></label>
          <input :value="advancedForm.runs" @input="$emit('update:advancedForm', { ...advancedForm, runs: Number(($event.target as HTMLInputElement).value) })"
            type="number" min="1" class="input input-bordered input-sm w-full" />
        </div>
        <div class="form-control">
          <label class="label"><span class="label-text">{{ $t('plugins.concurrency', '并发数') }}</span></label>
          <input :value="advancedForm.concurrency" @input="$emit('update:advancedForm', { ...advancedForm, concurrency: Number(($event.target as HTMLInputElement).value) })"
            type="number" min="1" class="input input-bordered input-sm w-full" />
        </div>
      </div>
      <div v-if="advancedError" class="alert alert-error mt-4">
        <i class="fas fa-exclamation-circle"></i><span>{{ advancedError }}</span>
      </div>
      <div class="mt-4">
        <button class="btn btn-primary" :disabled="advancedTesting" @click="$emit('runAdvancedTest')">
          <span v-if="advancedTesting" class="loading loading-spinner"></span>
          {{ advancedTesting ? $t('plugins.testing', '正在测试...') : $t('plugins.startTest', '开始测试') }}
        </button>
      </div>
      <div v-if="advancedTesting" class="alert alert-info mt-4">
        <span class="loading loading-spinner"></span>
        <span class="ml-2">{{ $t('plugins.testing', '正在测试插件...') }}</span>
      </div>
      <div v-else-if="advancedResult" class="space-y-4 mt-4">
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
        <div class="card bg-base-200">
          <div class="card-body">
            <h4 class="font-semibold mb-2">{{ $t('plugins.runDetails', '运行详情') }}</h4>
            <div class="overflow-x-auto">
              <table class="table table-zebra w-full">
                <thead><tr><th>#</th><th>{{ $t('plugins.duration', '耗时(ms)') }}</th><th>{{ $t('plugins.findings', '发现') }}</th><th>{{ $t('plugins.error', '错误') }}</th></tr></thead>
                <tbody>
                  <tr v-for="r in sortedRuns" :key="r.run_index">
                    <td>{{ r.run_index }}</td><td>{{ r.duration_ms }}</td><td>{{ r.findings }}</td>
                    <td><span v-if="r.error" class="badge badge-error">{{ r.error }}</span><span v-else class="badge badge-success">OK</span></td>
                  </tr>
                </tbody>
              </table>
            </div>
          </div>
        </div>
        
        <!-- Detailed execution outputs -->
        <div v-if="sortedRuns.length > 0" class="space-y-3">
          <div v-for="run in sortedRuns" :key="run.run_index" class="collapse collapse-arrow bg-base-200">
            <input type="checkbox" :id="`run-output-${run.run_index}`" />
            <label :for="`run-output-${run.run_index}`" class="collapse-title font-medium flex items-center gap-2">
              <span>{{ $t('plugins.runOutput', '运行') }} #{{ run.run_index }} {{ $t('plugins.executionResult', '执行结果') }}</span>
              <span class="badge badge-sm" :class="run.error ? 'badge-error' : 'badge-success'">
                {{ run.error ? $t('plugins.failed', '失败') : $t('plugins.success', '成功') }}
              </span>
            </label>
            <div class="collapse-content">
              <div v-if="run.output" class="card bg-base-100">
                <div class="card-body p-4">
                  <div class="flex justify-between items-center mb-2">
                    <h5 class="font-semibold text-sm">{{ $t('plugins.agentToolResult', 'Agent工具执行结果') }}</h5>
                    <div class="join">
                      <button 
                        @click="copyRunOutput(run)" 
                        class="btn btn-xs btn-ghost join-item"
                        :title="$t('tools.copyResult', '复制结果')"
                      >
                        <i class="fas fa-copy"></i>
                      </button>
                      <button 
                        @click="toggleRunJsonView(run.run_index)" 
                        class="btn btn-xs btn-ghost join-item"
                        :class="{ 'btn-active': runJsonViewStates[run.run_index] }"
                        :title="$t('tools.toggleJsonView', '切换JSON渲染')"
                      >
                        <i class="fas fa-code"></i>
                      </button>
                    </div>
                  </div>
                  
                  <!-- 原始文本视图 -->
                  <pre 
                    v-if="!runJsonViewStates[run.run_index]"
                    class="text-xs overflow-x-auto whitespace-pre-wrap break-all bg-base-200 p-3 rounded"
                  >{{ JSON.stringify(run.output, null, 2) }}</pre>
                  
                  <!-- JSON渲染视图 -->
                  <div 
                    v-else 
                    class="text-xs overflow-x-auto bg-base-200 p-3 rounded max-h-96"
                  >
                    <JsonViewer :data="run.output" :expanded="true" />
                  </div>
                </div>
              </div>
              <div v-else class="alert alert-warning">
                <i class="fas fa-exclamation-triangle"></i>
                <span>{{ $t('plugins.noOutputData', '无输出数据') }}</span>
              </div>
            </div>
          </div>
        </div>
      </div>
      <div class="modal-action">
        <button class="btn" @click="closeAdvancedDialog">{{ $t('common.close', '关闭') }}</button>
      </div>
    </div>
    <form method="dialog" class="modal-backdrop"><button @click="closeAdvancedDialog">close</button></form>
  </dialog>
</template>

<script setup lang="ts">
import { ref, computed, watch, reactive } from 'vue'
import { useI18n } from 'vue-i18n'
import { dialog } from '@/composables/useDialog'
import JsonViewer from '@/components/Tools/JsonViewer.vue'
import type { PluginRecord, ReviewPlugin, TestResult, AdvancedTestResult, AdvancedRunStat, AdvancedForm } from './types'

const { t } = useI18n()

const props = defineProps<{
  // Review dialog
  selectedReviewPlugin: ReviewPlugin | null
  reviewEditMode: boolean
  savingReview: boolean
  // Upload dialog
  selectedFile: File | null
  uploading: boolean
  uploadError: string
  // Delete dialog
  deletingPlugin: PluginRecord | null
  deleting: boolean
  // AI Generate dialog
  aiPrompt: string
  aiPluginType: string
  aiSeverity: string
  aiGenerating: boolean
  aiGenerateError: string
  // Test dialog
  testing: boolean
  testResult: TestResult | null
  isFullscreenEditorMode: boolean  // Whether in fullscreen code editor mode
  // Advanced test dialog
  advancedPlugin: PluginRecord | null
  advancedTesting: boolean
  advancedError: string
  advancedResult: AdvancedTestResult | null
  advancedForm: AdvancedForm
  isAdvancedAgent: boolean
  sortedRuns: AdvancedRunStat[]
}>()

const emit = defineEmits<{
  'copyReviewCode': []
  'toggleReviewEditMode': []
  'saveReviewEdit': []
  'approveReviewPlugin': []
  'rejectReviewPlugin': []
  'handleFileSelect': [event: Event]
  'uploadPlugin': []
  'deletePlugin': []
  'update:aiPrompt': [value: string]
  'update:aiPluginType': [value: string]
  'update:aiSeverity': [value: string]
  'generatePluginWithAi': []
  'runAdvancedTest': []
  'update:advancedForm': [value: AdvancedForm]
  'closeReviewDetailDialog': []
  'closeUploadDialog': []
  'closeDeleteDialog': []
  'closeAiGenerateDialog': []
  'closeTestResultDialog': []
  'closeAdvancedDialog': []
  'referTestResultToAi': []
}>()

// Dialog refs
const reviewDetailDialogRef = ref<HTMLDialogElement>()
const uploadDialogRef = ref<HTMLDialogElement>()
const deleteDialogRef = ref<HTMLDialogElement>()
const aiGenerateDialogRef = ref<HTMLDialogElement>()
const testResultDialogRef = ref<HTMLDialogElement>()
const advancedDialogRef = ref<HTMLDialogElement>()
const fileInputRef = ref<HTMLInputElement>()
const reviewCodeEditorContainerRef = ref<HTMLDivElement>()

// Quality breakdown items
const qualityBreakdownItems = computed(() => {
  if (!props.selectedReviewPlugin?.quality_breakdown) return []
  const breakdown = props.selectedReviewPlugin.quality_breakdown
  return [
    { key: 'syntax_score', label: t('plugins.syntaxScore', '语法正确性'), score: breakdown.syntax_score },
    { key: 'logic_score', label: t('plugins.logicScore', '逻辑完整性'), score: breakdown.logic_score },
    { key: 'security_score', label: t('plugins.securityScore', '安全性'), score: breakdown.security_score },
    { key: 'code_quality_score', label: t('plugins.codeQuality', '代码质量'), score: breakdown.code_quality_score },
  ]
})

// Helper methods
const getVulnTypeBadgeClass = (vulnType: string): string => {
  const classMap: Record<string, string> = {
    'sqli': 'badge-error', 'command_injection': 'badge-error',
    'xss': 'badge-warning', 'idor': 'badge-info', 'auth_bypass': 'badge-info',
    'csrf': 'badge-primary', 'info_leak': 'badge-success',
    'file_upload': 'badge-secondary', 'file_inclusion': 'badge-secondary',
    'path_traversal': 'badge-accent', 'xxe': 'badge-neutral', 'ssrf': 'badge-neutral'
  }
  return classMap[vulnType] || 'badge-ghost'
}

const getProgressClass = (score: number): string => {
  if (score >= 80) return 'progress-success'
  if (score >= 60) return 'progress-warning'
  return 'progress-error'
}

const getScoreTextClass = (score: number): string => {
  if (score >= 80) return 'text-success'
  if (score >= 60) return 'text-warning'
  return 'text-error'
}

const getSeverityBadgeClass = (severity: string): string => {
  const classMap: Record<string, string> = {
    'critical': 'badge-error', 'high': 'badge-error',
    'medium': 'badge-warning', 'low': 'badge-info', 'info': 'badge-info'
  }
  return classMap[severity] || 'badge-ghost'
}

// JSON view states for each run
const runJsonViewStates = reactive<Record<number, boolean>>({})

// Copy run output to clipboard
async function copyRunOutput(run: AdvancedRunStat) {
  try {
    const outputText = JSON.stringify(run.output, null, 2)
    await navigator.clipboard.writeText(outputText)
    dialog.toast.success(t('tools.copiedToClipboard'))
  } catch (error) {
    console.error('Failed to copy run output:', error)
    dialog.toast.error(t('tools.copyFailed'))
  }
}

// Toggle JSON view for a specific run
function toggleRunJsonView(runIndex: number) {
  runJsonViewStates[runIndex] = !runJsonViewStates[runIndex]
}

// Dialog methods
const showReviewDetailDialog = () => reviewDetailDialogRef.value?.showModal()
const closeReviewDetailDialog = () => { reviewDetailDialogRef.value?.close(); emit('closeReviewDetailDialog') }
const showUploadDialog = () => uploadDialogRef.value?.showModal()
const closeUploadDialog = () => { uploadDialogRef.value?.close(); emit('closeUploadDialog') }
const showDeleteDialog = () => deleteDialogRef.value?.showModal()
const closeDeleteDialog = () => { deleteDialogRef.value?.close(); emit('closeDeleteDialog') }
const showAIGenerateDialog = () => aiGenerateDialogRef.value?.showModal()
const closeAIGenerateDialog = () => { aiGenerateDialogRef.value?.close(); emit('closeAiGenerateDialog') }
const showTestResultDialog = () => testResultDialogRef.value?.showModal()
const closeTestResultDialog = () => { testResultDialogRef.value?.close(); emit('closeTestResultDialog') }
const handleReferToAi = () => {
  emit('referTestResultToAi')
  closeTestResultDialog()
}
const showAdvancedDialog = () => advancedDialogRef.value?.showModal()
const closeAdvancedDialog = () => { advancedDialogRef.value?.close(); emit('closeAdvancedDialog') }

defineExpose({
  showReviewDetailDialog, closeReviewDetailDialog,
  showUploadDialog, closeUploadDialog,
  showDeleteDialog, closeDeleteDialog,
  showAIGenerateDialog, closeAIGenerateDialog,
  showTestResultDialog, closeTestResultDialog,
  showAdvancedDialog, closeAdvancedDialog,
  reviewCodeEditorContainerRef, fileInputRef
})
</script>
