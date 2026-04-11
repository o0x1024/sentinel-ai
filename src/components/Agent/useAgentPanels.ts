import { computed, ref, watch, type ComputedRef, type Ref } from 'vue'
import type { Todo } from '@/types/todo'

export type RightPanelKey = 'todos' | 'html' | 'terminal' | 'team'

interface TodoSourceOption {
  key: string
  label: string
  count: number
}

interface ScopedTodoEntry {
  executionId: string
  todos: Todo[]
  updatedAt: number
}

interface TeamTodoBucket {
  key: string
  label: string
  todos: Todo[]
  updatedAt: number
}

const TODO_SOURCE_ALL_KEY = '__all__'
const SIDEBAR_MIN_WIDTH = 300
const SIDEBAR_MAX_WIDTH = 800
const SIDEBAR_DEFAULT_WIDTH = 350

export const useAgentPanels = (params: {
  activeTeamSessionId: Ref<string | null>
  agentError: ComputedRef<string | null | undefined>
  clearTodosForExecution: (executionId: string) => void
  conversationId: Ref<string | null>
  getTodosForExecution: (executionId: string) => Todo[]
  isTeamWorkspaceActive: Ref<boolean>
  isTodosPanelActive: ComputedRef<boolean>
  localError: Ref<string | null>
  parseTeamTodoExecutionId: (executionId: string) => {
    sessionId: string
    taskId: string
    memberId?: string
  } | null
  propsShowTodos: boolean
  resetAgentError: () => void
  resolveAgentName: (agentId?: string | null) => string
  selectedTeamTaskAssigneeId: ComputedRef<string | null>
  teamWorkspaceAvailable: ComputedRef<boolean>
  terminalClose: () => void
  terminalHasHistory: ComputedRef<boolean>
  terminalIsActive: ComputedRef<boolean>
  terminalOpen: () => void
  todosClose: () => void
  todosByExecutionId: ComputedRef<Record<string, Todo[]>>
  todosExecutionIds: ComputedRef<string[]>
  todosOpen: () => void
}): {
  activeRightPanel: Ref<RightPanelKey | null>
  activateRightPanel: (panel: RightPanelKey) => void
  clearError: () => void
  clearTodosForCurrentContext: () => void
  deactivateRightPanel: (panel: RightPanelKey) => void
  error: ComputedRef<string | null>
  handleCloseHtmlPanel: () => void
  handleCloseTerminal: () => void
  handleCloseTodos: () => void
  handleRenderHtml: (htmlContent: string) => void
  handleTodoSourceChange: (sourceKey: string) => void
  handleToggleHtmlPanel: () => void
  handleToggleTerminal: () => void
  handleToggleTodos: () => void
  hasHtmlPanelContent: ComputedRef<boolean>
  hasTerminalHistory: ComputedRef<boolean>
  htmlPanelContent: Ref<string>
  loadSidebarWidth: () => void
  selectedTodoSourceKey: Ref<string>
  selectedTaskTodoSourceKey: ComputedRef<string>
  sidebarWidth: Ref<number>
  startResize: (event: MouseEvent) => void
  todoBadgeCount: ComputedRef<number>
  todoSourceOptions: ComputedRef<TodoSourceOption[]>
  todos: ComputedRef<Todo[]>
} => {
  const selectedTodoSourceKey = ref<string>(TODO_SOURCE_ALL_KEY)
  const isHtmlPanelActive = ref(false)
  const htmlPanelContent = ref('')
  const activeRightPanel = ref<RightPanelKey | null>(null)
  const sidebarWidth = ref(SIDEBAR_DEFAULT_WIDTH)
  const isResizing = ref(false)
  let isSyncingRightPanel = false

  const isTodoExecutionInCurrentContext = (executionId: string) => {
    const convId = params.conversationId.value
    if (convId && executionId === convId) return true
    const parsed = params.parseTeamTodoExecutionId(executionId)
    if (!parsed) return false
    return !!params.activeTeamSessionId.value && parsed.sessionId === params.activeTeamSessionId.value
  }

  const scopedTodoEntries = computed<ScopedTodoEntry[]>(() => {
    const entries = Object.entries(params.todosByExecutionId.value)
      .filter(([executionId]) => isTodoExecutionInCurrentContext(executionId))
      .map(([executionId, list]) => ({
        executionId,
        todos: list,
        updatedAt: list.reduce((latest, todo) => Math.max(latest, Number(todo.updated_at || 0)), 0),
      }))
    return entries.sort((a, b) => b.updatedAt - a.updatedAt)
  })

  const teamTodoBuckets = computed<TeamTodoBucket[]>(() => {
    if (!params.teamWorkspaceAvailable.value || !params.activeTeamSessionId.value) return []
    const bucketMap = new Map<string, TeamTodoBucket>()

    for (const entry of scopedTodoEntries.value) {
      const parsed = params.parseTeamTodoExecutionId(entry.executionId)
      if (!parsed || parsed.sessionId !== params.activeTeamSessionId.value) continue
      const sourceKey = parsed.memberId ? `member:${parsed.memberId}` : `execution:${entry.executionId}`
      const label = parsed.memberId
        ? params.resolveAgentName(parsed.memberId)
        : `task ${parsed.taskId}`
      const existing = bucketMap.get(sourceKey)
      if (existing) {
        existing.todos = [...existing.todos, ...entry.todos]
        existing.updatedAt = Math.max(existing.updatedAt, entry.updatedAt)
      } else {
        bucketMap.set(sourceKey, {
          key: sourceKey,
          label,
          todos: [...entry.todos],
          updatedAt: entry.updatedAt,
        })
      }
    }

    return [...bucketMap.values()].sort((a, b) => b.updatedAt - a.updatedAt)
  })

  const todoSourceOptions = computed<TodoSourceOption[]>(() => {
    if (!params.teamWorkspaceAvailable.value || teamTodoBuckets.value.length === 0) return []
    const allCount = teamTodoBuckets.value.reduce((acc, bucket) => acc + bucket.todos.length, 0)
    return [
      {
        key: TODO_SOURCE_ALL_KEY,
        label: '全局',
        count: allCount,
      },
      ...teamTodoBuckets.value.map((bucket) => ({
        key: bucket.key,
        label: bucket.label,
        count: bucket.todos.length,
      })),
    ]
  })

  const buildLabeledTodos = (todos: Todo[], label: string): Todo[] => {
    return todos.map((todo) => ({
      ...todo,
      content: `[${label}] ${todo.content}`,
      active_form: todo.active_form ? `[${label}] ${todo.active_form}` : todo.active_form,
    }))
  }

  const teamTodos = computed<Todo[]>(() => {
    if (teamTodoBuckets.value.length === 0) return []
    const selected = selectedTodoSourceKey.value || TODO_SOURCE_ALL_KEY
    if (selected !== TODO_SOURCE_ALL_KEY) {
      return teamTodoBuckets.value.find((bucket) => bucket.key === selected)?.todos || []
    }
    if (teamTodoBuckets.value.length === 1) {
      return [...teamTodoBuckets.value[0].todos]
    }
    return teamTodoBuckets.value
      .flatMap((bucket) => buildLabeledTodos(bucket.todos, bucket.label))
      .sort((a, b) => Number(b.updated_at || 0) - Number(a.updated_at || 0))
  })

  const conversationTodos = computed<Todo[]>(() => {
    const convId = params.conversationId.value
    if (!convId) return []
    return params.getTodosForExecution(convId)
  })

  const todos = computed<Todo[]>(() => {
    if (params.teamWorkspaceAvailable.value && params.activeTeamSessionId.value) {
      if (teamTodoBuckets.value.length > 0) return teamTodos.value
    }
    return conversationTodos.value
  })

  const todoBadgeCount = computed(() => todos.value.filter((item) => !item.metadata?.parent_id).length)
  const isTerminalActive = computed(() => params.terminalIsActive.value)
  const hasTerminalHistory = computed(() => params.terminalHasHistory.value)

  const closeRightPanelByKey = (panel: RightPanelKey) => {
    if (panel === 'todos') {
      params.todosClose()
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
    params.isTeamWorkspaceActive.value = false
  }

  const closeOtherRightPanels = (activePanel: RightPanelKey) => {
    if (activePanel !== 'todos') params.todosClose()
    if (activePanel !== 'html') isHtmlPanelActive.value = false
    if (activePanel !== 'terminal') params.terminalClose()
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
      if (activeRightPanel.value && activeRightPanel.value !== panel) {
        closeRightPanelByKey(panel)
        return
      }
      activeRightPanel.value = panel
      closeOtherRightPanels(panel)
    } finally {
      isSyncingRightPanel = false
    }
  }

  watch(params.isTodosPanelActive, (active) => {
    syncRightPanelState('todos', active)
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

  const handleCloseTodos = () => deactivateRightPanel('todos')
  const handleCloseHtmlPanel = () => deactivateRightPanel('html')
  const handleCloseTerminal = () => deactivateRightPanel('terminal')

  const handleToggleTodos = () => {
    if (activeRightPanel.value === 'todos') {
      deactivateRightPanel('todos')
      return
    }
    activateRightPanel('todos')
    params.todosOpen()
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

  const loadSidebarWidth = () => {
    try {
      const saved = localStorage.getItem('sentinel:sidebar:width')
      if (saved) {
        const width = parseInt(saved, 10)
        if (width >= SIDEBAR_MIN_WIDTH && width <= SIDEBAR_MAX_WIDTH) {
          sidebarWidth.value = width
        }
      }
    } catch (e) {
      console.warn('[AgentView] Failed to load sidebar width:', e)
    }
  }

  const saveSidebarWidth = (width: number) => {
    try {
      localStorage.setItem('sentinel:sidebar:width', width.toString())
    } catch (e) {
      console.warn('[AgentView] Failed to save sidebar width:', e)
    }
  }

  const startResize = (event: MouseEvent) => {
    event.preventDefault()
    isResizing.value = true
    const startX = event.clientX
    const startWidth = sidebarWidth.value

    document.body.classList.add('resizing')
    document.body.style.cursor = 'col-resize'

    const onMouseMove = (moveEvent: MouseEvent) => {
      if (!isResizing.value) return
      const delta = startX - moveEvent.clientX
      const newWidth = Math.max(SIDEBAR_MIN_WIDTH, Math.min(SIDEBAR_MAX_WIDTH, startWidth + delta))
      sidebarWidth.value = newWidth
    }

    const onMouseUp = () => {
      if (isResizing.value) {
        isResizing.value = false
        saveSidebarWidth(sidebarWidth.value)
      }
      document.body.classList.remove('resizing')
      document.body.style.cursor = ''
      document.removeEventListener('mousemove', onMouseMove)
      document.removeEventListener('mouseup', onMouseUp)
    }

    document.addEventListener('mousemove', onMouseMove)
    document.addEventListener('mouseup', onMouseUp)
  }

  const handleTodoSourceChange = (sourceKey: string) => {
    selectedTodoSourceKey.value = sourceKey || TODO_SOURCE_ALL_KEY
  }

  const selectedTaskTodoSourceKey = computed(() => {
    const assigneeId = params.selectedTeamTaskAssigneeId.value
    if (!assigneeId) return TODO_SOURCE_ALL_KEY
    return `member:${assigneeId}`
  })

  watch(todoSourceOptions, (options) => {
    if (options.length === 0) {
      selectedTodoSourceKey.value = TODO_SOURCE_ALL_KEY
      return
    }
    if (options.some((option) => option.key === selectedTodoSourceKey.value)) return
    selectedTodoSourceKey.value = TODO_SOURCE_ALL_KEY
  }, { immediate: true })

  watch(selectedTaskTodoSourceKey, (nextKey) => {
    if (!params.teamWorkspaceAvailable.value || !params.activeTeamSessionId.value) return
    if (nextKey === TODO_SOURCE_ALL_KEY) {
      selectedTodoSourceKey.value = TODO_SOURCE_ALL_KEY
      return
    }
    if (todoSourceOptions.value.some((option) => option.key === nextKey)) {
      selectedTodoSourceKey.value = nextKey
      return
    }
    selectedTodoSourceKey.value = TODO_SOURCE_ALL_KEY
  }, { immediate: true })

  watch(params.teamWorkspaceAvailable, (available) => {
    if (available) return
    if (activeRightPanel.value === 'team') {
      activeRightPanel.value = null
    }
    params.isTeamWorkspaceActive.value = false
  }, { immediate: true })

  const clearTodosForCurrentContext = () => {
    const convId = params.conversationId.value
    if (convId) {
      params.clearTodosForExecution(convId)
    }
    const sessionId = params.activeTeamSessionId.value
    if (!sessionId) return
    for (const executionId of params.todosExecutionIds.value) {
      const parsed = params.parseTeamTodoExecutionId(executionId)
      if (!parsed || parsed.sessionId !== sessionId) continue
      params.clearTodosForExecution(executionId)
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
    clearTodosForCurrentContext,
    deactivateRightPanel,
    error,
    handleCloseHtmlPanel,
    handleCloseTerminal,
    handleCloseTodos,
    handleRenderHtml,
    handleTodoSourceChange,
    handleToggleHtmlPanel,
    handleToggleTerminal,
    handleToggleTodos,
    hasHtmlPanelContent,
    hasTerminalHistory,
    htmlPanelContent,
    loadSidebarWidth,
    selectedTodoSourceKey,
    selectedTaskTodoSourceKey,
    sidebarWidth,
    startResize,
    todoBadgeCount,
    todoSourceOptions,
    todos,
  }
}
