<template>
  <div class="border-t border-base-300/50 bg-base-100 flex-shrink-0">
    

    <!-- Input area (refactored) -->
    <div class="px-4 pb-3 pt-2">
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
            <button class="icon-btn" title="新建会话" @click="createNewConversation"><i class="fas fa-plus"></i></button>
            <button class="icon-btn" title="附件"><i class="fas fa-paperclip"></i></button>
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
              @click="$emit('stop-execution')"
              title="停止"
            >
              <i class="fas fa-stop"></i>
            </button>
          </div>
        </div>
      </div>
      
    </div>
  </div>
</template>

<script setup lang="ts">
import { onMounted, onUnmounted, ref, computed, nextTick } from 'vue'
import { sendMessageWithSearch } from '../services/search'

const props = defineProps<{
  inputMessage: string
  isLoading: boolean
  showDebugInfo: boolean
}>()

const emit = defineEmits([
  'update:input-message',
  'send-message',
  'stop-execution',
  'toggle-debug',
  'create-new-conversation',
  'clear-conversation',
  'toggle-task-mode',
  'toggle-rag'
])

// removed architecture utilities

// --- New input logic ---
const textareaRef = ref<HTMLTextAreaElement | null>(null)
const containerRef = ref<HTMLDivElement | null>(null)

// search state
const showSearch = ref(false)
const searchEnabled = ref(false)

// task mode state
const taskModeEnabled = ref(false)

// RAG state
const ragEnabled = ref(false)

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
}

const toggleRAG = () => {
  ragEnabled.value = !ragEnabled.value
  // 通知父组件RAG状态变化
  emit('toggle-rag', ragEnabled.value)
}

const toggleTaskMode = () => {
  taskModeEnabled.value = !taskModeEnabled.value
  emit('toggle-task-mode', taskModeEnabled.value)
}

// 点击外部区域关闭弹层
const handleClickOutside = (_e: MouseEvent) => {}


onMounted(() => {
  autoResize()
  window.addEventListener('resize', updatePopoverPosition)
  window.addEventListener('scroll', updatePopoverPosition, true)
  window.addEventListener('click', handleClickOutside, true)
})

onUnmounted(() => {
  window.removeEventListener('resize', updatePopoverPosition)
  window.removeEventListener('scroll', updatePopoverPosition, true)
  window.removeEventListener('click', handleClickOutside, true)
})

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
