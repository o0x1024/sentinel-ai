<template>
  <div class="rounded-lg border border-base-300 bg-base-100 p-2">
    <div class="tabs tabs-boxed gap-2 bg-transparent" role="tablist" aria-label="智能体工作区标签">
      <button
        v-for="item in items"
        :key="item.key"
        class="tab h-auto min-h-11 flex-1 justify-between px-4 py-2"
        :class="modelValue === item.key ? 'tab-active' : ''"
        type="button"
        @click="$emit('update:modelValue', item.key)"
      >
        <span>{{ item.label }}</span>
        <span
          v-if="typeof item.count === 'number'"
          class="badge badge-sm"
          :class="modelValue === item.key ? 'badge-primary' : 'badge-ghost'"
        >
          {{ item.count }}
        </span>
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
defineProps<{
  modelValue: string
  items: Array<{
    key: string
    label: string
    count?: number
  }>
}>()

defineEmits<{
  'update:modelValue': [value: string]
}>()
</script>
