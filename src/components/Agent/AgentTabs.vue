<template>
  <div class="agent-tabs flex items-center gap-1 overflow-x-auto no-scrollbar" role="tablist" :aria-label="t('agent.conversationList')">
    <div 
      v-for="session in sessions" 
      :key="session.id"
      class="tab-item flex items-center gap-1 rounded-t-lg transition-all duration-200 group min-w-[120px] max-w-[220px]"
      :class="activeSessionId === session.id ? 'bg-base-100 text-primary shadow-sm' : 'hover:bg-base-200/50 text-base-content/60 focus-within:bg-base-200/50'"
    >
      <button
        type="button"
        role="tab"
        :aria-selected="activeSessionId === session.id ? 'true' : 'false'"
        class="flex min-w-0 flex-1 items-center gap-2 px-4 py-2 text-left"
        @click="setActiveSession(session.id)"
      >
        <i class="fas fa-comment-alt text-xs" :class="activeSessionId === session.id ? 'text-primary' : 'text-base-content/40'"></i>
        <span class="text-sm truncate flex-1">{{ session.title }}</span>
      </button>
      <button 
        type="button"
        class="mr-2 rounded-full p-0.5 opacity-0 transition-opacity hover:bg-base-300 hover:text-error group-hover:opacity-100 group-focus-within:opacity-100 focus-visible:opacity-100"
        :title="t('agent.close')"
        @click.stop="removeSession(session.id)"
      >
        <i class="fas fa-times text-[10px]"></i>
      </button>
    </div>
    
    <button 
      type="button"
      class="btn btn-ghost btn-sm btn-circle ml-1 hover:bg-base-300"
      @click="$emit('new-tab')"
      :title="t('agent.newTab')"
    >
      <i class="fas fa-plus text-xs"></i>
    </button>
  </div>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { useAgentSessionManager } from '@/composables/useAgentSessionManager'

const { t } = useI18n()
const { sessions, activeSessionId, setActiveSession, removeSession } = useAgentSessionManager()

defineEmits<{
  (e: 'new-tab'): void
}>()
</script>

<style scoped>
.no-scrollbar::-webkit-scrollbar {
  display: none;
}
.no-scrollbar {
  -ms-overflow-style: none;
  scrollbar-width: none;
}
.agent-tabs {
  scrollbar-width: none;
}
.tab-item {
  border: 1px solid transparent;
  border-bottom: none;
}
</style>
