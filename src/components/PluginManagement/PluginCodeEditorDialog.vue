<template>
  <!-- Code Editor Dialog -->
  <dialog ref="codeEditorDialogRef" class="modal" :class="{ 'fullscreen-mode-active': isFullscreenEditor }" @cancel="handleDialogCancel">
    <div class="modal-box w-11/12 max-w-5xl max-h-[90vh] overflow-y-auto" :class="{ 'invisible': isFullscreenEditor }">
      <div class="flex justify-between items-start mb-4 sticky top-0 bg-base-100 z-10 pb-2">
        <h3 class="font-bold text-lg">
          {{ editingPlugin ? $t('plugins.codeEditor', '插件代码编辑器') : $t('plugins.newPlugin', '新增插件') }}
          <span v-if="editingPlugin" class="text-sm font-normal text-gray-500 ml-2">
            {{ editingPlugin.metadata.name }} ({{ editingPlugin.metadata.id }})
          </span>
        </h3>
        <button @click="closeDialog" class="btn btn-sm btn-circle btn-ghost">✕</button>
      </div>

      <!-- Plugin Metadata Form -->
      <div class="grid grid-cols-2 gap-4 mb-4">
        <div class="form-control">
          <label class="label">
            <span class="label-text">{{ $t('plugins.pluginId', '插件ID') }} <span class="text-error">*</span></span>
          </label>
          <input :value="newPluginMetadata.id" @input="updateMetadata('id', ($event.target as HTMLInputElement).value)"
            type="text" :placeholder="$t('plugins.pluginIdPlaceholder', '例如: sql_injection_scanner')"
            class="input input-bordered input-sm" :disabled="!!editingPlugin" />
        </div>

        <div class="form-control">
          <label class="label">
            <span class="label-text">{{ $t('plugins.pluginName', '插件名称') }} <span class="text-error">*</span></span>
          </label>
          <input :value="newPluginMetadata.name" @input="updateMetadata('name', ($event.target as HTMLInputElement).value)"
            type="text" :placeholder="$t('plugins.pluginNamePlaceholder', '例如: SQL注入扫描器')"
            class="input input-bordered input-sm" :disabled="editingPlugin && !isEditing" />
        </div>

        <div class="form-control">
          <label class="label"><span class="label-text">{{ $t('plugins.version', '版本') }}</span></label>
          <input :value="newPluginMetadata.version" @input="updateMetadata('version', ($event.target as HTMLInputElement).value)"
            type="text" placeholder="1.0.0" class="input input-bordered input-sm" :disabled="editingPlugin && !isEditing" />
        </div>

        <div class="form-control">
          <label class="label"><span class="label-text">{{ $t('plugins.author', '作者') }}</span></label>
          <input :value="newPluginMetadata.author" @input="updateMetadata('author', ($event.target as HTMLInputElement).value)"
            type="text" :placeholder="$t('plugins.authorPlaceholder', '作者名称')"
            class="input input-bordered input-sm" :disabled="editingPlugin && !isEditing" />
        </div>

        <div class="form-control">
          <label class="label">
            <span class="label-text">{{ $t('plugins.mainCategory', '主分类') }} <span class="text-error">*</span></span>
          </label>
          <select :value="newPluginMetadata.mainCategory" @change="updateMetadata('mainCategory', ($event.target as HTMLSelectElement).value)"
            class="select select-bordered select-sm" :disabled="editingPlugin && !isEditing">
            <option v-for="cat in mainCategories" :key="cat.value" :value="cat.value">{{ cat.label }}</option>
          </select>
        </div>

        <div class="form-control">
          <label class="label">
            <span class="label-text">{{ $t('plugins.subCategory', '子分类') }} <span class="text-error">*</span></span>
          </label>
          <select :value="newPluginMetadata.category" @change="updateMetadata('category', ($event.target as HTMLSelectElement).value)"
            class="select select-bordered select-sm" :disabled="editingPlugin && !isEditing">
            <option v-for="cat in subCategories" :key="cat.value" :value="cat.value">{{ cat.label }}</option>
          </select>
        </div>

        <div class="form-control">
          <label class="label"><span class="label-text">{{ $t('plugins.defaultSeverity', '默认严重程度') }}</span></label>
          <select :value="newPluginMetadata.default_severity" @change="updateMetadata('default_severity', ($event.target as HTMLSelectElement).value)"
            class="select select-bordered select-sm" :disabled="editingPlugin && !isEditing">
            <option value="info">{{ $t('common.info', '信息') }}</option>
            <option value="low">{{ $t('common.low', '低危') }}</option>
            <option value="medium">{{ $t('common.medium', '中危') }}</option>
            <option value="high">{{ $t('common.high', '高危') }}</option>
            <option value="critical">{{ $t('common.critical', '严重') }}</option>
          </select>
        </div>

        <div class="form-control col-span-2">
          <label class="label"><span class="label-text">{{ $t('common.description', '描述') }}</span></label>
          <input :value="newPluginMetadata.description" @input="updateMetadata('description', ($event.target as HTMLInputElement).value)"
            type="text" :placeholder="$t('plugins.descriptionPlaceholder', '插件功能描述')"
            class="input input-bordered input-sm" :disabled="editingPlugin && !isEditing" />
        </div>

        <div class="form-control col-span-2">
          <label class="label">
            <span class="label-text">{{ $t('plugins.tags', '标签') }} ({{ $t('plugins.commaSeparated', '逗号分隔') }})</span>
          </label>
          <input :value="newPluginMetadata.tagsString" @input="updateMetadata('tagsString', ($event.target as HTMLInputElement).value)"
            type="text" :placeholder="$t('plugins.tagsPlaceholder', '例如: security, scanner, sql')"
            class="input input-bordered input-sm" :disabled="editingPlugin && !isEditing" />
        </div>
      </div>

      <!-- Code Editor -->
      <div class="form-control w-full">
        <div class="flex justify-between items-center mb-2">
          <label class="label"><span class="label-text">{{ $t('plugins.pluginCode', '插件代码') }}</span></label>
          <div class="flex gap-2">
            <button v-if="!editingPlugin" class="btn btn-xs btn-outline" @click="$emit('insertTemplate')">
              <i class="fas fa-file-code mr-1"></i>{{ $t('plugins.insertTemplate', '插入模板') }}
            </button>
            <button class="btn btn-xs btn-outline" @click="$emit('formatCode')">
              <i class="fas fa-indent mr-1"></i>{{ $t('plugins.format', '格式化') }}
            </button>
            <button class="btn btn-xs btn-outline" @click="$emit('copyPlugin')">
              <i class="fas fa-copy mr-1"></i>{{ $t('plugins.copyPlugin', '复制插件') }}
            </button>
            <button class="btn btn-xs btn-outline" @click="$emit('toggleFullscreen')">
              <i :class="isFullscreenEditor ? 'fas fa-compress mr-1' : 'fas fa-expand mr-1'"></i>
              {{ isFullscreenEditor ? '退出全屏' : '全屏' }}
            </button>
          </div>
        </div>
        <div class="relative border border-base-300 rounded-lg overflow-hidden min-h-96">
          <div ref="codeEditorContainerRef"></div>
        </div>
      </div>

      <div v-if="codeError" class="alert alert-error mt-4">
        <i class="fas fa-exclamation-circle"></i><span>{{ codeError }}</span>
      </div>

      <div class="modal-action sticky bottom-0 bg-base-100 pt-4">
        <button class="btn btn-sm" @click="closeDialog">{{ $t('common.close', '关闭') }}</button>
        <template v-if="editingPlugin">
          <button v-if="!isEditing" class="btn btn-primary btn-sm" @click="$emit('enableEditing')">
            <i class="fas fa-edit mr-2"></i>{{ $t('common.edit', '编辑') }}
          </button>
          <template v-else>
            <button class="btn btn-warning btn-sm" @click="$emit('cancelEditing')">{{ $t('plugins.cancelEdit', '取消编辑') }}</button>
            <button class="btn btn-success btn-sm" :disabled="saving" @click="$emit('savePlugin')">
              <span v-if="saving" class="loading loading-spinner"></span>
              {{ saving ? $t('common.saving', '保存中...') : $t('common.save', '保存') }}
            </button>
          </template>
        </template>
        <template v-else>
          <button class="btn btn-success btn-sm" :disabled="saving || !isNewPluginValid" @click="$emit('createNewPlugin')">
            <span v-if="saving" class="loading loading-spinner"></span>
            {{ saving ? $t('plugins.creating', '创建中...') : $t('plugins.createPlugin', '创建插件') }}
          </button>
        </template>
      </div>
    </div>
    <form method="dialog" class="modal-backdrop" :class="{ 'invisible pointer-events-none': isFullscreenEditor }"><button @click="closeDialog">close</button></form>
  </dialog>

  <!-- Fullscreen Editor Overlay -->
  <Teleport to="body">
    <div v-if="isFullscreenEditor" ref="fullscreenOverlayRef" class="fullscreen-editor-overlay">
      <!-- Main content with AI panel -->
      <div class="fullscreen-editor-layout">
        <!-- Code Editor Area -->
        <div class="fullscreen-editor-content" :class="{ 'with-ai-panel': showAiPanel }" :style="showAiPanel ? { width: `calc(100% - ${aiPanelWidth}px)` } : {}">
          <div v-show="!isPreviewMode" ref="fullscreenCodeEditorContainerRef" class="h-full w-full"></div>
          <div v-show="isPreviewMode" ref="fullscreenDiffEditorContainerRef" class="h-full w-full">
            <!-- Diff View Header -->
            <div v-if="isPreviewMode" class="diff-header">
              <div class="diff-header-content">
                <i class="fas fa-code-compare mr-2"></i>
                <span class="font-semibold">{{ $t('plugins.codeComparison', '代码对比') }}</span>
                <span class="text-sm opacity-70 ml-2">{{ $t('plugins.leftOriginal', '左侧：当前代码') }} | {{ $t('plugins.rightModified', '右侧：AI修改') }}</span>
              </div>
              <div class="diff-header-actions">
                <button class="btn btn-sm btn-ghost" @click="$emit('exitPreviewMode')">
                  <i class="fas fa-times mr-1"></i>{{ $t('plugins.exitPreview', '退出预览') }}
                </button>
              </div>
            </div>
          </div>
        </div>

        <!-- AI Chat Panel -->
        <div v-if="showAiPanel" class="ai-chat-panel" :style="{ width: aiPanelWidth + 'px' }">
          <!-- Resize Handle -->
          <div class="resize-handle" @mousedown="startResize"></div>

          <div class="ai-chat-header">
            <div class="flex items-center gap-2">
              <i class="fas fa-robot text-primary"></i>
              <span class="font-semibold">{{ $t('plugins.aiAssistant', 'AI 助手') }}</span>
            </div>
            <button class="btn btn-xs btn-ghost btn-circle" @click="$emit('toggleAiPanel')">
              <i class="fas fa-times"></i>
            </button>
          </div>
          
          <!-- Chat Messages -->
          <div class="ai-chat-messages" ref="aiChatMessagesRef">
            <div v-if="aiMessages.length === 0" class="ai-chat-empty">
              <i class="fas fa-comments text-4xl opacity-30 mb-3"></i>
              <p class="text-sm opacity-60">{{ $t('plugins.aiAssistantHint', '描述你想要的修改，AI 将帮助你编辑代码') }}</p>
              <div class="ai-quick-actions mt-4">
                <button class="btn btn-xs btn-outline" @click="$emit('aiQuickAction', 'explain')">
                  <i class="fas fa-lightbulb mr-1"></i>{{ $t('plugins.explainCode', '解释代码') }}
                </button>
                <button class="btn btn-xs btn-outline" @click="$emit('aiQuickAction', 'optimize')">
                  <i class="fas fa-bolt mr-1"></i>{{ $t('plugins.optimizeCode', '优化代码') }}
                </button>
                <button class="btn btn-xs btn-outline" @click="$emit('aiQuickAction', 'fix')">
                  <i class="fas fa-bug mr-1"></i>{{ $t('plugins.fixBugs', '修复问题') }}
                </button>
              </div>
            </div>
            
            <template v-else>
              <div v-for="(msg, idx) in aiMessages" :key="idx" 
                   class="ai-chat-message" :class="msg.role">
                <div class="message-avatar">
                  <i :class="msg.role === 'user' ? 'fas fa-user' : 'fas fa-robot'"></i>
                </div>
                <div class="message-content">
                  <!-- Code reference in user message -->
                  <div v-if="msg.role === 'user' && msg.codeRef" class="message-code-ref">
                    <div class="code-ref-label">
                      <i class="fas fa-code text-xs mr-1"></i>
                      {{ msg.codeRef.isFullCode 
                        ? $t('plugins.fullCode', '完整代码') 
                        : `${$t('plugins.lines', '行')} ${msg.codeRef.startLine}-${msg.codeRef.endLine}` 
                      }}
                    </div>
                    <pre class="code-ref-content"><code>{{ msg.codeRef.preview }}</code></pre>
                  </div>
                  <!-- Test result reference in user message -->
                  <div v-if="msg.role === 'user' && msg.testResultRef" class="message-test-ref">
                    <div class="test-ref-label">
                      <i :class="msg.testResultRef.success ? 'fas fa-check-circle text-success' : 'fas fa-times-circle text-error'" class="text-xs mr-1"></i>
                      {{ $t('plugins.testResultRef', '测试结果') }}
                      <span :class="msg.testResultRef.success ? 'text-success' : 'text-error'">
                        ({{ msg.testResultRef.success ? $t('common.success', '成功') : $t('common.failed', '失败') }})
                      </span>
                    </div>
                    <pre class="test-ref-content"><code>{{ msg.testResultRef.preview }}</code></pre>
                  </div>
                  <div class="message-text" v-html="msg.content"></div>
                  
                  <!-- AI Suggested Changes -->
                  <div v-if="msg.role === 'assistant' && msg.codeBlocks && msg.codeBlocks.length > 0" class="message-ai-suggestions">
                    <div class="suggestions-header">
                      <i class="fas fa-magic text-xs text-primary mr-1"></i>
                      <span class="text-xs font-bold uppercase tracking-wider">{{ $t('plugins.aiSuggestions', 'AI 修改建议') }}</span>
                      <span class="ml-auto text-[10px] opacity-50">{{ msg.codeBlocks.length }} {{ $t('plugins.blocks', '个代码块') }}</span>
                    </div>
                    
                    <div class="suggestions-list">
                      <div v-for="(block, bIdx) in msg.codeBlocks" :key="bIdx" class="suggestion-item">
                        <div class="suggestion-info">
                          <span class="text-[10px] font-mono opacity-70">#{{ bIdx + 1 }}</span>
                          <div class="flex gap-1 ml-auto">
                            <button class="btn btn-mini h-6 min-h-0 btn-primary px-2" @click="handleApplyCode(block, idx)">
                              {{ $t('plugins.apply', '应用') }}
                            </button>
                            <button class="btn btn-mini h-6 min-h-0 btn-ghost px-2" @click="$emit('previewAiCode', block)">
                              {{ $t('plugins.preview', '预览') }}
                            </button>
                          </div>
                        </div>
                        <pre class="suggestion-preview"><code>{{ block.length > 100 ? block.substring(0, 100) + '...' : block }}</code></pre>
                      </div>
                    </div>
                    
                    <button v-if="msg.codeBlocks.length > 1" class="btn btn-xs btn-block btn-outline mt-2" @click="handleApplyAllCode(msg.codeBlocks.join('\n\n'), idx)">
                      <i class="fas fa-check-double mr-1"></i>{{ $t('plugins.applyAll', '全部应用') }}
                    </button>
                  </div>
                </div>
              </div>
              
              <!-- Streaming indicator -->
              <div v-if="aiStreaming" class="ai-chat-message assistant">
                <div class="message-avatar">
                  <i class="fas fa-robot"></i>
                </div>
                <div class="message-content">
                  <div class="message-text streaming-text">
                    <div v-if="aiStreamingContent" v-html="aiStreamingContentRendered"></div>
                    <span class="typing-indicator">
                      <span></span><span></span><span></span>
                    </span>
                  </div>
                </div>
              </div>
            </template>
          </div>
          
          <!-- Chat Input -->
          <div class="ai-chat-input">
            <!-- Code Reference Badge -->
            <div v-if="selectedCodeRef" class="code-reference-badge">
              <div class="code-ref-header">
                <i class="fas fa-code text-xs"></i>
                <span class="text-xs font-medium">
                  {{ selectedCodeRef.isFullCode 
                    ? $t('plugins.fullCode', '完整代码') 
                    : $t('plugins.selectedLines', '选中代码') + ` (${selectedCodeRef.startLine}-${selectedCodeRef.endLine})` 
                  }}
                </span>
                <button class="btn btn-xs btn-ghost btn-circle ml-auto" @click="$emit('clearCodeRef')">
                  <i class="fas fa-times text-xs"></i>
                </button>
              </div>
              <div class="code-ref-preview">
                <pre><code>{{ selectedCodeRef.preview }}</code></pre>
              </div>
            </div>

            <!-- Test Result Reference Badge -->
            <div v-if="selectedTestResultRef" class="test-result-reference-badge">
              <div class="test-ref-header">
                <i :class="selectedTestResultRef.success ? 'fas fa-check-circle text-success' : 'fas fa-times-circle text-error'" class="text-xs"></i>
                <span class="text-xs font-medium">
                  {{ $t('plugins.testResultRef', '测试结果') }}
                  <span :class="selectedTestResultRef.success ? 'text-success' : 'text-error'">
                    ({{ selectedTestResultRef.success ? $t('common.success', '成功') : $t('common.failed', '失败') }})
                  </span>
                </span>
                <button class="btn btn-xs btn-ghost btn-circle ml-auto" @click="$emit('clearTestResultRef')">
                  <i class="fas fa-times text-xs"></i>
                </button>
              </div>
              <div class="test-ref-preview">
                <pre><code>{{ selectedTestResultRef.preview }}</code></pre>
              </div>
            </div>
            
            <textarea 
              v-model="aiInputText"
              :placeholder="$t('plugins.aiInputPlaceholder', '描述你想要的修改...')"
              class="textarea textarea-bordered w-full resize-none"
              rows="2"
              :disabled="aiStreaming"
              @keydown.enter.exact.prevent="$emit('sendAiMessage', aiInputText)"
            ></textarea>
            <div class="ai-chat-input-actions">
              <div class="flex items-center gap-2 text-xs opacity-60">
                <i class="fas fa-info-circle"></i>
                <span>{{ $t('plugins.contextMenuHint', '右键编辑器添加代码到上下文') }}</span>
              </div>
              <button 
                class="btn btn-sm btn-primary" 
                :disabled="!aiInputText.trim() || aiStreaming"
                @click="$emit('sendAiMessage', aiInputText)"
              >
                <span v-if="aiStreaming" class="loading loading-spinner loading-xs"></span>
                <i v-else class="fas fa-paper-plane"></i>
              </button>
            </div>
          </div>
        </div>
      </div>

      <!-- Floating Toolbar - centered in editor area -->
      <div 
        class="fullscreen-floating-toolbar shadow-lg" 
        :class="{ 
          'with-ai-panel': showAiPanel && toolbarPosition.x === null && toolbarPosition.y === null,
          'compact': isCompactToolbar,
          'dragging': isDraggingToolbar
        }"
        :style="{
          left: toolbarPosition.x !== null ? `${toolbarPosition.x}px` : showAiPanel ? 'calc((100% - 400px) / 2)' : '50%',
          top: toolbarPosition.y !== null ? `${toolbarPosition.y}px` : '1rem',
          transform: toolbarPosition.x !== null || toolbarPosition.y !== null ? 'none' : 'translateX(-50%)'
        }"
      >
        <!-- Drag Handle -->
        <div class="toolbar-drag-handle" @mousedown="startToolbarDrag" title="拖动工具栏">
          <i class="fas fa-grip-vertical"></i>
        </div>
        
        <div class="flex items-center gap-3">
          <!-- Info Section (hidden in compact mode) -->
          <div v-if="!isCompactToolbar" class="flex flex-col">
            <span class="font-bold text-sm">
              {{ editingPlugin ? editingPlugin.metadata.name : $t('plugins.newPlugin', '新增插件') }}
            </span>
            <span v-if="editingPlugin" class="text-xs font-mono">{{ editingPlugin.metadata.id }}</span>
          </div>

          <div v-if="!isCompactToolbar" class="toolbar-divider"></div>

          <!-- Actions -->
          <div class="flex items-center gap-2">
            <!-- Compact Toggle -->
            <button 
              class="btn btn-xs btn-ghost" 
              @click="toggleCompactToolbar" 
              :title="isCompactToolbar ? '展开工具栏' : '收起工具栏'"
            >
              <i :class="isCompactToolbar ? 'fas fa-expand-alt' : 'fas fa-compress-alt'"></i>
            </button>
            
            <div class="toolbar-divider"></div>
            
            <!-- AI Toggle Button -->
            <button class="btn btn-sm" :class="showAiPanel ? 'btn-primary' : 'btn-ghost'" 
                    @click="$emit('toggleAiPanel')" :title="$t('plugins.aiAssistant', 'AI 助手')">
              <i class="fas fa-robot"></i>
              <span v-if="!isCompactToolbar" class="ml-1">AI</span>
            </button>

            <div class="toolbar-divider"></div>

            <button v-if="!editingPlugin" class="btn btn-sm btn-ghost" @click="$emit('insertTemplate')" :title="$t('plugins.insertTemplate', '插入模板')">
              <i class="fas fa-file-code"></i>
            </button>
            <button class="btn btn-sm btn-ghost" @click="$emit('formatCode')" :title="$t('plugins.format', '格式化')">
              <i class="fas fa-indent"></i>
              <span v-if="!isCompactToolbar" class="ml-1">{{ $t('plugins.format', '格式化') }}</span>
            </button>
            <button class="btn btn-sm btn-ghost" @click="$emit('copyPlugin')" :title="$t('plugins.copyPlugin', '复制插件')">
              <i class="fas fa-copy"></i>
            </button>

            <div class="toolbar-divider"></div>

            <!-- Test Button -->
            <button v-if="editingPlugin" 
                    class="btn btn-sm btn-info" 
                    :disabled="pluginTesting"
                    @click="$emit('testCurrentPlugin')" 
                    :title="$t('plugins.testPlugin', '测试插件')">
              <span v-if="pluginTesting" class="loading loading-spinner loading-xs"></span>
              <i v-else class="fas fa-play"></i>
              <span v-if="!isCompactToolbar" class="ml-1">{{ $t('plugins.test', '测试') }}</span>
            </button>
            
            <template v-if="editingPlugin">
              <button v-if="!isEditing" class="btn btn-sm btn-primary" @click="$emit('enableEditing')">
                <i class="fas fa-edit" :class="{ 'mr-1': !isCompactToolbar }"></i>
                <span v-if="!isCompactToolbar">{{ $t('common.edit', '编辑') }}</span>
              </button>
              <button v-if="isEditing" class="btn btn-sm btn-warning" @click="$emit('cancelEditing')">
                <i class="fas fa-eye" :class="{ 'mr-1': !isCompactToolbar }"></i>
                <span v-if="!isCompactToolbar">{{ $t('common.readonly', '只读') }}</span>
              </button>
            </template>

            <button v-if="isEditing || !editingPlugin" class="btn btn-sm btn-success" :disabled="saving" @click="$emit('savePlugin')">
              <span v-if="saving" class="loading loading-spinner loading-xs"></span>
              <template v-else>
                <i class="fas fa-save" :class="{ 'mr-1': !isCompactToolbar }"></i>
                <span v-if="!isCompactToolbar">{{ $t('common.save', '保存') }}</span>
              </template>
            </button>

            <div class="toolbar-divider"></div>

            <button class="btn btn-sm btn-ghost" @click="$emit('toggleFullscreen')">
              <i class="fas fa-compress" :class="{ 'mr-1': !isCompactToolbar }"></i>
              <span v-if="!isCompactToolbar">{{ $t('plugins.exitFullscreen', '退出全屏') }}</span>
            </button>
            <kbd v-if="!isCompactToolbar" class="kbd kbd-sm">ESC</kbd>
          </div>
        </div>
      </div>

      <div v-if="codeError" class="fullscreen-editor-error toast toast-bottom toast-center">
        <div class="alert alert-error shadow-lg">
          <i class="fas fa-exclamation-circle"></i><span>{{ codeError }}</span>
        </div>
      </div>
    </div>
  </Teleport>
</template>

<script setup lang="ts">
import { ref, computed, watch, nextTick } from 'vue'
import { marked } from 'marked'
import DOMPurify from 'dompurify'
import type { PluginRecord, NewPluginMetadata, SubCategory, CodeReference, TestResultReference, AiChatMessage } from './types'
import { mainCategories } from './types'

const props = defineProps<{
  editingPlugin: PluginRecord | null
  newPluginMetadata: NewPluginMetadata
  isEditing: boolean
  saving: boolean
  codeError: string
  isFullscreenEditor: boolean
  subCategories: SubCategory[]
  // AI related props
  showAiPanel: boolean
  aiMessages: AiChatMessage[]
  aiStreaming: boolean
  aiStreamingContent: string
  selectedCodeRef: CodeReference | null
  selectedTestResultRef: TestResultReference | null
  // Test related props
  pluginTesting: boolean
  // Preview related props
  isPreviewMode?: boolean
}>()

const emit = defineEmits<{
  'update:newPluginMetadata': [value: NewPluginMetadata]
  'insertTemplate': []
  'formatCode': []
  'copyPlugin': []
  'toggleFullscreen': []
  'enableEditing': []
  'cancelEditing': []
  'savePlugin': []
  'createNewPlugin': []
  'close': []
  // AI related emits
  'toggleAiPanel': []
  'sendAiMessage': [message: string]
  'aiQuickAction': [action: string]
  'applyAiCode': [code: string, context?: CodeReference | null]
  'previewAiCode': [code: string]
  'exitPreviewMode': []
  'addSelectedCode': []
  'addFullCode': []
  'clearCodeRef': []
  'clearTestResultRef': []
  'addTestResultToContext': []
  // Test related emits
  'testCurrentPlugin': []
}>()

const codeEditorDialogRef = ref<HTMLDialogElement>()
const codeEditorContainerRef = ref<HTMLDivElement>()
const fullscreenCodeEditorContainerRef = ref<HTMLDivElement>()
const fullscreenDiffEditorContainerRef = ref<HTMLDivElement>()
const fullscreenOverlayRef = ref<HTMLDivElement>()
const aiChatMessagesRef = ref<HTMLDivElement>()

// AI chat state
const aiInputText = ref('')
const includeCodeContext = ref(true)
const aiPanelWidth = ref(400)
const isResizing = ref(false)

// Toolbar state
const isCompactToolbar = ref(false)
const toolbarPosition = ref<{ x: number | null, y: number | null }>({ x: null, y: null })
const isDraggingToolbar = ref(false)
const dragOffset = ref({ x: 0, y: 0 })

// Configure marked for streaming content
marked.setOptions({
  breaks: true,
  gfm: true,
})

// Render streaming content as Markdown
const aiStreamingContentRendered = computed(() => {
  if (!props.aiStreamingContent) return ''
  const rawHtml = marked.parse(props.aiStreamingContent) as string
  return DOMPurify.sanitize(rawHtml, {
    ALLOWED_TAGS: ['p', 'br', 'strong', 'em', 'code', 'pre', 'ul', 'ol', 'li', 'h1', 'h2', 'h3', 'h4', 'h5', 'h6', 'blockquote', 'a'],
    ALLOWED_ATTR: ['href', 'class']
  })
})

const handleApplyCode = (code: string, messageIndex: number) => {
  // Find the preceding user message to get code context
  let codeRef: CodeReference | null = null
  for (let i = messageIndex - 1; i >= 0; i--) {
    if (props.aiMessages[i].role === 'user') {
      codeRef = props.aiMessages[i].codeRef || null
      break
    }
  }
  emit('applyAiCode', code, codeRef)
}

const handleApplyAllCode = (code: string, messageIndex: number) => {
  // Find the preceding user message to get code context
  let codeRef: CodeReference | null = null
  for (let i = messageIndex - 1; i >= 0; i--) {
    if (props.aiMessages[i].role === 'user') {
      codeRef = props.aiMessages[i].codeRef || null
      break
    }
  }
  emit('applyAiCode', code, codeRef)
}

const startResize = (e: MouseEvent) => {
  isResizing.value = true
  document.addEventListener('mousemove', handleResize)
  document.addEventListener('mouseup', stopResize)
  document.body.style.userSelect = 'none'
}

const handleResize = (e: MouseEvent) => {
  if (!isResizing.value) return
  // Calculate new width: window width - mouse X
  const newWidth = window.innerWidth - e.clientX
  // Constrain width
  if (newWidth >= 300 && newWidth <= 800) {
    aiPanelWidth.value = newWidth
  }
}

const stopResize = () => {
  isResizing.value = false
  document.removeEventListener('mousemove', handleResize)
  document.removeEventListener('mouseup', stopResize)
  document.body.style.userSelect = ''
}

// Toolbar drag functionality
const startToolbarDrag = (e: MouseEvent) => {
  e.preventDefault() // Prevent text selection
  e.stopPropagation()
  
  const toolbar = (e.currentTarget as HTMLElement).parentElement as HTMLElement
  const overlayRect = fullscreenOverlayRef.value?.getBoundingClientRect()
  if (!overlayRect) return
  const rect = toolbar.getBoundingClientRect()
  
  // If toolbar is in default position (centered), calculate actual position first
  if (toolbarPosition.value.x === null && toolbarPosition.value.y === null) {
    toolbarPosition.value = {
      x: rect.left - overlayRect.left,
      y: rect.top - overlayRect.top
    }
  }
  
  // Calculate drag offset from current toolbar position
  dragOffset.value = {
    x: e.clientX - rect.left,
    y: e.clientY - rect.top
  }
  
  isDraggingToolbar.value = true
  document.addEventListener('mousemove', handleToolbarDrag)
  document.addEventListener('mouseup', stopToolbarDrag)
  document.body.classList.add('toolbar-dragging')
  document.body.style.userSelect = 'none'
  document.body.style.cursor = 'grabbing'
}

const handleToolbarDrag = (e: MouseEvent) => {
  if (!isDraggingToolbar.value) return
  e.preventDefault() // Prevent text selection during drag
  const overlayRect = fullscreenOverlayRef.value?.getBoundingClientRect()
  if (!overlayRect) return
  toolbarPosition.value = {
    x: e.clientX - overlayRect.left - dragOffset.value.x,
    y: e.clientY - overlayRect.top - dragOffset.value.y
  }
}

const stopToolbarDrag = () => {
  isDraggingToolbar.value = false
  document.removeEventListener('mousemove', handleToolbarDrag)
  document.removeEventListener('mouseup', stopToolbarDrag)
  document.body.classList.remove('toolbar-dragging')
  document.body.style.userSelect = ''
  document.body.style.cursor = ''
}

const toggleCompactToolbar = () => {
  isCompactToolbar.value = !isCompactToolbar.value
}

const resetToolbarPosition = () => {
  toolbarPosition.value = { x: null, y: null }
}

const isNewPluginValid = computed(() => {
  return props.newPluginMetadata.id.trim() !== '' && props.newPluginMetadata.name.trim() !== ''
})

const updateMetadata = (key: keyof NewPluginMetadata, value: string) => {
  emit('update:newPluginMetadata', { ...props.newPluginMetadata, [key]: value })
}

const handleDialogCancel = (e: Event) => {
  if (props.isFullscreenEditor) {
    e.preventDefault()
    emit('toggleFullscreen')
  }
}

const showDialog = () => codeEditorDialogRef.value?.showModal()
const closeDialog = () => { codeEditorDialogRef.value?.close(); emit('close') }

// 临时隐藏 dialog 的 modal 状态，让全屏编辑器能接收事件
const hideModalTemporary = () => {
  codeEditorDialogRef.value?.close()
}

// 恢复 dialog 的 modal 状态
const restoreModal = () => {
  codeEditorDialogRef.value?.showModal()
}

// Auto scroll to bottom when new messages arrive
watch(() => props.aiMessages.length, () => {
  nextTick(() => {
    if (aiChatMessagesRef.value) {
      aiChatMessagesRef.value.scrollTop = aiChatMessagesRef.value.scrollHeight
    }
  })
})

// Clear input immediately when streaming starts
watch(() => props.aiStreaming, (streaming) => {
  if (streaming) {
    // Clear input immediately when message is sent
    aiInputText.value = ''
  }
})

defineExpose({
  showDialog, closeDialog,
  hideModalTemporary, restoreModal,
  codeEditorContainerRef, fullscreenCodeEditorContainerRef, fullscreenDiffEditorContainerRef,
  aiInputText, includeCodeContext
})
</script>

<style scoped>
.fullscreen-editor-overlay {
  position: fixed;
  top: 4rem; /* navbar height */
  left: 0;
  right: 0;
  bottom: 0;
  width: 100vw;
  height: calc(100vh - 4rem);
  z-index: 999999;
  background: oklch(var(--b1));
  display: block;
}

/* 全屏模式下禁用 dialog 的 pointer-events，让事件穿透到全屏编辑器覆盖层 */
:global(dialog.fullscreen-mode-active) {
  pointer-events: none !important;
}

:global(dialog.fullscreen-mode-active::backdrop) {
  pointer-events: none !important;
  opacity: 0;
}

/* 确保全屏编辑器覆盖层内所有元素能正确接收事件 */
.fullscreen-editor-overlay * {
  pointer-events: auto;
}

/* Floating Toolbar - centered in editor area, follows theme */
.fullscreen-floating-toolbar {
  position: absolute;
  top: 1rem;
  left: 50%;
  transform: translateX(-50%);
  padding: 0.75rem 1.25rem;
  padding-left: 0.5rem; /* Less padding on left for drag handle */
  background: oklch(var(--b2));
  border: 1px solid oklch(var(--bc) / 0.2);
  border-radius: 1rem;
  z-index: 100;
  box-shadow: 0 10px 25px -5px rgb(0 0 0 / 0.2), 0 8px 10px -6px rgb(0 0 0 / 0.15);
  transition: left 0.3s ease, transform 0.3s ease;
  color: oklch(var(--bc));
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

/* Dragging state */
.fullscreen-floating-toolbar.dragging {
  cursor: grabbing;
  transition: none;
  box-shadow: 0 20px 40px -10px rgb(0 0 0 / 0.4), 0 16px 20px -12px rgb(0 0 0 / 0.3);
  user-select: none;
  -webkit-user-select: none;
}

/* Disable text selection globally when dragging */
:global(body.toolbar-dragging) {
  user-select: none !important;
  -webkit-user-select: none !important;
  -moz-user-select: none !important;
  -ms-user-select: none !important;
  cursor: grabbing !important;
}

/* Compact mode */
.fullscreen-floating-toolbar.compact {
  padding: 0.5rem;
  padding-left: 0.5rem;
}

/* Drag Handle */
.toolbar-drag-handle {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 1.5rem;
  height: 2rem;
  cursor: grab;
  color: oklch(var(--bc) / 0.4);
  border-right: 1px solid oklch(var(--bc) / 0.1);
  margin-right: 0.5rem;
  transition: color 0.2s, background 0.2s;
  border-radius: 0.5rem 0 0 0.5rem;
  user-select: none;
  -webkit-user-select: none;
  -moz-user-select: none;
  -ms-user-select: none;
}

.toolbar-drag-handle:hover {
  color: oklch(var(--bc) / 0.7);
  background: oklch(var(--bc) / 0.05);
}

.toolbar-drag-handle:active {
  cursor: grabbing;
  color: oklch(var(--p));
}

/* Prevent text selection during drag */
.fullscreen-floating-toolbar.dragging,
.fullscreen-floating-toolbar.dragging * {
  user-select: none !important;
  -webkit-user-select: none !important;
  -moz-user-select: none !important;
  -ms-user-select: none !important;
}

/* When AI panel is open, center toolbar in editor area only */
.fullscreen-floating-toolbar.with-ai-panel {
  left: calc((100% - 400px) / 2);
}

/* Toolbar text follows theme */
.fullscreen-floating-toolbar span,
.fullscreen-floating-toolbar .font-bold,
.fullscreen-floating-toolbar .text-xs {
  color: oklch(var(--bc)) !important;
}

/* Toolbar divider */
.fullscreen-floating-toolbar .toolbar-divider {
  width: 1px;
  height: 1.5rem;
  background: oklch(var(--bc) / 0.2);
  margin: 0 0.25rem;
}

/* Buttons follow theme */
.fullscreen-floating-toolbar .btn {
  color: oklch(var(--bc));
  border-color: oklch(var(--bc) / 0.2);
  background: oklch(var(--b1));
}

.fullscreen-floating-toolbar .btn:hover {
  background: oklch(var(--b3));
}

.fullscreen-floating-toolbar .btn-ghost {
  color: oklch(var(--bc));
  background: transparent;
  border-color: transparent;
}

.fullscreen-floating-toolbar .btn-ghost:hover {
  background: oklch(var(--b3));
}

.fullscreen-floating-toolbar .btn-primary {
  background: oklch(var(--p));
  color: oklch(var(--pc));
  border-color: oklch(var(--p));
}

.fullscreen-floating-toolbar .btn-primary:hover {
  filter: brightness(0.9);
}

.fullscreen-floating-toolbar .btn-success {
  background: oklch(var(--su));
  color: oklch(var(--suc));
  border-color: oklch(var(--su));
}

.fullscreen-floating-toolbar .btn-success:hover {
  filter: brightness(0.9);
}

.fullscreen-floating-toolbar .btn-warning {
  background: oklch(var(--wa));
  color: oklch(var(--wac));
  border-color: oklch(var(--wa));
}

.fullscreen-floating-toolbar .btn-warning:hover {
  filter: brightness(0.9);
}

.fullscreen-floating-toolbar .kbd {
  background: oklch(var(--b3));
  color: oklch(var(--bc));
  border-color: oklch(var(--bc) / 0.2);
}

/* Layout for fullscreen with AI panel */
.fullscreen-editor-layout {
  display: flex;
  width: 100%;
  height: 100%;
  background: oklch(var(--b1));
  position: relative;
  isolation: isolate;
}

.fullscreen-editor-content {
  flex: 1;
  height: 100%;
  overflow: hidden;
  transition: width 0.3s ease;
  background: oklch(var(--b1));
}

.fullscreen-editor-content.with-ai-panel {
  width: calc(100% - 400px);
}

.fullscreen-editor-content :deep(.cm-editor) {
  height: 100%;
}

.fullscreen-editor-content :deep(.cm-scroller) {
  overflow: auto;
  padding-top: 1rem;
}

.fullscreen-editor-content :deep(.cm-content) {
  padding-top: 1rem;
  padding-bottom: 2rem;
}

/* AI Chat Panel Styles - uses oklch for DaisyUI 4.x compatibility */
.ai-chat-panel {
  width: 400px;
  height: 100%;
  display: flex;
  flex-direction: column;
  background: oklch(var(--b2));
  color: oklch(var(--bc));
  border-left: 1px solid oklch(var(--bc) / 0.15);
  position: relative;
  z-index: 10;
  box-shadow: -4px 0 20px oklch(var(--bc) / 0.1);
}

/* Resize Handle */
.resize-handle {
  position: absolute;
  left: 0;
  top: 0;
  bottom: 0;
  width: 6px;
  cursor: col-resize;
  background: transparent;
  transition: background 0.2s;
  z-index: 20;
}

.resize-handle:hover {
  background: oklch(var(--p) / 0.3);
}

.resize-handle::after {
  content: '';
  position: absolute;
  left: 2px;
  top: 50%;
  transform: translateY(-50%);
  width: 2px;
  height: 40px;
  background: oklch(var(--bc) / 0.2);
  border-radius: 1px;
}

.ai-chat-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 0.75rem 1rem;
  background: oklch(var(--b3));
  border-bottom: 1px solid oklch(var(--bc) / 0.15);
  color: oklch(var(--bc));
}

.ai-chat-header .text-primary {
  color: oklch(var(--p)) !important;
}

.ai-chat-header .btn-ghost {
  color: oklch(var(--bc) / 0.7);
}

.ai-chat-header .btn-ghost:hover {
  background: oklch(var(--b1));
  color: oklch(var(--bc));
}

.ai-chat-messages {
  flex: 1;
  overflow-y: auto;
  padding: 1rem;
  display: flex;
  flex-direction: column;
  gap: 1rem;
  background: oklch(var(--b2));
}

.ai-chat-empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  height: 100%;
  text-align: center;
  padding: 2rem;
  color: oklch(var(--bc) / 0.6);
}

.ai-chat-empty i {
  color: oklch(var(--bc) / 0.3);
}

.ai-quick-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 0.5rem;
  justify-content: center;
}

.ai-quick-actions .btn-outline {
  border-color: oklch(var(--bc) / 0.3);
  color: oklch(var(--bc) / 0.8);
}

.ai-quick-actions .btn-outline:hover {
  background: oklch(var(--b3));
  border-color: oklch(var(--p));
  color: oklch(var(--bc));
}

.ai-chat-message {
  display: flex;
  gap: 0.75rem;
  max-width: 100%;
}

.ai-chat-message.user {
  flex-direction: row-reverse;
}

.ai-chat-message .message-avatar {
  width: 2rem;
  height: 2rem;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  font-size: 0.875rem;
}

.ai-chat-message.user .message-avatar {
  background: oklch(var(--p));
  color: oklch(var(--pc));
}

.ai-chat-message.assistant .message-avatar {
  background: oklch(var(--su));
  color: oklch(var(--suc));
}

.ai-chat-message .message-content {
  flex: 1;
  min-width: 0;
}

/* Code reference in message */
.ai-chat-message .message-code-ref {
  background: oklch(var(--b1));
  border: 1px solid oklch(var(--bc) / 0.15);
  border-radius: 0.5rem;
  margin-bottom: 0.5rem;
  overflow: hidden;
}

.ai-chat-message .message-code-ref .code-ref-label {
  padding: 0.375rem 0.75rem;
  background: oklch(var(--b3));
  color: oklch(var(--bc) / 0.7);
  font-size: 0.75rem;
  border-bottom: 1px solid oklch(var(--bc) / 0.15);
}

.ai-chat-message .message-code-ref .code-ref-content {
  padding: 0.5rem 0.75rem;
  margin: 0;
  font-size: 0.75rem;
  color: oklch(var(--bc));
  max-height: 100px;
  overflow: auto;
  background: oklch(var(--b1));
}

.ai-chat-message .message-code-ref .code-ref-content code {
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
}

.ai-chat-message .message-text {
  padding: 0.75rem 1rem;
  border-radius: 1rem;
  font-size: 0.875rem;
  line-height: 1.5;
  word-break: break-word;
}

.ai-chat-message .message-text.streaming-text {
  font-family: inherit;
}

/* Markdown elements in message text */
.ai-chat-message .message-text :deep(p) {
  margin-bottom: 0.75rem;
}

.ai-chat-message .message-text :deep(p:last-child) {
  margin-bottom: 0;
}

.ai-chat-message .message-text :deep(h1),
.ai-chat-message .message-text :deep(h2),
.ai-chat-message .message-text :deep(h3) {
  font-weight: bold;
  margin-top: 1rem;
  margin-bottom: 0.5rem;
}

.ai-chat-message .message-text :deep(h1) { font-size: 1.25rem; }
.ai-chat-message .message-text :deep(h2) { font-size: 1.1rem; }
.ai-chat-message .message-text :deep(h3) { font-size: 1rem; }

.ai-chat-message .message-text :deep(blockquote) {
  border-left: 3px solid oklch(var(--p) / 0.3);
  padding-left: 1rem;
  margin: 0.75rem 0;
  color: oklch(var(--bc) / 0.8);
}

.ai-chat-message .message-text :deep(a) {
  color: oklch(var(--p));
  text-decoration: underline;
}

.ai-chat-message .message-text :deep(a:hover) {
  color: oklch(var(--p) / 0.8);
}

.ai-chat-message.user .message-text {
  background: oklch(var(--p));
  color: oklch(var(--pc));
  border-bottom-right-radius: 0.25rem;
}

.ai-chat-message.assistant .message-text {
  background: oklch(var(--b3));
  color: oklch(var(--bc));
  border-bottom-left-radius: 0.25rem;
}

.ai-chat-message .message-code-action {
  display: flex;
  gap: 0.5rem;
  margin-top: 0.5rem;
}

.ai-chat-message .message-code-action .btn-primary {
  background: oklch(var(--p));
  border-color: oklch(var(--p));
  color: oklch(var(--pc));
}

.ai-chat-message .message-code-action .btn-ghost {
  color: oklch(var(--bc) / 0.7);
}

.ai-chat-message .message-code-action .btn-ghost:hover {
  background: oklch(var(--b1));
  color: oklch(var(--bc));
}

/* Typing indicator */
.typing-indicator {
  display: inline-flex;
  gap: 0.25rem;
  margin-left: 0.5rem;
}

.typing-indicator span {
  width: 0.5rem;
  height: 0.5rem;
  background: oklch(var(--p));
  border-radius: 50%;
  animation: typing 1.4s infinite ease-in-out both;
  opacity: 0.4;
}

.typing-indicator span:nth-child(1) { animation-delay: -0.32s; }
.typing-indicator span:nth-child(2) { animation-delay: -0.16s; }
.typing-indicator span:nth-child(3) { animation-delay: 0s; }

@keyframes typing {
  0%, 80%, 100% { transform: scale(0.6); opacity: 0.4; }
  40% { transform: scale(1); opacity: 1; }
}

/* Chat Input */
.ai-chat-input {
  padding: 1rem;
  background: oklch(var(--b3));
  border-top: 1px solid oklch(var(--bc) / 0.15);
}

/* Code reference badge in input area */
.ai-chat-input .code-reference-badge {
  background: oklch(var(--b1));
  border: 1px solid oklch(var(--bc) / 0.15);
  border-radius: 0.5rem;
  margin-bottom: 0.75rem;
  overflow: hidden;
}

.ai-chat-input .code-reference-badge .code-ref-header {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.5rem 0.75rem;
  background: oklch(var(--b2));
  color: oklch(var(--bc));
}

.ai-chat-input .code-reference-badge .code-ref-preview {
  max-height: 80px;
  overflow: auto;
  background: oklch(var(--b1));
}

.ai-chat-input .code-reference-badge .code-ref-preview pre {
  margin: 0;
  padding: 0.5rem 0.75rem;
  font-size: 0.75rem;
  color: oklch(var(--p));
}

.ai-chat-input .code-reference-badge .code-ref-preview code {
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
}

/* Test result reference badge in input area */
.ai-chat-input .test-result-reference-badge {
  background: oklch(var(--b1));
  border: 1px solid oklch(var(--bc) / 0.15);
  border-radius: 0.5rem;
  margin-bottom: 0.75rem;
  overflow: hidden;
}

.ai-chat-input .test-result-reference-badge .test-ref-header {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.5rem 0.75rem;
  background: oklch(var(--b2));
  color: oklch(var(--bc));
}

.ai-chat-input .test-result-reference-badge .test-ref-preview {
  max-height: 100px;
  overflow: auto;
  background: oklch(var(--b1));
}

.ai-chat-input .test-result-reference-badge .test-ref-preview pre {
  margin: 0;
  padding: 0.5rem 0.75rem;
  font-size: 0.75rem;
  color: oklch(var(--bc));
}

.ai-chat-input .test-result-reference-badge .test-ref-preview code {
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
}

/* Test result reference in messages */
.ai-chat-message .message-test-ref {
  background: oklch(var(--b1));
  border: 1px solid oklch(var(--bc) / 0.15);
  border-radius: 0.5rem;
  margin-bottom: 0.5rem;
  overflow: hidden;
}

.ai-chat-message .message-test-ref .test-ref-label {
  padding: 0.375rem 0.75rem;
  background: oklch(var(--b3));
  color: oklch(var(--bc) / 0.7);
  font-size: 0.75rem;
  border-bottom: 1px solid oklch(var(--bc) / 0.15);
}

.ai-chat-message .message-test-ref .test-ref-content {
  padding: 0.5rem 0.75rem;
  margin: 0;
  font-size: 0.75rem;
  color: oklch(var(--bc));
  max-height: 100px;
  overflow: auto;
  background: oklch(var(--b1));
}

.ai-chat-message .message-test-ref .test-ref-content code {
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
}

.ai-chat-input .textarea {
  font-size: 0.875rem;
  min-height: 3rem;
  max-height: 8rem;
  background: oklch(var(--b1));
  border-color: oklch(var(--bc) / 0.2);
  color: oklch(var(--bc));
}

.ai-chat-input .textarea:focus {
  border-color: oklch(var(--p));
  outline: none;
}

.ai-chat-input .textarea::placeholder {
  color: oklch(var(--bc) / 0.4);
}

.ai-chat-input-actions {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-top: 0.5rem;
}

.ai-chat-input-actions .btn-ghost {
  color: oklch(var(--bc) / 0.7);
  border-color: transparent;
}

.ai-chat-input-actions .btn-ghost:hover {
  background: oklch(var(--b2));
  color: oklch(var(--bc));
}

.ai-chat-input-actions .btn-primary {
  background: oklch(var(--p));
  border-color: oklch(var(--p));
  color: oklch(var(--pc));
}

.ai-chat-input-actions .btn-primary:hover {
  filter: brightness(0.9);
}

.ai-chat-input-actions .btn-primary:disabled {
  background: oklch(var(--bc) / 0.2);
  border-color: oklch(var(--bc) / 0.2);
  color: oklch(var(--bc) / 0.4);
}

/* Code block in messages */
.ai-chat-message .message-text :deep(pre) {
  background: oklch(var(--b1));
  padding: 0.75rem;
  border-radius: 0.5rem;
  overflow-x: auto;
  margin: 0.5rem 0;
  font-size: 0.75rem;
  border: 1px solid oklch(var(--bc) / 0.1);
}

.ai-chat-message .message-text :deep(code) {
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
  font-size: 0.75rem;
}

.ai-chat-message .message-text :deep(code.inline-code) {
  background: oklch(var(--b3));
  padding: 0.125rem 0.375rem;
  border-radius: 0.25rem;
  font-size: 0.8em;
}

.ai-chat-message .message-text :deep(strong) {
  font-weight: 600;
}

.ai-chat-message .message-text :deep(ul) {
  margin: 0.5rem 0;
  padding-left: 1.5rem;
}

.ai-chat-message .message-text :deep(li) {
  margin: 0.25rem 0;
}

/* AI Suggestions in Message */
.message-ai-suggestions {
  margin-top: 0.75rem;
  padding: 0.75rem;
  background: oklch(var(--b1));
  border: 1px solid oklch(var(--p) / 0.2);
  border-radius: 0.75rem;
  box-shadow: 0 4px 12px oklch(0 0 0 / 0.05);
}

.suggestions-header {
  display: flex;
  align-items: center;
  margin-bottom: 0.5rem;
  padding-bottom: 0.5rem;
  border-bottom: 1px solid oklch(var(--bc) / 0.05);
}

.suggestions-list {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.suggestion-item {
  background: oklch(var(--b2));
  border-radius: 0.5rem;
  overflow: hidden;
  border: 1px solid oklch(var(--bc) / 0.05);
}

.suggestion-info {
  display: flex;
  align-items: center;
  padding: 0.25rem 0.5rem;
  background: oklch(var(--b3));
  border-bottom: 1px solid oklch(var(--bc) / 0.05);
}

.suggestion-preview {
  margin: 0;
  padding: 0.5rem;
  font-size: 10px;
  max-height: 60px;
  overflow: hidden;
  color: oklch(var(--bc) / 0.6);
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
}

.suggestion-preview code {
  white-space: pre-wrap;
}

.btn-mini {
  font-size: 10px;
}

/* Diff View Header */
.diff-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 0.75rem 1rem;
  background: oklch(var(--b3));
  border-bottom: 1px solid oklch(var(--bc) / 0.15);
  color: oklch(var(--bc));
}

.diff-header-content {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.diff-header-actions {
  display: flex;
  gap: 0.5rem;
}

/* Diff Editor Container */
.fullscreen-editor-content :deep(.cm-merge-view) {
  height: 100%;
  display: flex;
}

.fullscreen-editor-content :deep(.cm-merge-a),
.fullscreen-editor-content :deep(.cm-merge-b) {
  flex: 1;
  height: 100%;
}

.fullscreen-editor-content :deep(.cm-merge-spacer) {
  width: 2px;
  background: oklch(var(--bc) / 0.2);
}

/* Responsive */
@media (max-width: 1024px) {
  .ai-chat-panel {
    width: 320px;
  }
  
  .fullscreen-editor-content.with-ai-panel {
    width: calc(100% - 320px);
  }
  
  .fullscreen-floating-toolbar.with-ai-panel {
    left: calc((100% - 320px) / 2);
  }
}
</style>
