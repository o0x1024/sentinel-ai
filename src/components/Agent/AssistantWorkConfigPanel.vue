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

        <div class="form-control">
          <label class="label">
            <span class="label-text font-medium">执行模式</span>
          </label>
          <div class="join w-full">
            <button
              type="button"
              class="btn join-item flex-1"
              :class="executionMode === 'single' ? 'btn-primary' : 'btn-outline'"
              @click="$emit('update:execution-mode', 'single')"
            >
              单模型
            </button>
            <button
              type="button"
              class="btn join-item flex-1"
              :class="executionMode === 'parallel' ? 'btn-primary' : 'btn-outline'"
              @click="$emit('update:execution-mode', 'parallel')"
            >
              多模型并行
            </button>
          </div>
        </div>

        <div v-if="executionMode === 'parallel'" class="rounded-lg border border-base-300 bg-base-200/40 p-3 space-y-3">
          <div class="flex items-center justify-between gap-2">
            <span class="text-sm font-medium">并行模型</span>
            <span class="badge badge-sm" :class="parallelSelectedModels.length >= 2 ? 'badge-success' : 'badge-warning'">
              {{ parallelSelectedModels.length }}/{{ availableModels.length }}
            </span>
          </div>
          <div class="max-h-56 overflow-y-auto space-y-1 pr-1">
            <label
              v-for="model in displayParallelModels"
              :key="model.value"
              class="flex cursor-pointer items-center gap-2 rounded px-2 py-1.5 hover:bg-base-300/60"
            >
              <input
                type="checkbox"
                class="checkbox checkbox-sm"
                :checked="parallelSelectedModels.includes(model.value)"
                @change="handleParallelModelToggle(model.value)"
              />
              <span class="min-w-0 flex-1 truncate text-sm">{{ model.label }}</span>
              <span class="text-xs text-base-content/60">{{ model.description }}</span>
            </label>
          </div>
          <p v-if="parallelSelectedModels.length < 2" class="text-xs text-warning">
            多模型并行执行至少需要选择两个模型。
          </p>

          <div class="rounded border border-base-300 bg-base-100 p-2">
            <div class="mb-2 flex items-center gap-2">
              <input
                v-model="presetName"
                class="input input-xs input-bordered flex-1"
                placeholder="组合预设名称"
              />
              <button type="button" class="btn btn-xs btn-outline" @click="saveParallelPreset">
                保存
              </button>
            </div>
            <div v-if="parallelPresets.length" class="flex flex-wrap gap-2">
              <button
                v-for="preset in parallelPresets"
                :key="preset.name"
                type="button"
                class="btn btn-xs btn-ghost"
                @click="applyParallelPreset(preset)"
              >
                {{ preset.name }}
              </button>
            </div>
          </div>

          <div class="form-control">
            <label class="label">
              <span class="label-text font-medium">Judge 模型</span>
            </label>
            <SearchableSelect
              :model-value="parallelJudgeModel || ''"
              :options="displayJudgeModels"
              placeholder="不自动汇总"
              :search-placeholder="t('agent.searchModelsOrProviders')"
              :no-results-text="t('agent.noMatchingModels')"
              size="sm"
              group-by="description"
              @update:model-value="handleJudgeModelChange"
            />
          </div>
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
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import SearchableSelect from '@/components/SearchableSelect.vue'
import type { AssistantModelOption, AssistantContextMode, AssistantRunMode } from './agentDraftTypes'
import type { AssistantExecutionMode } from './useAgentModelAndToolConfig'
import type { AssistantProfileOption } from './assistantProfiles'
import ToolConfigPanel from './ToolConfigPanel.vue'
import type { UiToolConfigPayload } from './toolConfigRuntime'

const { t } = useI18n()
const PARALLEL_PRESETS_STORAGE_KEY = 'sentinel:agent:parallel-model-presets'

type ParallelPreset = {
  name: string
  models: string[]
  judgeModel: string
}

const props = defineProps<{
  availableModels: AssistantModelOption[]
  contextMode: AssistantContextMode
  executionMode: AssistantExecutionMode
  modelLoading: boolean
  parallelJudgeModel: string
  parallelSelectedModels: string[]
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

const displayParallelModels = computed(() =>
  [...props.availableModels]
    .map((model) => {
      const providerLabel = model.description?.trim()
        || model.value.split('/')[0]?.trim()
        || 'Unknown'
      return {
        ...model,
        label: `${model.label}${getVisionCapabilitySuffix(model.visionCapability)}`,
        description: providerLabel,
      }
    })
    .sort((a, b) => `${a.description || ''}/${a.label}`.localeCompare(`${b.description || ''}/${b.label}`)),
)

const displayJudgeModels = computed(() => [
  {
    value: '',
    label: '不自动汇总',
    description: '',
  },
  ...displayParallelModels.value,
])

const emit = defineEmits<{
  (e: 'close'): void
  (e: 'update:context-mode', value: AssistantContextMode): void
  (e: 'update:execution-mode', value: AssistantExecutionMode): void
  (e: 'update:model', value: string | null): void
  (e: 'update:parallel-judge-model', value: string): void
  (e: 'update:parallel-models', value: string[]): void
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

const handleParallelModelToggle = (value: string) => {
  const selected = new Set(props.parallelSelectedModels)
  if (selected.has(value)) {
    selected.delete(value)
  } else {
    selected.add(value)
  }
  emit('update:parallel-models', Array.from(selected))
}

const handleJudgeModelChange = (value: string) => {
  emit('update:parallel-judge-model', value.trim())
}

const presetName = ref('')
const parallelPresets = ref<ParallelPreset[]>([])

const loadParallelPresets = () => {
  try {
    const parsed = JSON.parse(localStorage.getItem(PARALLEL_PRESETS_STORAGE_KEY) || '[]')
    parallelPresets.value = Array.isArray(parsed) ? parsed : []
  } catch {
    parallelPresets.value = []
  }
}

const persistParallelPresets = () => {
  localStorage.setItem(PARALLEL_PRESETS_STORAGE_KEY, JSON.stringify(parallelPresets.value))
}

const saveParallelPreset = () => {
  const name = presetName.value.trim()
  if (!name || props.parallelSelectedModels.length < 2) return
  const next = {
    name,
    models: [...props.parallelSelectedModels],
    judgeModel: props.parallelJudgeModel || '',
  }
  parallelPresets.value = [
    next,
    ...parallelPresets.value.filter((preset) => preset.name !== name),
  ].slice(0, 12)
  persistParallelPresets()
}

const applyParallelPreset = (preset: ParallelPreset) => {
  emit('update:parallel-models', preset.models)
  emit('update:parallel-judge-model', preset.judgeModel || '')
}

loadParallelPresets()
</script>

<style scoped>
.assistant-work-config-panel {
  min-height: 0;
}

.panel-content {
  min-height: 0;
}
</style>
