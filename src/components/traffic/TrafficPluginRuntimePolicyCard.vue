<template>
  <div class="rounded-lg border border-base-300 p-4 space-y-4">
    <div>
      <h4 class="font-medium">{{ title }}</h4>
      <p v-if="description" class="mt-1 text-xs text-base-content/60">{{ description }}</p>
    </div>

    <div class="grid gap-4 md:grid-cols-2 xl:grid-cols-3">
      <label class="form-control">
        <span class="label-text text-xs">{{
          $t('trafficAnalysis.proxyConfiguration.fetchMaxQueueDepth')
        }}</span>
        <input
          :value="policy.maxQueueDepth"
          type="number"
          min="1"
          max="5000"
          class="input input-bordered"
          @input="updateNumberField('maxQueueDepth', $event)"
        />
      </label>

      <label class="form-control">
        <span class="label-text text-xs">{{
          $t('trafficAnalysis.proxyConfiguration.fetchMaxPendingPerRun')
        }}</span>
        <input
          :value="policy.maxPendingPerRun"
          type="number"
          min="1"
          max="2000"
          class="input input-bordered"
          @input="updateNumberField('maxPendingPerRun', $event)"
        />
      </label>

      <label class="form-control">
        <span class="label-text text-xs">{{
          $t('trafficAnalysis.proxyConfiguration.fetchMaxPendingPerPlugin')
        }}</span>
        <input
          :value="policy.maxPendingPerPlugin"
          type="number"
          min="1"
          max="5000"
          class="input input-bordered"
          @input="updateNumberField('maxPendingPerPlugin', $event)"
        />
      </label>

      <label class="form-control">
        <span class="label-text text-xs">{{
          $t('trafficAnalysis.proxyConfiguration.fetchMaxGlobalConcurrent')
        }}</span>
        <input
          :value="policy.maxGlobalConcurrent"
          type="number"
          min="1"
          max="128"
          class="input input-bordered"
          @input="updateNumberField('maxGlobalConcurrent', $event)"
        />
      </label>

      <label class="form-control">
        <span class="label-text text-xs">{{
          $t('trafficAnalysis.proxyConfiguration.fetchMaxConcurrentPerHost')
        }}</span>
        <input
          :value="policy.maxConcurrentPerHost"
          type="number"
          min="1"
          max="32"
          class="input input-bordered"
          @input="updateNumberField('maxConcurrentPerHost', $event)"
        />
      </label>

      <label class="form-control">
        <span class="label-text text-xs">{{
          $t('trafficAnalysis.proxyConfiguration.fetchMaxConcurrentPerRun')
        }}</span>
        <input
          :value="policy.maxConcurrentPerRun"
          type="number"
          min="1"
          max="128"
          class="input input-bordered"
          @input="updateNumberField('maxConcurrentPerRun', $event)"
        />
      </label>

      <label class="form-control">
        <span class="label-text text-xs">{{
          $t('trafficAnalysis.proxyConfiguration.fetchMaxConcurrentPerPlugin')
        }}</span>
        <input
          :value="policy.maxConcurrentPerPlugin"
          type="number"
          min="1"
          max="128"
          class="input input-bordered"
          @input="updateNumberField('maxConcurrentPerPlugin', $event)"
        />
      </label>

      <label class="form-control">
        <span class="label-text text-xs">{{ delayLabel }}</span>
        <input
          :value="policy[delayField]"
          type="number"
          min="0"
          max="60000"
          class="input input-bordered"
          @input="updateDelayField($event)"
        />
      </label>

      <label class="form-control">
        <span class="label-text text-xs">{{
          $t('trafficAnalysis.proxyConfiguration.fetchTimeoutMs')
        }}</span>
        <input
          :value="policy.timeoutMs"
          type="number"
          min="1000"
          max="120000"
          class="input input-bordered"
          @input="updateNumberField('timeoutMs', $event)"
        />
      </label>

      <label class="form-control">
        <span class="label-text text-xs">{{
          $t('trafficAnalysis.proxyConfiguration.fetchJitterMinMs')
        }}</span>
        <input
          :value="policy.jitterRange[0]"
          type="number"
          min="0"
          max="30000"
          class="input input-bordered"
          @input="updateJitterField(0, $event)"
        />
      </label>

      <label class="form-control">
        <span class="label-text text-xs">{{
          $t('trafficAnalysis.proxyConfiguration.fetchJitterMaxMs')
        }}</span>
        <input
          :value="policy.jitterRange[1]"
          type="number"
          min="0"
          max="30000"
          class="input input-bordered"
          @input="updateJitterField(1, $event)"
        />
      </label>
    </div>
  </div>
</template>

<script setup lang="ts">
import type { TrafficPluginRuntimePolicySettings } from './pluginRuntimeSettingsSupport'

const props = defineProps<{
  title: string
  description?: string
  delayLabel: string
  policy: TrafficPluginRuntimePolicySettings
  delayField: 'minHostCooldownMs' | 'minHostDelayMs'
}>()

const emit = defineEmits<{
  'update:policy': [value: TrafficPluginRuntimePolicySettings]
}>()

function readInputNumber(event: Event): number {
  return Number((event.target as HTMLInputElement).value)
}

function emitPolicy(patch: Partial<TrafficPluginRuntimePolicySettings>) {
  emit('update:policy', {
    ...props.policy,
    ...patch,
    jitterRange: [...(patch.jitterRange || props.policy.jitterRange)] as [number, number],
  })
}

function updateNumberField(field: keyof TrafficPluginRuntimePolicySettings, event: Event) {
  emitPolicy({
    [field]: readInputNumber(event),
  } as Partial<TrafficPluginRuntimePolicySettings>)
}

function updateDelayField(event: Event) {
  emitPolicy({
    [props.delayField]: readInputNumber(event),
  } as Partial<TrafficPluginRuntimePolicySettings>)
}

function updateJitterField(index: 0 | 1, event: Event) {
  const jitterRange = [...props.policy.jitterRange] as [number, number]
  jitterRange[index] = readInputNumber(event)
  emitPolicy({ jitterRange })
}
</script>
