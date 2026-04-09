<template>
  <button
    class="btn btn-ghost btn-xs btn-circle text-base-content/40 hover:text-primary"
    aria-label="编辑固定快捷入口"
    @click.stop="$emit('edit')"
  >
    <i class="fas fa-pen"></i>
  </button>
</template>

<script setup lang="ts">
defineEmits<{
  edit: []
}>()
</script>
