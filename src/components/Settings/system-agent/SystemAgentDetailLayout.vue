<template>
  <div class="space-y-6">
    <slot name="toolbar" />
    <slot v-if="!hasSelection" name="empty" />
    <slot v-else />
  </div>
</template>

<script setup lang="ts">
defineProps<{
  hasSelection: boolean
}>()
</script>
