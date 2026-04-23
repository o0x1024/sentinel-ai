<template>
  <div class="traffic-message-view-tabs" :class="{ compact }" role="tablist">
    <button
      v-for="tab in tabs"
      :key="tab.value"
      type="button"
      class="traffic-message-view-tab"
      :class="{ active: modelValue === tab.value }"
      :title="tab.label"
      :aria-label="tab.label"
      :aria-selected="modelValue === tab.value"
      @click="emit('update:modelValue', tab.value)"
    >
      {{ compact && tab.shortLabel ? tab.shortLabel : tab.label }}
    </button>
  </div>
</template>

<script setup lang="ts">
export interface TrafficMessageViewTabOption {
  value: string
  label: string
  shortLabel?: string
}

withDefaults(defineProps<{
  modelValue: string
  tabs: TrafficMessageViewTabOption[]
  compact?: boolean
}>(), {
  compact: false,
})

const emit = defineEmits<{
  (e: 'update:modelValue', value: string): void
}>()
</script>

<style scoped>
.traffic-message-view-tabs {
  display: inline-flex;
  align-items: stretch;
  gap: 0.15rem;
  min-width: 0;
  max-width: 100%;
  overflow-x: auto;
  scrollbar-width: thin;
}

.traffic-message-view-tab {
  display: inline-flex;
  align-items: center;
  flex: 0 0 auto;
  height: 2rem;
  padding: 0 0.8rem;
  border: none;
  border-bottom: 2px solid transparent;
  background: transparent;
  color: oklch(var(--bc) / 0.72);
  font-size: 0.8125rem;
  font-weight: 500;
  white-space: nowrap;
  transition: color 120ms ease, border-color 120ms ease, background-color 120ms ease;
}

.traffic-message-view-tabs.compact .traffic-message-view-tab {
  padding: 0 0.55rem;
  font-size: 0.75rem;
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
