<template>
  <div class="traffic-message-view-tabs" role="tablist">
    <button
      v-for="tab in tabs"
      :key="tab.value"
      type="button"
      class="traffic-message-view-tab"
      :class="{ active: modelValue === tab.value }"
      :aria-selected="modelValue === tab.value"
      @click="emit('update:modelValue', tab.value)"
    >
      {{ tab.label }}
    </button>
  </div>
</template>

<script setup lang="ts">
export interface TrafficMessageViewTabOption {
  value: string
  label: string
}

defineProps<{
  modelValue: string
  tabs: TrafficMessageViewTabOption[]
}>()

const emit = defineEmits<{
  (e: 'update:modelValue', value: string): void
}>()
</script>

<style scoped>
.traffic-message-view-tabs {
  display: inline-flex;
  align-items: stretch;
  gap: 0.15rem;
}

.traffic-message-view-tab {
  display: inline-flex;
  align-items: center;
  height: 2rem;
  padding: 0 0.8rem;
  border: none;
  border-bottom: 2px solid transparent;
  background: transparent;
  color: oklch(var(--bc) / 0.72);
  font-size: 0.8125rem;
  font-weight: 500;
  transition: color 120ms ease, border-color 120ms ease, background-color 120ms ease;
}

.traffic-message-view-tab:hover {
  color: oklch(var(--bc) / 0.96);
  background: oklch(var(--b2) / 0.55);
}

.traffic-message-view-tab.active {
  color: oklch(var(--bc));
  border-bottom-color: #f97316;
}
</style>
