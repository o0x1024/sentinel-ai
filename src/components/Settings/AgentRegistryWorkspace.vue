<template>
  <div class="space-y-4">
    <div class="rounded-2xl border border-base-300 bg-base-100 p-2">


      <div class=" grid grid-cols-1 gap-3 xl:grid-cols-2">
        <button
          v-for="mode in registryModes"
          :key="`${mode.key}-card`"
          class="rounded-xl border px-4 py-4 text-left transition"
          :class="activeRegistryMode === mode.key
            ? 'border-primary bg-primary/10 shadow-sm'
            : 'border-base-300 bg-base-100 hover:border-primary/40 hover:bg-base-200/30'"
          @click="activeRegistryMode = mode.key"
        >
          <div class="flex items-center justify-between gap-3">
            <div>
              <div class="text-sm font-semibold text-base-content">{{ mode.label }}</div>
              <div class="mt-1 text-xs text-base-content/60">{{ mode.caption }}</div>
            </div>
            <span class="badge badge-ghost badge-sm">{{ mode.badge }}</span>
          </div>
          <p class="mt-3 text-sm leading-6 text-base-content/70">
            {{ mode.description }}
          </p>
        </button>
      </div>


    </div>

    <AssistantProfileRegistryPanel v-if="activeRegistryMode === 'interactive'" />
    <SystemAgentSettings v-else />
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'

import AssistantProfileRegistryPanel from './AssistantProfileRegistryPanel.vue'
import SystemAgentSettings from './SystemAgentSettings.vue'

defineOptions({
  name: 'AgentRegistryWorkspace',
})

type AgentRegistryMode = 'interactive' | 'background'

const activeRegistryMode = ref<AgentRegistryMode>('interactive')

const registryModes: Array<{
  key: AgentRegistryMode
  label: string
  caption: string
  badge: string
  description: string
}> = [
  {
    key: 'interactive',
    label: '交互型 Agent',
    caption: '面向人工发起的会话与协作入口',
    badge: 'Human-in-the-loop',
    description: '统一管理对话入口使用的 Agent、上下文模式、默认模型、工具策略和 Team preset。',
  },
  {
    key: 'background',
    label: '后台型 Agent',
    caption: '面向事件触发的自动执行体',
    badge: 'Event-driven',
    description: '统一管理后台 Agent 的事件绑定、运行约束、审计记录、发现结果和洞察。',
  },
]

const activeRegistryModeDescription = computed(() =>
  registryModes.find(mode => mode.key === activeRegistryMode.value)?.description || '',
)
</script>
