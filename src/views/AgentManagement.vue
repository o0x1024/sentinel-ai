<template>
  <div class="page-content-padded safe-top">
    <div class="mb-3">
      <h2 class="text-2xl font-bold">Agent 管理</h2>
      <p class="text-sm text-base-content/70 mt-2">
        管理系统中的主动类和被动类 Agent，支持配置、手动触发和查看运行记录。
      </p>
    </div>

    <SystemAgentSettings />
  </div>
</template>

<script setup lang="ts">
import SystemAgentSettings from '@/components/Settings/SystemAgentSettings.vue'

defineOptions({
  name: 'AgentManagement',
})
</script>
