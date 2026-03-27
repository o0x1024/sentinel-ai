<template>
  <div class="rounded-lg border border-base-300">
    <div class="border-b border-base-300 bg-base-200 px-4 py-2 text-xs font-semibold uppercase tracking-wide text-base-content/70">
      {{ $t('trafficAnalysis.intruder.labels.requestHeaders') }}
    </div>

    <div class="space-y-3 p-4">
      <p class="text-sm text-base-content/70">
        {{ $t('trafficAnalysis.intruder.help.requestHeadersHint') }}
      </p>

      <label class="flex items-center gap-2">
        <input
          :checked="attackOptions.updateContentLength"
          type="checkbox"
          class="checkbox checkbox-sm"
          @change="emitOption('updateContentLength', ($event.target as HTMLInputElement).checked)"
        />
        <span>{{ $t('trafficAnalysis.intruder.labels.updateContentLengthHeader') }}</span>
      </label>

      <label class="flex items-center gap-2">
        <input
          :checked="attackOptions.setConnectionClose"
          type="checkbox"
          class="checkbox checkbox-sm"
          @change="emitOption('setConnectionClose', ($event.target as HTMLInputElement).checked)"
        />
        <span>{{ $t('trafficAnalysis.intruder.labels.setConnectionHeader') }}</span>
      </label>
    </div>
  </div>
</template>

<script setup lang="ts">
import type { IntruderAttackOptions } from './types'

const props = defineProps<{
  attackOptions: Pick<IntruderAttackOptions, 'updateContentLength' | 'setConnectionClose'>
}>()

const emit = defineEmits<{
  (e: 'update:options', value: Partial<Pick<IntruderAttackOptions, 'updateContentLength' | 'setConnectionClose'>>): void
}>()

function emitOption<K extends keyof typeof props.attackOptions>(key: K, value: (typeof props.attackOptions)[K]) {
  emit('update:options', { [key]: value })
}
</script>
