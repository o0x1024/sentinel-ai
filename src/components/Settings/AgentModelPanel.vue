<template>
  <div class="rounded-lg border border-base-300 bg-base-100 p-4">
    <div class="flex items-start justify-between gap-3">
      <div>
        <div class="text-sm font-semibold">{{ title }}</div>
        <div class="mt-1 text-xs text-base-content/60">
          {{ description }}
        </div>
      </div>
      <div class="badge badge-ghost text-xs">
        {{ hasOverride ? customBadgeLabel : followBadgeLabel }}
      </div>
    </div>

    <div class="mt-4 grid grid-cols-1 gap-4 lg:grid-cols-2">
      <label class="form-control">
        <span class="label-text">{{ providerLabel }}</span>
        <select
          :value="providerValue"
          class="select select-bordered"
          @change="handleProviderChange"
        >
          <option value="">{{ followDefaultOptionLabel }}</option>
          <option
            v-for="option in providerOptions"
            :key="option.value"
            :value="option.value"
          >
            {{ option.label }}
          </option>
        </select>
        <span class="label-text-alt mt-1 text-base-content/60">
          当前全局默认：{{ globalDefaultLabel }}
        </span>
      </label>

      <label class="form-control">
        <span class="label-text">{{ modelLabel }}</span>
        <input
          :value="modelValue"
          :disabled="disableModelInput && !providerValue"
          :list="datalistId"
          type="text"
          class="input input-bordered"
          :placeholder="modelPlaceholder"
          @input="handleModelInput"
        />
        <datalist :id="datalistId">
          <option v-for="option in modelOptions" :key="option.value" :value="option.value">
            {{ option.label }}
          </option>
        </datalist>
        <span class="label-text-alt mt-1 text-base-content/60">
          {{ modelHintText }}
        </span>
      </label>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'

type SelectOption = {
  value: string
  label: string
}

const props = withDefaults(defineProps<{
  title?: string
  description?: string
  providerLabel?: string
  modelLabel?: string
  providerValue?: string
  modelValue?: string
  providerOptions: SelectOption[]
  modelOptions?: SelectOption[]
  globalDefaultLabel: string
  followDefaultOptionLabel?: string
  followBadgeLabel?: string
  customBadgeLabel?: string
  datalistId: string
  disableModelInput?: boolean
  noProviderPlaceholder?: string
  suggestedPlaceholder?: string
  manualPlaceholder?: string
  noProviderHint?: string
  suggestedHint?: string
  manualHint?: string
}>(), {
  title: '模型配置',
  description: '不设置时自动继承 AI 设置中的默认提供商和模型。',
  providerLabel: '提供商',
  modelLabel: '模型',
  providerValue: '',
  modelValue: '',
  modelOptions: () => [],
  followDefaultOptionLabel: '跟随全局默认',
  followBadgeLabel: '跟随全局',
  customBadgeLabel: '自定义中',
  disableModelInput: true,
  noProviderPlaceholder: '先选择提供商',
  suggestedPlaceholder: '可输入或选择建议模型',
  manualPlaceholder: '输入模型 ID',
  noProviderHint: '清空提供商后会回退到全局默认 provider/model。',
  suggestedHint: '可从建议列表选择，也可手动输入模型 ID。',
  manualHint: '当前提供商没有可枚举模型，手动输入模型 ID 即可。',
})

const emit = defineEmits<{
  'update:providerValue': [value: string]
  'update:modelValue': [value: string]
}>()

const hasOverride = computed(() => {
  return props.providerValue.trim().length > 0 || props.modelValue.trim().length > 0
})

const modelPlaceholder = computed(() => {
  if (!props.providerValue && props.disableModelInput) return props.noProviderPlaceholder
  if (props.modelOptions.length > 0) return props.suggestedPlaceholder
  return props.manualPlaceholder
})

const modelHintText = computed(() => {
  if (!props.providerValue && props.disableModelInput) return props.noProviderHint
  if (props.modelOptions.length > 0) return props.suggestedHint
  return props.manualHint
})

function handleProviderChange(event: Event) {
  const target = event.target as HTMLSelectElement | null
  emit('update:providerValue', target?.value?.trim() || '')
}

function handleModelInput(event: Event) {
  const target = event.target as HTMLInputElement | null
  emit('update:modelValue', target?.value?.trim() || '')
}
</script>
