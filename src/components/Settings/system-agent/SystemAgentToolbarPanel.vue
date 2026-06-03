<template>
  <div class="flex flex-wrap items-center justify-end gap-2">
    <div class="mr-auto text-xs text-base-content/60">
      已启用 {{ enabledCount }} / {{ totalCount }}
    </div>
    <button
      class="btn btn-sm btn-primary"
      @click="$emit('enable-all')"
      :disabled="loading || bulkMutating || enabledCount === totalCount"
    >
      <span v-if="bulkMutating" class="loading loading-spinner loading-xs"></span>
      <i v-else class="fas fa-toggle-on mr-1"></i>
      全部开启
    </button>
    <button
      class="btn btn-sm btn-outline"
      @click="$emit('disable-all')"
      :disabled="loading || bulkMutating || enabledCount === 0"
    >
      <span v-if="bulkMutating" class="loading loading-spinner loading-xs"></span>
      <i v-else class="fas fa-toggle-off mr-1"></i>
      全部关闭
    </button>
    <button class="btn btn-sm btn-outline" @click="$emit('seed')" :disabled="loading || bulkMutating">
      <i class="fas fa-seedling mr-1"></i>
      初始化默认后台 Agent
    </button>
    <button class="btn btn-sm btn-ghost" @click="$emit('refresh')" :disabled="loading || bulkMutating">
      <i class="fas fa-rotate mr-1"></i>
      刷新
    </button>
  </div>
</template>

<script setup lang="ts">
defineProps<{
  loading: boolean
  bulkMutating: boolean
  enabledCount: number
  totalCount: number
}>()

defineEmits<{
  'enable-all': []
  'disable-all': []
  seed: []
  refresh: []
}>()
</script>
