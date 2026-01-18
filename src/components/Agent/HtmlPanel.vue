<template>
  <div v-if="isActive" class="html-panel h-full flex flex-col bg-base-100">
    <div class="html-header flex items-center gap-2 px-4 py-3 border-b border-base-300">
      <i class="fas fa-code text-primary"></i>
      <span class="font-semibold text-base-content">{{ $t('agent.htmlPanel') }}</span>
      <button
        @click="$emit('close')"
        class="btn btn-ghost btn-sm btn-square ml-auto"
        :title="$t('common.close')"
      >
        <i class="fas fa-times"></i>
      </button>
    </div>

    <div v-if="rawHtml" class="html-content flex-1 overflow-hidden">
      <iframe
        class="html-iframe w-full h-full"
        :srcdoc="rawHtml"
        sandbox="allow-scripts allow-same-origin"
      ></iframe>
    </div>

    <div v-else class="flex-1 flex flex-col items-center justify-center text-base-content/60 p-8">
      <div class="avatar placeholder mb-4">
        <div class="bg-base-200 text-base-content/40 rounded-full w-16 flex items-center justify-center">
          <i class="fas fa-file-code text-2xl"></i>
        </div>
      </div>
      <h3 class="text-base font-semibold mb-2 text-base-content/80">{{ $t('agent.noHtmlContent') }}</h3>
      <p class="text-sm text-center max-w-xs text-base-content/60">{{ $t('agent.htmlPanelHint') }}</p>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'

const props = defineProps<{
  htmlContent: string
  isActive?: boolean
}>()

defineEmits<{
  close: []
}>()

// 直接使用原始 HTML，iframe sandbox 已提供隔离
const rawHtml = computed(() => (props.htmlContent || '').trim())
</script>

<style scoped>
.html-content :deep(table) {
  width: 100%;
}
.html-iframe {
  border: 0;
}
</style>
