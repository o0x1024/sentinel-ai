<template>
  <div class="card bg-transparent shadow-none">
    <div class="card-body gap-4 px-0 py-0">
      <div class="flex flex-wrap items-center gap-2">
        <div class="flex items-center gap-2">
          <button class="btn btn-sm btn-ghost" :disabled="loading" @click="reloadProfiles">
            刷新
          </button>
          <button class="btn btn-sm btn-primary" :disabled="!canSave" @click="saveProfiles">
            {{ isSavingAssistantProfiles ? '保存中...' : '保存' }}
          </button>
        </div>
      </div>

      <div v-if="loading" class="flex items-center justify-center py-10">
        <span class="loading loading-spinner loading-lg" />
      </div>

      <div v-else class="grid grid-cols-1 xl:grid-cols-[320px_minmax(0,1fr)] gap-4">
        <AgentListPanel
          title="交互型 Agent 列表"
          empty-text="当前还没有可用交互型 Agent。"
          :loading="loading"
          :items="assistantListItems"
          :filter-options="ASSISTANT_AGENT_LIST_FILTER_OPTIONS"
          :selected-id="selectedProfileId"
          @select="selectedProfileId = $event"
        >
          <template #actions>
            <button class="btn btn-xs btn-outline" @click="createProfile">新增 Agent</button>
          </template>
        </AgentListPanel>

        <SystemAgentDetailLayout :has-selection="!!selectedProfile">
          <template #empty>
            <div class="py-12 text-center text-base-content/60">
              请选择一个交互型 Agent。
            </div>
          </template>

          <template v-if="selectedProfile">
            <div class="space-y-4">
              <AgentIdentityPanel
                eyebrow="交互型 Agent"
                :title="selectedProfile.label || selectedProfile.id"
                :description="selectedProfile.description"
                :meta-items="selectedProfileMetaItems"
              >
                <template #badges>
                  <span
                    v-if="defaultAssistantProfileId === selectedProfile.id"
                    class="badge badge-primary badge-sm"
                  >
                    默认入口
                  </span>
                </template>

                <template #actions>
                  <div class="flex flex-wrap items-center justify-end gap-2">
                    <button
                      class="btn btn-sm btn-outline"
                      :disabled="!selectedProfile || isSavingDefaultAssistantProfile"
                      @click="saveSelectedAsDefault"
                    >
                      {{ defaultAssistantProfileId === selectedProfile.id ? '当前默认 Agent' : '设为默认 Agent' }}
                    </button>
                    <button
                      class="btn btn-sm btn-error btn-outline"
                      :disabled="draftProfiles.length <= 1"
                      @click="removeSelectedProfile"
                    >
                      删除
                    </button>
                  </div>
                </template>
              </AgentIdentityPanel>

            <AgentWorkspaceTabs v-model="activeWorkspaceTab" :items="workspaceTabs" />

            <div class="rounded-lg border border-base-300 bg-base-100 px-4 py-3 text-sm text-base-content/70">
              {{ activeWorkspaceTabDescription }}
            </div>

            <AssistantAgentOverviewPanel
              v-if="activeWorkspaceTab === 'overview'"
              :profile="selectedProfile"
              :default-model-label="selectedProfileResolvedModelLabel"
              :tool-config="selectedProfileToolConfig"
            />

            <div v-else class="space-y-4">
              <label class="form-control">
                <span class="label-text mb-2">显示名称</span>
                <input v-model.trim="selectedProfile.label" class="input input-bordered" type="text" />
              </label>

              <label class="form-control">
                <span class="label-text mb-2">描述</span>
                <textarea
                  v-model.trim="selectedProfile.description"
                  class="textarea textarea-bordered min-h-[110px]"
                />
              </label>

              <AgentModelPanel
                title="默认模型"
                description="配置交互型 Agent 默认使用的 provider/model；不设置时跟随 AI 全局默认。"
                provider-label="默认提供商"
                model-label="默认模型"
                :provider-value="selectedProfileDefaultProvider"
                :model-value="selectedProfileDefaultModel"
                :provider-options="aiProviderOptions"
                :model-options="selectedProfileModelOptions"
                :global-default-label="aiDefaultModelLabel"
                :datalist-id="selectedProfileModelDatalistId"
                follow-default-option-label="跟随 AI 默认配置"
                follow-badge-label="跟随默认"
                custom-badge-label="已覆盖"
                suggested-hint="已加载该提供商的建议模型，也可手动输入模型 ID。"
                @update:provider-value="updateSelectedProfileDefaultProvider"
                @update:model-value="updateSelectedProfileDefaultModel"
              />

              <div class="grid grid-cols-1 md:grid-cols-2 gap-3 rounded-xl border border-base-300 bg-base-200/30 p-4">
                <label class="flex items-center justify-between gap-3">
                  <span class="text-sm font-medium">默认启用 RAG</span>
                  <input v-model="selectedProfile.defaultRagEnabled" type="checkbox" class="toggle toggle-sm" />
                </label>
                <label class="flex items-center justify-between gap-3">
                  <span class="text-sm font-medium">默认启用 Web 搜索</span>
                  <input v-model="selectedProfile.defaultWebSearchEnabled" type="checkbox" class="toggle toggle-sm" />
                </label>
                <label class="flex items-center justify-between gap-3">
                  <span class="text-sm font-medium">默认启用 Tools</span>
                  <input v-model="selectedProfile.defaultToolsEnabled" type="checkbox" class="toggle toggle-sm" />
                </label>
                <label class="flex items-center justify-between gap-3">
                  <span class="text-sm font-medium">默认启用 10th Man</span>
                  <input v-model="selectedProfile.defaultTenthManEnabled" type="checkbox" class="toggle toggle-sm" />
                </label>
              </div>

              <AgentToolPolicyPanel
                title="默认工具策略"
                description="配置交互型 Agent 默认是否启用工具、如何选工具，以及可固定或禁用的工具范围。"
              >
                <template #summary>
                  <span class="badge badge-sm" :class="selectedProfileToolConfig.enabled ? 'badge-primary' : 'badge-ghost'">
                    {{ selectedProfileToolConfig.enabled ? '工具已启用' : '工具已关闭' }}
                  </span>
                  <span class="badge badge-outline badge-sm">
                    {{ `策略 ${selectedProfileToolConfig.selection_strategy}` }}
                  </span>
                  <span class="badge badge-outline badge-sm">
                    {{ `上限 ${selectedProfileToolConfig.max_tools}` }}
                  </span>
                  <span class="badge badge-outline badge-sm">
                    {{ `固定 ${selectedProfileToolConfig.fixed_tools.length}` }}
                  </span>
                  <span class="badge badge-outline badge-sm">
                    {{ `禁用 ${selectedProfileToolConfig.disabled_tools.length}` }}
                  </span>
                </template>

                <div class="collapse collapse-arrow rounded-xl border border-base-300 bg-base-100">
                  <input type="checkbox" />
                  <div class="collapse-title font-semibold">展开默认工具配置</div>
                  <div class="collapse-content">
                    <ToolConfigPanel
                      :config="selectedProfileToolConfig"
                      :show-header="false"
                      :show-footer="false"
                      @update:config="updateSelectedProfileToolConfig"
                    />
                  </div>
                </div>
              </AgentToolPolicyPanel>

              <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
                <label class="form-control">
                  <span class="label-text mb-2">上下文模式</span>
                  <select v-model="selectedProfile.contextMode" class="select select-bordered">
                    <option value="claude-like">claude-like</option>
                    <option value="codex-like">codex-like</option>
                  </select>
                </label>

                <label class="form-control">
                  <span class="label-text mb-2">运行模式</span>
                  <select v-model="selectedProfile.runMode" class="select select-bordered">
                    <option value="assistant">assistant</option>
                    <option value="team">team</option>
                  </select>
                </label>
              </div>

              <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
                <label class="form-control">
                  <span class="label-text mb-2">Team 编排模板（可选）</span>
                  <select
                    v-model="selectedProfile.defaultTeamOrchestrationPresetId"
                    class="select select-bordered"
                  >
                    <option :value="null">不设置</option>
                    <option
                      v-for="preset in teamOrchestrationPresetOptions"
                      :key="preset.id"
                      :value="preset.id"
                    >
                      {{ preset.label }}
                    </option>
                  </select>
                </label>

                <label class="form-control">
                  <span class="label-text mb-2">Team 恢复策略（可选）</span>
                  <select
                    v-model="selectedProfile.defaultTeamRecoveryPresetId"
                    class="select select-bordered"
                  >
                    <option :value="null">不设置</option>
                    <option
                      v-for="preset in teamRecoveryPresetOptions"
                      :key="preset.id"
                      :value="preset.id"
                    >
                      {{ preset.label }}
                    </option>
                  </select>
                </label>
              </div>
            </div>
            </div>
          </template>
        </SystemAgentDetailLayout>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core'
import { computed, onMounted, ref, watch } from 'vue'
import type { AssistantProfileOption } from '@/components/Agent/assistantProfiles'
import { useAssistantProfiles } from '@/components/Agent/assistantProfiles'
import ToolConfigPanel from '@/components/Agent/ToolConfigPanel.vue'
import AgentIdentityPanel from '@/components/Settings/AgentIdentityPanel.vue'
import AgentListPanel from '@/components/Settings/AgentListPanel.vue'
import AgentModelPanel from '@/components/Settings/AgentModelPanel.vue'
import AgentToolPolicyPanel from '@/components/Settings/AgentToolPolicyPanel.vue'
import AgentWorkspaceTabs from '@/components/Settings/AgentWorkspaceTabs.vue'
import type { AgentListItemViewModel } from '@/components/Settings/agentListItemSupport'
import type { UiToolConfigPayload } from '@/components/Agent/toolConfigRuntime'
import AssistantAgentOverviewPanel from '@/components/Settings/assistant-agent/AssistantAgentOverviewPanel.vue'
import SystemAgentDetailLayout from '@/components/Settings/system-agent/SystemAgentDetailLayout.vue'
import {
  ASSISTANT_AGENT_LIST_FILTER_OPTIONS,
  getAssistantAgentModelBadge,
  getAssistantAgentRunModeBadge,
  getAssistantAgentSecondarySummary,
  getAssistantAgentToolsBadge,
} from '@/components/Settings/assistant-agent/assistantAgentListSupport'
import {
  applyToolConfigToProfile,
  createNextProfileIdentity,
  normalizeAssistantProfileDraft,
  profileToToolConfig,
  teamOrchestrationPresetOptions,
  teamRecoveryPresetOptions,
} from '@/components/Settings/assistantProfileRegistrySupport'

const {
  defaultAssistantProfileId,
  isLoadingDefaultAssistantProfile,
  isLoadingAssistantProfiles,
  isSavingDefaultAssistantProfile,
  isSavingAssistantProfiles,
  loadDefaultAssistantProfile,
  loadAssistantProfiles,
  profileOptions,
  saveAssistantProfiles,
  saveDefaultAssistantProfile,
} = useAssistantProfiles()

type WorkspaceTabKey = 'overview' | 'config'

const draftProfiles = ref<AssistantProfileOption[]>([])
const selectedProfileId = ref('')
const activeWorkspaceTab = ref<WorkspaceTabKey>('overview')
const aiConfig = ref<any | null>(null)
const selectedProfileModelDatalistId = 'assistant-profile-default-model-options'

const normalizeProviderName = (provider: string) => {
  const lower = provider.toLowerCase()
  const names: Record<string, string> = {
    anthropic: 'Anthropic',
    openai: 'OpenAI',
    gemini: 'Gemini',
    deepseek: 'DeepSeek',
    ollama: 'Ollama',
    openrouter: 'OpenRouter',
    xai: 'xAI',
  }
  return names[lower] || provider
}

const extractModelId = (item: any) => {
  if (!item) return ''
  if (typeof item === 'string') return item
  if (typeof item.id === 'string') return item.id
  if (typeof item.name === 'string') return item.name
  return ''
}

const getProviderConfigByKey = (providerKey: string) => {
  const providers = aiConfig.value?.providers && typeof aiConfig.value.providers === 'object'
    ? aiConfig.value.providers
    : {}
  const matchedKey = Object.keys(providers).find(key => key.toLowerCase() === providerKey.toLowerCase())
  return matchedKey ? providers[matchedKey] : null
}

const aiProviderOptions = computed(() => {
  const providers = aiConfig.value?.providers && typeof aiConfig.value.providers === 'object'
    ? aiConfig.value.providers
    : {}
  return Object.entries(providers)
    .filter(([, providerValue]) => (providerValue as any)?.enabled !== false)
    .map(([providerKey, providerValue]) => {
      const providerRaw = String((providerValue as any)?.provider || providerKey).trim()
      const value = providerRaw.toLowerCase()
      return {
        value,
        label: normalizeProviderName(providerRaw),
      }
    })
    .sort((a, b) => a.label.localeCompare(b.label))
})

const aiDefaultModelLabel = computed(() => {
  const defaultModel = String(aiConfig.value?.default_llm_model || '').trim()
  return defaultModel || '未设置，跟随 AI 全局默认'
})

const selectedProfileDefaultProvider = computed(() => {
  const defaultModel = selectedProfile.value?.defaultModel?.trim() || ''
  if (!defaultModel.includes('/')) return ''
  return defaultModel.slice(0, defaultModel.indexOf('/')).toLowerCase()
})

const selectedProfileDefaultModel = computed(() => {
  const defaultModel = selectedProfile.value?.defaultModel?.trim() || ''
  if (!defaultModel.includes('/')) return defaultModel
  return defaultModel.slice(defaultModel.indexOf('/') + 1)
})

const selectedProfileModelOptions = computed(() => {
  const providerConfig = getProviderConfigByKey(selectedProfileDefaultProvider.value)
  const models = Array.isArray(providerConfig?.models) ? providerConfig.models : []
  const out = models.map((model: any) => {
    const value = extractModelId(model)
    return {
      value,
      label: typeof model?.name === 'string' ? model.name : value,
    }
  }).filter((model: { value: string }) => model.value)
  const providerDefaultModel = String(providerConfig?.default_model || '').trim()
  if (providerDefaultModel && !out.some((model: { value: string }) => model.value === providerDefaultModel)) {
    out.unshift({
      value: providerDefaultModel,
      label: providerDefaultModel,
    })
  }
  return out
})

const updateSelectedProfileDefaultProvider = (value: string) => {
  if (!selectedProfile.value) return
  const provider = value.trim()
  if (!provider) {
    selectedProfile.value.defaultModel = null
    return
  }
  const providerConfig = getProviderConfigByKey(provider)
  const providerDefaultModel = String(providerConfig?.default_model || '').trim()
  const firstModel = Array.isArray(providerConfig?.models)
    ? providerConfig.models.map(extractModelId).find((modelId: string) => !!modelId) || ''
    : ''
  const model = providerDefaultModel || firstModel
  selectedProfile.value.defaultModel = model ? `${provider}/${model}` : null
}

const updateSelectedProfileDefaultModel = (value: string) => {
  if (!selectedProfile.value || !selectedProfileDefaultProvider.value) return
  const model = value.trim()
  selectedProfile.value.defaultModel = model ? `${selectedProfileDefaultProvider.value}/${model}` : null
}

const selectedProfileToolConfig = computed(() =>
  selectedProfile.value ? profileToToolConfig(selectedProfile.value) : {
    enabled: false,
    selection_strategy: 'Keyword',
    max_tools: 5,
    fixed_tools: ['interactive_shell'],
    disabled_tools: [],
    manual_tools: [],
  }
)
const selectedProfileResolvedModelLabel = computed(() =>
  selectedProfile.value?.defaultModel?.trim() || aiDefaultModelLabel.value
)
const workspaceTabs: Array<{
  key: WorkspaceTabKey
  label: string
  description: string
}> = [
  {
    key: 'overview',
    label: '概览',
    description: '先看默认能力、工具策略和 Team 预设，再决定是否进入配置区修改。',
  },
  {
    key: 'config',
    label: '配置',
    description: '集中调整名称、描述、默认模型、工具策略、上下文模式和 Team 相关参数。',
  },
]
const activeWorkspaceTabDescription = computed(() =>
  workspaceTabs.find(tab => tab.key === activeWorkspaceTab.value)?.description || ''
)

const updateSelectedProfileToolConfig = (config: UiToolConfigPayload) => {
  if (!selectedProfile.value) return
  applyToolConfigToProfile(selectedProfile.value, config)
}

const loading = computed(() => isLoadingAssistantProfiles.value || isLoadingDefaultAssistantProfile.value)
const selectedProfile = computed(
  () => draftProfiles.value.find(profile => profile.id === selectedProfileId.value) || null
)
const selectedProfileMetaItems = computed(() => {
  if (!selectedProfile.value) return []

  return [
    {
      label: 'Agent ID',
      value: selectedProfile.value.id,
    },
    {
      label: '运行模式',
      value: selectedProfile.value.runMode,
    },
    {
      label: '上下文模式',
      value: selectedProfile.value.contextMode,
    },
    {
      label: '默认模型',
      value: selectedProfile.value.defaultModel?.trim() || '跟随 AI 全局默认',
    },
  ]
})
const assistantListItems = computed<AgentListItemViewModel[]>(() =>
  draftProfiles.value.map(profile => {
    const badges = []

    if (defaultAssistantProfileId.value === profile.id) {
      badges.push({
        label: '默认',
        className: 'badge-primary',
      })
    }

    badges.push(getAssistantAgentRunModeBadge(profile))
    badges.push(getAssistantAgentModelBadge(profile))
    badges.push(getAssistantAgentToolsBadge(profile))

    return {
      id: profile.id,
      title: profile.label || profile.id,
      description: profile.description,
      metaLine: getAssistantAgentSecondarySummary(profile),
      badges,
      searchText: [
        profile.runMode,
        profile.contextMode,
        profile.defaultModel || '',
        profile.defaultTeamOrchestrationPresetId || '',
        profile.defaultTeamRecoveryPresetId || '',
      ].join(' '),
      filterKeys: [
        ...(defaultAssistantProfileId.value === profile.id ? ['default'] : []),
        profile.runMode === 'team' ? 'team' : 'assistant',
        profile.defaultModel?.trim() ? 'model-override' : 'model-global',
        profile.defaultToolsEnabled ? 'tools-on' : 'tools-off',
      ],
    }
  })
)
const canSave = computed(
  () =>
    !loading.value
    && !isSavingAssistantProfiles.value
    && draftProfiles.value.length > 0
    && draftProfiles.value.every(
      profile => profile.id.trim() && profile.label.trim() && profile.description.trim()
    )
)

const resetDraftProfiles = (profiles: AssistantProfileOption[]) => {
  draftProfiles.value = profiles.map(profile => ({ ...profile }))
  if (!draftProfiles.value.some(profile => profile.id === selectedProfileId.value)) {
    selectedProfileId.value = draftProfiles.value[0]?.id || ''
  }
}

const reloadProfiles = async () => {
  await Promise.all([
    loadAssistantProfiles(true),
    loadDefaultAssistantProfile(true),
    loadAiConfig(),
  ])
  resetDraftProfiles(profileOptions.value)
}

const loadAiConfig = async () => {
  aiConfig.value = await invoke('get_ai_config')
}

const saveProfiles = async () => {
  await saveAssistantProfiles(
    draftProfiles.value.map(normalizeAssistantProfileDraft)
  )
  resetDraftProfiles(profileOptions.value)
}

const createProfile = () => {
  const { id, nextIndex } = createNextProfileIdentity(draftProfiles.value)
  const profile: AssistantProfileOption = {
    id,
    label: `Custom ${nextIndex}`,
    description: '自定义交互型 Agent',
    defaultModel: null,
    defaultRagEnabled: false,
    defaultWebSearchEnabled: false,
    defaultToolsEnabled: false,
    defaultTenthManEnabled: false,
    defaultToolSelectionStrategy: 'Keyword',
    defaultMaxTools: 5,
    defaultFixedTools: ['interactive_shell'],
    defaultDisabledTools: [],
    defaultManualTools: [],
    defaultTeamOrchestrationPresetId: null,
    defaultTeamRecoveryPresetId: null,
    contextMode: 'claude-like',
    runMode: 'assistant',
  }
  draftProfiles.value.push(profile)
  selectedProfileId.value = profile.id
}

const removeSelectedProfile = () => {
  if (!selectedProfile.value || draftProfiles.value.length <= 1) return
  draftProfiles.value = draftProfiles.value.filter(profile => profile !== selectedProfile.value)
  selectedProfileId.value = draftProfiles.value[0]?.id || ''
}

const saveSelectedAsDefault = async () => {
  if (!selectedProfile.value) return
  await saveDefaultAssistantProfile(selectedProfile.value.id.trim())
}

onMounted(() => {
  void reloadProfiles()
})

watch(
  profileOptions,
  profiles => {
    resetDraftProfiles(profiles)
  },
  { deep: true }
)
</script>
