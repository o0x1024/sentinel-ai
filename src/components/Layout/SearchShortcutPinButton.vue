<template>
  <button
    class="btn btn-ghost btn-xs btn-circle"
    :class="pinned ? 'text-warning' : 'text-base-content/40 hover:text-warning'"
    :aria-label="pinned ? '取消固定快捷入口' : '固定为快捷入口'"
    @click.stop="$emit('toggle')"
  >
    <i :class="pinned ? 'fas fa-star' : 'far fa-star'"></i>
  </button>
</template>

<script setup lang="ts">
defineProps<{
  pinned: boolean
}>()

defineEmits<{
  toggle: []
}>()
</script>
