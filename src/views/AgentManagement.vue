<template>
  <div class="page-content-padded safe-top">
    <div class="mb-3">
      <h2 class="text-2xl font-bold">智能体管理</h2>
      <p class="text-sm text-base-content/70 mt-2">
        集中查看和管理被动系统智能体的状态、配置、运行记录与安全洞察。
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
