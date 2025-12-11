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
    <div v-if="isFullscreenEditor" class="fullscreen-editor-overlay">
      <div class="fullscreen-editor-content">
        <div ref="fullscreenCodeEditorContainerRef" class="h-full w-full"></div>
      </div>

      <!-- Floating Toolbar -->
      <div class="fullscreen-floating-toolbar shadow-lg border border-base-content/10">
        <div class="flex items-center gap-3">
          <!-- Info Section -->
          <div class="flex flex-col">
            <span class="font-bold text-sm">
              {{ editingPlugin ? editingPlugin.metadata.name : $t('plugins.newPlugin', '新增插件') }}
            </span>
            <span v-if="editingPlugin" class="text-xs opacity-80 font-mono">{{ editingPlugin.metadata.id }}</span>
          </div>

          <div class="divider divider-horizontal mx-0 my-1 h-8"></div>

          <!-- Actions -->
          <div class="flex items-center gap-2">
            <button v-if="!editingPlugin" class="btn btn-sm btn-ghost" @click="$emit('insertTemplate')" :title="$t('plugins.insertTemplate', '插入模板')">
              <i class="fas fa-file-code"></i>
            </button>
            <button class="btn btn-sm btn-ghost" @click="$emit('formatCode')" :title="$t('plugins.format', '格式化')">
              <i class="fas fa-indent"></i> {{ $t('plugins.format', '格式化') }}
            </button>
            
            <template v-if="editingPlugin">
              <button v-if="!isEditing" class="btn btn-sm btn-primary" @click="$emit('enableEditing')">
                <i class="fas fa-edit mr-1"></i>{{ $t('common.edit', '编辑') }}
              </button>
              <button v-if="isEditing" class="btn btn-sm btn-warning" @click="$emit('cancelEditing')">
                <i class="fas fa-eye mr-1"></i>{{ $t('common.readonly', '只读') }}
              </button>
            </template>

            <button v-if="isEditing || !editingPlugin" class="btn btn-sm btn-success" :disabled="saving" @click="$emit('savePlugin')">
              <span v-if="saving" class="loading loading-spinner loading-xs"></span>
              <template v-else>
                <i class="fas fa-save mr-1"></i>{{ $t('common.save', '保存') }}
              </template>
            </button>

            <div class="divider divider-horizontal mx-0 my-1 h-8"></div>

            <button class="btn btn-sm btn-ghost" @click="$emit('toggleFullscreen')">
              <i class="fas fa-compress mr-1"></i>{{ $t('plugins.exitFullscreen', '退出全屏') }}
            </button>
            <kbd class="kbd kbd-sm opacity-70">ESC</kbd>
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
import { ref, computed } from 'vue'
import type { PluginRecord, NewPluginMetadata, SubCategory } from './types'
import { mainCategories } from './types'

const props = defineProps<{
  editingPlugin: PluginRecord | null
  newPluginMetadata: NewPluginMetadata
  isEditing: boolean
  saving: boolean
  codeError: string
  isFullscreenEditor: boolean
  subCategories: SubCategory[]
}>()

const emit = defineEmits<{
  'update:newPluginMetadata': [value: NewPluginMetadata]
  'insertTemplate': []
  'formatCode': []
  'toggleFullscreen': []
  'enableEditing': []
  'cancelEditing': []
  'savePlugin': []
  'createNewPlugin': []
  'close': []
}>()

const codeEditorDialogRef = ref<HTMLDialogElement>()
const codeEditorContainerRef = ref<HTMLDivElement>()
const fullscreenCodeEditorContainerRef = ref<HTMLDivElement>()

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

defineExpose({
  showDialog, closeDialog,
  hideModalTemporary, restoreModal,
  codeEditorContainerRef, fullscreenCodeEditorContainerRef
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
  background-color: hsl(var(--b1));
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

.fullscreen-floating-toolbar:hover {
  opacity: 1;
}

.fullscreen-floating-toolbar {
  position: absolute;
  top: 1rem;
  right: 2rem;
  padding: 0.5rem 1rem;
  /* Use base-300 for better contrast against the base-100/editor background */
  background-color: hsl(var(--b3));
  border: 1px solid hsl(var(--bc) / 0.2);
  backdrop-filter: blur(8px);
  border-radius: 1rem;
  z-index: 100;
  /* Stronger shadow for "floating" effect */
  box-shadow: 0 10px 25px -5px rgb(0 0 0 / 0.3), 0 8px 10px -6px rgb(0 0 0 / 0.3);
  transition: opacity 0.2s ease, transform 0.2s ease;
  /* Enforce high contrast text color */
  color: hsl(var(--bc));
}

.fullscreen-floating-toolbar .btn-ghost {
  color: hsl(var(--bc));
}

.fullscreen-editor-content {
  width: 100%;
  height: 100%;
  overflow: hidden;
}

.fullscreen-editor-content :deep(.cm-editor) {
  height: 100%;
}

.fullscreen-editor-content :deep(.cm-scroller) {
  overflow: auto;
  padding-top: 1rem; /* Spacing for aesthetics */
  /* Ensure content doesn't get hidden behind floating toolbar initially by adding some top padding 
     or relying on user to scroll. Since the toolbar is floating on the right, it might not obscure critical code */
}

/* Add padding to the top of the editor content so the first few lines aren't hidden by the floating toolbar if code is long?
   Actually, the toolbar is on the right. Usually code starts on the left. So standard padding is fine. 
   But let's add a bit of padding to the editor container generally for aesthetic */
.fullscreen-editor-content :deep(.cm-content) {
  padding-top: 1rem;
  padding-bottom: 2rem;
}
</style>
