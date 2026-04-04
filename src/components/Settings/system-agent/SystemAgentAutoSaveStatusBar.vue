<template>
  <div class="text-sm" :class="statusClass">
    {{ statusText }}
  </div>
</template>

<script setup lang="ts">
defineProps<{
  statusText: string
  statusClass: string
}>()
</script>
