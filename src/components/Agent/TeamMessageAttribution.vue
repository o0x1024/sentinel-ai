<template>
  <div class="inline-flex items-center gap-1 text-xs font-medium text-primary">
    <i class="fas fa-users"></i>
    <span>{{ label }}</span>
  </div>
</template>

<script setup lang="ts">
defineProps<{
  label: string
}>()
</script>
