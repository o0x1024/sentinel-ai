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
          v-model.number="policy.maxQueueDepth"
          type="number"
          min="1"
          max="5000"
          class="input input-bordered"
        />
      </label>

      <label class="form-control">
        <span class="label-text text-xs">{{
          $t('trafficAnalysis.proxyConfiguration.fetchMaxPendingPerRun')
        }}</span>
        <input
          v-model.number="policy.maxPendingPerRun"
          type="number"
          min="1"
          max="2000"
          class="input input-bordered"
        />
      </label>

      <label class="form-control">
        <span class="label-text text-xs">{{
          $t('trafficAnalysis.proxyConfiguration.fetchMaxPendingPerPlugin')
        }}</span>
        <input
          v-model.number="policy.maxPendingPerPlugin"
          type="number"
          min="1"
          max="5000"
          class="input input-bordered"
        />
      </label>

      <label class="form-control">
        <span class="label-text text-xs">{{
          $t('trafficAnalysis.proxyConfiguration.fetchMaxGlobalConcurrent')
        }}</span>
        <input
          v-model.number="policy.maxGlobalConcurrent"
          type="number"
          min="1"
          max="128"
          class="input input-bordered"
        />
      </label>

      <label class="form-control">
        <span class="label-text text-xs">{{
          $t('trafficAnalysis.proxyConfiguration.fetchMaxConcurrentPerHost')
        }}</span>
        <input
          v-model.number="policy.maxConcurrentPerHost"
          type="number"
          min="1"
          max="32"
          class="input input-bordered"
        />
      </label>

      <label class="form-control">
        <span class="label-text text-xs">{{
          $t('trafficAnalysis.proxyConfiguration.fetchMaxConcurrentPerRun')
        }}</span>
        <input
          v-model.number="policy.maxConcurrentPerRun"
          type="number"
          min="1"
          max="128"
          class="input input-bordered"
        />
      </label>

      <label class="form-control">
        <span class="label-text text-xs">{{
          $t('trafficAnalysis.proxyConfiguration.fetchMaxConcurrentPerPlugin')
        }}</span>
        <input
          v-model.number="policy.maxConcurrentPerPlugin"
          type="number"
          min="1"
          max="128"
          class="input input-bordered"
        />
      </label>

      <label class="form-control">
        <span class="label-text text-xs">{{ delayLabel }}</span>
        <input
          v-model.number="delayModel"
          type="number"
          min="0"
          max="60000"
          class="input input-bordered"
        />
      </label>

      <label class="form-control">
        <span class="label-text text-xs">{{
          $t('trafficAnalysis.proxyConfiguration.fetchTimeoutMs')
        }}</span>
        <input
          v-model.number="policy.timeoutMs"
          type="number"
          min="1000"
          max="120000"
          class="input input-bordered"
        />
      </label>

      <label class="form-control">
        <span class="label-text text-xs">{{
          $t('trafficAnalysis.proxyConfiguration.fetchJitterMinMs')
        }}</span>
        <input
          v-model.number="policy.jitterRange[0]"
          type="number"
          min="0"
          max="30000"
          class="input input-bordered"
        />
      </label>

      <label class="form-control">
        <span class="label-text text-xs">{{
          $t('trafficAnalysis.proxyConfiguration.fetchJitterMaxMs')
        }}</span>
        <input
          v-model.number="policy.jitterRange[1]"
          type="number"
          min="0"
          max="30000"
          class="input input-bordered"
        />
      </label>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'

const props = defineProps<{
  title: string
  description?: string
  delayLabel: string
  policy: Record<string, any>
  delayField: 'minHostCooldownMs' | 'minHostDelayMs'
}>()

const delayModel = computed({
  get: () => props.policy[props.delayField],
  set: value => {
    props.policy[props.delayField] = value
  },
})
</script>
