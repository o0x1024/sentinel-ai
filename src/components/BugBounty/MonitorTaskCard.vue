<template>
  <div class="card bg-base-200 hover:bg-base-300 transition-colors">
    <div class="card-body p-4">
      <div class="flex items-start justify-between">
        <div class="flex items-center gap-3 flex-1">
          <input
            type="checkbox"
            class="toggle toggle-success"
            :checked="task.enabled"
            @change="emit('toggle', task)"
          />
          <div class="flex-1">
            <div class="flex items-center gap-2">
              <h4 class="font-medium">{{ task.name }}</h4>
              <span v-if="task.enabled" class="badge badge-success badge-xs">
                {{ t('bugBounty.monitor.enabled') }}
              </span>
              <span v-else class="badge badge-ghost badge-xs">
                {{ t('bugBounty.monitor.disabled') }}
              </span>
            </div>
            <div class="text-xs text-base-content/60 mt-1 space-y-1">
              <div>
                <i class="fas fa-clock mr-1"></i>
                {{ t('bugBounty.monitor.interval') }}: {{ formatInterval(task.interval_secs) }}
              </div>
              <div v-if="task.next_run_at">
                <i class="fas fa-calendar-alt mr-1"></i>
                {{ t('bugBounty.monitor.nextRun') }}: {{ formatDateTime(task.next_run_at) }}
              </div>
              <div>
                <i class="fas fa-chart-line mr-1"></i>
                {{ task.run_count }} {{ t('bugBounty.monitor.runs') }},
                {{ task.events_detected }} {{ t('bugBounty.monitor.events') }}
              </div>
            </div>
          </div>
        </div>
        <div class="flex gap-1">
          <button
            type="button"
            class="btn btn-primary btn-xs"
            @click="emit('discover', task)"
            :title="t('bugBounty.monitor.discoverAssets')"
          >
            <i class="fas fa-search"></i>
          </button>
          <button
            v-if="isRunning"
            type="button"
            class="btn btn-error btn-xs"
            @click="emit('stop', task)"
            :disabled="stopping"
            :title="t('bugBounty.monitor.stopTask')"
          >
            <span v-if="stopping" class="loading loading-spinner loading-xs"></span>
            <i v-else class="fas fa-stop"></i>
          </button>
          <button
            v-else
            type="button"
            class="btn btn-ghost btn-xs"
            @click="emit('trigger', task)"
            :title="t('bugBounty.monitor.runNow')"
          >
            <i class="fas fa-play"></i>
          </button>
          <button
            type="button"
            class="btn btn-ghost btn-xs"
            @click="emit('edit', task)"
            :title="t('common.edit')"
          >
            <i class="fas fa-edit"></i>
          </button>
          <button
            type="button"
            class="btn btn-ghost btn-xs text-error"
            @click="emit('delete', task)"
            :title="t('common.delete')"
          >
            <i class="fas fa-trash"></i>
          </button>
        </div>
      </div>

      <div v-if="showProgressCard" class="mt-3 rounded-lg border border-info/20 bg-info/5 px-3 py-3">
        <div class="flex items-center justify-between gap-3">
          <div class="flex items-center gap-2 min-w-0">
            <span
              class="inline-flex h-6 w-6 items-center justify-center rounded-full"
              :class="progressIconClass"
            >
              <i :class="progressIcon"></i>
            </span>
            <div class="min-w-0">
              <div class="text-sm font-medium truncate">{{ progressStatusLabel }}</div>
              <div class="text-xs text-base-content/60 truncate">
                {{ progressSummary }}
              </div>
            </div>
          </div>
          <span v-if="!progress.indeterminate" class="text-sm font-semibold text-info">
            {{ displayProgress }}%
          </span>
          <span v-else class="text-xs font-medium text-info/80">
            {{ t('bugBounty.monitor.runningNow') }}
          </span>
        </div>

        <div class="flex flex-wrap items-center gap-2 mt-3">
          <span v-if="activeStepLabel" class="badge badge-info badge-sm">{{ activeStepLabel }}</span>
          <span v-if="serviceProbeStageLabel" class="badge badge-secondary badge-sm">
            {{ serviceProbeStageLabel }}
          </span>
          <span v-if="portScanStageLabel" class="badge badge-accent badge-sm">
            {{ portScanStageLabel }}
          </span>
          <span
            v-if="genericPluginPhaseLabel && !serviceProbeStageLabel && !portScanStageLabel"
            class="badge badge-primary badge-sm"
          >
            {{ genericPluginPhaseLabel }}
          </span>
          <span class="badge badge-sm" :class="heartbeatBadgeClass">{{ heartbeatLabel }}</span>
          <span v-if="progress.execution_mode === 'scheduler'" class="badge badge-outline badge-sm">
            {{ t('bugBounty.monitor.scheduledRun') }}
          </span>
          <span v-else class="badge badge-outline badge-sm">
            {{ t('bugBounty.monitor.manualRun') }}
          </span>
        </div>

        <progress
          v-if="!progress.indeterminate"
          class="progress progress-info w-full mt-3"
          :value="displayProgress"
          max="100"
        ></progress>
        <progress
          v-else
          class="progress progress-info w-full mt-3"
        ></progress>

        <div class="flex flex-wrap gap-x-4 gap-y-1 mt-2 text-xs text-base-content/70">
          <span>
            {{ t('bugBounty.monitor.stepProgress', { current: progress.completed_steps, total: progress.total_steps || 0 }) }}
          </span>
          <span v-if="progress.current_plugin">
            {{ t('bugBounty.monitor.currentPlugin') }}: {{ progress.current_plugin }}
          </span>
          <span v-if="scanTargetProgressLabel">
            {{ scanTargetProgressLabel }}
          </span>
          <span v-if="scanUnitProgressLabel">
            {{ scanUnitProgressLabel }}
          </span>
          <span v-else-if="pluginUnitProgressLabel">
            {{ pluginUnitProgressLabel }}
          </span>
          <span v-if="progress.current_target">
            {{ t('bugBounty.monitor.currentTarget') }}: {{ progress.current_target }}
          </span>
          <span v-if="progress.target_count > 0">
            {{ t('bugBounty.monitor.targetCount', { count: progress.target_count }) }}
          </span>
          <span v-if="progress.target_breakdown_label">
            {{ progress.target_breakdown_label }}
          </span>
          <span v-if="progress.imported_assets > 0">
            {{ t('bugBounty.monitor.assetsImported') }}: {{ progress.imported_assets }}
          </span>
          <span v-if="progress.started_at">
            {{ t('bugBounty.monitor.elapsedTime') }}: {{ elapsedLabel }}
          </span>
          <span v-if="progress.updated_at">
            {{ t('bugBounty.monitor.lastHeartbeat') }}: {{ heartbeatAgeLabel }}
          </span>
          <span v-if="progress.updated_at">
            {{ t('bugBounty.monitor.lastUpdate') }}: {{ formatDateTime(progress.updated_at) }}
          </span>
        </div>
      </div>

      <div class="flex flex-wrap gap-2 mt-2">
        <span v-if="task.config.enable_dns_monitoring" class="badge badge-outline badge-xs">
          <i class="fas fa-network-wired mr-1"></i>{{ t('bugBounty.monitor.dns') }}
        </span>
        <span v-if="task.config.enable_ip_monitoring" class="badge badge-outline badge-xs">
          <i class="fas fa-diagram-project mr-1"></i>{{ t('bugBounty.monitor.ip') }}
        </span>
        <span v-if="task.config.enable_port_monitoring" class="badge badge-outline badge-xs">
          <i class="fas fa-network-wired mr-1"></i>{{ t('bugBounty.monitor.port') }}
        </span>
        <span v-if="task.config.enable_service_monitoring" class="badge badge-outline badge-xs">
          <i class="fas fa-server mr-1"></i>{{ t('bugBounty.monitor.service') }}
        </span>
        <span v-if="task.config.enable_cert_monitoring" class="badge badge-outline badge-xs">
          <i class="fas fa-certificate mr-1"></i>{{ t('bugBounty.monitor.cert') }}
        </span>
        <span
          v-for="engine in serviceProbeEngineBadges"
          :key="`service-engine-${engine.id}`"
          class="badge badge-outline badge-xs"
        >
          <i class="fas fa-microchip mr-1"></i>{{ t('bugBounty.monitor.serviceProbeEngine') }}: {{ engine.label }}
        </span>
        <span v-if="task.config.enable_web_monitoring" class="badge badge-outline badge-xs">
          <i class="fas fa-globe mr-1"></i>{{ t('bugBounty.monitor.web') }}
        </span>
        <span v-if="task.config.enable_api_monitoring" class="badge badge-outline badge-xs">
          <i class="fas fa-plug mr-1"></i>{{ t('bugBounty.monitor.api') }}
        </span>
        <span v-if="task.config.enable_content_monitoring" class="badge badge-outline badge-xs">
          <i class="fas fa-file-alt mr-1"></i>{{ t('bugBounty.monitor.content') }}
        </span>
        <span v-if="task.config.enable_risk_monitoring" class="badge badge-outline badge-xs">
          <i class="fas fa-shield-alt mr-1"></i>{{ t('bugBounty.monitor.vuln') }}
        </span>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'

import {
  deriveMonitorTaskProgressValue,
  type MonitorTaskProgressState as MonitorTaskProgress,
} from '../../composables/monitorTaskProgressSupport'

const props = defineProps<{
  task: any
  isRunning: boolean
  stopping: boolean
  progress: MonitorTaskProgress | null
}>()

const emit = defineEmits<{
  (e: 'toggle', task: any): void
  (e: 'discover', task: any): void
  (e: 'stop', task: any): void
  (e: 'trigger', task: any): void
  (e: 'edit', task: any): void
  (e: 'delete', task: any): void
}>()

const { t } = useI18n()
const now = ref(Date.now())
let ticker: ReturnType<typeof setInterval> | null = null

onMounted(() => {
  ticker = setInterval(() => {
    now.value = Date.now()
  }, 1000)
})

onUnmounted(() => {
  if (ticker) {
    clearInterval(ticker)
  }
})

const toTimestamp = (value?: string | null) => {
  if (!value) return null
  const timestamp = Date.parse(value)
  return Number.isFinite(timestamp) ? timestamp : null
}

const progressStatusLabel = computed(() => {
  switch (props.progress?.status) {
    case 'completed':
      return t('bugBounty.monitor.progressCompleted')
    case 'failed':
      return t('bugBounty.monitor.progressFailed')
    case 'stopped':
      return t('bugBounty.monitor.progressStopped')
    default:
      return t('bugBounty.monitor.progressRunning')
  }
})

const startedAtMs = computed(() => toTimestamp(props.progress?.started_at))
const updatedAtMs = computed(() => toTimestamp(props.progress?.updated_at))
const isProgressRunning = computed(() => props.progress?.status === 'running')
const showProgressCard = computed(() => Boolean(props.progress && props.isRunning))

const elapsedMs = computed(() => {
  if (!startedAtMs.value) return null
  const endTime = isProgressRunning.value ? now.value : (updatedAtMs.value ?? now.value)
  return Math.max(0, endTime - startedAtMs.value)
})

const heartbeatAgeMs = computed(() => {
  if (!updatedAtMs.value) return null
  if (!isProgressRunning.value) return 0
  return Math.max(0, now.value - updatedAtMs.value)
})

const activeStepLabel = computed(() => {
  if (!props.progress) return null
  const current = props.progress.current_plugin_index || props.progress.completed_steps || 0
  const total = props.progress.total_steps || 0
  if (current <= 0 && total <= 0) return null
  return t('bugBounty.monitor.activeStep', { current, total })
})

const heartbeatState = computed(() => {
  if (!isProgressRunning.value) return 'idle'
  const age = heartbeatAgeMs.value ?? 0
  if (age <= 10_000) return 'fresh'
  if (age <= 30_000) return 'lagging'
  return 'stale'
})

const heartbeatBadgeClass = computed(() => {
  switch (heartbeatState.value) {
    case 'fresh':
      return 'badge-success'
    case 'lagging':
      return 'badge-warning'
    case 'stale':
      return 'badge-error'
    default:
      return 'badge-ghost'
  }
})

const heartbeatLabel = computed(() => {
  switch (heartbeatState.value) {
    case 'fresh':
      return t('bugBounty.monitor.heartbeatFresh')
    case 'lagging':
      return t('bugBounty.monitor.heartbeatLagging')
    case 'stale':
      return t('bugBounty.monitor.heartbeatStale')
    default:
      return t('bugBounty.monitor.heartbeatIdle')
  }
})

const formatCompactDuration = (durationMs: number | null) => {
  if (durationMs == null) return '--'

  const totalSeconds = Math.max(0, Math.floor(durationMs / 1000))
  const hours = Math.floor(totalSeconds / 3600)
  const minutes = Math.floor((totalSeconds % 3600) / 60)
  const seconds = totalSeconds % 60

  if (hours > 0) return `${hours}h ${minutes}m ${seconds}s`
  if (minutes > 0) return `${minutes}m ${seconds}s`
  return `${seconds}s`
}

const elapsedLabel = computed(() => formatCompactDuration(elapsedMs.value))
const heartbeatAgeLabel = computed(() => formatCompactDuration(heartbeatAgeMs.value))
const displayProgress = computed(() => deriveMonitorTaskProgressValue(props.progress))
const genericPluginPhaseLabel = computed(() => {
  if (props.progress?.plugin_phase_label) {
    return props.progress.plugin_phase_label
  }

  switch (props.progress?.plugin_phase) {
    case 'prepare':
      return t('bugBounty.monitor.pluginPhasePrepare')
    case 'resolve':
      return t('bugBounty.monitor.pluginPhaseResolve')
    case 'probe':
      return t('bugBounty.monitor.pluginPhaseProbe')
    case 'discover':
      return t('bugBounty.monitor.pluginPhaseDiscover')
    case 'compare':
      return t('bugBounty.monitor.pluginPhaseCompare')
    case 'build':
      return t('bugBounty.monitor.pluginPhaseBuild')
    default:
      return null
  }
})
const isServiceProbePlugin = computed(() =>
  ['service_monitor', 'service_probe'].includes(String(props.progress?.current_plugin || ''))
)
const isPortMonitorPlugin = computed(() =>
  String(props.progress?.current_plugin || '') === 'port_monitor'
)

const extractServiceProbeTarget = (message?: string | null) => {
  const source = String(message || '')
  const forMatch = source.match(/\sfor\s(.+)$/i)
  if (forMatch?.[1]) return forMatch[1].trim()

  const progressMatch = source.match(/\(([^)]+)\)\s*$/)
  if (progressMatch?.[1]) return progressMatch[1].trim()

  return null
}

const resolveServiceProbeStageLabel = (
  message?: string | null,
  completedUnits?: number | null,
  totalUnits?: number | null,
) => {
  const normalizedMessage = String(message || '').toLowerCase()

  if (normalizedMessage.includes('preparing service probe')) {
    return t('bugBounty.monitor.serviceProbeStagePreparing')
  }
  if (normalizedMessage.includes('matching fingerprints')) {
    return t('bugBounty.monitor.serviceProbeStageFingerprint')
  }
  if (normalizedMessage.includes('parsing http response headers')) {
    return t('bugBounty.monitor.serviceProbeStageResponse')
  }
  if (normalizedMessage.includes('opening ') || normalizedMessage.includes('preparing http request')) {
    return t('bugBounty.monitor.serviceProbeStageConnect')
  }
  if (
    normalizedMessage.includes('reading ')
    || normalizedMessage.includes('sending ')
    || normalizedMessage.includes('probe evidence')
  ) {
    return t('bugBounty.monitor.serviceProbeStageProbe')
  }
  if (normalizedMessage.includes('target progress')) {
    return t('bugBounty.monitor.serviceProbeStageComplete')
  }

  const completed = Number(completedUnits || 0)
  const total = Number(totalUnits || 0)
  if (total <= 0) return null
  if (completed >= total) return t('bugBounty.monitor.serviceProbeStageComplete')
  if (completed >= 3) return t('bugBounty.monitor.serviceProbeStageFingerprint')
  if (completed >= 2) return t('bugBounty.monitor.serviceProbeStageProbe')
  return t('bugBounty.monitor.serviceProbeStageConnect')
}

const formatServiceProbeStatus = (
  message?: string | null,
  target?: string | null,
  completedUnits?: number | null,
  totalUnits?: number | null,
) => {
  const stage = resolveServiceProbeStageLabel(message, completedUnits, totalUnits)
  if (!stage) return null

  const resolvedTarget = target || extractServiceProbeTarget(message)
  if (resolvedTarget) {
    return t('bugBounty.monitor.serviceProbeStageRunning', {
      stage,
      target: resolvedTarget,
    })
  }

  return stage
}

const extractPortScanTarget = (message?: string | null) => {
  const source = String(message || '')
  const scanningForMatch = source.match(/scanning\s+\d+\s+sockets\s+for\s+(.+)$/i)
  if (scanningForMatch?.[1]) return scanningForMatch[1].trim()

  const progressMatch = source.match(/scanning\s+(.+?):\s+socket progress/i)
  if (progressMatch?.[1]) return progressMatch[1].trim()

  return null
}

const resolvePortScanStageLabel = (
  message?: string | null,
  completedUnits?: number | null,
  totalUnits?: number | null,
) => {
  const normalizedMessage = String(message || '').toLowerCase()
  if (normalizedMessage.includes('preparing port scan')) {
    return t('bugBounty.monitor.portScanStagePreparing')
  }
  if (normalizedMessage.includes('socket progress')) {
    return t('bugBounty.monitor.portScanStageScanning')
  }
  if (normalizedMessage.includes('scanning ') && normalizedMessage.includes(' sockets for ')) {
    return t('bugBounty.monitor.portScanStageConnecting')
  }

  const completed = Number(completedUnits || 0)
  const total = Number(totalUnits || 0)
  if (total <= 0) return null
  if (completed >= total) return t('bugBounty.monitor.portScanStageScanning')
  if (completed > 0) return t('bugBounty.monitor.portScanStageScanning')
  return t('bugBounty.monitor.portScanStageConnecting')
}

const formatPortScanStatus = (
  message?: string | null,
  target?: string | null,
  completedUnits?: number | null,
  totalUnits?: number | null,
) => {
  const stage = resolvePortScanStageLabel(message, completedUnits, totalUnits)
  if (!stage) return null

  const resolvedTarget = target || extractPortScanTarget(message)
  if (resolvedTarget) {
    return t('bugBounty.monitor.portScanStageRunning', {
      stage,
      target: resolvedTarget,
    })
  }

  return stage
}

const serviceProbeStageLabel = computed(() => {
  if (!isServiceProbePlugin.value || props.progress?.status !== 'running' || props.progress?.plugin_phase) {
    return null
  }
  return resolveServiceProbeStageLabel(
    props.progress?.message,
    props.progress?.scan_completed_units,
    props.progress?.scan_total_units,
  )
})

const portScanStageLabel = computed(() => {
  if (!isPortMonitorPlugin.value || props.progress?.status !== 'running' || props.progress?.plugin_phase) {
    return null
  }
  return resolvePortScanStageLabel(
    props.progress?.message,
    props.progress?.scan_completed_units,
    props.progress?.scan_total_units,
  )
})

const scanTargetProgressLabel = computed(() => {
  const completed = props.progress?.scan_completed_targets
  const total = props.progress?.scan_total_targets
  if (completed == null || total == null || total <= 0) return null
  return t('bugBounty.monitor.scanTargetProgress', { current: completed, total })
})

const scanUnitProgressLabel = computed(() => {
  const completed = props.progress?.scan_completed_units
  const total = props.progress?.scan_total_units
  if (completed == null || total == null || total <= 0) return null
  if (['service_monitor', 'service_probe'].includes(String(props.progress?.current_plugin || ''))) {
    return t('bugBounty.monitor.serviceProbePhaseProgress', { current: completed, total })
  }
  return t('bugBounty.monitor.scanSocketProgress', { current: completed, total })
})

const pluginUnitProgressLabel = computed(() => {
  const completed = props.progress?.plugin_completed_units
  const total = props.progress?.plugin_total_units
  if (completed == null || total == null || total <= 0) return null
  return t('bugBounty.monitor.pluginUnitProgress', { current: completed, total })
})

const progressSummary = computed(() => {
  if (isServiceProbePlugin.value && serviceProbeStageLabel.value) {
    return formatServiceProbeStatus(
      props.progress?.message,
      props.progress?.current_target,
      props.progress?.scan_completed_units,
      props.progress?.scan_total_units,
    )
  }
  if (isPortMonitorPlugin.value && portScanStageLabel.value) {
    return formatPortScanStatus(
      props.progress?.message,
      props.progress?.current_target,
      props.progress?.scan_completed_units,
      props.progress?.scan_total_units,
    )
  }
  if (genericPluginPhaseLabel.value) {
    if (props.progress?.current_target) {
      return t('bugBounty.monitor.genericPhaseRunning', {
        stage: genericPluginPhaseLabel.value,
        target: props.progress.current_target,
      })
    }
    return genericPluginPhaseLabel.value
  }
  if (props.progress?.message) {
    return props.progress.message
  }
  if (props.progress?.current_plugin) {
    return t('bugBounty.monitor.pluginRunning', { plugin: props.progress.current_plugin })
  }
  if (props.progress?.status === 'completed') return t('bugBounty.monitor.progressCompleted')
  if (props.progress?.status === 'failed') return t('bugBounty.monitor.progressFailed')
  if (props.progress?.status === 'stopped') return t('bugBounty.monitor.progressStopped')
  return t('bugBounty.monitor.progressPreparing')
})

const progressIcon = computed(() => {
  switch (props.progress?.status) {
    case 'completed':
      return 'fas fa-check'
    case 'failed':
      return 'fas fa-times'
    case 'stopped':
      return 'fas fa-stop'
    default:
      return 'fas fa-spinner fa-spin'
  }
})

const progressIconClass = computed(() => {
  switch (props.progress?.status) {
    case 'completed':
      return 'bg-success/20 text-success'
    case 'failed':
      return 'bg-error/20 text-error'
    case 'stopped':
      return 'bg-warning/20 text-warning'
    default:
      return 'bg-info/20 text-info'
  }
})

const serviceProbeEngineBadges = computed(() => {
  const plugins = Array.isArray(props.task?.config?.service_plugins)
    ? props.task.config.service_plugins
    : []

  const engines = new Set<string>()
  for (const plugin of plugins) {
    const pluginId = String(plugin?.plugin_id || '').trim()
    if (!['service_monitor', 'service_probe'].includes(pluginId)) {
      continue
    }

    engines.add('native')
  }

  return Array.from(engines).map(engine => ({
    id: engine,
    label: t('bugBounty.monitor.serviceProbeEngineNative'),
  }))
})

const formatInterval = (secs: number) => {
  const minutes = secs / 60
  const hours = secs / 3600
  const days = hours / 24
  if (Number.isInteger(days) && days >= 1) return `${days} ${t('bugBounty.monitor.days')}`
  if (Number.isInteger(hours) && hours >= 1) return `${hours} ${t('bugBounty.monitor.hours')}`
  if (Number.isInteger(minutes) && minutes >= 1) return `${minutes} ${t('bugBounty.monitor.minutes')}`
  return `${secs} ${t('bugBounty.monitor.seconds')}`
}

const formatDateTime = (dateStr: string) => new Date(dateStr).toLocaleString()
</script>
