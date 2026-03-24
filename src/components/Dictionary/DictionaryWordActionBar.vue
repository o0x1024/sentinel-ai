<template>
  <div class="flex justify-between items-center mt-4">
    <div class="flex gap-2">
      <button
        v-if="isStructured && selectedCount > 0"
        class="btn btn-info btn-sm"
        @click="emit('batch-edit')"
      >
        <i class="fas fa-layer-group mr-2"></i>
        批量编辑 ({{ selectedCount }})
      </button>
      <button
        v-if="isStructured && selectedCount > 0"
        class="btn btn-outline btn-sm"
        @click="emit('export-selected')"
      >
        <i class="fas fa-download mr-2"></i>
        导出选中 ({{ selectedCount }})
      </button>
      <button
        v-if="selectedCount > 0"
        class="btn btn-error btn-sm"
        @click="emit('remove-selected')"
      >
        <i class="fas fa-trash mr-2"></i>
        删除选中 ({{ selectedCount }})
      </button>
      <button class="btn btn-warning btn-sm" @click="emit('clear')">
        <i class="fas fa-broom mr-2"></i>
        清空字典
      </button>
    </div>

    <div class="modal-action">
      <button class="btn" @click="emit('close')">关闭</button>
    </div>
  </div>
</template>

<script setup lang="ts">
defineProps<{
  isStructured: boolean
  selectedCount: number
}>()

const emit = defineEmits<{
  (e: 'batch-edit'): void
  (e: 'export-selected'): void
  (e: 'remove-selected'): void
  (e: 'clear'): void
  (e: 'close'): void
}>()
</script>
