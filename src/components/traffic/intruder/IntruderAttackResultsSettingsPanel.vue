<template>
  <div class="rounded-lg border border-base-300">
    <div class="border-b border-base-300 bg-base-200 px-4 py-2 text-xs font-semibold uppercase tracking-wide text-base-content/70">
      {{ $t('trafficAnalysis.intruder.labels.attackResults') }}
    </div>

    <div class="space-y-3 p-4">
      <p class="text-sm text-base-content/70">
        {{ $t('trafficAnalysis.intruder.help.attackResultsHint') }}
      </p>

      <label class="flex items-center gap-2">
        <input
          :checked="attackOptions.storeRequests"
          type="checkbox"
          class="checkbox checkbox-sm"
          @change="emitOption('storeRequests', ($event.target as HTMLInputElement).checked)"
        />
        <span>{{ $t('trafficAnalysis.intruder.labels.storeRequests') }}</span>
      </label>

      <label class="flex items-center gap-2">
        <input
          :checked="attackOptions.storeResponses"
          type="checkbox"
          class="checkbox checkbox-sm"
          @change="emitOption('storeResponses', ($event.target as HTMLInputElement).checked)"
        />
        <span>{{ $t('trafficAnalysis.intruder.labels.storeResponses') }}</span>
      </label>

      <label class="flex items-center gap-2">
        <input
          :checked="attackOptions.makeUnmodifiedBaseline"
          type="checkbox"
          class="checkbox checkbox-sm"
          @change="emitOption('makeUnmodifiedBaseline', ($event.target as HTMLInputElement).checked)"
        />
        <span>{{ $t('trafficAnalysis.intruder.labels.makeUnmodifiedBaseline') }}</span>
      </label>

      <label class="flex items-center gap-2">
        <input
          :checked="attackOptions.denialOfServiceMode"
          type="checkbox"
          class="checkbox checkbox-sm"
          @change="emitOption('denialOfServiceMode', ($event.target as HTMLInputElement).checked)"
        />
        <span>{{ $t('trafficAnalysis.intruder.labels.denialOfServiceMode') }}</span>
      </label>

      <label class="flex items-center gap-2">
        <input
          :checked="attackOptions.storeFullPayloads"
          type="checkbox"
          class="checkbox checkbox-sm"
          @change="emitOption('storeFullPayloads', ($event.target as HTMLInputElement).checked)"
        />
        <span>{{ $t('trafficAnalysis.intruder.labels.storeFullPayloads') }}</span>
      </label>
    </div>
  </div>
</template>

<script setup lang="ts">
import type { IntruderAttackOptions } from './types'

const props = defineProps<{
  attackOptions: Pick<
    IntruderAttackOptions,
    'storeRequests' | 'storeResponses' | 'makeUnmodifiedBaseline' | 'denialOfServiceMode' | 'storeFullPayloads'
  >
}>()

const emit = defineEmits<{
  (
    e: 'update:options',
    value: Partial<
      Pick<
        IntruderAttackOptions,
        'storeRequests' | 'storeResponses' | 'makeUnmodifiedBaseline' | 'denialOfServiceMode' | 'storeFullPayloads'
      >
    >,
  ): void
}>()

function emitOption<K extends keyof typeof props.attackOptions>(key: K, value: (typeof props.attackOptions)[K]) {
  emit('update:options', { [key]: value })
}
</script>
