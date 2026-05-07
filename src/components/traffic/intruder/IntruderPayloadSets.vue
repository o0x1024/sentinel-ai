<template>
  <section class="flex h-full flex-col rounded-lg border border-base-300 bg-base-100">
    <header class="border-b border-base-300 bg-base-200 px-4 py-3">
      <div class="flex flex-wrap items-center gap-3">
        <div class="min-w-0 flex-1">
          <h3 class="text-sm font-semibold">{{ $t('trafficAnalysis.intruder.sections.payloads') }}</h3>
          <p class="text-xs text-base-content/70">
            {{ $t('trafficAnalysis.intruder.help.attackTypeHint') }}
          </p>
        </div>

        <div class="badge badge-outline">
          {{ $t('trafficAnalysis.intruder.labels.estimatedRequests') }}: {{ estimatedRequests }}
        </div>

      </div>
    </header>

    <div class="space-y-4 overflow-auto p-4">
      <div class="grid gap-3 md:grid-cols-2">
        <label class="form-control">
          <span class="label-text text-xs">{{ $t('trafficAnalysis.intruder.labels.attackType') }}</span>
          <IntruderAttackTypeSelect
            :model-value="attackType"
            @update:model-value="$emit('update:attackType', $event)"
          />
        </label>

        <div class="flex items-end">
          <button class="btn btn-sm btn-ghost" type="button" @click="$emit('syncPayloadSets')">
            <i class="fas fa-layer-group"></i>
            {{ $t('trafficAnalysis.intruder.actions.syncPayloadSets') }}
          </button>
        </div>
      </div>

      <div class="grid gap-3 md:grid-cols-2">
        <label class="form-control">
          <span class="label-text text-xs">{{ $t('trafficAnalysis.intruder.labels.concurrency') }}</span>
          <input
            :value="attackOptions.concurrency"
            type="number"
            min="1"
            max="20"
            class="input input-bordered input-sm"
            @input="updateOption('concurrency', clampNumber(($event.target as HTMLInputElement).value, 1, 20, 1))"
          />
        </label>

        <label class="form-control">
          <span class="label-text text-xs">{{ $t('trafficAnalysis.intruder.labels.delayMs') }}</span>
          <input
            :value="attackOptions.delayMs"
            type="number"
            min="0"
            max="10000"
            class="input input-bordered input-sm"
            @input="updateOption('delayMs', clampNumber(($event.target as HTMLInputElement).value, 0, 10000, 0))"
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
            @input="updateOption('timeoutSecs', clampNumber(($event.target as HTMLInputElement).value, 1, 300, 30))"
          />
        </label>

      </div>

      <article v-for="(payloadSet, index) in payloadSets" :key="payloadSet.id" class="rounded-lg border border-base-300 bg-base-100">
        <header class="flex items-center gap-3 border-b border-base-300 bg-base-200 px-4 py-3">
          <div class="min-w-0 flex-1">
            <input
              :value="payloadSet.name"
              type="text"
              class="input input-ghost input-sm w-full px-0 font-semibold"
              :placeholder="$t('trafficAnalysis.intruder.placeholders.payloadSetName', { index: index + 1 })"
              @input="$emit('updatePayloadSet', payloadSet.id, { name: ($event.target as HTMLInputElement).value })"
            />
          </div>

          <label class="flex items-center gap-2 text-xs">
            <input
              :checked="payloadSet.urlEncode"
              type="checkbox"
              class="checkbox checkbox-xs"
              @change="$emit('updatePayloadSet', payloadSet.id, { urlEncode: ($event.target as HTMLInputElement).checked })"
            />
            <span>{{ $t('trafficAnalysis.intruder.labels.urlEncode') }}</span>
          </label>

          <button
            v-if="payloadSets.length > 1"
            class="btn btn-ghost btn-xs text-error"
            type="button"
            @click="$emit('removePayloadSet', payloadSet.id)"
          >
            <i class="fas fa-trash"></i>
          </button>
        </header>

        <div class="p-4">
          <textarea
            :value="payloadSet.payloadsText"
            class="h-40 w-full resize-y rounded-lg border border-base-300 bg-base-100 p-3 font-mono text-xs leading-6 outline-none transition focus:border-primary"
            :placeholder="$t('trafficAnalysis.intruder.placeholders.payloads')"
            @input="$emit('updatePayloadSet', payloadSet.id, { payloadsText: ($event.target as HTMLTextAreaElement).value })"
          ></textarea>
        </div>
      </article>

      <button class="btn btn-sm btn-outline w-full" type="button" @click="$emit('addPayloadSet')">
        <i class="fas fa-plus"></i>
        {{ $t('trafficAnalysis.intruder.actions.addPayloadSet') }}
      </button>
    </div>
  </section>
</template>

<script setup lang="ts">
import IntruderAttackTypeSelect from './IntruderAttackTypeSelect.vue'
import type { IntruderAttackOptions, IntruderAttackType, IntruderPayloadSet } from './types'

const props = defineProps<{
  attackType: IntruderAttackType
  payloadSets: IntruderPayloadSet[]
  attackOptions: IntruderAttackOptions
  estimatedRequests: number
}>()

const emit = defineEmits<{
  (e: 'update:attackType', value: IntruderAttackType): void
  (e: 'update:attackOptions', value: IntruderAttackOptions): void
  (e: 'updatePayloadSet', id: string, patch: Partial<IntruderPayloadSet>): void
  (e: 'addPayloadSet'): void
  (e: 'removePayloadSet', id: string): void
  (e: 'syncPayloadSets'): void
}>()

function clampNumber(rawValue: string, min: number, max: number, fallback: number): number {
  const value = Number(rawValue)
  if (!Number.isFinite(value)) return fallback
  return Math.min(max, Math.max(min, value))
}

function updateOption<K extends keyof IntruderAttackOptions>(key: K, value: IntruderAttackOptions[K]) {
  emit('update:attackOptions', {
    ...props.attackOptions,
    [key]: value,
  })
}
</script>
