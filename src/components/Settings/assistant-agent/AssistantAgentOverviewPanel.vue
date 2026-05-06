<template>
  <div class="space-y-4">
    <div class="grid grid-cols-1 gap-4 md:grid-cols-2 2xl:grid-cols-4">
      <div
        v-for="card in summaryCards"
        :key="card.label"
        class="rounded-xl border border-base-300 bg-base-100 px-4 py-4"
      >
        <div class="text-xs uppercase tracking-wide text-base-content/50">{{ card.label }}</div>
        <div class="mt-2 text-lg font-semibold text-base-content">{{ card.value }}</div>
        <div class="mt-1 text-xs text-base-content/60">{{ card.hint }}</div>
      </div>
    </div>

    <div class="rounded-xl border border-base-300 bg-base-100 p-4">
      <div class="text-sm font-semibold">默认能力开关</div>
      <div class="mt-1 text-xs text-base-content/60">
        这些开关会决定交互型 Agent 在新会话里的默认能力基线。
      </div>
      <div class="mt-4 flex flex-wrap gap-2">
        <span
          v-for="item in capabilityBadges"
          :key="item.label"
          class="badge badge-sm"
          :class="item.enabled ? 'badge-primary' : 'badge-ghost'"
        >
          {{ item.label }} {{ item.enabled ? '开启' : '关闭' }}
        </span>
      </div>
    </div>

    <div class="grid grid-cols-1 gap-4 xl:grid-cols-2">
      <div class="rounded-xl border border-base-300 bg-base-100 p-4">
        <div class="text-sm font-semibold">Team 预设</div>
        <div class="mt-1 text-xs text-base-content/60">
          仅在 Team 运行模式下生效；未设置时由运行时策略自行决定。
        </div>
        <div class="mt-4 space-y-3">
          <div class="rounded-lg border border-base-300 bg-base-200/40 px-3 py-3">
            <div class="text-xs text-base-content/50">编排模板</div>
            <div class="mt-1 text-sm font-medium text-base-content">
              {{ profile.defaultTeamOrchestrationPresetId || '未设置' }}
            </div>
          </div>
          <div class="rounded-lg border border-base-300 bg-base-200/40 px-3 py-3">
            <div class="text-xs text-base-content/50">恢复策略</div>
            <div class="mt-1 text-sm font-medium text-base-content">
              {{ profile.defaultTeamRecoveryPresetId || '未设置' }}
            </div>
          </div>
        </div>
      </div>

      <div class="rounded-xl border border-base-300 bg-base-100 p-4">
        <div class="text-sm font-semibold">工具策略摘要</div>
        <div class="mt-1 text-xs text-base-content/60">
          先看默认工具边界，再决定是否进入配置区调整详细策略。
        </div>
        <div class="mt-4 flex flex-wrap gap-2">
          <span class="badge badge-sm" :class="toolConfig.enabled ? 'badge-primary' : 'badge-ghost'">
            {{ toolConfig.enabled ? '工具已启用' : '工具已关闭' }}
          </span>
          <span class="badge badge-outline badge-sm">{{ `策略 ${toolConfig.selection_strategy}` }}</span>
          <span class="badge badge-outline badge-sm">{{ `上限 ${toolConfig.max_tools}` }}</span>
          <span class="badge badge-outline badge-sm">{{ `预选 ${toolConfig.preselected_tools.length}` }}</span>
          <span class="badge badge-outline badge-sm">{{ `禁用 ${toolConfig.disabled_tools.length}` }}</span>
          <span
            v-if="toolConfig.manual_tools && toolConfig.manual_tools.length > 0"
            class="badge badge-outline badge-sm"
          >
            {{ `手动 ${toolConfig.manual_tools.length}` }}
          </span>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'

import type { AssistantProfileOption } from '@/components/Agent/assistantProfiles'
import type { UiToolConfigPayload } from '@/components/Agent/toolConfigRuntime'

const props = defineProps<{
  profile: AssistantProfileOption
  defaultModelLabel: string
  toolConfig: UiToolConfigPayload
}>()

const summaryCards = computed(() => [
  {
    label: '运行模式',
    value: props.profile.runMode,
    hint: props.profile.runMode === 'team' ? '进入团队编排入口' : '保持单助手会话模式',
  },
  {
    label: '上下文模式',
    value: props.profile.contextMode,
    hint: '决定提示词与上下文组织方式',
  },
  {
    label: '默认模型',
    value: props.defaultModelLabel,
    hint: '未覆盖时跟随 AI 全局默认',
  },
  {
    label: '工具策略',
    value: props.toolConfig.selection_strategy,
    hint: props.toolConfig.enabled ? '工具调用默认已开启' : '工具调用默认关闭',
  },
])

const capabilityBadges = computed(() => [
  { label: 'RAG', enabled: props.profile.defaultRagEnabled === true },
  { label: 'Web 搜索', enabled: props.profile.defaultWebSearchEnabled === true },
  { label: 'Tools', enabled: props.profile.defaultToolsEnabled === true },
  { label: '10th Man', enabled: props.profile.defaultTenthManEnabled === true },
])
</script>
