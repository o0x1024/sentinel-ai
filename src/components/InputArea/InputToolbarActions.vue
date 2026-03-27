<template>
  <div class="flex items-center gap-2 text-base-content/60 shrink-0">
    <button class="icon-btn" title="附件" @click="emit('trigger-file-select')">
      <i class="fas fa-paperclip"></i>
    </button>
    <button class="icon-btn" :class="{ active: toolsEnabled }" title="工具调用" @click="emit('toggle-tools')">
      <i class="fas fa-tools"></i>
    </button>
    <button v-if="toolsEnabled" class="icon-btn" title="工具配置" @click="emit('open-tool-config')">
      <i class="fas fa-cog"></i>
    </button>
    <button class="icon-btn" :class="{ active: teamEnabled }" title="Team 模式" @click="emit('toggle-team')">
      <i class="fas fa-users"></i>
    </button>
    <button
      class="icon-btn"
      :class="{ active: ragEnabled }"
      title="知识检索增强 - AI将使用 [SOURCE n] 格式引用知识库内容"
      @click="emit('toggle-rag')"
    >
      <i class="fas fa-brain"></i>
    </button>
    <button
      class="icon-btn"
      :class="{ active: webSearchEnabled }"
      title="联网搜索 - 向 LLM 暴露 web_search"
      @click="emit('toggle-web-search')"
    >
      <i class="fas fa-globe"></i>
    </button>
    <button class="icon-btn" title="@ 引用">
      <i class="fas fa-at"></i>
    </button>
    <button class="icon-btn" title="快速指令" @click="emit('open-slash-manager')">
      <i class="fas fa-bolt"></i>
    </button>
    <button class="icon-btn" title="选择">
      <i class="fas fa-border-all"></i>
    </button>
    <button class="icon-btn" title="清空会话" @click="emit('clear-conversation')">
      <i class="fas fa-eraser"></i>
    </button>
  </div>
</template>

<script setup lang="ts">
defineProps<{
  toolsEnabled: boolean
  teamEnabled: boolean
  ragEnabled: boolean
  webSearchEnabled: boolean
}>()

const emit = defineEmits<{
  (e: 'trigger-file-select'): void
  (e: 'toggle-tools'): void
  (e: 'open-tool-config'): void
  (e: 'toggle-team'): void
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
