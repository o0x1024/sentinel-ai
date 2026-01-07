<template>
  <!-- Code Editor Dialog -->
  <dialog ref="codeEditorDialogRef" class="modal" :class="{ 'fullscreen-mode-active': isFullscreenEditor }" @cancel="handleDialogCancel">
    <div class="modal-box w-11/12 max-w-5xl max-h-[90vh] overflow-y-auto" :class="{ 'invisible': isFullscreenEditor }">
      <div class="flex justify-between items-start mb-4 sticky top-0 bg-base-100 z-10 pb-2">
        <div class="flex items-center gap-2">
          <h3 class="font-bold text-lg">
            {{ editingPlugin ? $t('plugins.codeEditor', '插件代码编辑器') : $t('plugins.newPlugin', '新增插件') }}
            <span v-if="editingPlugin" class="text-sm font-normal text-gray-500 ml-2">
              {{ editingPlugin.metadata.name }} ({{ editingPlugin.metadata.id }})
            </span>
          </h3>
          <div class="relative">
            <button 
              class="btn btn-xs btn-ghost btn-circle" 
              :title="$t('plugins.shortcuts', '快捷键')"
              @click="toggleShortcutsMenu"
            >
              <i class="fas fa-keyboard"></i>
            </button>
            <transition name="fade-scale">
              <div 
                v-if="showShortcutsMenu"
                v-click-outside="closeShortcutsMenu"
                class="absolute top-full right-0 mt-2 p-3 shadow-xl bg-base-200 rounded-lg w-80 text-xs z-[1000] border border-base-300"
              >
                <div class="flex items-center gap-2 mb-3 pb-2 border-b border-base-300">
                  <i class="fas fa-keyboard text-primary"></i>
                  <span class="font-semibold text-sm">{{ $t('plugins.keyboardShortcuts', '键盘快捷键') }}</span>
                </div>
                <div class="space-y-1">
                  <div class="flex justify-between items-center px-2 py-2 hover:bg-base-300/50 rounded transition-colors">
                    <span class="text-xs">{{ $t('plugins.toggleAiPanel', '切换AI面板') }}</span>
                    <kbd class="kbd kbd-xs">Ctrl/Cmd + K</kbd>
                  </div>
                  <div class="flex justify-between items-center px-2 py-2 hover:bg-base-300/50 rounded transition-colors">
                    <span class="text-xs">{{ $t('plugins.savePlugin', '保存插件') }}</span>
                    <kbd class="kbd kbd-xs">Ctrl/Cmd + S</kbd>
                  </div>
                  <div class="flex justify-between items-center px-2 py-2 hover:bg-base-300/50 rounded transition-colors">
                    <span class="text-xs">{{ $t('plugins.formatCode', '格式化代码') }}</span>
                    <kbd class="kbd kbd-xs">Ctrl/Cmd + Shift + F</kbd>
                  </div>
                  <div class="flex justify-between items-center px-2 py-2 hover:bg-base-300/50 rounded transition-colors">
                    <span class="text-xs">{{ $t('plugins.copyCode', '复制代码') }}</span>
                    <kbd class="kbd kbd-xs">Ctrl/Cmd + Shift + C</kbd>
                  </div>
                  <div class="flex justify-between items-center px-2 py-2 hover:bg-base-300/50 rounded transition-colors">
                    <span class="text-xs">{{ $t('plugins.toggleFullscreen', '切换全屏') }}</span>
                    <kbd class="kbd kbd-xs">F11</kbd>
                  </div>
                  <div class="flex justify-between items-center px-2 py-2 hover:bg-base-300/50 rounded transition-colors">
                    <span class="text-xs">{{ $t('plugins.enableEdit', '启用编辑') }}</span>
                    <kbd class="kbd kbd-xs">Ctrl/Cmd + E</kbd>
                  </div>
                  <div class="flex justify-between items-center px-2 py-2 hover:bg-base-300/50 rounded transition-colors">
                    <span class="text-xs">{{ $t('plugins.exitFullscreen', '退出全屏') }}</span>
                    <kbd class="kbd kbd-xs">ESC</kbd>
                  </div>
                </div>
              </div>
            </transition>
          </div>
        </div>
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
    <div v-if="isFullscreenEditor && !isMinimized" ref="fullscreenOverlayRef" class="fullscreen-editor-overlay">
      <!-- Main content with AI panel -->
      <div class="fullscreen-editor-layout">
        <!-- Code Editor Area -->
        <div class="fullscreen-editor-content" :class="{ 'with-ai-panel': showAiPanel }">
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
        <AiAssistantPanel
          :show="showAiPanel"
          :messages="aiMessages"
          :streaming="aiStreaming"
          :streaming-content="aiStreamingContent"
          :code-ref="selectedCodeRef"
          :test-result-ref="selectedTestResultRef"
          @close="$emit('toggleAiPanel')"
          @send-message="$emit('sendAiMessage', $event)"
          @quick-action="$emit('aiQuickAction', $event)"
          @apply-code="(...args) => $emit('applyAiCode', ...args)"
          @preview-code="$emit('previewAiCode', $event)"
            @clear-code-ref="$emit('clearCodeRef')"
            @clear-test-result-ref="$emit('clearTestResultRef')"
            @clear-history="$emit('clearHistory')"
          />
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
          left: toolbarPosition.x !== null ? `${toolbarPosition.x}px` : (showAiPanel ? 'calc((100% - 400px) / 2)' : '50%'),
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

            <!-- Minimize Button -->
            <button class="btn btn-sm btn-ghost" @click="$emit('minimize')" :title="$t('common.minimize', '最小化')">
              <i class="fas fa-window-minimize"></i>
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
import { ref, computed, watch, nextTick, onMounted, onBeforeUnmount } from 'vue'
import type { PluginRecord, NewPluginMetadata, SubCategory, CodeReference, TestResultReference, AiChatMessage } from './types'
import { mainCategories } from './types'
import AiAssistantPanel from './AiAssistantPanel.vue'

// Type extension for click outside handler
declare module '@vue/runtime-core' {
  interface HTMLElement {
    _clickOutsideHandler?: (event: MouseEvent) => void
  }
}

// Click outside directive implementation
const vClickOutside = {
  mounted(el: HTMLElement, binding: any) {
    el._clickOutsideHandler = (event: MouseEvent) => {
      if (!el.contains(event.target as Node)) {
        binding.value()
      }
    }
    setTimeout(() => {
      document.addEventListener('click', el._clickOutsideHandler)
    }, 0)
  },
  beforeUnmount(el: HTMLElement) {
    if (el._clickOutsideHandler) {
      document.removeEventListener('click', el._clickOutsideHandler)
    }
  }
}

const props = defineProps<{
  editingPlugin: PluginRecord | null
  newPluginMetadata: NewPluginMetadata
  isEditing: boolean
  saving: boolean
  codeError: string
  isFullscreenEditor: boolean
  isMinimized?: boolean
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
  'minimize': []
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
  'clearHistory': []
  // Test related emits
  'testCurrentPlugin': []
}>()

const codeEditorDialogRef = ref<HTMLDialogElement>()
const codeEditorContainerRef = ref<HTMLDivElement>()
const fullscreenCodeEditorContainerRef = ref<HTMLDivElement>()
const fullscreenDiffEditorContainerRef = ref<HTMLDivElement>()
const fullscreenOverlayRef = ref<HTMLDivElement>()

// Toolbar state
const isCompactToolbar = ref(false)
const toolbarPosition = ref<{ x: number | null, y: number | null }>({ x: null, y: null })
const isDraggingToolbar = ref(false)
const dragOffset = ref({ x: 0, y: 0 })

// Shortcuts menu state
const showShortcutsMenu = ref(false)

const toggleShortcutsMenu = () => {
  showShortcutsMenu.value = !showShortcutsMenu.value
}

const closeShortcutsMenu = () => {
  showShortcutsMenu.value = false
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
  e.preventDefault()
  if (props.isFullscreenEditor) {
    emit('toggleFullscreen')
  } else {
    closeDialog()
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

defineExpose({
  showDialog, closeDialog,
  hideModalTemporary, restoreModal,
  codeEditorContainerRef, fullscreenCodeEditorContainerRef, fullscreenDiffEditorContainerRef
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
/* Shortcuts menu animation */
.fade-scale-enter-active,
.fade-scale-leave-active {
  transition: opacity 0.2s ease, transform 0.2s ease;
}

.fade-scale-enter-from {
  opacity: 0;
  transform: scale(0.95) translateY(-10px);
}

.fade-scale-leave-to {
  opacity: 0;
  transform: scale(0.95) translateY(-10px);
}

.fade-scale-enter-to,
.fade-scale-leave-from {
  opacity: 1;
  transform: scale(1) translateY(0);
}
</style>
