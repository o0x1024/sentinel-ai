<template>
  <div>
    <!-- Non-streaming or completed streaming content -->
    <div v-if="!message.isStreaming && message.content" 
         :class="[
           'prose prose-sm max-w-none leading-relaxed cursor-pointer',
           message.role === 'user' ? 'prose-invert' : 'prose-neutral'
         ]" 
         v-html="renderMarkdown(message.content)">
    </div>
    
    <!-- Typewriter effect content - streaming with active typewriter -->
    <div v-else-if="message.isStreaming && message.role === 'assistant' && (isTyping || displayedContent)" 
         :class="[
           'prose prose-sm max-w-none leading-relaxed cursor-pointer typewriter-text',
           'streaming-content'
         ]" 
         :title="'点击跳过打字机效果'"
         @click="$emit('skip-typewriter')"
         v-html="renderMarkdown(displayedContent)">
    </div>
         
    <!-- Fallback display for content without proper streaming state -->
    <div v-else-if="message.content || displayedContent" 
         :class="[
           'prose prose-sm max-w-none leading-relaxed',
           message.role === 'user' ? 'prose-invert' : 'prose-neutral',
           'border border-warning/30 bg-warning/5 p-2 rounded'
         ]" 
         :title="'备用显示模式'"
         v-html="renderMarkdown(message.content || displayedContent)">
    </div>
    
    <!-- Streaming indicator when no content is available yet -->
    <div v-else-if="message.isStreaming && message.role === 'assistant'" 
         class="flex items-center gap-3 p-3 text-base-content/70">
      <div class="flex items-center gap-2">
        <span class="loading loading-dots loading-sm text-primary"></span>
        <div class="flex flex-col gap-1">
          <span class="text-sm font-medium">
            {{ isTyping ? t('aiAssistant.typing', 'AI正在输入...') : t('aiAssistant.generating', 'AI正在思考...') }}
          </span>
          <div class="flex items-center gap-4 text-xs text-base-content/50">
            <span v-if="streamCharCount > 0" class="flex items-center gap-1">
              <svg class="w-3 h-3" fill="currentColor" viewBox="0 0 20 20">
                <path d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z"/>
              </svg>
              {{ streamCharCount }} 字符
            </span>
            <span v-if="streamSpeed > 0" class="flex items-center gap-1">
              <svg class="w-3 h-3" fill="currentColor" viewBox="0 0 20 20">
                <path d="M13 6a3 3 0 11-6 0 3 3 0 016 0zM18 8a2 2 0 11-4 0 2 2 0 014 0zM14 15a4 4 0 00-8 0v3h8v-3z"/>
              </svg>
              {{ streamSpeed }} 字符/秒
            </span>
            <span v-if="isTyping" class="flex items-center gap-1 text-primary">
              <svg class="w-3 h-3" fill="currentColor" viewBox="0 0 20 20">
                <path d="M4 4a2 2 0 00-2 2v8a2 2 0 002 2h12a2 2 0 002-2V6a2 2 0 00-2-2H4zm0 2h12v8H4V6z"/>
              </svg>
              打字机模式
            </span>
          </div>
        </div>
      </div>
    </div>
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
}

const props = defineProps<{
  message: ChatMessage
  displayedContent: string
  isTyping: boolean
  streamCharCount?: number
  streamSpeed?: number
}>()

defineEmits(['skip-typewriter'])

const { t } = useI18n()
const { renderMarkdown } = useMessageUtils()
</script>