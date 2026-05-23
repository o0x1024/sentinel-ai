<template>
  <div class="rounded-lg border border-base-300 bg-base-100 p-4 shadow-sm space-y-4">
    <div>
      <h2 class="text-base font-semibold">
        {{ $t('trafficAnalysis.proxyConfiguration.pluginRuntimeTitle') }}
      </h2>
      <p class="mt-1 text-sm text-base-content/70">
        {{ description }}
      </p>
    </div>

    <p class="text-xs text-base-content/60">
      {{ hint }}
    </p>

    <div class="flex flex-wrap gap-2">
      <button
        class="btn btn-xs btn-outline"
        type="button"
        :disabled="isSavingTrafficPluginRuntimeSettings"
        @click="applyTrafficPluginRuntimePreset('local_fast', policyIds)"
      >
        {{ $t('trafficAnalysis.proxyConfiguration.activeProbePresetLocalFast') }}
      </button>
      <button
        class="btn btn-xs btn-outline"
        type="button"
        :disabled="isSavingTrafficPluginRuntimeSettings"
        @click="applyTrafficPluginRuntimePreset('balanced', policyIds)"
      >
        {{ $t('trafficAnalysis.proxyConfiguration.activeProbePresetBalanced') }}
      </button>
      <button
        class="btn btn-xs btn-outline"
        type="button"
        :disabled="isSavingTrafficPluginRuntimeSettings"
        @click="applyTrafficPluginRuntimePreset('conservative', policyIds)"
      >
        {{ $t('trafficAnalysis.proxyConfiguration.activeProbePresetConservative') }}
      </button>
    </div>

    <div class="flex flex-wrap items-center gap-2">
      <button
        class="btn btn-sm btn-outline"
        type="button"
        :disabled="isSavingTrafficPluginRuntimeSettings"
        @click="resetTrafficPluginRuntimePolicies(policyIds)"
      >
        {{ $t('trafficAnalysis.proxyConfiguration.resetToDefaults') }}
      </button>
    </div>

    <div class="space-y-3">
      <section
        v-for="item in visiblePolicies"
        :key="item.id"
        class="rounded-lg border border-base-300 bg-base-100"
      >
        <button
          class="flex w-full items-start justify-between gap-3 px-4 py-3 text-left"
          type="button"
          @click="togglePolicy(item.id)"
        >
          <div class="min-w-0 flex-1">
            <div class="flex items-center gap-2">
              <h3 class="truncate text-sm font-medium">{{ item.title }}</h3>
              <span class="badge badge-sm badge-outline">{{ item.queueKind }}</span>
            </div>
            <p class="mt-1 text-xs text-base-content/60">{{ item.description }}</p>
            <div
              v-if="item.queueStats"
              class="mt-2 flex flex-wrap gap-2 text-[11px] text-base-content/65"
            >
              <span
                class="rounded px-2 py-1"
                :class="summaryChipClass(getPendingPressureLevel(item.queueStats))"
              >
                P {{ item.queueStats.pendingCount }}/{{ item.queueStats.configuredMaxQueueDepth }}
              </span>
              <span class="rounded bg-base-200/70 px-2 py-1">
                R {{ item.queueStats.runningCount }}/{{
                  item.queueStats.configuredMaxGlobalConcurrent
                }}
              </span>
              <span class="rounded bg-base-200/70 px-2 py-1">
                T {{ item.queueStats.recentWindowTotalCount }}/min
              </span>
              <span
                class="rounded px-2 py-1"
                :class="summaryChipClass(getRejectedTrendLevel(item.rejectedSeries))"
              >
                X {{ item.queueStats.rejectedTotalCount }}
              </span>
            </div>
          </div>
          <i
            v-if="props.collapsible"
            class="fas mt-1 text-base-content/55"
            :class="isExpanded(item.id) ? 'fa-chevron-up' : 'fa-chevron-down'"
          ></i>
        </button>

        <div v-if="isExpanded(item.id)" class="border-t border-base-300 px-4 py-4 space-y-4">
          <TrafficPluginRuntimePolicyCard
            :title="item.title"
            :description="item.description"
            :delay-label="item.delayLabel"
            :policy="trafficPluginRuntimeSettings[item.id]"
            :delay-field="item.delayField"
            @update:policy="updateTrafficPluginRuntimePolicy(item.id, $event)"
          />
          <TrafficPluginRuntimeQueueStatusGrid
            :visible-policy-kinds="[item.queueKind]"
            :show-header="false"
          />
        </div>
      </section>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import TrafficPluginRuntimePolicyCard from '@/components/traffic/TrafficPluginRuntimePolicyCard.vue'
import TrafficPluginRuntimeQueueStatusGrid from '@/components/traffic/TrafficPluginRuntimeQueueStatusGrid.vue'
import { getTrafficPluginRuntimeHistorySeries } from '@/components/traffic/trafficPluginRuntimeQueueHistorySupport'
import type { TrafficPluginRuntimeSettings } from '@/components/traffic/proxyConfigurationTypes'
import type {
  TrafficPluginRuntimePolicyId,
  TrafficPluginRuntimePreset,
} from '@/components/traffic/pluginRuntimeSettingsSupport'
import { useTrafficPluginRuntimeQueue } from '@/components/traffic/useTrafficPluginRuntimeQueue'
import type {
  TrafficPluginRuntimePolicyKind,
  TrafficPluginRuntimeQueueStats,
} from '@/components/traffic/trafficPluginRuntimeQueueTypes'
import {
  getPendingPressureLevel,
  getRejectedTrendLevel,
  getRuntimeSummaryChipClass,
} from './pluginRuntimeSchedulerSummarySupport'

const props = withDefaults(
  defineProps<{
    trafficPluginRuntimeSettings: TrafficPluginRuntimeSettings
    isSavingTrafficPluginRuntimeSettings: boolean
    resetTrafficPluginRuntimePolicies: (
      policyIds: TrafficPluginRuntimePolicyId[]
    ) => void | Promise<unknown>
    applyTrafficPluginRuntimePreset: (
      preset: TrafficPluginRuntimePreset,
      policyIds: TrafficPluginRuntimePolicyId[]
    ) => void
    updateTrafficPluginRuntimePolicy: (
      policyId: TrafficPluginRuntimePolicyId,
      policy: TrafficPluginRuntimeSettings[TrafficPluginRuntimePolicyId]
    ) => void
    policyIds?: TrafficPluginRuntimePolicyId[]
    collapsible?: boolean
    defaultExpandedPolicyIds?: TrafficPluginRuntimePolicyId[]
    description?: string
    hint?: string
  }>(),
  {
    policyIds: () => [
      'activeProbe',
      'bountyFetch',
      'monitorFetch',
      'agentFetch',
      'pluginTestFetch',
    ],
    collapsible: false,
    defaultExpandedPolicyIds: () => [],
    description: '',
    hint: '',
  }
)

const { t } = useI18n()
const { snapshot, history } = useTrafficPluginRuntimeQueue()
const expandedPolicyIds = ref<TrafficPluginRuntimePolicyId[]>(
  props.defaultExpandedPolicyIds.length > 0
    ? [...props.defaultExpandedPolicyIds]
    : props.collapsible
      ? []
      : [...props.policyIds]
)

const queueStatsByKind = computed<
  Record<TrafficPluginRuntimePolicyKind, TrafficPluginRuntimeQueueStats | null>
>(() => ({
  traffic_active_probe: snapshot.value?.activeProbe ?? null,
  bounty_fetch: snapshot.value?.bountyFetch ?? null,
  monitor_fetch: snapshot.value?.monitorFetch ?? null,
  agent_fetch: snapshot.value?.agentFetch ?? null,
  plugin_test_fetch: snapshot.value?.pluginTestFetch ?? null,
}))

const visiblePolicies = computed(() => {
  const allPolicies = [
    {
      id: 'activeProbe' as const,
      queueKind: 'traffic_active_probe' as const,
      title: t('trafficAnalysis.proxyConfiguration.pluginRuntimePolicyActiveProbeTitle'),
      description: t('trafficAnalysis.proxyConfiguration.pluginRuntimePolicyActiveProbeDesc'),
      delayLabel: t('trafficAnalysis.proxyConfiguration.activeProbeMinHostCooldownMs'),
      delayField: 'minHostCooldownMs' as const,
    },
    {
      id: 'bountyFetch' as const,
      queueKind: 'bounty_fetch' as const,
      title: t('trafficAnalysis.proxyConfiguration.pluginRuntimePolicyBountyTitle'),
      description: t('trafficAnalysis.proxyConfiguration.pluginRuntimePolicyBountyDesc'),
      delayLabel: t('trafficAnalysis.proxyConfiguration.fetchMinHostDelayMs'),
      delayField: 'minHostDelayMs' as const,
    },
    {
      id: 'monitorFetch' as const,
      queueKind: 'monitor_fetch' as const,
      title: t('trafficAnalysis.proxyConfiguration.pluginRuntimePolicyMonitorTitle'),
      description: t('trafficAnalysis.proxyConfiguration.pluginRuntimePolicyMonitorDesc'),
      delayLabel: t('trafficAnalysis.proxyConfiguration.fetchMinHostDelayMs'),
      delayField: 'minHostDelayMs' as const,
    },
    {
      id: 'agentFetch' as const,
      queueKind: 'agent_fetch' as const,
      title: t('trafficAnalysis.proxyConfiguration.pluginRuntimePolicyAgentTitle'),
      description: t('trafficAnalysis.proxyConfiguration.pluginRuntimePolicyAgentDesc'),
      delayLabel: t('trafficAnalysis.proxyConfiguration.fetchMinHostDelayMs'),
      delayField: 'minHostDelayMs' as const,
    },
    {
      id: 'pluginTestFetch' as const,
      queueKind: 'plugin_test_fetch' as const,
      title: t('trafficAnalysis.proxyConfiguration.pluginRuntimePolicyPluginTestTitle'),
      description: t('trafficAnalysis.proxyConfiguration.pluginRuntimePolicyPluginTestDesc'),
      delayLabel: t('trafficAnalysis.proxyConfiguration.fetchMinHostDelayMs'),
      delayField: 'minHostDelayMs' as const,
    },
  ]

  return allPolicies
    .filter(policy => props.policyIds.includes(policy.id))
    .map(policy => ({
      ...policy,
      queueStats: queueStatsByKind.value[policy.queueKind],
      rejectedSeries: getTrafficPluginRuntimeHistorySeries(
        history.value,
        policy.queueKind,
        'rejectedTotalCount'
      ),
    }))
})

function summaryChipClass(level: 'neutral' | 'warning' | 'error') {
  return getRuntimeSummaryChipClass(level)
}

function isExpanded(policyId: TrafficPluginRuntimePolicyId) {
  return expandedPolicyIds.value.includes(policyId)
}

function togglePolicy(policyId: TrafficPluginRuntimePolicyId) {
  if (!props.collapsible) {
    return
  }

  if (isExpanded(policyId)) {
    expandedPolicyIds.value = expandedPolicyIds.value.filter(id => id !== policyId)
    return
  }

  expandedPolicyIds.value = [...expandedPolicyIds.value, policyId]
}
</script>
