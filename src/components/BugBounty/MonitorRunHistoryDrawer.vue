<template>
  <Teleport to="body">
    <div v-if="open" class="fixed inset-0 z-[70]">
      <div class="absolute inset-0 bg-black/30 backdrop-blur-[1px]" @click="emit('close')"></div>
      <Transition name="slide-drawer-right">
        <aside
          v-if="open"
          class="absolute inset-y-0 right-0 flex w-full max-w-3xl flex-col overflow-hidden bg-base-100 shadow-2xl"
        >
          <div class="border-b border-base-200 px-5 py-4">
            <div class="flex items-start justify-between gap-4">
              <div>
                <h3 class="text-lg font-semibold">{{ t('bugBounty.monitor.runHistoryTitle') }}</h3>
                <p class="mt-1 text-sm text-base-content/60">
                  {{ t('bugBounty.monitor.runHistoryDescription') }}
                </p>
              </div>
              <button type="button" class="btn btn-sm btn-ghost" @click="emit('close')">
                <i class="fas fa-times"></i>
              </button>
            </div>

            <div class="mt-4 flex flex-col gap-3 md:flex-row">
              <label class="input input-bordered flex flex-1 items-center gap-2">
                <i class="fas fa-search text-base-content/50"></i>
                <input v-model.trim="search" type="text" class="grow" :placeholder="t('bugBounty.monitor.runHistorySearchPlaceholder')" />
              </label>
              <select v-model="statusFilter" class="select select-bordered md:w-48">
                <option value="all">{{ t('bugBounty.monitor.runHistoryAllStatuses') }}</option>
                <option value="running">{{ t('bugBounty.monitor.running') }}</option>
                <option value="completed">{{ t('bugBounty.monitor.progressCompleted') }}</option>
                <option value="failed">{{ t('bugBounty.monitor.progressFailed') }}</option>
              </select>
              <button type="button" class="btn btn-outline" :disabled="loading" @click="loadHistory">
                <i class="fas fa-rotate-right mr-2"></i>
                {{ t('bugBounty.surface.refresh') }}
              </button>
            </div>
          </div>

          <div class="flex-1 overflow-y-auto px-5 py-4">
            <div v-if="loading" class="flex justify-center py-10">
              <span class="loading loading-spinner loading-lg"></span>
            </div>

            <div v-else-if="filteredRuns.length === 0" class="rounded-xl border border-dashed border-base-300 px-6 py-10 text-center text-sm text-base-content/60">
              {{ t('bugBounty.monitor.runHistoryEmpty') }}
            </div>

            <div v-else class="space-y-3">
              <article v-for="run in filteredRuns" :key="run.run_id" class="rounded-2xl border border-base-200 bg-base-100 p-4 shadow-sm">
                <div class="flex flex-col gap-3 lg:flex-row lg:items-start lg:justify-between">
                  <div class="min-w-0">
                    <div class="flex flex-wrap items-center gap-2">
                      <h4 class="truncate text-base font-semibold">{{ run.task_name || t('bugBounty.monitor.unknownTask') }}</h4>
                      <span class="badge badge-sm" :class="statusBadgeClass(run.status)">{{ run.status }}</span>
                      <span v-if="run.execution_mode" class="badge badge-outline badge-sm">{{ run.execution_mode }}</span>
                      <span class="badge badge-ghost badge-sm">{{ programLabel(run.program_id) }}</span>
                    </div>
                    <div class="mt-2 flex flex-wrap gap-x-4 gap-y-1 text-xs text-base-content/60">
                      <span>{{ t('bugBounty.monitor.runHistoryTaskId') }}: {{ run.task_id || '-' }}</span>
                      <span>{{ t('bugBounty.monitor.runHistoryPlugin') }}: {{ run.plugin_id || '-' }}</span>
                      <span>{{ t('bugBounty.monitor.runHistoryTrigger') }}: {{ run.trigger_source }}</span>
                    </div>
                  </div>

                  <div class="grid min-w-0 grid-cols-2 gap-2 text-xs text-base-content/70 lg:min-w-[16rem]">
                    <div>
                      <div class="text-base-content/50">{{ t('bugBounty.monitor.runHistoryStartedAt') }}</div>
                      <div class="mt-1 font-medium">{{ formatDateTime(run.started_at) }}</div>
                    </div>
                    <div>
                      <div class="text-base-content/50">{{ t('bugBounty.monitor.runHistoryCompletedAt') }}</div>
                      <div class="mt-1 font-medium">{{ formatDateTime(run.completed_at) }}</div>
                    </div>
                    <div>
                      <div class="text-base-content/50">{{ t('bugBounty.monitor.runHistoryDuration') }}</div>
                      <div class="mt-1 font-medium">{{ formatDuration(run.started_at, run.completed_at) }}</div>
                    </div>
                    <div>
                      <div class="text-base-content/50">{{ t('bugBounty.monitor.runHistoryRunId') }}</div>
                      <div class="mt-1 truncate font-mono">{{ run.run_id }}</div>
                    </div>
                  </div>
                </div>

                <div class="mt-4 grid grid-cols-1 gap-3 md:grid-cols-3">
                  <div class="rounded-xl bg-base-200/70 px-3 py-3">
                    <div class="text-xs text-base-content/60">{{ t('bugBounty.monitor.runHistoryObservations') }}</div>
                    <div class="mt-1 text-lg font-semibold">{{ run.observation_count }}</div>
                  </div>
                  <div class="rounded-xl bg-base-200/70 px-3 py-3">
                    <div class="text-xs text-base-content/60">{{ t('bugBounty.monitor.runHistoryImportedAssets') }}</div>
                    <div class="mt-1 text-lg font-semibold">{{ runImportedOrEnrichedCount(run) }}</div>
                  </div>
                  <div class="rounded-xl bg-base-200/70 px-3 py-3">
                    <div class="text-xs text-base-content/60">{{ t('bugBounty.monitor.runHistoryChangedAssets') }}</div>
                    <div class="mt-1 text-lg font-semibold">{{ run.changed_asset_count }}</div>
                  </div>
                </div>

                <div v-if="run.error_message" class="alert alert-warning mt-4 py-3 text-sm">
                  <i class="fas fa-triangle-exclamation"></i>
                  <span>{{ run.error_message }}</span>
                </div>
              </article>
            </div>
          </div>
        </aside>
      </Transition>
    </div>
  </Teleport>
</template>

<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useI18n } from 'vue-i18n'
import { useToast } from '../../composables/useToast'

interface MonitorRunHistoryItem {
  run_id: string
  program_id: string
  task_id?: string | null
  task_name?: string | null
  trigger_source: string
  execution_mode?: string | null
  plugin_id?: string | null
  status: string
  observation_count: number
  imported_asset_count: number
  changed_asset_count: number
  error_message?: string | null
  started_at: string
  completed_at?: string | null
}

const props = defineProps<{
  open: boolean
  programs?: any[]
}>()

const emit = defineEmits<{
  (e: 'close'): void
}>()

const { t } = useI18n()
const toast = useToast()

const loading = ref(false)
const search = ref('')
const statusFilter = ref<'all' | 'running' | 'completed' | 'failed'>('all')
const runs = ref<MonitorRunHistoryItem[]>([])
const programNameById = computed(() => new Map(
  (Array.isArray(props.programs) ? props.programs : []).map(program => [String(program.id), String(program.name || program.id)]),
))

const filteredRuns = computed(() => {
  const needle = search.value.trim().toLowerCase()
  return runs.value.filter(run => {
    if (statusFilter.value !== 'all' && run.status !== statusFilter.value) return false
    if (!needle) return true
    return [
      run.run_id,
      run.task_id || '',
      run.task_name || '',
      programLabel(run.program_id),
      run.plugin_id || '',
      run.trigger_source,
      run.execution_mode || '',
      run.error_message || '',
    ].some(value => value.toLowerCase().includes(needle))
  })
})

const runImportedOrEnrichedCount = (run: MonitorRunHistoryItem) =>
  Number(run.imported_asset_count || 0) + Number(run.changed_asset_count || 0)

const loadHistory = async () => {
  if (!props.open) {
    runs.value = []
    return
  }

  try {
    loading.value = true
    runs.value = await invoke<MonitorRunHistoryItem[]>('monitor_list_run_history', {
      programId: null,
      limit: 100,
    })
  } catch (error) {
    console.error('Failed to load monitor run history:', error)
    toast.error(t('bugBounty.errors.loadFailed'))
    runs.value = []
  } finally {
    loading.value = false
  }
}

const formatDateTime = (value?: string | null) => {
  if (!value) return '-'
  const parsed = new Date(value)
  if (Number.isNaN(parsed.getTime())) return value
  return parsed.toLocaleString()
}

const formatDuration = (startedAt?: string | null, completedAt?: string | null) => {
  if (!startedAt || !completedAt) return '-'
  const start = new Date(startedAt).getTime()
  const end = new Date(completedAt).getTime()
  if (!Number.isFinite(start) || !Number.isFinite(end) || end < start) return '-'
  const seconds = Math.round((end - start) / 1000)
  if (seconds < 60) return `${seconds}s`
  if (seconds < 3600) return `${Math.floor(seconds / 60)}m ${seconds % 60}s`
  return `${Math.floor(seconds / 3600)}h ${Math.floor((seconds % 3600) / 60)}m`
}

const statusBadgeClass = (status: string) => {
  switch (String(status || '').toLowerCase()) {
    case 'completed':
      return 'badge-success'
    case 'failed':
      return 'badge-error'
    case 'running':
      return 'badge-info'
    default:
      return 'badge-ghost'
  }
}

const programLabel = (programId: string) => programNameById.value.get(String(programId)) || String(programId || '-')

watch(
  () => props.open,
  async (open) => {
    if (open) await loadHistory()
  },
  { immediate: true },
)
</script>

<style scoped>
.slide-drawer-right-enter-active,
.slide-drawer-right-leave-active {
  transition: transform 0.28s cubic-bezier(0.4, 0, 0.2, 1);
}

.slide-drawer-right-enter-from,
.slide-drawer-right-leave-to {
  transform: translateX(100%);
}
</style>
