<template>
  <div class="ai-team-assistant-view page-content-full h-full flex flex-col bg-base-100 overflow-hidden">
    <div class="flex-1 overflow-hidden min-h-0">
      <AgentTeamView class="h-full" />
    </div>
  </div>
</template>

<script setup lang="ts">
import AgentTeamView from '@/components/Agent/AgentTeamView.vue'

defineOptions({
  name: 'AITeamAssistant',
})
</script>

<style scoped>
.ai-team-assistant-view {
  font-family: 'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
}
</style>
