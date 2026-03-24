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
            class="btn btn-primary btn-xs"
            @click="emit('discover', task)"
            :title="t('bugBounty.monitor.discoverAssets')"
          >
            <i class="fas fa-search"></i>
          </button>
          <button
            v-if="isRunning"
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
            class="btn btn-ghost btn-xs"
            @click="emit('trigger', task)"
            :title="t('bugBounty.monitor.runNow')"
          >
            <i class="fas fa-play"></i>
          </button>
          <button
            class="btn btn-ghost btn-xs"
            @click="emit('edit', task)"
            :title="t('common.edit')"
          >
            <i class="fas fa-edit"></i>
          </button>
          <button
            class="btn btn-ghost btn-xs text-error"
            @click="emit('delete', task)"
            :title="t('common.delete')"
          >
            <i class="fas fa-trash"></i>
          </button>
        </div>
      </div>

      <div v-if="progress" class="mt-3 rounded-lg border border-info/20 bg-info/5 px-3 py-3">
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
          <span class="text-sm font-semibold text-info">{{ progress.progress }}%</span>
        </div>

        <progress
          class="progress progress-info w-full mt-3"
          :value="progress.progress"
          max="100"
        ></progress>

        <div class="flex flex-wrap gap-x-4 gap-y-1 mt-2 text-xs text-base-content/70">
          <span>
            {{ t('bugBounty.monitor.stepProgress', { current: progress.completed_steps, total: progress.total_steps || 0 }) }}
          </span>
          <span v-if="progress.current_plugin">
            {{ t('bugBounty.monitor.currentPlugin') }}: {{ progress.current_plugin }}
          </span>
          <span v-if="progress.target_count > 0">
            {{ t('bugBounty.monitor.targetCount', { count: progress.target_count }) }}
          </span>
          <span v-if="progress.imported_assets > 0">
            {{ t('bugBounty.monitor.assetsImported') }}: {{ progress.imported_assets }}
          </span>
          <span v-if="progress.execution_mode === 'scheduler'">
            {{ t('bugBounty.monitor.scheduledRun') }}
          </span>
          <span v-else>
            {{ t('bugBounty.monitor.manualRun') }}
          </span>
        </div>
      </div>

      <details v-if="logs.length > 0" class="collapse collapse-arrow bg-base-100/70 mt-3">
        <summary class="collapse-title min-h-0 py-3 px-4 text-sm font-medium">
          {{ t('bugBounty.monitor.executionLogs') }}
          <span class="ml-2 text-xs text-base-content/50">({{ logs.length }})</span>
        </summary>
        <div class="collapse-content px-4 pb-4">
          <div class="space-y-2">
            <div
              v-for="(log, index) in logs"
              :key="`${log.plugin_id}-${log.occurred_at}-${index}`"
              class="rounded-lg border border-base-300 bg-base-100 px-3 py-2"
            >
              <div class="flex items-center justify-between gap-3">
                <div class="min-w-0">
                  <div class="flex items-center gap-2 min-w-0">
                    <span class="font-mono text-xs text-base-content/70">
                      #{{ log.step_index }}/{{ log.total_steps }}
                    </span>
                    <span class="font-medium truncate">{{ log.plugin_id }}</span>
                    <span class="badge badge-xs" :class="getLogBadgeClass(log.status)">
                      {{ getLogStatusLabel(log.status) }}
                    </span>
                  </div>
                  <div class="text-xs text-base-content/60 mt-1 break-words">
                    {{ log.message }}
                  </div>
                </div>
                <div class="text-right text-xs text-base-content/50 shrink-0">
                  <div v-if="log.duration_ms != null">{{ formatDuration(log.duration_ms) }}</div>
                  <div v-if="log.imported_assets_delta > 0" class="text-success">
                    +{{ log.imported_assets_delta }} {{ t('bugBounty.monitor.assetsImported') }}
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
      </details>

      <div class="flex flex-wrap gap-2 mt-2">
        <span v-if="task.config.enable_dns_monitoring" class="badge badge-outline badge-xs">
          <i class="fas fa-network-wired mr-1"></i>DNS
        </span>
        <span v-if="task.config.enable_cert_monitoring" class="badge badge-outline badge-xs">
          <i class="fas fa-certificate mr-1"></i>{{ t('bugBounty.monitor.cert') }}
        </span>
        <span v-if="task.config.enable_content_monitoring" class="badge badge-outline badge-xs">
          <i class="fas fa-file-alt mr-1"></i>{{ t('bugBounty.monitor.content') }}
        </span>
        <span v-if="task.config.enable_api_monitoring" class="badge badge-outline badge-xs">
          <i class="fas fa-plug mr-1"></i>API
        </span>
        <span v-if="task.config.enable_port_monitoring" class="badge badge-outline badge-xs">
          <i class="fas fa-network-wired mr-1"></i>Port
        </span>
        <span v-if="task.config.enable_web_monitoring" class="badge badge-outline badge-xs">
          <i class="fas fa-globe mr-1"></i>Web
        </span>
        <span v-if="task.config.enable_risk_monitoring" class="badge badge-outline badge-xs">
          <i class="fas fa-shield-alt mr-1"></i>Risk
        </span>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'

interface MonitorTaskProgress {
  status: string
  progress: number
  completed_steps: number
  total_steps: number
  current_plugin?: string | null
  target_count: number
  imported_assets: number
  message?: string | null
  execution_mode: string
}

interface MonitorTaskLog {
  plugin_id: string
  status: string
  step_index: number
  total_steps: number
  duration_ms?: number | null
  imported_assets_delta: number
  message: string
  occurred_at: string
}

const props = defineProps<{
  task: any
  isRunning: boolean
  stopping: boolean
  progress: MonitorTaskProgress | null
  logs: MonitorTaskLog[]
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

const progressSummary = computed(() => {
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

const getLogStatusLabel = (status: string) => {
  switch (status) {
    case 'completed':
      return t('bugBounty.monitor.progressCompleted')
    case 'failed':
      return t('bugBounty.monitor.progressFailed')
    case 'stopped':
      return t('bugBounty.monitor.progressStopped')
    default:
      return t('bugBounty.monitor.progressRunning')
  }
}

const getLogBadgeClass = (status: string) => {
  switch (status) {
    case 'completed':
      return 'badge-success'
    case 'failed':
      return 'badge-error'
    case 'stopped':
      return 'badge-warning'
    default:
      return 'badge-info'
  }
}

const formatDuration = (durationMs: number) => {
  if (durationMs >= 1000) return `${(durationMs / 1000).toFixed(1)}s`
  return `${durationMs}ms`
}

const formatInterval = (secs: number) => {
  const hours = secs / 3600
  const days = hours / 24
  if (days >= 1) return `${days} ${t('bugBounty.monitor.days')}`
  return `${hours} ${t('bugBounty.monitor.hours')}`
}

const formatDateTime = (dateStr: string) => new Date(dateStr).toLocaleString()
</script>
