<template>
  <div class="rounded-lg border border-base-300">
    <div class="border-b border-base-300 bg-base-200 px-4 py-2 text-xs font-semibold uppercase tracking-wide text-base-content/70">
      {{ $t('trafficAnalysis.intruder.labels.grepPayloads') }}
    </div>

    <div class="space-y-3 p-4 text-sm">
      <p class="text-base-content/70">
        {{ $t('trafficAnalysis.intruder.help.grepPayloadsHint') }}
      </p>

      <label class="flex items-center gap-2">
        <input
          :checked="settings.enabled"
          type="checkbox"
          class="checkbox checkbox-sm"
          @change="updateSettings({ enabled: ($event.target as HTMLInputElement).checked })"
        />
        <span>{{ $t('trafficAnalysis.intruder.labels.enableGrepPayloads') }}</span>
      </label>

      <div class="grid gap-3 md:grid-cols-2">
        <label class="flex items-center gap-2 rounded border border-base-300 px-3 py-2">
          <input
            :checked="settings.caseSensitive"
            type="checkbox"
            class="checkbox checkbox-sm"
            :disabled="!settings.enabled"
            @change="updateSettings({ caseSensitive: ($event.target as HTMLInputElement).checked })"
          />
          <span>{{ $t('trafficAnalysis.intruder.labels.caseSensitive') }}</span>
        </label>

        <label class="flex items-center gap-2 rounded border border-base-300 px-3 py-2">
          <input
            :checked="settings.excludeHeaders"
            type="checkbox"
            class="checkbox checkbox-sm"
            :disabled="!settings.enabled"
            @change="updateSettings({ excludeHeaders: ($event.target as HTMLInputElement).checked })"
          />
          <span>{{ $t('trafficAnalysis.intruder.labels.excludeResponseHeaders') }}</span>
        </label>

        <label class="flex items-center gap-2 rounded border border-base-300 px-3 py-2 md:col-span-2">
          <input
            :checked="settings.matchUrlEncoded"
            type="checkbox"
            class="checkbox checkbox-sm"
            :disabled="!settings.enabled"
            @change="updateSettings({ matchUrlEncoded: ($event.target as HTMLInputElement).checked })"
          />
          <span>{{ $t('trafficAnalysis.intruder.labels.matchUrlEncodedPayloads') }}</span>
        </label>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import type { IntruderGrepPayloadSettings } from './types'

const props = defineProps<{
  settings: IntruderGrepPayloadSettings
}>()

const emit = defineEmits<{
  (e: 'update:settings', value: IntruderGrepPayloadSettings): void
}>()

function updateSettings(patch: Partial<IntruderGrepPayloadSettings>) {
  emit('update:settings', {
    ...props.settings,
    ...patch,
  })
}
</script>
