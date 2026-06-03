<template>
  <div class="page-content-padded safe-top">
    <div class="mb-3">
      <h2 class="text-2xl font-bold">智能体管理</h2>
      <p class="text-sm text-base-content/70 mt-2">
        从同一个入口管理交互型 Agent、后台型 Agent，以及它们共享的运行环境。
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


    <div>
      <KeepAlive>
        <component :is="activeWorkspaceComponent" />
      </KeepAlive>
    </div>
  </div>
</template>

<script setup lang="ts">
import { KeepAlive, computed, ref } from 'vue'
import AgentSettings from '@/components/Settings/AgentSettings.vue'
import AgentRegistryWorkspace from '@/components/Settings/AgentRegistryWorkspace.vue'

defineOptions({
  name: 'AgentManagement',
})

type WorkspaceTabKey = 'agent-registry' | 'runtime-settings'

const activeWorkspaceTab = ref<WorkspaceTabKey>('agent-registry')

const workspaceTabs: Array<{
  key: WorkspaceTabKey
  label: string
  description: string
}> = [
  {
    key: 'agent-registry',
    label: 'Agent 管理',
    description: '统一管理交互型 Agent 与后台型 Agent，并按运行方式切换具体配置工作区。',
  },
  {
    key: 'runtime-settings',
    label: '运行环境',
    description: '管理 Agent 终端执行环境、工作目录、文件上传、图片附件、Subagent 超时和完成守卫。',
  },
]

const activeWorkspaceComponent = computed(() =>
  activeWorkspaceTab.value === 'agent-registry' ? AgentRegistryWorkspace : AgentSettings,
)

</script>
