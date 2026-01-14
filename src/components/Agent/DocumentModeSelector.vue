<template>
  <div class="document-mode-selector">
    <!-- Compact layout: file info + mode buttons in one row -->
    <div class="file-row">
      <!-- File icon -->
      <div class="file-icon">
        <i :class="['fas', documentIcon]"></i>
      </div>
      
      <!-- File info -->
      <div class="file-info">
        <span class="file-name" :title="attachment.original_filename">
          {{ truncateFilename(attachment.original_filename, 24) }}
        </span>
        <span class="file-size">{{ formatFileSize(attachment.file_size) }}</span>
      </div>

      <!-- Mode buttons (compact) or selected mode badge -->
      <div v-if="!attachment.processing_mode" class="mode-buttons">
        <button
          class="mode-btn-compact content"
          :class="{ disabled: isProcessing }"
          @click="selectMode('content')"
          :disabled="isProcessing"
          :title="t('agent.document.contentModeDesc')"
        >
          <i class="fas fa-book-open"></i>
          <span>{{ t('agent.document.contentMode') }}</span>
        </button>
        <button
          class="mode-btn-compact security"
          :class="{ disabled: !dockerAvailable || isProcessing }"
          @click="selectMode('security')"
          :disabled="!dockerAvailable || isProcessing"
          :title="dockerAvailable ? t('agent.document.securityModeDesc') : t('agent.document.requiresDocker')"
        >
          <i class="fas fa-shield-alt"></i>
          <span>{{ t('agent.document.securityMode') }}</span>
        </button>
      </div>

      <!-- Selected mode badge -->
      <div v-else class="mode-badge" :class="attachment.processing_mode">
        <i :class="['fas', attachment.processing_mode === 'content' ? 'fa-book-open' : 'fa-shield-alt']"></i>
        <span>{{ attachment.processing_mode === 'content' ? t('agent.document.contentMode') : t('agent.document.securityMode') }}</span>
        <i v-if="statusClass === 'ready'" class="fas fa-check-circle text-success ml-1"></i>
        <span v-else-if="isProcessing" class="loading loading-spinner loading-xs ml-1"></span>
      </div>

      <!-- Remove button -->
      <button class="btn btn-ghost btn-xs btn-circle ml-1" @click="$emit('remove')" :title="t('agent.document.remove')">
        <i class="fas fa-times"></i>
      </button>
    </div>

    <!-- Error message -->
    <div v-if="processedResult?.error_message" class="error-message">
      <i class="fas fa-exclamation-circle"></i>
      <span>{{ processedResult.error_message }}</span>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { invoke } from '@tauri-apps/api/core'
import type { PendingDocumentAttachment, ProcessedDocumentResult, DocumentProcessingMode } from '@/types/agent'
import { getDocumentIcon } from '@/types/agent'

const { t } = useI18n()

const props = defineProps<{
  attachment: PendingDocumentAttachment
  dockerAvailable: boolean
  processedResult?: ProcessedDocumentResult
}>()

const emit = defineEmits<{
  (e: 'remove'): void
  (e: 'processed', result: ProcessedDocumentResult): void
  (e: 'error', error: string): void
}>()

const isProcessing = ref(false)

const documentIcon = computed(() => {
  const ext = props.attachment.original_filename.split('.').pop() || ''
  return getDocumentIcon(ext)
})

const statusClass = computed(() => {
  if (!props.processedResult) return 'pending'
  return props.processedResult.status
})

const statusIcon = computed(() => {
  const icons: Record<string, string> = {
    pending: 'fa-clock',
    processing: 'fa-spinner fa-spin',
    ready: 'fa-check-circle',
    failed: 'fa-times-circle',
  }
  return icons[statusClass.value] || 'fa-clock'
})

const statusText = computed(() => {
  if (!props.processedResult) return t('agent.document.statusPending')
  const statusMap: Record<string, string> = {
    pending: t('agent.document.statusPending'),
    processing: t('agent.document.statusProcessing'),
    ready: t('agent.document.statusReady'),
    failed: t('agent.document.statusFailed'),
  }
  return statusMap[props.processedResult.status] || ''
})

function truncateFilename(name: string, maxLen: number): string {
  if (name.length <= maxLen) return name
  const ext = name.split('.').pop() || ''
  const baseName = name.slice(0, name.length - ext.length - 1)
  const truncated = baseName.slice(0, maxLen - ext.length - 4) + '...'
  return truncated + '.' + ext
}

function formatFileSize(bytes: number): string {
  if (bytes < 1024) return bytes + ' B'
  if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(1) + ' KB'
  return (bytes / (1024 * 1024)).toFixed(1) + ' MB'
}

async function selectMode(mode: DocumentProcessingMode) {
  if (isProcessing.value) return
  if (mode === 'security' && !props.dockerAvailable) return

  isProcessing.value = true

  try {
    const result = await invoke<ProcessedDocumentResult>('process_document_attachment', {
      filePath: props.attachment.original_path,
      mode: mode,
    })

    // Use the frontend attachment ID to ensure proper matching
    const resultWithCorrectId: ProcessedDocumentResult = {
      ...result,
      id: props.attachment.id,
    }

    emit('processed', resultWithCorrectId)
  } catch (error) {
    console.error('Document processing failed:', error)
    emit('error', String(error))
  } finally {
    isProcessing.value = false
  }
}
</script>

<style scoped>
.document-mode-selector {
  @apply bg-base-200 rounded-lg px-2 py-1.5 mb-1;
  border: 1px solid hsl(var(--bc) / 0.1);
}

.file-row {
  @apply flex items-center gap-2;
}

.file-icon {
  @apply w-6 h-6 flex items-center justify-center rounded bg-base-300 text-sm flex-shrink-0;
  color: hsl(var(--p));
}

.file-info {
  @apply flex-1 min-w-0;
}

.file-name {
  @apply block text-xs font-medium truncate;
}

.file-size {
  @apply text-[10px] opacity-60;
}

.mode-buttons {
  @apply flex gap-1 flex-shrink-0;
}

.mode-btn-compact {
  @apply px-2 py-1 rounded text-xs font-medium flex items-center gap-1 transition-all cursor-pointer border;
  background: hsl(var(--b1));
  border-color: transparent;
}

.mode-btn-compact:hover:not(.disabled) {
  border-color: hsl(var(--bc) / 0.3);
}

.mode-btn-compact.disabled {
  @apply opacity-40 cursor-not-allowed;
}

.mode-btn-compact.content {
  color: hsl(var(--su));
}

.mode-btn-compact.content:hover:not(.disabled) {
  @apply bg-success/10;
}

.mode-btn-compact.security {
  color: hsl(var(--wa));
}

.mode-btn-compact.security:hover:not(.disabled) {
  @apply bg-warning/10;
}

.mode-badge {
  @apply px-2 py-1 rounded text-xs font-medium flex items-center gap-1 flex-shrink-0;
}

.mode-badge.content {
  @apply bg-success/20 text-success;
}

.mode-badge.security {
  @apply bg-warning/20 text-warning;
}

.error-message {
  @apply flex items-center gap-2 mt-1 text-xs text-error;
}
</style>
