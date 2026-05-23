<template>
  <div class="rounded-lg bg-base-200 transition-colors hover:bg-base-300">
    <div class="p-4">
      <div class="flex items-start justify-between gap-4">
        <div class="flex items-center gap-3 min-w-0 flex-1">
          <button
            type="button"
            class="btn btn-ghost btn-xs shrink-0"
            @click="expanded = !expanded"
            :title="expanded ? t('bugBounty.monitor.collapseGroup') : t('bugBounty.monitor.expandGroup')"
          >
            <i class="fas" :class="expanded ? 'fa-chevron-down' : 'fa-chevron-right'"></i>
          </button>
          <input
            type="checkbox"
            class="toggle toggle-success shrink-0"
            :checked="task.enabled"
            @change="emit('toggle', task)"
          />
          <div class="min-w-0 flex-1">
            <div class="flex flex-wrap items-center gap-2">
              <h4 class="font-medium truncate">{{ task.name }}</h4>
              <span v-if="task.enabled" class="badge badge-success badge-xs">
                {{ t('bugBounty.monitor.enabled') }}
              </span>
              <span v-else class="badge badge-ghost badge-xs">
                {{ t('bugBounty.monitor.disabled') }}
              </span>
            </div>
            <div class="text-xs text-base-content/60 mt-1 space-y-1">
              <div>
                <i class="fas fa-layer-group mr-1"></i>
                {{
                  t('bugBounty.monitor.groupedTaskPrograms', {
                    count: programDisplayNames.length,
                    names: programDisplayNames.join(', '),
                  })
                }}
              </div>
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
        <div class="flex gap-1 shrink-0">
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
    </div>

    <div v-if="expanded" class="border-t border-base-300 px-4 pb-4 pt-3">
      <div class="mb-2 text-xs font-medium text-base-content/60">
        {{ t('bugBounty.monitor.groupChildren', { count: childTasks.length }) }}
      </div>
      <div class="space-y-2">
        <MonitorTaskCard
          v-for="child in childTasks"
          :key="child.id"
          :task="child"
          :is-running="isChildRunning(child)"
          :stopping="isChildStopping(child)"
          :progress="getChildProgress(child)"
          @toggle="emit('toggle', child)"
          @discover="emit('discover', child)"
          @stop="emit('stop', child)"
          @trigger="emit('trigger', child)"
          @edit="emit('edit', child)"
          @delete="emit('delete', child)"
        />
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'

import type { MonitorTaskProgressState as MonitorTaskProgress } from '../../composables/monitorTaskProgressSupport'
import MonitorTaskCard from './MonitorTaskCard.vue'
import { getMonitorTaskChildren } from './monitorTaskGrouping'

const props = defineProps<{
  task: any
  isRunning: boolean
  stopping: boolean
  progress: MonitorTaskProgress | null
  isChildRunning: (task: any) => boolean
  isChildStopping: (task: any) => boolean
  getChildProgress: (task: any) => MonitorTaskProgress | null
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
const expanded = ref(false)
const childTasks = computed(() => getMonitorTaskChildren(props.task))
const programDisplayNames = computed(() =>
  Array.isArray(props.task?.program_names) ? props.task.program_names : []
)

const formatInterval = (seconds: number) => {
  if (seconds < 3600) return `${Math.floor(seconds / 60)} ${t('bugBounty.monitor.minutes')}`
  if (seconds < 86400) return `${Math.floor(seconds / 3600)} ${t('bugBounty.monitor.hours')}`
  return `${Math.floor(seconds / 86400)} ${t('bugBounty.monitor.days')}`
}

const formatDateTime = (dateStr: string) => {
  return new Date(dateStr).toLocaleString()
}
</script>
