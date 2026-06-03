<template>
  <div class="rounded-lg border border-base-300">
    <div class="border-b border-base-300 bg-base-200 px-4 py-2 text-xs font-semibold uppercase tracking-wide text-base-content/70">
      {{ $t('trafficAnalysis.intruder.labels.redirectHandling') }}
    </div>

    <div class="space-y-4 p-4 text-sm">
      <p class="text-base-content/70">
        {{ $t('trafficAnalysis.intruder.help.redirectHandlingHint') }}
      </p>

      <label class="flex items-center gap-2">
        <input
          :checked="attackOptions.followRedirects"
          type="checkbox"
          class="checkbox checkbox-sm"
          @change="emitOption('followRedirects', ($event.target as HTMLInputElement).checked)"
        />
        <span>{{ $t('trafficAnalysis.intruder.labels.followRedirects') }}</span>
      </label>

      <div class="grid gap-3 md:grid-cols-2">
        <label class="form-control">
          <span class="label-text text-xs">{{ $t('trafficAnalysis.intruder.labels.maxRedirects') }}</span>
          <input
            :value="attackOptions.maxRedirects"
            type="number"
            min="0"
            max="20"
            class="input input-bordered input-sm"
            :disabled="!attackOptions.followRedirects"
            @input="emitOption('maxRedirects', clampNumber(($event.target as HTMLInputElement).value, 0, 20, 5))"
          />
        </label>

        <label class="flex items-center gap-2 rounded border border-base-300 px-3 py-2">
          <input
            :checked="attackOptions.processCookiesInRedirects"
            type="checkbox"
            class="checkbox checkbox-sm"
            :disabled="!attackOptions.followRedirects"
            @change="emitOption('processCookiesInRedirects', ($event.target as HTMLInputElement).checked)"
          />
          <span>{{ $t('trafficAnalysis.intruder.labels.processCookiesInRedirects') }}</span>
        </label>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import type { IntruderAttackOptions } from './types'

const props = defineProps<{
  attackOptions: Pick<
    IntruderAttackOptions,
    'followRedirects' | 'maxRedirects' | 'processCookiesInRedirects'
  >
}>()

const emit = defineEmits<{
  (
    e: 'update:options',
    value: Partial<
      Pick<IntruderAttackOptions, 'followRedirects' | 'maxRedirects' | 'processCookiesInRedirects'>
    >,
  ): void
}>()

function emitOption<K extends keyof typeof props.attackOptions>(
  key: K,
  value: (typeof props.attackOptions)[K],
) {
  emit('update:options', { [key]: value })
}

function clampNumber(value: string, min: number, max: number, fallback: number): number {
  const parsed = Number(value)
  if (!Number.isFinite(parsed)) return fallback
  return Math.max(min, Math.min(max, parsed))
}
</script>
