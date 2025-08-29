<template>
  <div>
    <!-- 消息内容显示 -->
    <div class="message-content">
      <!-- Non-streaming or completed streaming content -->
      <div v-if="message.content"
           :class="[
             'prose prose-sm max-w-none leading-relaxed',
             message.role === 'user' ? 'prose-invert' : 'prose-neutral'
           ]"
           v-html="renderMarkdown(message.content)">
      </div>

      <!-- Streaming indicator when no content is available yet -->
      <div v-else-if="message.isStreaming && message.role === 'assistant'"
           class="flex items-center gap-3 p-3 text-base-content/70">
        <div class="flex items-center gap-2">
          <span class="loading loading-dots loading-sm text-primary"></span>
          <div class="flex flex-col gap-1">
            <span class="text-sm font-medium">
              {{ t('aiAssistant.generating', 'AI正在思考...') }}
            </span>
            
          </div>
        </div>
      </div>
    </div>

    <!-- 工具执行显示 -->
    <div v-if="message.toolExecutions && message.toolExecutions.length > 0" class="tool-executions mt-4">
      <div class="collapse collapse-arrow bg-base-200 rounded-lg">
        <input type="checkbox" class="peer" /> 
        <div class="collapse-title text-sm font-medium flex items-center">
          <svg class="w-4 h-4 mr-2 text-primary" fill="currentColor" viewBox="0 0 20 20">
            <path fill-rule="evenodd" d="M5 2a1 1 0 011-1h8a1 1 0 011 1v10a1 1 0 01-1 1H6a1 1 0 01-1-1V2zm2 1h6v8H7V3zm3 10a1 1 0 100 2 1 1 0 000-2z" clip-rule="evenodd"/>
          </svg>
          工具执行 ({{ message.toolExecutions.length }})
        </div>
        <div class="collapse-content">
          <div v-for="(execution, index) in message.toolExecutions" :key="index" class="mb-2 p-2 rounded bg-base-300">
            <div class="font-medium text-sm mb-1">{{ execution.tool || '未命名工具' }}</div>
            <div class="text-xs bg-base-100 p-2 rounded overflow-x-auto">
              <pre class="whitespace-pre-wrap">{{ JSON.stringify(execution.result || {}, null, 2) }}</pre>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- 执行计划和结果作为普通消息的一部分，不单独显示 -->
  </div>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { useMessageUtils } from '../../composables/useMessageUtils'

interface ChatMessage {
  id: string
  role: 'user' | 'assistant'
  content: string
  isStreaming?: boolean
  executionPlan?: any
  toolExecutions?: any[]
  executionResult?: any
  executionProgress?: number
  currentStep?: string
}

const props = defineProps<{
  message: ChatMessage
  isTyping: boolean
  streamCharCount?: number
  streamSpeed?: number
}>()

const { t } = useI18n()
const { renderMarkdown } = useMessageUtils()
</script>