import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { dialog } from '@/composables/useDialog'
import {
  createDefaultTrafficPluginRuntimeSettings,
  type TrafficPluginRuntimeSettings,
} from './proxyConfigurationTypes'
import {
  applyTrafficPluginRuntimePresetToPolicies,
  mergeTrafficPluginRuntimeSettings,
  normalizeTrafficPluginRuntimeSettings,
  resetTrafficPluginRuntimePoliciesToDefaults,
  updateTrafficPluginRuntimePolicySettings,
  type TrafficPluginRuntimePolicyId,
  type TrafficPluginRuntimePolicySettings,
  type TrafficPluginRuntimePreset,
} from './pluginRuntimeSettingsSupport'

interface CommandResponse<T = unknown> {
  success?: boolean
  data?: T
  error?: string
}

export function useTrafficPluginRuntimeSettings() {
  const trafficPluginRuntimeSettings = ref<TrafficPluginRuntimeSettings>(
    createDefaultTrafficPluginRuntimeSettings()
  )
  const isSavingTrafficPluginRuntimeSettings = ref(false)

  const applyLoadedTrafficPluginRuntimeSettings = (settings: TrafficPluginRuntimeSettings) => {
    trafficPluginRuntimeSettings.value = mergeTrafficPluginRuntimeSettings(settings)
  }

  const loadTrafficPluginRuntimeSettings = async () => {
    const response = await invoke<CommandResponse<TrafficPluginRuntimeSettings>>(
      'get_traffic_plugin_runtime_settings'
    )
    if (!response.success || !response.data) {
      throw new Error(response.error || '加载失败')
    }
    applyLoadedTrafficPluginRuntimeSettings(response.data)
    return response.data
  }

  const saveTrafficPluginRuntimeSettings = async () => {
    isSavingTrafficPluginRuntimeSettings.value = true
    try {
      const payload = normalizeTrafficPluginRuntimeSettings(trafficPluginRuntimeSettings.value)
      const response = await invoke<CommandResponse<TrafficPluginRuntimeSettings>>(
        'set_traffic_plugin_runtime_settings',
        {
          payload: {
            settings: payload,
          },
        }
      )

      if (!response.success || !response.data) {
        throw new Error(response.error || '保存失败')
      }

      applyLoadedTrafficPluginRuntimeSettings(response.data)
      dialog.toast.success('已保存插件运行时设置')
      return response.data
    } catch (error: any) {
      dialog.toast.error(`保存配置失败: ${error}`)
      throw error
    } finally {
      isSavingTrafficPluginRuntimeSettings.value = false
    }
  }

  const resetTrafficPluginRuntimePoliciesDraft = (
    policyIds: TrafficPluginRuntimePolicyId[]
  ): TrafficPluginRuntimeSettings => {
    const nextSettings = resetTrafficPluginRuntimePoliciesToDefaults(
      trafficPluginRuntimeSettings.value,
      policyIds
    )
    trafficPluginRuntimeSettings.value = nextSettings
    return nextSettings
  }

  const resetTrafficPluginRuntimePolicies = (policyIds: TrafficPluginRuntimePolicyId[]) => {
    resetTrafficPluginRuntimePoliciesDraft(policyIds)
    return saveTrafficPluginRuntimeSettings()
  }

  const applyTrafficPluginRuntimePreset = (
    preset: TrafficPluginRuntimePreset,
    policyIds: TrafficPluginRuntimePolicyId[]
  ) => {
    trafficPluginRuntimeSettings.value = applyTrafficPluginRuntimePresetToPolicies(
      trafficPluginRuntimeSettings.value,
      preset,
      policyIds
    )
  }

  const updateTrafficPluginRuntimePolicy = (
    policyId: TrafficPluginRuntimePolicyId,
    policy: TrafficPluginRuntimePolicySettings
  ) => {
    trafficPluginRuntimeSettings.value = updateTrafficPluginRuntimePolicySettings(
      trafficPluginRuntimeSettings.value,
      policyId,
      policy
    )
  }

  const updateDirectFetchMaxConcurrent = (value: number) => {
    trafficPluginRuntimeSettings.value = normalizeTrafficPluginRuntimeSettings({
      ...trafficPluginRuntimeSettings.value,
      directFetchMaxConcurrent: value,
    })
  }

  return {
    trafficPluginRuntimeSettings,
    isSavingTrafficPluginRuntimeSettings,
    applyLoadedTrafficPluginRuntimeSettings,
    loadTrafficPluginRuntimeSettings,
    saveTrafficPluginRuntimeSettings,
    resetTrafficPluginRuntimePoliciesDraft,
    resetTrafficPluginRuntimePolicies,
    applyTrafficPluginRuntimePreset,
    updateTrafficPluginRuntimePolicy,
    updateDirectFetchMaxConcurrent,
  }
}
