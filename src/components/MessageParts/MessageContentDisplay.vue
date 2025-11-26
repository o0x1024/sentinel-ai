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

    <!-- 图片附件预览：仅用于结构验证和简单展示 -->
    <div
      v-if="imageAttachments.length"
      class="mt-2 flex flex-wrap gap-2"
    >
      <img
        v-for="(att, idx) in imageAttachments"
        :key="idx"
        class="max-h-32 rounded border border-base-300 shadow-sm bg-base-100 object-contain"
        :src="toImageSrc(att)"
        :alt="att.image?.filename || 'attachment'"
      />
    </div>

    <!-- 流式指示器 -->
    <div v-if="isTyping" class="flex items-center gap-2 mt-2 text-base-content/70">
      <span class="loading loading-dots loading-sm text-primary"></span>
      <span class="text-sm">{{ t('aiAssistant.generating', 'AI正在思考...') }}</span>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { useMessageUtils } from '../../composables/useMessageUtils'

interface SimplifiedChatMessage {
  id: string
  role: 'user' | 'assistant'
  content: string
  isStreaming?: boolean
  // 可选的附件数组（运行时通过any访问）
  attachments?: any[]
}

const props = defineProps<{
  message: SimplifiedChatMessage
  isTyping: boolean
  streamCharCount?: number
  streamSpeed?: number
}>()

const { t } = useI18n()
const { renderMarkdown } = useMessageUtils()

// 计算当前消息中的图片附件（仅做简单过滤）
const imageAttachments = computed(() => {
  const raw = (props.message as any).attachments as any[] | undefined
  if (!raw || !Array.isArray(raw)) return []
  return raw.filter((att) => att && (att.image || att.data))
})

// 将后端返回的图片附件结构转换为data URL
const toImageSrc = (att: any): string => {
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
    console.error('[MessageContentDisplay] 构造图片src失败:', e, att)
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
</script>