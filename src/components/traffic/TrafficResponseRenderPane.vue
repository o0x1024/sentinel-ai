<template>
  <div class="traffic-response-render-pane">
    <div v-if="imagePreviewSrc" class="traffic-response-render-image-shell">
      <img
        :src="imagePreviewSrc"
        :alt="previewAlt"
        class="traffic-response-render-image"
      >
    </div>
    <iframe
      v-else-if="body"
      :srcdoc="body"
      class="traffic-response-render-frame"
      sandbox="allow-scripts allow-same-origin allow-forms allow-popups allow-modals"
    ></iframe>
    <div v-else-if="isImageResponse" class="traffic-response-render-empty">
      图片响应为空，无法预览。
    </div>
    <div v-else class="traffic-response-render-empty">
      当前响应没有可渲染的内容。
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import {
  buildImagePreviewSrc,
  isImageResponseContentType,
  normalizeResponseContentType,
} from './trafficResponsePreviewSupport'

const props = defineProps<{
  body: string
  contentType?: string
}>()

const normalizedContentType = computed(() => normalizeResponseContentType(props.contentType || ''))
const isImageResponse = computed(() => isImageResponseContentType(normalizedContentType.value))
const imagePreviewSrc = computed(() => buildImagePreviewSrc(props.body || '', normalizedContentType.value))
const previewAlt = computed(() => normalizedContentType.value || 'response preview image')
</script>

<style scoped>
.traffic-response-render-pane {
  display: flex;
  height: 100%;
  width: 100%;
  min-height: 0;
  align-items: stretch;
  justify-content: stretch;
  background:
    linear-gradient(135deg, hsl(var(--b1)) 0%, hsl(var(--b2) / 0.8) 100%);
}

.traffic-response-render-image-shell {
  display: flex;
  flex: 1;
  min-height: 0;
  align-items: center;
  justify-content: center;
  overflow: auto;
  padding: 1rem;
  background-image:
    linear-gradient(45deg, hsl(var(--b3) / 0.22) 25%, transparent 25%),
    linear-gradient(-45deg, hsl(var(--b3) / 0.22) 25%, transparent 25%),
    linear-gradient(45deg, transparent 75%, hsl(var(--b3) / 0.22) 75%),
    linear-gradient(-45deg, transparent 75%, hsl(var(--b3) / 0.22) 75%);
  background-position: 0 0, 0 10px, 10px -10px, -10px 0;
  background-size: 20px 20px;
}

.traffic-response-render-image {
  max-width: 100%;
  max-height: 100%;
  object-fit: contain;
  box-shadow: 0 18px 42px rgb(15 23 42 / 0.18);
  background: white;
}

.traffic-response-render-frame {
  height: 100%;
  width: 100%;
  border: 0;
  background: white;
}

.traffic-response-render-empty {
  display: flex;
  flex: 1;
  align-items: center;
  justify-content: center;
  padding: 1rem;
  color: hsl(var(--bc) / 0.58);
  font-size: 0.875rem;
  text-align: center;
}
</style>
