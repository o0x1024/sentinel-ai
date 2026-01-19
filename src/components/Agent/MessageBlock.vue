<template>
  <!-- Tenth Man Critique - Special Alert Message -->
  <div v-if="isTenthManCritique" class="tenth-man-panel rounded-lg overflow-hidden bg-error/10 border-l-4 border-error mb-2 shadow-lg">
    <div class="flex items-center gap-3 px-4 py-3 bg-error/20 border-b border-error/20">
      <div class="w-8 h-8 rounded-full bg-error flex items-center justify-center flex-shrink-0 shadow-sm">
        <i class="fas fa-user-secret text-white text-sm"></i>
      </div>
      <div class="flex-1">
        <div class="font-bold text-error text-sm uppercase tracking-wide">The Tenth Man Rule</div>
        <div class="text-xs text-error/70 font-medium">Adversarial Review Initiated</div>
      </div>
    </div>
    <div class="px-5 py-4 bg-base-100/80 text-base-content relative">
      <!-- Watermark -->
      <i class="fas fa-exclamation-triangle absolute right-4 top-4 text-8xl text-error/5 pointer-events-none"></i>
      <div class="relative z-10 prose prose-sm max-w-none">
        <MarkdownRenderer :content="message.content" />
      </div>
    </div>
  </div>

  <!-- Segment Summary Message - Sliding Window Memory -->
  <div v-else-if="isSegmentSummary" class="segment-summary-panel rounded-lg overflow-hidden bg-info/10 border-l-4 border-info">
    <!-- Panel Header -->
    <div 
      @click="toggleSummaryPanel" 
      class="summary-panel-header flex items-center gap-3 px-4 py-3 cursor-pointer hover:bg-info/20 transition-colors"
    >
      <!-- Icon -->
      <i class="fas fa-layer-group text-info text-lg"></i>
      
      <!-- Title -->
      <div class="flex-1">
        <div class="font-semibold text-sm text-info">{{ t('agent.segmentSummary') }}</div>
        <div class="text-xs text-base-content/60 mt-0.5">
          {{ t('agent.segmentIndex') }}: #{{ message.metadata?.segment_index }} · {{ formatNumber(message.metadata?.summary_tokens) }} tokens
        </div>
      </div>
      
      <!-- Expand/Collapse Icon -->
      <i :class="['fas transition-transform text-xs text-info', isSummaryPanelExpanded ? 'fa-chevron-up' : 'fa-chevron-down']"></i>
    </div>
    
    <!-- Panel Content (collapsible) -->
    <div v-show="isSummaryPanelExpanded" class="summary-panel-content border-t border-info/30">
      <div class="px-4 py-3 bg-base-100/50">
        <div class="text-xs text-base-content/70">
          <div v-if="message.metadata?.summary_content" class="summary-content-box p-3 bg-base-200/50 rounded border border-base-300 max-h-96 overflow-y-auto">
            <MarkdownRenderer :content="message.metadata.summary_content" />
          </div>
        </div>
      </div>
    </div>
  </div>

  <!-- Global Summary Message - Long-term Memory -->
  <div v-else-if="isGlobalSummary" class="global-summary-panel rounded-lg overflow-hidden bg-warning/10 border-l-4 border-warning">
    <!-- Panel Header -->
    <div 
      @click="toggleSummaryPanel" 
      class="summary-panel-header flex items-center gap-3 px-4 py-3 cursor-pointer hover:bg-warning/20 transition-colors"
    >
      <!-- Icon -->
      <i class="fas fa-brain text-warning text-lg"></i>
      
      <!-- Title -->
      <div class="flex-1">
        <div class="font-semibold text-sm text-warning">{{ t('agent.globalSummary') }}</div>
        <div class="text-xs text-base-content/60 mt-0.5">
          {{ t('agent.longTermMemory') }} · {{ formatNumber(message.metadata?.summary_tokens) }} tokens
        </div>
      </div>
      
      <!-- Expand/Collapse Icon -->
      <i :class="['fas transition-transform text-xs text-warning', isSummaryPanelExpanded ? 'fa-chevron-up' : 'fa-chevron-down']"></i>
    </div>
    
    <!-- Panel Content (collapsible) -->
    <div v-show="isSummaryPanelExpanded" class="summary-panel-content border-t border-warning/30">
      <div class="px-4 py-3 bg-base-100/50">
        <div class="text-xs text-base-content/70">
          <div v-if="message.metadata?.summary_content" class="summary-content-box p-3 bg-base-200/50 rounded border border-base-300 max-h-96 overflow-y-auto">
            <MarkdownRenderer :content="message.metadata.summary_content" />
          </div>
        </div>
      </div>
    </div>
  </div>

  <!-- Shell Tool - Render as independent message block -->
  <ShellToolResult
    v-else-if="isShellTool && message.type === 'tool_call'"
    :args="message.metadata?.tool_args"
    :result="message.metadata?.tool_result"
    :error="message.metadata?.error"
    :status="message.metadata?.status"
  />
  
  <!-- Tool Call Message - Collapsible Panel (only render if has content) -->
  <div v-else-if="message.type === 'tool_call' && hasToolCallContent" class="tool-call-panel rounded-lg overflow-hidden  bg-base-200 border-l-4" :class="toolPanelBorderClass">
    <!-- Panel Header (always visible) -->
    <div 
      @click="toggleToolPanel" 
      class="tool-panel-header flex items-center gap-2 px-4 py-3 cursor-pointer hover:bg-base-300/50 transition-colors"
    >
      <!-- Expand/Collapse Icon -->
      <i :class="['fas transition-transform text-xs', isToolPanelExpanded ? 'fa-chevron-down' : 'fa-chevron-right']"></i>
      
      <!-- Tool Name -->
      <span class="font-mono text-sm font-semibold">{{ toolName || 'Tool' }}</span>
      
      <!-- Status Badge -->
      <span v-if="toolStatus" :class="['status-badge px-2 py-0.5 rounded-full text-xs font-medium ml-auto', toolStatusClass]">
        {{ toolStatusText }}
      </span>
      
      <!-- Duration -->
      <span v-if="duration" class="text-xs text-base-content/60">{{ duration }}</span>
    </div>
    
    <!-- Panel Content (collapsible) -->
    <div v-show="isToolPanelExpanded" class="tool-panel-content">
      <!-- Tool Arguments -->
      <div v-if="hasToolArgs" class="border-t border-base-300">
        <div 
          ref="argsBodyRef"
          @click="toggleArgs"
          :class="['px-4 py-3 bg-base-100 cursor-pointer transition-all relative', 
                   isArgsExpanded ? 'max-h-96 overflow-y-auto' : 'max-h-24 overflow-hidden']"
        >
          <div class="text-xs text-base-content/50 mb-2">📥 {{ t('agent.inputParameters') }}</div>
          <pre class="text-xs font-mono text-base-content/70 whitespace-pre-wrap break-words overflow-x-auto">{{ formattedArgs }}</pre>
          
          <!-- Expand hint overlay -->
          <div v-if="!isArgsExpanded && argsHasOverflow" class="expand-hint absolute bottom-0 left-0 right-0 h-8 bg-gradient-to-t from-base-100 to-transparent flex items-end justify-center pb-1 pointer-events-none">
            <span class="text-base-content/50 text-xs">点击展开</span>
          </div>
        </div>
      </div>
      
      <!-- Tool Result -->
      <div v-if="hasToolResult" class="border-t border-base-300">
        <div 
          ref="resultBodyRef"
          @click="toggleResult"
          :class="['px-4 py-3 bg-base-100 cursor-pointer transition-all relative', 
                   isResultExpanded ? 'max-h-96 overflow-y-auto' : 'max-h-24 overflow-hidden']"
        >
          <div class="text-xs text-base-content/50 mb-2">📤 {{ t('agent.executionResult') }}</div>
          <pre class="text-xs font-mono text-base-content/70 whitespace-pre-wrap break-words overflow-x-auto">{{ formattedToolResult }}</pre>
          
          <!-- Expand hint overlay -->
          <div v-if="!isResultExpanded && resultHasOverflow" class="expand-hint absolute bottom-0 left-0 right-0 h-8 bg-gradient-to-t from-base-100 to-transparent flex items-end justify-center pb-1 pointer-events-none">
            <span class="text-base-content/50 text-xs">点击展开</span>
          </div>
        </div>
      </div>
      
      <!-- Tool Call ID -->
      <div v-if="message.metadata?.tool_call_id" class="px-4 py-2 border-t border-base-300 bg-base-100">
        <span class="text-xs text-base-content/50">
          {{ t('agent.toolCallId') }}: <code class="font-mono">{{ message.metadata.tool_call_id }}</code>
        </span>
      </div>
    </div>
  </div>

  <!-- Regular message block for non-tool-call messages (only render if has content) -->
  <div v-else-if="hasRegularMessageContent" class="message-container group relative max-w-full">
    <div :class="['message-block relative rounded-lg px-3 py-2 overflow-hidden', typeClass]">
      <!-- Actions (overlay) - hide when editing -->
      <div
        v-if="!isEditing && (message.type === 'user' || message.type === 'final')"
        class="message-actions absolute right-2 top-2 z-10"
      >
        <!-- Desktop/hover: icon buttons -->
        <div class="hidden md:flex gap-1 opacity-0 group-hover:opacity-100 transition-opacity pointer-events-none group-hover:pointer-events-auto">
          <button
            v-if="message.type === 'user'"
            @click="handleEdit"
            class="action-btn btn btn-xs btn-ghost bg-base-100/70 hover:bg-base-100 backdrop-blur text-base-content/60 hover:text-base-content"
            :title="t('agent.editMessage')"
          >
            <i class="fas fa-edit"></i>
          </button>
          <button
            @click="handleCopy"
            class="action-btn btn btn-xs btn-ghost bg-base-100/70 hover:bg-base-100 backdrop-blur text-base-content/60 hover:text-base-content"
            :title="t('agent.copyMessage')"
          >
            <i :class="['fas', copySuccess ? 'fa-check text-success' : 'fa-copy']"></i>
          </button>
          <button
            v-if="message.type === 'user'"
            @click="handleResend"
            class="action-btn btn btn-xs btn-ghost bg-base-100/70 hover:bg-base-100 backdrop-blur text-base-content/60 hover:text-base-content"
            :title="t('agent.resendMessage')"
          >
            <i class="fas fa-redo"></i>
          </button>
        </div>

        <!-- Touch/mobile: overflow menu -->
        <details class="dropdown dropdown-end md:hidden">
          <summary class="btn btn-xs btn-ghost bg-base-100/70 hover:bg-base-100 backdrop-blur text-base-content/60 hover:text-base-content">
            <i class="fas fa-ellipsis-h"></i>
          </summary>
          <ul class="menu dropdown-content bg-base-100 rounded-box shadow w-40 p-1 mt-1">
            <li v-if="message.type === 'user'">
              <button @click="handleEdit">
                <i class="fas fa-edit"></i>
                <span class="text-xs">{{ t('agent.edit') }}</span>
              </button>
            </li>
            <li>
              <button @click="handleCopy">
                <i :class="['fas', copySuccess ? 'fa-check text-success' : 'fa-copy']"></i>
                <span class="text-xs">{{ t('agent.copy') }}</span>
              </button>
            </li>
            <li v-if="message.type === 'user'">
              <button @click="handleResend">
                <i class="fas fa-redo"></i>
                <span class="text-xs">{{ t('agent.resend') }}</span>
              </button>
            </li>
          </ul>
        </details>
      </div>
      
      <!-- Header with type indicator -->
      <div class="message-header flex items-center gap-2 mb-2 text-sm" v-if="showHeader">
        <span class="message-type font-semibold text-base-content/70">{{ typeName }}</span>
        <span v-if="toolName" class="tool-name font-mono text-xs text-primary">`{{ toolName }}`</span>
        <!-- Tool Status Indicator -->
        <span v-if="toolStatus" :class="['status-badge px-2 py-0.5 rounded text-xs font-medium', toolStatusClass]">
          {{ toolStatusText }}
        </span>
        <span v-if="statusIcon" :class="['status-icon font-bold', statusClass]">{{ statusIcon }}</span>
        <span v-if="duration" class="duration ml-auto text-xs text-base-content/60">{{ duration }}</span>
      </div>
      
      <!-- RAG Citation Indicator -->
      <div v-if="ragInfo" class="rag-indicator flex items-center gap-2 mb-2 px-3 py-2 bg-info/10 rounded-md border border-info/30">
        <i class="fas fa-book text-info text-sm"></i>
        <span class="text-xs text-info font-medium">
          <template v-if="ragInfo.rag_sources_used">
            {{ t('agent.knowledgeBaseCited', { count: ragInfo.source_count }) }}
          </template>
          <template v-else>
            {{ t('agent.noKnowledgeBaseCitations') }}
          </template>
        </span>
      </div>
      
      <!-- Image Attachments (for user messages) -->
      <div v-if="message.type === 'user' && imageAttachments.length > 0" class="image-attachments mb-2">
        <div class="flex items-center gap-2 mb-2">
          <i class="fas fa-image text-primary text-sm"></i>
          <span class="text-xs text-base-content/60">
            {{ t('agent.imageAttachments') }} ({{ imageAttachments.length }})
          </span>
        </div>
        <div class="flex flex-wrap gap-2">
          <div
            v-for="(img, idx) in imageAttachments"
            :key="idx"
            class="image-attachment relative group"
          >
            <img
              :src="getImagePreviewUrl(img)"
              class="h-24 w-24 object-cover rounded border border-base-300 bg-base-200 cursor-pointer hover:opacity-80 transition-opacity"
              :alt="getImageFilename(img)"
              :title="getImageFilename(img)"
              @click="openImagePreview(getImagePreviewUrl(img))"
            />
            <div class="absolute bottom-0 left-0 right-0 bg-black/60 text-white text-xs px-1 py-0.5 truncate opacity-0 group-hover:opacity-100 transition-opacity">
              {{ getImageFilename(img) }}
            </div>
          </div>
        </div>
      </div>
      
      <!-- Content -->
      <div class="message-content text-base-content break-words overflow-hidden">
        <!-- Edit Mode -->
        <div v-if="isEditing" class="edit-mode space-y-2">
          <textarea
            ref="editTextareaRef"
            v-model="editedContent"
            class="w-full bg-base-100 border border-base-300 rounded px-3 py-2 text-sm resize-none focus:outline-none focus:border-primary"
            rows="3"
            @keydown.ctrl.enter="handleSaveEdit"
            @keydown.meta.enter="handleSaveEdit"
            @keydown.esc="handleCancelEdit"
          ></textarea>
          <div class="flex items-center gap-2">
            <button
              @click="handleSaveEdit"
              class="btn btn-xs btn-primary gap-1"
              :disabled="!editedContent.trim()"
            >
              <i class="fas fa-paper-plane"></i>
              <span>{{ t('agent.sendEdited') }}</span>
            </button>
            <button
              @click="handleCancelEdit"
              class="btn btn-xs btn-ghost"
            >
              <i class="fas fa-times"></i>
              <span>{{ t('common.cancel') }}</span>
            </button>
            <span class="text-xs text-base-content/50 ml-auto">
              {{ t('agent.editHint') }}
            </span>
          </div>
        </div>
        
        <!-- Display Mode -->
        <div v-else>
          <div v-if="shouldHideContent" class="text-xs text-base-content/50 italic py-1 flex items-center gap-2">
            <i class="fas fa-external-link-alt"></i>
            <span>{{ t('agent.detailsInVisionPanel') }}</span>
          </div>
          <MarkdownRenderer 
            v-else
            :content="formattedContent" 
            :citations="ragInfo?.citations"
            :show-table-download="showTableDownload"
            @download-table="handleDownloadTable"
            @render-html="(html: string) => emit('renderHtml', html)"
          />
          
          <!-- Document attachments for user messages (shown below content) -->
          <div v-if="message.type === 'user' && documentAttachments.length > 0" class="document-attachments mt-2 pt-2 border-t border-base-300/50">
            <div class="flex flex-wrap gap-2">
              <div
                v-for="doc in documentAttachments"
                :key="doc.id"
                class="doc-attachment inline-flex items-center gap-2 px-2 py-1 rounded-lg text-xs"
                :class="doc.processing_mode === 'security' ? 'bg-warning/20 text-warning' : 'bg-success/20 text-success'"
              >
                <i :class="['fas', doc.processing_mode === 'security' ? 'fa-shield-alt' : 'fa-book-open']"></i>
                <span class="font-medium truncate max-w-32" :title="doc.original_filename">{{ doc.original_filename }}</span>
                <span class="opacity-70">({{ formatDocSize(doc.file_size) }})</span>
              </div>
            </div>
          </div>
        </div>
      </div>
      
      <!-- Tool Result details (for standalone tool_result messages) -->
      <div v-if="message.type === 'tool_result' && (hasToolArgs || message.content)" class="tool-details mt-2 pt-2 border-t border-base-300">
        <button @click="toggleDetails" class="toggle-btn text-xs text-base-content/60 bg-transparent border-none cursor-pointer p-0 underline hover:text-base-content">
          {{ isExpanded ? t('agent.collapseDetails') : t('agent.expandDetails') }}
        </button>
        <div v-if="isExpanded" class="mt-2 space-y-3">
          <!-- Tool Arguments -->
          <div v-if="hasToolArgs" class="tool-args-section">
            <div class="text-xs text-base-content/60 mb-1 font-medium">📥 {{ t('agent.inputParameters') }}:</div>
            <pre class="tool-args p-2 bg-base-300 rounded text-xs font-mono overflow-x-auto text-base-content/70 max-h-48 overflow-y-auto">{{ formattedArgs }}</pre>
          </div>
          <!-- Tool Result -->
          <div v-if="message.content" class="tool-result-section">
            <div class="text-xs text-base-content/60 mb-1 font-medium">📤 {{ t('agent.executionResult') }}:</div>
            <pre class="tool-result p-2 bg-base-300 rounded text-xs font-mono overflow-x-auto text-base-content/70 max-h-64 overflow-y-auto whitespace-pre-wrap">{{ message.content }}</pre>
          </div>
          <!-- Tool Call ID -->
          <div v-if="message.metadata?.tool_call_id" class="text-xs text-base-content/50">
            {{ t('agent.toolCallId') }}: <code class="font-mono">{{ message.metadata.tool_call_id }}</code>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, nextTick, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { save } from '@tauri-apps/plugin-dialog'
import { writeTextFile } from '@tauri-apps/plugin-fs'
import type { AgentMessage } from '@/types/agent'
import { getMessageTypeName } from '@/types/agent'
import MarkdownRenderer from './MarkdownRenderer.vue'
import ShellToolResult from './ShellToolResult.vue'

const { t } = useI18n()

const props = defineProps<{
  message: AgentMessage
  isWebExplorerActive?: boolean
  isExecuting?: boolean
}>()

const emit = defineEmits<{
  (e: 'resend', message: AgentMessage): void
  (e: 'edit', message: AgentMessage, newContent: string): void
  (e: 'heightChanged'): void
  (e: 'renderHtml', htmlContent: string): void
}>()

const isExpanded = ref(false)
const copySuccess = ref(false)

// Edit mode
const isEditing = ref(false)
const editedContent = ref('')
const editTextareaRef = ref<HTMLTextAreaElement | null>(null)

// Tool panel collapse states
const isToolPanelExpanded = ref(false)
const isArgsExpanded = ref(false)
const isResultExpanded = ref(false)
const argsHasOverflow = ref(false)
const resultHasOverflow = ref(false)
const argsBodyRef = ref<HTMLElement | null>(null)
const resultBodyRef = ref<HTMLElement | null>(null)

// Summary panel collapse state
const isSummaryPanelExpanded = ref(false)

const toggleDetails = () => {
  isExpanded.value = !isExpanded.value
}

const toggleToolPanel = () => {
  isToolPanelExpanded.value = !isToolPanelExpanded.value
  // Emit height change event after animation completes
  setTimeout(() => {
    emit('heightChanged')
  }, 250)
}

const toggleSummaryPanel = () => {
  isSummaryPanelExpanded.value = !isSummaryPanelExpanded.value
}

const toggleArgs = () => {
  isArgsExpanded.value = !isArgsExpanded.value
  // Emit height change event
  setTimeout(() => {
    emit('heightChanged')
  }, 50)
}

const toggleResult = () => {
  isResultExpanded.value = !isResultExpanded.value
  // Emit height change event
  setTimeout(() => {
    emit('heightChanged')
  }, 50)
}

// Check if content overflows
function checkArgsOverflow() {
  nextTick(() => {
    if (argsBodyRef.value) {
      argsHasOverflow.value = argsBodyRef.value.scrollHeight > argsBodyRef.value.clientHeight
    }
  })
}

function checkResultOverflow() {
  nextTick(() => {
    if (resultBodyRef.value) {
      resultHasOverflow.value = resultBodyRef.value.scrollHeight > resultBodyRef.value.clientHeight
    }
  })
}

// Check overflow on mount and when content changes
onMounted(() => {
  checkArgsOverflow()
  checkResultOverflow()
})

// Watch for content changes
watch(() => props.message.metadata?.tool_args, () => {
  checkArgsOverflow()
})

watch(() => props.message.metadata?.tool_result, () => {
  checkResultOverflow()
})

// 复制消息内容
const handleCopy = async () => {
  try {
    await navigator.clipboard.writeText(props.message.content)
    copySuccess.value = true
    setTimeout(() => {
      copySuccess.value = false
    }, 2000)
  } catch (err) {
    console.error('Failed to copy:', err)
  }
}

// 重新发送消息
const handleResend = () => {
  emit('resend', props.message)
}

// 编辑消息
const handleEdit = () => {
  isEditing.value = true
  editedContent.value = props.message.content
  // Focus textarea after mount
  nextTick(() => {
    if (editTextareaRef.value) {
      editTextareaRef.value.focus()
      // Auto-resize textarea
      autoResizeTextarea()
      // Select all text for easy editing
      editTextareaRef.value.select()
    }
  })
}

// 保存编辑
const handleSaveEdit = () => {
  const newContent = editedContent.value.trim()
  if (!newContent) return
  
  isEditing.value = false
  emit('edit', props.message, newContent)
}

// 取消编辑
const handleCancelEdit = () => {
  isEditing.value = false
  editedContent.value = ''
}

// Auto-resize textarea
const autoResizeTextarea = () => {
  nextTick(() => {
    const textarea = editTextareaRef.value
    if (textarea) {
      textarea.style.height = 'auto'
      textarea.style.height = Math.min(textarea.scrollHeight, 300) + 'px'
    }
  })
}

// Watch for edited content changes to auto-resize
watch(editedContent, () => {
  if (isEditing.value) {
    autoResizeTextarea()
  }
})

// Type name
const typeName = computed(() => getMessageTypeName(props.message.type))

// RAG信息
const ragInfo = computed(() => props.message.metadata?.rag_info)

// Tool name from metadata
const toolName = computed(() => props.message.metadata?.tool_name)

// Check if this is a shell tool
const isShellTool = computed(() => {
  const name = props.message.metadata?.tool_name?.toLowerCase()
  return name === 'shell' || name === 'bash' || name === 'cmd' || name === 'powershell'
})

// Check if this is a segment summary message (sliding window)
const isSegmentSummary = computed(() => {
  return props.message.type === 'system' && 
         props.message.metadata?.kind === 'segment_summary'
})

// Check if this is a global summary message (long-term memory)
const isGlobalSummary = computed(() => {
  return props.message.type === 'system' && 
         props.message.metadata?.kind === 'global_summary'
})

// Check if this is a Tenth Man Critique message
const isTenthManCritique = computed(() => {
  return props.message.type === 'system' && 
         (props.message.metadata?.kind === 'tenth_man_critique' ||
          props.message.metadata?.kind === 'tenth_man_intervention' ||
          props.message.metadata?.kind === 'tenth_man_warning')
})

// Format number with commas
const formatNumber = (num: number | undefined) => {
  if (num === undefined) return '0'
  return num.toLocaleString()
}

const isMarkdownTableSeparator = (line: string) => {
  const trimmed = line.trim()
  return /^\|?\s*:?-+:?\s*(\|\s*:?-+:?\s*)+\|?$/.test(trimmed)
}

const parseMarkdownTableRow = (line: string): string[] => {
  const trimmed = line.trim()
  const withoutEdges = trimmed.replace(/^\|/, '').replace(/\|$/, '')
  return withoutEdges.split('|').map(cell => cell.trim())
}

const extractMarkdownTableData = (content: string): string[][] => {
  const lines = content.split('\n')
  for (let i = 0; i < lines.length - 1; i += 1) {
    const headerLine = lines[i]
    const separatorLine = lines[i + 1]
    if (!headerLine.includes('|') || !isMarkdownTableSeparator(separatorLine)) continue

    const header = parseMarkdownTableRow(headerLine)
    const rows: string[][] = []
    let j = i + 2
    while (j < lines.length && lines[j].includes('|')) {
      rows.push(parseMarkdownTableRow(lines[j]))
      j += 1
    }
    return [header, ...rows].filter(row => row.length > 0)
  }
  return []
}

const extractHtmlTableData = (html: string): string[][] => {
  try {
    const parser = new DOMParser()
    const doc = parser.parseFromString(html, 'text/html')
    const table = doc.querySelector('table')
    if (!table) return []
    const rows = Array.from(table.querySelectorAll('tr'))
    return rows.map(row => {
      const cells = Array.from(row.querySelectorAll('th, td'))
      return cells.map(cell => (cell.textContent || '').trim())
    })
  } catch {
    return []
  }
}

const extractHtmlBlock = (content: string): string | null => {
  const match = content.match(/```html\s*([\s\S]*?)```/i)
  if (match && match[1]) return match[1].trim()
  return null
}

const extractTableDataFromContent = (content: string): string[][] => {
  if (!content) return []
  const htmlBlock = extractHtmlBlock(content)
  const htmlCandidate = htmlBlock || content
  const htmlTable = extractHtmlTableData(htmlCandidate)
  if (htmlTable.length > 0) return htmlTable
  return extractMarkdownTableData(content)
}

// Status icon
const statusIcon = computed(() => {
  if (props.message.type === 'tool_result') {
    return props.message.metadata?.success ? '✓' : '✗'
  }
  return null
})

// Status class for icon color
const statusClass = computed(() => {
  if (props.message.type === 'tool_result') {
    return props.message.metadata?.success ? 'text-success' : 'text-error'
  }
  return ''
})

// Tool status from metadata
const toolStatus = computed(() => props.message.metadata?.status)

// Tool status display class
const toolStatusClass = computed(() => {
  switch (toolStatus.value) {
    case 'running':
      return 'bg-warning/20 text-warning'
    case 'completed':
      return 'bg-success/20 text-success'
    case 'failed':
      return 'bg-error/20 text-error'
    case 'pending':
      return 'bg-base-300 text-base-content/60'
    default:
      return ''
  }
})

// Tool status display text
const toolStatusText = computed(() => {
  switch (toolStatus.value) {
    case 'running':
      return `⏳ ${t('agent.statusRunning')}`
    case 'completed':
      return `✓ ${t('agent.statusCompleted')}`
    case 'failed':
      return `✗ ${t('agent.statusFailed')}`
    case 'pending':
      return t('agent.statusPending')
    default:
      return ''
  }
})

// Duration
const duration = computed(() => {
  const ms = props.message.metadata?.duration_ms
  if (ms) {
    return `${(ms / 1000).toFixed(1)}s`
  }
  return null
})

// Whether to show header
const showHeader = computed(() => {
  return ['tool_result', 'progress'].includes(props.message.type)
})

// Tool panel styling - left border color (always orange/warning)
const toolPanelBorderClass = computed(() => {
  return 'border-l-warning' // Always use orange/warning color for tool calls
})

// Has tool args
const hasToolArgs = computed(() => {
  return props.message.metadata?.tool_args && 
    Object.keys(props.message.metadata.tool_args).length > 0
})

// Has tool result (合并显示的结果)
const hasToolResult = computed(() => {
  return !!props.message.metadata?.tool_result
})

// Check if tool_call message has any content to display
const hasToolCallContent = computed(() => {
  if (props.message.type !== 'tool_call') return false
  
  // Has content, args, result, or call_id
  return !!(
    props.message.content ||
    hasToolArgs.value ||
    hasToolResult.value ||
    props.message.metadata?.tool_call_id
  )
})

// Check if regular message has any content to display
const hasRegularMessageContent = computed(() => {
  // tool_call messages are handled separately
  if (props.message.type === 'tool_call') return false
  
  // For tool_result, check if has content or args
  if (props.message.type === 'tool_result') {
    return !!(props.message.content || hasToolArgs.value)
  }
  
  // For all other message types, check if content is not empty
  return !!props.message.content && props.message.content.trim().length > 0
})

// Formatted args
const formattedArgs = computed(() => {
  return JSON.stringify(props.message.metadata?.tool_args, null, 2)
})

// Formatted tool result
const formattedToolResult = computed(() => {
  const result = props.message.metadata?.tool_result
  if (typeof result === 'string') {
    return result
  }
  return JSON.stringify(result, null, 2)
})

// Type-specific class
const typeClass = computed(() => {
  switch (props.message.type) {
    case 'thinking':
      return 'type-thinking bg-info/10 border-l-[3px] border-info'
    case 'planning':
      return 'type-planning bg-primary/10 border-l-[3px] border-primary'
    case 'tool_call':
      return 'type-tool_call bg-base-200 border-l-[3px] border-warning'
    case 'tool_result':
      return 'type-tool_result bg-base-200 border-l-[3px] border-success'
    case 'progress':
      return 'type-progress bg-base-200 border-l-[3px] border-base-content/30'
    case 'error':
      return 'type-error bg-error/10 border-l-[3px] border-error'
    case 'final':
      return 'type-final bg-success/5 border-l-[3px] border-success'
    default:
      return 'bg-base-200'
  }
})

// Get document attachments from user message metadata
const documentAttachments = computed(() => {
  if (props.message.type !== 'user') return []
  return props.message.metadata?.document_attachments || []
})

// Get image attachments from user message metadata
const imageAttachments = computed(() => {
  if (props.message.type !== 'user') return []
  const attachments = props.message.metadata?.image_attachments
  if (!attachments) return []
  
  // Handle both array format and single object format
  if (Array.isArray(attachments)) {
    return attachments.map((att: any) => {
      // Handle MessageAttachment::Image format (with type discriminator)
      if (att.type === 'image') {
        return att
      }
      // Handle legacy format (has 'image' property)
      if (att.image) {
        return att.image
      }
      // Handle direct ImageAttachment format
      return att
    })
  }
  
  return []
})

// Get image preview URL from base64 data
const getImagePreviewUrl = (img: any): string => {
  try {
    // Handle new ImageAttachment structure
    if (img?.data) {
      const mediaTypeRaw: string | undefined = img.media_type
      const mime = toMimeType(mediaTypeRaw)
      
      // Handle base64 data
      if (img.data.type === 'base64' && img.data.data) {
        return `data:${mime};base64,${img.data.data}`
      }
      
      // Handle URL
      if (img.data.type === 'url' && img.data.url) {
        return img.data.url
      }
    }
    
    // Handle legacy format (direct base64 string in data field)
    const mediaTypeRaw: string | undefined = img?.media_type
    const mime = toMimeType(mediaTypeRaw)
    const dataField = img?.data
    const base64 = typeof dataField === 'string' ? dataField : dataField?.data
    if (!base64) return ''
    return `data:${mime};base64,${base64}`
  } catch (e) {
    console.error('[MessageBlock] Failed to construct image preview:', e, img)
    return ''
  }
}

// Convert media type enum to MIME type
const toMimeType = (mediaType?: string): string => {
  if (!mediaType) return 'image/jpeg'
  const t = mediaType.toLowerCase()
  if (t === 'jpeg' || t === 'jpg') return 'image/jpeg'
  if (t === 'png') return 'image/png'
  if (t === 'gif') return 'image/gif'
  if (t === 'webp') return 'image/webp'
  return t.startsWith('image/') ? t : `image/${t}`
}

// Get filename from image attachment
const getImageFilename = (img: any): string => {
  return img?.filename || 'attachment'
}

// Open image preview (simple implementation - can be enhanced)
const openImagePreview = (url: string) => {
  if (!url) return
  // Open in new window
  window.open(url, '_blank')
}

// Format document file size
const formatDocSize = (bytes: number): string => {
  if (!bytes || bytes === 0) return '0 B'
  if (bytes < 1024) return bytes + ' B'
  if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(1) + ' KB'
  return (bytes / (1024 * 1024)).toFixed(1) + ' MB'
}

const looksLikeHtmlDocument = (text: string) => {
  return /<!doctype\s+html|<html[\s>]|<head[\s>]|<body[\s>]/i.test(text)
}

const wrapHtmlAsCodeBlock = (text: string, cursor: string) => {
  if (text.includes('```') || !looksLikeHtmlDocument(text)) return text + cursor
  return `\`\`\`html\n${text}${cursor}\n\`\`\``
}

// Format content based on message type
const formattedContent = computed(() => {
  const { type, content, metadata } = props.message
  const cursor = props.isExecuting ? ' ▍' : ''

  switch (type) {
    case 'thinking':
      return `> **Thinking**\n>\n> ${content.replace(/\n/g, '\n> ')}${cursor}`
    
    case 'planning':
      return `**Planning**\n\n${content}${cursor}`
    
    case 'tool_result':
      // Wrap result in code block if not already markdown
      let result = content
      if (!content.includes('```') && !content.includes('#')) {
        result = `\`\`\`\n${content}\n\`\`\``
      }
      return result + cursor
    
    case 'progress': {
      const step = metadata?.step_index ?? 0
      const total = metadata?.total_steps ?? 0
      return `**Progress** Step ${step}/${total}\n\n${content}${cursor}`
    }
    
    case 'error':
      return `> **Error**\n>\n> ${content}`
    
    case 'final':
      return wrapHtmlAsCodeBlock(content, cursor)
    
    default:
      return wrapHtmlAsCodeBlock(content, cursor)
  }
})

// Check if content should be hidden (Web Explorer duplication)
const shouldHideContent = computed(() => {
  // Only apply if web explorer drawer is active
  if (!props.isWebExplorerActive) return false
  
  // Check if it is a web explorer tool message
  const toolName = props.message.metadata?.tool_name
  if (toolName === 'web_explorer' || toolName === 'vision_explorer') {
    // Hide tool_result and progress messages (which are usually verbose logs)
    return ['tool_result', 'progress'].includes(props.message.type)
  }
  
  // Also check if content looks like iteration logs
  if (['tool_result', 'final'].includes(props.message.type)) {
     if (props.message.content.includes('**迭代') && (props.message.content.includes('web_explorer') || props.message.content.includes('vision_explorer'))) {
       return true
     }
  }
  
  return false
})

const tableData = computed(() => extractTableDataFromContent(props.message.content || ''))

const showTableDownload = computed(() => {
  if (props.message.type === 'user') return false
  return tableData.value.length > 0
})

const escapeCsvCell = (value: string) => {
  if (value.includes('"')) {
    value = value.replace(/"/g, '""')
  }
  if (/[",\n\r]/.test(value)) {
    return `"${value}"`
  }
  return value
}

const buildCsvContent = (rows: string[][]) => {
  return rows
    .map(row => row.map(cell => escapeCsvCell(cell ?? '')).join(','))
    .join('\n')
}

const downloadTableAsCsv = async (data: string[][]) => {
  if (data.length === 0) return
  
  const csv = buildCsvContent(data)
  const defaultFilename = `table-${new Date().toISOString().replace(/[:.]/g, '-')}.csv`
  
  try {
    // Use Tauri save dialog
    const filePath = await save({
      defaultPath: defaultFilename,
      filters: [{ name: 'CSV', extensions: ['csv'] }]
    })
    
    if (filePath) {
      await writeTextFile(filePath, csv)
      console.log('[MessageBlock] Table saved to:', filePath)
    }
  } catch (e) {
    console.error('[MessageBlock] Failed to save table:', e)
    // Fallback to browser download
    try {
      const blob = new Blob([csv], { type: 'text/csv;charset=utf-8;' })
      const url = URL.createObjectURL(blob)
      const link = document.createElement('a')
      link.href = url
      link.download = defaultFilename
      document.body.appendChild(link)
      link.click()
      document.body.removeChild(link)
      URL.revokeObjectURL(url)
    } catch (fallbackError) {
      console.error('[MessageBlock] Fallback download also failed:', fallbackError)
    }
  }
}

// Extract all tables from content
const extractAllTablesFromContent = (content: string): string[][][] => {
  if (!content) return []
  const tables: string[][][] = []
  
  // Extract HTML tables
  const htmlBlock = extractHtmlBlock(content)
  const htmlCandidate = htmlBlock || content
  const htmlTable = extractHtmlTableData(htmlCandidate)
  if (htmlTable.length > 0) {
    tables.push(htmlTable)
  }
  
  // Extract Markdown tables
  const lines = content.split('\n')
  for (let i = 0; i < lines.length - 1; i += 1) {
    const headerLine = lines[i]
    const separatorLine = lines[i + 1]
    if (!headerLine.includes('|') || !isMarkdownTableSeparator(separatorLine)) continue

    const header = parseMarkdownTableRow(headerLine)
    const rows: string[][] = []
    let j = i + 2
    while (j < lines.length && lines[j].includes('|')) {
      rows.push(parseMarkdownTableRow(lines[j]))
      j += 1
    }
    const tableData = [header, ...rows].filter(row => row.length > 0)
    if (tableData.length > 0) {
      tables.push(tableData)
    }
    i = j - 1 // Skip processed lines
  }
  
  return tables
}

const allTables = computed(() => extractAllTablesFromContent(props.message.content || ''))

const handleDownloadTable = (tableIndex: number) => {
  const tables = allTables.value
  if (tableIndex >= 0 && tableIndex < tables.length) {
    downloadTableAsCsv(tables[tableIndex])
  } else if (tables.length > 0) {
    // Fallback to first table
    downloadTableAsCsv(tables[0])
  }
}
</script>

<style scoped>
.tool-args {
  word-break: break-word;
  white-space: pre-wrap;
}

.action-btn {
  min-height: 1.5rem;
  height: 1.5rem;
  padding: 0 0.5rem;
}

/* Image attachments styles */
.image-attachments {
  position: relative;
}

.image-attachment {
  position: relative;
  overflow: hidden;
}

.image-attachment img {
  display: block;
}

.action-btn i {
  font-size: 0.75rem;
}

/* Edit mode styles */
.edit-mode textarea {
  transition: border-color 0.2s;
  min-height: 4rem;
}

.edit-mode textarea:focus {
  box-shadow: 0 0 0 3px rgba(var(--p), 0.1);
}

/* Tool panel styles */
.tool-call-panel {
  transition: all 0.2s ease;
}

.tool-panel-header {
  user-select: none;
}

.tool-panel-header:active {
  transform: scale(0.99);
}

.tool-panel-content {
  animation: slideDown 0.2s ease-out;
}

@keyframes slideDown {
  from {
    opacity: 0;
    max-height: 0;
  }
  to {
    opacity: 1;
    max-height: 1000px;
  }
}

/* Scrollbar styles for tool args and result */
.tool-panel-content > div > div::-webkit-scrollbar {
  width: 8px;
}

.tool-panel-content > div > div::-webkit-scrollbar-track {
  background: transparent;
}

.tool-panel-content > div > div::-webkit-scrollbar-thumb {
  background: #424242;
  border-radius: 4px;
}

.tool-panel-content > div > div::-webkit-scrollbar-thumb:hover {
  background: #555;
}

</style>
