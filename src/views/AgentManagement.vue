<template>
  <div class="page-content-padded safe-top">
    <div class="mb-3">
      <h2 class="text-2xl font-bold">智能体管理</h2>
      <p class="text-sm text-base-content/70 mt-2">
        集中管理交互助手 profile、Agent 运行环境与被动系统智能体的状态、配置和运行记录。
      </p>
    </div>

    <div class="tabs tabs-boxed mb-4 bg-base-200/60">
      <button
        v-for="tab in workspaceTabs"
        :key="tab.key"
        class="tab"
        :class="{ 'tab-active': activeWorkspaceTab === tab.key }"
        @click="activeWorkspaceTab = tab.key"
      >
        {{ tab.label }}
      </button>
    </div>

    <div class="mb-4 rounded-xl border border-base-300 bg-base-100 px-4 py-3 text-sm text-base-content/70">
      {{ activeWorkspaceTabDescription }}
    </div>

    <div>
      <AssistantProfileRegistryPanel v-if="activeWorkspaceTab === 'assistant-profiles'" />
      <AgentSettings v-if="activeWorkspaceTab === 'runtime-settings'" />
      <SystemAgentSettings v-if="activeWorkspaceTab === 'system-agents'" />
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'
import AgentSettings from '@/components/Settings/AgentSettings.vue'
import AssistantProfileRegistryPanel from '@/components/Settings/AssistantProfileRegistryPanel.vue'
import SystemAgentSettings from '@/components/Settings/SystemAgentSettings.vue'

defineOptions({
  name: 'AgentManagement',
})

type WorkspaceTabKey = 'assistant-profiles' | 'runtime-settings' | 'system-agents'

const activeWorkspaceTab = ref<WorkspaceTabKey>('assistant-profiles')

const workspaceTabs: Array<{
  key: WorkspaceTabKey
  label: string
  description: string
}> = [
  {
    key: 'assistant-profiles',
    label: '助手 Profiles',
    description: '管理交互助手的默认 profile、上下文模式、模型、工具策略和 Team preset。',
  },
  {
    key: 'runtime-settings',
    label: '运行环境',
    description: '管理 Agent 终端执行环境、工作目录、文件上传、图片附件、Subagent 超时和完成守卫。',
  },
  {
    key: 'system-agents',
    label: '系统智能体',
    description: '管理被动系统智能体的状态、配置、运行记录、发现与洞察。',
  },
]

const activeWorkspaceTabDescription = computed(() =>
  workspaceTabs.find(tab => tab.key === activeWorkspaceTab.value)?.description || '',
)
</script>
