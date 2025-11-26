<template>
  <div class="border-t border-base-300/50 bg-base-100 flex-shrink-0">
    

    <!-- Input area (refactored) -->
    <div class="px-4 pb-3 pt-2">
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
            :disabled="isLoading"
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
            <button class="icon-btn" :class="{ active: searchEnabled }" title="联网搜索" @click="toggleWebSearch"><i class="fas fa-globe"></i></button>
            <button class="icon-btn" :class="{ active: ragEnabled }" title="知识检索增强" @click="toggleRAG"><i class="fas fa-brain"></i></button>
            <button class="icon-btn" :class="{ active: taskModeEnabled }" title="任务模式" @click="toggleTaskMode"><i class="fas fa-tasks"></i></button>
            <button class="icon-btn " title="@ 引用"><i class="fas fa-at"></i></button>
            <button class="icon-btn" title="快速指令"><i class="fas fa-bolt"></i></button>
            <button class="icon-btn" title="工具库"><i class="fas fa-box"></i></button>
            <button class="icon-btn" title="选择"><i class="fas fa-border-all"></i></button>
            <button class="icon-btn" title="清空会话" @click="clearConversation"><i class="fas fa-eraser"></i></button>
          </div>

          <!-- Right side icons -->
          <div class="flex items-center gap-2 shrink-0">
            <button class="icon-btn" title="语言 / 翻译"><i class="fas fa-language"></i></button>
            <button
              v-if="!isLoading"
              class="send-btn"
              :disabled="!inputMessage.trim()"
              :class="{ 'opacity-40 cursor-not-allowed': !inputMessage.trim() }"
              @click="emitSend"
              title="发送 (Enter)"
            >
              <i class="fas fa-arrow-up"></i>
            </button>
            <button
              v-else
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

const props = defineProps<{
  inputMessage: string
  isLoading: boolean
  showDebugInfo: boolean
  ragEnabled?: boolean
  pendingAttachments?: any[]
}>()

const emit = defineEmits([
  'update:input-message',
  'send-message',
  'stop-execution',
  'toggle-debug',
  'create-new-conversation',
  'clear-conversation',
  'toggle-task-mode',
  'toggle-rag',
  'add-attachments',
  'remove-attachment'
])

// removed architecture utilities

// --- New input logic ---
const textareaRef = ref<HTMLTextAreaElement | null>(null)
const containerRef = ref<HTMLDivElement | null>(null)
const fileInputRef = ref<HTMLInputElement | null>(null)

// --- Persistence helpers ---
const STORAGE_KEYS = {
  search: 'sentinel:input:searchEnabled',
  rag: 'sentinel:input:ragEnabled',
  task: 'sentinel:input:taskModeEnabled',
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

// search state
const showSearch = ref(false)
const searchEnabled = ref(false)

// task mode state
const taskModeEnabled = ref(false)

// RAG state (controlled by parent via prop, with persistence)
const ragEnabled = ref<boolean>(!!props.ragEnabled)

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

const placeholderText = computed(() => 
  taskModeEnabled.value 
    ? '任务模式：输入任务描述，系统将自动规划和执行步骤' 
    : '普通聊天模式：在这里输入消息，按 Enter 发送'
)

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
  if (!props.inputMessage.trim() || props.isLoading) return
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
}

const toggleRAG = () => {
  ragEnabled.value = !ragEnabled.value
  setBool(STORAGE_KEYS.rag, ragEnabled.value)
  // 通知父组件RAG状态变化
  emit('toggle-rag', ragEnabled.value)
}

const toggleTaskMode = () => {
  taskModeEnabled.value = !taskModeEnabled.value
  setBool(STORAGE_KEYS.task, taskModeEnabled.value)
  emit('toggle-task-mode', taskModeEnabled.value)
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

onMounted(() => {
  autoResize()
  // 同步父组件传入的初始值
  // Initialize persistent states (persisted values take precedence)
  try {
    // search (local only)
    searchEnabled.value = getBool(STORAGE_KEYS.search, searchEnabled.value)

    // task mode (notify parent)
    const savedTask = getBool(STORAGE_KEYS.task, taskModeEnabled.value)
    taskModeEnabled.value = savedTask
    emit('toggle-task-mode', savedTask)

    // RAG: prefer persisted value if exists, otherwise use prop
    const hasPersistedRag = localStorage.getItem(STORAGE_KEYS.rag) !== null
    const savedRag = hasPersistedRag ? getBool(STORAGE_KEYS.rag) : !!props.ragEnabled
    ragEnabled.value = savedRag
    setBool(STORAGE_KEYS.rag, savedRag)
    emit('toggle-rag', savedRag)
  } catch {
    // fallback to prop on any error
    ragEnabled.value = !!props.ragEnabled
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
      ragEnabled.value = val
      setBool(STORAGE_KEYS.rag, val)
    }
  }
)

// End script
</script>

<style scoped>
.chat-input { position: relative; }
.icon-btn { width:1.75rem; height:1.75rem; display:flex; align-items:center; justify-content:center; border-radius:0.375rem; font-size:0.75rem; transition:background-color .15s,color .15s; }
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
