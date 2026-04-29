<template>
  <div
    class="traffic-message-display-controls"
    :class="{ compact }"
    role="toolbar"
    :aria-label="t('trafficAnalysis.httpEditor.toolbar.showLineEndings')"
  >
    <span v-if="modeLabel" class="traffic-display-mode-badge">
      {{ modeLabel }}
    </span>
    <button
      v-if="showLineEndings"
      type="button"
      class="traffic-display-control-button"
      :class="{ active: settings.showLineEndings }"
      :title="lineEndingToggleTitle"
      @click="settings.showLineEndings = !settings.showLineEndings"
    >
      <span class="traffic-display-control-label">↵</span>
    </button>
    <button
      v-if="showLineWrap"
      type="button"
      class="traffic-display-control-button"
      :class="{ active: settings.wrapLongLines }"
      :title="lineWrapToggleTitle"
      @click="settings.wrapLongLines = !settings.wrapLongLines"
    >
      <i class="fas fa-text-width"></i>
    </button>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { useTrafficDisplaySettings } from './trafficDisplaySettings'

withDefaults(defineProps<{
  modeLabel?: string
  showLineEndings?: boolean
  showLineWrap?: boolean
  compact?: boolean
}>(), {
  modeLabel: '',
  showLineEndings: true,
  showLineWrap: true,
  compact: false,
})

const { t } = useI18n()
const { settings } = useTrafficDisplaySettings()

const lineEndingToggleTitle = computed(() => (
  settings.value.showLineEndings
    ? t('trafficAnalysis.httpEditor.toolbar.hideLineEndings')
    : t('trafficAnalysis.httpEditor.toolbar.showLineEndings')
))

const lineWrapToggleTitle = computed(() => (
  settings.value.wrapLongLines
    ? t('trafficAnalysis.httpEditor.toolbar.disableLineWrap')
    : t('trafficAnalysis.httpEditor.toolbar.enableLineWrap')
))
</script>

<style scoped>
.traffic-message-display-controls {
  display: inline-flex;
  align-items: center;
  flex: 0 0 auto;
  gap: 0.1rem;
  min-width: 0;
  overflow-x: auto;
  scrollbar-width: thin;
}

.traffic-message-display-controls.compact {
  gap: 0.1rem;
}

.traffic-display-mode-badge {
  display: inline-flex;
  align-items: center;
  height: 1.45rem;
  padding: 0 0.45rem;
  border: 1px solid oklch(var(--b3));
  border-radius: 999px;
  background: oklch(var(--b2) / 0.55);
  color: oklch(var(--bc) / 0.72);
  font-size: 0.68rem;
  font-weight: 600;
  letter-spacing: 0.02em;
  text-transform: uppercase;
}

.traffic-display-control-button {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 1.7rem;
  height: 1.7rem;
  border: none;
  border-radius: 0.4rem;
  background: transparent;
  color: oklch(var(--bc) / 0.7);
  transition: background-color 120ms ease, color 120ms ease;
}

.traffic-display-control-button:hover {
  background: oklch(var(--b3) / 0.8);
  color: oklch(var(--bc) / 0.92);
}

.traffic-display-control-button.active {
  background: oklch(var(--p) / 0.18);
  color: oklch(var(--p));
}

.traffic-display-control-label {
  font-size: 0.78rem;
  line-height: 1;
}
</style>
