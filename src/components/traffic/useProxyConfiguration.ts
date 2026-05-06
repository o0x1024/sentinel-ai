import { computed, inject, onMounted, onUnmounted, ref, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { open } from '@tauri-apps/plugin-dialog'
import { dialog } from '@/composables/useDialog'
import {
  createDefaultEditingListener,
  createDefaultEditingMatchReplace,
  createDefaultEditingRule,
  createDefaultProxyScopeRule,
  createDefaultEditingTlsPassThrough,
  createDefaultEditingUpstream,
  createDefaultMatchReplaceRules,
  createDefaultProxyConfig,
  createDefaultProxyListener,
  createDefaultTrafficBehaviorSignalSettings,
  createDefaultTrafficOastConfig,
  createDefaultRequestRules,
  createDefaultResponseRules,
  createDefaultTlsPassThroughRules,
  relationshipValues,
  requestMatchTypeValues,
  responseMatchTypeValues,
  type FilterRulePayload,
  type InterceptionRule,
  type MatchReplaceRule,
  type ProxyListener,
  type ProxyScopeRule,
  type TrafficBehaviorSignalMode,
  type TrafficBehaviorSignalSettings,
  type TrafficOastConfig,
  type TrafficOastTestResult,
  type TlsPassThroughRule,
  type UpstreamProxyConfig,
} from './proxyConfigurationTypes'
import { useTrafficPluginRuntimeSettings } from './useTrafficPluginRuntimeSettings'

type TranslateFn = (key: string, ...args: any[]) => string
type FilterRuleAddedPayload = {
  matchType: string
  condition: string
  relationship: string
}

interface UseProxyConfigurationOptions {
  t: TranslateFn
  emitFilterRuleAdded: (payload: FilterRuleAddedPayload) => void
}

interface CommandResponse<T = unknown> {
  success?: boolean
  data?: T
  error?: string
  message?: string
}

interface TrafficBehaviorExtensionInstallation {
  bridgeUrl: string
  extensionDirectory: string
  directorySource: string
  bundledWithApp: boolean
}

interface CopyTrafficBehaviorExtensionResult {
  copiedDirectory: string
}

function moveItem<T>(items: T[], index: number, nextIndex: number) {
  const temp = items[index]
  items[index] = items[nextIndex]
  items[nextIndex] = temp
}

function normalizeScopeRules(rules: unknown): ProxyScopeRule[] {
  if (!Array.isArray(rules)) {
    return []
  }

  return rules.map(rule => ({
    ...createDefaultProxyScopeRule(),
    ...(rule as Partial<ProxyScopeRule>),
  }))
}

function buildRuntimeRulePayload(rules: InterceptionRule[]) {
  return rules.map(rule => ({
    enabled: rule.enabled,
    operator: rule.operator || '',
    match_type: rule.matchType,
    relationship: rule.relationship,
    condition: rule.condition || '',
  }))
}

export function useProxyConfiguration({ t, emitFilterRuleAdded }: UseProxyConfigurationOptions) {
  const refreshTrigger = inject<any>('refreshTrigger', ref(0))

  const isSaving = ref(false)
  const saveQueued = ref(false)
  const saveAgainAfterCurrent = ref(false)
  const localSettingsToastTimeout = ref<ReturnType<typeof setTimeout> | null>(null)
  const isInitialLoad = ref(true)
  const trafficOastSaveQueued = ref(false)
  const suppressTrafficOastAutoSave = ref(false)
  const lastSavedTrafficOastConfigSnapshot = ref('')
  const trafficOastAutoSaveState = ref<'idle' | 'dirty' | 'saving' | 'saved' | 'error'>('idle')
  const trafficOastLastSavedAt = ref<string | null>(null)

  const proxyConfig = ref(createDefaultProxyConfig())
  const requestBodySizeMB = ref(2)
  const responseBodySizeMB = ref(2)
  const proxyAutoStart = ref(false)
  const trafficAnalysisPluginEnabled = ref(true)
  const browserExtensionBridgeUrl = ref('http://127.0.0.1:18931')
  const browserExtensionDirectoryPath = ref('')
  const browserExtensionBundledWithApp = ref(false)
  const isCopyingBrowserExtension = ref(false)
  const behaviorSignalSettings = ref<TrafficBehaviorSignalSettings>(
    createDefaultTrafficBehaviorSignalSettings()
  )
  const lastSavedBehaviorSignalMode = ref<TrafficBehaviorSignalMode>('proxy_inferred')
  const trafficOastConfig = ref<TrafficOastConfig>(createDefaultTrafficOastConfig())
  const {
    trafficPluginRuntimeSettings,
    isSavingTrafficPluginRuntimeSettings,
    loadTrafficPluginRuntimeSettings,
    saveTrafficPluginRuntimeSettings,
    resetTrafficPluginRuntimePolicies,
    applyTrafficPluginRuntimePreset: applyTrafficPluginRuntimePresetToPolicies,
  } = useTrafficPluginRuntimeSettings()
  const testingTrafficOastConfig = ref(false)
  const lastTrafficOastTestResult = ref<TrafficOastTestResult | null>(null)

  const proxyListeners = ref<ProxyListener[]>([createDefaultProxyListener()])
  const selectedListeners = ref<number[]>([])

  const masterInterceptionEnabled = ref(false)
  const interceptRequests = ref(true)
  const interceptResponses = ref(false)

  const requestRules = ref(createDefaultRequestRules())
  const responseRules = ref(createDefaultResponseRules())
  const autoFixNewlines = ref(false)
  const autoUpdateContentLength = ref(true)
  const autoUpdateResponseContentLength = ref(true)

  const selectedRequestRuleIndex = ref(-1)
  const selectedResponseRuleIndex = ref(-1)
  const ruleDialogRef = ref<HTMLDialogElement | null>(null)
  const editingRuleIsNew = ref(false)
  const editingRuleType = ref<'request' | 'response'>('request')
  const editingRuleIndex = ref(-1)
  const editingRule = ref<InterceptionRule>(createDefaultEditingRule())

  const currentMatchTypes = computed(() => {
    const values =
      editingRuleType.value === 'request' ? requestMatchTypeValues : responseMatchTypeValues
    return values.map(value => ({
      value,
      label: t(`trafficAnalysis.proxyConfiguration.matchTypes.${value}`),
    }))
  })

  const relationshipOptions = computed(() => {
    return relationshipValues.map(value => ({
      value,
      label: t(`trafficAnalysis.proxyConfiguration.relationships.${value}`),
    }))
  })

  const getMatchTypeLabel = (value: string) => {
    return t(`trafficAnalysis.proxyConfiguration.matchTypes.${value}`, value)
  }

  const getRelationshipLabel = (value: string) => {
    return t(`trafficAnalysis.proxyConfiguration.relationships.${value}`, value)
  }

  const normalizeTrafficOastConfig = (config: TrafficOastConfig): TrafficOastConfig => ({
    enabled: Boolean(config.enabled),
    serverBaseUrl: config.serverBaseUrl.trim().replace(/\/+$/g, ''),
    apiKey: config.apiKey.trim(),
    pollIntervalSecs: Math.min(300, Math.max(5, Math.round(config.pollIntervalSecs || 5))),
    requestTimeoutSecs: Math.min(60, Math.max(3, Math.round(config.requestTimeoutSecs || 3))),
  })

  const buildTrafficOastConfigSnapshot = (config: TrafficOastConfig) =>
    JSON.stringify(normalizeTrafficOastConfig(config))

  const applyLoadedTrafficOastConfig = (config: TrafficOastConfig) => {
    suppressTrafficOastAutoSave.value = true
    const normalized = normalizeTrafficOastConfig({
      ...createDefaultTrafficOastConfig(),
      ...config,
    })
    trafficOastConfig.value = normalized
    lastSavedTrafficOastConfigSnapshot.value = JSON.stringify(normalized)
    queueMicrotask(() => {
      suppressTrafficOastAutoSave.value = false
    })
  }

  const upstreamProxy = ref<UpstreamProxyConfig | null>(null)
  const upstreamProxies = ref<UpstreamProxyConfig[]>([])
  const selectedUpstreamIndex = ref(-1)
  const upstreamDialogRef = ref<HTMLDialogElement | null>(null)
  const editingUpstreamIsNew = ref(false)
  const editingUpstreamIndex = ref(-1)
  const editingUpstream = ref<UpstreamProxyConfig>(createDefaultEditingUpstream())

  const selectedMatchReplaceIndex = ref(-1)
  const matchReplaceDialogRef = ref<HTMLDialogElement | null>(null)
  const editingMatchReplaceIsNew = ref(false)
  const editingMatchReplaceIndex = ref(-1)
  const editingMatchReplace = ref(createDefaultEditingMatchReplace())

  const selectedTlsPassThroughIndex = ref(-1)
  const tlsPassThroughDialogRef = ref<HTMLDialogElement | null>(null)
  const editingTlsIsNew = ref(false)
  const editingTlsIndex = ref(-1)
  const editingTlsPassThrough = ref(createDefaultEditingTlsPassThrough())

  const interceptClientToServer = ref(true)
  const interceptServerToClient = ref(true)
  const onlyInterceptInScope = ref(false)

  const useHTTP1_1ToServer = ref(false)
  const useHTTP1_1ToClient = ref(false)
  const setConnectionClose = ref(false)
  const setConnectionHeader = ref(true)
  const stripProxyHeaders = ref(true)
  const removeUnsupportedEncodings = ref(true)
  const stripWebSocketExtensions = ref(true)
  const unpackCompressedRequests = ref(false)
  const unpackCompressedResponses = ref(true)

  const unhideHiddenFields = ref(false)
  const prominentlyHighlightUnhidden = ref(false)
  const enableDisabledFields = ref(false)
  const removeInputFieldLengthLimits = ref(false)
  const removeJavaScriptFormValidation = ref(false)
  const removeAllJavaScript = ref(false)
  const onlyApplyToInScope = ref(true)

  const matchReplaceRules = ref<MatchReplaceRule[]>(createDefaultMatchReplaceRules())
  const tlsPassThroughRules = ref<TlsPassThroughRule[]>(createDefaultTlsPassThroughRules())
  const autoAddTLSOnFailure = ref(false)
  const applyToOutOfScope = ref(false)

  const historyLogging = ref(true)
  const interceptionState = ref<'intercept' | 'forward'>('forward')

  const disableWebInterface = ref(false)
  const suppressBurpErrorMessages = ref(false)
  const dontSendToProxyHistory = ref(false)
  const dontSendToProxyHistoryIfOutOfScope = ref(false)

  const isDownloadingCert = ref(false)
  const isRegeneratingCert = ref(false)
  const isOpeningCertDir = ref(false)
  const certDialogRef = ref<HTMLDialogElement | null>(null)
  const certOperation = ref('')
  const isProcessingCert = ref(false)

  const editDialogRef = ref<HTMLDialogElement | null>(null)
  const editingIndex = ref(-1)
  const editingListener = ref(createDefaultEditingListener())

  const toggleListenerSelection = (index: number) => {
    const selectedIndex = selectedListeners.value.indexOf(index)
    if (selectedIndex > -1) {
      selectedListeners.value.splice(selectedIndex, 1)
      return
    }
    selectedListeners.value.push(index)
  }

  const toggleListenerRunning = async (listener: ProxyListener, index: number) => {
    try {
      if (listener.running) {
        const [host, port] = listener.interface.split(':')
        const response = await invoke<CommandResponse>('start_proxy_listener', {
          host,
          port: parseInt(port),
          index,
        })

        if (response.success) {
          dialog.toast.success(`代理监听器 ${listener.interface} 已启动`)
          return
        }

        listener.running = false
        dialog.toast.error(`启动失败: ${response.error || '端口可能被占用'}`)
        return
      }

      const response = await invoke<CommandResponse>('stop_proxy_listener', { index })
      if (response.success) {
        dialog.toast.success(`代理监听器 ${listener.interface} 已停止`)
        return
      }

      listener.running = true
      dialog.toast.error(`停止失败: ${response.error || '未知错误'}`)
    } catch (error: any) {
      console.error('Failed to toggle listener:', error)
      listener.running = !listener.running
      dialog.toast.error(`操作失败: ${error}`)
    }
  }

  const addListener = () => {
    const newPort = 8080 + proxyListeners.value.length
    proxyListeners.value.push({
      ...createDefaultProxyListener(newPort),
      running: false,
    })
    dialog.toast.success('已添加新的监听器')
  }

  const editListenerByIndex = (index: number) => {
    const listener = proxyListeners.value[index]
    const [host, portString] = listener.interface.split(':')

    editingIndex.value = index
    editingListener.value = {
      host,
      port: parseInt(portString),
      certificate: listener.certificate,
      tlsProtocols: listener.tlsProtocols,
      supportHTTP2: listener.supportHTTP2,
      invisible: listener.invisible,
      redirect: listener.redirect,
    }

    editDialogRef.value?.showModal()
  }

  const editListener = () => {
    if (selectedListeners.value.length !== 1) {
      dialog.toast.warning('请选择一个监听器进行编辑')
      return
    }

    editListenerByIndex(selectedListeners.value[0])
  }

  const saveEdit = () => {
    if (editingIndex.value === -1) return

    if (editingListener.value.port < 1024 || editingListener.value.port > 65535) {
      dialog.toast.error('端口号必须在 1024-65535 之间')
      return
    }

    if (!editingListener.value.host.trim()) {
      dialog.toast.error('绑定地址不能为空')
      return
    }

    const listener = proxyListeners.value[editingIndex.value]
    const wasRunning = listener.running

    listener.interface = `${editingListener.value.host}:${editingListener.value.port}`
    listener.certificate = editingListener.value.certificate
    listener.tlsProtocols = editingListener.value.tlsProtocols
    listener.supportHTTP2 = editingListener.value.supportHTTP2
    listener.invisible = editingListener.value.invisible
    listener.redirect = editingListener.value.redirect

    if (editingIndex.value === 0) {
      proxyConfig.value.start_port = editingListener.value.port
      console.log('[ProxyConfiguration] Updated start_port to:', editingListener.value.port)
    }

    if (wasRunning) {
      dialog.toast.warning('监听器配置已更新，请重启以应用新配置')
      listener.running = false
    }

    editDialogRef.value?.close()
    dialog.toast.success('监听器配置已保存')
    editingIndex.value = -1
    debouncedSave()
  }

  const cancelEdit = () => {
    editDialogRef.value?.close()
    editingIndex.value = -1
  }

  const onUpstreamProxyChange = () => {
    if (upstreamProxies.value.length === 0) return

    upstreamProxy.value = upstreamProxies.value[0]
    proxyConfig.value.upstream_proxy = upstreamProxy.value
    debouncedSave()
  }

  const addRequestRule = () => {
    editingRuleType.value = 'request'
    editingRuleIsNew.value = true
    editingRuleIndex.value = -1
    editingRule.value = {
      enabled: true,
      operator: requestRules.value.length > 0 ? 'And' : '',
      matchType: 'domain_name',
      relationship: 'matches',
      condition: '',
    }
    ruleDialogRef.value?.showModal()
  }

  const editRequestRule = () => {
    if (selectedRequestRuleIndex.value === -1) return
    editRequestRuleByIndex(selectedRequestRuleIndex.value)
  }

  const editRequestRuleByIndex = (index: number) => {
    editingRuleType.value = 'request'
    editingRuleIsNew.value = false
    editingRuleIndex.value = index
    editingRule.value = { ...requestRules.value[index] }
    ruleDialogRef.value?.showModal()
  }

  const removeRequestRule = () => {
    if (selectedRequestRuleIndex.value === -1) return
    requestRules.value.splice(selectedRequestRuleIndex.value, 1)
    selectedRequestRuleIndex.value = -1
    debouncedSave()
  }

  const moveRequestRuleUp = () => {
    if (selectedRequestRuleIndex.value <= 0) return
    const index = selectedRequestRuleIndex.value
    moveItem(requestRules.value, index, index - 1)
    selectedRequestRuleIndex.value = index - 1
    debouncedSave()
  }

  const moveRequestRuleDown = () => {
    if (
      selectedRequestRuleIndex.value === -1 ||
      selectedRequestRuleIndex.value >= requestRules.value.length - 1
    ) {
      return
    }
    const index = selectedRequestRuleIndex.value
    moveItem(requestRules.value, index, index + 1)
    selectedRequestRuleIndex.value = index + 1
    debouncedSave()
  }

  const addResponseRule = () => {
    editingRuleType.value = 'response'
    editingRuleIsNew.value = true
    editingRuleIndex.value = -1
    editingRule.value = {
      enabled: true,
      operator: responseRules.value.length > 0 ? 'And' : '',
      matchType: 'domain_name',
      relationship: 'matches',
      condition: '',
    }
    ruleDialogRef.value?.showModal()
  }

  const editResponseRule = () => {
    if (selectedResponseRuleIndex.value === -1) return
    editResponseRuleByIndex(selectedResponseRuleIndex.value)
  }

  const editResponseRuleByIndex = (index: number) => {
    editingRuleType.value = 'response'
    editingRuleIsNew.value = false
    editingRuleIndex.value = index
    editingRule.value = { ...responseRules.value[index] }
    ruleDialogRef.value?.showModal()
  }

  const removeResponseRule = () => {
    if (selectedResponseRuleIndex.value === -1) return
    responseRules.value.splice(selectedResponseRuleIndex.value, 1)
    selectedResponseRuleIndex.value = -1
    debouncedSave()
  }

  const moveResponseRuleUp = () => {
    if (selectedResponseRuleIndex.value <= 0) return
    const index = selectedResponseRuleIndex.value
    moveItem(responseRules.value, index, index - 1)
    selectedResponseRuleIndex.value = index - 1
    debouncedSave()
  }

  const moveResponseRuleDown = () => {
    if (
      selectedResponseRuleIndex.value === -1 ||
      selectedResponseRuleIndex.value >= responseRules.value.length - 1
    ) {
      return
    }
    const index = selectedResponseRuleIndex.value
    moveItem(responseRules.value, index, index + 1)
    selectedResponseRuleIndex.value = index + 1
    debouncedSave()
  }

  const saveRuleEdit = () => {
    const rules = editingRuleType.value === 'request' ? requestRules.value : responseRules.value

    if (editingRuleIsNew.value) {
      rules.push({ ...editingRule.value })
      if (editingRuleType.value === 'request') {
        selectedRequestRuleIndex.value = rules.length - 1
      } else {
        selectedResponseRuleIndex.value = rules.length - 1
      }
    } else {
      rules[editingRuleIndex.value] = { ...editingRule.value }
    }

    ruleDialogRef.value?.close()
    debouncedSave()
  }

  const cancelRuleEdit = () => {
    ruleDialogRef.value?.close()
  }

  const addUpstreamProxy = () => {
    editingUpstreamIsNew.value = true
    editingUpstreamIndex.value = -1
    editingUpstream.value = {
      enabled: true,
      destination_host: '*',
      proxy_host: '127.0.0.1',
      proxy_port: 10809,
      auth_type: '',
      username: '',
      password: '',
    }
    upstreamDialogRef.value?.showModal()
  }

  const editUpstreamProxyByIndex = (index: number) => {
    editingUpstreamIsNew.value = false
    editingUpstreamIndex.value = index
    editingUpstream.value = { ...upstreamProxies.value[index] }
    upstreamDialogRef.value?.showModal()
  }

  const editUpstreamProxy = () => {
    if (selectedUpstreamIndex.value === -1) return
    editUpstreamProxyByIndex(selectedUpstreamIndex.value)
  }

  const removeUpstreamProxy = () => {
    if (selectedUpstreamIndex.value === -1) return
    upstreamProxies.value.splice(selectedUpstreamIndex.value, 1)
    selectedUpstreamIndex.value = -1
    upstreamProxy.value = upstreamProxies.value.length > 0 ? upstreamProxies.value[0] : null
    proxyConfig.value.upstream_proxy = upstreamProxy.value
    debouncedSave()
  }

  const saveUpstreamEdit = () => {
    if (!editingUpstream.value.proxy_host.trim()) {
      dialog.toast.error('Proxy host is required')
      return
    }

    if (editingUpstream.value.proxy_port < 1 || editingUpstream.value.proxy_port > 65535) {
      dialog.toast.error('Port must be between 1 and 65535')
      return
    }

    if (editingUpstreamIsNew.value) {
      upstreamProxies.value.push({ ...editingUpstream.value })
      selectedUpstreamIndex.value = upstreamProxies.value.length - 1
    } else {
      upstreamProxies.value[editingUpstreamIndex.value] = { ...editingUpstream.value }
    }

    upstreamProxy.value = upstreamProxies.value.length > 0 ? upstreamProxies.value[0] : null
    proxyConfig.value.upstream_proxy = upstreamProxy.value

    upstreamDialogRef.value?.close()
    dialog.toast.success('Upstream proxy saved')
    debouncedSave()
  }

  const cancelUpstreamEdit = () => {
    upstreamDialogRef.value?.close()
  }

  const addMatchReplaceRule = () => {
    editingMatchReplaceIsNew.value = true
    editingMatchReplaceIndex.value = -1
    editingMatchReplace.value = createDefaultEditingMatchReplace()
    matchReplaceDialogRef.value?.showModal()
  }

  const editMatchReplaceRuleByIndex = (index: number) => {
    editingMatchReplaceIsNew.value = false
    editingMatchReplaceIndex.value = index
    const rule = matchReplaceRules.value[index]
    editingMatchReplace.value = {
      enabled: rule.enabled,
      type: rule.type,
      match: rule.match,
      replace: rule.replace,
      comment: rule.comment,
    }
    matchReplaceDialogRef.value?.showModal()
  }

  const editMatchReplaceRule = () => {
    if (selectedMatchReplaceIndex.value === -1) return
    editMatchReplaceRuleByIndex(selectedMatchReplaceIndex.value)
  }

  const removeMatchReplaceRule = () => {
    if (selectedMatchReplaceIndex.value === -1) return
    matchReplaceRules.value.splice(selectedMatchReplaceIndex.value, 1)
    selectedMatchReplaceIndex.value = -1
    debouncedSave()
  }

  const moveMatchReplaceRuleUp = () => {
    if (selectedMatchReplaceIndex.value <= 0) return
    const index = selectedMatchReplaceIndex.value
    moveItem(matchReplaceRules.value, index, index - 1)
    selectedMatchReplaceIndex.value = index - 1
    debouncedSave()
  }

  const moveMatchReplaceRuleDown = () => {
    if (
      selectedMatchReplaceIndex.value === -1 ||
      selectedMatchReplaceIndex.value >= matchReplaceRules.value.length - 1
    ) {
      return
    }
    const index = selectedMatchReplaceIndex.value
    moveItem(matchReplaceRules.value, index, index + 1)
    selectedMatchReplaceIndex.value = index + 1
    debouncedSave()
  }

  const saveMatchReplaceEdit = () => {
    if (editingMatchReplaceIsNew.value) {
      matchReplaceRules.value.push({
        ...editingMatchReplace.value,
        scope: onlyApplyToInScope.value ? 'In scope' : '',
        item: String(matchReplaceRules.value.length + 1),
      })
      selectedMatchReplaceIndex.value = matchReplaceRules.value.length - 1
    } else {
      const existing = matchReplaceRules.value[editingMatchReplaceIndex.value]
      matchReplaceRules.value[editingMatchReplaceIndex.value] = {
        ...existing,
        ...editingMatchReplace.value,
        scope: onlyApplyToInScope.value ? 'In scope' : '',
      }
    }

    matchReplaceDialogRef.value?.close()
    debouncedSave()
  }

  const cancelMatchReplaceEdit = () => {
    matchReplaceDialogRef.value?.close()
  }

  const addTlsPassThroughRule = () => {
    editingTlsIsNew.value = true
    editingTlsIndex.value = -1
    editingTlsPassThrough.value = createDefaultEditingTlsPassThrough()
    tlsPassThroughDialogRef.value?.showModal()
  }

  const editTlsPassThroughRuleByIndex = (index: number) => {
    editingTlsIsNew.value = false
    editingTlsIndex.value = index
    editingTlsPassThrough.value = { ...tlsPassThroughRules.value[index] }
    tlsPassThroughDialogRef.value?.showModal()
  }

  const editTlsPassThroughRule = () => {
    if (selectedTlsPassThroughIndex.value === -1) return
    editTlsPassThroughRuleByIndex(selectedTlsPassThroughIndex.value)
  }

  const removeTlsPassThroughRule = () => {
    if (selectedTlsPassThroughIndex.value === -1) return
    tlsPassThroughRules.value.splice(selectedTlsPassThroughIndex.value, 1)
    selectedTlsPassThroughIndex.value = -1
    debouncedSave()
  }

  const saveTlsPassThroughEdit = () => {
    if (!editingTlsPassThrough.value.host.trim()) {
      dialog.toast.error('Host is required')
      return
    }

    if (editingTlsIsNew.value) {
      tlsPassThroughRules.value.push({
        ...editingTlsPassThrough.value,
        destination: '*',
        protocol: 'TLS',
      })
      selectedTlsPassThroughIndex.value = tlsPassThroughRules.value.length - 1
    } else {
      const existing = tlsPassThroughRules.value[editingTlsIndex.value]
      tlsPassThroughRules.value[editingTlsIndex.value] = {
        ...existing,
        ...editingTlsPassThrough.value,
      }
    }

    tlsPassThroughDialogRef.value?.close()
    debouncedSave()
  }

  const cancelTlsPassThroughEdit = () => {
    tlsPassThroughDialogRef.value?.close()
  }

  const pasteUrlToTlsPassThrough = async () => {
    try {
      const text = await navigator.clipboard.readText()
      if (!text) return

      const url = new URL(text)
      editingTlsPassThrough.value = {
        enabled: true,
        host: url.hostname,
        port: url.port || '443',
      }
      editingTlsIsNew.value = true
      tlsPassThroughDialogRef.value?.showModal()
    } catch {
      dialog.toast.error('Failed to paste URL from clipboard')
    }
  }

  const removeListener = async () => {
    if (selectedListeners.value.length === 0) {
      dialog.toast.warning('请至少选择一个监听器')
      return
    }

    const sortedIndices = [...selectedListeners.value].sort((a, b) => b - a)
    for (const index of sortedIndices) {
      const listener = proxyListeners.value[index]
      if (listener.running) {
        try {
          await invoke('stop_proxy_listener', { index })
        } catch (error) {
          console.error('Failed to stop listener before removal:', error)
        }
      }
      proxyListeners.value.splice(index, 1)
    }

    selectedListeners.value = []
    dialog.toast.success('已删除选中的监听器')
  }

  const updateRequestBodySize = () => {
    proxyConfig.value.max_request_body_size = requestBodySizeMB.value * 1024 * 1024
    debouncedSave()
  }

  const updateResponseBodySize = () => {
    proxyConfig.value.max_response_body_size = responseBodySizeMB.value * 1024 * 1024
    debouncedSave()
  }

  const saveConfiguration = async () => {
    try {
      isSaving.value = true
      const configToSave = {
        ...proxyConfig.value,
        match_replace_rules: matchReplaceRules.value.map(rule => ({ ...rule })),
      }
      console.log('[ProxyConfiguration] Saving configuration...', configToSave)

      const response = await invoke<CommandResponse>('save_proxy_config', {
        config: configToSave,
      })
      if (!response.success) {
        throw new Error(response.error || '保存失败')
      }

      try {
        localStorage.setItem('proxy_request_filter_rules', JSON.stringify(requestRules.value))
        localStorage.setItem('proxy_response_filter_rules', JSON.stringify(responseRules.value))
        console.log('[ProxyConfiguration] Filter rules saved to localStorage')
      } catch (error) {
        console.error('[ProxyConfiguration] Failed to save filter rules to localStorage:', error)
      }

      dialog.toast.success('修改成功')
    } catch (error: any) {
      console.error('[ProxyConfiguration] Failed to save configuration:', error)
      dialog.toast.error(`保存配置失败: ${error}`)
    } finally {
      isSaving.value = false
    }
  }

  const debouncedSave = () => {
    if (isInitialLoad.value) return

    if (isSaving.value) {
      saveAgainAfterCurrent.value = true
      return
    }

    if (saveQueued.value) return

    saveQueued.value = true
    queueMicrotask(async () => {
      saveQueued.value = false
      if (isSaving.value) {
        saveAgainAfterCurrent.value = true
        return
      }

      await saveConfiguration()
      if (saveAgainAfterCurrent.value && !isInitialLoad.value) {
        saveAgainAfterCurrent.value = false
        debouncedSave()
      }
    })
  }

  const debouncedLocalSettingsToast = () => {
    if (isInitialLoad.value) return

    if (localSettingsToastTimeout.value) {
      clearTimeout(localSettingsToastTimeout.value)
    }

    localSettingsToastTimeout.value = setTimeout(() => {
      dialog.toast.success('修改成功')
      localSettingsToastTimeout.value = null
    }, 0)
  }

  const resetToDefaults = () => {
    proxyConfig.value = createDefaultProxyConfig()
    matchReplaceRules.value = createDefaultMatchReplaceRules()
    selectedMatchReplaceIndex.value = -1
    requestBodySizeMB.value = 2
    responseBodySizeMB.value = 2
    upstreamProxy.value = null
    dialog.toast.info('已重置为默认配置')
  }

  function openCertDialog() {
    certOperation.value = ''
    certDialogRef.value?.showModal()
  }

  function closeCertDialog() {
    certDialogRef.value?.close()
    certOperation.value = ''
  }

  async function exportCertInDer() {
    const response = await invoke<CommandResponse<{ path: string }>>('export_ca_cert', {
      format: 'der',
    })
    if (response.success && response.data) {
      dialog.toast.success(
        `${t('trafficAnalysis.proxyConfiguration.certInDerFormat')}: ${response.data.path}`
      )
      return
    }
    throw new Error(response.error || 'Export failed')
  }

  async function exportKeyInDer() {
    const response = await invoke<CommandResponse<{ path: string }>>('export_ca_key', {
      format: 'der',
    })
    if (response.success && response.data) {
      dialog.toast.success(
        `${t('trafficAnalysis.proxyConfiguration.privateKeyInDerFormat')}: ${response.data.path}`
      )
      return
    }
    throw new Error(response.error || 'Export failed')
  }

  async function exportPkcs12() {
    const response = await invoke<CommandResponse<{ path: string }>>('export_ca_pkcs12', {})
    if (response.success && response.data) {
      dialog.toast.success(
        `${t('trafficAnalysis.proxyConfiguration.certAndKeyInPkcs12')}: ${response.data.path}`
      )
      return
    }
    throw new Error(response.error || 'Export failed')
  }

  async function importDerCert() {
    const response = await invoke<CommandResponse>('import_ca_der', {})
    if (response.success) {
      dialog.toast.success('Certificate imported successfully')
      return
    }
    throw new Error(response.error || 'Import failed')
  }

  async function importPkcs12() {
    const response = await invoke<CommandResponse>('import_ca_pkcs12', {})
    if (response.success) {
      dialog.toast.success('Certificate imported successfully')
      return
    }
    throw new Error(response.error || 'Import failed')
  }

  async function executeCertOperation() {
    if (!certOperation.value) return

    isProcessingCert.value = true
    try {
      switch (certOperation.value) {
        case 'export_der_cert':
          await exportCertInDer()
          break
        case 'export_der_key':
          await exportKeyInDer()
          break
        case 'export_pkcs12':
          await exportPkcs12()
          break
        case 'import_der':
          await importDerCert()
          break
        case 'import_pkcs12':
          await importPkcs12()
          break
      }
      closeCertDialog()
    } catch (error: any) {
      console.error('Certificate operation failed:', error)
      dialog.toast.error(`${error}`)
    } finally {
      isProcessingCert.value = false
    }
  }

  async function downloadCACert() {
    isDownloadingCert.value = true
    try {
      const response = await invoke<CommandResponse<{ path: string }>>('download_ca_cert')
      if (response.success && response.data) {
        dialog.toast.success(`证书已下载到: ${response.data.path}`)
        return
      }
      dialog.toast.error(`下载证书失败: ${response.message || '未知错误'}`)
    } catch (error: any) {
      console.error('Failed to download CA cert:', error)
      dialog.toast.error(`下载证书失败: ${error}`)
    } finally {
      isDownloadingCert.value = false
    }
  }

  async function regenerateCACert() {
    isRegeneratingCert.value = true
    try {
      await invoke('regenerate_ca_cert')
      dialog.toast.success('证书已重新生成，请重新安装到系统')
    } catch (error: any) {
      console.error('Failed to regenerate CA cert:', error)
      dialog.toast.error(`重新生成证书失败: ${error}`)
    } finally {
      isRegeneratingCert.value = false
    }
  }

  async function openCertDir() {
    isOpeningCertDir.value = true
    try {
      const response = await invoke<CommandResponse<string>>('open_ca_cert_dir')
      if (response.success) {
        dialog.toast.success(`已打开证书目录: ${response.data}`)
        return
      }
      dialog.toast.error(`打开证书目录失败: ${response.error || '未知错误'}`)
    } catch (error: any) {
      console.error('Failed to open cert directory:', error)
      dialog.toast.error(`打开证书目录失败: ${error}`)
    } finally {
      isOpeningCertDir.value = false
    }
  }

  const saveProxyAutoStart = async () => {
    try {
      console.log('[ProxyConfiguration] Saving proxy auto-start:', proxyAutoStart.value)
      const response = await invoke<CommandResponse>('set_proxy_auto_start', {
        enabled: proxyAutoStart.value,
      })

      if (!response.success) {
        throw new Error(response.error || '保存失败')
      }

      dialog.toast.success(proxyAutoStart.value ? '已启用代理自动启动' : '已禁用代理自动启动')
    } catch (error: any) {
      console.error('[ProxyConfiguration] Failed to save proxy auto-start:', error)
      dialog.toast.error(`保存配置失败: ${error}`)
      proxyAutoStart.value = !proxyAutoStart.value
    }
  }

  const saveTrafficAnalysisPluginEnabled = async () => {
    try {
      console.log(
        '[ProxyConfiguration] Saving traffic analysis plugin enabled:',
        trafficAnalysisPluginEnabled.value
      )
      const response = await invoke<CommandResponse>('set_traffic_analysis_plugin_enabled', {
        enabled: trafficAnalysisPluginEnabled.value,
      })

      if (!response.success) {
        throw new Error(response.error || '保存失败')
      }

      dialog.toast.success(
        trafficAnalysisPluginEnabled.value ? '已启用流量分析插件扫描' : '已禁用流量分析插件扫描'
      )
    } catch (error: any) {
      console.error('[ProxyConfiguration] Failed to save traffic analysis plugin enabled:', error)
      dialog.toast.error(`保存配置失败: ${error}`)
      trafficAnalysisPluginEnabled.value = !trafficAnalysisPluginEnabled.value
    }
  }

  const saveTrafficBehaviorSignalSettings = async () => {
    const previous = lastSavedBehaviorSignalMode.value
    try {
      const response = await invoke<CommandResponse<TrafficBehaviorSignalSettings>>(
        'set_traffic_behavior_signal_settings',
        {
          payload: {
            mode: behaviorSignalSettings.value.mode,
          },
        }
      )

      if (!response.success || !response.data) {
        throw new Error(response.error || '保存失败')
      }

      behaviorSignalSettings.value = response.data
      lastSavedBehaviorSignalMode.value = response.data.mode
      dialog.toast.success(
        behaviorSignalSettings.value.mode === 'browser_extension'
          ? '已切换为浏览器扩展增强模式'
          : '已切换为代理侧弱行为推断模式'
      )
    } catch (error: any) {
      console.error('[ProxyConfiguration] Failed to save behavior signal settings:', error)
      behaviorSignalSettings.value.mode = previous
      dialog.toast.error(`保存配置失败: ${error}`)
    }
  }

  const resetTrafficPluginRuntimeSettings = async () => {
    await resetTrafficPluginRuntimePolicies(['activeProbe'])
  }

  const applyTrafficPluginRuntimePreset = (preset: 'local_fast' | 'balanced' | 'conservative') => {
    applyTrafficPluginRuntimePresetToPolicies(preset, ['activeProbe'])
  }

  const saveTrafficOastConfig = async () => {
    const requestConfig = normalizeTrafficOastConfig(trafficOastConfig.value)
    const requestSnapshot = JSON.stringify(requestConfig)

    try {
      trafficOastAutoSaveState.value = 'saving'
      const response = await invoke<CommandResponse<TrafficOastConfig>>('set_traffic_oast_config', {
        payload: {
          config: requestConfig,
        },
      })

      if (!response.success || !response.data) {
        throw new Error(response.error || '保存失败')
      }

      if (buildTrafficOastConfigSnapshot(trafficOastConfig.value) !== requestSnapshot) {
        return
      }

      applyLoadedTrafficOastConfig(response.data)
      trafficOastLastSavedAt.value = new Date().toISOString()
      trafficOastAutoSaveState.value = 'saved'
      dialog.toast.success('修改成功')
    } catch (error: any) {
      if (buildTrafficOastConfigSnapshot(trafficOastConfig.value) !== requestSnapshot) {
        return
      }

      trafficOastAutoSaveState.value = 'error'
      console.error('[ProxyConfiguration] Failed to save traffic OAST config:', error)
      dialog.toast.error(`保存 OAST 配置失败: ${error}`)
    }
  }

  const debouncedSaveTrafficOastConfig = () => {
    if (isInitialLoad.value || suppressTrafficOastAutoSave.value) {
      return
    }

    if (trafficOastSaveQueued.value) return

    trafficOastSaveQueued.value = true
    queueMicrotask(() => {
      trafficOastSaveQueued.value = false
      void saveTrafficOastConfig()
    })
  }

  const testTrafficOastConfig = async () => {
    testingTrafficOastConfig.value = true
    try {
      const response = await invoke<CommandResponse<TrafficOastTestResult>>(
        'test_traffic_oast_config_command',
        {
          payload: {
            config: trafficOastConfig.value,
          },
        }
      )

      if (!response.success || !response.data) {
        throw new Error(response.error || '测试失败')
      }

      lastTrafficOastTestResult.value = response.data
      dialog.toast.success('OAST 服务连通性正常')
    } catch (error: any) {
      console.error('[ProxyConfiguration] Failed to test traffic OAST config:', error)
      lastTrafficOastTestResult.value = {
        reachable: false,
        message: String(error),
        generatedToken: null,
        generatedFqdn: null,
      }
      dialog.toast.error(`OAST 测试失败: ${error}`)
    } finally {
      testingTrafficOastConfig.value = false
    }
  }

  const copyBrowserExtensionBridgeUrl = async () => {
    try {
      await navigator.clipboard.writeText(browserExtensionBridgeUrl.value)
      dialog.toast.success(t('trafficAnalysis.proxyConfiguration.copiedToClipboard'))
    } catch (error) {
      console.error('[ProxyConfiguration] Failed to copy bridge URL:', error)
      dialog.toast.error(t('trafficAnalysis.proxyConfiguration.copyFailed', '复制失败'))
    }
  }

  const copyBrowserExtensionDirectory = async () => {
    try {
      await navigator.clipboard.writeText(browserExtensionDirectoryPath.value)
      dialog.toast.success(t('trafficAnalysis.proxyConfiguration.copiedToClipboard'))
    } catch (error) {
      console.error('[ProxyConfiguration] Failed to copy extension directory:', error)
      dialog.toast.error(t('trafficAnalysis.proxyConfiguration.copyFailed', '复制失败'))
    }
  }

  const copyBrowserExtensionToDirectory = async () => {
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        title: t('trafficAnalysis.proxyConfiguration.selectExtensionCopyTarget'),
      })

      if (!selected || Array.isArray(selected)) {
        return
      }

      isCopyingBrowserExtension.value = true
      const response = await invoke<CommandResponse<CopyTrafficBehaviorExtensionResult>>(
        'copy_traffic_behavior_extension_to_directory',
        {
          targetDirectory: selected,
        }
      )

      if (!response.success || !response.data) {
        throw new Error(response.error || '复制失败')
      }

      dialog.toast.success(
        t('trafficAnalysis.proxyConfiguration.copyExtensionToDirectorySuccess', {
          path: response.data.copiedDirectory,
        })
      )
    } catch (error) {
      console.error('[ProxyConfiguration] Failed to copy browser extension to directory:', error)
      dialog.toast.error(
        t('trafficAnalysis.proxyConfiguration.copyExtensionToDirectoryFailed', {
          error: error instanceof Error ? error.message : String(error),
        })
      )
    } finally {
      isCopyingBrowserExtension.value = false
    }
  }

  const loadConfig = async () => {
    try {
      console.log('[ProxyConfiguration] Loading config...')

      const configResponse =
        await invoke<CommandResponse<typeof proxyConfig.value>>('get_proxy_config')
      if (configResponse.success && configResponse.data) {
        proxyConfig.value = {
          ...createDefaultProxyConfig(),
          ...configResponse.data,
          scope_include_rules: normalizeScopeRules(configResponse.data.scope_include_rules),
          scope_exclude_rules: normalizeScopeRules(configResponse.data.scope_exclude_rules),
        }
        matchReplaceRules.value = Array.isArray(configResponse.data.match_replace_rules)
          ? configResponse.data.match_replace_rules.map(rule => ({
              ...rule,
            }))
          : createDefaultMatchReplaceRules()
        onlyApplyToInScope.value =
          matchReplaceRules.value.length === 0 ||
          matchReplaceRules.value.every(rule => rule.scope.trim().toLowerCase() === 'in scope')
        selectedMatchReplaceIndex.value = -1
        requestBodySizeMB.value = Math.round(
          configResponse.data.max_request_body_size / (1024 * 1024)
        )
        responseBodySizeMB.value = Math.round(
          configResponse.data.max_response_body_size / (1024 * 1024)
        )
        proxyListeners.value[0].interface = `127.0.0.1:${configResponse.data.start_port}`

        if (configResponse.data.upstream_proxy) {
          upstreamProxy.value = configResponse.data.upstream_proxy
          console.log('[ProxyConfiguration] Loaded upstream proxy:', upstreamProxy.value)
        } else {
          upstreamProxy.value = null
        }
      }

      const autoStartResponse = await invoke<CommandResponse<boolean>>('get_proxy_auto_start')
      if (autoStartResponse.success) {
        proxyAutoStart.value = autoStartResponse.data || false
        console.log('[ProxyConfiguration] Loaded proxy auto-start:', proxyAutoStart.value)
      }

      const pluginEnabledResponse = await invoke<CommandResponse<boolean>>(
        'get_traffic_analysis_plugin_enabled'
      )
      if (pluginEnabledResponse.success) {
        trafficAnalysisPluginEnabled.value = pluginEnabledResponse.data !== false
        console.log(
          '[ProxyConfiguration] Loaded traffic analysis plugin enabled:',
          trafficAnalysisPluginEnabled.value
        )
      }

      const behaviorSignalResponse = await invoke<CommandResponse<TrafficBehaviorSignalSettings>>(
        'get_traffic_behavior_signal_settings'
      )
      if (behaviorSignalResponse.success && behaviorSignalResponse.data) {
        behaviorSignalSettings.value = behaviorSignalResponse.data
        lastSavedBehaviorSignalMode.value = behaviorSignalResponse.data.mode
        console.log(
          '[ProxyConfiguration] Loaded traffic behavior signal settings:',
          behaviorSignalSettings.value
        )
      }

      await loadTrafficPluginRuntimeSettings()

      const trafficOastResponse =
        await invoke<CommandResponse<TrafficOastConfig>>('get_traffic_oast_config')
      if (trafficOastResponse.success && trafficOastResponse.data) {
        applyLoadedTrafficOastConfig(trafficOastResponse.data)
        lastTrafficOastTestResult.value = null
        trafficOastAutoSaveState.value = 'idle'
      }

      try {
        const extensionInstallationResponse = await invoke<
          CommandResponse<TrafficBehaviorExtensionInstallation>
        >('get_traffic_behavior_extension_installation')
        if (extensionInstallationResponse.success && extensionInstallationResponse.data) {
          browserExtensionBridgeUrl.value = extensionInstallationResponse.data.bridgeUrl
          browserExtensionDirectoryPath.value =
            extensionInstallationResponse.data.extensionDirectory
          browserExtensionBundledWithApp.value = extensionInstallationResponse.data.bundledWithApp
        }
      } catch (error) {
        console.warn(
          '[ProxyConfiguration] Failed to load browser extension installation info:',
          error
        )
      }

      const statusResponse =
        await invoke<CommandResponse<{ running: boolean; port: number }>>('get_proxy_status')
      if (statusResponse.success && statusResponse.data) {
        const { running, port } = statusResponse.data
        if (running && port > 0) {
          const listenerIndex = proxyListeners.value.findIndex(
            listener => listener.interface === `127.0.0.1:${port}`
          )
          if (listenerIndex !== -1) {
            proxyListeners.value[listenerIndex].running = true
          } else {
            proxyListeners.value[0].interface = `127.0.0.1:${port}`
            proxyListeners.value[0].running = true
          }
          console.log(`[ProxyConfiguration] Proxy is running on port ${port}`)
        } else {
          proxyListeners.value.forEach(listener => {
            listener.running = false
          })
          console.log('[ProxyConfiguration] Proxy is not running')
        }
      }

      const interceptResponse = await invoke<CommandResponse<boolean>>('get_intercept_enabled')
      if (interceptResponse.success) {
        const enabled = Boolean(interceptResponse.data)
        masterInterceptionEnabled.value = enabled
        console.log('[ProxyConfiguration] Master intercept:', interceptResponse.data)
      }

      const requestInterceptResponse = await invoke<CommandResponse<boolean>>(
        'get_request_intercept_enabled'
      )
      if (requestInterceptResponse.success) {
        interceptRequests.value = Boolean(requestInterceptResponse.data)
        console.log('[ProxyConfiguration] Request intercept:', requestInterceptResponse.data)
      }

      const responseInterceptResponse = await invoke<CommandResponse<boolean>>(
        'get_response_intercept_enabled'
      )
      if (responseInterceptResponse.success) {
        interceptResponses.value = Boolean(responseInterceptResponse.data)
        console.log('[ProxyConfiguration] Response intercept:', responseInterceptResponse.data)
      }

      try {
        const savedRequestRules = localStorage.getItem('proxy_request_filter_rules')
        if (savedRequestRules) {
          const parsed = JSON.parse(savedRequestRules)
          if (Array.isArray(parsed) && parsed.length > 0) {
            requestRules.value = parsed
            console.log(
              '[ProxyConfiguration] Loaded request filter rules from localStorage:',
              parsed.length
            )
          }
        }

        const savedResponseRules = localStorage.getItem('proxy_response_filter_rules')
        if (savedResponseRules) {
          const parsed = JSON.parse(savedResponseRules)
          if (Array.isArray(parsed) && parsed.length > 0) {
            responseRules.value = parsed
            console.log(
              '[ProxyConfiguration] Loaded response filter rules from localStorage:',
              parsed.length
            )
          }
        }
      } catch (error) {
        console.error('[ProxyConfiguration] Failed to load filter rules from localStorage:', error)
      }
    } catch (error) {
      console.error('[ProxyConfiguration] Failed to load config or status:', error)
      proxyListeners.value.forEach(listener => {
        listener.running = false
      })
    }
  }

  const autoStartProxy = async () => {
    if (proxyListeners.value.length === 0 || proxyListeners.value[0].running) return

    console.log('[ProxyConfiguration] Manually starting proxy listener...')
    const listener = proxyListeners.value[0]
    try {
      const [host, port] = listener.interface.split(':')
      const response = await invoke<CommandResponse>('start_proxy_listener', {
        host,
        port: parseInt(port),
        index: 0,
      })

      if (response.success) {
        listener.running = true
        console.log(`[ProxyConfiguration] Proxy listener ${listener.interface} manually started`)
        return
      }

      console.warn(
        `[ProxyConfiguration] Failed to start proxy: ${response.error || 'port may be in use'}`
      )
    } catch (error: any) {
      console.warn('[ProxyConfiguration] Failed to start proxy:', error)
    }
  }

  function handleAddFilterRule(payload: FilterRulePayload) {
    console.log('[ProxyConfiguration] Received filter rule:', payload)

    const rules = payload.ruleType === 'request' ? requestRules.value : responseRules.value
    const existingIndex = rules.findIndex(
      rule => rule.matchType === payload.rule.matchType && rule.condition === payload.rule.condition
    )

    if (existingIndex !== -1) {
      rules[existingIndex] = { ...payload.rule }
      console.log('[ProxyConfiguration] Updated existing rule at index:', existingIndex)
    } else {
      rules.push({ ...payload.rule })
      console.log('[ProxyConfiguration] Added new rule')
    }

    debouncedSave()
  }

  const syncFilterRulesToBackend = async () => {
    if (isInitialLoad.value) return

    try {
      const requestPayload = buildRuntimeRulePayload(requestRules.value)
      await invoke('update_runtime_filter_rules', {
        ruleType: 'request',
        rules: requestPayload,
      })
      console.log(
        '[ProxyConfiguration] Request filter rules synced to backend:',
        requestPayload.length
      )

      const responsePayload = buildRuntimeRulePayload(responseRules.value)
      await invoke('update_runtime_filter_rules', {
        ruleType: 'response',
        rules: responsePayload,
      })
      console.log(
        '[ProxyConfiguration] Response filter rules synced to backend:',
        responsePayload.length
      )
    } catch (error) {
      console.error('[ProxyConfiguration] Failed to sync filter rules:', error)
    }
  }

  const addRequestFilterRule = (
    matchType: string,
    condition: string,
    relationship: string = 'matches'
  ) => {
    const newRule = {
      enabled: true,
      operator: requestRules.value.length > 0 ? 'And' : '',
      matchType,
      relationship,
      condition,
    }
    requestRules.value.push(newRule)
    selectedRequestRuleIndex.value = requestRules.value.length - 1
    debouncedSave()

    emitFilterRuleAdded({ matchType, condition, relationship })
    dialog.toast.success(`Filter rule added: ${matchType} ${relationship} ${condition}`)
  }

  let unlistenProxyStatus: (() => void) | null = null
  let unlistenFilterRule: (() => void) | null = null
  let unlistenBehaviorStatus: (() => void) | null = null

  onMounted(async () => {
    await loadConfig()

    console.log(
      '[ProxyConfiguration] Configuration loaded, proxy auto-start is now handled by backend'
    )

    setTimeout(() => {
      isInitialLoad.value = false
      console.log('[ProxyConfiguration] Auto-save enabled')
    }, 500)

    unlistenProxyStatus = await listen('proxy:status', (event: any) => {
      const payload = event.payload
      console.log('Received proxy status event:', payload)

      if (payload.running && payload.port > 0) {
        const listenerIndex = proxyListeners.value.findIndex(
          listener => listener.interface === `127.0.0.1:${payload.port}`
        )
        if (listenerIndex !== -1) {
          proxyListeners.value[listenerIndex].running = true
        } else {
          proxyListeners.value[0].interface = `127.0.0.1:${payload.port}`
          proxyListeners.value[0].running = true
        }
      } else {
        proxyListeners.value.forEach(listener => {
          listener.running = false
        })
      }
    })

    unlistenFilterRule = await listen<FilterRulePayload>('intercept:add-filter-rule', event => {
      handleAddFilterRule(event.payload)
    })

    unlistenBehaviorStatus = await listen<{
      connected: boolean
      lastSeenAt?: string | null
    }>('traffic-behavior:extension-status', event => {
      behaviorSignalSettings.value = {
        ...behaviorSignalSettings.value,
        browserExtensionConnected: event.payload.connected,
        browserExtensionLastSeenAt: event.payload.lastSeenAt || null,
      }
    })
  })

  onUnmounted(() => {
    if (unlistenProxyStatus) unlistenProxyStatus()
    if (unlistenFilterRule) unlistenFilterRule()
    if (unlistenBehaviorStatus) unlistenBehaviorStatus()
    if (localSettingsToastTimeout.value) {
      clearTimeout(localSettingsToastTimeout.value)
    }
  })

  watch(refreshTrigger, async () => {
    console.log('[ProxyConfiguration] Refresh triggered by parent')
    isInitialLoad.value = true
    await loadConfig()
    setTimeout(() => {
      isInitialLoad.value = false
    }, 500)
  })

  watch(
    proxyConfig,
    () => {
      console.log('[ProxyConfiguration] Config changed, triggering auto-save')
      debouncedSave()
    },
    { deep: true }
  )

  watch(interceptRequests, async newValue => {
    if (isInitialLoad.value) return

    console.log('[ProxyConfiguration] Request intercept changed:', newValue)
    try {
      await invoke('set_request_intercept_enabled', { enabled: newValue })
      dialog.toast.success('修改成功')
    } catch (error) {
      console.error('[ProxyConfiguration] Failed to set request intercept:', error)
    }
  })

  watch(interceptResponses, async newValue => {
    if (isInitialLoad.value) return

    console.log('[ProxyConfiguration] Response intercept changed:', newValue)
    try {
      await invoke('set_response_intercept_enabled', { enabled: newValue })
      dialog.toast.success('修改成功')
    } catch (error) {
      console.error('[ProxyConfiguration] Failed to set response intercept:', error)
    }
  })

  watch(
    requestRules,
    () => {
      if (isInitialLoad.value) return
      console.log('[ProxyConfiguration] Request rules changed, syncing to backend')
      debouncedSave()
      syncFilterRulesToBackend()
    },
    { deep: true }
  )

  watch(
    responseRules,
    () => {
      if (isInitialLoad.value) return
      console.log('[ProxyConfiguration] Response rules changed, syncing to backend')
      debouncedSave()
      syncFilterRulesToBackend()
    },
    { deep: true }
  )

  watch(
    matchReplaceRules,
    () => {
      if (isInitialLoad.value) return
      console.log('[ProxyConfiguration] Match-replace rules changed, triggering auto-save')
      debouncedSave()
    },
    { deep: true }
  )

  watch(onlyApplyToInScope, value => {
    if (isInitialLoad.value) return

    const nextScope = value ? 'In scope' : ''
    let changed = false
    matchReplaceRules.value.forEach(rule => {
      if (rule.scope === nextScope) return
      rule.scope = nextScope
      changed = true
    })

    if (changed) {
      debouncedSave()
    }
  })

  watch(
    [
      autoFixNewlines,
      autoUpdateContentLength,
      autoUpdateResponseContentLength,
      interceptClientToServer,
      interceptServerToClient,
      onlyInterceptInScope,
      onlyApplyToInScope,
      historyLogging,
      interceptionState,
    ],
    debouncedLocalSettingsToast
  )

  watch(
    trafficOastConfig,
    () => {
      if (isInitialLoad.value || suppressTrafficOastAutoSave.value) return

      const snapshot = buildTrafficOastConfigSnapshot(trafficOastConfig.value)
      if (snapshot === lastSavedTrafficOastConfigSnapshot.value) {
        return
      }

      trafficOastAutoSaveState.value = 'dirty'
      lastTrafficOastTestResult.value = null
      debouncedSaveTrafficOastConfig()
    },
    { deep: true }
  )

  return {
    isSaving,
    proxyConfig,
    requestBodySizeMB,
    responseBodySizeMB,
    proxyAutoStart,
    trafficAnalysisPluginEnabled,
    browserExtensionBridgeUrl,
    browserExtensionDirectoryPath,
    browserExtensionBundledWithApp,
    isCopyingBrowserExtension,
    behaviorSignalSettings,
    trafficPluginRuntimeSettings,
    isSavingTrafficPluginRuntimeSettings,
    trafficOastConfig,
    trafficOastAutoSaveState,
    trafficOastLastSavedAt,
    testingTrafficOastConfig,
    lastTrafficOastTestResult,
    proxyListeners,
    selectedListeners,
    masterInterceptionEnabled,
    interceptRequests,
    interceptResponses,
    requestRules,
    responseRules,
    autoFixNewlines,
    autoUpdateContentLength,
    autoUpdateResponseContentLength,
    selectedRequestRuleIndex,
    selectedResponseRuleIndex,
    ruleDialogRef,
    editingRuleIsNew,
    editingRuleType,
    editingRule,
    currentMatchTypes,
    relationshipOptions,
    getMatchTypeLabel,
    getRelationshipLabel,
    upstreamProxy,
    upstreamProxies,
    selectedUpstreamIndex,
    upstreamDialogRef,
    editingUpstreamIsNew,
    editingUpstream,
    selectedMatchReplaceIndex,
    matchReplaceDialogRef,
    editingMatchReplaceIsNew,
    editingMatchReplace,
    selectedTlsPassThroughIndex,
    tlsPassThroughDialogRef,
    editingTlsIsNew,
    editingTlsPassThrough,
    interceptClientToServer,
    interceptServerToClient,
    onlyInterceptInScope,
    useHTTP1_1ToServer,
    useHTTP1_1ToClient,
    setConnectionClose,
    setConnectionHeader,
    stripProxyHeaders,
    removeUnsupportedEncodings,
    stripWebSocketExtensions,
    unpackCompressedRequests,
    unpackCompressedResponses,
    unhideHiddenFields,
    prominentlyHighlightUnhidden,
    enableDisabledFields,
    removeInputFieldLengthLimits,
    removeJavaScriptFormValidation,
    removeAllJavaScript,
    onlyApplyToInScope,
    matchReplaceRules,
    tlsPassThroughRules,
    autoAddTLSOnFailure,
    applyToOutOfScope,
    historyLogging,
    interceptionState,
    disableWebInterface,
    suppressBurpErrorMessages,
    dontSendToProxyHistory,
    dontSendToProxyHistoryIfOutOfScope,
    isDownloadingCert,
    isRegeneratingCert,
    isOpeningCertDir,
    certDialogRef,
    certOperation,
    isProcessingCert,
    editDialogRef,
    editingListener,
    toggleListenerSelection,
    toggleListenerRunning,
    addListener,
    editListenerByIndex,
    editListener,
    saveEdit,
    cancelEdit,
    onUpstreamProxyChange,
    addRequestRule,
    editRequestRule,
    editRequestRuleByIndex,
    removeRequestRule,
    moveRequestRuleUp,
    moveRequestRuleDown,
    addResponseRule,
    editResponseRule,
    editResponseRuleByIndex,
    removeResponseRule,
    moveResponseRuleUp,
    moveResponseRuleDown,
    saveRuleEdit,
    cancelRuleEdit,
    addUpstreamProxy,
    editUpstreamProxyByIndex,
    editUpstreamProxy,
    removeUpstreamProxy,
    saveUpstreamEdit,
    cancelUpstreamEdit,
    addMatchReplaceRule,
    editMatchReplaceRuleByIndex,
    editMatchReplaceRule,
    removeMatchReplaceRule,
    moveMatchReplaceRuleUp,
    moveMatchReplaceRuleDown,
    saveMatchReplaceEdit,
    cancelMatchReplaceEdit,
    addTlsPassThroughRule,
    editTlsPassThroughRuleByIndex,
    editTlsPassThroughRule,
    removeTlsPassThroughRule,
    saveTlsPassThroughEdit,
    cancelTlsPassThroughEdit,
    pasteUrlToTlsPassThrough,
    removeListener,
    updateRequestBodySize,
    updateResponseBodySize,
    debouncedSave,
    resetToDefaults,
    openCertDialog,
    closeCertDialog,
    executeCertOperation,
    downloadCACert,
    regenerateCACert,
    openCertDir,
    saveProxyAutoStart,
    saveTrafficAnalysisPluginEnabled,
    saveTrafficBehaviorSignalSettings,
    saveTrafficPluginRuntimeSettings,
    applyTrafficPluginRuntimePreset,
    resetTrafficPluginRuntimePolicies,
    resetTrafficPluginRuntimeSettings,
    testTrafficOastConfig,
    copyBrowserExtensionBridgeUrl,
    copyBrowserExtensionDirectory,
    copyBrowserExtensionToDirectory,
    loadConfig,
    autoStartProxy,
    addRequestFilterRule,
  }
}
