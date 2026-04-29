<template>
  <div class="flex items-center gap-2 text-base-content/60 shrink-0">
    <button class="icon-btn" title="附件" @click="emit('trigger-file-select')">
      <i class="fas fa-paperclip"></i>
    </button>
    <button class="icon-btn" title="Agent 配置" @click="emit('open-tool-config')">
      <i class="fas fa-sliders-h"></i>
    </button>
    <button
      class="icon-btn"
      :class="{ active: ragEnabled }"
      title="知识检索增强"
      @click="emit('toggle-rag')"
    >
      <i class="fas fa-brain"></i>
    </button>
    <button
      class="icon-btn"
      :class="{ active: webSearchEnabled }"
      title="联网搜索"
      @click="emit('toggle-web-search')"
    >
      <i class="fas fa-globe"></i>
    </button>
    <button class="icon-btn" title="快速指令" @click="emit('open-slash-manager')">
      <i class="fas fa-bolt"></i>
    </button>
    <button class="icon-btn" title="清空会话" @click="emit('clear-conversation')">
      <i class="fas fa-eraser"></i>
    </button>
  </div>
</template>

<script setup lang="ts">
defineProps<{
  ragEnabled: boolean
  webSearchEnabled: boolean
}>()

const emit = defineEmits<{
  (e: 'trigger-file-select'): void
  (e: 'open-tool-config'): void
  (e: 'toggle-rag'): void
  (e: 'toggle-web-search'): void
  (e: 'open-slash-manager'): void
  (e: 'clear-conversation'): void
}>()
</script>

<style scoped>
.icon-btn {
  width: 1.75rem;
  height: 1.75rem;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 0.375rem;
  font-size: calc(var(--font-size-base, 14px) * 0.75);
  transition: background-color 0.15s, color 0.15s;
}

.icon-btn:hover {
  background-color: hsl(var(--b3) / 0.7);
}

.icon-btn.active {
  background: hsl(var(--p));
  color: hsl(var(--pc));
  box-shadow: 0 2px 4px rgba(0, 0, 0, 0.15);
}
</style>
