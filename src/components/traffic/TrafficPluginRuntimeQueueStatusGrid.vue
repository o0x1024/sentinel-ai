<template>
  <div class="rounded-lg border border-base-300 p-4 space-y-4">
    <div v-if="showHeader" class="flex flex-wrap items-start justify-between gap-3">
      <div>
        <h4 class="font-medium">
          {{ $t('trafficAnalysis.proxyConfiguration.pluginRuntimeQueueStatusTitle') }}
        </h4>
        <p class="mt-1 text-xs text-base-content/60">
          {{ $t('trafficAnalysis.proxyConfiguration.pluginRuntimeQueueStatusDesc') }}
        </p>
      </div>

      <div class="flex items-center gap-2">
        <span v-if="error" class="text-xs text-error">{{ error }}</span>
        <span v-else-if="lastUpdatedLabel" class="text-xs text-base-content/55">
          {{
            $t('trafficAnalysis.proxyConfiguration.pluginRuntimeQueueUpdatedAt', {
              time: lastUpdatedLabel,
            })
          }}
        </span>
        <button class="btn btn-xs btn-outline" type="button" :disabled="loading" @click="refresh">
          <i :class="loading ? 'fas fa-spinner fa-spin' : 'fas fa-sync-alt'"></i>
          <span>{{ $t('trafficAnalysis.proxyConfiguration.pluginRuntimeQueueRefresh') }}</span>
        </button>
      </div>
    </div>

    <div v-if="cards.length > 0" class="grid gap-3" :class="cardsGridClass">
      <div
        v-for="card in cards"
        :key="card.id"
        class="rounded-lg border border-base-300 bg-base-100 p-3"
      >
        <div class="grid gap-3" :class="cardContentClass">
          <div class="min-w-0 space-y-3">
            <div class="flex items-start justify-between gap-3">
              <div class="min-w-0">
                <h5 class="truncate text-sm font-medium">{{ card.title }}</h5>
                <p class="mt-1 text-[11px] text-base-content/55">{{ card.subtitle }}</p>
              </div>
              <span class="badge badge-sm badge-outline shrink-0">
                {{ card.stats.runningCount }}/{{ card.stats.configuredMaxGlobalConcurrent }}
              </span>
            </div>

            <div class="space-y-1 text-[11px] text-base-content/65">
              <div class="flex items-center justify-between gap-2">
                <span>{{
                  $t('trafficAnalysis.proxyConfiguration.pluginRuntimeQueueRejected')
                }}</span>
                <span>{{ card.stats.rejectedTotalCount }}</span>
              </div>
              <div class="flex items-center justify-between gap-2">
                <span>{{
                  $t('trafficAnalysis.proxyConfiguration.pluginRuntimeQueueQueuedScheduled')
                }}</span>
                <span>{{ card.stats.queuedCount }} / {{ card.stats.scheduledCount }}</span>
              </div>
              <div class="flex items-center justify-between gap-2">
                <span>{{
                  $t('trafficAnalysis.proxyConfiguration.pluginRuntimeQueueActiveRunsPlugins')
                }}</span>
                <span>{{ card.stats.activeRuns }} / {{ card.stats.activePlugins }}</span>
              </div>
              <div class="flex items-center justify-between gap-2">
                <span>{{
                  $t('trafficAnalysis.proxyConfiguration.pluginRuntimeQueueHostPeak')
                }}</span>
                <span>
                  {{ card.stats.hottestHostActive }}/{{ card.stats.configuredMaxConcurrentPerHost }}
                </span>
              </div>
              <div class="flex items-center justify-between gap-2">
                <span>{{
                  $t('trafficAnalysis.proxyConfiguration.pluginRuntimeQueueCancelledRuns')
                }}</span>
                <span>{{ card.stats.cancelledRunCount }}</span>
              </div>
              <div class="truncate">
                <span class="text-base-content/50">{{
                  $t('trafficAnalysis.proxyConfiguration.pluginRuntimeQueueHottestHost')
                }}</span>
                <span class="ml-1 font-mono">{{
                  card.stats.hottestHost ||
                  $t('trafficAnalysis.proxyConfiguration.pluginRuntimeQueueNoHost')
                }}</span>
              </div>
            </div>
          </div>

          <div class="grid grid-cols-2 gap-2 text-xs" :class="metricGridClass">
            <div class="rounded border border-base-300/80 bg-base-200/40 px-2 py-2">
              <div class="text-base-content/55">
                {{ $t('trafficAnalysis.proxyConfiguration.pluginRuntimeQueuePending') }}
              </div>
              <div class="mt-1 font-medium">
                {{ card.stats.pendingCount }}/{{ card.stats.configuredMaxQueueDepth }}
              </div>
            </div>
            <div class="rounded border border-base-300/80 bg-base-200/40 px-2 py-2">
              <div class="text-base-content/55">
                {{ $t('trafficAnalysis.proxyConfiguration.pluginRuntimeQueueRunning') }}
              </div>
              <div class="mt-1 font-medium">
                {{ card.stats.runningCount }}/{{ card.stats.configuredMaxGlobalConcurrent }}
              </div>
            </div>
            <div class="rounded border border-base-300/80 bg-base-200/40 px-2 py-2">
              <div class="text-base-content/55">
                {{ $t('trafficAnalysis.proxyConfiguration.pluginRuntimeQueueActiveHosts') }}
              </div>
              <div class="mt-1 font-medium">{{ card.stats.activeHosts }}</div>
            </div>
            <div class="rounded border border-base-300/80 bg-base-200/40 px-2 py-2">
              <div class="text-base-content/55">
                {{ $t('trafficAnalysis.proxyConfiguration.pluginRuntimeQueueRecent') }}
              </div>
              <div class="mt-1 font-medium">{{ card.stats.recentCount }}</div>
            </div>
            <div class="rounded border border-base-300/80 bg-base-200/40 px-2 py-2">
              <div class="text-base-content/55">
                {{ $t('trafficAnalysis.proxyConfiguration.pluginRuntimeQueueThroughput1m') }}
              </div>
              <div class="mt-1 font-medium">{{ card.stats.recentWindowTotalCount }}/min</div>
            </div>
            <div class="rounded border border-base-300/80 bg-base-200/40 px-2 py-2">
              <div class="text-base-content/55">
                {{ $t('trafficAnalysis.proxyConfiguration.pluginRuntimeQueueAvgWait1m') }}
              </div>
              <div class="mt-1 font-medium">
                {{ formatMs(card.stats.recentWindowAvgQueueWaitMs) }}
              </div>
            </div>
            <div class="rounded border border-base-300/80 bg-base-200/40 px-2 py-2">
              <div class="text-base-content/55">
                {{ $t('trafficAnalysis.proxyConfiguration.pluginRuntimeQueueAvgResponse1m') }}
              </div>
              <div class="mt-1 font-medium">
                {{ formatMs(card.stats.recentWindowAvgResponseElapsedMs) }}
              </div>
            </div>
            <div class="rounded border border-base-300/80 bg-base-200/40 px-2 py-2">
              <div class="text-base-content/55">
                {{ $t('trafficAnalysis.proxyConfiguration.pluginRuntimeQueueOutcome1m') }}
              </div>
              <div class="mt-1 font-medium">
                {{ card.stats.recentWindowCompletedCount }} /
                {{ card.stats.recentWindowFailedCount }} /
                {{ card.stats.recentWindowCancelledCount }}
              </div>
            </div>
          </div>

          <div class="grid gap-3" :class="detailGridClass">
            <div
              class="space-y-1 rounded border border-base-300/80 bg-base-200/30 p-2 text-[11px] text-base-content/65"
            >
              <div class="text-base-content/50">
                {{ $t('trafficAnalysis.proxyConfiguration.pluginRuntimeQueueRejectReasons') }}
              </div>
              <div class="flex items-center justify-between gap-2">
                <span>{{
                  $t('trafficAnalysis.proxyConfiguration.pluginRuntimeQueueRejectQueueLimit')
                }}</span>
                <span>{{ card.stats.rejectedQueueLimitCount }}</span>
              </div>
              <div class="flex items-center justify-between gap-2">
                <span>{{
                  $t('trafficAnalysis.proxyConfiguration.pluginRuntimeQueueRejectRunPending')
                }}</span>
                <span>{{ card.stats.rejectedRunPendingLimitCount }}</span>
              </div>
              <div class="flex items-center justify-between gap-2">
                <span>{{
                  $t('trafficAnalysis.proxyConfiguration.pluginRuntimeQueueRejectPluginPending')
                }}</span>
                <span>{{ card.stats.rejectedPluginPendingLimitCount }}</span>
              </div>
              <div class="flex items-center justify-between gap-2">
                <span>{{
                  $t('trafficAnalysis.proxyConfiguration.pluginRuntimeQueueRejectCancelledRun')
                }}</span>
                <span>{{ card.stats.rejectedCancelledRunCount }}</span>
              </div>
            </div>
            <div class="space-y-2 rounded border border-base-300/80 bg-base-200/20 p-2">
              <div class="text-[11px] text-base-content/50">
                {{ $t('trafficAnalysis.proxyConfiguration.pluginRuntimeQueueTrendTitle') }}
              </div>
              <div class="grid gap-2 md:grid-cols-2">
                <TrafficPluginRuntimeQueueSparkline
                  :label="$t('trafficAnalysis.proxyConfiguration.pluginRuntimeQueueTrendPending')"
                  :value="String(card.stats.pendingCount)"
                  :values="card.history.pending"
                  stroke-class="text-info"
                />
                <TrafficPluginRuntimeQueueSparkline
                  :label="$t('trafficAnalysis.proxyConfiguration.pluginRuntimeQueueTrendRunning')"
                  :value="String(card.stats.runningCount)"
                  :values="card.history.running"
                  stroke-class="text-success"
                />
                <TrafficPluginRuntimeQueueSparkline
                  :label="$t('trafficAnalysis.proxyConfiguration.pluginRuntimeQueueTrendRejected')"
                  :value="String(card.stats.rejectedTotalCount)"
                  :values="card.history.rejected"
                  stroke-class="text-error"
                />
                <TrafficPluginRuntimeQueueSparkline
                  :label="
                    $t('trafficAnalysis.proxyConfiguration.pluginRuntimeQueueTrendThroughput')
                  "
                  :value="`${card.stats.recentWindowTotalCount}/min`"
                  :values="card.history.throughput"
                  stroke-class="text-warning"
                />
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>

    <p v-else-if="!loading && !error" class="text-xs text-base-content/55">
      {{ $t('trafficAnalysis.proxyConfiguration.pluginRuntimeQueueEmpty') }}
    </p>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import TrafficPluginRuntimeQueueSparkline from './TrafficPluginRuntimeQueueSparkline.vue'
import { getTrafficPluginRuntimeHistorySeries } from './trafficPluginRuntimeQueueHistorySupport'
import { useTrafficPluginRuntimeQueue } from './useTrafficPluginRuntimeQueue'
import type { TrafficPluginRuntimePolicyKind } from './trafficPluginRuntimeQueueTypes'

const props = withDefaults(
  defineProps<{
    visiblePolicyKinds?: TrafficPluginRuntimePolicyKind[]
    showHeader?: boolean
  }>(),
  {
    visiblePolicyKinds: () => [],
    showHeader: true,
  }
)

function formatLocalTime(value: string | null): string | null {
  if (!value) {
    return null
  }
  const parsed = new Date(value)
  if (Number.isNaN(parsed.getTime())) {
    return null
  }
  return parsed.toLocaleTimeString()
}

function formatMs(value: number): string {
  return `${Math.max(0, Math.round(value))} ms`
}

const { snapshot, history, loading, error, lastUpdatedAt, refresh } = useTrafficPluginRuntimeQueue()
const { t } = useI18n()

const lastUpdatedLabel = computed(() => formatLocalTime(lastUpdatedAt.value))

const cards = computed(() => {
  if (!snapshot.value) {
    return []
  }

  const allCards = [
    {
      id: 'activeProbe',
      kind: 'traffic_active_probe' as const,
      title: t('trafficAnalysis.proxyConfiguration.pluginRuntimePolicyActiveProbeTitle'),
      subtitle: 'traffic_active_probe',
      stats: snapshot.value.activeProbe,
      history: {
        pending: getTrafficPluginRuntimeHistorySeries(
          history.value,
          'traffic_active_probe',
          'pendingCount'
        ),
        running: getTrafficPluginRuntimeHistorySeries(
          history.value,
          'traffic_active_probe',
          'runningCount'
        ),
        rejected: getTrafficPluginRuntimeHistorySeries(
          history.value,
          'traffic_active_probe',
          'rejectedTotalCount'
        ),
        throughput: getTrafficPluginRuntimeHistorySeries(
          history.value,
          'traffic_active_probe',
          'recentWindowTotalCount'
        ),
      },
    },
    {
      id: 'bountyFetch',
      kind: 'bounty_fetch' as const,
      title: t('trafficAnalysis.proxyConfiguration.pluginRuntimePolicyBountyTitle'),
      subtitle: 'bounty_fetch',
      stats: snapshot.value.bountyFetch,
      history: {
        pending: getTrafficPluginRuntimeHistorySeries(
          history.value,
          'bounty_fetch',
          'pendingCount'
        ),
        running: getTrafficPluginRuntimeHistorySeries(
          history.value,
          'bounty_fetch',
          'runningCount'
        ),
        rejected: getTrafficPluginRuntimeHistorySeries(
          history.value,
          'bounty_fetch',
          'rejectedTotalCount'
        ),
        throughput: getTrafficPluginRuntimeHistorySeries(
          history.value,
          'bounty_fetch',
          'recentWindowTotalCount'
        ),
      },
    },
    {
      id: 'monitorFetch',
      kind: 'monitor_fetch' as const,
      title: t('trafficAnalysis.proxyConfiguration.pluginRuntimePolicyMonitorTitle'),
      subtitle: 'monitor_fetch',
      stats: snapshot.value.monitorFetch,
      history: {
        pending: getTrafficPluginRuntimeHistorySeries(
          history.value,
          'monitor_fetch',
          'pendingCount'
        ),
        running: getTrafficPluginRuntimeHistorySeries(
          history.value,
          'monitor_fetch',
          'runningCount'
        ),
        rejected: getTrafficPluginRuntimeHistorySeries(
          history.value,
          'monitor_fetch',
          'rejectedTotalCount'
        ),
        throughput: getTrafficPluginRuntimeHistorySeries(
          history.value,
          'monitor_fetch',
          'recentWindowTotalCount'
        ),
      },
    },
    {
      id: 'agentFetch',
      kind: 'agent_fetch' as const,
      title: t('trafficAnalysis.proxyConfiguration.pluginRuntimePolicyAgentTitle'),
      subtitle: 'agent_fetch',
      stats: snapshot.value.agentFetch,
      history: {
        pending: getTrafficPluginRuntimeHistorySeries(history.value, 'agent_fetch', 'pendingCount'),
        running: getTrafficPluginRuntimeHistorySeries(history.value, 'agent_fetch', 'runningCount'),
        rejected: getTrafficPluginRuntimeHistorySeries(
          history.value,
          'agent_fetch',
          'rejectedTotalCount'
        ),
        throughput: getTrafficPluginRuntimeHistorySeries(
          history.value,
          'agent_fetch',
          'recentWindowTotalCount'
        ),
      },
    },
    {
      id: 'pluginTestFetch',
      kind: 'plugin_test_fetch' as const,
      title: t('trafficAnalysis.proxyConfiguration.pluginRuntimePolicyPluginTestTitle'),
      subtitle: 'plugin_test_fetch',
      stats: snapshot.value.pluginTestFetch,
      history: {
        pending: getTrafficPluginRuntimeHistorySeries(
          history.value,
          'plugin_test_fetch',
          'pendingCount'
        ),
        running: getTrafficPluginRuntimeHistorySeries(
          history.value,
          'plugin_test_fetch',
          'runningCount'
        ),
        rejected: getTrafficPluginRuntimeHistorySeries(
          history.value,
          'plugin_test_fetch',
          'rejectedTotalCount'
        ),
        throughput: getTrafficPluginRuntimeHistorySeries(
          history.value,
          'plugin_test_fetch',
          'recentWindowTotalCount'
        ),
      },
    },
  ]

  if (!props.visiblePolicyKinds || props.visiblePolicyKinds.length === 0) {
    return allCards
  }

  return allCards.filter(card => props.visiblePolicyKinds?.includes(card.kind))
})

const isSingleCard = computed(() => cards.value.length === 1)

const cardsGridClass = computed(() =>
  isSingleCard.value ? 'grid-cols-1' : 'md:grid-cols-2 xl:grid-cols-3'
)

const cardContentClass = computed(() =>
  isSingleCard.value ? 'lg:grid-cols-[15rem_minmax(0,1fr)_minmax(20rem,24rem)]' : ''
)

const metricGridClass = computed(() =>
  isSingleCard.value ? 'sm:grid-cols-4 lg:grid-cols-2 xl:grid-cols-4' : ''
)

const detailGridClass = computed(() => (isSingleCard.value ? 'md:grid-cols-2 lg:grid-cols-1' : ''))
</script>
