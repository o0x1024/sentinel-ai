<script setup lang="ts">
import { ref, computed, onMounted, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useAssistantProfiles } from '@/components/Agent/assistantProfiles'
import {
  listMissions,
  listMissionRuns,
  createMission,
  activateMission,
  pauseMission,
  resumeMission,
  archiveMission,
  deleteMission,
  runMissionNow,
  type Mission,
  type MissionRun,
  type ListMissionsFilter,
  type CreateMissionRequest,
} from '@/api/missions'

const props = defineProps<{
  ownerKind?: string
  ownerRef?: string
}>()

const { t } = useI18n()
const { profileOptions, loadAssistantProfiles } = useAssistantProfiles()

const missions = ref<Mission[]>([])
const selectedMission = ref<Mission | null>(null)
const missionRuns = ref<MissionRun[]>([])
const loading = ref(false)
const loadingRuns = ref(false)
const statusFilter = ref<string>('all')
const error = ref('')

// Create dialog
const showCreateDialog = ref(false)
const createForm = ref({
  title: '',
  objective: '',
  triggerKind: 'cron' as 'cron' | 'interval' | 'manual',
  cronExpr: '0 9 * * *',
  intervalSeconds: 3600,
  timezone: 'UTC',
  assistantProfileId: '',
  contextStrategy: 'stateless',
  missedRunPolicy: 'skip',
  deliveryTarget: 'app_notification',
  onSuccess: 'summary',
  onFailure: 'immediate',
  maxRunsPerDay: 10,
  timeoutSeconds: 300,
})
const createError = ref('')
const creating = ref(false)

const statusOptions = ['all', 'draft', 'active', 'paused', 'blocked', 'completed', 'failed', 'archived']

const statusBadgeClass = (status: string) => {
  const map: Record<string, string> = {
    draft: 'badge-ghost',
    active: 'badge-success',
    paused: 'badge-warning',
    blocked: 'badge-error',
    completed: 'badge-info',
    failed: 'badge-error',
    archived: 'badge-neutral',
    queued: 'badge-ghost',
    running: 'badge-info',
    succeeded: 'badge-success',
    partial: 'badge-warning',
    timed_out: 'badge-error',
    cancelled: 'badge-neutral',
  }
  return map[status] || 'badge-ghost'
}

const filteredMissions = computed(() => {
  if (statusFilter.value === 'all') return missions.value
  return missions.value.filter((m) => m.status === statusFilter.value)
})

async function loadMissions() {
  loading.value = true
  error.value = ''
  try {
    const filter: ListMissionsFilter = { limit: 100, offset: 0 }
    if (props.ownerKind) filter.ownerKind = props.ownerKind
    if (props.ownerRef) filter.ownerRef = props.ownerRef
    missions.value = await listMissions(filter)
  } catch (e) {
    error.value = String(e)
  } finally {
    loading.value = false
  }
}

async function selectMission(mission: Mission) {
  selectedMission.value = mission
  await loadRuns(mission.id)
}

async function loadRuns(missionId: string) {
  loadingRuns.value = true
  try {
    missionRuns.value = await listMissionRuns(missionId, 50)
  } catch (e) {
    console.error('Failed to load mission runs:', e)
  } finally {
    loadingRuns.value = false
  }
}

async function handlePause(id: string) {
  try {
    await pauseMission(id)
    await loadMissions()
  } catch (e) {
    error.value = String(e)
  }
}

async function handleResume(id: string) {
  try {
    await resumeMission(id)
    await loadMissions()
  } catch (e) {
    error.value = String(e)
  }
}

async function handleArchive(id: string) {
  try {
    await archiveMission(id)
    await loadMissions()
  } catch (e) {
    error.value = String(e)
  }
}

async function handleDelete(id: string) {
  try {
    await deleteMission(id)
    if (selectedMission.value?.id === id) selectedMission.value = null
    await loadMissions()
  } catch (e) {
    error.value = String(e)
  }
}

async function handleRunNow(id: string) {
  try {
    await runMissionNow(id)
    if (selectedMission.value?.id === id) {
      await loadRuns(id)
    }
    await loadMissions()
  } catch (e) {
    error.value = String(e)
  }
}

async function handleActivate(id: string) {
  try {
    await activateMission(id)
    await loadMissions()
    if (selectedMission.value?.id === id) {
      selectedMission.value = missions.value.find((m) => m.id === id) || null
    }
  } catch (e) {
    error.value = String(e)
  }
}

function openCreateDialog() {
  createForm.value = {
    title: '',
    objective: '',
    triggerKind: 'cron',
    cronExpr: '0 9 * * *',
    intervalSeconds: 3600,
    timezone: 'UTC',
    assistantProfileId: '',
    contextStrategy: 'stateless',
    missedRunPolicy: 'skip',
    deliveryTarget: 'app_notification',
    onSuccess: 'summary',
    onFailure: 'immediate',
    maxRunsPerDay: 10,
    timeoutSeconds: 300,
  }
  createError.value = ''
  showCreateDialog.value = true
}

async function handleCreate() {
  const f = createForm.value
  if (!f.title.trim() || !f.objective.trim()) {
    createError.value = t('botConsole.missions.create.errorRequired')
    return
  }

  creating.value = true
  createError.value = ''

  try {
    let triggerJson: string | undefined
    if (f.triggerKind === 'cron') {
      triggerJson = JSON.stringify({ kind: 'cron', cron_expr: f.cronExpr, timezone: f.timezone })
    } else if (f.triggerKind === 'interval') {
      triggerJson = JSON.stringify({ kind: 'interval', interval_seconds: f.intervalSeconds })
    } else {
      triggerJson = JSON.stringify({ kind: 'manual' })
    }

    const deliveryPolicyJson = JSON.stringify({
      onSuccess: f.onSuccess,
      onChange: 'none',
      onFailure: f.onFailure,
      primary: { kind: f.deliveryTarget, refData: {} },
    })

    const budgetJson = JSON.stringify({
      max_runs_per_day: f.maxRunsPerDay,
      timeout_seconds: f.timeoutSeconds,
    })

    const contextStrategyJson = JSON.stringify({ mode: f.contextStrategy })

    const request: CreateMissionRequest = {
      title: f.title.trim(),
      objective: f.objective.trim(),
      ownerKind: props.ownerKind || 'user',
      ownerRef: props.ownerRef || 'default',
      triggerJson,
      deliveryPolicyJson,
      assistantProfileId: f.assistantProfileId || undefined,
      contextStrategyJson,
      budgetJson,
      missedRunPolicy: f.missedRunPolicy,
    }

    const mission = await createMission(request)
    showCreateDialog.value = false
    await loadMissions()
    selectedMission.value = mission
  } catch (e) {
    createError.value = String(e)
  } finally {
    creating.value = false
  }
}

function formatDate(dateStr: string | null): string {
  if (!dateStr) return '-'
  try {
    return new Date(dateStr).toLocaleString()
  } catch {
    return dateStr
  }
}

watch(() => [props.ownerKind, props.ownerRef], loadMissions, { immediate: false })
onMounted(() => {
  loadMissions()
  loadAssistantProfiles()
})
</script>

<template>
  <div class="flex flex-col h-full">
    <!-- Header -->
    <div class="flex items-center justify-between px-4 py-2 border-b border-base-300">
      <div class="flex items-center gap-2">
        <select v-model="statusFilter" class="select select-bordered select-xs">
          <option v-for="s in statusOptions" :key="s" :value="s">
            {{ s === 'all' ? t('botConsole.missions.allStatuses') : s }}
          </option>
        </select>
        <span class="text-xs text-base-content/50">
          {{ filteredMissions.length }} {{ t('botConsole.missions.missions') }}
        </span>
      </div>
      <div class="flex items-center gap-2">
        <button class="btn btn-primary btn-xs" @click="openCreateDialog">
          + {{ t('botConsole.missions.create.button') }}
        </button>
        <button class="btn btn-ghost btn-xs" @click="loadMissions" :disabled="loading">
          {{ t('botConsole.missions.refresh') }}
        </button>
      </div>
    </div>

    <!-- Error -->
    <div v-if="error" class="alert alert-error mx-4 mt-2 text-sm">
      {{ error }}
    </div>

    <!-- Loading -->
    <div v-if="loading" class="flex items-center justify-center py-8">
      <span class="loading loading-spinner loading-sm"></span>
    </div>

    <!-- Content -->
    <div v-else class="flex flex-1 overflow-hidden">
      <!-- Mission List -->
      <div class="w-1/3 border-r border-base-300 overflow-y-auto">
        <div
          v-for="mission in filteredMissions"
          :key="mission.id"
          class="px-3 py-2 border-b border-base-200 cursor-pointer hover:bg-base-200/50 transition-colors"
          :class="{ 'bg-primary/10': selectedMission?.id === mission.id }"
          @click="selectMission(mission)"
        >
          <div class="flex items-center justify-between">
            <span class="font-medium text-sm truncate">{{ mission.title }}</span>
            <span class="badge badge-xs" :class="statusBadgeClass(mission.status)">
              {{ mission.status }}
            </span>
          </div>
          <div class="text-xs text-base-content/50 mt-0.5 truncate">
            {{ mission.objective }}
          </div>
          <div class="flex items-center gap-2 mt-1 text-xs text-base-content/40">
            <span>{{ t('botConsole.missions.runs') }}: {{ mission.run_count }}</span>
            <span v-if="mission.next_run_at">
              {{ t('botConsole.missions.nextRun') }}: {{ formatDate(mission.next_run_at) }}
            </span>
          </div>
        </div>
        <div v-if="filteredMissions.length === 0" class="flex items-center justify-center py-8 text-base-content/40 text-sm">
          {{ t('botConsole.missions.noMissions') }}
        </div>
      </div>

      <!-- Mission Detail -->
      <div class="flex-1 overflow-y-auto">
        <div v-if="!selectedMission" class="flex items-center justify-center h-full text-base-content/40 text-sm">
          {{ t('botConsole.missions.selectMission') }}
        </div>
        <div v-else class="p-4 space-y-4">
          <!-- Header -->
          <div class="flex items-center justify-between">
            <div>
              <h3 class="font-semibold text-lg">{{ selectedMission.title }}</h3>
              <p class="text-sm text-base-content/60">{{ selectedMission.objective }}</p>
            </div>
            <span class="badge" :class="statusBadgeClass(selectedMission.status)">
              {{ selectedMission.status }}
            </span>
          </div>

          <!-- Actions -->
          <div class="flex gap-2 flex-wrap">
            <button
              v-if="selectedMission.status === 'draft'"
              class="btn btn-success btn-xs"
              @click="handleActivate(selectedMission.id)"
            >
              {{ t('botConsole.missions.activate') }}
            </button>
            <button
              v-if="selectedMission.status === 'active'"
              class="btn btn-warning btn-xs"
              @click="handlePause(selectedMission.id)"
            >
              {{ t('botConsole.missions.pause') }}
            </button>
            <button
              v-if="selectedMission.status === 'paused' || selectedMission.status === 'failed' || selectedMission.status === 'blocked'"
              class="btn btn-success btn-xs"
              @click="handleResume(selectedMission.id)"
            >
              {{ t('botConsole.missions.resume') }}
            </button>
            <button
              v-if="selectedMission.status === 'active' || selectedMission.status === 'paused'"
              class="btn btn-primary btn-xs"
              @click="handleRunNow(selectedMission.id)"
            >
              {{ t('botConsole.missions.runNow') }}
            </button>
            <button
              v-if="selectedMission.status !== 'archived'"
              class="btn btn-ghost btn-xs"
              @click="handleArchive(selectedMission.id)"
            >
              {{ t('botConsole.missions.archive') }}
            </button>
            <button
              v-if="selectedMission.status === 'draft' || selectedMission.status === 'archived'"
              class="btn btn-error btn-xs btn-outline"
              @click="handleDelete(selectedMission.id)"
            >
              {{ t('botConsole.missions.delete') }}
            </button>
          </div>

          <!-- Info Grid -->
          <div class="grid grid-cols-2 gap-2 text-sm">
            <div>
              <span class="text-base-content/50">{{ t('botConsole.missions.owner') }}:</span>
              {{ selectedMission.owner_kind }} / {{ selectedMission.owner_ref }}
            </div>
            <div>
              <span class="text-base-content/50">{{ t('botConsole.missions.runCount') }}:</span>
              {{ selectedMission.run_count }}
            </div>
            <div>
              <span class="text-base-content/50">{{ t('botConsole.missions.lastRun') }}:</span>
              {{ formatDate(selectedMission.last_run_at) }}
            </div>
            <div>
              <span class="text-base-content/50">{{ t('botConsole.missions.nextRun') }}:</span>
              {{ formatDate(selectedMission.next_run_at) }}
            </div>
            <div v-if="selectedMission.assistant_profile_id">
              <span class="text-base-content/50">{{ t('botConsole.missions.profile') }}:</span>
              {{ selectedMission.assistant_profile_id }}
            </div>
            <div v-if="selectedMission.missed_run_policy">
              <span class="text-base-content/50">{{ t('botConsole.missions.missedPolicy') }}:</span>
              {{ selectedMission.missed_run_policy }}
            </div>
          </div>

          <!-- Last Error -->
          <div v-if="selectedMission.last_error" class="alert alert-error text-sm">
            {{ selectedMission.last_error }}
          </div>

          <!-- Runs -->
          <div>
            <h4 class="font-medium text-sm mb-2">{{ t('botConsole.missions.runHistory') }}</h4>
            <div v-if="loadingRuns" class="flex items-center justify-center py-4">
              <span class="loading loading-spinner loading-xs"></span>
            </div>
            <div v-else-if="missionRuns.length === 0" class="text-sm text-base-content/40 py-2">
              {{ t('botConsole.missions.noRuns') }}
            </div>
            <div v-else class="space-y-1">
              <div
                v-for="run in missionRuns"
                :key="run.id"
                class="border border-base-200 rounded-lg p-3 text-sm"
              >
                <div class="flex items-center justify-between">
                  <span class="font-medium">#{{ run.run_index }}</span>
                  <span class="badge badge-xs" :class="statusBadgeClass(run.status)">
                    {{ run.status }}
                  </span>
                </div>
                <div class="text-xs text-base-content/50 mt-1">
                  {{ run.trigger_kind }} · {{ formatDate(run.started_at) }}
                  <span v-if="run.completed_at"> → {{ formatDate(run.completed_at) }}</span>
                </div>
                <div v-if="run.result_summary" class="text-xs mt-1 text-base-content/70 line-clamp-2">
                  {{ run.result_summary }}
                </div>
                <div v-if="run.error_message" class="text-xs mt-1 text-error">
                  {{ run.error_message }}
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- Create Mission Dialog -->
    <dialog class="modal" :class="{ 'modal-open': showCreateDialog }">
      <div class="modal-box w-11/12 max-w-2xl">
        <h3 class="font-bold text-lg mb-4">{{ t('botConsole.missions.create.title') }}</h3>

        <div v-if="createError" class="alert alert-error text-sm mb-4">{{ createError }}</div>

        <div class="space-y-4">
          <!-- Title -->
          <div class="form-control">
            <label class="label"><span class="label-text">{{ t('botConsole.missions.create.missionTitle') }} *</span></label>
            <input v-model="createForm.title" type="text" class="input input-bordered input-sm" :placeholder="t('botConsole.missions.create.titlePlaceholder')" />
          </div>

          <!-- Objective -->
          <div class="form-control">
            <label class="label"><span class="label-text">{{ t('botConsole.missions.create.objective') }} *</span></label>
            <textarea v-model="createForm.objective" class="textarea textarea-bordered textarea-sm" rows="3" :placeholder="t('botConsole.missions.create.objectivePlaceholder')"></textarea>
          </div>

          <div class="grid grid-cols-2 gap-4">
            <!-- Trigger Kind -->
            <div class="form-control">
              <label class="label"><span class="label-text">{{ t('botConsole.missions.create.trigger') }}</span></label>
              <select v-model="createForm.triggerKind" class="select select-bordered select-sm">
                <option value="cron">Cron</option>
                <option value="interval">{{ t('botConsole.missions.create.interval') }}</option>
                <option value="manual">{{ t('botConsole.missions.create.manual') }}</option>
              </select>
            </div>

            <!-- Cron -->
            <div v-if="createForm.triggerKind === 'cron'" class="form-control">
              <label class="label"><span class="label-text">Cron</span></label>
              <input v-model="createForm.cronExpr" type="text" class="input input-bordered input-sm font-mono" placeholder="0 9 * * *" />
            </div>

            <!-- Interval -->
            <div v-if="createForm.triggerKind === 'interval'" class="form-control">
              <label class="label"><span class="label-text">{{ t('botConsole.missions.create.intervalSec') }}</span></label>
              <input v-model.number="createForm.intervalSeconds" type="number" class="input input-bordered input-sm" min="60" />
            </div>
          </div>

          <div class="grid grid-cols-2 gap-4">
            <!-- Profile -->
            <div class="form-control">
              <label class="label"><span class="label-text">{{ t('botConsole.missions.create.profile') }}</span></label>
              <select v-model="createForm.assistantProfileId" class="select select-bordered select-sm">
                <option value="">{{ t('botConsole.missions.create.defaultProfile') }}</option>
                <option v-for="p in profileOptions" :key="p.id" :value="p.id">
                  {{ p.label }}
                </option>
              </select>
            </div>

            <!-- Context Strategy -->
            <div class="form-control">
              <label class="label"><span class="label-text">{{ t('botConsole.missions.create.context') }}</span></label>
              <select v-model="createForm.contextStrategy" class="select select-bordered select-sm">
                <option value="stateless">Stateless</option>
                <option value="incremental">Incremental</option>
                <option value="cumulative">Cumulative</option>
              </select>
            </div>
          </div>

          <div class="grid grid-cols-2 gap-4">
            <!-- Delivery -->
            <div class="form-control">
              <label class="label"><span class="label-text">{{ t('botConsole.missions.create.delivery') }}</span></label>
              <select v-model="createForm.deliveryTarget" class="select select-bordered select-sm">
                <option value="app_notification">{{ t('botConsole.missions.create.appNotification') }}</option>
                <option value="webhook">Webhook</option>
              </select>
            </div>

            <!-- Missed Run Policy -->
            <div class="form-control">
              <label class="label"><span class="label-text">{{ t('botConsole.missions.create.missedPolicy') }}</span></label>
              <select v-model="createForm.missedRunPolicy" class="select select-bordered select-sm">
                <option value="skip">{{ t('botConsole.missions.create.policySkip') }}</option>
                <option value="run_once_on_startup">{{ t('botConsole.missions.create.policyRunOnce') }}</option>
                <option value="run_all_missed">{{ t('botConsole.missions.create.policyRunAll') }}</option>
              </select>
            </div>
          </div>

          <div class="grid grid-cols-2 gap-4">
            <!-- Max runs per day -->
            <div class="form-control">
              <label class="label"><span class="label-text">{{ t('botConsole.missions.create.maxRunsPerDay') }}</span></label>
              <input v-model.number="createForm.maxRunsPerDay" type="number" class="input input-bordered input-sm" min="1" />
            </div>
            <!-- Timeout -->
            <div class="form-control">
              <label class="label"><span class="label-text">{{ t('botConsole.missions.create.timeout') }}</span></label>
              <input v-model.number="createForm.timeoutSeconds" type="number" class="input input-bordered input-sm" min="30" />
            </div>
          </div>
        </div>

        <div class="modal-action">
          <button class="btn btn-ghost btn-sm" @click="showCreateDialog = false" :disabled="creating">
            {{ t('botConsole.missions.create.cancel') }}
          </button>
          <button class="btn btn-primary btn-sm" @click="handleCreate" :disabled="creating">
            <span v-if="creating" class="loading loading-spinner loading-xs"></span>
            {{ t('botConsole.missions.create.createDraft') }}
          </button>
        </div>
      </div>
      <form method="dialog" class="modal-backdrop">
        <button @click="showCreateDialog = false">close</button>
      </form>
    </dialog>
  </div>
</template>
