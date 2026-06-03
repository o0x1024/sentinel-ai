<template>
  <div class="rounded-lg border border-base-300/70 bg-base-100/70 p-3 space-y-3">
    <div class="flex flex-wrap items-center justify-between gap-2">
      <div>
        <div class="text-sm font-medium">Seed Bindings</div>
        <div class="text-xs text-base-content/60">
          定义插件支持的发现种子映射。监控任务会基于这里的声明做显式配置。
        </div>
      </div>
      <div class="join">
        <button
          type="button"
          class="join-item btn btn-xs"
          :class="mode === 'visual' ? 'btn-primary' : 'btn-ghost'"
          @click="switchMode('visual')"
        >
          图形化
        </button>
        <button
          type="button"
          class="join-item btn btn-xs"
          :class="mode === 'json' ? 'btn-primary' : 'btn-ghost'"
          @click="switchMode('json')"
        >
          JSON
        </button>
      </div>
    </div>

    <div v-if="mode === 'visual'" class="space-y-2">
      <div
        v-if="inputKeyLoading"
        class="rounded-md border border-base-300/70 bg-base-200/40 px-3 py-2 text-xs text-base-content/70"
      >
        正在从当前 input schema 提取可绑定字段...
      </div>
      <div
        v-else-if="inputKeyError"
        class="rounded-md border border-warning/40 bg-warning/10 px-3 py-2 text-xs text-warning"
      >
        {{ inputKeyError }}
      </div>
      <div
        v-else-if="resolvedInputKeyOptions.length === 0"
        class="rounded-md border border-base-300/70 bg-base-200/40 px-3 py-2 text-xs text-base-content/70"
      >
        当前 input schema 中没有可绑定字段。图形化模式只支持 `string` 和 `array&lt;string&gt;` 顶层参数。
      </div>

      <div class="grid grid-cols-[minmax(0,1fr)_minmax(0,1fr)_auto] gap-2 px-1">
        <div class="text-xs font-medium text-base-content/70">Seed Type</div>
        <div class="text-xs font-medium text-base-content/70">Input Key</div>
        <div></div>
      </div>

      <div
        v-for="(binding, index) in visualBindings"
        :key="`${binding.seed_type}-${binding.input_key}-${index}`"
        class="grid grid-cols-[minmax(0,1fr)_minmax(0,1fr)_auto] gap-2 items-start"
      >
        <div class="form-control">
          <select
            :value="binding.seed_type"
            class="select select-bordered select-sm"
            :disabled="disabled"
            @change="updateVisualBinding(index, 'seed_type', ($event.target as HTMLSelectElement).value)"
          >
            <option value="">请选择</option>
            <option v-for="option in seedTypeOptions" :key="option.value" :value="option.value">
              {{ option.label }}
            </option>
          </select>
        </div>
        <div class="form-control">
          <select
            :value="binding.input_key"
            class="select select-bordered select-sm"
            :disabled="disabled || resolvedInputKeyOptions.length === 0"
            @change="updateVisualBinding(index, 'input_key', ($event.target as HTMLSelectElement).value)"
          >
            <option value="">请选择</option>
            <option v-for="option in resolvedInputKeyOptions" :key="option.value" :value="option.value">
              {{ option.label }}
            </option>
          </select>
          <div class="min-h-5 pt-1 text-xs text-base-content/60">
            {{ selectedInputKeyDescription(binding.input_key) }}
          </div>
        </div>
        <button
          type="button"
          class="btn btn-ghost btn-sm text-error self-start mt-0.5"
          :disabled="disabled"
          @click="removeVisualBinding(index)"
        >
          <i class="fas fa-trash"></i>
        </button>
      </div>

      <div class="flex items-center gap-2">
        <button
          type="button"
          class="btn btn-ghost btn-sm"
          :disabled="disabled || resolvedInputKeyOptions.length === 0"
          @click="addVisualBinding"
        >
          <i class="fas fa-plus mr-1"></i>
          添加绑定
        </button>
        <span class="text-xs text-base-content/60">
          当前 {{ visualBindings.length }} 条
        </span>
      </div>
    </div>

    <div v-else class="space-y-2">
      <textarea
        :value="modelValue"
        rows="6"
        class="textarea textarea-bordered textarea-sm font-mono w-full"
        :class="jsonError ? 'textarea-error' : ''"
        :disabled="disabled"
        placeholder='[
  { "seed_type": "root_domain", "input_key": "domains" }
]'
        @input="$emit('update:modelValue', ($event.target as HTMLTextAreaElement).value)"
      />
      <div class="flex items-center justify-between gap-2">
        <span class="text-xs text-base-content/60">
          空数组表示插件不声明任何发现种子能力。
        </span>
        <span v-if="jsonError" class="text-xs text-error">
          {{ jsonError }}
        </span>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import {
  MONITOR_SEED_TYPE_OPTIONS,
  type SeedBindingInputKeyOption,
  normalizeSeedBindings,
  parseSeedBindingsText,
  stringifySeedBindings,
  humanizeSeedType,
} from './seedBindingsSupport'

defineOptions({ name: 'SeedBindingsEditor' })

const props = defineProps<{
  modelValue: string
  disabled?: boolean
  jsonError?: string
  inputKeyOptions?: SeedBindingInputKeyOption[]
  inputKeyLoading?: boolean
  inputKeyError?: string
}>()

const emit = defineEmits<{
  (e: 'update:modelValue', value: string): void
}>()

const mode = ref<'visual' | 'json'>('visual')
const lastVisualEmission = ref<string | null>(null)

const parsedBindings = computed(() => parseSeedBindingsText(props.modelValue || '[]').bindings)

const visualBindings = ref(normalizeSeedBindings(parsedBindings.value))

const seedTypeOptions = computed(() =>
  MONITOR_SEED_TYPE_OPTIONS.map(value => ({
    value,
    label: humanizeSeedType(value),
  }))
)

const resolvedInputKeyOptions = computed(() => {
  const options = Array.isArray(props.inputKeyOptions) ? props.inputKeyOptions : []
  const dedupe = new Set<string>()
  const merged: SeedBindingInputKeyOption[] = []

  for (const option of options) {
    if (!option?.value || dedupe.has(option.value)) {
      continue
    }
    dedupe.add(option.value)
    merged.push(option)
  }

  for (const binding of visualBindings.value) {
    if (!binding.input_key || dedupe.has(binding.input_key)) {
      continue
    }
    dedupe.add(binding.input_key)
    merged.push({
      value: binding.input_key,
      label: `${binding.input_key} (当前值)`,
      typeLabel: 'unknown',
      description: '当前绑定值未出现在最新 input schema 中。',
    })
  }

  return merged
})

const selectedInputKeyDescription = (inputKey: string) =>
  resolvedInputKeyOptions.value.find(option => option.value === inputKey)?.description || ''

const syncVisualToModel = () => {
  const nextValue = stringifySeedBindings(visualBindings.value)
  lastVisualEmission.value = nextValue
  emit('update:modelValue', nextValue)
}

const addVisualBinding = () => {
  visualBindings.value = [
    ...visualBindings.value,
    { seed_type: '', input_key: resolvedInputKeyOptions.value[0]?.value || '' },
  ]
  syncVisualToModel()
}

const updateVisualBinding = (index: number, key: 'seed_type' | 'input_key', value: string) => {
  visualBindings.value = visualBindings.value.map((binding, bindingIndex) =>
    bindingIndex === index ? { ...binding, [key]: value } : binding
  )
  syncVisualToModel()
}

const removeVisualBinding = (index: number) => {
  visualBindings.value = visualBindings.value.filter((_, bindingIndex) => bindingIndex !== index)
  syncVisualToModel()
}

const switchMode = (nextMode: 'visual' | 'json') => {
  if (nextMode === 'visual' && props.jsonError) {
    return
  }
  mode.value = nextMode
}

watch(
  () => props.modelValue,
  value => {
    if (mode.value === 'visual' && value === lastVisualEmission.value) {
      return
    }
    const parsed = parseSeedBindingsText(value || '[]')
    if (!parsed.error) {
      visualBindings.value = normalizeSeedBindings(parsed.bindings)
    }
  },
  { immediate: true },
)
</script>
