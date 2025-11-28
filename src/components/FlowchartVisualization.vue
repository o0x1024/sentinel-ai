<template>
    <div class="flowchart-visualization">
        <!-- 工具栏 -->
        <div class="card bg-base-100 shadow-xl mb-4">
            <div class="card-body py-3">
                <div class="flex justify-between items-center">
                    <h3 class="card-title text-lg">执行流程图</h3>

                    <div class="flex gap-2">
                        <!-- 缩放控制 -->
                        <div class="join">
                            <button class="btn btn-sm join-item" @click="zoomOut">
                                <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24"
                                    stroke="currentColor">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                                        d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0zM13 10H7" />
                                </svg>
                            </button>
                            <button class="btn btn-sm join-item" @click="resetZoom">
                                {{ Math.round(zoomLevel * 100) }}%
                            </button>
                            <button class="btn btn-sm join-item" @click="zoomIn">
                                <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24"
                                    stroke="currentColor">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                                        d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0zM10 7v3m0 0v3m0-3h3m-3 0H7" />
                                </svg>
                            </button>
                        </div>

                        <!-- 全屏切换 -->
                        <button class="btn btn-sm btn-outline" @click="toggleFullscreen">
                            <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24"
                                stroke="currentColor">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                                    d="M4 8V4m0 0h4M4 4l5 5m11-1V4m0 0h-4m4 0l-5 5M4 16v4m0 0h4m-4 0l5-5m11 5l-5-5m5 5v-4m0 4h-4" />
                            </svg>
                        </button>

                        <!-- 重置视图 -->
                        <button class="btn btn-sm btn-outline" @click="resetView">
                            重置视图
                        </button>

                        <!-- 一键整理节点 -->
                        <button class="btn btn-sm btn-outline" @click="arrangeNodes" title="自动整理节点布局">
                            整理节点
                        </button>

                        <!-- 撤销/重做 -->
                        <div class="join">
                            <button class="btn btn-sm join-item" @click="undo" :disabled="!canUndo" title="撤销 (Ctrl+Z)">
                                <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 10h10a8 8 0 018 8v2M3 10l6 6m-6-6l6-6" />
                                </svg>
                            </button>
                            <button class="btn btn-sm join-item" @click="redo" :disabled="!canRedo" title="重做 (Ctrl+Y)">
                                <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 10h-10a8 8 0 00-8 8v2M21 10l-6 6m6-6l-6-6" />
                                </svg>
                            </button>
                        </div>

                        <!-- 删除连接 -->
                        <button class="btn btn-sm btn-outline" @click="toggleDeleteConnectionMode" :class="{ 'btn-error': deleteConnectionMode }" title="点击连接线删除">
                            <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
                            </svg>
                            删除连接
                        </button>
                    </div>
                </div>
            </div>
        </div>

        <!-- 流程图容器 -->
        <div class="card bg-base-100 shadow-xl relative" :class="{ 'fullscreen': isFullscreen }">

            <!-- 流程图容器 -->
            <div ref="flowchartContainer"
                class="flowchart-container bg-base-200 rounded-lg p-4 min-h-[80vh] relative overflow-auto"
                :class="{ 'cursor-grab': !isDragging && !isPanningCanvas, 'cursor-grabbing': isPanningCanvas }"
                @pointerdown="on_pointer_down" @pointermove="on_pointer_move" @pointerup="on_pointer_up">
                
                <!-- 空状态提示 -->
                <div v-if="nodes.length === 0" class="absolute inset-0 flex items-center justify-center pointer-events-none">
                    <div class="text-center text-base-content/40">
                        <svg xmlns="http://www.w3.org/2000/svg" class="h-16 w-16 mx-auto mb-4 opacity-30" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
                        </svg>
                        <p class="text-lg font-semibold mb-2">画布为空</p>
                        <p class="text-sm">从左侧节点库拖拽节点到这里开始创建工作流</p>
                        <p class="text-xs mt-2">提示：按住 Shift 键拖拽可以平移画布</p>
                    </div>
                </div>
                
                <div class="flowchart-content" :style="contentStyle">
                    <svg class="absolute inset-0 w-full h-full"
                        :viewBox="`0 0 ${containerSize.width} ${containerSize.height}`">
                        <defs>
                            <marker id="arrowhead" markerWidth="10" markerHeight="7" refX="9" refY="3.5" orient="auto">
                                <polygon points="0 0, 10 3.5, 0 7" class="fill-primary" />
                            </marker>
                        </defs>

                        <path v-for="connection in connections" :key="connection.id" :d="connection.path" :class="[
                            'stroke-2 fill-none',
                            getConnectionClass(connection)
                        ]" marker-end="url(#arrowhead)" @click="onConnectionClick(connection)" />
                        
                        <!-- 临时连接线 -->
                        <path v-if="isDraggingConnection && tempConnectionPath" 
                              :d="tempConnectionPath" 
                              class="stroke-2 fill-none stroke-primary stroke-dasharray-4 opacity-70"
                              marker-end="url(#arrowhead)" />
                    </svg>

                    <div v-for="node in nodes" :key="node.id" :class="[
                        'flowchart-node absolute',
                        node.id === draggedNode?.id ? 'cursor-grabbing duration-0' : 'cursor-pointer transition-all duration-200',
                        'border-2 rounded-lg p-3 min-w-[120px] max-w-[200px]',
                        selectedNodes.has(node.id) ? 'ring-2 ring-primary ring-offset-2' : '',
                        highlightedNodes.has(node.id) ? 'ring-2 ring-warning ring-offset-2 animate-pulse' : '',
                        getNodeClass(node)
                    ]" :style="{
                    transform: `translate3d(${node.x}px, ${node.y}px, 0) ${node.id === draggedNode?.id ? 'scale(1.05)' : 'scale(1)'}`
                }" @pointerdown="on_node_pointer_down($event, node)" @click="onNodeClick(node)" @contextmenu="onNodeContextMenu($event, node)" @mouseenter="onNodeEnter(node)" @mouseleave="onNodeLeave(node)">
                    <!-- 输入端口 -->
                    <div class="absolute left-0 top-1/2 -translate-y-1/2 -translate-x-1/2 flex flex-col gap-1">
                        <div 
                            v-for="port in node.metadata?.input_ports || [{id: 'in', name: '输入'}]" 
                            :key="port.id"
                            class="port port-input w-3 h-3 rounded-full bg-primary border-2 border-white cursor-pointer hover:scale-125 transition-transform"
                            :class="{ 'ring-2 ring-success': isDraggingConnection && hover_port?.nodeId === node.id && hover_port?.portId === port.id }"
                            :title="port.name"
                            @pointerup.stop="end_drag_connection(node.id, port.id, 'input')"
                            @pointerenter="hover_port = { nodeId: node.id, portId: port.id, type: 'input' }"
                            @pointerleave="hover_port = null"
                        ></div>
                    </div>
                    
                    <!-- 断点标记 -->
                    <div v-if="breakpoints.has(node.id)" class="absolute -top-2 -left-2 w-4 h-4 rounded-full bg-error flex items-center justify-center z-10" title="断点">
                        <svg xmlns="http://www.w3.org/2000/svg" class="h-3 w-3 text-white" fill="currentColor" viewBox="0 0 24 24">
                            <circle cx="12" cy="12" r="10" />
                        </svg>
                    </div>
                    
                    <!-- 节点图标和状态 -->
                    <div class="flex items-center gap-2 mb-2">
                        <div :class="['w-4 h-4 rounded-full flex items-center justify-center', getStatusIndicatorClass(node.status)]">
                            <svg v-if="node.status === 'completed'" xmlns="http://www.w3.org/2000/svg" class="h-3 w-3 text-white" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="3" d="M5 13l4 4L19 7" />
                            </svg>
                            <svg v-else-if="node.status === 'failed'" xmlns="http://www.w3.org/2000/svg" class="h-3 w-3 text-white" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="3" d="M6 18L18 6M6 6l12 12" />
                            </svg>
                            <svg v-else-if="node.status === 'paused'" xmlns="http://www.w3.org/2000/svg" class="h-3 w-3 text-white" fill="currentColor" viewBox="0 0 24 24">
                                <path d="M6 4h4v16H6V4zm8 0h4v16h-4V4z"/>
                            </svg>
                        </div>
                        <span class="font-semibold text-sm truncate flex-1">{{ node.name }}</span>
                        <span v-if="get_node_icon(node.type)" class="text-lg" :title="node.type">{{ get_node_icon(node.type) }}</span>
                    </div>
                    
                    <!-- 输出端口 -->
                    <div class="absolute right-0 top-1/2 -translate-y-1/2 translate-x-1/2 flex flex-col gap-1">
                        <div 
                            v-for="port in node.metadata?.output_ports || [{id: 'out', name: '输出'}]" 
                            :key="port.id"
                            class="port port-output w-3 h-3 rounded-full bg-secondary border-2 border-white cursor-pointer hover:scale-125 transition-transform"
                            :class="{ 'ring-2 ring-success': isDraggingConnection && dragConnectionStart?.nodeId === node.id && dragConnectionStart?.portId === port.id }"
                            :title="port.name"
                            @pointerdown.stop="start_drag_connection(node.id, port.id, 'output', $event)"
                            @pointerenter="hover_port = { nodeId: node.id, portId: port.id, type: 'output' }"
                            @pointerleave="hover_port = null"
                        ></div>
                    </div>

                    <!-- 节点描述 -->
                    <div class="text-xs text-base-content/70 mb-2 line-clamp-2">
                        {{ node.description }}
                    </div>

                    <!-- 节点状态信息 -->
                    <div class="flex justify-between items-center text-xs">
                        <span :class="['badge badge-xs', getStatusBadgeClass(node.status)]">
                            {{ getStatusText(node.status) }}
                        </span>
                        <span v-if="node.progress !== undefined" class="text-base-content/60">
                            {{ Math.round(node.progress) }}%
                        </span>
                    </div>

                    <!-- 进度条 -->
                    <div v-if="node.progress !== undefined && node.status === 'running'" class="mt-2">
                        <progress class="progress progress-primary w-full h-1" :value="node.progress"
                            max="100"></progress>
                    </div>
                </div>
            </div>


            </div>
            <button v-if="isFullscreen" class="btn btn-sm btn-outline absolute top-2 right-2" @click="toggleFullscreen">退出全屏</button>
        </div>

        <!-- 右键菜单 -->
        <div v-if="contextMenu.visible" 
            class="fixed z-50 bg-base-100 shadow-xl rounded-lg border border-base-300 py-1 min-w-[160px]"
            :style="{ left: contextMenu.x + 'px', top: contextMenu.y + 'px' }">
            <div v-for="(item, index) in contextMenu.items" :key="index"
                class="px-4 py-2 hover:bg-base-200 cursor-pointer text-sm transition-colors"
                :class="{ 'text-error': item.danger }"
                @click="handleContextMenuClick(item)">
                {{ item.label }}
            </div>
        </div>
    </div>
</template>

<script setup lang="ts">
import { ref, reactive, computed, onMounted, onUnmounted, watch, nextTick } from 'vue'
import type { CSSProperties } from 'vue'

// 节点状态枚举
type NodeStatus = 'pending' | 'planning' | 'running' | 'completed' | 'failed' | 'paused' | 'cancelled'

// 节点类型
    interface FlowchartNode {
        id: string
        name: string
        description: string
        status: NodeStatus
        progress?: number
        x: number
        y: number
        type: string
        dependencies: string[]
        params?: Record<string, any>
        metadata?: Record<string, any>
    }

// 连接线类型
interface FlowchartConnection {
    id: string
    from: string
    to: string
    path: string
    status: 'inactive' | 'active' | 'completed' | 'failed'
    condition?: string
}

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
}>()

// 响应式数据
const flowchartContainer = ref<HTMLElement>()
const nodes = ref<FlowchartNode[]>([])
const connections = ref<FlowchartConnection[]>([])
const deleteConnectionMode = ref(false)
const customEdges = ref<Array<{ from_node: string, to_node: string, from_port: string, to_port: string }>>([])
const autoLayout = ref(false)
const draggedNode = ref<FlowchartNode | null>(null)
const isDragging = ref(false)
const dragMoved = ref(false)
const dragOffset = reactive({ x: 0, y: 0 })
const isFullscreen = ref(false)
const zoomLevel = ref(1)
// 已移除布局模式，保留自由拖拽

// 画布拖拽
const isPanningCanvas = ref(false)
const panStart = reactive({ x: 0, y: 0 })
const panOffset = reactive({ x: 0, y: 0 })

// 多选功能
const selectedNodes = ref<Set<string>>(new Set())
const isSelecting = ref(false)
const selectionBox = reactive({ startX: 0, startY: 0, endX: 0, endY: 0 })

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

// 撤销/重做
interface HistoryState {
  nodes: FlowchartNode[]
  edges: Array<{ from_node: string, to_node: string, from_port: string, to_port: string }>
}
const history = ref<HistoryState[]>([])
const historyIndex = ref(-1)
const MAX_HISTORY = 50

const canUndo = computed(() => historyIndex.value > 0)
const canRedo = computed(() => historyIndex.value < history.value.length - 1)

const containerSize = reactive({
    width: 800,
    height: 600
})
const viewportSize = reactive({
    width: 800,
    height: 600
})
const viewportScale = computed(() => {
    const sx = viewportSize.width / containerSize.width
    const sy = viewportSize.height / containerSize.height
    return Math.min(sx, sy)
})

const contentStyle = computed<CSSProperties>(() => ({
    transform: `translate(${panOffset.x}px, ${panOffset.y}px) scale(${zoomLevel.value * viewportScale.value})`,
    transformOrigin: 'top left',
    width: containerSize.width + 'px',
    height: containerSize.height + 'px',
    position: 'relative',
    transition: isPanningCanvas.value ? 'none' : 'transform 0.1s ease-out'
}))

const saveHistory = () => {
  const state: HistoryState = {
    nodes: JSON.parse(JSON.stringify(nodes.value)),
    edges: JSON.parse(JSON.stringify(customEdges.value))
  }
  
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
  const state = history.value[historyIndex.value]
  nodes.value = JSON.parse(JSON.stringify(state.nodes))
  customEdges.value = JSON.parse(JSON.stringify(state.edges))
  updateConnections()
}

// 计算属性
    const getNodeClass = computed(() => (node: FlowchartNode) => {
        const baseClasses = ['bg-base-100', 'hover:shadow-lg']
        
        // 根据节点类型添加特殊样式
        const typeClasses = []
        // 通用样式，不根据架构
        
        switch (node.status) {
            case 'pending':
                return [...baseClasses, ...typeClasses, 'border-gray-300', 'text-base-content/70']
            case 'planning':
                return [...baseClasses, ...typeClasses, 'border-blue-400', 'bg-blue-50']
            case 'running':
                return [...baseClasses, ...typeClasses, 'border-yellow-400', 'bg-yellow-50', 'animate-pulse']
            case 'completed':
                return [...baseClasses, ...typeClasses, 'border-green-400', 'bg-green-50']
            case 'failed':
                return [...baseClasses, ...typeClasses, 'border-red-400', 'bg-red-50']
            case 'paused':
                return [...baseClasses, ...typeClasses, 'border-orange-400', 'bg-orange-50']
            default:
                return [...baseClasses, ...typeClasses]
        }
    })

// 已移除 fromPortOptions 和 toPortOptions - 使用拖拽连接代替

const getStatusIndicatorClass = computed(() => (status: NodeStatus) => {
    switch (status) {
        case 'pending': return 'bg-gray-400'
        case 'planning': return 'bg-blue-400 animate-pulse'
        case 'running': return 'bg-yellow-400 animate-pulse'
        case 'completed': return 'bg-green-400'
        case 'failed': return 'bg-red-400'
        case 'paused': return 'bg-orange-400'
        default: return 'bg-gray-400'
    }
})

const getStatusBadgeClass = computed(() => (status: NodeStatus) => {
    switch (status) {
        case 'pending': return 'badge-ghost'
        case 'planning': return 'badge-info'
        case 'running': return 'badge-warning'
        case 'completed': return 'badge-success'
        case 'failed': return 'badge-error'
        case 'paused': return 'badge-warning'
        default: return 'badge-ghost'
    }
})

const getConnectionClass = computed(() => (connection: FlowchartConnection) => {
    switch (connection.status) {
        case 'active': return 'stroke-yellow-400'
        case 'completed': return 'stroke-green-400'
        case 'failed': return 'stroke-red-400'
        default: return 'stroke-gray-300'
    }
})

// 方法
const getStatusText = (status: NodeStatus): string => {
    const statusMap = {
        pending: '待执行',
        planning: '规划中',
        running: '执行中',
        completed: '已完成',
        failed: '失败',
        paused: '已暂停',
        cancelled: '已取消'
    }
    return statusMap[status] || status
}

const initializeFlowchart = () => {
    nodes.value = []
    connections.value = []
}

const onNodeClick = (node: FlowchartNode, event?: MouseEvent) => {
    if (dragMoved.value) { dragMoved.value = false; return }
    emit('nodeClick', node)
}

// 右键菜单事件处理
const onNodeContextMenu = (event: MouseEvent, node: FlowchartNode) => {
    event.preventDefault()
    event.stopPropagation()
    showNodeContextMenu(node, event)
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
            label: hasBreakpoint ? '移除断点' : '添加断点', 
            action: () => toggleBreakpoint(node.id) 
        },
        { 
            label: '复制节点', 
            action: () => duplicateNode(node) 
        },
        { 
            label: '删除节点', 
            action: () => removeNode(node.id),
            danger: true
        }
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
        name: `${node.name} (副本)`
    }
    saveHistory()
    nodes.value.push(newNode)
    updateConnections()
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
}

const onNodeEnter = (node: FlowchartNode) => {
    // Node hover handling - reserved for future use
}

const onNodeLeave = (_node: FlowchartNode) => {
    // Node hover handling - reserved for future use
}

const updateConnections = () => {
    const newConnections: FlowchartConnection[] = []
    customEdges.value.forEach(edge => {
        const fromNode = nodes.value.find(n => n.id === edge.from_node)
        const toNode = nodes.value.find(n => n.id === edge.to_node)
        if (fromNode && toNode) {
            newConnections.push({
                id: `${edge.from_node}-${edge.to_node}-${edge.from_port}-${edge.to_port}`,
                from: edge.from_node,
                to: edge.to_node,
                path: calculateConnectionPath(fromNode, toNode),
                status: getConnectionStatus(fromNode, toNode)
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
                conn.path = calculateConnectionPath(fromNode, toNode)
                conn.status = getConnectionStatus(fromNode, toNode)
            }
        }
    }
}

const calculateConnectionPath = (from: FlowchartNode, to: FlowchartNode, curved = false): string => {
    const fromX = from.x + 100 // 节点宽度的一半
    const fromY = from.y + 40  // 节点高度的一半
    const toX = to.x + 100
    const toY = to.y + 40

    if (curved) {
        // 曲线连接（用于循环）
        const midX = (fromX + toX) / 2 + 100
        const midY = Math.min(fromY, toY) - 50
        return `M ${fromX} ${fromY} Q ${midX} ${midY} ${toX} ${toY}`
    } else {
        // 直线连接
        return `M ${fromX} ${fromY} L ${toX} ${toY}`
    }
}

const getConnectionStatus = (from: FlowchartNode, to: FlowchartNode): 'inactive' | 'active' | 'completed' | 'failed' => {
    if (from.status === 'failed' || to.status === 'failed') {
        return 'failed'
    }
    if (from.status === 'completed' && to.status === 'running') {
        return 'active'
    }
    if (from.status === 'completed' && to.status === 'completed') {
        return 'completed'
    }
    return 'inactive'
}


const resetView = () => {
    initializeFlowchart()
}

const arrangeNodes = () => {
    const list = nodes.value
    if (!list.length) return

    const level_map: Record<string, number> = {}

    const compute_level = (n: FlowchartNode, seen: Set<string>): number => {
        if (level_map[n.id] !== undefined) return level_map[n.id]
        if (!n.dependencies || n.dependencies.length === 0) {
            level_map[n.id] = 0
            return 0
        }
        if (seen.has(n.id)) {
            level_map[n.id] = 0
            return 0
        }
        seen.add(n.id)
        let max_dep = 0
        n.dependencies.forEach(dep_id => {
            const dep = list.find(x => x.id === dep_id)
            if (dep) {
                const lvl = compute_level(dep, seen)
                if (lvl > max_dep) max_dep = lvl
            }
        })
        level_map[n.id] = max_dep + 1
        return level_map[n.id]
    }

    list.forEach(n => compute_level(n, new Set()))

    const grouped: Record<number, FlowchartNode[]> = {}
    Object.keys(level_map).forEach(id => {
        const lvl = level_map[id]
        const node = list.find(x => x.id === id)
        if (node) {
            if (!grouped[lvl]) grouped[lvl] = []
            grouped[lvl].push(node)
        }
    })

    const levels = Object.keys(grouped).map(x => parseInt(x, 10)).sort((a, b) => a - b)
    const h_spacing = 200
    const v_spacing = 140
    const start_x = 50
    const start_y = 50

    levels.forEach((lvl, li) => {
        const row = grouped[lvl]
        row.forEach((node, idx) => {
            node.x = start_x + idx * h_spacing
            node.y = start_y + li * v_spacing
        })
    })

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

const toggleDeleteConnectionMode = () => {
    deleteConnectionMode.value = !deleteConnectionMode.value
}

// 拖拽功能
const drag_ctx = reactive({ rect_left: 0, rect_top: 0, scale: 1 })

const on_node_pointer_down = (event: PointerEvent, node: FlowchartNode) => {
    event.stopPropagation() // 阻止事件冒泡到画布
    event.preventDefault()
    dragMoved.value = false
    
    // 如果是Shift+点击，不拖拽节点，而是画布平移
    if (event.shiftKey) {
        return
    }
    
    draggedNode.value = node
    isDragging.value = true
    const rect = flowchartContainer.value?.getBoundingClientRect()
    if (rect) {
        drag_ctx.rect_left = rect.left
        drag_ctx.rect_top = rect.top
        drag_ctx.scale = zoomLevel.value * viewportScale.value
        const localX = (event.clientX - drag_ctx.rect_left) / drag_ctx.scale
        const localY = (event.clientY - drag_ctx.rect_top) / drag_ctx.scale
        dragOffset.x = localX - node.x
        dragOffset.y = localY - node.y
    }
    if (flowchartContainer.value && (flowchartContainer.value as any).setPointerCapture) {
        (flowchartContainer.value as any).setPointerCapture(event.pointerId)
    }
}

const on_pointer_down = (event: PointerEvent) => {
    if (event.target === flowchartContainer.value || (event.target as HTMLElement).closest('.flowchart-content')) {
        // 空白区域：开始画布拖拽（按住空格键或中键）
        if (event.button === 1 || (event.button === 0 && event.shiftKey)) {
            event.preventDefault()
            isPanningCanvas.value = true
            panStart.x = event.clientX - panOffset.x
            panStart.y = event.clientY - panOffset.y
        }
        draggedNode.value = null
    }
    if (flowchartContainer.value && (flowchartContainer.value as any).setPointerCapture) {
        (flowchartContainer.value as any).setPointerCapture(event.pointerId)
    }
}

const on_pointer_move = (event: PointerEvent) => {
    // 拖拽连接线
    if (isDraggingConnection.value) {
        event.preventDefault()
        const rect = flowchartContainer.value?.getBoundingClientRect()
        if (rect) {
            const scale = zoomLevel.value * viewportScale.value
            dragConnectionEnd.x = (event.clientX - rect.left) / scale
            dragConnectionEnd.y = (event.clientY - rect.top) / scale
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
    
    // 节点拖拽
    if (isDragging.value && draggedNode.value) {
        event.preventDefault()
        dragMoved.value = true
        const localX = (event.clientX - drag_ctx.rect_left) / drag_ctx.scale
        const localY = (event.clientY - drag_ctx.rect_top) / drag_ctx.scale
        draggedNode.value.x = localX - dragOffset.x
        draggedNode.value.y = localY - dragOffset.y
        draggedNode.value.x = Math.max(0, Math.min(draggedNode.value.x, containerSize.width - 200))
        draggedNode.value.y = Math.max(0, Math.min(draggedNode.value.y, containerSize.height - 100))
        scheduleConnectionsUpdate()
    }
}

const on_pointer_up = (event: PointerEvent) => {
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
    
    if (isDragging.value && dragMoved.value) {
        saveHistory()
    }
    
    isDragging.value = false
    draggedNode.value = null
    updateConnections()
}

const onConnectionClick = (connection: FlowchartConnection) => {
    if (deleteConnectionMode.value) {
        saveHistory()
        customEdges.value = customEdges.value.filter(e => !(e.from_node === connection.from && e.to_node === connection.to))
        const target = nodes.value.find(n => n.id === connection.to)
        if (target && target.dependencies) {
            target.dependencies = target.dependencies.filter(d => d !== connection.from)
        }
        updateConnections()
        emit('connectionClick', connection)
    } else {
        emit('connectionClick', connection)
    }
}

// 更新容器尺寸
const updateContainerSize = () => {
    if (flowchartContainer.value) {
        const rect = flowchartContainer.value.getBoundingClientRect()
        viewportSize.width = rect.width
        viewportSize.height = rect.height
    }
}

// 生命周期
onMounted(() => {
    initializeFlowchart()
    updateContainerSize()
    window.addEventListener('resize', updateContainerSize)
    
    const onKeyDown = (e: KeyboardEvent) => {
        // ESC 关闭右键菜单或全屏
        if (e.key === 'Escape') {
            if (contextMenu.visible) {
                closeContextMenu()
            } else if (isFullscreen.value) {
                isFullscreen.value = false
                nextTick(() => updateContainerSize())
            }
        }
        
        // 撤销/重做快捷键
        if ((e.ctrlKey || e.metaKey) && e.key === 'z' && !e.shiftKey) {
            e.preventDefault()
            undo()
        }
        if ((e.ctrlKey || e.metaKey) && (e.key === 'y' || (e.key === 'z' && e.shiftKey))) {
            e.preventDefault()
            redo()
        }
        
        // 删除选中节点
        if ((e.key === 'Delete' || e.key === 'Backspace') && selectedNodes.value.size > 0) {
            e.preventDefault()
            deleteSelectedNodes()
        }
        
        // 全选
        if ((e.ctrlKey || e.metaKey) && e.key === 'a') {
            e.preventDefault()
            selectAllNodes()
        }
    }
    
    // 点击其他地方关闭右键菜单
    const onClickOutside = () => {
        if (contextMenu.visible) {
            closeContextMenu()
        }
    }
    
    window.addEventListener('keydown', onKeyDown)
    window.addEventListener('click', onClickOutside)
    onUnmounted(() => {
        window.removeEventListener('keydown', onKeyDown)
        window.removeEventListener('click', onClickOutside)
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
}

const selectAllNodes = () => {
    selectedNodes.value = new Set(nodes.value.map(n => n.id))
}

const get_node_icon = (node_type: string): string => {
    // 根据节点类型返回emoji图标
    if (node_type.startsWith('tool::')) return '🔧'
    if (node_type === 'branch') return '🔀'
    if (node_type === 'merge') return '🔗'
    if (node_type === 'retry') return '🔄'
    if (node_type.startsWith('rag::')) return '📚'
    if (node_type.startsWith('prompt::')) return '💬'
    if (node_type === 'trigger') return '⚡'
    if (node_type === 'output') return '📤'
    return ''
}

// 开始拖拽连接
const start_drag_connection = (nodeId: string, portId: string, portType: 'input' | 'output', event: PointerEvent) => {
    // 只允许从输出端口开始拖拽
    if (portType === 'input') return
    
    event.preventDefault()
    event.stopPropagation()
    
    const node = nodes.value.find(n => n.id === nodeId)
    if (!node) return
    
    isDraggingConnection.value = true
    dragConnectionStart.value = {
        nodeId,
        portId,
        portType,
        x: node.x + 100, // 节点中心
        y: node.y + 40
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
    
    customEdges.value.push({
        from_node: dragConnectionStart.value.nodeId,
        to_node: targetNodeId,
        from_port: dragConnectionStart.value.portId,
        to_port: targetPortId
    })
    
    updateConnections()
    
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
    
    tempConnectionPath.value = `M ${fromX} ${fromY} L ${toX} ${toY}`
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
    },
    removeNode: (nodeId: string) => {
        nodes.value = nodes.value.filter(n => n.id !== nodeId)
        updateConnections()
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
        customEdges.value.push({ from_node: fromId, to_node: toId, from_port: 'out', to_port: 'in' })
        updateConnections()
    },
    addConnectionWithPorts: (fromId: string, toId: string, fromPort: string, toPort: string) => {
        const target = nodes.value.find(n => n.id === toId)
        if (target) {
            if (!target.dependencies) target.dependencies = []
            if (!target.dependencies.includes(fromId)) {
                target.dependencies.push(fromId)
            }
        }
        customEdges.value.push({ from_node: fromId, to_node: toId, from_port: fromPort, to_port: toPort })
        updateConnections()
    },
    removeConnection: (fromId: string, toId: string) => {
        const target = nodes.value.find(n => n.id === toId)
        if (target && target.dependencies) {
            target.dependencies = target.dependencies.filter(d => d !== fromId)
            updateConnections()
        }
    },
    updateNodeParams: (nodeId: string, params: Record<string, any>) => {
        const node = nodes.value.find(n => n.id === nodeId)
        if (node) {
            node.params = { ...params }
        }
    },
    resetFlowchart: initializeFlowchart,
    getFlowchartNodes: (): FlowchartNode[] => {
        return [...nodes.value]
    },
    getFlowchartEdges: (): Array<{ from_node: string, to_node: string }> => {
        if (customEdges.value.length) {
            return customEdges.value.map(e => ({ from_node: e.from_node, to_node: e.to_node }))
        }
        const edges: Array<{ from_node: string, to_node: string }> = []
        nodes.value.forEach(n => {
            n.dependencies.forEach(dep => edges.push({ from_node: dep, to_node: n.id }))
        })
        return edges
    },
    getFlowchartEdgesDetailed: (): Array<{ from_node: string, to_node: string, from_port: string, to_port: string }> => {
        if (customEdges.value.length) {
            return customEdges.value.map(e => ({ ...e }))
        }
        const edges: Array<{ from_node: string, to_node: string, from_port: string, to_port: string }> = []
        nodes.value.forEach(n => {
            n.dependencies.forEach(dep => edges.push({ from_node: dep, to_node: n.id, from_port: 'out', to_port: 'in' }))
        })
        return edges
    }
})
</script>

<style scoped>
.flowchart-visualization {
    @apply w-full;
}

.flowchart-container {
    user-select: none;
}

.fullscreen {
    @apply fixed inset-0 z-50;
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
}
.flowchart-container {
    user-select: none;
}
.flowchart-node {
    will-change: transform;
}
</style>
