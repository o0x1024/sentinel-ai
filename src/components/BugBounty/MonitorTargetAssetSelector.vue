<template>
  <div class="form-control">
    <label class="label py-1">
      <span class="label-text-alt">目标资产类型</span>
      <span class="label-text-alt text-base-content/50">留空使用插件默认</span>
    </label>
    <div class="grid grid-cols-2 gap-2 md:grid-cols-3 xl:grid-cols-5">
      <label
        v-for="option in visibleOptions"
        :key="option.value"
        class="label cursor-pointer justify-start gap-2 rounded border border-base-300 px-2 py-2"
      >
        <input
          type="checkbox"
          class="checkbox checkbox-sm checkbox-primary rounded-sm"
          :checked="selectedValues.includes(option.value)"
          @change="toggleAssetType(option.value, ($event.target as HTMLInputElement).checked)"
        />
        <span class="text-xs">{{ option.label }}</span>
      </label>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'

const TARGET_ASSET_OPTIONS = [
  { value: 'web', label: '网站' },
  { value: 'domain', label: '域名' },
  { value: 'domain_root', label: '根' },
  { value: 'domain_level_1', label: '一级' },
  { value: 'domain_level_2', label: '二级' },
  { value: 'domain_level_3_plus', label: '三级及以上' },
  { value: 'host', label: '主机' },
  { value: 'ip', label: 'IP' },
  { value: 'service', label: '服务端点' },
]

const props = defineProps<{
  modelValue?: string[]
  allowedValues?: string[]
}>()

const emit = defineEmits<{
  'update:modelValue': [string[]]
}>()

const allowedValueSet = computed(() => {
  const values = Array.isArray(props.allowedValues) ? props.allowedValues : []
  const normalized = values
    .map(value => String(value || '').trim().toLowerCase())
    .filter(Boolean)
  return new Set(normalized)
})

const visibleOptions = computed(() => {
  if (allowedValueSet.value.size === 0) {
    return TARGET_ASSET_OPTIONS
  }

  return TARGET_ASSET_OPTIONS.filter(option => allowedValueSet.value.has(option.value))
})

const selectedValues = computed(() => {
  const values = Array.isArray(props.modelValue) ? props.modelValue : []
  const normalized = Array.from(
    new Set(
      values
        .map(value => String(value || '').trim().toLowerCase())
        .filter(Boolean)
    )
  )

  if (allowedValueSet.value.size === 0) {
    return normalized
  }

  return normalized.filter(value => allowedValueSet.value.has(value))
})

const toggleAssetType = (assetType: string, checked: boolean) => {
  const next = new Set(selectedValues.value)
  if (checked) {
    next.add(assetType)
  } else {
    next.delete(assetType)
  }
  emit('update:modelValue', Array.from(next))
}
</script>
