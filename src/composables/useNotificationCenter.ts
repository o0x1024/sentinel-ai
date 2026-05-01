import { computed, ref } from 'vue'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { getCurrentWindow } from '@tauri-apps/api/window'
import type { Router } from 'vue-router'
import i18n from '@/i18n'
import {
  NOTIFICATION_STORAGE_KEY,
  persistNotificationItems,
} from '@/composables/notificationCenterStorage'
import { useNotificationPreferences } from '@/composables/useNotificationPreferences'
import { useToast } from '@/composables/useToast'
import type {
  AppNotificationItem,
  NotificationCategory,
  NotificationRouteTarget,
  NotificationSource,
} from '@/types/notification'

const MAX_ITEMS = 120

interface WorkflowRunCompletePayload {
  execution_id?: string
  status?: string
  errors?: Array<unknown>
  results?: Record<string, unknown>
}

interface WorkflowResultSummaryPayload {
  execution_id?: string
  workflow_name?: string
  status?: string
  findings_count?: number
  asset_count?: number
  errors_count?: number
  completed_at?: string
}

interface WorkflowRunStartPayload {
  execution_id?: string
  workflow_name?: string
}

interface AgentExecutionFinishedPayload {
  execution_id?: string
  outcome?: 'succeeded' | 'failed' | 'cancelled'
  success?: boolean
  error?: string | null
  response?: string | null
  message?: string | null
}

interface MonitorTaskSummaryPayload {
  task_id?: string
  task_name?: string
  program_id?: string
  execution_mode?: string
  status?: string
  imported_assets?: number
  findings_created?: number
  findings_updated?: number
  started_at?: string
  completed_at?: string
  message?: string | null
}

interface MonitorPluginFailurePayload {
  task_id?: string
  task_name?: string
  program_id?: string
  execution_mode?: string
  plugin_id?: string
  plugin_label?: string
  error?: string | null
  started_at?: string
  created_at?: string
}

type PushNotificationInput = Omit<AppNotificationItem, 'id' | 'read' | 'createdAt'> & {
  id?: string
  createdAt?: string
}

const toast = useToast()
const { preferences, isSourceEnabled } = useNotificationPreferences()
const items = ref<AppNotificationItem[]>(loadStoredItems())
const initialized = ref(false)
const unlisteners: UnlistenFn[] = []
const workflowNamesByExecutionId = new Map<string, string>()
const desktopPermission = ref<'default' | 'granted' | 'denied' | 'unsupported'>(getDesktopPermissionState())
let activeRouter: Router | null = null
let desktopPermissionRequested = false

function loadStoredItems(): AppNotificationItem[] {
  if (typeof window === 'undefined') return []

  try {
    const raw = window.localStorage.getItem(NOTIFICATION_STORAGE_KEY)
    if (!raw) return []

    const parsed = JSON.parse(raw)
    if (!Array.isArray(parsed)) return []

    return parsed
      .filter((item): item is AppNotificationItem => Boolean(item && typeof item === 'object'))
      .map((item) => ({
        ...item,
        route: item.route || null,
        metadata: item.metadata || {},
        read: Boolean(item.read),
      }))
      .sort((left, right) => new Date(right.createdAt).getTime() - new Date(left.createdAt).getTime())
      .slice(0, MAX_ITEMS)
  } catch (error) {
    console.warn('[useNotificationCenter] Failed to load stored items:', error)
    return []
  }
}

function persistItems() {
  if (typeof window === 'undefined') return
  persistNotificationItems(window.localStorage, items.value)
}

function getTranslator() {
  return i18n.global.t.bind(i18n.global)
}

function shortenId(value: string, head = 8) {
  const normalized = String(value || '').trim()
  return normalized.length > head ? normalized.slice(0, head) : normalized
}

function toPlainText(value: unknown, fallback = '') {
  const text = String(value ?? fallback)
    .replace(/\s+/g, ' ')
    .replace(/<[^>]+>/g, ' ')
    .trim()
  return text || fallback
}

function truncateText(value: string, max = 120) {
  const normalized = toPlainText(value)
  if (normalized.length <= max) return normalized
  return `${normalized.slice(0, max - 1)}...`
}

function canUseDesktopNotification() {
  return typeof window !== 'undefined' && typeof Notification !== 'undefined'
}

function getDesktopPermissionState() {
  if (!canUseDesktopNotification()) {
    return 'unsupported' as const
  }
  return Notification.permission
}

function refreshDesktopPermissionState() {
  desktopPermission.value = getDesktopPermissionState()
  return desktopPermission.value
}

function shouldShowDesktopNotification() {
  if (!canUseDesktopNotification()) return false
  if (!preferences.value.desktopEnabled) return false
  if (preferences.value.desktopMode === 'always') return true
  return document.hidden || !document.hasFocus()
}

async function ensureDesktopNotificationPermission() {
  if (!canUseDesktopNotification()) return false

  if (refreshDesktopPermissionState() === 'granted') {
    return true
  }

  if (desktopPermission.value === 'denied') {
    return false
  }

  if (desktopPermissionRequested) {
    return false
  }

  desktopPermissionRequested = true

  try {
    const permission = await Notification.requestPermission()
    desktopPermission.value = permission
    return permission === 'granted'
  } catch (error) {
    console.warn('[useNotificationCenter] Failed to request desktop notification permission:', error)
    refreshDesktopPermissionState()
    return false
  }
}

function pushNotification(input: PushNotificationInput) {
  if (!isSourceEnabled(input.source)) {
    return null
  }

  if (input.eventKey) {
    const existing = items.value.find((item) => item.eventKey === input.eventKey)
    if (existing) return existing
  }

  const notification: AppNotificationItem = {
    id: input.id || crypto.randomUUID(),
    eventKey: input.eventKey,
    category: input.category,
    source: input.source,
    level: input.level,
    title: input.title,
    message: input.message,
    icon: input.icon,
    route: input.route || null,
    metadata: input.metadata || {},
    read: false,
    createdAt: input.createdAt || new Date().toISOString(),
  }

  items.value = [notification, ...items.value].slice(0, MAX_ITEMS)
  persistItems()
  showToastForNotification(notification)
  void playNotificationSound(notification.source)
  void showDesktopNotification(notification)
  return notification
}

function showToastForNotification(item: AppNotificationItem) {
  const text = `${item.title}${item.message ? `: ${truncateText(item.message, 80)}` : ''}`

  switch (item.level) {
    case 'success':
      toast.success(text, 2400)
      break
    case 'warning':
      toast.warning(text, 2600)
      break
    case 'error':
      toast.error(text, 3200)
      break
    default:
      toast.info(text, 2200)
      break
  }
}

async function focusAppWindowAndOpen(item: AppNotificationItem) {
  try {
    const currentWindow = getCurrentWindow()
    await currentWindow.show()
    await currentWindow.setFocus()
  } catch (error) {
    console.warn('[useNotificationCenter] Failed to focus app window from desktop notification:', error)
  }

  if (!activeRouter) return
  await openNotification(activeRouter, item)
}

async function showDesktopNotification(item: AppNotificationItem) {
  if (!shouldShowDesktopNotification()) return

  const permissionGranted = await ensureDesktopNotificationPermission()
  if (!permissionGranted) return

  await openDesktopNotification(
    item.title,
    truncateText(item.message, 120),
    item.eventKey || item.id,
    () => {
      void focusAppWindowAndOpen(item)
    },
  )
}

async function openDesktopNotification(
  title: string,
  body: string,
  tag: string,
  onClick?: () => void,
) {
  try {
    const desktopNotification = new Notification(title, {
      body,
      tag,
    })

    desktopNotification.onclick = () => {
      desktopNotification.close()
      onClick?.()
    }

    window.setTimeout(() => desktopNotification.close(), 10_000)
  } catch (error) {
    console.warn('[useNotificationCenter] Failed to show desktop notification:', error)
  }
}

async function playNotificationSound(source: NotificationSource) {
  if (!preferences.value.soundEnabled || !isSourceEnabled(source)) return
  if (typeof window === 'undefined') return

  const AudioContextCtor = window.AudioContext || (window as typeof window & {
    webkitAudioContext?: typeof AudioContext
  }).webkitAudioContext

  if (!AudioContextCtor) return

  try {
    const audioContext = new AudioContextCtor()
    const oscillator = audioContext.createOscillator()
    const gainNode = audioContext.createGain()

    oscillator.type = 'sine'
    oscillator.frequency.value = source === 'ai_assistant' ? 660 : 520
    gainNode.gain.setValueAtTime(0.0001, audioContext.currentTime)
    gainNode.gain.exponentialRampToValueAtTime(0.05, audioContext.currentTime + 0.01)
    gainNode.gain.exponentialRampToValueAtTime(0.0001, audioContext.currentTime + 0.18)

    oscillator.connect(gainNode)
    gainNode.connect(audioContext.destination)

    oscillator.start()
    oscillator.stop(audioContext.currentTime + 0.2)

    window.setTimeout(() => {
      void audioContext.close().catch(() => {})
    }, 300)
  } catch (error) {
    console.warn('[useNotificationCenter] Failed to play notification sound:', error)
  }
}

async function requestDesktopPermission() {
  if (!canUseDesktopNotification()) {
    refreshDesktopPermissionState()
    return false
  }

  try {
    const permission = await Notification.requestPermission()
    desktopPermissionRequested = permission !== 'default'
    desktopPermission.value = permission
    return permission === 'granted'
  } catch (error) {
    console.warn('[useNotificationCenter] Failed to request desktop permission manually:', error)
    refreshDesktopPermissionState()
    return false
  }
}

async function testDesktopNotification() {
  const t = getTranslator()
  const granted = await requestDesktopPermission()
  if (!granted) {
    return false
  }

  await openDesktopNotification(
    t('notifications.center.testDesktopTitle'),
    t('notifications.center.testDesktopMessage'),
    `notification-test:${Date.now()}`,
    () => {
      void focusAppWindowAndOpen({
        id: `notification-test:${Date.now()}`,
        category: 'notification',
        source: 'workflow',
        level: 'info',
        title: t('notifications.center.testDesktopTitle'),
        message: t('notifications.center.testDesktopMessage'),
        icon: 'fas fa-bell',
        read: true,
        createdAt: new Date().toISOString(),
        route: {
          path: '/notification-center',
        },
      })
    },
  )

  await playNotificationSound('workflow')
  return true
}

function markAsRead(id: string) {
  const target = items.value.find((item) => item.id === id)
  if (!target || target.read) return
  target.read = true
  persistItems()
}

function markAllAsRead(category?: NotificationCategory) {
  let changed = false
  items.value = items.value.map((item) => {
    if (category && item.category !== category) return item
    if (item.read) return item
    changed = true
    return {
      ...item,
      read: true,
    }
  })
  if (changed) {
    persistItems()
  }
}

function removeNotification(id: string) {
  const nextItems = items.value.filter((item) => item.id !== id)
  if (nextItems.length === items.value.length) return
  items.value = nextItems
  persistItems()
}

function clearCategory(category: NotificationCategory) {
  items.value = items.value.filter((item) => item.category !== category)
  persistItems()
}

function normalizeRoute(route?: NotificationRouteTarget | null) {
  if (!route?.path) return null
  return {
    path: route.path,
    query: route.query || undefined,
  }
}

async function openNotification(router: Router, item: AppNotificationItem) {
  markAsRead(item.id)

  const route = normalizeRoute(item.route)
  if (!route) return

  try {
    await router.push(route)
  } catch (error) {
    console.warn('[useNotificationCenter] Failed to navigate from notification:', error)
  }
}

function formatWorkflowNotification(payload: WorkflowRunCompletePayload): PushNotificationInput {
  const t = getTranslator()
  const executionId = String(payload.execution_id || '').trim()
  const workflowName = workflowNamesByExecutionId.get(executionId)
    || t('notifications.center.workflowFallbackName', { id: shortenId(executionId) })
  const errors = Array.isArray(payload.errors) ? payload.errors.length : 0

  let message = t('notifications.center.workflowCompletedMessage', { name: workflowName })
  if (errors > 0) {
    message = t('notifications.center.workflowCompletedWithErrors', {
      name: workflowName,
      count: errors,
    })
  }

  return {
    eventKey: `workflow:${executionId}:${payload.status || 'completed'}`,
    category: 'notification',
    source: 'workflow',
    level: errors > 0 ? 'warning' : 'success',
    title: t('notifications.center.workflowTitle'),
    message,
    icon: 'fas fa-circle-check',
    route: {
      path: '/workflow-studio',
      query: { execution_id: executionId },
    },
    metadata: {
      execution_id: executionId,
      status: payload.status || 'completed',
      errors,
    },
  }
}

function formatWorkflowResultSummaryNotification(payload: WorkflowResultSummaryPayload): PushNotificationInput {
  const t = getTranslator()
  const executionId = String(payload.execution_id || '').trim()
  const workflowName = String(payload.workflow_name || '').trim()
    || workflowNamesByExecutionId.get(executionId)
    || t('notifications.center.workflowFallbackName', { id: shortenId(executionId) })
  const findingsCount = Number(payload.findings_count || 0)
  const assetCount = Number(payload.asset_count || 0)
  const errorsCount = Number(payload.errors_count || 0)

  let message = t('notifications.center.workflowCompletedMessage', { name: workflowName })
  if (findingsCount > 0 && assetCount > 0) {
    message = t('notifications.center.workflowCompletedWithAssetsAndFindings', {
      name: workflowName,
      assets: assetCount,
      findings: findingsCount,
    })
  } else if (findingsCount > 0) {
    message = t('notifications.center.workflowCompletedWithFindings', {
      name: workflowName,
      findings: findingsCount,
    })
  } else if (assetCount > 0) {
    message = t('notifications.center.workflowCompletedWithAssets', {
      name: workflowName,
      assets: assetCount,
    })
  } else if (errorsCount > 0) {
    message = t('notifications.center.workflowCompletedWithErrors', {
      name: workflowName,
      count: errorsCount,
    })
  }

  return {
    eventKey: `workflow:${executionId}:${payload.status || 'completed'}`,
    category: 'notification',
    source: 'bug_bounty_workflow',
    level: errorsCount > 0 ? 'warning' : 'success',
    title: t('notifications.center.workflowAutomationTitle'),
    message,
    icon: 'fas fa-diagram-project',
    route: {
      path: '/bug-bounty',
      query: { tab: findingsCount > 0 ? 'findings' : assetCount > 0 ? 'assets' : 'workflows' },
    },
    metadata: {
      execution_id: executionId,
      workflow_name: workflowName,
      status: payload.status || 'completed',
      findings: findingsCount,
      assets: assetCount,
      errors: errorsCount,
    },
  }
}

function formatMonitorNotification(payload: MonitorTaskSummaryPayload): PushNotificationInput {
  const t = getTranslator()
  const taskId = String(payload.task_id || '').trim()
  const taskName = String(payload.task_name || '').trim() || t('notifications.center.monitorFallbackTask')
  const status = String(payload.status || 'completed').trim()
  const importedAssets = Number(payload.imported_assets || 0)
  const findingsCreated = Number(payload.findings_created || 0)
  const findingsUpdated = Number(payload.findings_updated || 0)

  let message = t('notifications.center.monitorCompletedNoChanges', { name: taskName })
  if (findingsCreated > 0 && importedAssets > 0) {
    message = t('notifications.center.monitorCompletedWithAssetsAndFindings', {
      name: taskName,
      assets: importedAssets,
      findings: findingsCreated,
    })
  } else if (findingsCreated > 0) {
    message = t('notifications.center.monitorCompletedWithFindings', {
      name: taskName,
      findings: findingsCreated,
    })
  } else if (importedAssets > 0) {
    message = t('notifications.center.monitorCompletedWithAssets', {
      name: taskName,
      assets: importedAssets,
    })
  }

  if (status === 'stopped') {
    message = t('notifications.center.monitorStopped', { name: taskName })
  } else if (status === 'failed') {
    message = String(payload.message || '').trim()
      || t('notifications.center.monitorFailed', { name: taskName })
  }

  return {
    eventKey: `monitor:${taskId}:${payload.started_at || payload.completed_at || status}`,
    category: 'notification',
    source: 'monitor',
    level: status === 'failed' ? 'error' : status === 'stopped' ? 'warning' : 'success',
    title: status === 'failed'
      ? t('notifications.center.monitorFailureTitle')
      : t('notifications.center.monitorTitle'),
    message,
    icon: status === 'failed' ? 'fas fa-triangle-exclamation' : findingsCreated > 0 ? 'fas fa-bug' : 'fas fa-radar',
    route: {
      path: '/bug-bounty',
      query: {
        tab: findingsCreated > 0 ? 'findings' : importedAssets > 0 ? 'assets' : 'monitor',
      },
    },
    metadata: {
      task_id: taskId,
      status,
      imported_assets: importedAssets,
      findings_created: findingsCreated,
      findings_updated: findingsUpdated,
    },
  }
}

function formatMonitorPluginFailureNotification(payload: MonitorPluginFailurePayload): PushNotificationInput {
  const t = getTranslator()
  const taskId = String(payload.task_id || '').trim()
  const taskName = String(payload.task_name || '').trim() || t('notifications.center.monitorFallbackTask')
  const pluginLabel = String(payload.plugin_label || payload.plugin_id || '').trim() || 'plugin'
  const error = truncateText(
    toPlainText(payload.error || t('notifications.center.monitorFailed', { name: taskName })),
    200,
  )

  return {
    eventKey: `monitor-plugin:${taskId}:${pluginLabel}:${payload.started_at || payload.created_at || error}`,
    category: 'notification',
    source: 'monitor',
    level: 'error',
    title: t('notifications.center.monitorFailureTitle'),
    message: t('notifications.center.monitorPluginFailed', {
      task: taskName,
      plugin: pluginLabel,
      reason: error,
    }),
    icon: 'fas fa-triangle-exclamation',
    route: {
      path: '/bug-bounty',
      query: {
        tab: 'monitor',
      },
    },
    metadata: {
      task_id: taskId,
      plugin_id: String(payload.plugin_id || '').trim(),
      plugin_label: pluginLabel,
      error,
      execution_mode: String(payload.execution_mode || '').trim(),
    },
  }
}

function formatAssistantNotification(payload: AgentExecutionFinishedPayload): PushNotificationInput {
  const t = getTranslator()
  const conversationId = String(payload.execution_id || '').trim()
  const outcome = payload.outcome || (payload.success ? 'succeeded' : 'failed')
  const preview = truncateText(
    toPlainText(payload.response || payload.message || payload.error || ''),
    120,
  )

  if (outcome === 'failed' || outcome === 'cancelled') {
    return {
      eventKey: `assistant:${conversationId}:${outcome}`,
      category: 'message',
      source: 'ai_assistant',
      level: outcome === 'failed' ? 'error' : 'warning',
      title: t('notifications.center.aiAssistantFailedTitle'),
      message: preview || t('notifications.center.aiAssistantFailedMessage', { id: shortenId(conversationId) }),
      icon: 'fas fa-robot',
      route: {
        path: '/ai-assistant',
        query: { conversationId },
      },
      metadata: {
        conversation_id: conversationId,
        outcome,
      },
    }
  }

  return {
    eventKey: `assistant:${conversationId}:succeeded`,
    category: 'message',
    source: 'ai_assistant',
    level: 'info',
    title: t('notifications.center.aiAssistantTitle'),
    message: preview || t('notifications.center.aiAssistantCompletedMessage', { id: shortenId(conversationId) }),
    icon: 'fas fa-message',
    route: {
      path: '/ai-assistant',
      query: { conversationId },
    },
    metadata: {
      conversation_id: conversationId,
      outcome,
    },
  }
}

async function initializeNotificationCenter(router?: Router) {
  if (router) {
    activeRouter = router
  }

  if (initialized.value) return
  initialized.value = true

  unlisteners.push(
    await listen<WorkflowRunStartPayload>('workflow:run-start', (event) => {
      const executionId = String(event.payload?.execution_id || '').trim()
      const workflowName = String(event.payload?.workflow_name || '').trim()
      if (!executionId || !workflowName) return
      workflowNamesByExecutionId.set(executionId, workflowName)
    }),
  )

  unlisteners.push(
    await listen<WorkflowResultSummaryPayload>('workflow:result-summary', (event) => {
      const executionId = String(event.payload?.execution_id || '').trim()
      if (!executionId) return
      pushNotification(formatWorkflowResultSummaryNotification(event.payload || {}))
    }),
  )

  unlisteners.push(
    await listen<WorkflowRunCompletePayload>('workflow:run-complete', (event) => {
      const executionId = String(event.payload?.execution_id || '').trim()
      if (!executionId) return
      pushNotification(formatWorkflowNotification(event.payload || {}))
    }),
  )

  unlisteners.push(
    await listen<MonitorTaskSummaryPayload>('monitor:task-summary', (event) => {
      const taskId = String(event.payload?.task_id || '').trim()
      if (!taskId) return
      pushNotification(formatMonitorNotification(event.payload || {}))
    }),
  )

  unlisteners.push(
    await listen<MonitorPluginFailurePayload>('monitor:plugin-failed', (event) => {
      const taskId = String(event.payload?.task_id || '').trim()
      const pluginId = String(event.payload?.plugin_id || '').trim()
      if (!taskId || !pluginId) return
      pushNotification(formatMonitorPluginFailureNotification(event.payload || {}))
    }),
  )

  unlisteners.push(
    await listen<AgentExecutionFinishedPayload>('agent:execution_finished', (event) => {
      const executionId = String(event.payload?.execution_id || '').trim()
      if (!executionId) return
      pushNotification(formatAssistantNotification(event.payload || {}))
    }),
  )
}

const sortedItems = computed(() => {
  return [...items.value].sort((left, right) => new Date(right.createdAt).getTime() - new Date(left.createdAt).getTime())
})

const messageItems = computed(() => sortedItems.value.filter((item) => item.category === 'message'))
const notificationItems = computed(() => sortedItems.value.filter((item) => item.category === 'notification'))
const unreadMessageCount = computed(() => messageItems.value.filter((item) => !item.read).length)
const unreadNotificationCount = computed(() => notificationItems.value.filter((item) => !item.read).length)

export function useNotificationCenter() {
  return {
    initializeNotificationCenter,
    desktopPermission,
    refreshDesktopPermissionState,
    requestDesktopPermission,
    testDesktopNotification,
    items: sortedItems,
    messageItems,
    notificationItems,
    unreadMessageCount,
    unreadNotificationCount,
    pushNotification,
    markAsRead,
    markAllAsRead,
    removeNotification,
    clearCategory,
    openNotification,
  }
}
