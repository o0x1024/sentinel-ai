<template>
  <div class="rounded-lg border border-base-300 bg-base-100 p-4">
    <div class="flex items-start justify-between gap-3">
      <div>
        <div class="text-sm font-semibold">LLM 覆盖</div>
        <div class="mt-1 text-xs text-base-content/60">
          不设置时自动继承 AI 设置中的默认提供商和模型。
        </div>
      </div>
      <div class="badge badge-ghost text-xs">
        {{ hasOverride ? '自定义中' : '跟随全局' }}
      </div>
    </div>

    <div class="mt-4 grid grid-cols-1 lg:grid-cols-2 gap-4">
      <label class="form-control">
        <span class="label-text">LLM 提供商</span>
        <select
          :value="modelValue.provider || ''"
          class="select select-bordered"
          @change="handleProviderChange"
        >
          <option value="">跟随全局默认</option>
          <option
            v-for="option in providerOptions"
            :key="option.value"
            :value="option.value"
          >
            {{ option.label }}
          </option>
        </select>
        <span class="label-text-alt text-base-content/60 mt-1">
          当前全局默认：{{ globalDefaultLabel }}
        </span>
      </label>

      <label class="form-control">
        <span class="label-text">模型</span>
        <input
          :value="modelValue.model || ''"
          :disabled="!modelValue.provider"
          :list="modelSuggestionsListId"
          type="text"
          class="input input-bordered"
          :placeholder="modelPlaceholder"
          @input="handleModelInput"
        />
        <datalist :id="modelSuggestionsListId">
          <option v-for="item in modelSuggestions" :key="item" :value="item" />
        </datalist>
        <span class="label-text-alt text-base-content/60 mt-1">
          {{ modelHintText }}
        </span>
      </label>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'

type ProviderOption = {
  value: string
  label: string
}

const props = defineProps<{
  profileId: string
  modelValue: {
    provider?: string | null
    model?: string | null
  }
  providerOptions: ProviderOption[]
  modelSuggestions: string[]
  globalDefaultLabel: string
}>()

const emit = defineEmits<{
  'update:modelValue': [value: { provider: string | null; model: string | null }]
}>()

const hasOverride = computed(() => {
  return !!props.modelValue.provider || !!props.modelValue.model
})

const modelSuggestionsListId = computed(() => {
  return `system-agent-llm-models-${props.profileId || 'profile'}`
})

const modelPlaceholder = computed(() => {
  if (!props.modelValue.provider) return '先选择提供商'
  if (props.modelSuggestions.length > 0) return '可输入或选择建议模型'
  return '输入该提供商下的模型 ID'
})

const modelHintText = computed(() => {
  if (!props.modelValue.provider) {
    return '清空提供商后会回退到全局默认 provider/model。'
  }
  if (props.modelSuggestions.length > 0) {
    return `已加载 ${props.modelSuggestions.length} 个模型建议，也可手动输入。`
  }
  return '当前提供商没有可枚举模型，手动输入模型 ID 即可。'
})

function handleProviderChange(event: Event) {
  const target = event.target as HTMLSelectElement | null
  const provider = target?.value?.trim() || null
  emit('update:modelValue', {
    provider,
    model: provider ? props.modelValue.model || null : null,
  })
}

function handleModelInput(event: Event) {
  const target = event.target as HTMLInputElement | null
  const model = target?.value?.trim() || null
  emit('update:modelValue', {
    provider: props.modelValue.provider?.trim() || null,
    model,
  })
}
</script>
