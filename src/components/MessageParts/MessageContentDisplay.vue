<template>
  <div class="message-content">
    <!-- 简化的消息内容显示 - 统一使用 Markdown 渲染 -->
    <div 
      :class="[
        'prose prose-sm max-w-none leading-relaxed',
        message.role === 'user' ? 'prose-invert' : 'prose-neutral'
      ]"
      v-html="renderMarkdown(message.content)"
    />

    <!-- 流式指示器 -->
    <div v-if="isTyping" class="flex items-center gap-2 mt-2 text-base-content/70">
      <span class="loading loading-dots loading-sm text-primary"></span>
      <span class="text-sm">{{ t('aiAssistant.generating', 'AI正在思考...') }}</span>
    </div>
  </div>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { useMessageUtils } from '../../composables/useMessageUtils'

interface SimplifiedChatMessage {
  id: string
  role: 'user' | 'assistant'
  content: string
  isStreaming?: boolean
}

const props = defineProps<{
  message: SimplifiedChatMessage
  isTyping: boolean
  streamCharCount?: number
  streamSpeed?: number
}>()

const { t } = useI18n()
const { renderMarkdown } = useMessageUtils()
</script>