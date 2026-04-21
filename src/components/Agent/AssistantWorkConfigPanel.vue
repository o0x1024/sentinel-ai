<template>
  <div class="assistant-work-config-panel h-full flex flex-col bg-base-100">
    <div class="panel-header flex items-center justify-between p-4 border-b border-base-300">
      <div class="flex items-center gap-2">
        <i class="fas fa-sliders-h text-primary"></i>
        <h3 class="text-lg font-semibold">工作配置</h3>
      </div>
      <button @click="$emit('close')" class="btn btn-sm btn-ghost btn-circle">
        <i class="fas fa-times"></i>
      </button>
    </div>

    <div class="panel-content flex-1 overflow-y-auto p-4 space-y-4">
      <section class="rounded-xl border border-base-300 bg-base-100 p-4 space-y-4">
        <div>
          <h4 class="text-sm font-semibold text-base-content">当前会话</h4>
          <p class="text-xs text-base-content/60 mt-1">
            这些设置只作用于当前对话，会随会话绑定一起持久化。
          </p>
        </div>

        <div class="form-control">
          <label class="label">
            <span class="label-text font-medium">Agent Profile</span>
          </label>
          <select
            :value="profileId"
            class="select select-bordered w-full"
            @change="handleProfileIdChange"
          >
            <option
              v-for="profile in profileOptions"
              :key="profile.id"
              :value="profile.id"
            >
              {{ profile.label }}
            </option>
          </select>
          <label class="label">
            <span class="label-text-alt text-base-content/60">
              {{ profileLoading ? '正在加载 profile 列表…' : selectedProfileDescription }}
            </span>
          </label>
        </div>

        <div class="grid grid-cols-1 gap-4 md:grid-cols-2">
          <div class="form-control">
            <label class="label">
              <span class="label-text font-medium">上下文模式</span>
            </label>
            <select
              :value="contextMode"
              class="select select-bordered w-full"
              @change="handleContextModeChange"
            >
              <option value="claude-like">Claude-like</option>
              <option value="codex-like">Codex-like</option>
              <option value="sentinel-like">Sentinel-like</option>
            </select>
          </div>

          <div class="form-control">
            <label class="label">
              <span class="label-text font-medium">运行模式</span>
            </label>
            <select
              :value="runMode"
              class="select select-bordered w-full"
              @change="handleRunModeChange"
            >
              <option value="assistant">Assistant</option>
              <option value="team">Team</option>
            </select>
          </div>
        </div>

        <div class="form-control">
          <label class="label">
            <span class="label-text font-medium">模型</span>
          </label>
          <SearchableSelect
            :model-value="selectedModel || ''"
            :options="displayModels"
            :placeholder="t('agent.followDefaultModel')"
            :search-placeholder="t('agent.searchModelsOrProviders')"
            :no-results-text="t('agent.noMatchingModels')"
            :disabled="modelLoading"
            size="md"
            group-by="description"
            @update:model-value="handleModelChange"
          />
          <label class="label">
            <span class="label-text-alt text-base-content/60">
              {{ modelLoading ? t('agent.loadingAssistantModels') : t('agent.workConfigVisionHint') }}
            </span>
          </label>
        </div>

      </section>

      <section class="rounded-xl border border-base-300 bg-base-100 overflow-hidden">
        <ToolConfigPanel
          :config="toolConfig"
          :show-header="false"
          :show-footer="false"
          @update:config="$emit('update:tool-config', $event)"
        />
      </section>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import SearchableSelect from '@/components/SearchableSelect.vue'
import type { AssistantModelOption, AssistantContextMode, AssistantRunMode } from './agentDraftTypes'
import type { AssistantProfileOption } from './assistantProfiles'
import ToolConfigPanel from './ToolConfigPanel.vue'
import type { UiToolConfigPayload } from './toolConfigRuntime'

const { t } = useI18n()

const props = defineProps<{
  availableModels: AssistantModelOption[]
  contextMode: AssistantContextMode
  modelLoading: boolean
  profileId: string
  profileLoading: boolean
  profileOptions: AssistantProfileOption[]
  runMode: AssistantRunMode
  selectedModel: string | null
  toolConfig: UiToolConfigPayload
}>()

const selectedProfileDescription = computed(() =>
  props.profileOptions.find((profile) => profile.id === props.profileId)?.description ||
  '当前会话会跟随这个 profile 的默认上下文与运行模式。',
)

const getVisionCapabilitySuffix = (visionCapability: AssistantModelOption['visionCapability']) => {
  switch (visionCapability) {
    case 'supported':
      return ` [${t('agent.visionCapabilitySupportedLabel')}]`
    case 'unsupported':
      return ` [${t('agent.visionCapabilityUnsupportedLabel')}]`
    default:
      return ` [${t('agent.visionCapabilityUnknownLabel')}]`
  }
}

const displayModels = computed(() =>
  [
    {
      value: '',
      label: t('agent.followDefaultModel'),
      description: '',
    },
    ...[...props.availableModels]
      .map((model) => {
        const providerLabel = model.description?.trim()
          || model.value.split('/')[0]?.trim()
          || 'Unknown'
        return {
          ...model,
          label: `${model.label}${getVisionCapabilitySuffix(model.visionCapability)}`,
          description: providerLabel,
        }
      }),
  ]
    .sort((a, b) => {
      if (!a.value) return -1
      if (!b.value) return 1
      return `${a.description || ''}/${a.label}`.localeCompare(`${b.description || ''}/${b.label}`)
    }),
)

const emit = defineEmits<{
  (e: 'close'): void
  (e: 'update:context-mode', value: AssistantContextMode): void
  (e: 'update:model', value: string | null): void
  (e: 'update:profile-id', value: string): void
  (e: 'update:run-mode', value: AssistantRunMode): void
  (e: 'update:tool-config', value: UiToolConfigPayload): void
}>()

const handleProfileIdChange = (event: Event) => {
  const target = event.target as HTMLSelectElement
  emit('update:profile-id', target.value.trim())
}

const handleContextModeChange = (event: Event) => {
  const target = event.target as HTMLSelectElement
  emit('update:context-mode', target.value as AssistantContextMode)
}

const handleRunModeChange = (event: Event) => {
  const target = event.target as HTMLSelectElement
  emit('update:run-mode', target.value as AssistantRunMode)
}

const handleModelChange = (value: string) => {
  const normalized = value.trim()
  emit('update:model', normalized || null)
}
</script>

<style scoped>
.assistant-work-config-panel {
  min-height: 0;
}

.panel-content {
  min-height: 0;
}
</style>
