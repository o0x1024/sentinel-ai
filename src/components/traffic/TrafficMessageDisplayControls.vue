<template>
  <div class="traffic-message-display-controls" role="toolbar" :aria-label="t('trafficAnalysis.httpEditor.toolbar.showLineEndings')">
    <button
      type="button"
      class="traffic-display-control-button"
      :class="{ active: settings.showLineEndings }"
      :title="lineEndingToggleTitle"
      @click="settings.showLineEndings = !settings.showLineEndings"
    >
      <span class="traffic-display-control-label">↵</span>
    </button>
    <button
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
  gap: 0.125rem;
}

.traffic-display-control-button {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 1.9rem;
  height: 1.9rem;
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
  font-size: 0.85rem;
  line-height: 1;
}
</style>
