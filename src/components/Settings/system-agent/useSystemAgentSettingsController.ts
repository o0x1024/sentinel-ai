import { computed, onMounted, onUnmounted, ref, watch, type Ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'

import { dialog } from '@/composables/useDialog'
import {
  createDefaultTrafficContextExtractionSettings,
  type TrafficBehaviorSignalSettings,
  type TrafficContextExtractionSettings,
} from '@/components/traffic/proxyConfigurationTypes'
import {
  getSystemAgentDescription,
  getSystemAgentModeBadge,
  getSystemAgentPassiveEventName,
  getSystemAgentPromptPatchGuidance,
  getSystemAgentPromptPatchPlaceholder,
  getSystemAgentSafetyMode,
  getSystemAgentStatsMode,
  supportsSystemAgentBehaviorSource,
  supportsSystemAgentFindings,
} from './systemAgentRegistry'
import {
  cloneProfile,
  parseJsonText,
  type CommandResponse,
  type SystemAgentBehaviorEffectStats,
  type SystemAgentFindingSummary,
  type SystemAgentListItem,
  type SystemAgentProfilePayload,
  type SystemAgentProfileVersionPayload,
  type SystemAgentProfileSummary,
  type SystemAgentRunPayload,
  type SystemAgentSafetyPolicyForm,
} from '../systemAgentSettingsSupport'

export function useSystemAgentSettingsController() {
  const loading = ref(false)
  const saving = ref(false)
  const dispatching = ref(false)
  const autoSaveState = ref<'idle' | 'saving' | 'saved' | 'error'>('idle')
  const profiles = ref<SystemAgentProfileSummary[]>([])
  const runs = ref<SystemAgentRunPayload[]>([])
  const versions = ref<SystemAgentProfileVersionPayload[]>([])
  const recentFindings = ref<SystemAgentFindingSummary[]>([])
  const totalFindingsCount = ref(0)
  const falsePositiveFindingsCount = ref(0)
  const behaviorEffectStats = ref<SystemAgentBehaviorEffectStats | null>(null)
  const statsWindow = ref<'24h' | '7d' | '30d'>('24h')
  const selectedProfileId = ref('')
  const selectedProfile = ref<SystemAgentProfilePayload | null>(null)
  const llmProviderOptions = ref<Array<{ value: string; label: string }>>([])
  const llmProviderModelsMap = ref<Record<string, string[]>>({})
  const globalDefaultLlmLabel = ref('未配置')
  const behaviorSignalSettings = ref<TrafficBehaviorSignalSettings>({
    mode: 'proxy_inferred',
    browserExtensionConnected: false,
    browserExtensionLastSeenAt: null,
  })
  const contextExtractionSettings = ref<TrafficContextExtractionSettings>(
    createDefaultTrafficContextExtractionSettings()
  )
  const contextExtractionSaving = ref(false)

  const safetyPolicyValue = ref<SystemAgentSafetyPolicyForm>({
    allowActiveReplay: false,
    autoMode: false,
    shadowMode: false,
    scopeHostsText: '',
  })
  const dispatchPayloadText = ref(
    '{\n  "clusterKey": "demo-cluster",\n  "host": "example.com",\n  "pathTemplate": "/api/demo",\n  "method": "GET"\n}'
  )

  const toolBindingValue = ref({
    requiredTools: [] as string[],
    optionalTools: [] as string[],
    forbiddenTools: [] as string[],
  })
  const suspendAutoSave = ref(false)
  const lastSavedSnapshot = ref('')
  let autoSaveTimer: ReturnType<typeof setTimeout> | null = null
  let unlisten: UnlistenFn | null = null
  let unlistenFinding: UnlistenFn | null = null
  let unlistenBehaviorStatus: UnlistenFn | null = null

  const promptPatchPlaceholder = computed(() => {
    if (!selectedProfile.value) return ''
    return getSystemAgentPromptPatchPlaceholder(selectedProfile.value)
  })

  const promptPatchGuidance = computed(() => {
    if (!selectedProfile.value) return ''
    return getSystemAgentPromptPatchGuidance(selectedProfile.value)
  })

  const selectedProfileDescription = computed(() => {
    if (!selectedProfile.value) return ''
    return getProfileDescription(selectedProfile.value)
  })

  const autoSaveStatusText = computed(() => {
    if (autoSaveState.value === 'saving') return '自动保存中'
    if (autoSaveState.value === 'saved') return '已自动保存'
    if (autoSaveState.value === 'error') return '自动保存失败'
    return '修改后自动保存'
  })

  const autoSaveStatusClass = computed(() => {
    if (autoSaveState.value === 'saving') return 'text-info'
    if (autoSaveState.value === 'saved') return 'text-success'
    if (autoSaveState.value === 'error') return 'text-error'
    return 'text-base-content/60'
  })

  const showRecentFindingsPanel = computed(() => {
    return selectedProfile.value ? supportsSystemAgentFindings(selectedProfile.value) : false
  })

  const showBehaviorSourcePanel = computed(() => {
    return selectedProfile.value ? supportsSystemAgentBehaviorSource(selectedProfile.value) : false
  })

  const selectedProfileModeBadge = computed(() => {
    if (!selectedProfile.value) {
      return {
        label: '事件驱动',
        className: 'badge-warning',
      }
    }
    return getSystemAgentModeBadge(selectedProfile.value)
  })

  const selectedProfilePassiveEventName = computed(() => {
    if (!selectedProfile.value || selectedProfile.value.mode !== 'passive') return ''
    return getSystemAgentPassiveEventName(selectedProfile.value)
  })

  const selectedProfileStatsMode = computed(() => {
    if (!selectedProfile.value) return 'versions' as const
    return getSystemAgentStatsMode(selectedProfile.value)
  })

  const selectedProfileSafetyMode = computed(() => {
    if (!selectedProfile.value) return 'generic' as const
    return getSystemAgentSafetyMode(selectedProfile.value)
  })

  const statsWindowStart = computed(() => {
    const now = Date.now()
    if (statsWindow.value === '7d') return now - 7 * 24 * 60 * 60 * 1000
    if (statsWindow.value === '30d') return now - 30 * 24 * 60 * 60 * 1000
    return now - 24 * 60 * 60 * 1000
  })

  const profileListItems = computed<SystemAgentListItem[]>(() => {
    return profiles.value.map(profile => ({
      id: profile.id,
      name: profile.name,
      description: getProfileDescription(profile),
      enabled: profile.enabled,
      modeBadge: getModeBadge(profile),
    }))
  })

  const llmModelSuggestions = computed(() => {
    const provider = selectedProfile.value?.llmProviderOverride?.trim().toLowerCase()
    if (!provider) return []
    return llmProviderModelsMap.value[provider] || []
  })

  const editableProfileSnapshot = computed(() => buildProfileSnapshot(buildProfilePayload()))

  const syncTextFieldsFromProfile = (profile: SystemAgentProfilePayload | null) => {
    toolBindingValue.value = {
      requiredTools: [...(profile?.requiredTools || [])],
      optionalTools: [...(profile?.optionalTools || [])],
      forbiddenTools: [...(profile?.forbiddenTools || [])],
    }
    safetyPolicyValue.value = normalizeSafetyPolicy(profile?.safetyPolicy)
  }

  const updateProfileSummary = (profile: SystemAgentProfilePayload) => {
    const nextSummary: SystemAgentProfileSummary = {
      id: profile.id,
      name: profile.name,
      description: profile.description,
      mode: profile.mode,
      capability: profile.capability,
      enabled: profile.enabled,
      triggerMode: profile.triggerMode,
      riskLevel: profile.riskLevel,
      visibility: profile.visibility,
      updatedAt: profile.updatedAt || new Date().toISOString(),
    }

    const index = profiles.value.findIndex(item => item.id === profile.id)
    if (index >= 0) {
      profiles.value.splice(index, 1, nextSummary)
    }
  }

  const loadProfiles = async () => {
    loading.value = true
    try {
      const response = await invoke<CommandResponse<SystemAgentProfileSummary[]>>(
        'list_system_agent_profiles'
      )
      profiles.value = (response.data ?? []).filter(
        profile =>
          profile.visibility !== 'hidden'
          && profile.mode === 'passive'
          && profile.id !== 'traffic_idor_triage'
      )
      if (!profiles.value.some(profile => profile.id === selectedProfileId.value)) {
        selectedProfileId.value = ''
        selectedProfile.value = null
      }
      if (!selectedProfileId.value && profiles.value.length > 0) {
        await selectProfile(profiles.value[0].id)
      }
    } catch (error) {
      console.error('Failed to load system agent profiles', error)
      dialog.toast.error('加载系统智能体列表失败')
    } finally {
      loading.value = false
    }
  }

  const loadLlmOptions = async () => {
    try {
      const aiConfig = await invoke<any>('get_ai_config')
      const providers = aiConfig?.providers && typeof aiConfig.providers === 'object'
        ? aiConfig.providers
        : {}
      const providerOptions: Array<{ value: string; label: string }> = []
      const providerModelsMap: Record<string, string[]> = {}

      Object.entries(providers).forEach(([providerKey, providerValue]) => {
        const config = providerValue as any
        if (config?.enabled === false) return

        const providerRaw = String(config?.provider || providerKey).trim()
        if (!providerRaw) return

        const providerValueNormalized = providerRaw.toLowerCase()
        providerOptions.push({
          value: providerValueNormalized,
          label: normalizeProviderName(providerRaw),
        })

        const modelIds = new Set<string>()
        const models = Array.isArray(config?.models) ? config.models : []
        models.forEach((item: any) => {
          const modelId = extractModelId(item)
          if (modelId) modelIds.add(modelId)
        })
        if (typeof config?.default_model === 'string' && config.default_model.trim()) {
          modelIds.add(config.default_model.trim())
        }
        providerModelsMap[providerValueNormalized] = Array.from(modelIds).sort((a, b) => a.localeCompare(b))
      })

      providerOptions.sort((a, b) => a.label.localeCompare(b.label))
      llmProviderOptions.value = providerOptions
      llmProviderModelsMap.value = providerModelsMap

      const defaultModel = typeof aiConfig?.default_llm_model === 'string'
        ? aiConfig.default_llm_model.trim()
        : ''
      globalDefaultLlmLabel.value = defaultModel || '未配置'
    } catch (error) {
      console.error('Failed to load LLM options for system agents', error)
      llmProviderOptions.value = []
      llmProviderModelsMap.value = {}
      globalDefaultLlmLabel.value = '未配置'
    }
  }

  const loadBehaviorSignalSettings = async () => {
    try {
      const response = await invoke<CommandResponse<TrafficBehaviorSignalSettings>>(
        'get_traffic_behavior_signal_settings'
      )
      if (response.data) {
        behaviorSignalSettings.value = response.data
      }
    } catch (error) {
      console.error('Failed to load traffic behavior signal settings', error)
    }
  }

  const loadContextExtractionSettings = async () => {
    try {
      const response = await invoke<CommandResponse<TrafficContextExtractionSettings>>(
        'get_traffic_context_extraction_settings'
      )
      if (response.data) {
        contextExtractionSettings.value = response.data
      }
    } catch (error) {
      console.error('Failed to load traffic context extraction settings', error)
    }
  }

  const loadRuns = async () => {
    if (!selectedProfileId.value) {
      runs.value = []
      return
    }

    try {
      const response = await invoke<CommandResponse<SystemAgentRunPayload[]>>(
        'list_system_agent_runs',
        {
          profileId: selectedProfileId.value,
          limit: 100,
        }
      )
      runs.value = response.data ?? []
    } catch (error) {
      console.error('Failed to load system agent runs', error)
      dialog.toast.error('加载运行记录失败')
    }
  }

  const loadVersions = async () => {
    if (!selectedProfileId.value) {
      versions.value = []
      return
    }

    try {
      const response = await invoke<CommandResponse<SystemAgentProfileVersionPayload[]>>(
        'list_system_agent_profile_versions',
        {
          profileId: selectedProfileId.value,
          limit: 10,
        }
      )
      versions.value = response.data ?? []
    } catch (error) {
      console.error('Failed to load system agent profile versions', error)
      versions.value = []
    }
  }

  const loadRecentFindings = async () => {
    if (!selectedProfile.value?.id || !showRecentFindingsPanel.value) {
      recentFindings.value = []
      totalFindingsCount.value = 0
      falsePositiveFindingsCount.value = 0
      return
    }

    try {
      const pluginId = `agent:${selectedProfile.value.id}`
      const [response, totalResponse, falsePositiveResponse] = await Promise.all([
        invoke<CommandResponse<SystemAgentFindingSummary[]>>('list_findings', {
          limit: 30,
          offset: 0,
          severityFilter: null,
          pluginId,
          statusFilter: null,
        }),
        invoke<CommandResponse<number>>('count_findings', {
          severityFilter: null,
          pluginId,
          statusFilter: null,
        }),
        invoke<CommandResponse<number>>('count_findings', {
          severityFilter: null,
          pluginId,
          statusFilter: 'false_positive',
        }),
      ])
      recentFindings.value = response.data ?? []
      totalFindingsCount.value = totalResponse.data ?? 0
      falsePositiveFindingsCount.value = falsePositiveResponse.data ?? 0
    } catch (error) {
      console.error('Failed to load recent findings for system agent', error)
      totalFindingsCount.value = 0
      falsePositiveFindingsCount.value = 0
      dialog.toast.error('加载智能体最近发现失败')
    }
  }

  const loadBehaviorEffectStats = async () => {
    if (!selectedProfile.value?.id || !showBehaviorSourcePanel.value) {
      behaviorEffectStats.value = null
      return
    }

    try {
      const response = await invoke<CommandResponse<SystemAgentBehaviorEffectStats>>(
        'get_traffic_behavior_effect_stats',
        {
          profileId: selectedProfile.value.id,
          window: statsWindow.value,
        }
      )
      behaviorEffectStats.value = response.data ?? null
    } catch (error) {
      console.error('Failed to load traffic behavior effect stats', error)
      behaviorEffectStats.value = null
    }
  }

  const selectProfile = async (profileId: string) => {
    selectedProfileId.value = profileId
    try {
      suspendAutoSave.value = true
      const response = await invoke<CommandResponse<SystemAgentProfilePayload | null>>(
        'get_system_agent_profile',
        {
          id: profileId,
        }
      )
      selectedProfile.value = response.data ? cloneProfile(response.data) : null
      syncTextFieldsFromProfile(selectedProfile.value)
      lastSavedSnapshot.value = buildProfileSnapshot(selectedProfile.value)
      autoSaveState.value = 'idle'
      await loadRuns()
      await loadVersions()
      await loadRecentFindings()
      await loadBehaviorEffectStats()
    } catch (error) {
      console.error('Failed to load system agent profile detail', error)
      dialog.toast.error('加载 Profile 详情失败')
    } finally {
      suspendAutoSave.value = false
    }
  }

  const buildProfilePayload = (): SystemAgentProfilePayload | null => {
    if (!selectedProfile.value) return null

    const profile = cloneProfile(selectedProfile.value)
    const providerOverride = profile.llmProviderOverride?.trim() || null
    const modelOverride = profile.llmModelOverride?.trim() || null
    profile.llmProviderOverride = providerOverride
    profile.llmModelOverride = providerOverride ? modelOverride : null
    profile.requiredTools = [...toolBindingValue.value.requiredTools]
    profile.optionalTools = [...toolBindingValue.value.optionalTools]
    profile.forbiddenTools = [...toolBindingValue.value.forbiddenTools]
    profile.safetyPolicy = buildSafetyPolicyPayload(safetyPolicyValue)
    return profile
  }

  const buildProfileSnapshot = (profile: SystemAgentProfilePayload | null) => {
    if (!profile) return ''
    return JSON.stringify({
      id: profile.id,
      name: profile.name,
      enabled: profile.enabled,
      llmProviderOverride: profile.llmProviderOverride || '',
      llmModelOverride: profile.llmModelOverride || '',
      promptPatch: profile.promptPatch || '',
      requiredTools: profile.requiredTools,
      optionalTools: profile.optionalTools,
      forbiddenTools: profile.forbiddenTools,
      safetyPolicy: profile.safetyPolicy || {},
      cooldownSecs: profile.cooldownSecs,
      maxConcurrency: profile.maxConcurrency,
    })
  }

  const saveSelectedProfile = async (options?: { silent?: boolean }) => {
    try {
      const profile = buildProfilePayload()
      if (!profile) return
      if (!profile.id.trim() || !profile.name.trim()) {
        dialog.toast.warning('Profile ID 和名称不能为空')
        return
      }

      saving.value = true
      autoSaveState.value = 'saving'
      const response = await invoke<CommandResponse<SystemAgentProfilePayload>>(
        'save_system_agent_profile',
        { profile }
      )
      if (!response.data) throw new Error(response.error || '保存失败')
      suspendAutoSave.value = true
      selectedProfile.value = cloneProfile(response.data)
      selectedProfileId.value = response.data.id
      syncTextFieldsFromProfile(selectedProfile.value)
      updateProfileSummary(response.data)
      lastSavedSnapshot.value = buildProfileSnapshot(response.data)
      autoSaveState.value = 'saved'
      await loadVersions()
      if (!options?.silent) {
        dialog.toast.success('智能体配置已保存')
      }
    } catch (error) {
      console.error('Failed to save system agent profile', error)
      autoSaveState.value = 'error'
      dialog.toast.error(`保存失败: ${String(error)}`)
    } finally {
      suspendAutoSave.value = false
      saving.value = false
      if (
        editableProfileSnapshot.value &&
        editableProfileSnapshot.value !== lastSavedSnapshot.value
      ) {
        queueAutoSave()
      }
    }
  }

  const queueAutoSave = () => {
    if (!selectedProfile.value || suspendAutoSave.value) return
    if (autoSaveTimer) {
      clearTimeout(autoSaveTimer)
    }
    autoSaveTimer = setTimeout(() => {
      autoSaveTimer = null
      if (saving.value || !selectedProfile.value) return
      if (editableProfileSnapshot.value === lastSavedSnapshot.value) return
      void saveSelectedProfile({ silent: true })
    }, 600)
  }

  const dispatchSelectedProfileEvent = async () => {
    if (!selectedProfile.value?.id) return
    const eventName = selectedProfilePassiveEventName.value
    if (!eventName) return
    dispatching.value = true
    try {
      const payload = parseJsonText(dispatchPayloadText.value)
      await invoke<CommandResponse<unknown>>('dispatch_system_agent_event', {
        eventName,
        payload,
        source: 'settings_panel',
      })
      dialog.toast.success(`事件 ${eventName} 已派发`)
      await loadRuns()
    } catch (error) {
      console.error('Failed to dispatch system agent event', error)
      dialog.toast.error(`派发失败: ${String(error)}`)
    } finally {
      dispatching.value = false
    }
  }

  const seedDefaults = async () => {
    try {
      await invoke<CommandResponse<number>>('seed_system_agent_profiles')
      await refreshAll()
      dialog.toast.success('默认系统智能体已初始化')
    } catch (error) {
      console.error('Failed to seed system agents', error)
      dialog.toast.error('初始化默认智能体失败')
    }
  }

  const refreshAll = async () => {
    await loadProfiles()
    await loadLlmOptions()
    await loadRuns()
    await loadVersions()
    await loadRecentFindings()
    await loadBehaviorEffectStats()
    await loadContextExtractionSettings()
  }

  const saveContextExtractionSettings = async () => {
    contextExtractionSaving.value = true
    try {
      const response = await invoke<CommandResponse<TrafficContextExtractionSettings>>(
        'set_traffic_context_extraction_settings',
        {
          payload: {
            settings: contextExtractionSettings.value,
          },
        }
      )
      if (!response.data) throw new Error(response.error || '保存失败')
      contextExtractionSettings.value = response.data
      dialog.toast.success('上下文抽取词典已保存')
    } catch (error) {
      console.error('Failed to save traffic context extraction settings', error)
      dialog.toast.error(`保存上下文抽取词典失败: ${String(error)}`)
    } finally {
      contextExtractionSaving.value = false
    }
  }

  onMounted(async () => {
    await loadProfiles()
    await loadLlmOptions()
    await loadBehaviorSignalSettings()
    await loadContextExtractionSettings()
    unlisten = await listen('system-agent:run-updated', async () => {
      await loadRuns()
    })
    unlistenFinding = await listen('scan:finding', async () => {
      await loadRecentFindings()
      await loadBehaviorEffectStats()
    })
    unlistenBehaviorStatus = await listen<{
      connected: boolean
      lastSeenAt?: string | null
    }>('traffic-behavior:extension-status', async event => {
      behaviorSignalSettings.value = {
        ...behaviorSignalSettings.value,
        browserExtensionConnected: event.payload.connected,
        browserExtensionLastSeenAt: event.payload.lastSeenAt || null,
      }
      await loadBehaviorEffectStats()
    })
  })

  onUnmounted(() => {
    unlisten?.()
    unlistenFinding?.()
    unlistenBehaviorStatus?.()
    if (autoSaveTimer) {
      clearTimeout(autoSaveTimer)
      autoSaveTimer = null
    }
  })

  watch(editableProfileSnapshot, snapshot => {
    if (!snapshot || suspendAutoSave.value || !selectedProfile.value) return
    if (snapshot === lastSavedSnapshot.value) return
    autoSaveState.value = 'idle'
    queueAutoSave()
  })

  watch(statsWindow, async () => {
    await loadBehaviorEffectStats()
  })

  return {
    loading,
    dispatching,
    profiles,
    runs,
    versions,
    recentFindings,
    totalFindingsCount,
    falsePositiveFindingsCount,
    statsWindow,
    selectedProfileId,
    selectedProfile,
    llmProviderOptions,
    llmModelSuggestions,
    globalDefaultLlmLabel,
    safetyPolicyValue,
    dispatchPayloadText,
    toolBindingValue,
    promptPatchPlaceholder,
    promptPatchGuidance,
    selectedProfileDescription,
    autoSaveStatusText,
    autoSaveStatusClass,
    behaviorSignalSettings,
    contextExtractionSettings,
    contextExtractionSaving,
    behaviorEffectStats,
    showBehaviorSourcePanel,
    showRecentFindingsPanel,
    selectedProfileModeBadge,
    selectedProfilePassiveEventName,
    selectedProfileStatsMode,
    selectedProfileSafetyMode,
    statsWindowStart,
    profileListItems,
    selectProfile,
    dispatchSelectedProfileEvent,
    seedDefaults,
    refreshAll,
    saveContextExtractionSettings,
    loadRuns,
    loadVersions,
    loadRecentFindings,
  }
}

function getProfileDescription(
  profile:
    | Pick<SystemAgentProfileSummary, 'id' | 'description' | 'mode' | 'capability'>
    | Pick<SystemAgentProfilePayload, 'id' | 'description' | 'mode' | 'capability'>
) {
  return getSystemAgentDescription(profile)
}

function getModeBadge(
  profile:
    | Pick<SystemAgentProfileSummary, 'id' | 'description' | 'mode' | 'capability'>
    | Pick<SystemAgentProfilePayload, 'id' | 'description' | 'mode' | 'capability'>
) {
  return getSystemAgentModeBadge(profile)
}

function normalizeSafetyPolicy(
  value: Record<string, unknown> | null | undefined
): SystemAgentSafetyPolicyForm {
  return {
    allowActiveReplay: value?.allowActiveReplay === true,
    autoMode: value?.autoMode === true,
    shadowMode: value?.shadowMode === true,
    scopeHostsText: Array.isArray(value?.scopeHosts)
      ? value.scopeHosts
          .map((item: unknown) => String(item).trim())
          .filter(Boolean)
          .join('\n')
      : '',
  }
}

function buildSafetyPolicyPayload(safetyPolicyValue: Ref<SystemAgentSafetyPolicyForm>) {
  return {
    allowActiveReplay: safetyPolicyValue.value.allowActiveReplay,
    autoMode: safetyPolicyValue.value.autoMode,
    shadowMode: safetyPolicyValue.value.shadowMode,
    scopeHosts: safetyPolicyValue.value.scopeHostsText
      .split('\n')
      .map(item => item.trim())
      .filter(Boolean),
  }
}

function normalizeProviderName(provider: string) {
  const lower = provider.toLowerCase()
  const names: Record<string, string> = {
    openai: 'OpenAI',
    anthropic: 'Anthropic',
    gemini: 'Gemini',
    deepseek: 'DeepSeek',
    moonshot: 'Moonshot',
    ollama: 'Ollama',
    openrouter: 'OpenRouter',
    modelscope: 'ModelScope',
    groq: 'Groq',
    perplexity: 'Perplexity',
    togetherai: 'TogetherAI',
    xai: 'xAI',
    cohere: 'Cohere',
    'lm studio': 'LM Studio',
    lmstudio: 'LM Studio',
    lm_studio: 'LM Studio',
  }
  return names[lower] || provider
}

function extractModelId(item: any): string {
  if (!item) return ''
  if (typeof item === 'string') return item.trim()
  if (typeof item.id === 'string') return item.id.trim()
  if (typeof item.name === 'string') return item.name.trim()
  return ''
}
