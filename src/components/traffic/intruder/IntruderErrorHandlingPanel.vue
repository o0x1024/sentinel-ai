<template>
  <div class="rounded-lg border border-base-300">
    <div class="border-b border-base-300 bg-base-200 px-4 py-2 text-xs font-semibold uppercase tracking-wide text-base-content/70">
      {{ $t('trafficAnalysis.intruder.labels.errorHandling') }}
    </div>

    <div class="space-y-3 p-4">
      <p class="text-sm text-base-content/70">
        {{ $t('trafficAnalysis.intruder.help.errorHandlingHint') }}
      </p>

      <div class="grid gap-3 md:grid-cols-3">
        <label class="form-control">
          <span class="label-text text-xs">{{ $t('trafficAnalysis.intruder.labels.retryCount') }}</span>
          <input
            :value="attackOptions.retryCount"
            type="number"
            min="0"
            max="10"
            class="input input-bordered input-sm"
            @input="emitOption('retryCount', clampNumber(($event.target as HTMLInputElement).value, 0, 10, 0))"
          />
        </label>

        <label class="form-control">
          <span class="label-text text-xs">{{ $t('trafficAnalysis.intruder.labels.retryPauseMs') }}</span>
          <input
            :value="attackOptions.retryPauseMs"
            type="number"
            min="0"
            max="10000"
            class="input input-bordered input-sm"
            @input="emitOption('retryPauseMs', clampNumber(($event.target as HTMLInputElement).value, 0, 10000, 0))"
          />
        </label>

        <label class="form-control">
          <span class="label-text text-xs">{{ $t('trafficAnalysis.intruder.labels.timeoutSecs') }}</span>
          <input
            :value="attackOptions.timeoutSecs"
            type="number"
            min="1"
            max="300"
            class="input input-bordered input-sm"
            @input="emitOption('timeoutSecs', clampNumber(($event.target as HTMLInputElement).value, 1, 300, 30))"
          />
        </label>

      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import type { IntruderAttackOptions } from './types'

const props = defineProps<{
  attackOptions: Pick<IntruderAttackOptions, 'retryCount' | 'retryPauseMs' | 'timeoutSecs'>
}>()

const emit = defineEmits<{
  (e: 'update:options', value: Partial<Pick<IntruderAttackOptions, 'retryCount' | 'retryPauseMs' | 'timeoutSecs'>>): void
}>()

function clampNumber(rawValue: string, min: number, max: number, fallback: number): number {
  const value = Number(rawValue)
  if (!Number.isFinite(value)) return fallback
  return Math.min(max, Math.max(min, value))
}

function emitOption<K extends keyof typeof props.attackOptions>(key: K, value: (typeof props.attackOptions)[K]) {
  emit('update:options', { [key]: value })
}
</script>
