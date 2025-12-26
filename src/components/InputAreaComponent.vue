<template>
  <div class="border-t border-base-300/50 bg-base-100 flex-shrink-0">
    

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

      <!-- 附件预览区（如果有待发附件） -->
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

      <div ref="containerRef" class="chat-input rounded-2xl bg-base-200/60 border border-base-300/60 backdrop-blur-sm flex flex-col gap-2 px-3 py-2 shadow-sm focus-within:border-primary transition-colors">
        <!-- Text input (auto-resize textarea) -->
        <div class="flex-1 min-w-0">
          <textarea
            ref="textareaRef"
            :value="inputMessage"
            @input="onInput"
            @keydown="onKeydown"
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
            <button class="icon-btn" :class="{ active: searchEnabled }" title="联网搜索" @click="toggleWebSearch"><i class="fas fa-globe"></i></button>
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
            <button class="icon-btn" title="语言 / 翻译"><i class="fas fa-language"></i></button>
            <button
              v-if="!isLoading || allowTakeover"
              class="send-btn"
              :disabled="!inputMessage.trim()"
              :class="{ 'opacity-40 cursor-not-allowed': !inputMessage.trim() }"
              @click="emitSend"
              :title="isLoading ? '接管并发送 (Enter)' : '发送 (Enter)'"
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

const props = defineProps<{
  inputMessage: string
  isLoading: boolean
  showDebugInfo: boolean
  allowTakeover?: boolean
  ragEnabled?: boolean
  toolsEnabled?: boolean
  webSearchEnabled?: boolean
  pendingAttachments?: any[]
  referencedTraffic?: ReferencedTraffic[]
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
  (e: 'toggle-web-search', enabled: boolean): void
  (e: 'add-attachments', files: string[]): void
  (e: 'remove-attachment', index: number): void
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
  search: 'sentinel:input:webSearchEnabled',
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
const showSearch = ref(false)
const searchEnabled = ref(false)
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

const emitSend = () => {
  if (!props.inputMessage.trim()) return
  if (props.isLoading && !allowTakeover.value) return
  emit('send-message')
  // 发送后恢复高度
  requestAnimationFrame(() => autoResize())
}

const handleStop = () => {
  console.log('InputAreaComponent: 停止按钮被点击')
  emit('stop-execution')
}

const onKeydown = (e: KeyboardEvent) => {
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

const toggleWebSearch = () => {
  searchEnabled.value = !searchEnabled.value
  setBool(STORAGE_KEYS.search, searchEnabled.value)
  // 通知父组件Web Search状态变化
  emit('toggle-web-search', searchEnabled.value)
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

onMounted(() => {
  autoResize()
  // 同步父组件传入的初始值
  // Initialize persistent states (persisted values take precedence)
  try {
    // Web Search: prefer persisted value if exists, otherwise use prop
    const hasPersistedSearch = localStorage.getItem(STORAGE_KEYS.search) !== null
    const savedSearch = hasPersistedSearch ? getBool(STORAGE_KEYS.search) : !!props.webSearchEnabled
    searchEnabled.value = savedSearch
    setBool(STORAGE_KEYS.search, savedSearch)
    emit('toggle-web-search', savedSearch)

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
    searchEnabled.value = !!props.webSearchEnabled
    ragEnabled.value = !!props.ragEnabled
    toolsEnabled.value = !!props.toolsEnabled
  }
  initialized.value = true
  window.addEventListener('resize', updatePopoverPosition)
  window.addEventListener('scroll', updatePopoverPosition, true)
  window.addEventListener('click', handleClickOutside, true)
})

onUnmounted(() => {
  window.removeEventListener('resize', updatePopoverPosition)
  window.removeEventListener('scroll', updatePopoverPosition, true)
  window.removeEventListener('click', handleClickOutside, true)
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

// End script
</script>

<style scoped>
.chat-input { position: relative; }
.icon-btn { width:1.75rem; height:1.75rem; display:flex; align-items:center; justify-content:center; border-radius:0.375rem; font-size:calc(var(--font-size-base, 14px) * 0.75); transition:background-color .15s,color .15s; }
.icon-btn:hover { background-color: hsl(var(--b3)/0.7); }
.icon-btn.active { background: hsl(var(--p)); color: hsl(var(--pc)); box-shadow:0 2px 4px rgba(0,0,0,.15); }
.send-btn { width:2rem; height:2rem; border-radius:9999px; background: hsl(var(--b3)); color: hsl(var(--bc)); display:flex; align-items:center; justify-content:center; transition: background-color .15s,color .15s; }
.send-btn:hover { background: hsl(var(--p)); color: hsl(var(--pc)); }
.send-btn:disabled { opacity:.4; cursor:not-allowed; }

/* Search popover positioned above the toolbar */
.search-popover {
  position: absolute;
  bottom: calc(100% + 8px);
  left: 0;
  width: 24rem;
  max-width: 90vw;
}

@media (max-width: 640px) {
  .search-popover { width: 18rem; }
}
</style>
