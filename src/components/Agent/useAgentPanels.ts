import { computed, ref, watch, type ComputedRef, type Ref } from 'vue'
import type { AgentTask } from '@/types/agentTask'
import type { ParallelTaskSource } from '@/composables/useAgentParallelRunState'

export type RightPanelKey =
  | 'tasks'
  | 'html'
  | 'terminal'
  | 'browser-shell'
  | 'team'
  | 'harness'
  | 'work-config'
  | 'workspace-files'

interface TaskSourceOption {
  key: string
  label: string
  count: number
}

interface ScopedTaskEntry {
  executionId: string
  tasks: AgentTask[]
  updatedAt: number
}

interface ParallelTaskBucket {
  key: string
  label: string
  tasks: AgentTask[]
  updatedAt: number
}

const TASK_SOURCE_ALL_KEY = '__all__'
const SIDEBAR_MIN_WIDTH = 300
const SIDEBAR_MAX_WIDTH = 800
const SIDEBAR_DEFAULT_WIDTH = 350
const SIDEBAR_WIDTH_STORAGE_KEY = 'sentinel:sidebar:width'
const TOOL_CONFIG_DRAWER_MIN_WIDTH = 360
const TOOL_CONFIG_DRAWER_MAX_WIDTH = 720
const TOOL_CONFIG_DRAWER_DEFAULT_WIDTH = 420
const TOOL_CONFIG_DRAWER_WIDTH_STORAGE_KEY = 'sentinel:tool-config-drawer:width'

export const useAgentPanels = (params: {
  agentError: ComputedRef<string | null | undefined>
  clearTasksForExecution: (executionId: string) => void
  conversationId: Ref<string | null>
  getConversationIdForExecution: (executionId: string) => string | undefined
  getTasksForExecution: (executionId: string) => AgentTask[]
  isTeamWorkspaceActive: Ref<boolean>
  isTaskPanelActive: ComputedRef<boolean>
  localError: Ref<string | null>
  propsShowTasks: boolean
  parallelTaskSources?: ComputedRef<ParallelTaskSource[]>
  pruneTasksForExecutionAfter: (executionId: string, timestampMs: number) => Promise<AgentTask[]>
  resetAgentError: () => void
  setTasksForExecution: (executionId: string, tasks: AgentTask[]) => void
  terminalClose: () => void
  terminalHasHistory: ComputedRef<boolean>
  terminalIsActive: ComputedRef<boolean>
  terminalOpen: () => void
  tasksClose: () => void
  tasksByExecutionId: ComputedRef<Record<string, AgentTask[]>>
  taskExecutionIds: ComputedRef<string[]>
  tasksOpen: () => void
}): {
  activeRightPanel: Ref<RightPanelKey | null>
  activateRightPanel: (panel: RightPanelKey) => void
  clearError: () => void
  clearTasksForCurrentContext: () => void
  deactivateRightPanel: (panel: RightPanelKey) => void
  error: ComputedRef<string | null>
  handleCloseHtmlPanel: () => void
  handleCloseBrowserShell: () => void
  handleCloseHarness: () => void
  handleCloseTasks: () => void
  handleCloseTerminal: () => void
  handleRenderHtml: (htmlContent: string) => void
  handleToggleBrowserShell: () => void
  handleToggleHarness: () => void
  handleTaskSourceChange: (sourceKey: string) => void
  handleToggleHtmlPanel: () => void
  handleToggleTasks: () => void
  handleToggleTerminal: () => void
  hasHtmlPanelContent: ComputedRef<boolean>
  hasTerminalHistory: ComputedRef<boolean>
  htmlPanelContent: Ref<string>
  loadSidebarWidth: () => void
  loadToolConfigDrawerWidth: () => void
  pruneTasksForCurrentContextAfter: (timestampMs: number) => Promise<void>
  selectedTaskSourceKey: Ref<string>
  sidebarWidth: Ref<number>
  startToolConfigDrawerResize: (event: MouseEvent) => void
  startResize: (event: MouseEvent) => void
  taskBadgeCount: ComputedRef<number>
  taskSourceOptions: ComputedRef<TaskSourceOption[]>
  tasks: ComputedRef<AgentTask[]>
  toolConfigDrawerWidth: Ref<number>
} => {
  const selectedTaskSourceKey = ref<string>(TASK_SOURCE_ALL_KEY)
  const isHtmlPanelActive = ref(false)
  const htmlPanelContent = ref('')
  const activeRightPanel = ref<RightPanelKey | null>(null)
  const sidebarWidth = ref(SIDEBAR_DEFAULT_WIDTH)
  const toolConfigDrawerWidth = ref(TOOL_CONFIG_DRAWER_DEFAULT_WIDTH)
  const isResizing = ref(false)
  let isSyncingRightPanel = false

  const isTaskExecutionInCurrentContext = (executionId: string) => {
    const convId = params.conversationId.value
    if (convId && executionId === convId) return true
    if (convId && params.getConversationIdForExecution(executionId) === convId) return true
    if (convId && (params.parallelTaskSources?.value || []).some((source) =>
      source.parentConversationId === convId && source.executionId === executionId,
    )) return true
    return false
  }

  const scopedTaskEntries = computed<ScopedTaskEntry[]>(() => {
    const entries = Object.entries(params.tasksByExecutionId.value)
      .filter(([executionId]) => isTaskExecutionInCurrentContext(executionId))
      .map(([executionId, list]) => ({
        executionId,
        tasks: list,
        updatedAt: list.reduce((latest, task) => Math.max(latest, Number(task.updated_at || 0)), 0),
      }))
    return entries.sort((a, b) => b.updatedAt - a.updatedAt)
  })

  const buildLabeledTasks = (tasks: AgentTask[], label: string): AgentTask[] => {
    return tasks.map((task) => ({
      ...task,
      title: `[${label}] ${task.title}`,
      active_title: task.active_title ? `[${label}] ${task.active_title}` : task.active_title,
      metadata: {
        ...task.metadata,
        source_label: label,
      },
    }))
  }

  const parallelTaskBuckets = computed<ParallelTaskBucket[]>(() => {
    const convId = params.conversationId.value
    if (!convId) return []
    return (params.parallelTaskSources?.value || [])
      .filter((source) => source.parentConversationId === convId)
      .map((source) => {
        const sourceTasks = params.getTasksForExecution(source.executionId)
        return {
          key: `parallel:${source.executionId}`,
          label: source.label,
          tasks: sourceTasks,
          updatedAt: sourceTasks.reduce((latest, task) => Math.max(latest, Number(task.updated_at || 0)), 0),
        }
      })
      .filter((bucket) => bucket.tasks.length > 0)
      .sort((a, b) => b.updatedAt - a.updatedAt)
  })

  const conversationTaskBuckets = computed<ParallelTaskBucket[]>(() => {
    const convId = params.conversationId.value
    if (!convId) return []
    const parallelExecutionIds = new Set(
      (params.parallelTaskSources?.value || [])
        .filter((source) => source.parentConversationId === convId)
        .map((source) => source.executionId),
    )
    return scopedTaskEntries.value
      .filter((entry) => {
        if (parallelExecutionIds.has(entry.executionId)) return false
        if (entry.executionId === convId) return true
        return params.getConversationIdForExecution(entry.executionId) === convId
      })
      .map((entry, index) => ({
        key: `execution:${entry.executionId}`,
        label: index === 0 ? '最新执行' : `执行 ${index + 1}`,
        tasks: entry.tasks,
        updatedAt: entry.updatedAt,
      }))
  })

  const taskSourceOptions = computed<TaskSourceOption[]>(() => {
    const isParallelTaskSource = parallelTaskBuckets.value.length > 0
    const buckets = isParallelTaskSource ? parallelTaskBuckets.value : conversationTaskBuckets.value
    if (buckets.length === 0) return []
    const allCount = buckets.reduce((acc, bucket) => acc + bucket.tasks.length, 0)
    return [
      {
        key: TASK_SOURCE_ALL_KEY,
        label: isParallelTaskSource ? '全部模型' : '全部执行',
        count: allCount,
      },
      ...buckets.map((bucket) => ({
        key: bucket.key,
        label: bucket.label,
        count: bucket.tasks.length,
      })),
    ]
  })

  const parallelTasks = computed<AgentTask[]>(() => {
    if (parallelTaskBuckets.value.length === 0) return []
    const selected = selectedTaskSourceKey.value || TASK_SOURCE_ALL_KEY
    if (selected !== TASK_SOURCE_ALL_KEY) {
      return parallelTaskBuckets.value.find((bucket) => bucket.key === selected)?.tasks || []
    }
    if (parallelTaskBuckets.value.length === 1) {
      return [...parallelTaskBuckets.value[0].tasks]
    }
    return parallelTaskBuckets.value
      .flatMap((bucket) => buildLabeledTasks(bucket.tasks, bucket.label))
      .sort((a, b) => Number(b.updated_at || 0) - Number(a.updated_at || 0))
  })

  const conversationTasks = computed<AgentTask[]>(() => {
    if (conversationTaskBuckets.value.length === 0) return []
    const selected = selectedTaskSourceKey.value || TASK_SOURCE_ALL_KEY
    if (selected !== TASK_SOURCE_ALL_KEY) {
      return conversationTaskBuckets.value.find((bucket) => bucket.key === selected)?.tasks || []
    }
    if (conversationTaskBuckets.value.length === 1) {
      return [...conversationTaskBuckets.value[0].tasks]
    }
    return conversationTaskBuckets.value
      .flatMap((bucket) => buildLabeledTasks(bucket.tasks, bucket.label))
      .sort((a, b) => Number(b.updated_at || 0) - Number(a.updated_at || 0))
  })

  const tasks = computed<AgentTask[]>(() => {
    if (parallelTaskBuckets.value.length > 0) return parallelTasks.value
    return conversationTasks.value
  })

  const taskBadgeCount = computed(() => tasks.value.filter((item) => !item.metadata?.parent_id).length)
  const isTerminalActive = computed(() => params.terminalIsActive.value)
  const hasTerminalHistory = computed(() => params.terminalHasHistory.value)

  const closeRightPanelByKey = (panel: RightPanelKey) => {
    if (panel === 'tasks') {
      params.tasksClose()
      return
    }
    if (panel === 'html') {
      isHtmlPanelActive.value = false
      return
    }
    if (panel === 'terminal') {
      params.terminalClose()
      return
    }
    if (panel === 'browser-shell') {
      return
    }
    if (panel === 'work-config') {
      return
    }
    if (panel === 'harness') {
      return
    }
    if (panel === 'workspace-files') {
      return
    }
    params.isTeamWorkspaceActive.value = false
  }

  const closeOtherRightPanels = (activePanel: RightPanelKey) => {
    if (activePanel !== 'tasks') params.tasksClose()
    if (activePanel !== 'html') isHtmlPanelActive.value = false
    if (activePanel !== 'terminal') params.terminalClose()
    if (activePanel !== 'browser-shell') {
      // browser-shell panel is driven only by activeRightPanel
    }
    if (activePanel !== 'work-config') {
      // work-config panel is driven only by activeRightPanel
    }
    if (activePanel !== 'harness') {
      // harness panel is driven only by activeRightPanel
    }
    if (activePanel !== 'team') params.isTeamWorkspaceActive.value = false
  }

  const syncRightPanelState = (panel: RightPanelKey, isActive: boolean) => {
    if (isSyncingRightPanel) return
    isSyncingRightPanel = true
    try {
      if (!isActive) {
        if (activeRightPanel.value === panel) {
          activeRightPanel.value = null
        }
        return
      }
      activeRightPanel.value = panel
      closeOtherRightPanels(panel)
    } finally {
      isSyncingRightPanel = false
    }
  }

  watch(params.isTaskPanelActive, (active) => {
    syncRightPanelState('tasks', active)
  }, { immediate: true })

  watch(isHtmlPanelActive, (active) => {
    syncRightPanelState('html', active)
  }, { immediate: true })

  watch(isTerminalActive, (active) => {
    syncRightPanelState('terminal', active)
  }, { immediate: true })

  watch(params.isTeamWorkspaceActive, (active) => {
    syncRightPanelState('team', active)
  }, { immediate: true })

  const activateRightPanel = (panel: RightPanelKey) => {
    activeRightPanel.value = panel
    closeOtherRightPanels(panel)
  }

  const deactivateRightPanel = (panel: RightPanelKey) => {
    if (activeRightPanel.value === panel) {
      activeRightPanel.value = null
    }
    closeRightPanelByKey(panel)
  }

  const handleRenderHtml = (htmlContent: string) => {
    htmlPanelContent.value = htmlContent
    activateRightPanel('html')
    isHtmlPanelActive.value = true
  }

  const handleCloseTasks = () => deactivateRightPanel('tasks')
  const handleCloseHtmlPanel = () => deactivateRightPanel('html')
  const handleCloseTerminal = () => deactivateRightPanel('terminal')
  const handleCloseBrowserShell = () => deactivateRightPanel('browser-shell')
  const handleCloseHarness = () => deactivateRightPanel('harness')

  const handleToggleTasks = () => {
    if (activeRightPanel.value === 'tasks') {
      deactivateRightPanel('tasks')
      return
    }
    activateRightPanel('tasks')
    params.tasksOpen()
  }

  const handleToggleHtmlPanel = () => {
    if (activeRightPanel.value === 'html') {
      deactivateRightPanel('html')
      return
    }
    activateRightPanel('html')
    isHtmlPanelActive.value = true
  }

  const handleToggleTerminal = () => {
    if (activeRightPanel.value === 'terminal') {
      deactivateRightPanel('terminal')
      return
    }
    activateRightPanel('terminal')
    params.terminalOpen()
  }

  const handleToggleBrowserShell = () => {
    if (activeRightPanel.value === 'browser-shell') {
      deactivateRightPanel('browser-shell')
      return
    }
    activateRightPanel('browser-shell')
  }

  const handleToggleHarness = () => {
    if (activeRightPanel.value === 'harness') {
      deactivateRightPanel('harness')
      return
    }
    activateRightPanel('harness')
  }

  const clampWidth = (width: number, minWidth: number, maxWidth: number) => {
    return Math.max(minWidth, Math.min(maxWidth, width))
  }

  const getToolConfigDrawerMaxWidth = () => {
    if (typeof window === 'undefined') return TOOL_CONFIG_DRAWER_MAX_WIDTH
    return Math.max(
      TOOL_CONFIG_DRAWER_MIN_WIDTH,
      Math.min(TOOL_CONFIG_DRAWER_MAX_WIDTH, window.innerWidth - 32),
    )
  }

  const loadStoredWidth = (
    storageKey: string,
    applyWidth: (width: number) => void,
    isAllowedWidth: (width: number) => boolean,
  ) => {
    try {
      const saved = localStorage.getItem(storageKey)
      if (saved) {
        const width = parseInt(saved, 10)
        if (isAllowedWidth(width)) {
          applyWidth(width)
        }
      }
    } catch (e) {
      console.warn(`[AgentView] Failed to load width from ${storageKey}:`, e)
    }
  }

  const saveStoredWidth = (storageKey: string, width: number) => {
    try {
      localStorage.setItem(storageKey, width.toString())
    } catch (e) {
      console.warn(`[AgentView] Failed to save width to ${storageKey}:`, e)
    }
  }

  const applySidebarWidth = (width: number) => {
    sidebarWidth.value = clampWidth(width, SIDEBAR_MIN_WIDTH, SIDEBAR_MAX_WIDTH)
  }

  const applyToolConfigDrawerWidth = (width: number) => {
    toolConfigDrawerWidth.value = clampWidth(
      width,
      TOOL_CONFIG_DRAWER_MIN_WIDTH,
      getToolConfigDrawerMaxWidth(),
    )
  }

  const loadSidebarWidth = () => {
    loadStoredWidth(
      SIDEBAR_WIDTH_STORAGE_KEY,
      applySidebarWidth,
      (width) => width >= SIDEBAR_MIN_WIDTH && width <= SIDEBAR_MAX_WIDTH,
    )
  }

  const loadToolConfigDrawerWidth = () => {
    loadStoredWidth(
      TOOL_CONFIG_DRAWER_WIDTH_STORAGE_KEY,
      applyToolConfigDrawerWidth,
      (width) => width >= TOOL_CONFIG_DRAWER_MIN_WIDTH && width <= TOOL_CONFIG_DRAWER_MAX_WIDTH,
    )
  }

  const startHorizontalResize = (
    event: MouseEvent,
    options: {
      getWidth: () => number
      applyWidth: (width: number) => void
      persist: () => void
    },
  ) => {
    event.preventDefault()
    isResizing.value = true
    const startX = event.clientX
    const startWidth = options.getWidth()

    document.body.classList.add('resizing')
    document.body.style.cursor = 'col-resize'

    const onMouseMove = (moveEvent: MouseEvent) => {
      if (!isResizing.value) return
      const delta = startX - moveEvent.clientX
      options.applyWidth(startWidth + delta)
    }

    const onMouseUp = () => {
      if (isResizing.value) {
        isResizing.value = false
        options.persist()
      }
      document.body.classList.remove('resizing')
      document.body.style.cursor = ''
      document.removeEventListener('mousemove', onMouseMove)
      document.removeEventListener('mouseup', onMouseUp)
    }

    document.addEventListener('mousemove', onMouseMove)
    document.addEventListener('mouseup', onMouseUp)
  }

  const startResize = (event: MouseEvent) => {
    startHorizontalResize(event, {
      getWidth: () => sidebarWidth.value,
      applyWidth: applySidebarWidth,
      persist: () => saveStoredWidth(SIDEBAR_WIDTH_STORAGE_KEY, sidebarWidth.value),
    })
  }

  const startToolConfigDrawerResize = (event: MouseEvent) => {
    startHorizontalResize(event, {
      getWidth: () => toolConfigDrawerWidth.value,
      applyWidth: applyToolConfigDrawerWidth,
      persist: () => saveStoredWidth(TOOL_CONFIG_DRAWER_WIDTH_STORAGE_KEY, toolConfigDrawerWidth.value),
    })
  }

  const handleTaskSourceChange = (sourceKey: string) => {
    selectedTaskSourceKey.value = sourceKey || TASK_SOURCE_ALL_KEY
  }

  watch(taskSourceOptions, (options) => {
    if (options.length === 0) {
      selectedTaskSourceKey.value = TASK_SOURCE_ALL_KEY
      return
    }
    if (options.some((option) => option.key === selectedTaskSourceKey.value)) return
    selectedTaskSourceKey.value = TASK_SOURCE_ALL_KEY
  }, { immediate: true })

  const currentContextTaskExecutionIds = () => {
    const ids = new Set<string>()
    const convId = params.conversationId.value
    if (convId) {
      ids.add(convId)
      for (const executionId of params.taskExecutionIds.value) {
        if (params.getConversationIdForExecution(executionId) === convId) {
          ids.add(executionId)
        }
      }
      for (const source of params.parallelTaskSources?.value || []) {
        if (source.parentConversationId === convId) {
          ids.add(source.executionId)
        }
      }
    }
    return Array.from(ids)
  }

  const clearTasksForCurrentContext = () => {
    for (const executionId of currentContextTaskExecutionIds()) {
      params.clearTasksForExecution(executionId)
    }
  }

  const pruneTasksForCurrentContextAfter = async (timestampMs: number) => {
    for (const executionId of currentContextTaskExecutionIds()) {
      const remaining = await params.pruneTasksForExecutionAfter(executionId, timestampMs)
      if (remaining.length > 0) {
        params.setTasksForExecution(executionId, remaining)
      } else {
        params.clearTasksForExecution(executionId)
      }
    }
  }

  const error = computed(() => params.localError.value || params.agentError.value || null)

  const clearError = () => {
    params.localError.value = null
    params.resetAgentError()
  }

  const hasHtmlPanelContent = computed(() => !!htmlPanelContent.value)

  return {
    activeRightPanel,
    activateRightPanel,
    clearError,
    clearTasksForCurrentContext,
    deactivateRightPanel,
    error,
    handleCloseBrowserShell,
    handleCloseHarness,
    handleCloseHtmlPanel,
    handleCloseTasks,
    handleCloseTerminal,
    handleRenderHtml,
    handleToggleBrowserShell,
    handleToggleHarness,
    handleTaskSourceChange,
    handleToggleHtmlPanel,
    handleToggleTasks,
    handleToggleTerminal,
    hasHtmlPanelContent,
    hasTerminalHistory,
    htmlPanelContent,
    loadSidebarWidth,
    loadToolConfigDrawerWidth,
    pruneTasksForCurrentContextAfter,
    selectedTaskSourceKey,
    sidebarWidth,
    startToolConfigDrawerResize,
    startResize,
    taskBadgeCount,
    taskSourceOptions,
    tasks,
    toolConfigDrawerWidth,
  }
}
