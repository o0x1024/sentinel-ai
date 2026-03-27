<template>
    <div class="flowchart-visualization">
        <FlowchartToolbar
            :title="t('trafficAnalysis.workflowStudio.flowchart.toolbar.title')"
            :new-workflow-label="t('trafficAnalysis.workflowStudio.flowchart.toolbar.newWorkflow')"
            :new-workflow-tooltip="t('trafficAnalysis.workflowStudio.flowchart.toolbar.newWorkflowTooltip')"
            :ai-generate-label="t('trafficAnalysis.workflowStudio.flowchart.toolbar.aiGenerate')"
            :ai-generate-tooltip="t('trafficAnalysis.workflowStudio.flowchart.toolbar.aiGenerateTooltip')"
            :zoom-label="`${Math.round(zoomLevel * 100)}%`"
            :fit-to-view-tooltip="t('trafficAnalysis.workflowStudio.flowchart.toolbar.fitToViewTooltip')"
            :reset-view-tooltip="t('trafficAnalysis.workflowStudio.flowchart.toolbar.resetViewTooltip')"
            :minimap-tooltip="t('trafficAnalysis.workflowStudio.flowchart.toolbar.minimapTooltip')"
            :arrange-nodes-label="t('trafficAnalysis.workflowStudio.flowchart.toolbar.arrangeNodes')"
            :arrange-nodes-tooltip="t('trafficAnalysis.workflowStudio.flowchart.toolbar.arrangeNodesTooltip')"
            :undo-tooltip="t('trafficAnalysis.workflowStudio.flowchart.toolbar.undoTooltip')"
            :redo-tooltip="t('trafficAnalysis.workflowStudio.flowchart.toolbar.redoTooltip')"
            :can-undo="canUndo"
            :can-redo="canRedo"
            :show-minimap="showMinimap"
            :on-new-workflow="onNewWorkflow"
            :on-open-ai-generate-modal="openAiGenerateModal"
            :on-zoom-out="zoomOut"
            :on-reset-zoom="resetZoom"
            :on-zoom-in="zoomIn"
            :on-toggle-fullscreen="toggleFullscreen"
            :on-fit-to-view="fitToView"
            :on-reset-canvas-view="resetCanvasView"
            :on-toggle-minimap="() => { showMinimap = !showMinimap }"
            :on-arrange-nodes="arrangeNodes"
            :on-undo="undo"
            :on-redo="redo"
        />

        <!-- 流程图容器 -->
        <div class="card bg-base-100 shadow-xl relative" :class="{ 'fullscreen': isFullscreen }">
            <FlowchartCanvasStage
                ref="flowchartContainer"
                :is-dragging="isDragging"
                :is-panning-canvas="isPanningCanvas"
                :is-space-pressed="isSpacePressed"
                :nodes="nodes"
                :connections="connections"
                :container-size="containerSize"
                :content-style="contentStyle"
                :get-connection-class="getConnectionClass"
                :is-dragging-connection="isDraggingConnection"
                :temp-connection-path="tempConnectionPath"
                :is-selecting="isSelecting"
                :selection-box="selectionBox"
                :dragged-node="draggedNode"
                :selected-nodes="selectedNodes"
                :highlighted-nodes="highlightedNodes"
                :get-node-class="getNodeClass"
                :hover-port="hover_port"
                :drag-connection-start="dragConnectionStart"
                :breakpoints="breakpoints"
                :breakpoints-title="t('trafficAnalysis.workflowStudio.flowchart.breakpoints.title')"
                :get-status-indicator-class="getStatusIndicatorClass"
                :get-status-badge-class="getStatusBadgeClass"
                :get-status-text="getStatusText"
                :get-node-icon="get_node_icon"
                :input-label="t('trafficAnalysis.workflowStudio.flowchart.ports.input')"
                :output-label="t('trafficAnalysis.workflowStudio.flowchart.ports.output')"
                :show-minimap="showMinimap"
                :minimap-view-box="minimapViewBox"
                :minimap-viewport-rect="minimapViewportRect"
                :selected-hint="t('trafficAnalysis.workflowStudio.flowchart.canvasHints.selected', { count: selectedNodes.size })"
                :space-hint="t('trafficAnalysis.workflowStudio.flowchart.canvasHints.space')"
                :scroll-hint="t('trafficAnalysis.workflowStudio.flowchart.canvasHints.scroll')"
                :drag-hint="t('trafficAnalysis.workflowStudio.flowchart.canvasHints.drag')"
                :select-all-hint="`Ctrl+A: ${t('trafficAnalysis.workflowStudio.flowchart.canvasHints.selectAll')}`"
                :is-fullscreen="isFullscreen"
                :exit-fullscreen-label="t('trafficAnalysis.workflowStudio.flowchart.toolbar.exitFullscreen')"
                :empty-title="t('trafficAnalysis.workflowStudio.flowchart.emptyState.title')"
                :empty-description="t('trafficAnalysis.workflowStudio.flowchart.emptyState.description')"
                :empty-tip="t('trafficAnalysis.workflowStudio.flowchart.emptyState.tip')"
                :on-pointer-down="on_pointer_down"
                :on-pointer-move="on_pointer_move"
                :on-pointer-up="on_pointer_up"
                :on-wheel="onWheel"
                :on-connection-click="onConnectionClick"
                :on-connection-context-menu="onConnectionContextMenu"
                :on-node-pointer-down="on_node_pointer_down"
                :on-node-context-menu="onNodeContextMenu"
                :on-node-enter="onNodeEnter"
                :on-node-leave="onNodeLeave"
                :start-drag-connection="start_drag_connection"
                :end-drag-connection="end_drag_connection"
                :set-hover-port="setHoverPort"
                :clear-hover-port="clearHoverPort"
                :on-minimap-click="onMinimapClick"
                :on-toggle-fullscreen="toggleFullscreen"
                @toggle-minimap="showMinimap = false"
            />
        </div>

        <FlowchartOverlays
            :context-menu="contextMenu"
            :show-ai-generate-modal="showAiGenerateModal"
            :ai-generate-text="aiGenerateText"
            :is-ai-generating="isAiGenerating"
            :ai-generate-error="aiGenerateError"
            :title="t('trafficAnalysis.workflowStudio.flowchart.aiGenerate.title')"
            :help-text="t('trafficAnalysis.workflowStudio.flowchart.aiGenerate.help')"
            :placeholder="t('trafficAnalysis.workflowStudio.flowchart.aiGenerate.placeholder')"
            :cancel-label="t('trafficAnalysis.workflowStudio.flowchart.aiGenerate.cancel')"
            :generate-label="t('trafficAnalysis.workflowStudio.flowchart.aiGenerate.generateAndLoad')"
            :on-context-menu-click="handleContextMenuClick"
            :on-close-ai-generate-modal="closeAiGenerateModal"
            :on-generate-workflow-from-nl="generateWorkflowFromNl"
            @update:ai-generate-text="aiGenerateText = $event"
        />
    </div>
</template>

<script setup lang="ts">
import { ref, reactive, computed, onMounted, onUnmounted, watch, nextTick } from 'vue'
import type { CSSProperties } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useI18n } from 'vue-i18n'
import type { EdgeDef } from '@/types/workflow'
import FlowchartCanvasStage from './FlowchartCanvasStage.vue'
import FlowchartOverlays from './FlowchartOverlays.vue'
import FlowchartToolbar from './FlowchartToolbar.vue'
import {
  arrangeFlowchartNodes,
  calculateFlowchartConnectionLabelPosition,
  calculateFlowchartConnectionPath,
  createFlowchartHistoryState,
  describeFlowchartEdgeMapping,
  getFlowchartConnectionClass,
  getFlowchartConnectionStatus,
  getFlowchartNodeClass,
  getFlowchartNodeIcon,
  getFlowchartStatusBadgeClass,
  getFlowchartStatusIndicatorClass,
  getFlowchartStatusText,
  restoreFlowchartHistoryState,
  type FlowchartConnection,
  type FlowchartNode,
  type HistoryState,
  type NodeStatus,
} from './flowchartVisualizationSupport'

const { t } = useI18n()

defineOptions({
  name: 'Flowchartvisualization'
});

// Props
interface Props {
    sessionId?: string
    planData?: any
    realTimeUpdates?: boolean
    highlightedNodes?: Set<string>
}

const props = withDefaults(defineProps<Props>(), {
    realTimeUpdates: true,
    highlightedNodes: () => new Set()
})

// Emits
const emit = defineEmits<{
    nodeClick: [node: FlowchartNode]
    connectionClick: [connection: FlowchartConnection]
    newWorkflow: []
    change: [] // 流程图发生变化时触发
}>()

// 响应式数据
const flowchartContainer = ref<{ containerEl: HTMLElement | null } | null>(null)
const nodes = ref<FlowchartNode[]>([])
const connections = ref<FlowchartConnection[]>([])
const customEdges = ref<EdgeDef[]>([])
const autoLayout = ref(false)
const draggedNode = ref<FlowchartNode | null>(null)
const isDragging = ref(false)
const dragMoved = ref(false)
const dragOffset = reactive({ x: 0, y: 0 })
const isFullscreen = ref(false)
const zoomLevel = ref(1)
// 已移除布局模式，保留自由拖拽

// AI生成
const showAiGenerateModal = ref(false)
const aiGenerateText = ref('')
const isAiGenerating = ref(false)
const aiGenerateError = ref('')

// 画布拖拽
const isPanningCanvas = ref(false)
const panStart = reactive({ x: 0, y: 0 })
const panOffset = reactive({ x: 0, y: 0 })

// 空格键状态
const isSpacePressed = ref(false)

// 小地图
const showMinimap = ref(true)

// 多选功能
const selectedNodes = ref<Set<string>>(new Set())
const isSelecting = ref(false)
const selectionBox = reactive({ startX: 0, startY: 0, endX: 0, endY: 0 })

// 多选拖拽
const isDraggingSelection = ref(false)
const selectionDragStart = reactive({ x: 0, y: 0 })
const nodeStartPositions = ref<Map<string, { x: number, y: number }>>(new Map())

// 拖拽连接
const isDraggingConnection = ref(false)
const dragConnectionStart = ref<{ nodeId: string, portId: string, portType: 'input' | 'output', x: number, y: number } | null>(null)
const dragConnectionEnd = reactive({ x: 0, y: 0 })
const tempConnectionPath = ref('')
const hover_port = ref<{ nodeId: string, portId: string, type: 'input' | 'output' } | null>(null)

// 断点调试
const breakpoints = ref<Set<string>>(new Set())
const debugMode = ref(false)
const currentDebugNode = ref<string | null>(null)

// 右键菜单
const contextMenu = reactive({
    visible: false,
    x: 0,
    y: 0,
    node: null as FlowchartNode | null,
    items: [] as Array<{ label: string, action: () => void, danger?: boolean }>
})

const history = ref<HistoryState[]>([])
const historyIndex = ref(-1)
const MAX_HISTORY = 50

const createEdgeId = () => `edge_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`

const canUndo = computed(() => historyIndex.value > 0)
const canRedo = computed(() => historyIndex.value < history.value.length - 1)

const getFlowchartContainerElement = () => flowchartContainer.value?.containerEl ?? null

// 无限画布 - 动态计算内容边界
const CANVAS_PADDING = 200
const MIN_CANVAS_SIZE = 2000

const canvasBounds = computed(() => {
    if (nodes.value.length === 0) {
        return { minX: 0, minY: 0, maxX: MIN_CANVAS_SIZE, maxY: MIN_CANVAS_SIZE }
    }
    let minX = Infinity, minY = Infinity, maxX = -Infinity, maxY = -Infinity
    const NODE_WIDTH = 180
    const NODE_HEIGHT = 80
    nodes.value.forEach(n => {
        minX = Math.min(minX, n.x)
        minY = Math.min(minY, n.y)
        maxX = Math.max(maxX, n.x + NODE_WIDTH)
        maxY = Math.max(maxY, n.y + NODE_HEIGHT)
    })
    return {
        minX: Math.min(minX - CANVAS_PADDING, 0),
        minY: Math.min(minY - CANVAS_PADDING, 0),
        maxX: Math.max(maxX + CANVAS_PADDING, MIN_CANVAS_SIZE),
        maxY: Math.max(maxY + CANVAS_PADDING, MIN_CANVAS_SIZE)
    }
})

const containerSize = computed(() => ({
    width: canvasBounds.value.maxX - canvasBounds.value.minX,
    height: canvasBounds.value.maxY - canvasBounds.value.minY
}))

const viewportSize = reactive({
    width: 800,
    height: 600
})

const contentStyle = computed<CSSProperties>(() => ({
    transform: `translate(${panOffset.x}px, ${panOffset.y}px) scale(${zoomLevel.value})`,
    transformOrigin: 'top left',
    width: containerSize.value.width + 'px',
    height: containerSize.value.height + 'px',
    position: 'relative',
    transition: isPanningCanvas.value ? 'none' : 'transform 0.1s ease-out'
}))

// 小地图相关计算
const minimapViewBox = computed(() => {
    const b = canvasBounds.value
    return `${b.minX} ${b.minY} ${b.maxX - b.minX} ${b.maxY - b.minY}`
})

const minimapViewportRect = computed(() => {
    const scale = zoomLevel.value
    return {
        x: -panOffset.x / scale,
        y: -panOffset.y / scale,
        width: viewportSize.width / scale,
        height: viewportSize.height / scale
    }
})

const saveHistory = () => {
  const state = createFlowchartHistoryState(nodes.value, customEdges.value)
  
  // 删除当前索引之后的历史
  if (historyIndex.value < history.value.length - 1) {
    history.value = history.value.slice(0, historyIndex.value + 1)
  }
  
  history.value.push(state)
  
  // 限制历史记录数量
  if (history.value.length > MAX_HISTORY) {
    history.value.shift()
  } else {
    historyIndex.value++
  }
}

const undo = () => {
  if (!canUndo.value) return
  historyIndex.value--
  restoreHistory()
}

const redo = () => {
  if (!canRedo.value) return
  historyIndex.value++
  restoreHistory()
}

const restoreHistory = () => {
  if (historyIndex.value < 0 || historyIndex.value >= history.value.length) return
  const restored = restoreFlowchartHistoryState(history.value[historyIndex.value])
  nodes.value = restored.nodes
  customEdges.value = restored.edges
  updateConnections()
}

// 计算属性
    const getNodeClass = computed(() => (node: FlowchartNode) => getFlowchartNodeClass(node))

// 已移除 fromPortOptions 和 toPortOptions - 使用拖拽连接代替

const getStatusIndicatorClass = computed(() => (status: NodeStatus) => getFlowchartStatusIndicatorClass(status))
const getStatusBadgeClass = computed(() => (status: NodeStatus) => getFlowchartStatusBadgeClass(status))
const getConnectionClass = computed(() => (connection: FlowchartConnection) => getFlowchartConnectionClass(connection))

// 方法
const getStatusText = (status: NodeStatus): string => getFlowchartStatusText(status, t)

const initializeFlowchart = () => {
    nodes.value = []
    connections.value = []
}

const onNodeClick = (node: FlowchartNode, event?: MouseEvent) => {
    // 如果刚刚拖拽移动过节点，不触发点击事件
    if (dragMoved.value) { 
        dragMoved.value = false
        return 
    }
    emit('nodeClick', node)
}

// 右键菜单事件处理
const onNodeContextMenu = (event: MouseEvent, node: FlowchartNode) => {
    event.stopPropagation() // 阻止冒泡到容器
    console.log('onNodeContextMenu triggered for node:', node.id)
    showNodeContextMenu(node, event)
}

const removeConnectionByEdgeId = (edgeId: string) => {
    saveHistory()
    const connection = connections.value.find(item => item.edgeId === edgeId)
    if (connection) {
        const target = nodes.value.find(n => n.id === connection.to)
        if (target && target.dependencies) {
            target.dependencies = target.dependencies.filter(d => d !== connection.from)
        }
    }
    customEdges.value = customEdges.value.filter(e => e.id !== edgeId)
    updateConnections()
    emit('change')
}

// 显示节点上下文菜单
const showNodeContextMenu = (node: FlowchartNode, event: MouseEvent) => {
    const hasBreakpoint = breakpoints.value.has(node.id)
    
    contextMenu.visible = true
    contextMenu.x = event.clientX
    contextMenu.y = event.clientY
    contextMenu.node = node
    contextMenu.items = [
        { 
            label: hasBreakpoint
                ? t('trafficAnalysis.workflowStudio.flowchart.contextMenu.removeBreakpoint')
                : t('trafficAnalysis.workflowStudio.flowchart.contextMenu.addBreakpoint'),
            action: () => toggleBreakpoint(node.id) 
        },
        { 
            label: t('trafficAnalysis.workflowStudio.flowchart.contextMenu.duplicateNode'),
            action: () => duplicateNode(node) 
        },
        { 
            label: t('trafficAnalysis.workflowStudio.flowchart.contextMenu.deleteNode'),
            action: () => removeNode(node.id),
            danger: true
        }
    ]
}

const showConnectionContextMenu = (connection: FlowchartConnection, event: MouseEvent) => {
    contextMenu.visible = true
    contextMenu.x = event.clientX
    contextMenu.y = event.clientY
    contextMenu.node = null
    contextMenu.items = [
        {
            label: '编辑边映射',
            action: () => emit('connectionClick', connection),
        },
        {
            label: '删除连接',
            action: () => removeConnectionByEdgeId(connection.edgeId),
            danger: true,
        },
    ]
}

// 处理右键菜单点击
const handleContextMenuClick = (item: { label: string, action: () => void, danger?: boolean }) => {
    item.action()
    contextMenu.visible = false
}

// 关闭右键菜单
const closeContextMenu = () => {
    contextMenu.visible = false
}

// 切换断点
const toggleBreakpoint = (nodeId: string) => {
    if (breakpoints.value.has(nodeId)) {
        breakpoints.value.delete(nodeId)
    } else {
        breakpoints.value.add(nodeId)
    }
}

// 复制节点
const duplicateNode = (node: FlowchartNode) => {
    const newNode = {
        ...node,
        id: `node_${Date.now()}`,
        x: node.x + 50,
        y: node.y + 50,
        name: t('trafficAnalysis.workflowStudio.flowchart.contextMenu.duplicateNodeName', { name: node.name })
    }
    saveHistory()
    nodes.value.push(newNode)
    updateConnections()
    emit('change')
}

// 删除节点
const removeNode = (nodeId: string) => {
    saveHistory()
    nodes.value = nodes.value.filter(n => n.id !== nodeId)
    // 删除相关连接
    customEdges.value = customEdges.value.filter(e => e.from_node !== nodeId && e.to_node !== nodeId)
    // 从其他节点的依赖中移除
    nodes.value.forEach(node => {
        if (node.dependencies) {
            node.dependencies = node.dependencies.filter(d => d !== nodeId)
        }
    })
    updateConnections()
    emit('change')
}

const onNodeEnter = (node: FlowchartNode) => {
    // Node hover handling - reserved for future use
}

const onNodeLeave = (_node: FlowchartNode) => {
    // Node hover handling - reserved for future use
}

const updateConnections = () => {
    const nodeIds = new Set(nodes.value.map(n => n.id))
    // 清理引用了不存在节点的无效边
    customEdges.value = customEdges.value.filter(e => nodeIds.has(e.from_node) && nodeIds.has(e.to_node))
    
    const newConnections: FlowchartConnection[] = []
    customEdges.value.forEach(edge => {
        const fromNode = nodes.value.find(n => n.id === edge.from_node)
        const toNode = nodes.value.find(n => n.id === edge.to_node)
        if (fromNode && toNode) {
            const labelPosition = calculateFlowchartConnectionLabelPosition(fromNode, toNode)
            newConnections.push({
                id: edge.id,
                edgeId: edge.id,
                from: edge.from_node,
                to: edge.to_node,
                path: calculateFlowchartConnectionPath(fromNode, toNode),
                status: getFlowchartConnectionStatus(fromNode, toNode),
                label: describeFlowchartEdgeMapping(edge),
                labelX: labelPosition.x,
                labelY: labelPosition.y,
            })
        }
    })
    connections.value = newConnections
}

let rafId: number | null = null
let connThrottleTs = 0
const CONN_THROTTLE_MS = 80
const scheduleConnectionsUpdate = () => {
    if (isDragging.value) {
        const now = performance.now()
        if (now - connThrottleTs < CONN_THROTTLE_MS) return
        connThrottleTs = now
    }
    if (rafId !== null) return
    rafId = requestAnimationFrame(() => {
        rafId = null
        if (isDragging.value && draggedNode.value) {
            updateConnectionsPartial(draggedNode.value.id)
        } else {
            updateConnections()
        }
    })
}

const updateConnectionsPartial = (nodeId: string) => {
    if (!connections.value.length) return
    const n = nodes.value.find(x => x.id === nodeId)
    if (!n) return
    for (const conn of connections.value) {
        if (conn.from === nodeId || conn.to === nodeId) {
            const fromNode = nodes.value.find(x => x.id === conn.from)
            const toNode = nodes.value.find(x => x.id === conn.to)
            if (fromNode && toNode) {
                const labelPosition = calculateFlowchartConnectionLabelPosition(fromNode, toNode)
                conn.path = calculateFlowchartConnectionPath(fromNode, toNode)
                conn.status = getFlowchartConnectionStatus(fromNode, toNode)
                conn.labelX = labelPosition.x
                conn.labelY = labelPosition.y
            }
        }
    }
}


const resetView = () => {
    initializeFlowchart()
}

// 重置画布视图（不清空节点）
const resetCanvasView = () => {
    panOffset.x = 0
    panOffset.y = 0
    zoomLevel.value = 1
}

// 适应视图 - 缩放并平移以显示所有节点
const fitToView = () => {
    if (nodes.value.length === 0) {
        resetCanvasView()
        return
    }
    
    const NODE_WIDTH = 180
    const NODE_HEIGHT = 80
    const PADDING = 50
    
    let minX = Infinity, minY = Infinity, maxX = -Infinity, maxY = -Infinity
    nodes.value.forEach(n => {
        minX = Math.min(minX, n.x)
        minY = Math.min(minY, n.y)
        maxX = Math.max(maxX, n.x + NODE_WIDTH)
        maxY = Math.max(maxY, n.y + NODE_HEIGHT)
    })
    
    const contentWidth = maxX - minX + PADDING * 2
    const contentHeight = maxY - minY + PADDING * 2
    
    const scaleX = viewportSize.width / contentWidth
    const scaleY = viewportSize.height / contentHeight
    const newZoom = Math.min(scaleX, scaleY, 1.5) // 最大1.5倍
    
    zoomLevel.value = Math.max(0.1, newZoom)
    
    // 居中显示
    const centerX = (minX + maxX) / 2
    const centerY = (minY + maxY) / 2
    panOffset.x = viewportSize.width / 2 - centerX * zoomLevel.value
    panOffset.y = viewportSize.height / 2 - centerY * zoomLevel.value
}

// 滚轮缩放
const onWheel = (event: WheelEvent) => {
    event.preventDefault()
    
    const rect = getFlowchartContainerElement()?.getBoundingClientRect()
    if (!rect) return
    
    // 鼠标相对于容器的位置
    const mouseX = event.clientX - rect.left
    const mouseY = event.clientY - rect.top
    
    // 缩放前鼠标对应的画布坐标
    const beforeX = (mouseX - panOffset.x) / zoomLevel.value
    const beforeY = (mouseY - panOffset.y) / zoomLevel.value
    
    // 计算新的缩放级别
    const delta = event.deltaY > 0 ? 0.9 : 1.1
    const newZoom = Math.max(0.1, Math.min(3, zoomLevel.value * delta))
    zoomLevel.value = newZoom
    
    // 缩放后保持鼠标位置不变
    panOffset.x = mouseX - beforeX * newZoom
    panOffset.y = mouseY - beforeY * newZoom
}

// 小地图点击导航
const onMinimapClick = (event: PointerEvent) => {
    const target = event.currentTarget as HTMLElement
    const rect = target.getBoundingClientRect()
    const bounds = canvasBounds.value
    
    // 计算点击位置对应的画布坐标
    const relX = (event.clientX - rect.left) / rect.width
    const relY = (event.clientY - rect.top) / rect.height
    
    const canvasX = bounds.minX + relX * (bounds.maxX - bounds.minX)
    const canvasY = bounds.minY + relY * (bounds.maxY - bounds.minY)
    
    // 将该位置移动到视口中心
    panOffset.x = viewportSize.width / 2 - canvasX * zoomLevel.value
    panOffset.y = viewportSize.height / 2 - canvasY * zoomLevel.value
}

const arrangeNodes = () => {
    arrangeFlowchartNodes(nodes.value)
    updateConnections()
}




const zoomIn = () => {
    zoomLevel.value = Math.min(zoomLevel.value * 1.2, 3)
}

const zoomOut = () => {
    zoomLevel.value = Math.max(zoomLevel.value / 1.2, 0.1)
}

const resetZoom = () => {
    zoomLevel.value = 1
}

const toggleFullscreen = () => {
    isFullscreen.value = !isFullscreen.value
    nextTick(() => {
        updateContainerSize()
    })
}

// 拖拽功能
const drag_ctx = reactive({ rect_left: 0, rect_top: 0, scale: 1, startX: 0, startY: 0 })
const DRAG_THRESHOLD = 5 // 移动超过5像素才认为是拖拽

const on_node_pointer_down = (event: PointerEvent, node: FlowchartNode) => {
    // 只允许左键拖拽
    if (event.button !== 0) {
        // 如果是右键，阻止冒泡以防止容器捕获指针
        if (event.button === 2) {
            event.stopPropagation()
        }
        return
    }

    event.stopPropagation() // 阻止事件冒泡到画布
    // 注意：不调用 preventDefault()，否则会阻止 click 事件
    dragMoved.value = false
    
    // 如果是Shift+点击，不拖拽节点，而是画布平移
    if (event.shiftKey && !selectedNodes.value.has(node.id)) {
        return
    }
    
    const rect = getFlowchartContainerElement()?.getBoundingClientRect()
    if (!rect) return
    
    drag_ctx.rect_left = rect.left
    drag_ctx.rect_top = rect.top
    drag_ctx.scale = zoomLevel.value
    drag_ctx.startX = event.clientX
    drag_ctx.startY = event.clientY
    
    const localX = (event.clientX - drag_ctx.rect_left - panOffset.x) / drag_ctx.scale
    const localY = (event.clientY - drag_ctx.rect_top - panOffset.y) / drag_ctx.scale
    
    // 如果点击的节点在选中集合中，整体拖动选中的节点
    if (selectedNodes.value.has(node.id) && selectedNodes.value.size > 1) {
        isDraggingSelection.value = true
        selectionDragStart.x = localX
        selectionDragStart.y = localY
        // 记录所有选中节点的初始位置
        nodeStartPositions.value.clear()
        selectedNodes.value.forEach(nodeId => {
            const n = nodes.value.find(x => x.id === nodeId)
            if (n) {
                nodeStartPositions.value.set(nodeId, { x: n.x, y: n.y })
            }
        })
    } else {
        // 单节点拖拽
        // Ctrl/Cmd+点击切换选中状态
        if (event.ctrlKey || event.metaKey) {
            if (selectedNodes.value.has(node.id)) {
                selectedNodes.value.delete(node.id)
            } else {
                selectedNodes.value.add(node.id)
            }
            return
        }
        
        // 普通点击，清除其他选中，只选中当前节点
        if (!selectedNodes.value.has(node.id)) {
            selectedNodes.value.clear()
            selectedNodes.value.add(node.id)
        }
        
        draggedNode.value = node
        isDragging.value = true
        dragOffset.x = localX - node.x
        dragOffset.y = localY - node.y
    }
    
    const containerEl = getFlowchartContainerElement() as HTMLElement & { setPointerCapture?: (pointerId: number) => void } | null
    if (containerEl?.setPointerCapture) {
        containerEl.setPointerCapture(event.pointerId)
    }
}

const on_pointer_down = (event: PointerEvent) => {
    // 右键点击不处理，让其触发 contextmenu
    if (event.button === 2) return

    const containerEl = getFlowchartContainerElement()
    const isOnConnection = Boolean((event.target as HTMLElement).closest('.flowchart-connection-hit'))
    const isOnCanvas = event.target === containerEl || 
        ((event.target as HTMLElement).closest('.flowchart-content') && 
         !(event.target as HTMLElement).closest('.flowchart-node') &&
         !isOnConnection)
    
    if (isOnCanvas) {
        // 空白区域左键拖拽：直接移动画布
        // Shift+拖拽或中键：也是移动画布
        // Ctrl/Cmd+拖拽：框选
        if (event.button === 1 || (event.button === 0 && !event.ctrlKey && !event.metaKey)) {
            event.preventDefault()
            isPanningCanvas.value = true
            panStart.x = event.clientX - panOffset.x
            panStart.y = event.clientY - panOffset.y
        } else if (event.button === 0 && (event.ctrlKey || event.metaKey)) {
            // Ctrl/Cmd+左键在空白区域：开始框选
            event.preventDefault()
            const rect = getFlowchartContainerElement()?.getBoundingClientRect()
            if (rect) {
                const scale = zoomLevel.value
                const localX = (event.clientX - rect.left - panOffset.x) / scale
                const localY = (event.clientY - rect.top - panOffset.y) / scale
                isSelecting.value = true
                selectionBox.startX = localX
                selectionBox.startY = localY
                selectionBox.endX = localX
                selectionBox.endY = localY
            }
        }
        draggedNode.value = null
    }
    const captureEl = getFlowchartContainerElement() as HTMLElement & { setPointerCapture?: (pointerId: number) => void } | null
    if (captureEl?.setPointerCapture) {
        captureEl.setPointerCapture(event.pointerId)
    }
}

    const on_pointer_move = (event: PointerEvent) => {
    // 框选
    if (isSelecting.value) {
        event.preventDefault()
        const rect = getFlowchartContainerElement()?.getBoundingClientRect()
        if (rect) {
            const scale = zoomLevel.value
            selectionBox.endX = (event.clientX - rect.left - panOffset.x) / scale
            selectionBox.endY = (event.clientY - rect.top - panOffset.y) / scale
            
            // 实时更新选中的节点
            const minX = Math.min(selectionBox.startX, selectionBox.endX)
            const maxX = Math.max(selectionBox.startX, selectionBox.endX)
            const minY = Math.min(selectionBox.startY, selectionBox.endY)
            const maxY = Math.max(selectionBox.startY, selectionBox.endY)
            
            const NODE_WIDTH = 180
            const NODE_HEIGHT = 80
            
            // 框选时清除之前的选择，重新计算
            selectedNodes.value.clear()
            nodes.value.forEach(node => {
                const nodeRight = node.x + NODE_WIDTH
                const nodeBottom = node.y + NODE_HEIGHT
                // 检查节点是否与选择框相交
                const intersects = !(node.x > maxX || nodeRight < minX || node.y > maxY || nodeBottom < minY)
                if (intersects) {
                    selectedNodes.value.add(node.id)
                }
            })
        }
        return
    }
    
    // 拖拽连接线
    if (isDraggingConnection.value) {
        event.preventDefault()
        const rect = getFlowchartContainerElement()?.getBoundingClientRect()
        if (rect) {
            const scale = zoomLevel.value
            
            // 检查是否有吸附目标
            if (hover_port.value && hover_port.value.type === 'input') {
                const node = nodes.value.find(n => n.id === hover_port.value!.nodeId)
                if (node) {
                    // 吸附到节点输入端口中心
                    // 注意：这里需要与 calculateConnectionPath 中的高度假设保持一致
                    const NODE_HEIGHT = 80
                    dragConnectionEnd.x = node.x
                    dragConnectionEnd.y = node.y + NODE_HEIGHT / 2
                } else {
                    // 节点未找到（异常情况），回退到鼠标位置
                    dragConnectionEnd.x = (event.clientX - rect.left - panOffset.x) / scale
                    dragConnectionEnd.y = (event.clientY - rect.top - panOffset.y) / scale
                }
            } else {
                // 无吸附目标，跟随鼠标
                dragConnectionEnd.x = (event.clientX - rect.left - panOffset.x) / scale
                dragConnectionEnd.y = (event.clientY - rect.top - panOffset.y) / scale
            }
            
            updateTempConnectionPath()
        }
        return
    }
    
    // 画布拖拽优先级最高
    if (isPanningCanvas.value) {
        event.preventDefault()
        panOffset.x = event.clientX - panStart.x
        panOffset.y = event.clientY - panStart.y
        return
    }
    
    // 多选节点整体拖拽
    if (isDraggingSelection.value) {
        event.preventDefault()
        
        const deltaX = Math.abs(event.clientX - drag_ctx.startX)
        const deltaY = Math.abs(event.clientY - drag_ctx.startY)
        if (deltaX > DRAG_THRESHOLD || deltaY > DRAG_THRESHOLD) {
            dragMoved.value = true
        }
        
        const localX = (event.clientX - drag_ctx.rect_left - panOffset.x) / drag_ctx.scale
        const localY = (event.clientY - drag_ctx.rect_top - panOffset.y) / drag_ctx.scale
        const dx = localX - selectionDragStart.x
        const dy = localY - selectionDragStart.y
        
        // 移动所有选中的节点
        selectedNodes.value.forEach(nodeId => {
            const node = nodes.value.find(n => n.id === nodeId)
            const startPos = nodeStartPositions.value.get(nodeId)
            if (node && startPos) {
                node.x = startPos.x + dx
                node.y = startPos.y + dy
            }
        })
        scheduleConnectionsUpdate()
        return
    }
    
    // 单节点拖拽
    if (isDragging.value && draggedNode.value) {
        event.preventDefault()
        
        // 计算移动距离，只有超过阈值才认为是拖拽
        const deltaX = Math.abs(event.clientX - drag_ctx.startX)
        const deltaY = Math.abs(event.clientY - drag_ctx.startY)
        if (deltaX > DRAG_THRESHOLD || deltaY > DRAG_THRESHOLD) {
            dragMoved.value = true
        }
        
        const localX = (event.clientX - drag_ctx.rect_left - panOffset.x) / drag_ctx.scale
        const localY = (event.clientY - drag_ctx.rect_top - panOffset.y) / drag_ctx.scale
        draggedNode.value.x = localX - dragOffset.x
        draggedNode.value.y = localY - dragOffset.y
        // 无限画布：不限制节点位置
        scheduleConnectionsUpdate()
    }
}

const on_pointer_up = (event: PointerEvent) => {
    // 框选结束
    if (isSelecting.value) {
        isSelecting.value = false
        return
    }
    
    // 拖拽连接结束 - 延迟处理以便端口的pointerup先触发
    if (isDraggingConnection.value) {
        setTimeout(() => {
            // 如果没有悬停在端口上，取消连接
            if (isDraggingConnection.value) {
                isDraggingConnection.value = false
                dragConnectionStart.value = null
                tempConnectionPath.value = ''
            }
        }, 50)
        return
    }
    
    if (isPanningCanvas.value) {
        isPanningCanvas.value = false
        return
    }
    
    // 多选拖拽结束
    if (isDraggingSelection.value) {
        if (dragMoved.value) {
            saveHistory()
            emit('change')
        }
        isDraggingSelection.value = false
        nodeStartPositions.value.clear()
        updateConnections()
        return
    }
    
    if (isDragging.value && dragMoved.value) {
        saveHistory()
        emit('change') // 节点位置变化
    } else if (isDragging.value && !dragMoved.value && draggedNode.value) {
        // 模拟点击事件：如果是拖拽状态但没有移动，说明是点击
        emit('nodeClick', draggedNode.value)
    }
    
    isDragging.value = false
    draggedNode.value = null
    // 注意：不在这里重置 dragMoved，因为 click 事件会在 pointerup 之后触发
    // dragMoved 会在 onNodeClick 中被重置
    updateConnections()
}

const onConnectionClick = (connection: FlowchartConnection) => {
    emit('connectionClick', connection)
}

const onConnectionContextMenu = (event: MouseEvent, connection: FlowchartConnection) => {
    event.stopPropagation()
    showConnectionContextMenu(connection, event)
}

// 更新容器尺寸
const updateContainerSize = () => {
    const containerEl = getFlowchartContainerElement()
    if (containerEl) {
        const rect = containerEl.getBoundingClientRect()
        viewportSize.width = rect.width
        viewportSize.height = rect.height
    }
}

// 生命周期
onMounted(() => {
    initializeFlowchart()
    updateContainerSize()
    window.addEventListener('resize', updateContainerSize)

    // 从其他入口触发自动打开 AI生成
    const flag = localStorage.getItem('open_ai_generate_workflow')
    if (flag === '1') {
        localStorage.removeItem('open_ai_generate_workflow')
        openAiGenerateModal()
    }
    
    // 使用事件代理处理节点右键菜单
    const handleContextMenu = (e: MouseEvent) => {
        e.preventDefault() // 阻止所有默认右键菜单
        
        // 查找是否点击在节点上
        const nodeElement = (e.target as HTMLElement).closest('.flowchart-node') as HTMLElement | null
        if (nodeElement) {
            const nodeId = nodeElement.dataset.nodeId
            if (nodeId) {
                const node = nodes.value.find(n => n.id === nodeId)
                if (node) {
                    console.log('Context menu for node:', node.id)
                    showNodeContextMenu(node, e)
                }
            }
        }
    }
    
    // 在流程图容器上监听右键事件
    const containerEl = getFlowchartContainerElement()
    if (containerEl) {
        containerEl.addEventListener('contextmenu', handleContextMenu)
    }
    
    const onKeyDown = (e: KeyboardEvent) => {
        // 空格键按下 - 启用画布拖拽模式
        if (e.code === 'Space' && !e.repeat) {
            const activeEl = document.activeElement
            const isInInput = activeEl && (
                activeEl.tagName === 'INPUT' || 
                activeEl.tagName === 'TEXTAREA' || 
                (activeEl as HTMLElement).isContentEditable
            )
            if (!isInInput) {
                e.preventDefault()
                isSpacePressed.value = true
            }
        }
        
        // ESC 关闭右键菜单或全屏
        if (e.key === 'Escape') {
            if (contextMenu.visible) {
                closeContextMenu()
            } else if (isFullscreen.value) {
                isFullscreen.value = false
                nextTick(() => updateContainerSize())
            }
        }
        
        // 检查焦点是否在输入框内，如果是则不拦截快捷键
        const activeEl = document.activeElement
        const isInInput = activeEl && (
            activeEl.tagName === 'INPUT' || 
            activeEl.tagName === 'TEXTAREA' || 
            (activeEl as HTMLElement).isContentEditable
        )
        
        // 撤销/重做快捷键
        if ((e.ctrlKey || e.metaKey) && e.key === 'z' && !e.shiftKey) {
            if (isInInput) return
            e.preventDefault()
            undo()
        }
        if ((e.ctrlKey || e.metaKey) && (e.key === 'y' || (e.key === 'z' && e.shiftKey))) {
            if (isInInput) return
            e.preventDefault()
            redo()
        }
        
        // 删除选中节点
        if ((e.key === 'Delete' || e.key === 'Backspace') && selectedNodes.value.size > 0) {
            if (isInInput) return
            e.preventDefault()
            deleteSelectedNodes()
        }
        
        // 全选
        if ((e.ctrlKey || e.metaKey) && e.key === 'a') {
            if (isInInput) return
            e.preventDefault()
            selectAllNodes()
        }
        
        // 适应视图快捷键
        if ((e.ctrlKey || e.metaKey) && e.key === '0') {
            if (isInInput) return
            e.preventDefault()
            fitToView()
        }
        
        // 重置视图快捷键
        if ((e.ctrlKey || e.metaKey) && e.key === '1') {
            if (isInInput) return
            e.preventDefault()
            resetCanvasView()
        }
    }
    
    const onKeyUp = (e: KeyboardEvent) => {
        // 空格键释放 - 退出画布拖拽模式
        if (e.code === 'Space') {
            isSpacePressed.value = false
            if (isPanningCanvas.value) {
                isPanningCanvas.value = false
            }
        }
    }
    
    // 点击其他地方关闭右键菜单
    const onClickOutside = () => {
        if (contextMenu.visible) {
            closeContextMenu()
        }
    }
    
    window.addEventListener('keydown', onKeyDown)
    window.addEventListener('keyup', onKeyUp)
    window.addEventListener('click', onClickOutside)
    onUnmounted(() => {
        window.removeEventListener('keydown', onKeyDown)
        window.removeEventListener('keyup', onKeyUp)
        window.removeEventListener('click', onClickOutside)
        const containerEl = getFlowchartContainerElement()
        if (containerEl) {
            containerEl.removeEventListener('contextmenu', handleContextMenu)
        }
    })
})

const deleteSelectedNodes = () => {
    if (selectedNodes.value.size === 0) return
    saveHistory()
    selectedNodes.value.forEach(nodeId => {
        nodes.value = nodes.value.filter(n => n.id !== nodeId)
        customEdges.value = customEdges.value.filter(e => e.from_node !== nodeId && e.to_node !== nodeId)
    })
    selectedNodes.value.clear()
    updateConnections()
    emit('change')
}

const selectAllNodes = () => {
    selectedNodes.value = new Set(nodes.value.map(n => n.id))
}

const setHoverPort = (nodeId: string, portId: string, type: 'input' | 'output') => {
    hover_port.value = { nodeId, portId, type }
}

const clearHoverPort = () => {
    hover_port.value = null
}

const get_node_icon = (node_type: string): string => getFlowchartNodeIcon(node_type)

// 检查是否有未保存的更改
const hasUnsavedChanges = (): boolean => {
    return nodes.value.length > 0 || customEdges.value.length > 0
}

// 新建工作流
const onNewWorkflow = () => {
    emit('newWorkflow')
}

// 开始拖拽连接
const start_drag_connection = (nodeId: string, portId: string, portType: 'input' | 'output', event: PointerEvent) => {
    // 只允许从输出端口开始拖拽
    if (portType === 'input') return
    
    event.preventDefault()
    event.stopPropagation()
    
    const node = nodes.value.find(n => n.id === nodeId)
    if (!node) return
    
    const NODE_WIDTH = 180
    const NODE_HEIGHT = 80
    
    isDraggingConnection.value = true
    dragConnectionStart.value = {
        nodeId,
        portId,
        portType,
        x: node.x + NODE_WIDTH, // 输出端口在右侧
        y: node.y + NODE_HEIGHT / 2
    }
    dragConnectionEnd.x = dragConnectionStart.value.x
    dragConnectionEnd.y = dragConnectionStart.value.y
    
    updateTempConnectionPath()
}

// 结束拖拽连接
const end_drag_connection = (targetNodeId: string, targetPortId: string, targetPortType: 'input' | 'output') => {
    if (!isDraggingConnection.value || !dragConnectionStart.value) return
    
    // 只能连接到输入端口
    if (targetPortType !== 'input') {
        isDraggingConnection.value = false
        dragConnectionStart.value = null
        tempConnectionPath.value = ''
        return
    }
    
    // 不能连接到自己
    if (dragConnectionStart.value.nodeId === targetNodeId) {
        isDraggingConnection.value = false
        dragConnectionStart.value = null
        tempConnectionPath.value = ''
        return
    }
    
    // 创建连接
    saveHistory()
    const target = nodes.value.find(n => n.id === targetNodeId)
    if (target) {
        if (!target.dependencies) target.dependencies = []
        if (!target.dependencies.includes(dragConnectionStart.value.nodeId)) {
            target.dependencies.push(dragConnectionStart.value.nodeId)
        }
    }
    
    const newEdge: EdgeDef = {
        id: createEdgeId(),
        from_node: dragConnectionStart.value.nodeId,
        to_node: targetNodeId,
        from_port: dragConnectionStart.value.portId,
        to_port: targetPortId,
        source_scope: 'output',
        target_path: '',
        merge_mode: 'replace',
    }
    customEdges.value.push(newEdge)
    
    updateConnections()
    const createdConnection = connections.value.find(connection => connection.edgeId === newEdge.id)
    if (createdConnection) {
        emit('connectionClick', createdConnection)
    }
    emit('change') // 连接创建
    
    isDraggingConnection.value = false
    dragConnectionStart.value = null
    tempConnectionPath.value = ''
}

// 更新临时连接线路径
const updateTempConnectionPath = () => {
    if (!dragConnectionStart.value) return
    
    const fromX = dragConnectionStart.value.x
    const fromY = dragConnectionStart.value.y
    const toX = dragConnectionEnd.x
    const toY = dragConnectionEnd.y
    
    // 贝塞尔曲线
    const dx = toX - fromX
    const controlOffset = Math.min(Math.abs(dx) * 0.5, 80)
    tempConnectionPath.value = `M ${fromX} ${fromY} C ${fromX + controlOffset} ${fromY}, ${toX - controlOffset} ${toY}, ${toX} ${toY}`
}

onUnmounted(() => {
    window.removeEventListener('resize', updateContainerSize)
})

// 监听props变化
watch(() => props.planData, (newPlan) => {
    if (newPlan) {
        // 根据实际计划数据更新流程图
        updateFlowchartFromPlan(newPlan)
    }
}, { deep: true })

const updateFlowchartFromPlan = (planData: any) => {
    // 根据实际的计划数据更新节点状态
    // 这里可以根据实际的API响应格式来实现
    console.log('Updating flowchart from plan data:', planData)
}

const openAiGenerateModal = () => {
    aiGenerateError.value = ''
    showAiGenerateModal.value = true
    nextTick(() => {
        const el = document.querySelector('.modal-open textarea') as HTMLTextAreaElement | null
        el?.focus()
    })
}

const closeAiGenerateModal = () => {
    showAiGenerateModal.value = false
    aiGenerateError.value = ''
}

const applyWorkflowGraph = (graph: any) => {
    if (!graph?.nodes || !Array.isArray(graph.nodes)) {
        throw new Error(t('trafficAnalysis.workflowStudio.flowchart.aiGenerate.missingNodesError'))
    }
    const deps: Record<string, string[]> = {}
    const edges = Array.isArray(graph.edges) ? graph.edges : []
    edges.forEach((e: any) => {
        if (!deps[e.to_node]) deps[e.to_node] = []
        deps[e.to_node].push(e.from_node)
    })

    nodes.value = graph.nodes.map((n: any) => ({
        id: n.id,
        name: n.node_name || n.node_type || n.id,
        description: n.params?.description || n.node_type || '',
        status: 'pending',
        x: typeof n.x === 'number' ? n.x : 80,
        y: typeof n.y === 'number' ? n.y : 80,
        type: n.node_type,
        dependencies: deps[n.id] || [],
        params: n.params || {},
        metadata: {
            input_ports: n.input_ports || [],
            output_ports: n.output_ports || [],
        },
    }))

    customEdges.value = edges.map((e: any) => ({
        id: e.id || createEdgeId(),
        from_node: e.from_node,
        to_node: e.to_node,
        from_port: e.from_port || 'out',
        to_port: e.to_port || 'in',
        source_scope: e.source_scope || 'output',
        source_path: e.source_path || '',
        target_path: e.target_path || '',
        merge_mode: e.merge_mode || 'replace',
    }))
    updateConnections()
    history.value = []
    historyIndex.value = -1
    saveHistory()
    emit('change')
}

const generateWorkflowFromNl = async () => {
    if (!aiGenerateText.value.trim()) return
    isAiGenerating.value = true
    aiGenerateError.value = ''
    try {
        const graph = await invoke<any>('generate_workflow_from_nl', { description: aiGenerateText.value.trim() })
        applyWorkflowGraph(graph)
        closeAiGenerateModal()
    } catch (e: any) {
        console.error('AI generate workflow failed:', e)
        aiGenerateError.value = e?.message || String(e)
    } finally {
        isAiGenerating.value = false
    }
}

// 暴露方法给父组件
defineExpose({
    updateNodeStatus: (nodeId: string, status: NodeStatus, progress?: number) => {
        const node = nodes.value.find(n => n.id === nodeId)
        if (node) {
            node.status = status
            if (progress !== undefined) {
                node.progress = progress
            }
            updateConnections()
        }
    },
    addNode: (node: FlowchartNode) => {
        saveHistory()
        nodes.value.push(node)
        updateConnections()
        emit('change')
    },
    removeNode: (nodeId: string) => {
        nodes.value = nodes.value.filter(n => n.id !== nodeId)
        updateConnections()
        emit('change')
    },
    addConnection: (fromId: string, toId: string) => {
        saveHistory()
        const target = nodes.value.find(n => n.id === toId)
        if (target) {
            if (!target.dependencies) target.dependencies = []
            if (!target.dependencies.includes(fromId)) {
                target.dependencies.push(fromId)
            }
        }
        customEdges.value.push({
            id: createEdgeId(),
            from_node: fromId,
            to_node: toId,
            from_port: 'out',
            to_port: 'in',
            source_scope: 'output',
            target_path: '',
            merge_mode: 'replace',
        })
        updateConnections()
        emit('change')
    },
    addConnectionWithPorts: (fromId: string, toId: string, fromPort: string, toPort: string) => {
        saveHistory()
        const target = nodes.value.find(n => n.id === toId)
        if (target) {
            if (!target.dependencies) target.dependencies = []
            if (!target.dependencies.includes(fromId)) {
                target.dependencies.push(fromId)
            }
        }
        customEdges.value.push({
            id: createEdgeId(),
            from_node: fromId,
            to_node: toId,
            from_port: fromPort,
            to_port: toPort,
            source_scope: 'output',
            target_path: '',
            merge_mode: 'replace',
        })
        updateConnections()
        emit('change')
    },
    addConnectionEdge: (edge: EdgeDef) => {
        const target = nodes.value.find(n => n.id === edge.to_node)
        if (target) {
            if (!target.dependencies) target.dependencies = []
            if (!target.dependencies.includes(edge.from_node)) {
                target.dependencies.push(edge.from_node)
            }
        }
        customEdges.value.push({
            id: edge.id || createEdgeId(),
            from_node: edge.from_node,
            to_node: edge.to_node,
            from_port: edge.from_port || 'out',
            to_port: edge.to_port || 'in',
            source_scope: edge.source_scope || 'output',
            source_path: edge.source_path || '',
            target_path: edge.target_path || '',
            merge_mode: edge.merge_mode || 'replace',
        })
        updateConnections()
        emit('change')
    },
    removeConnection: (fromId: string, toId: string) => {
        const target = nodes.value.find(n => n.id === toId)
        if (target && target.dependencies) {
            target.dependencies = target.dependencies.filter(d => d !== fromId)
            updateConnections()
            emit('change')
        }
    },
    updateNodeParams: (nodeId: string, params: Record<string, any>) => {
        const node = nodes.value.find(n => n.id === nodeId)
        if (node) {
            node.params = { ...params }
            emit('change')
        }
    },
    updateEdgeMapping: (edgeId: string, patch: Partial<EdgeDef>) => {
        const edge = customEdges.value.find(item => item.id === edgeId)
        if (!edge) return
        Object.assign(edge, patch)
        updateConnections()
        emit('change')
    },
    resetFlowchart: initializeFlowchart,
    getFlowchartNodes: (): FlowchartNode[] => {
        return [...nodes.value]
    },
    getFlowchartEdges: (): Array<{ from_node: string, to_node: string }> => {
        const nodeIds = new Set(nodes.value.map(n => n.id))
        if (customEdges.value.length) {
            // 过滤掉引用了不存在节点的无效边
            return customEdges.value
                .filter(e => nodeIds.has(e.from_node) && nodeIds.has(e.to_node))
                .map(e => ({ from_node: e.from_node, to_node: e.to_node }))
        }
        const edges: Array<{ from_node: string, to_node: string }> = []
        nodes.value.forEach(n => {
            n.dependencies.forEach(dep => {
                if (nodeIds.has(dep)) {
                    edges.push({ from_node: dep, to_node: n.id })
                }
            })
        })
        return edges
    },
    getFlowchartEdgesDetailed: (): EdgeDef[] => {
        const nodeIds = new Set(nodes.value.map(n => n.id))
        if (customEdges.value.length) {
            // 过滤掉引用了不存在节点的无效边
            return customEdges.value
                .filter(e => nodeIds.has(e.from_node) && nodeIds.has(e.to_node))
                .map(e => ({ ...e }))
        }
        const edges: EdgeDef[] = []
        nodes.value.forEach(n => {
            n.dependencies.forEach(dep => {
                if (nodeIds.has(dep)) {
                    edges.push({
                        id: createEdgeId(),
                        from_node: dep,
                        to_node: n.id,
                        from_port: 'out',
                        to_port: 'in',
                        source_scope: 'output',
                        target_path: '',
                        merge_mode: 'replace',
                    })
                }
            })
        })
        return edges
    },
    hasUnsavedChanges
})
</script>

<style scoped>
.flowchart-visualization {
    width: 100%;
}

.flowchart-container {
    user-select: none;
}

.fullscreen {
    position: fixed;
    inset: 0;
    z-index: 50;
    min-height: 100vh;
}

.flowchart-node {
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
}

.flowchart-node:hover {
    box-shadow: 0 4px 16px rgba(0, 0, 0, 0.15);
}

.line-clamp-2 {
    display: -webkit-box;
    -webkit-line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
    line-clamp: 2;
}
.flowchart-container {
    user-select: none;
}
.flowchart-node {
    will-change: transform;
}
/* 删除模式下连接线悬停效果 */
.stroke-transparent:hover {
    stroke: rgba(239, 68, 68, 0.3);
}
</style>
