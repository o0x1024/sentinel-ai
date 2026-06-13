<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import ObserverConfigDialog from '@/components/Bot/ObserverConfigDialog.vue'
import ObserverReportDetail from '@/components/Bot/ObserverReportDetail.vue'
import {
  activateMission,
  getMissionRuntimeDetail,
  listMissionRuns,
  listMissions,
  pauseMission,
  resumeMission,
  runMissionNow,
  type Mission,
  type MissionRun,
  type MissionRuntimeDetail,
} from '@/api/missions'

const { t } = useI18n()

const observer = ref<Mission | null>(null)
const runs = ref<MissionRun[]>([])
const selectedRunId = ref('')
const runtime = ref<MissionRuntimeDetail | null>(null)
const loading = ref(false)
const loadingRuns = ref(false)
const loadingRuntime = ref(false)
const error = ref('')
const showConfigDialog = ref(false)

const selectedRun = computed(() => runs.value.find((run) => run.id === selectedRunId.value) ?? null)

const scheduleLabel = computed(() => {
  if (!observer.value?.trigger_json) return '—'
  try {
    const trigger = JSON.parse(observer.value.trigger_json)
    if (trigger.kind === 'cron') {
      return `${trigger.cron_expr || trigger.expr || '—'} (${trigger.timezone || 'UTC'})`
    }
    return trigger.kind || '—'
  } catch {
    return observer.value.trigger_json
  }
})

const deliveryLabel = computed(() => {
  if (!observer.value?.delivery_policy_json) return '—'
  try {
    const policy = JSON.parse(observer.value.delivery_policy_json)
    return policy.primary?.kind || '—'
  } catch {
    return '—'
  }
})

async function loadObserver() {
  loading.value = true
  error.value = ''
  try {
    const missions = await listMissions({ ownerKind: 'observer', limit: 1 })
    observer.value = missions[0] ?? null
    if (observer.value) {
      await loadRuns(observer.value.id)
    } else {
      runs.value = []
      selectedRunId.value = ''
      runtime.value = null
    }
  } catch (e) {
    error.value = String(e)
  } finally {
    loading.value = false
  }
}

async function loadRuns(missionId: string) {
  loadingRuns.value = true
  try {
    runs.value = await listMissionRuns(missionId, 20, 0)
    selectedRunId.value = runs.value[0]?.id || ''
    if (selectedRunId.value) {
      await loadRuntime(missionId, selectedRunId.value)
    } else {
      runtime.value = null
    }
  } catch (e) {
    error.value = String(e)
  } finally {
    loadingRuns.value = false
  }
}

async function loadRuntime(missionId: string, runId: string) {
  loadingRuntime.value = true
  try {
    runtime.value = await getMissionRuntimeDetail(missionId, runId, 50)
  } catch (e) {
    error.value = String(e)
    runtime.value = null
  } finally {
    loadingRuntime.value = false
  }
}

async function selectRun(runId: string) {
  if (!observer.value || selectedRunId.value === runId) return
  selectedRunId.value = runId
  await loadRuntime(observer.value.id, runId)
}

async function handleRunNow() {
  if (!observer.value) return
  try {
    await runMissionNow(observer.value.id)
    await loadRuns(observer.value.id)
    await loadObserver()
  } catch (e) {
    error.value = String(e)
  }
}

async function handlePauseResume() {
  if (!observer.value) return
  try {
    if (observer.value.status === 'paused') {
      observer.value = await resumeMission(observer.value.id)
    } else {
      observer.value = await pauseMission(observer.value.id)
    }
    await loadObserver()
  } catch (e) {
    error.value = String(e)
  }
}

async function handleActivate() {
  if (!observer.value) return
  try {
    observer.value = await activateMission(observer.value.id)
    await loadObserver()
  } catch (e) {
    error.value = String(e)
  }
}

function openCreateDialog() {
  showConfigDialog.value = true
}

function openEditDialog() {
  showConfigDialog.value = true
}

function closeConfigDialog() {
  showConfigDialog.value = false
}

async function handleConfigSaved(mission: Mission) {
  observer.value = mission
  await loadObserver()
}

function formatDate(value?: string | null): string {
  if (!value) return '—'
  const date = new Date(value)
  return Number.isNaN(date.getTime()) ? value : date.toLocaleString()
}

onMounted(loadObserver)
</script>

<template>
  <div class="space-y-4">
    <div class="flex flex-col gap-3 lg:flex-row lg:items-start lg:justify-between">
      <div>
        <h3 class="text-lg font-semibold">{{ t('botConsole.observer.title') }}</h3>
        <p class="text-sm text-base-content/70">{{ t('botConsole.observer.description') }}</p>
      </div>
      <button class="btn btn-sm btn-outline" :disabled="loading" @click="loadObserver">
        {{ t('botConsole.refresh') }}
      </button>
    </div>

    <div v-if="error" class="alert alert-error"><span>{{ error }}</span></div>
    <div v-if="loading" class="text-sm text-base-content/60">{{ t('botConsole.loadingPeers') }}</div>

    <div v-else-if="!observer" class="rounded-lg border border-dashed border-base-300 p-6 text-center">
      <p class="text-sm text-base-content/70">{{ t('botConsole.observer.empty') }}</p>
      <button class="btn btn-sm btn-primary mt-4" @click="openCreateDialog">
        {{ t('botConsole.observer.create') }}
      </button>
    </div>

    <template v-else>
      <section class="rounded-lg border border-base-300 p-4">
        <div class="flex flex-col gap-3 lg:flex-row lg:items-center lg:justify-between">
          <div class="space-y-2">
            <div class="flex flex-wrap items-center gap-2">
              <span class="font-medium">{{ observer.title }}</span>
              <span class="badge badge-sm">{{ observer.status }}</span>
            </div>
            <div class="flex flex-wrap gap-2 text-xs text-base-content/70">
              <span>{{ t('botConsole.observer.schedule') }}: {{ scheduleLabel }}</span>
              <span v-if="observer.next_run_at">
                {{ t('botConsole.missions.nextRun') }}: {{ formatDate(observer.next_run_at) }}
              </span>
              <span>{{ t('botConsole.observer.deliveryChannel') }}: {{ deliveryLabel }}</span>
            </div>
          </div>
          <div class="flex flex-wrap gap-2">
            <button
              v-if="observer.status === 'draft'"
              class="btn btn-sm btn-primary"
              @click="handleActivate"
            >
              {{ t('botConsole.missions.activate') }}
            </button>
            <button
              v-if="observer.status === 'active' || observer.status === 'paused'"
              class="btn btn-sm btn-ghost"
              @click="handlePauseResume"
            >
              {{ observer.status === 'paused' ? t('botConsole.observer.resume') : t('botConsole.observer.pause') }}
            </button>
            <button class="btn btn-sm btn-ghost" @click="openEditDialog">
              {{ t('botConsole.observer.editConfig') }}
            </button>
            <button class="btn btn-sm btn-primary" @click="handleRunNow">
              {{ t('botConsole.observer.runNow') }}
            </button>
          </div>
        </div>
      </section>

      <div class="grid min-h-0 grid-cols-1 gap-4 xl:grid-cols-[20rem_minmax(0,1fr)]">
        <section class="rounded-lg border border-base-300 overflow-hidden">
          <div class="border-b border-base-300 px-4 py-3 text-sm font-semibold">
            {{ t('botConsole.observer.reports') }}
          </div>
          <div v-if="loadingRuns" class="p-4 text-sm text-base-content/60">
            {{ t('botConsole.observer.loadingReports') }}
          </div>
          <div v-else-if="runs.length === 0" class="p-4 text-sm text-base-content/60">
            {{ t('botConsole.observer.noReports') }}
          </div>
          <div v-else class="max-h-[32rem] overflow-y-auto">
            <button
              v-for="run in runs"
              :key="run.id"
              class="w-full border-b border-base-200 px-4 py-3 text-left hover:bg-base-200/60"
              :class="selectedRunId === run.id ? 'bg-primary/10' : ''"
              @click="selectRun(run.id)"
            >
              <div class="flex items-start justify-between gap-3">
                <div class="min-w-0">
                  <div class="truncate text-sm font-medium">
                    {{ run.result_summary || t('botConsole.observer.noSummary') }}
                  </div>
                  <div class="mt-1 text-xs text-base-content/60">
                    {{ run.trigger_kind }} · {{ run.started_at ? new Date(run.started_at).toLocaleString() : '—' }}
                  </div>
                </div>
                <span class="badge badge-xs">{{ run.status }}</span>
              </div>
            </button>
          </div>
        </section>

        <section>
          <ObserverReportDetail
            v-if="selectedRun"
            :run="selectedRun"
            :runtime="runtime"
            :loading="loadingRuntime"
          />
          <div v-else class="rounded-lg border border-base-300 p-6 text-sm text-base-content/60">
            {{ t('botConsole.observer.selectReport') }}
          </div>
        </section>
      </div>
    </template>

    <ObserverConfigDialog
      :open="showConfigDialog"
      :mission="observer"
      @close="closeConfigDialog"
      @saved="handleConfigSaved"
    />
  </div>
</template>
