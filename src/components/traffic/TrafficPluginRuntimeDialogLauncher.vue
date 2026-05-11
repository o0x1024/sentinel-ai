<template>
  <div class="contents">
    <button
      :class="[buttonClass, launcherSummary ? 'h-auto min-h-0 flex-wrap justify-start py-2' : '']"
      type="button"
      @click="openDialog"
    >
      <span class="inline-flex items-center gap-2">
        <i v-if="iconClass" :class="iconClass"></i>
        <span>{{ resolvedButtonLabel }}</span>
      </span>
      <span v-if="launcherSummary" class="inline-flex flex-wrap items-center gap-1 text-[11px]">
        <span
          class="tooltip tooltip-bottom"
          :data-tip="getTrafficPluginRuntimePendingTooltip(launcherSummary)"
        >
          <span
            class="rounded px-2 py-1 font-mono"
            :class="summaryChipClass(getLauncherPendingLevel(launcherSummary))"
          >
            P {{ launcherSummary.pendingCount }}/{{ launcherSummary.pendingLimit }}
          </span>
        </span>
        <span
          class="tooltip tooltip-bottom"
          :data-tip="getTrafficPluginRuntimeRunningTooltip(launcherSummary)"
        >
          <span class="rounded bg-base-200/70 px-2 py-1 font-mono text-base-content/70">
            R {{ launcherSummary.runningCount }}/{{ launcherSummary.runningLimit }}
          </span>
        </span>
        <span
          class="tooltip tooltip-bottom"
          :data-tip="getTrafficPluginRuntimeRejectedTooltip(launcherSummary)"
        >
          <span
            class="rounded px-2 py-1 font-mono"
            :class="summaryChipClass(getRejectedTrendLevel(launcherSummary.rejectedSeries))"
          >
            X {{ launcherSummary.rejectedCount }}
          </span>
        </span>
      </span>
    </button>

    <AppDialog :class="['modal', { 'modal-open': dialogOpen }]" @click.self="closeDialog">
      <div v-if="dialogOpen" class="modal-box flex max-h-[90vh] w-11/12 max-w-7xl flex-col p-0">
        <div
          class="flex items-start justify-between gap-4 border-b border-base-300 bg-base-200 px-6 py-4"
        >
          <div>
            <h3 class="text-lg font-bold">
              {{ resolvedTitle }}
            </h3>
            <p class="mt-1 text-sm text-base-content/70">
              {{ resolvedDescription }}
            </p>
          </div>
          <button class="btn btn-sm btn-circle btn-ghost" type="button" @click="closeDialog">
            ✕
          </button>
        </div>

        <div class="flex-1 overflow-y-auto px-6 py-5">
          <div v-if="loading" class="flex min-h-[16rem] items-center justify-center">
            <span class="loading loading-spinner loading-lg"></span>
          </div>
          <div v-else class="space-y-4">
            <div v-if="loadError" class="alert alert-error">
              <i class="fas fa-circle-exclamation"></i>
              <span>{{ loadError }}</span>
            </div>

            <div v-if="undoRecommendationState" class="alert alert-success">
              <i class="fas fa-rotate-left"></i>
              <div class="min-w-0 flex-1">
                <div class="font-medium">
                  已应用推荐预设：{{ getPresetLabel(undoRecommendationState.appliedPreset) }}
                </div>
                <div class="text-sm">已保存当前推荐预设。你可以撤销到刚才的配置。</div>
              </div>
              <button
                class="btn btn-sm btn-outline"
                :disabled="isSavingTrafficPluginRuntimeSettings"
                type="button"
                @click="undoRecommendedPreset"
              >
                <span
                  v-if="isSavingTrafficPluginRuntimeSettings"
                  class="loading loading-spinner loading-xs"
                ></span>
                撤销到刚才配置
              </button>
            </div>

            <div
              v-if="activeRecommendation"
              class="alert"
              :class="activeRecommendation.severity === 'error' ? 'alert-warning' : 'alert-info'"
            >
              <i
                class="fas"
                :class="
                  activeRecommendation.severity === 'error'
                    ? 'fa-triangle-exclamation'
                    : 'fa-circle-info'
                "
              ></i>
              <div class="min-w-0 flex-1">
                <div class="font-medium">
                  建议应用预设：{{ getPresetLabel(activeRecommendation.preset) }}
                </div>
                <div class="text-sm">
                  {{ activeRecommendation.reason }}
                </div>
                <div class="mt-1 text-xs text-base-content/70">
                  {{ activeRecommendation.alternativeExplanation }}
                </div>
              </div>
              <button
                class="btn btn-sm"
                :class="activeRecommendation.severity === 'error' ? 'btn-warning' : 'btn-info'"
                :disabled="isSavingTrafficPluginRuntimeSettings"
                type="button"
                @click="applyRecommendedPreset"
              >
                <span
                  v-if="isSavingTrafficPluginRuntimeSettings"
                  class="loading loading-spinner loading-xs"
                ></span>
                应用 {{ getPresetLabel(activeRecommendation.preset) }}
              </button>
            </div>

            <PluginRuntimeSchedulerPanel
              :traffic-plugin-runtime-settings="trafficPluginRuntimeSettings"
              :is-saving-traffic-plugin-runtime-settings="isSavingTrafficPluginRuntimeSettings"
              :reset-traffic-plugin-runtime-policies="resetTrafficPluginRuntimePoliciesDraft"
              :apply-traffic-plugin-runtime-preset="applyTrafficPluginRuntimePreset"
              :policy-ids="policyIds"
              :collapsible="collapsible"
              :default-expanded-policy-ids="defaultExpandedPolicyIds"
              :description="resolvedDescription"
              :hint="resolvedHint"
            />
          </div>
        </div>
      </div>

      <form method="dialog" class="modal-backdrop" @click="closeDialog">
        <button>close</button>
      </form>
    </AppDialog>
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import AppDialog from '@/components/AppDialog.vue'
import PluginRuntimeSchedulerPanel from '@/components/PluginManagement/PluginRuntimeSchedulerPanel.vue'
import {
  getPendingPressureLevel,
  getRejectedTrendLevel,
  getRuntimeSummaryChipClass,
} from '@/components/PluginManagement/pluginRuntimeSchedulerSummarySupport'
import { useTrafficPluginRuntimeSettings } from '@/components/traffic/useTrafficPluginRuntimeSettings'
import { useTrafficPluginRuntimeQueue } from '@/components/traffic/useTrafficPluginRuntimeQueue'
import type {
  TrafficPluginRuntimePolicyId,
  TrafficPluginRuntimePreset,
} from '@/components/traffic/pluginRuntimeSettingsSupport'
import { normalizeTrafficPluginRuntimeSettings } from '@/components/traffic/pluginRuntimeSettingsSupport'
import type { TrafficPluginRuntimeSettings } from '@/components/traffic/proxyConfigurationTypes'
import {
  buildTrafficPluginRuntimeLauncherSummary,
  getTrafficPluginRuntimeLauncherPresetRecommendation,
  getTrafficPluginRuntimePendingTooltip,
  getTrafficPluginRuntimeRejectedTooltip,
  getTrafficPluginRuntimeRunningTooltip,
  isTrafficPluginRuntimePresetApplied,
  type TrafficPluginRuntimeLauncherSummary,
} from './trafficPluginRuntimeDialogLauncherSupport'

const props = withDefaults(
  defineProps<{
    policyIds?: TrafficPluginRuntimePolicyId[]
    defaultExpandedPolicyIds?: TrafficPluginRuntimePolicyId[]
    collapsible?: boolean
    title?: string
    description?: string
    hint?: string
    buttonLabel?: string
    buttonClass?: string
    iconClass?: string
  }>(),
  {
    policyIds: () => [
      'activeProbe',
      'bountyFetch',
      'monitorFetch',
      'agentFetch',
      'pluginTestFetch',
    ],
    defaultExpandedPolicyIds: () => [],
    collapsible: true,
    title: '',
    description: '',
    hint: '',
    buttonLabel: '',
    buttonClass: 'btn btn-outline',
    iconClass: 'fas fa-gauge-high',
  }
)

const { t } = useI18n()
const dialogOpen = ref(false)
const loading = ref(false)
const loadError = ref('')
const undoRecommendationState = ref<{
  previousSettings: TrafficPluginRuntimeSettings
  appliedPreset: TrafficPluginRuntimePreset
} | null>(null)

const {
  trafficPluginRuntimeSettings,
  isSavingTrafficPluginRuntimeSettings,
  applyLoadedTrafficPluginRuntimeSettings,
  loadTrafficPluginRuntimeSettings,
  saveTrafficPluginRuntimeSettings,
  resetTrafficPluginRuntimePoliciesDraft: resetTrafficPluginRuntimePoliciesDraftSettings,
  applyTrafficPluginRuntimePreset,
} = useTrafficPluginRuntimeSettings()
const { snapshot, history, refresh: refreshQueueState } = useTrafficPluginRuntimeQueue()
const AUTO_SAVE_DELAY_MS = 700
const autoSaveEnabled = ref(false)
const autoSaveTimer = ref<ReturnType<typeof setTimeout> | null>(null)
const autoSaveInFlight = ref(false)
const autoSaveRequestedDuringSave = ref(false)
const lastSavedRuntimeSignature = ref('')

const resolvedTitle = computed(
  () => props.title || t('trafficAnalysis.proxyConfiguration.pluginRuntimeTitle', '插件运行时')
)
const resolvedDescription = computed(
  () =>
    props.description ||
    t(
      'trafficAnalysis.proxyConfiguration.pluginRuntimeDesc',
      '统一管理插件请求调度、队列状态和实时趋势。'
    )
)
const resolvedHint = computed(
  () => props.hint || t('trafficAnalysis.proxyConfiguration.pluginRuntimeFetchHint')
)
const resolvedButtonLabel = computed(
  () =>
    props.buttonLabel || t('trafficAnalysis.proxyConfiguration.pluginRuntimeTitle', '插件运行时')
)
const launcherSummary = computed(() =>
  buildTrafficPluginRuntimeLauncherSummary(snapshot.value, history.value, props.policyIds)
)
const activeRecommendation = computed(() => {
  const recommendation = getTrafficPluginRuntimeLauncherPresetRecommendation(launcherSummary.value)
  if (!recommendation) {
    return null
  }
  if (
    isTrafficPluginRuntimePresetApplied(
      trafficPluginRuntimeSettings.value,
      recommendation.preset,
      props.policyIds
    )
  ) {
    return null
  }
  return recommendation
})
const getLauncherPendingLevel = (summary: TrafficPluginRuntimeLauncherSummary) =>
  getPendingPressureLevel({
    pendingCount: summary.pendingCount,
    configuredMaxQueueDepth: summary.pendingLimit,
  })

const summaryChipClass = (level: 'neutral' | 'warning' | 'error') =>
  getRuntimeSummaryChipClass(level)

const buildRuntimeSettingsSignature = (settings: TrafficPluginRuntimeSettings) =>
  JSON.stringify(normalizeTrafficPluginRuntimeSettings(settings))

const updateLastSavedRuntimeSignature = (settings: TrafficPluginRuntimeSettings) => {
  lastSavedRuntimeSignature.value = buildRuntimeSettingsSignature(settings)
}

const clearAutoSaveTimer = () => {
  if (!autoSaveTimer.value) {
    return
  }
  clearTimeout(autoSaveTimer.value)
  autoSaveTimer.value = null
}

const persistTrafficPluginRuntimeSettings = async (): Promise<boolean> => {
  if (!dialogOpen.value || loading.value || !autoSaveEnabled.value) {
    return true
  }

  const currentSignature = buildRuntimeSettingsSignature(trafficPluginRuntimeSettings.value)
  if (currentSignature === lastSavedRuntimeSignature.value) {
    return true
  }

  if (autoSaveInFlight.value) {
    autoSaveRequestedDuringSave.value = true
    return true
  }

  autoSaveInFlight.value = true
  autoSaveRequestedDuringSave.value = false
  let saved = false
  try {
    const savedSettings = await saveTrafficPluginRuntimeSettings()
    updateLastSavedRuntimeSignature(savedSettings)
    await refreshQueueState()
    saved = true
    return true
  } catch {
    return false
  } finally {
    autoSaveInFlight.value = false
    const latestSignature = buildRuntimeSettingsSignature(trafficPluginRuntimeSettings.value)
    if (
      saved &&
      (autoSaveRequestedDuringSave.value || latestSignature !== lastSavedRuntimeSignature.value)
    ) {
      autoSaveRequestedDuringSave.value = false
      scheduleAutoSave()
    } else if (!saved) {
      autoSaveRequestedDuringSave.value = false
    }
  }
}

function scheduleAutoSave() {
  if (!dialogOpen.value || loading.value || !autoSaveEnabled.value) {
    return
  }
  clearAutoSaveTimer()
  autoSaveTimer.value = setTimeout(() => {
    autoSaveTimer.value = null
    void persistTrafficPluginRuntimeSettings()
  }, AUTO_SAVE_DELAY_MS)
}

const getPresetLabel = (preset: TrafficPluginRuntimePreset) => {
  if (preset === 'local_fast') {
    return t('trafficAnalysis.proxyConfiguration.activeProbePresetLocalFast')
  }
  if (preset === 'conservative') {
    return t('trafficAnalysis.proxyConfiguration.activeProbePresetConservative')
  }
  return t('trafficAnalysis.proxyConfiguration.activeProbePresetBalanced')
}

const openDialog = async () => {
  dialogOpen.value = true
  loading.value = true
  autoSaveEnabled.value = false
  clearAutoSaveTimer()
  loadError.value = ''
  let loaded = false
  try {
    await loadTrafficPluginRuntimeSettings()
    updateLastSavedRuntimeSignature(trafficPluginRuntimeSettings.value)
    loaded = true
  } catch (error) {
    loadError.value = error instanceof Error ? error.message : '加载插件运行时设置失败'
  } finally {
    loading.value = false
  }
  await nextTick()
  if (dialogOpen.value && loaded) {
    autoSaveEnabled.value = true
  }
  undoRecommendationState.value = null
}

const closeDialog = async () => {
  clearAutoSaveTimer()
  const saved = await persistTrafficPluginRuntimeSettings()
  if (!saved) {
    return
  }
  autoSaveEnabled.value = false
  dialogOpen.value = false
  undoRecommendationState.value = null
}

const resetTrafficPluginRuntimePoliciesDraft = (policyIds: TrafficPluginRuntimePolicyId[]) => {
  resetTrafficPluginRuntimePoliciesDraftSettings(policyIds)
}

const applyRecommendedPreset = async () => {
  if (!activeRecommendation.value) {
    return
  }

  const shouldResumeAutoSave = autoSaveEnabled.value
  autoSaveEnabled.value = false
  clearAutoSaveTimer()
  try {
    undoRecommendationState.value = {
      previousSettings: normalizeTrafficPluginRuntimeSettings(trafficPluginRuntimeSettings.value),
      appliedPreset: activeRecommendation.value.preset,
    }
    applyTrafficPluginRuntimePreset(activeRecommendation.value.preset, props.policyIds)
    const savedSettings = await saveTrafficPluginRuntimeSettings()
    updateLastSavedRuntimeSignature(savedSettings)
    await refreshQueueState()
  } catch {
    undoRecommendationState.value = null
    // saveTrafficPluginRuntimeSettings already reports the failure to the user
  } finally {
    if (dialogOpen.value && shouldResumeAutoSave) {
      autoSaveEnabled.value = true
    }
  }
}

const undoRecommendedPreset = async () => {
  if (!undoRecommendationState.value) {
    return
  }

  const rollbackSettings = undoRecommendationState.value.previousSettings
  const shouldResumeAutoSave = autoSaveEnabled.value
  autoSaveEnabled.value = false
  clearAutoSaveTimer()
  try {
    applyLoadedTrafficPluginRuntimeSettings(rollbackSettings)
    const savedSettings = await saveTrafficPluginRuntimeSettings()
    updateLastSavedRuntimeSignature(savedSettings)
    await refreshQueueState()
    undoRecommendationState.value = null
  } catch {
    // saveTrafficPluginRuntimeSettings already reports the failure to the user
  } finally {
    if (dialogOpen.value && shouldResumeAutoSave) {
      autoSaveEnabled.value = true
    }
  }
}

watch(
  () => buildRuntimeSettingsSignature(trafficPluginRuntimeSettings.value),
  signature => {
    if (!dialogOpen.value || loading.value || !autoSaveEnabled.value) {
      return
    }
    if (signature === lastSavedRuntimeSignature.value) {
      clearAutoSaveTimer()
      return
    }
    if (autoSaveInFlight.value) {
      autoSaveRequestedDuringSave.value = true
      return
    }
    scheduleAutoSave()
  }
)

onBeforeUnmount(() => {
  clearAutoSaveTimer()
})
</script>
