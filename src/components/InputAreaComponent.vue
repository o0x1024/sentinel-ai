<template>
  <div 
    class="input-area-container border-t border-base-300/50 bg-base-100 flex-shrink-0 relative z-0"
    @dragover.prevent="onDragOver"
    @dragleave.prevent="onDragLeave"
    @drop.prevent="onDrop"
    :class="{ 'drag-over': isDragOver }"
  >
    <!-- Drag overlay -->
    <div v-if="isDragOver" class="drag-overlay">
      <div class="drag-content">
        <i class="fas fa-file-upload text-4xl mb-2"></i>
        <span class="text-lg">{{ t('agent.document.dropDocuments') }}</span>
        <span class="text-sm opacity-70">{{ t('agent.document.supportedTypes') }}</span>
      </div>
    </div>

    <!-- Input area (refactored) -->
    <div class="px-4 pb-3 pt-2">
      <!-- 流量引用显示区 -->
      <div v-if="props.referencedTraffic && props.referencedTraffic.length > 0" class="mb-2">
        <div class="flex items-center justify-between mb-1">
          <span class="text-xs text-base-content/60 flex items-center gap-1">
            <i class="fas fa-network-wired text-accent"></i>
            引用的流量 ({{ props.referencedTraffic.length }})
          </span>
          <button 
            @click="emit('clear-traffic')"
            class="btn btn-xs btn-ghost text-base-content/60 hover:text-error"
            title="清除所有引用"
          >
            <i class="fas fa-times"></i>
            清除
          </button>
        </div>
        <div class="flex flex-wrap gap-2 max-h-32 overflow-y-auto">
          <div
            v-for="(traffic, idx) in props.referencedTraffic"
            :key="traffic.id"
            class="group relative flex items-center gap-2 px-2 py-1 bg-accent/10 border border-accent/30 rounded-lg text-xs"
          >
            <!-- 类型标签 -->
            <span :class="['badge badge-xs', getTypeBadgeClass(traffic.sendType)]">
              {{ getTypeLabel(traffic.sendType) }}
            </span>
            <span :class="['badge badge-xs', getMethodBadgeClass(traffic.method)]">
              {{ traffic.method }}
            </span>
            <span class="text-base-content/80 truncate max-w-40" :title="traffic.url">
              {{ traffic.host }}{{ getUrlPath(traffic.url) }}
            </span>
            <span v-if="traffic.sendType !== 'request'" :class="['badge badge-xs', getStatusBadgeClass(traffic.status_code)]">
              {{ traffic.status_code || 'N/A' }}
            </span>
            <button
              @click="emit('remove-traffic', idx)"
              class="w-4 h-4 rounded-full bg-error/80 text-error-content opacity-0 group-hover:opacity-100 transition-opacity flex items-center justify-center text-xs ml-1"
              title="移除"
            >
              <i class="fas fa-times text-[10px]"></i>
            </button>
          </div>
        </div>
      </div>

      <!-- 图片附件预览区 -->
      <div v-if="pendingAttachments && pendingAttachments.length > 0" class="mb-2 flex flex-wrap gap-2">
        <div
          v-for="(att, idx) in pendingAttachments"
          :key="idx"
          class="relative group"
        >
          <img
            :src="getAttachmentPreview(att)"
            class="h-16 w-16 object-cover rounded border border-base-300 bg-base-200"
            :alt="att.image?.filename || 'attachment'"
          />
          <button
            @click="removeAttachment(idx)"
            class="absolute -top-1 -right-1 w-5 h-5 rounded-full bg-error text-error-content opacity-0 group-hover:opacity-100 transition-opacity flex items-center justify-center text-xs"
            title="移除"
          >
            <i class="fas fa-times"></i>
          </button>
        </div>
      </div>

      <!-- 文档附件预览区 -->
      <div v-if="pendingDocuments && pendingDocuments.length > 0" class="mb-2 space-y-2">
        <DocumentModeSelector
          v-for="(doc, idx) in pendingDocuments"
          :key="doc.id"
          :attachment="doc"
          :docker-available="dockerAvailable"
          :processed-result="getProcessedResult(doc.id)"
          @remove="removeDocument(idx)"
          @processed="onDocumentProcessed"
          @error="onDocumentError"
        />
      </div>

      <!-- Docker 不可用警告 -->
      <div v-if="!dockerAvailable && pendingDocuments && pendingDocuments.length > 0" class="mb-2 alert alert-warning text-xs py-2">
        <i class="fas fa-exclamation-triangle"></i>
        <span>{{ t('agent.document.dockerNotAvailable') }}</span>
      </div>

      <div ref="containerRef" class="chat-input rounded-2xl bg-base-200/60 border border-base-300/60 backdrop-blur-sm flex flex-col gap-2 px-3 py-2 shadow-sm focus-within:border-primary transition-colors">
        <!-- Text input (auto-resize textarea) -->
        <div class="flex-1 min-w-0">
          <textarea
            ref="textareaRef"
            :value="inputMessage"
            @input="onInput"
            @keydown="onKeydown"
            @compositionstart="onCompositionStart"
            @compositionend="onCompositionEnd"
            :disabled="isLoading && !allowTakeover"
            :placeholder="placeholderText"
            class="w-full bg-transparent outline-none resize-none leading-relaxed text-sm placeholder:text-base-content/50 max-h-40"
            rows="1"
          />
        </div>

        <!-- Toolbar: left actions and right send/stop -->
        <div class="flex items-center justify-between gap-2">
          <!-- Leading action icons -->
          <div class="flex items-center gap-2 text-base-content/60 shrink-0">
            <button class="icon-btn" title="附件" @click="triggerFileSelect"><i class="fas fa-paperclip"></i></button>
            <button class="icon-btn" :class="{ active: localToolsEnabled }" title="工具调用" @click="toggleTools"><i class="fas fa-tools"></i></button>
            <button v-if="localToolsEnabled" class="icon-btn" title="工具配置" @click="emit('open-tool-config')"><i class="fas fa-cog"></i></button>
            <button 
              class="icon-btn" 
              :class="{ active: localRagEnabled }" 
              title="知识检索增强 - AI将使用 [SOURCE n] 格式引用知识库内容" 
              @click="toggleRAG"
            >
              <i class="fas fa-brain"></i>
            </button>
            <button class="icon-btn " title="@ 引用"><i class="fas fa-at"></i></button>
            <button class="icon-btn" title="快速指令"><i class="fas fa-bolt"></i></button>
            <button class="icon-btn" title="选择"><i class="fas fa-border-all"></i></button>
            <button class="icon-btn" title="清空会话" @click="clearConversation"><i class="fas fa-eraser"></i></button>
          </div>

          <!-- Right side icons -->
          <div class="flex items-center gap-2 shrink-0">
            <!-- Context usage indicator -->
            <div 
              v-if="effectiveContextUsage" 
              class="context-usage-indicator flex items-center gap-1 px-2 py-1 rounded-md text-xs cursor-default"
              :class="contextUsageClass"
              :title="contextUsageTooltip"
            >
              <span class="font-medium">{{ contextUsagePercentage }}%</span>
              <span class="opacity-70">·</span>
              <span class="opacity-80">{{ formatTokenCount(effectiveContextUsage.usedTokens) }} / {{ formatTokenCount(effectiveContextUsage.maxTokens) }}</span>
              <span class="opacity-70 hidden sm:inline">{{ t('agent.contextUsed') }}</span>
            </div>
            <!-- 未处理文档提示 -->
            <span v-if="hasUnprocessedDocuments" class="text-xs text-warning flex items-center gap-1">
              <i class="fas fa-exclamation-triangle"></i>
              {{ t('agent.document.selectModeFirst') }}
            </span>
            <button class="icon-btn" title="语言 / 翻译"><i class="fas fa-language"></i></button>
            <button
              v-if="!isLoading || allowTakeover"
              class="send-btn"
              :disabled="!canSend"
              :class="{ 'opacity-40 cursor-not-allowed': !canSend }"
              @click="emitSend"
              :title="hasUnprocessedDocuments ? t('agent.document.selectModeFirst') : (isLoading ? '接管并发送 (Enter)' : '发送 (Enter)')"
            >
              <i class="fas fa-arrow-up"></i>
            </button>
            <button
              v-if="isLoading"
              class="send-btn bg-error text-error-content hover:bg-error/90"
              @click="handleStop"
              title="停止执行"
            >
              <i class="fas fa-stop"></i>
            </button>
          </div>
        </div>
      </div>
      <!-- Hidden file input for attachments -->
      <input
        ref="fileInputRef"
        type="file"
        class="hidden"
        multiple
        accept="image/*"
        @change="onFilesSelected"
      />
      
    </div>
  </div>
</template>

<script setup lang="ts">
import { onMounted, onUnmounted, ref, computed, nextTick, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { invoke } from '@tauri-apps/api/core'
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow'
import type { UnlistenFn } from '@tauri-apps/api/event'
import DocumentModeSelector from './Agent/DocumentModeSelector.vue'
import type { PendingDocumentAttachment, ProcessedDocumentResult, DockerAnalysisStatus } from '@/types/agent'

const { t } = useI18n()

// 流量引用类型
type TrafficSendType = 'request' | 'response' | 'both'
interface ReferencedTraffic {
  id: number
  url: string
  method: string
  host: string
  status_code: number
  request_headers?: string
  request_body?: string
  response_headers?: string
  response_body?: string
  sendType?: TrafficSendType
}

// Context usage info type
interface ContextUsageInfo {
  usedTokens: number
  maxTokens: number
  usagePercentage: number
  systemPromptTokens: number
  historyTokens: number
  historyCount: number
  summaryTokens: number
  summaryGlobalTokens: number
  summarySegmentTokens: number
  summarySegmentCount: number
}

const props = defineProps<{
  inputMessage: string
  isLoading: boolean
  showDebugInfo: boolean
  allowTakeover?: boolean
  ragEnabled?: boolean
  toolsEnabled?: boolean
  pendingAttachments?: any[]
  pendingDocuments?: PendingDocumentAttachment[]
  processedDocuments?: ProcessedDocumentResult[]
  referencedTraffic?: ReferencedTraffic[]
  contextUsage?: ContextUsageInfo | null
}>()

const emit = defineEmits<{
  (e: 'update:input-message', value: string): void
  (e: 'send-message'): void
  (e: 'stop-execution'): void
  (e: 'toggle-debug', value: boolean): void
  (e: 'create-new-conversation'): void
  (e: 'clear-conversation'): void
  (e: 'toggle-rag', enabled: boolean): void
  (e: 'toggle-tools', enabled: boolean): void
  (e: 'open-tool-config'): void
  (e: 'add-attachments', files: string[]): void
  (e: 'remove-attachment', index: number): void
  (e: 'add-documents', files: PendingDocumentAttachment[]): void
  (e: 'remove-document', index: number): void
  (e: 'document-processed', result: ProcessedDocumentResult): void
  (e: 'remove-traffic', index: number): void
  (e: 'clear-traffic'): void
}>()

// removed architecture utilities

const allowTakeover = computed(() => props.allowTakeover === true)

// --- New input logic ---
const textareaRef = ref<HTMLTextAreaElement | null>(null)
const containerRef = ref<HTMLDivElement | null>(null)
const fileInputRef = ref<HTMLInputElement | null>(null)

// --- Persistence helpers ---
const STORAGE_KEYS = {
  rag: 'sentinel:input:ragEnabled',
  tools: 'sentinel:input:toolsEnabled',
} as const

const getBool = (key: string, fallback = false) => {
  try {
    const v = localStorage.getItem(key)
    if (v === null) return fallback
    return v === '1' || v === 'true'
  } catch {
    return fallback
  }
}

const setBool = (key: string, value: boolean) => {
  try {
    localStorage.setItem(key, value ? '1' : '0')
  } catch {
    // ignore
  }
}

// Feature states (controlled by parent via props, with persistence)
const localRagEnabled = ref<boolean>(!!props.ragEnabled)
const localToolsEnabled = ref<boolean>(!!props.toolsEnabled)

// init guard
const initialized = ref(false)

// popover positioning (fixed)
const popoverStyle = ref<Record<string, string>>({})
const updatePopoverPosition = () => {
  const el = containerRef.value
  if (!el) return
  const rect = el.getBoundingClientRect()
  const desiredWidth = Math.min(rect.width, 384) // 24rem max
  popoverStyle.value = {
    top: `${rect.top}px`,
    left: `${rect.left}px`,
    width: `${desiredWidth}px`,
    transform: 'translateY(calc(-100% - 8px))',
  }
}

const placeholderText = computed(() => '在这里输入消息，按 Enter 发送')

const autoResize = () => {
  const el = textareaRef.value
  if (!el) return
  el.style.height = 'auto'
  el.style.height = Math.min(el.scrollHeight, 320) + 'px'
}

const onInput = (e: Event) => {
  const target = e.target as HTMLTextAreaElement
  emit('update:input-message', target.value)
  autoResize()
}

// 检查是否有未选择处理模式的文档
const hasUnprocessedDocuments = computed(() => {
  if (!props.pendingDocuments || props.pendingDocuments.length === 0) return false
  // 检查是否有文档没有选择处理模式
  return props.pendingDocuments.some(doc => !doc.processing_mode)
})

// Context usage computed properties
const estimateTokens = (text: string): number => {
  if (!text) return 0
  // Heuristic: CJK chars are closer to 1 token each, others ~4 chars/token
  const cjkCount = (text.match(/[\u4e00-\u9fff]/g) || []).length
  const nonCjk = Math.max(0, text.length - cjkCount)
  const cjkTokens = cjkCount
  const nonCjkTokens = Math.ceil(nonCjk / 4)
  return cjkTokens + nonCjkTokens
}

const inputTokenEstimate = computed(() => estimateTokens(props.inputMessage || ''))

const effectiveContextUsage = computed(() => {
  const base = props.contextUsage
  const inputTokens = inputTokenEstimate.value
  if (!base) {
    if (inputTokens === 0) return null
    const maxTokens = 128000
    const usedTokens = inputTokens
    const usagePercentage = maxTokens > 0
      ? Math.min(100, (usedTokens / maxTokens) * 100)
      : 0
    return {
      usedTokens,
      maxTokens,
      usagePercentage,
      systemPromptTokens: 0,
      historyTokens: inputTokens,
      historyCount: 0,
      summaryTokens: 0,
      summaryGlobalTokens: 0,
      summarySegmentTokens: 0,
      summarySegmentCount: 0,
    }
  }
  if (inputTokens === 0) return base
  const usedTokens = base.usedTokens + inputTokens
  const historyTokens = base.historyTokens + inputTokens
  const usagePercentage = base.maxTokens > 0
    ? Math.min(100, (usedTokens / base.maxTokens) * 100)
    : 0
  return {
    ...base,
    usedTokens,
    historyTokens,
    usagePercentage,
  }
})

const contextUsagePercentage = computed(() => {
  if (!effectiveContextUsage.value) return 0
  return Math.round(effectiveContextUsage.value.usagePercentage * 10) / 10
})

const contextUsageClass = computed(() => {
  const percentage = contextUsagePercentage.value
  if (percentage >= 90) return 'bg-error/20 text-error border border-error/30'
  if (percentage >= 70) return 'bg-warning/20 text-warning border border-warning/30'
  if (percentage >= 50) return 'bg-info/20 text-info border border-info/30'
  return 'bg-base-300/50 text-base-content/70 border border-base-300'
})

const contextUsageTooltip = computed(() => {
  if (!effectiveContextUsage.value) return ''
  const { 
    usedTokens, 
    maxTokens, 
    systemPromptTokens, 
    historyTokens, 
    historyCount, 
    summaryTokens, 
    summaryGlobalTokens, 
    summarySegmentTokens, 
    summarySegmentCount 
  } = effectiveContextUsage.value
  const inputHint = inputTokenEstimate.value > 0 ? `\n${t('agent.inputTokens')}: ~${formatTokenCount(inputTokenEstimate.value)}` : ''
  return `${t('agent.contextUsageDetails')}
${t('agent.systemPromptTokens')}: ${formatTokenCount(systemPromptTokens)}
${t('agent.summaryTokens')}: ${formatTokenCount(summaryTokens)}
${t('agent.summaryGlobalTokens')}: ${formatTokenCount(summaryGlobalTokens)}
${t('agent.summarySegmentTokens')}: ${formatTokenCount(summarySegmentTokens)} (${t('agent.summarySegments')}: ${summarySegmentCount})
${t('agent.historyTokens')}: ${formatTokenCount(historyTokens)}
${t('agent.historyMessages')}: ${historyCount}
${t('agent.totalUsed')}: ${formatTokenCount(usedTokens)} / ${formatTokenCount(maxTokens)}${inputHint}`
})

const formatTokenCount = (count: number): string => {
  if (count >= 1000000) {
    return (count / 1000000).toFixed(1) + 'M'
  }
  if (count >= 1000) {
    return (count / 1000).toFixed(1) + 'K'
  }
  return count.toString()
}

// 检查是否可以发送
const canSend = computed(() => {
  if (!props.inputMessage.trim()) return false
  if (hasUnprocessedDocuments.value) return false
  return true
})

const emitSend = () => {
  if (!canSend.value) return
  if (props.isLoading && !allowTakeover.value) return
  emit('send-message')
  // 发送后恢复高度
  requestAnimationFrame(() => autoResize())
}

const handleStop = () => {
  console.log('InputAreaComponent: 停止按钮被点击')
  emit('stop-execution')
}

// Track IME composition state
const isComposing = ref(false)

const onCompositionStart = () => {
  isComposing.value = true
}

const onCompositionEnd = () => {
  isComposing.value = false
}

const onKeydown = (e: KeyboardEvent) => {
  // Ignore Enter key during IME composition
  if (isComposing.value && e.key === 'Enter') {
    return
  }
  
  if (e.key === 'Enter' && !e.shiftKey) {
    e.preventDefault()
    emitSend()
  } else if (e.key === 'Enter' && e.shiftKey) {
    // allow newline
    return
  } else if (e.key === 'Escape') {
    // blur
    (e.target as HTMLTextAreaElement).blur()
  }
}

const clearCurrent = () => {
  emit('update:input-message', '')
  requestAnimationFrame(() => autoResize())
}

const createNewConversation = () => {
  emit('create-new-conversation')
  // 清空输入框
  emit('update:input-message', '')
  requestAnimationFrame(() => autoResize())
}

const clearConversation = () => {
  emit('clear-conversation')
  // 清空输入框
  emit('update:input-message', '')
  requestAnimationFrame(() => autoResize())
}

const toggleRAG = () => {
  localRagEnabled.value = !localRagEnabled.value
  setBool(STORAGE_KEYS.rag, localRagEnabled.value)
  // 通知父组件RAG状态变化
  emit('toggle-rag', localRagEnabled.value)
}

const toggleTools = () => {
  localToolsEnabled.value = !localToolsEnabled.value
  setBool(STORAGE_KEYS.tools, localToolsEnabled.value)
  // 通知父组件Tools状态变化
  emit('toggle-tools', localToolsEnabled.value)
}

// 点击外部区域关闭弹层
const handleClickOutside = (_e: MouseEvent) => {}

const triggerFileSelect = async () => {
  // 直接按 Tauri 环境处理：使用原生文件选择对话框
  try {
    const { open } = await import('@tauri-apps/plugin-dialog')
    const selected = await open({
      multiple: true,
      filters: [
        {
          name: 'Images',
          extensions: ['jpg', 'jpeg', 'png', 'gif', 'webp'],
        },
      ],
    })

    if (selected) {
      const filePaths = Array.isArray(selected) ? selected : [selected]
      emit('add-attachments', filePaths)
    }
  } catch (error) {
    console.error('[InputArea] Tauri 文件选择失败:', error)
  }
}

// 保留 onFilesSelected 仅作为兜底（理论上不会触发）
const onFilesSelected = (e: Event) => {
  const input = e.target as HTMLInputElement
  if (!input.files || input.files.length === 0) return
  const files = Array.from(input.files)
  console.warn('[InputArea] 收到 File 对象，当前默认按 Tauri 模式运行，建议通过对话框选择文件')
  // 重置 input
  input.value = ''
}

// 获取附件预览图
const getAttachmentPreview = (att: any): string => {
  try {
    // 兼容两种结构：MessageAttachment::Image{ image } 或直接 { data, media_type, filename }
    const img = att.image ?? att
    const mediaTypeRaw: string | undefined = img?.media_type
    const mime = toMimeType(mediaTypeRaw)
    const dataField = img?.data
    const base64 = typeof dataField === 'string' ? dataField : dataField?.data
    if (!base64) return ''
    return `data:${mime};base64,${base64}`
  } catch (e) {
    console.error('[InputArea] 构造图片预览失败:', e, att)
    return ''
  }
}

// 将枚举/简写媒体类型转换为标准MIME
const toMimeType = (mediaType?: string): string => {
  if (!mediaType) return 'image/jpeg'
  const t = mediaType.toLowerCase()
  if (t === 'jpeg' || t === 'jpg') return 'image/jpeg'
  if (t === 'png') return 'image/png'
  if (t === 'gif') return 'image/gif'
  if (t === 'webp') return 'image/webp'
  return t.startsWith('image/') ? t : `image/${t}`
}

// 移除附件
const removeAttachment = (index: number) => {
  emit('remove-attachment', index)
}

// 流量显示辅助函数
const getMethodBadgeClass = (method: string): string => {
  switch (method?.toUpperCase()) {
    case 'GET': return 'badge-info'
    case 'POST': return 'badge-success'
    case 'PUT': return 'badge-warning'
    case 'DELETE': return 'badge-error'
    case 'PATCH': return 'badge-accent'
    default: return 'badge-ghost'
  }
}

const getStatusBadgeClass = (status: number): string => {
  if (!status || status === 0) return 'badge-ghost'
  if (status >= 200 && status < 300) return 'badge-success'
  if (status >= 300 && status < 400) return 'badge-info'
  if (status >= 400 && status < 500) return 'badge-warning'
  if (status >= 500) return 'badge-error'
  return 'badge-ghost'
}

const getUrlPath = (url: string): string => {
  try {
    const urlObj = new URL(url)
    const path = urlObj.pathname + urlObj.search
    return path.length > 30 ? path.substring(0, 30) + '...' : path
  } catch {
    return url.length > 30 ? url.substring(0, 30) + '...' : url
  }
}

const getTypeBadgeClass = (type?: TrafficSendType): string => {
  switch (type) {
    case 'request': return 'badge-primary'
    case 'response': return 'badge-secondary'
    default: return 'badge-accent'
  }
}

const getTypeLabel = (type?: TrafficSendType): string => {
  switch (type) {
    case 'request': return 'REQ'
    case 'response': return 'RES'
    default: return 'ALL'
  }
}

// 聚焦输入框
const focusInput = () => {
  nextTick(() => {
    textareaRef.value?.focus()
  })
}

// ====== 文档拖放功能 ======
const isDragOver = ref(false)
const dockerAvailable = ref(false)
let unlistenDragDrop: UnlistenFn | null = null

// 检查 Docker 状态
const checkDockerStatus = async () => {
  try {
    const status = await invoke<DockerAnalysisStatus>('check_docker_for_file_analysis')
    dockerAvailable.value = status.ready_for_file_analysis
  } catch (error) {
    console.error('Failed to check Docker status:', error)
    dockerAvailable.value = false
  }
}

// 设置 Tauri 原生拖放监听
const setupTauriDragDrop = async () => {
  try {
    const webview = getCurrentWebviewWindow()
    
    // Listen for drag-drop events
    unlistenDragDrop = await webview.onDragDropEvent(async (event) => {
      console.log('[InputArea] Drag drop event:', event.payload.type)
      
      if (event.payload.type === 'over' || event.payload.type === 'enter') {
        isDragOver.value = true
      } else if (event.payload.type === 'leave') {
        isDragOver.value = false
      } else if (event.payload.type === 'drop') {
        isDragOver.value = false
        const paths = event.payload.paths
        console.log('[InputArea] Dropped files:', paths)
        
        if (!paths || paths.length === 0) return
        
        await processDroppedFiles(paths)
      }
    })
    
    console.log('[InputArea] Tauri drag-drop listener registered')
  } catch (error) {
    console.error('[InputArea] Failed to setup Tauri drag-drop:', error)
  }
}

// 处理拖放的文件
const processDroppedFiles = async (paths: string[]) => {
  const imageFiles: string[] = []
  const documentFiles: PendingDocumentAttachment[] = []

  for (const filePath of paths) {
    const fileName = filePath.split('/').pop() || filePath.split('\\').pop() || 'unknown'
    const ext = fileName.split('.').pop()?.toLowerCase() || ''
    
    console.log('[InputArea] Processing file:', fileName, 'ext:', ext)
    
    // 判断是图片还是其他文件（其他文件统一作为文档处理）
    if (['jpg', 'jpeg', 'png', 'gif', 'webp'].includes(ext)) {
      imageFiles.push(filePath)
    } else {
      // 所有非图片文件都作为文档处理（包括未识别的文件类型）
      let fileSize = 0
      try {
        const stat = await invoke<{ size: number }>('get_file_stat', { path: filePath })
        fileSize = stat.size
      } catch {
        console.warn('[InputArea] Could not get file size for:', filePath)
      }
      
      documentFiles.push({
        id: crypto.randomUUID(),
        original_path: filePath,
        original_filename: fileName,
        file_size: fileSize,
        mime_type: getMimeTypeFromExt(ext),
        processing_mode: undefined,
      })
    }
  }

  if (documentFiles.length > 0) {
    console.log('[InputArea] Adding', documentFiles.length, 'document(s)')
    emit('add-documents', documentFiles)
    await checkDockerStatus()
  }
  
  if (imageFiles.length > 0) {
    console.log('[InputArea] Adding', imageFiles.length, 'image(s)')
    emit('add-attachments', imageFiles)
  }
}

// 从扩展名获取 MIME 类型
const getMimeTypeFromExt = (ext: string): string => {
  const mimeMap: Record<string, string> = {
    // Office 文档
    docx: 'application/vnd.openxmlformats-officedocument.wordprocessingml.document',
    doc: 'application/msword',
    xlsx: 'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet',
    xls: 'application/vnd.ms-excel',
    pptx: 'application/vnd.openxmlformats-officedocument.presentationml.presentation',
    ppt: 'application/vnd.ms-powerpoint',
    pdf: 'application/pdf',
    rtf: 'application/rtf',
    // 邮件
    eml: 'message/rfc822',
    msg: 'application/vnd.ms-outlook',
    // 文本类
    txt: 'text/plain',
    md: 'text/markdown',
    json: 'application/json',
    xml: 'application/xml',
    csv: 'text/csv',
    html: 'text/html',
    htm: 'text/html',
    css: 'text/css',
    // 代码文件（统一作为文本处理）
    js: 'text/javascript',
    ts: 'text/typescript',
    jsx: 'text/javascript',
    tsx: 'text/typescript',
    py: 'text/x-python',
    java: 'text/x-java',
    c: 'text/x-c',
    cpp: 'text/x-c++',
    h: 'text/x-c',
    hpp: 'text/x-c++',
    rs: 'text/x-rust',
    go: 'text/x-go',
    rb: 'text/x-ruby',
    php: 'text/x-php',
    sh: 'text/x-shellscript',
    bash: 'text/x-shellscript',
    zsh: 'text/x-shellscript',
    sql: 'text/x-sql',
    yaml: 'text/yaml',
    yml: 'text/yaml',
    toml: 'text/x-toml',
    ini: 'text/x-ini',
    conf: 'text/plain',
    cfg: 'text/plain',
    log: 'text/plain',
    // 压缩文件
    zip: 'application/zip',
    tar: 'application/x-tar',
    gz: 'application/gzip',
    rar: 'application/vnd.rar',
    '7z': 'application/x-7z-compressed',
  }
  // 未知类型默认作为文本处理
  return mimeMap[ext] || 'text/plain'
}

// 保留 HTML5 拖放作为备用（用于视觉反馈）
const onDragOver = (e: DragEvent) => {
  // Tauri 会处理实际的拖放，这里只用于视觉反馈
  if (e.dataTransfer?.types.includes('Files')) {
    isDragOver.value = true
  }
}

const onDragLeave = () => {
  isDragOver.value = false
}

const onDrop = async (_e: DragEvent) => {
  // Tauri 的 onDragDropEvent 会处理实际的文件
  // 这里只重置状态
  isDragOver.value = false
}

// 获取已处理的文档结果
const getProcessedResult = (docId: string): ProcessedDocumentResult | undefined => {
  return props.processedDocuments?.find(d => d.id === docId)
}

// 移除文档
const removeDocument = (index: number) => {
  emit('remove-document', index)
}

// 文档处理完成
const onDocumentProcessed = (result: ProcessedDocumentResult) => {
  emit('document-processed', result)
}

// 文档处理错误
const onDocumentError = (error: string) => {
  console.error('Document processing error:', error)
}

onMounted(async () => {
  autoResize()
  // 同步父组件传入的初始值
  // Initialize persistent states (persisted values take precedence)
  try {
    // RAG: prefer persisted value if exists, otherwise use prop
    const hasPersistedRag = localStorage.getItem(STORAGE_KEYS.rag) !== null
    const savedRag = hasPersistedRag ? getBool(STORAGE_KEYS.rag) : !!props.ragEnabled
    localRagEnabled.value = savedRag
    setBool(STORAGE_KEYS.rag, savedRag)
    emit('toggle-rag', savedRag)
    
    // Tools: prefer persisted value if exists, otherwise use prop
    const hasPersistedTools = localStorage.getItem(STORAGE_KEYS.tools) !== null
    const savedTools = hasPersistedTools ? getBool(STORAGE_KEYS.tools) : !!props.toolsEnabled
    localToolsEnabled.value = savedTools
    setBool(STORAGE_KEYS.tools, savedTools)
    emit('toggle-tools', savedTools)
  } catch {
    // fallback to props on any error
    localRagEnabled.value = !!props.ragEnabled
    localToolsEnabled.value = !!props.toolsEnabled
  }
  initialized.value = true
  window.addEventListener('resize', updatePopoverPosition)
  window.addEventListener('scroll', updatePopoverPosition, true)
  window.addEventListener('click', handleClickOutside, true)
  
  // 设置 Tauri 拖放监听
  await setupTauriDragDrop()
  
  // 检查 Docker 状态
  await checkDockerStatus()
  
  // 自动聚焦输入框
  focusInput()
})

onUnmounted(() => {
  window.removeEventListener('resize', updatePopoverPosition)
  window.removeEventListener('scroll', updatePopoverPosition, true)
  window.removeEventListener('click', handleClickOutside, true)
  
  // 清理 Tauri 拖放监听
  if (unlistenDragDrop) {
    unlistenDragDrop()
    unlistenDragDrop = null
  }
})

// 监听父组件状态变化，保持本地按钮状态一致（并持久化）
watch(
  () => props.ragEnabled,
  (val) => {
    if (typeof val === 'boolean') {
      localRagEnabled.value = val
      setBool(STORAGE_KEYS.rag, val)
    }
  }
)

// 监听工具状态变化（父组件从数据库加载后会更新）
watch(
  () => props.toolsEnabled,
  (val) => {
    if (typeof val === 'boolean') {
      localToolsEnabled.value = val
      setBool(STORAGE_KEYS.tools, val)
    }
  }
)

// 暴露方法供父组件调用
defineExpose({
  focusInput,
})

// End script
</script>

<style scoped>
.input-area-container {
  /* Ensure input area doesn't overlap sidebar */
  width: 100%;
  max-width: 100%;
}

.chat-input { 
  position: relative; 
}

.icon-btn { 
  width:1.75rem; 
  height:1.75rem; 
  display:flex; 
  align-items:center; 
  justify-content:center; 
  border-radius:0.375rem; 
  font-size:calc(var(--font-size-base, 14px) * 0.75); 
  transition:background-color .15s,color .15s; 
}

.icon-btn:hover { 
  background-color: hsl(var(--b3)/0.7); 
}

.icon-btn.active { 
  background: hsl(var(--p)); 
  color: hsl(var(--pc)); 
  box-shadow:0 2px 4px rgba(0,0,0,.15); 
}

.send-btn { 
  width:2rem; 
  height:2rem; 
  border-radius:9999px; 
  background: hsl(var(--b3)); 
  color: hsl(var(--bc)); 
  display:flex; 
  align-items:center; 
  justify-content:center; 
  transition: background-color .15s,color .15s; 
}

.send-btn:hover { 
  background: hsl(var(--p)); 
  color: hsl(var(--pc)); 
}

.send-btn:disabled { 
  opacity:.4; 
  cursor:not-allowed; 
}

/* Search popover positioned above the toolbar */
.search-popover {
  position: absolute;
  bottom: calc(100% + 8px);
  left: 0;
  width: 24rem;
  max-width: 90vw;
  z-index: 50;
}

@media (max-width: 640px) {
  .search-popover { 
    width: 18rem; 
  }
}

/* Drag and drop styles */
.input-area-container.drag-over {
  position: relative;
}

.drag-overlay {
  position: absolute;
  inset: 0;
  background: hsl(var(--p) / 0.1);
  border: 2px dashed hsl(var(--p));
  border-radius: 0.5rem;
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 100;
  backdrop-filter: blur(4px);
}

.drag-content {
  display: flex;
  flex-direction: column;
  align-items: center;
  color: hsl(var(--p));
  text-align: center;
  padding: 1rem;
}

/* Context usage indicator */
.context-usage-indicator {
  font-variant-numeric: tabular-nums;
  white-space: nowrap;
  transition: all 0.2s ease;
}

.context-usage-indicator:hover {
  opacity: 0.9;
}
</style>
