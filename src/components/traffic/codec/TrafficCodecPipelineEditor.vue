<template>
  <div class="space-y-3">
    <div
      v-for="(step, index) in modelValue"
      :key="step.id"
      class="card border border-base-300 bg-base-100 shadow-sm"
      :class="dragOverStepId === step.id ? 'ring-2 ring-primary/40' : ''"
      draggable="true"
      @dragstart="handleDragStart(step.id)"
      @dragenter.prevent="handleDragEnter(step.id)"
      @dragover.prevent="handleDragEnter(step.id)"
      @drop.prevent="handleDrop(step.id)"
      @dragend="resetDragState"
    >
      <div class="card-body gap-3 p-4">
        <div class="flex items-start gap-3">
          <button
            type="button"
            class="btn btn-ghost btn-xs mt-1 cursor-grab px-2 active:cursor-grabbing"
            tabindex="-1"
            aria-label="拖拽排序"
          >
            <i class="fas fa-grip-lines text-base-content/40"></i>
          </button>

          <div class="min-w-0 flex-1 space-y-3">
            <div class="flex flex-wrap items-center gap-3">
              <select
                :value="step.codec"
                class="select select-bordered select-sm min-w-[10rem]"
                @change="onCodecChange(index, ($event.target as HTMLSelectElement).value)"
              >
                <optgroup
                  v-for="group in codecGroups"
                  :key="group.category"
                  :label="group.label"
                >
                  <option v-for="codec in group.items" :key="codec.id" :value="codec.id">
                    {{ codec.label }}
                  </option>
                </optgroup>
              </select>

              <label class="flex items-center gap-2 text-sm">
                <input
                  :checked="step.enabled"
                  type="checkbox"
                  class="toggle toggle-sm toggle-primary"
                  @change="updateStep(index, { enabled: ($event.target as HTMLInputElement).checked })"
                />
                <span>启用</span>
              </label>
            </div>

            <div v-if="isBase64Codec(step.codec)" class="grid gap-2 sm:grid-cols-[8rem_minmax(0,1fr)] sm:items-center">
              <span class="text-sm text-base-content/70">变体</span>
              <select
                :value="getConfigValue(step, 'variant') || 'standard'"
                class="select select-bordered select-sm"
                @change="setConfigValue(index, 'variant', ($event.target as HTMLSelectElement).value)"
              >
                <option value="standard">Standard</option>
                <option value="url-safe">URL-Safe</option>
              </select>
            </div>

            <div v-else-if="isAesCodec(step.codec)" class="space-y-2">
              <div class="grid gap-2 sm:grid-cols-[8rem_minmax(0,1fr)] sm:items-center">
                <span class="text-sm text-base-content/70">密钥 (key)</span>
                <input
                  :value="getConfigValue(step, 'key')"
                  type="text"
                  class="input input-bordered input-sm font-mono"
                  placeholder="密钥"
                  @input="setConfigValue(index, 'key', ($event.target as HTMLInputElement).value)"
                />
              </div>
              <div class="grid gap-2 sm:grid-cols-[8rem_minmax(0,1fr)] sm:items-center">
                <span class="text-sm text-base-content/70">密钥格式</span>
                <select
                  :value="getConfigValue(step, 'key_format') || 'utf8'"
                  class="select select-bordered select-sm"
                  @change="setConfigValue(index, 'key_format', ($event.target as HTMLSelectElement).value)"
                >
                  <option value="hex">Hex</option>
                  <option value="base64">Base64</option>
                  <option value="utf8">UTF-8</option>
                </select>
              </div>
              <div
                v-if="step.codec === 'aes-cbc'"
                class="grid gap-2 sm:grid-cols-[8rem_minmax(0,1fr)] sm:items-center"
              >
                <span class="text-sm text-base-content/70">IV</span>
                <input
                  :value="getConfigValue(step, 'iv')"
                  type="text"
                  class="input input-bordered input-sm font-mono"
                  placeholder="初始化向量 (16 字节)"
                  @input="setConfigValue(index, 'iv', ($event.target as HTMLInputElement).value)"
                />
              </div>
              <div
                v-if="step.codec === 'aes-cbc'"
                class="grid gap-2 sm:grid-cols-[8rem_minmax(0,1fr)] sm:items-center"
              >
                <span class="text-sm text-base-content/70">IV 格式</span>
                <select
                  :value="getConfigValue(step, 'iv_format') || 'utf8'"
                  class="select select-bordered select-sm"
                  @change="setConfigValue(index, 'iv_format', ($event.target as HTMLSelectElement).value)"
                >
                  <option value="hex">Hex</option>
                  <option value="base64">Base64</option>
                  <option value="utf8">UTF-8</option>
                </select>
              </div>
              <p class="text-xs text-base-content/55">填充方式：PKCS#7</p>
            </div>

            <div v-else-if="step.codec === 'xor'" class="grid gap-2 sm:grid-cols-[8rem_minmax(0,1fr)] sm:items-center">
              <span class="text-sm text-base-content/70">密钥 (key)</span>
              <input
                :value="getConfigValue(step, 'key')"
                type="text"
                class="input input-bordered input-sm font-mono"
                placeholder="XOR 密钥"
                @input="setConfigValue(index, 'key', ($event.target as HTMLInputElement).value)"
              />
            </div>

            <div v-else-if="needsGenericConfig(step.codec)" class="space-y-2">
              <div
                v-for="entry in getGenericConfigEntries(step)"
                :key="`${step.id}-${entry.key}`"
                class="flex items-center gap-2"
              >
                <input
                  :value="entry.key"
                  type="text"
                  class="input input-bordered input-sm w-32 font-mono"
                  placeholder="键"
                  @input="renameGenericConfigKey(index, entry.key, ($event.target as HTMLInputElement).value)"
                />
                <input
                  :value="entry.value"
                  type="text"
                  class="input input-bordered input-sm min-w-0 flex-1 font-mono"
                  placeholder="值"
                  @input="setConfigValue(index, entry.key, ($event.target as HTMLInputElement).value)"
                />
                <button
                  type="button"
                  class="btn btn-ghost btn-xs"
                  aria-label="删除配置项"
                  @click="removeGenericConfigKey(index, entry.key)"
                >
                  <i class="fas fa-times"></i>
                </button>
              </div>
              <button type="button" class="btn btn-ghost btn-xs" @click="addGenericConfigEntry(index)">
                <i class="fas fa-plus mr-1"></i>
                添加配置项
              </button>
            </div>

            <p v-else-if="hasNoConfig(step.codec)" class="text-xs text-base-content/55">
              此编解码器无需额外配置
            </p>
          </div>

          <button
            type="button"
            class="btn btn-ghost btn-xs text-error"
            aria-label="删除步骤"
            @click="removeStep(index)"
          >
            <i class="fas fa-trash-alt"></i>
          </button>
        </div>
      </div>
    </div>

    <button type="button" class="btn btn-outline btn-sm" @click="addStep">
      <i class="fas fa-plus mr-1"></i>
      添加步骤
    </button>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'
import { BUILTIN_CODECS } from './trafficCodecTypes'
import type { CodecStep } from './trafficCodecTypes'

const props = defineProps<{
  modelValue: CodecStep[]
}>()

const emit = defineEmits<{
  'update:modelValue': [steps: CodecStep[]]
}>()

const NO_CONFIG_CODECS = new Set(['gzip', 'deflate', 'hex', 'url'])
const AES_CODECS = new Set(['aes-cbc', 'aes-ecb'])

const dragStepId = ref<string | null>(null)
const dragOverStepId = ref<string | null>(null)

const categoryLabels: Record<string, string> = {
  encoding: '编码',
  symmetric: '对称加密',
  asymmetric: '非对称加密',
  compression: '压缩',
  bitwise: '位运算',
}

const codecGroups = computed(() => {
  const groups = new Map<string, typeof BUILTIN_CODECS[number][]>()
  for (const codec of BUILTIN_CODECS) {
    const items = groups.get(codec.category) ?? []
    items.push(codec)
    groups.set(codec.category, items)
  }
  return Array.from(groups.entries()).map(([category, items]) => ({
    category,
    label: categoryLabels[category] ?? category,
    items,
  }))
})

function createStep(codec = 'base64'): CodecStep {
  return {
    id: crypto.randomUUID(),
    type: 'builtin',
    codec,
    config: defaultConfigForCodec(codec),
    enabled: true,
  }
}

function defaultConfigForCodec(codec: string): Record<string, string> {
  if (codec === 'base64') {
    return { variant: 'standard' }
  }
  if (AES_CODECS.has(codec)) {
    return { key_format: 'utf8', iv_format: 'utf8' }
  }
  if (codec === 'xor') {
    return { key_format: 'utf8' }
  }
  return {}
}

function updateSteps(steps: CodecStep[]) {
  emit('update:modelValue', steps)
}

function updateStep(index: number, patch: Partial<CodecStep>) {
  updateSteps(props.modelValue.map((step, i) => (i === index ? { ...step, ...patch } : step)))
}

function onCodecChange(index: number, codec: string) {
  updateStep(index, { codec, config: defaultConfigForCodec(codec) })
}

function addStep() {
  updateSteps([...props.modelValue, createStep()])
}

function removeStep(index: number) {
  updateSteps(props.modelValue.filter((_, i) => i !== index))
}

function getConfigValue(step: CodecStep, key: string): string {
  return step.config[key] ?? ''
}

function setConfigValue(index: number, key: string, value: string) {
  const step = props.modelValue[index]
  if (!step) return
  updateStep(index, { config: { ...step.config, [key]: value } })
}

function isBase64Codec(codec: string): boolean {
  return codec === 'base64'
}

function isAesCodec(codec: string): boolean {
  return AES_CODECS.has(codec)
}

function hasNoConfig(codec: string): boolean {
  return NO_CONFIG_CODECS.has(codec)
}

function needsGenericConfig(codec: string): boolean {
  return !hasNoConfig(codec) && !isBase64Codec(codec) && !isAesCodec(codec) && codec !== 'xor'
}

function getGenericConfigEntries(step: CodecStep): Array<{ key: string; value: string }> {
  return Object.entries(step.config).map(([key, value]) => ({ key, value }))
}

function addGenericConfigEntry(index: number) {
  const step = props.modelValue[index]
  if (!step) return
  let suffix = 1
  let key = 'key'
  while (step.config[key] !== undefined) {
    suffix += 1
    key = `key${suffix}`
  }
  setConfigValue(index, key, '')
}

function removeGenericConfigKey(index: number, key: string) {
  const step = props.modelValue[index]
  if (!step) return
  const nextConfig = { ...step.config }
  delete nextConfig[key]
  updateStep(index, { config: nextConfig })
}

function renameGenericConfigKey(index: number, oldKey: string, newKey: string) {
  const step = props.modelValue[index]
  if (!step || oldKey === newKey) return
  const trimmed = newKey.trim()
  if (!trimmed || trimmed === oldKey) return
  const nextConfig = { ...step.config }
  nextConfig[trimmed] = nextConfig[oldKey] ?? ''
  delete nextConfig[oldKey]
  updateStep(index, { config: nextConfig })
}

function handleDragStart(stepId: string) {
  dragStepId.value = stepId
}

function handleDragEnter(stepId: string) {
  dragOverStepId.value = stepId
}

function handleDrop(targetStepId: string) {
  if (!dragStepId.value || dragStepId.value === targetStepId) {
    resetDragState()
    return
  }

  const steps = [...props.modelValue]
  const fromIndex = steps.findIndex(step => step.id === dragStepId.value)
  const toIndex = steps.findIndex(step => step.id === targetStepId)
  if (fromIndex < 0 || toIndex < 0) {
    resetDragState()
    return
  }

  const [moved] = steps.splice(fromIndex, 1)
  steps.splice(toIndex, 0, moved)
  updateSteps(steps)
  resetDragState()
}

function resetDragState() {
  dragStepId.value = null
  dragOverStepId.value = null
}
</script>
