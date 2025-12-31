<template>
  <!-- AI Chat Panel -->
  <div v-if="show" class="ai-chat-panel" :style="{ width: panelWidth + 'px' }">
    <!-- Resize Handle -->
    <div class="resize-handle" @mousedown="startResize"></div>

    <div class="ai-chat-header">
      <div class="flex items-center gap-2">
        <i class="fas fa-robot text-primary"></i>
        <span class="font-semibold">{{ $t('plugins.aiAssistant', 'AI 助手') }}</span>
      </div>
      <button class="btn btn-xs btn-ghost btn-circle" @click="$emit('close')">
        <i class="fas fa-times"></i>
      </button>
    </div>
    
    <!-- Chat Messages -->
    <div class="ai-chat-messages" ref="aiChatMessagesRef">
      <div v-if="messages.length === 0" class="ai-chat-empty">
        <i class="fas fa-comments text-4xl opacity-30 mb-3"></i>
        <p class="text-sm opacity-60">{{ $t('plugins.aiAssistantHint', '描述你想要的修改，AI 将帮助你编辑代码') }}</p>
        <div class="ai-quick-actions mt-4">
          <button class="btn btn-xs btn-outline" @click="$emit('quickAction', 'explain')">
            <i class="fas fa-lightbulb mr-1"></i>{{ $t('plugins.explainCode', '解释代码') }}
          </button>
          <button class="btn btn-xs btn-outline" @click="$emit('quickAction', 'optimize')">
            <i class="fas fa-bolt mr-1"></i>{{ $t('plugins.optimizeCode', '优化代码') }}
          </button>
          <button class="btn btn-xs btn-outline" @click="$emit('quickAction', 'fix')">
            <i class="fas fa-bug mr-1"></i>{{ $t('plugins.fixBugs', '修复问题') }}
          </button>
        </div>
      </div>
      
      <template v-else>
        <div v-for="(msg, idx) in messages" :key="idx" 
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
                      <button class="btn btn-mini h-6 min-h-0 btn-ghost px-2" @click="$emit('previewCode', block)">
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
        <div v-if="streaming" class="ai-chat-message assistant">
          <div class="message-avatar">
            <i class="fas fa-robot"></i>
          </div>
          <div class="message-content">
            <div class="message-text streaming-text">
              <div v-if="streamingContent" v-html="streamingContentRendered"></div>
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
      <div v-if="codeRef" class="code-reference-badge">
        <div class="code-ref-header">
          <i class="fas fa-code text-xs"></i>
          <span class="text-xs font-medium">
            {{ codeRef.isFullCode 
              ? $t('plugins.fullCode', '完整代码') 
              : $t('plugins.selectedLines', '选中代码') + ` (${codeRef.startLine}-${codeRef.endLine})` 
            }}
          </span>
          <button class="btn btn-xs btn-ghost btn-circle ml-auto" @click="$emit('clearCodeRef')">
            <i class="fas fa-times text-xs"></i>
          </button>
        </div>
        <div class="code-ref-preview">
          <pre><code>{{ codeRef.preview }}</code></pre>
        </div>
      </div>

      <!-- Test Result Reference Badge -->
      <div v-if="testResultRef" class="test-result-reference-badge">
        <div class="test-ref-header">
          <i :class="testResultRef.success ? 'fas fa-check-circle text-success' : 'fas fa-times-circle text-error'" class="text-xs"></i>
          <span class="text-xs font-medium">
            {{ $t('plugins.testResultRef', '测试结果') }}
            <span :class="testResultRef.success ? 'text-success' : 'text-error'">
              ({{ testResultRef.success ? $t('common.success', '成功') : $t('common.failed', '失败') }})
            </span>
          </span>
          <button class="btn btn-xs btn-ghost btn-circle ml-auto" @click="$emit('clearTestResultRef')">
            <i class="fas fa-times text-xs"></i>
          </button>
        </div>
        <div class="test-ref-preview">
          <pre><code>{{ testResultRef.preview }}</code></pre>
        </div>
      </div>
      
      <textarea 
        v-model="inputText"
        :placeholder="$t('plugins.aiInputPlaceholder', '描述你想要的修改...')"
        class="textarea textarea-bordered w-full resize-none"
        rows="2"
        :disabled="streaming"
        @keydown.enter.exact.prevent="handleSendMessage"
      ></textarea>
      <div class="ai-chat-input-actions">
        <div class="flex items-center gap-2 text-xs opacity-60">
          <i class="fas fa-info-circle"></i>
          <span>{{ $t('plugins.contextMenuHint', '右键编辑器添加代码到上下文') }}</span>
        </div>
        <button 
          class="btn btn-sm btn-primary" 
          :disabled="!inputText.trim() || streaming"
          @click="handleSendMessage"
        >
          <span v-if="streaming" class="loading loading-spinner loading-xs"></span>
          <i v-else class="fas fa-paper-plane"></i>
        </button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch, nextTick } from 'vue'
import { marked } from 'marked'
import DOMPurify from 'dompurify'
import type { CodeReference, TestResultReference, AiChatMessage } from './types'

const props = defineProps<{
  show: boolean
  messages: AiChatMessage[]
  streaming: boolean
  streamingContent: string
  codeRef: CodeReference | null
  testResultRef: TestResultReference | null
}>()

const emit = defineEmits<{
  'close': []
  'sendMessage': [message: string]
  'quickAction': [action: string]
  'applyCode': [code: string, context?: CodeReference | null]
  'previewCode': [code: string]
  'clearCodeRef': []
  'clearTestResultRef': []
}>()

const aiChatMessagesRef = ref<HTMLDivElement>()
const inputText = ref('')
const panelWidth = ref(400)
const isResizing = ref(false)

// Configure marked for streaming content
marked.setOptions({
  breaks: true,
  gfm: true,
})

// Render streaming content as Markdown
const streamingContentRendered = computed(() => {
  if (!props.streamingContent) return ''
  const rawHtml = marked.parse(props.streamingContent) as string
  return DOMPurify.sanitize(rawHtml, {
    ALLOWED_TAGS: ['p', 'br', 'strong', 'em', 'code', 'pre', 'ul', 'ol', 'li', 'h1', 'h2', 'h3', 'h4', 'h5', 'h6', 'blockquote', 'a'],
    ALLOWED_ATTR: ['href', 'class']
  })
})

const handleApplyCode = (code: string, messageIndex: number) => {
  // Find the preceding user message to get code context
  let codeRef: CodeReference | null = null
  for (let i = messageIndex - 1; i >= 0; i--) {
    if (props.messages[i].role === 'user') {
      codeRef = props.messages[i].codeRef || null
      break
    }
  }
  emit('applyCode', code, codeRef)
}

const handleApplyAllCode = (code: string, messageIndex: number) => {
  // Find the preceding user message to get code context
  let codeRef: CodeReference | null = null
  for (let i = messageIndex - 1; i >= 0; i--) {
    if (props.messages[i].role === 'user') {
      codeRef = props.messages[i].codeRef || null
      break
    }
  }
  emit('applyCode', code, codeRef)
}

const handleSendMessage = () => {
  if (inputText.value.trim() && !props.streaming) {
    emit('sendMessage', inputText.value)
  }
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
    panelWidth.value = newWidth
  }
}

const stopResize = () => {
  isResizing.value = false
  document.removeEventListener('mousemove', handleResize)
  document.removeEventListener('mouseup', stopResize)
  document.body.style.userSelect = ''
}

// Auto scroll to bottom when new messages arrive
watch(() => props.messages.length, () => {
  nextTick(() => {
    if (aiChatMessagesRef.value) {
      aiChatMessagesRef.value.scrollTop = aiChatMessagesRef.value.scrollHeight
    }
  })
})

// Clear input immediately when streaming starts
watch(() => props.streaming, (streaming) => {
  if (streaming) {
    inputText.value = ''
  }
})

defineExpose({
  inputText
})
</script>

<style scoped>
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
</style>
