<template>
  <div class="flex flex-wrap items-center justify-end gap-2">
    <button class="btn btn-sm btn-outline" @click="$emit('seed')" :disabled="loading">
      <i class="fas fa-seedling mr-1"></i>
      初始化默认 Agent
    </button>
    <button class="btn btn-sm btn-ghost" @click="$emit('refresh')" :disabled="loading">
      <i class="fas fa-rotate mr-1"></i>
      刷新
    </button>
  </div>
</template>

<script setup lang="ts">
defineProps<{
  loading: boolean
}>()

defineEmits<{
  seed: []
  refresh: []
}>()
</script>
