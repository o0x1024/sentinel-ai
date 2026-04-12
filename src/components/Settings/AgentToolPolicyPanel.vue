<template>
  <div class="rounded-lg border border-base-300 bg-base-100 p-4">
    <div class="flex items-start justify-between gap-3">
      <div>
        <div class="text-sm font-semibold">{{ title }}</div>
        <div class="mt-1 text-xs text-base-content/60">
          {{ description }}
        </div>
      </div>
      <div v-if="$slots.actions" class="shrink-0">
        <slot name="actions" />
      </div>
    </div>

    <div v-if="$slots.summary" class="mt-4 flex flex-wrap gap-2">
      <slot name="summary" />
    </div>

    <div class="mt-4">
      <slot />
    </div>
  </div>
</template>

<script setup lang="ts">
withDefaults(defineProps<{
  title?: string
  description?: string
}>(), {
  title: '工具策略',
  description: '约束 Agent 可调用的工具范围与选择方式。',
})
</script>
