<template>
  <div class="card bg-base-100 shadow-sm">
    <div class="card-body gap-4">
      <div class="flex items-start justify-between gap-4">
        <div>
          <h3 class="card-title">助手 Profiles</h3>
          <p class="text-sm text-base-content/70">
            管理交互助手可用的 profile、默认上下文模式和运行模式。
          </p>
        </div>

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

      <div v-else class="grid grid-cols-1 xl:grid-cols-[280px_minmax(0,1fr)] gap-4">
        <div class="rounded-xl border border-base-300 bg-base-200/40 p-3 space-y-3">
          <div class="flex items-center justify-between">
            <h4 class="font-semibold">Profile 列表</h4>
            <button class="btn btn-xs btn-outline" @click="createProfile">新增</button>
          </div>

          <div class="space-y-2">
            <button
              v-for="profile in draftProfiles"
              :key="profile.id"
              class="w-full rounded-lg border px-3 py-3 text-left transition"
              :class="
                selectedProfileId === profile.id
                  ? 'border-primary bg-primary/10'
                  : 'border-base-300 bg-base-100 hover:border-primary/40'
              "
              @click="selectedProfileId = profile.id"
            >
              <div class="flex items-center justify-between gap-2">
                <p class="font-medium truncate">{{ profile.label || profile.id }}</p>
                <div class="flex items-center gap-2">
                  <span
                    v-if="defaultAssistantProfileId === profile.id"
                    class="badge badge-primary badge-sm"
                  >
                    默认
                  </span>
                  <span class="badge badge-ghost badge-sm">{{ profile.runMode }}</span>
                </div>
              </div>
              <p class="mt-1 text-xs text-base-content/60 truncate">{{ profile.id }}</p>
            </button>
          </div>
        </div>

        <div class="rounded-xl border border-base-300 bg-base-100 p-4">
          <div v-if="selectedProfile" class="space-y-4">
            <div class="flex items-center justify-between gap-4">
              <div>
                <h4 class="text-lg font-semibold">Profile 编辑</h4>
                <p class="text-sm text-base-content/60">修改后需要手动保存。</p>
              </div>
              <button
                class="btn btn-sm btn-error btn-outline"
                :disabled="draftProfiles.length <= 1"
                @click="removeSelectedProfile"
              >
                删除
              </button>
            </div>

            <button
              class="btn btn-sm btn-outline"
              :disabled="!selectedProfile || isSavingDefaultAssistantProfile"
              @click="saveSelectedAsDefault"
            >
              {{ defaultAssistantProfileId === selectedProfile.id ? '当前默认 Profile' : '设为默认 Profile' }}
            </button>

            <label class="form-control">
              <span class="label-text mb-2">Profile ID</span>
              <input
                :value="selectedProfile.id"
                class="input input-bordered bg-base-200 text-base-content/70"
                type="text"
                readonly
              />
              <span class="label-text-alt mt-2 text-base-content/60">
                Profile ID 是会话绑定引用的稳定身份；需要更换 ID 时请删除后新建。
              </span>
            </label>

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

            <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
              <label class="form-control">
                <span class="label-text mb-2">默认提供商（可选）</span>
                <select
                  :value="selectedProfileDefaultProvider"
                  class="select select-bordered"
                  @change="updateSelectedProfileDefaultProvider"
                >
                  <option value="">跟随 AI 默认配置</option>
                  <option
                    v-for="provider in aiProviderOptions"
                    :key="provider.value"
                    :value="provider.value"
                  >
                    {{ provider.label }}
                  </option>
                </select>
                <span class="label-text-alt mt-2 text-base-content/60">
                  当前 AI 默认：{{ aiDefaultModelLabel }}
                </span>
              </label>

              <label class="form-control">
                <span class="label-text mb-2">默认模型（可选）</span>
                <input
                  :value="selectedProfileDefaultModel"
                  :disabled="!selectedProfileDefaultProvider"
                  :list="selectedProfileModelDatalistId"
                  class="input input-bordered"
                  type="text"
                  placeholder="选择或输入模型 ID"
                  @input="updateSelectedProfileDefaultModel"
                />
                <datalist :id="selectedProfileModelDatalistId">
                  <option
                    v-for="model in selectedProfileModelOptions"
                    :key="model.value"
                    :value="model.value"
                  >
                    {{ model.label }}
                  </option>
                </datalist>
              </label>
            </div>

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

            <div class="collapse collapse-arrow rounded-xl border border-base-300 bg-base-100">
              <input type="checkbox" />
              <div class="collapse-title font-semibold">默认工具配置</div>
              <div class="collapse-content">
                <ToolConfigPanel
                  :config="selectedProfileToolConfig"
                  :show-header="false"
                  :show-footer="false"
                  @update:config="updateSelectedProfileToolConfig"
                />
              </div>
            </div>

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

          <div v-else class="py-12 text-center text-base-content/60">
            请选择一个助手 profile。
          </div>
        </div>
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
import type { UiToolConfigPayload } from '@/components/Agent/toolConfigRuntime'
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

const draftProfiles = ref<AssistantProfileOption[]>([])
const selectedProfileId = ref('')
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

const updateSelectedProfileDefaultProvider = (event: Event) => {
  if (!selectedProfile.value) return
  const provider = (event.target as HTMLSelectElement).value.trim()
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

const updateSelectedProfileDefaultModel = (event: Event) => {
  if (!selectedProfile.value || !selectedProfileDefaultProvider.value) return
  const model = (event.target as HTMLInputElement).value.trim()
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

const updateSelectedProfileToolConfig = (config: UiToolConfigPayload) => {
  if (!selectedProfile.value) return
  applyToolConfigToProfile(selectedProfile.value, config)
}

const loading = computed(() => isLoadingAssistantProfiles.value || isLoadingDefaultAssistantProfile.value)
const selectedProfile = computed(
  () => draftProfiles.value.find(profile => profile.id === selectedProfileId.value) || null
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
    description: '自定义助手 profile',
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
