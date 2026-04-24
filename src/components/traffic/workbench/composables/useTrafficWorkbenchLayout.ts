import { computed, ref, type Ref } from 'vue'

export const TRAFFIC_WORKBENCH_XL_BREAKPOINT = 1280
export const TRAFFIC_WORKBENCH_COLUMN_GAP = 12
export const TRAFFIC_WORKBENCH_RESIZER_WIDTH = 4
export const TRAFFIC_WORKBENCH_HISTORY_PANEL_DEFAULT_WIDTH = 620
export const TRAFFIC_WORKBENCH_SIDEBAR_DEFAULT_HEIGHT = 336
export const TRAFFIC_WORKBENCH_TOP_PANEL_DEFAULT_HEIGHT = 420
export const TRAFFIC_WORKBENCH_SIDEBAR_DEFAULT_WIDTH = 420

const INTERCEPT_DRAWER_DEFAULT_WIDTH = 760
const INTERCEPT_DRAWER_MIN_WIDTH = 560

export type TrafficWorkbenchGridMode = 'stacked' | 'split' | 'top-bottom'
export type TrafficWorkbenchLayoutPreference = 'side' | 'top-bottom'

function loadStoredNumber(storageKey: string, fallback: number) {
  if (typeof window === 'undefined') {
    return fallback
  }

  const stored = Number(window.localStorage.getItem(storageKey) || String(fallback))
  return Number.isFinite(stored) ? stored : fallback
}

function loadStoredBoolean(storageKey: string, fallback: boolean) {
  if (typeof window === 'undefined') {
    return fallback
  }

  const stored = window.localStorage.getItem(storageKey)
  if (stored === null) {
    return fallback
  }

  return stored === 'true'
}

export function getTrafficWorkbenchGridMode(
  viewportWidth: number,
  layoutPreference: TrafficWorkbenchLayoutPreference = 'side',
): TrafficWorkbenchGridMode {
  if (viewportWidth < TRAFFIC_WORKBENCH_XL_BREAKPOINT) {
    return 'stacked'
  }

  return layoutPreference === 'top-bottom' ? 'top-bottom' : 'split'
}

function resolveStageWidth(stageWidth: number) {
  return Number.isFinite(stageWidth) && stageWidth > 0 ? stageWidth : 1440
}

function resolveColumnBudget(mode: TrafficWorkbenchGridMode) {
  if (mode === 'split') {
    return (TRAFFIC_WORKBENCH_COLUMN_GAP * 2) + TRAFFIC_WORKBENCH_RESIZER_WIDTH
  }
  return 0
}

function clampFluidPanelSize(size: number, availableSize: number, occupiedSize = 0) {
  const maxSize = Math.max(0, availableSize - occupiedSize)
  return Math.min(Math.max(0, size), maxSize)
}

export function clampTrafficWorkbenchHistoryPanelWidth(options: {
  width: number
  stageWidth: number
  mode: TrafficWorkbenchGridMode
}) {
  const stage = resolveStageWidth(options.stageWidth)
  return clampFluidPanelSize(options.width, stage, resolveColumnBudget(options.mode))
}

export function buildTrafficWorkbenchGridTemplate(options: {
  mode: TrafficWorkbenchGridMode
  historyWidth: number
}) {
  if (options.mode === 'split') {
    return `${options.historyWidth}px ${TRAFFIC_WORKBENCH_RESIZER_WIDTH}px minmax(0, 1fr)`
  }
  return undefined
}

export function buildTrafficWorkbenchGridRows(options: {
  mode: TrafficWorkbenchGridMode
  topPanelHeight: number
}) {
  if (options.mode === 'top-bottom') {
    return `${options.topPanelHeight}px ${TRAFFIC_WORKBENCH_RESIZER_WIDTH}px minmax(0, 1fr)`
  }
  return undefined
}

export function clampTrafficWorkbenchSidebarHeight(options: {
  height: number
  columnHeight: number
}) {
  const columnHeight = resolveStageWidth(options.columnHeight)
  return clampFluidPanelSize(options.height, columnHeight, TRAFFIC_WORKBENCH_RESIZER_WIDTH)
}

export function buildTrafficWorkbenchLeftColumnTemplate(options: {
  mode: TrafficWorkbenchGridMode
  sidebarHeight: number
}) {
  if (options.mode === 'split') {
    return `minmax(0, 1fr) ${TRAFFIC_WORKBENCH_RESIZER_WIDTH}px ${options.sidebarHeight}px`
  }
  return undefined
}

export function buildTrafficWorkbenchLeftColumnColumns(options: {
  mode: TrafficWorkbenchGridMode
  sidebarWidth: number
}) {
  if (options.mode === 'top-bottom') {
    return `minmax(0, 1fr) ${TRAFFIC_WORKBENCH_RESIZER_WIDTH}px ${options.sidebarWidth}px`
  }
  return undefined
}

function normalizeLayoutPreference(value: string | null): TrafficWorkbenchLayoutPreference {
  return value === 'top-bottom' ? 'top-bottom' : 'side'
}

export function useTrafficWorkbenchLayout(options: {
  workspaceStageRef: Ref<HTMLElement | null>
  leftColumnRef: Ref<HTMLElement | null>
  historyPanelWidthStorageKey: string
  historyPanelHeightStorageKey: string
  sidebarHeightStorageKey: string
  sidebarWidthStorageKey: string
  interceptDrawerWidthStorageKey: string
  activeProbeCollapsedStorageKey: string
  layoutPreferenceStorageKey: string
}) {
  const {
    workspaceStageRef,
    leftColumnRef,
    historyPanelWidthStorageKey,
    historyPanelHeightStorageKey,
    sidebarHeightStorageKey,
    sidebarWidthStorageKey,
    interceptDrawerWidthStorageKey,
    activeProbeCollapsedStorageKey,
    layoutPreferenceStorageKey,
  } = options

  const viewportWidth = ref(typeof window === 'undefined' ? 1440 : window.innerWidth)
  const viewportHeight = ref(typeof window === 'undefined' ? 960 : window.innerHeight)
  const layoutPreference = ref<TrafficWorkbenchLayoutPreference>(
    typeof window === 'undefined'
      ? 'side'
      : normalizeLayoutPreference(window.localStorage.getItem(layoutPreferenceStorageKey)),
  )
  const historyPanelWidth = ref(
    loadStoredNumber(historyPanelWidthStorageKey, TRAFFIC_WORKBENCH_HISTORY_PANEL_DEFAULT_WIDTH),
  )
  const historyPanelHeight = ref(
    loadStoredNumber(historyPanelHeightStorageKey, TRAFFIC_WORKBENCH_TOP_PANEL_DEFAULT_HEIGHT),
  )
  const sidebarHeight = ref(
    loadStoredNumber(sidebarHeightStorageKey, TRAFFIC_WORKBENCH_SIDEBAR_DEFAULT_HEIGHT),
  )
  const sidebarWidth = ref(
    loadStoredNumber(sidebarWidthStorageKey, TRAFFIC_WORKBENCH_SIDEBAR_DEFAULT_WIDTH),
  )
  const interceptDrawerWidth = ref(
    loadStoredNumber(interceptDrawerWidthStorageKey, INTERCEPT_DRAWER_DEFAULT_WIDTH),
  )
  const activeProbeCollapsed = ref(loadStoredBoolean(activeProbeCollapsedStorageKey, false))
  const drawerWidthResizeState = ref<{
    drawer: 'intercept'
    startX: number
    startWidth: number
  } | null>(null)
  const panelResizeState = ref<{
    axis: 'x' | 'y'
    startPosition: number
    startSize: number
  } | null>(null)
  const sidebarHeightResizeState = ref<{
    axis: 'x' | 'y'
    startPosition: number
    startSize: number
  } | null>(null)
  const availableWorkbenchWidth = computed(
    () => workspaceStageRef.value?.offsetWidth ?? viewportWidth.value,
  )
  const workbenchGridMode = computed(() =>
    getTrafficWorkbenchGridMode(availableWorkbenchWidth.value, layoutPreference.value),
  )
  const isTopBottomLayout = computed(() => workbenchGridMode.value === 'top-bottom')
  const showHistoryPanelResizeHandle = computed(() =>
    workbenchGridMode.value === 'split' || workbenchGridMode.value === 'top-bottom',
  )
  const showSidebarHeightResizeHandle = computed(() =>
    workbenchGridMode.value === 'split' || workbenchGridMode.value === 'top-bottom',
  )
  const workbenchGridStyle = computed(() => {
    const templateColumns = buildTrafficWorkbenchGridTemplate({
      mode: workbenchGridMode.value,
      historyWidth: historyPanelWidth.value,
    })
    const templateRows = buildTrafficWorkbenchGridRows({
      mode: workbenchGridMode.value,
      topPanelHeight: historyPanelHeight.value,
    })

    if (!templateColumns && !templateRows) {
      return undefined
    }

    return {
      ...(templateColumns ? { gridTemplateColumns: templateColumns } : {}),
      ...(templateRows ? { gridTemplateRows: templateRows } : {}),
      ...(templateRows ? { rowGap: '0px' } : {}),
    }
  })
  const leftColumnStyle = computed(() => {
    const templateRows = buildTrafficWorkbenchLeftColumnTemplate({
      mode: workbenchGridMode.value,
      sidebarHeight: sidebarHeight.value,
    })
    const templateColumns = buildTrafficWorkbenchLeftColumnColumns({
      mode: workbenchGridMode.value,
      sidebarWidth: sidebarWidth.value,
    })
    if (workbenchGridMode.value === 'split') {
      return {
        gridTemplateRows: templateRows,
        rowGap: '0px',
      }
    }
    if (workbenchGridMode.value === 'top-bottom') {
      return {
        gridTemplateColumns: templateColumns,
        columnGap: '0px',
      }
    }
    return {
      rowGap: `${TRAFFIC_WORKBENCH_COLUMN_GAP}px`,
    }
  })
  const historyResizeHandleClass = computed(() =>
    isTopBottomLayout.value ? 'workbench-row-resizer' : 'workbench-column-resizer',
  )
  const sidebarResizeHandleClass = computed(() =>
    isTopBottomLayout.value ? 'workbench-column-resizer' : 'workbench-row-resizer',
  )
  const layoutToggleLabel = computed(() =>
    layoutPreference.value === 'top-bottom' ? '左右布局' : '上下布局',
  )
  const layoutToggleIcon = computed(() =>
    layoutPreference.value === 'top-bottom' ? 'fas fa-columns' : 'fas fa-grip-lines',
  )
  const effectiveLayoutLabel = computed(() =>
    workbenchGridMode.value === 'split' ? '左右布局' : '上下布局',
  )
  const interceptDrawerStyle = computed(() =>
    viewportWidth.value <= 1024 ? {} : { width: `${interceptDrawerWidth.value}px` },
  )
  const activeProbeShellStyle = computed(() => ({ right: '1.5rem', bottom: '1.5rem' }))

  function clampDrawerWidth(width: number) {
    const stageWidth = workspaceStageRef.value?.offsetWidth ?? viewportWidth.value
    const maxWidth = Math.max(INTERCEPT_DRAWER_MIN_WIDTH, stageWidth - 32)
    return Math.min(Math.max(INTERCEPT_DRAWER_MIN_WIDTH, width), maxWidth)
  }

  function clampTopPanelHeight(height: number) {
    const stageHeight = workspaceStageRef.value?.offsetHeight ?? viewportHeight.value
    return clampFluidPanelSize(height, stageHeight, TRAFFIC_WORKBENCH_RESIZER_WIDTH)
  }

  function clampSidebarWidth(width: number) {
    const columnWidth = leftColumnRef.value?.offsetWidth ?? availableWorkbenchWidth.value
    return clampFluidPanelSize(width, columnWidth, TRAFFIC_WORKBENCH_RESIZER_WIDTH)
  }

  function persistHistoryPanelWidth() {
    window.localStorage.setItem(historyPanelWidthStorageKey, String(historyPanelWidth.value))
  }

  function persistHistoryPanelHeight() {
    window.localStorage.setItem(historyPanelHeightStorageKey, String(historyPanelHeight.value))
  }

  function persistDrawerWidth() {
    window.localStorage.setItem(interceptDrawerWidthStorageKey, String(interceptDrawerWidth.value))
  }

  function persistSidebarHeight() {
    window.localStorage.setItem(sidebarHeightStorageKey, String(sidebarHeight.value))
  }

  function persistSidebarWidth() {
    window.localStorage.setItem(sidebarWidthStorageKey, String(sidebarWidth.value))
  }

  function persistActiveProbeCollapsed() {
    window.localStorage.setItem(activeProbeCollapsedStorageKey, String(activeProbeCollapsed.value))
  }

  function persistLayoutPreference() {
    window.localStorage.setItem(layoutPreferenceStorageKey, layoutPreference.value)
  }

  function applyDrawerWidth(width: number) {
    interceptDrawerWidth.value = clampDrawerWidth(width)
  }

  function normalizeWorkbenchLayout() {
    historyPanelWidth.value = clampTrafficWorkbenchHistoryPanelWidth({
      width: historyPanelWidth.value,
      stageWidth: workspaceStageRef.value?.offsetWidth ?? viewportWidth.value,
      mode: workbenchGridMode.value,
    })
    historyPanelHeight.value = clampTopPanelHeight(historyPanelHeight.value)
    sidebarHeight.value = clampTrafficWorkbenchSidebarHeight({
      height: sidebarHeight.value,
      columnHeight: leftColumnRef.value?.offsetHeight ?? viewportHeight.value,
    })
    sidebarWidth.value = clampSidebarWidth(sidebarWidth.value)
  }

  function handleDrawerWidthResize(event: MouseEvent) {
    const state = drawerWidthResizeState.value
    if (!state) {
      return
    }

    applyDrawerWidth(state.startWidth - (event.clientX - state.startX))
  }

  function stopDrawerWidthResize() {
    if (!drawerWidthResizeState.value) {
      return
    }

    persistDrawerWidth()
    drawerWidthResizeState.value = null
    document.body.style.cursor = ''
    document.body.style.userSelect = ''
    window.removeEventListener('mousemove', handleDrawerWidthResize)
    window.removeEventListener('mouseup', stopDrawerWidthResize)
  }

  function startDrawerWidthResize(drawer: 'intercept', event: MouseEvent) {
    if (viewportWidth.value <= 1024) {
      return
    }

    drawerWidthResizeState.value = {
      drawer,
      startX: event.clientX,
      startWidth: interceptDrawerWidth.value,
    }
    document.body.style.cursor = 'col-resize'
    document.body.style.userSelect = 'none'
    window.addEventListener('mousemove', handleDrawerWidthResize)
    window.addEventListener('mouseup', stopDrawerWidthResize)
    event.preventDefault()
  }

  function handleWorkbenchPanelResize(event: MouseEvent) {
    const state = panelResizeState.value
    if (!state) {
      return
    }

    if (state.axis === 'y') {
      historyPanelHeight.value = clampTopPanelHeight(
        state.startSize + (event.clientY - state.startPosition),
      )
      return
    }

    historyPanelWidth.value = clampTrafficWorkbenchHistoryPanelWidth({
      width: state.startSize + (event.clientX - state.startPosition),
      stageWidth: workspaceStageRef.value?.offsetWidth ?? viewportWidth.value,
      mode: workbenchGridMode.value,
    })
  }

  function stopWorkbenchPanelResize() {
    if (!panelResizeState.value) {
      return
    }

    if (panelResizeState.value.axis === 'y') {
      persistHistoryPanelHeight()
    } else {
      persistHistoryPanelWidth()
    }
    panelResizeState.value = null
    document.body.style.cursor = ''
    document.body.style.userSelect = ''
    window.removeEventListener('mousemove', handleWorkbenchPanelResize)
    window.removeEventListener('mouseup', stopWorkbenchPanelResize)
  }

  function startWorkbenchPanelResize(panel: 'history', event: MouseEvent) {
    if (
      panel !== 'history'
      || (workbenchGridMode.value !== 'split' && workbenchGridMode.value !== 'top-bottom')
    ) {
      return
    }

    const axis = workbenchGridMode.value === 'top-bottom' ? 'y' : 'x'
    panelResizeState.value = {
      axis,
      startPosition: axis === 'y' ? event.clientY : event.clientX,
      startSize: axis === 'y' ? historyPanelHeight.value : historyPanelWidth.value,
    }
    document.body.style.cursor = axis === 'y' ? 'row-resize' : 'col-resize'
    document.body.style.userSelect = 'none'
    window.addEventListener('mousemove', handleWorkbenchPanelResize)
    window.addEventListener('mouseup', stopWorkbenchPanelResize)
    event.preventDefault()
  }

  function handleSidebarHeightResize(event: MouseEvent) {
    const state = sidebarHeightResizeState.value
    if (!state) {
      return
    }

    if (state.axis === 'x') {
      sidebarWidth.value = clampSidebarWidth(
        state.startSize - (event.clientX - state.startPosition),
      )
      return
    }

    sidebarHeight.value = clampTrafficWorkbenchSidebarHeight({
      height: state.startSize - (event.clientY - state.startPosition),
      columnHeight: leftColumnRef.value?.offsetHeight ?? viewportHeight.value,
    })
  }

  function stopSidebarHeightResize() {
    if (!sidebarHeightResizeState.value) {
      return
    }

    if (sidebarHeightResizeState.value.axis === 'x') {
      persistSidebarWidth()
    } else {
      persistSidebarHeight()
    }
    sidebarHeightResizeState.value = null
    document.body.style.cursor = ''
    document.body.style.userSelect = ''
    window.removeEventListener('mousemove', handleSidebarHeightResize)
    window.removeEventListener('mouseup', stopSidebarHeightResize)
  }

  function startSidebarHeightResize(event: MouseEvent) {
    if (workbenchGridMode.value !== 'split' && workbenchGridMode.value !== 'top-bottom') {
      return
    }

    const axis = workbenchGridMode.value === 'top-bottom' ? 'x' : 'y'
    sidebarHeightResizeState.value = {
      axis,
      startPosition: axis === 'x' ? event.clientX : event.clientY,
      startSize: axis === 'x' ? sidebarWidth.value : sidebarHeight.value,
    }
    document.body.style.cursor = axis === 'x' ? 'col-resize' : 'row-resize'
    document.body.style.userSelect = 'none'
    window.addEventListener('mousemove', handleSidebarHeightResize)
    window.addEventListener('mouseup', stopSidebarHeightResize)
    event.preventDefault()
  }

  function handleWindowResize() {
    viewportWidth.value = window.innerWidth
    viewportHeight.value = window.innerHeight
    normalizeWorkbenchLayout()
    applyDrawerWidth(interceptDrawerWidth.value)
  }

  function toggleActiveProbeCollapsed() {
    activeProbeCollapsed.value = !activeProbeCollapsed.value
    persistActiveProbeCollapsed()
  }

  function toggleWorkbenchLayoutPreference() {
    layoutPreference.value = layoutPreference.value === 'top-bottom' ? 'side' : 'top-bottom'
    persistLayoutPreference()
    normalizeWorkbenchLayout()
  }

  return {
    viewportWidth,
    layoutPreference,
    historyPanelWidth,
    historyPanelHeight,
    sidebarHeight,
    sidebarWidth,
    interceptDrawerWidth,
    activeProbeCollapsed,
    workbenchGridMode,
    isTopBottomLayout,
    workbenchGridStyle,
    leftColumnStyle,
    historyResizeHandleClass,
    sidebarResizeHandleClass,
    layoutToggleLabel,
    layoutToggleIcon,
    effectiveLayoutLabel,
    showHistoryPanelResizeHandle,
    showSidebarHeightResizeHandle,
    interceptDrawerStyle,
    activeProbeShellStyle,
    handleWindowResize,
    startWorkbenchPanelResize,
    stopWorkbenchPanelResize,
    startSidebarHeightResize,
    stopSidebarHeightResize,
    startDrawerWidthResize,
    stopDrawerWidthResize,
    normalizeWorkbenchLayout,
    toggleWorkbenchLayoutPreference,
    toggleActiveProbeCollapsed,
  }
}
