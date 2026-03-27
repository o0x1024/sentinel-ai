<template>
  <div class="form-control">
    <label class="label py-1">
      <span class="label-text-alt">目标资产类型</span>
      <span class="label-text-alt text-base-content/50">留空使用插件默认</span>
    </label>
    <div class="grid grid-cols-2 gap-2 sm:grid-cols-5">
      <label
        v-for="option in TARGET_ASSET_OPTIONS"
        :key="option.value"
        class="label cursor-pointer justify-start gap-2 rounded border border-base-300 px-2 py-2"
      >
        <input
          type="checkbox"
          class="checkbox checkbox-xs checkbox-primary"
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
  { value: 'web', label: 'Web URL' },
  { value: 'domain', label: '域名' },
  { value: 'host', label: '主机' },
  { value: 'ip', label: 'IP' },
  { value: 'service', label: '服务端点' },
]

const props = defineProps<{
  modelValue?: string[]
}>()

const emit = defineEmits<{
  'update:modelValue': [string[]]
}>()

const selectedValues = computed(() => {
  const values = Array.isArray(props.modelValue) ? props.modelValue : []
  return Array.from(
    new Set(
      values
        .map(value => String(value || '').trim().toLowerCase())
        .filter(Boolean)
    )
  )
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
