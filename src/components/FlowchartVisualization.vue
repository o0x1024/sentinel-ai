<template>
    <div class="flowchart-visualization">
        <!-- 工具栏 -->
        <div class="card bg-base-100 shadow-xl mb-4">
            <div class="card-body py-3">
                <div class="flex justify-between items-center">
                    <h3 class="card-title text-lg">执行流程图</h3>

                    <div class="flex gap-2">
                        <!-- 布局控制 -->
                        <div class="dropdown dropdown-end">
                            <div tabindex="0" role="button" class="btn btn-sm btn-outline">
                                <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24"
                                    stroke="currentColor">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                                        d="M4 6h16M4 12h16M4 18h16" />
                                </svg>
                                布局
                            </div>
                            <ul tabindex="0"
                                class="dropdown-content z-[1] menu p-2 shadow bg-base-100 rounded-box w-32">
                                <li><a @click="setLayout('auto')">自动布局</a></li>
                                <li><a @click="setLayout('horizontal')">水平布局</a></li>
                                <li><a @click="setLayout('vertical')">垂直布局</a></li>
                                <li><a @click="setLayout('circular')">环形布局</a></li>
                            </ul>
                        </div>

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
                    </div>
                </div>
            </div>
        </div>

        <!-- 流程图容器 -->
        <div class="card bg-base-100 shadow-xl" :class="{ 'fullscreen': isFullscreen }">

            <!-- 流程图容器 -->
            <div ref="flowchartContainer"
                class="flowchart-container bg-base-200 rounded-lg p-4 min-h-[400px] relative overflow-auto"
                @mousedown="onCanvasMouseDown" @mousemove="onCanvasMouseMove" @mouseup="onCanvasMouseUp">
                <!-- 连接线 -->
                <svg class="absolute inset-0 w-full h-full pointer-events-none"
                    :viewBox="`0 0 ${containerSize.width} ${containerSize.height}`">
                    <defs>
                        <marker id="arrowhead" markerWidth="10" markerHeight="7" refX="9" refY="3.5" orient="auto">
                            <polygon points="0 0, 10 3.5, 0 7" class="fill-primary" />
                        </marker>
                    </defs>

                    <path v-for="connection in connections" :key="connection.id" :d="connection.path" :class="[
                        'stroke-2 fill-none',
                        getConnectionClass(connection)
                    ]" marker-end="url(#arrowhead)" />
                </svg>

                <!-- 流程节点 -->
                <div v-for="node in nodes" :key="node.id" :class="[
                    'flowchart-node absolute cursor-pointer transition-all duration-200',
                    'border-2 rounded-lg p-3 min-w-[120px] max-w-[200px]',
                    getNodeClass(node)
                ]" :style="{
                left: node.x + 'px',
                top: node.y + 'px',
                transform: node.id === draggedNode?.id ? 'scale(1.05)' : 'scale(1)'
            }" @mousedown="onNodeMouseDown($event, node)">
                    <!-- 节点图标 -->
                    <div class="flex items-center gap-2 mb-2">
                        <div :class="['w-3 h-3 rounded-full', getStatusIndicatorClass(node.status)]"></div>
                        <span class="font-semibold text-sm truncate">{{ node.name }}</span>
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

            <!-- 图例 -->
            <div class="mt-4 p-3 bg-base-200 rounded-lg">
                <h4 class="font-semibold text-sm mb-3">状态图例</h4>
                
                <!-- 执行状态 -->
                <div class="mb-3">
                    <h5 class="font-medium text-xs mb-2 text-base-content/80">执行状态</h5>
                    <div class="flex flex-wrap gap-4 text-xs">
                        <div class="flex items-center gap-1">
                            <div class="w-3 h-3 rounded-full bg-gray-400"></div>
                            <span>待执行</span>
                        </div>
                        <div class="flex items-center gap-1">
                            <div class="w-3 h-3 rounded-full bg-blue-400"></div>
                            <span>规划中</span>
                        </div>
                        <div class="flex items-center gap-1">
                            <div class="w-3 h-3 rounded-full bg-yellow-400"></div>
                            <span>执行中</span>
                        </div>
                        <div class="flex items-center gap-1">
                            <div class="w-3 h-3 rounded-full bg-green-400"></div>
                            <span>已完成</span>
                        </div>
                        <div class="flex items-center gap-1">
                            <div class="w-3 h-3 rounded-full bg-red-400"></div>
                            <span>失败</span>
                        </div>
                        <div class="flex items-center gap-1">
                            <div class="w-3 h-3 rounded-full bg-orange-400"></div>
                            <span>已暂停</span>
                        </div>
                    </div>
                </div>
                
                <!-- 架构类型 -->
                <div class="mb-3">
                    <h5 class="font-medium text-xs mb-2 text-base-content/80">架构类型</h5>
                    <div class="flex flex-wrap gap-4 text-xs">
                        <div class="flex items-center gap-1">
                            <div class="w-4 h-3 bg-base-100 border border-l-4 border-l-purple-500"></div>
                            <span>ReWOO</span>
                        </div>
                        <div class="flex items-center gap-1">
                            <div class="w-4 h-3 bg-base-100 border border-l-4 border-l-indigo-500"></div>
                            <span>LLMCompiler</span>
                        </div>
                        <div class="flex items-center gap-1">
                            <div class="w-4 h-3 bg-base-100 border border-gray-300"></div>
                            <span>Plan-Execute</span>
                        </div>
                    </div>
                </div>
                
                <!-- 节点类型 -->
                <div>
                    <h5 class="font-medium text-xs mb-2 text-base-content/80">节点类型</h5>
                    <div class="grid grid-cols-2 gap-2 text-xs">
                        <div class="flex items-center gap-1">
                            <div class="w-3 h-3 bg-purple-200 border border-purple-300"></div>
                            <span>ReWOO规划器</span>
                        </div>
                        <div class="flex items-center gap-1">
                            <div class="w-3 h-3 bg-blue-200 border border-blue-300"></div>
                            <span>ReWOO工作器</span>
                        </div>
                        <div class="flex items-center gap-1">
                            <div class="w-3 h-3 bg-green-200 border border-green-300"></div>
                            <span>ReWOO求解器</span>
                        </div>
                        <div class="flex items-center gap-1">
                            <div class="w-3 h-3 bg-indigo-200 border border-indigo-300"></div>
                            <span>DAG调度器</span>
                        </div>
                        <div class="flex items-center gap-1">
                            <div class="w-3 h-3 bg-teal-200 border border-teal-300"></div>
                            <span>并行执行器</span>
                        </div>
                        <div class="flex items-center gap-1">
                            <div class="w-3 h-3 bg-orange-200 border border-orange-300"></div>
                            <span>智能连接器</span>
                        </div>
                    </div>
                </div>
            </div>
        </div>
    </div>
</template>

<script setup lang="ts">
import { ref, reactive, computed, onMounted, onUnmounted, watch, nextTick } from 'vue'

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
    type: 'start' | 'planner' | 'agent' | 'tools' | 'replan' | 'end' | 
          'rewoo_planner' | 'rewoo_worker' | 'rewoo_solver' | 'rewoo_variable' |
          'dag_scheduler' | 'task_fetcher' | 'parallel_executor' | 'joiner' | 'dependency_resolver'
    dependencies: string[]
    metadata?: Record<string, any>
    architecture?: 'plan_execute' | 'rewoo' | 'llm_compiler'
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
}

const props = withDefaults(defineProps<Props>(), {
    realTimeUpdates: true
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
const autoLayout = ref(true)
const draggedNode = ref<FlowchartNode | null>(null)
const isDragging = ref(false)
const dragOffset = reactive({ x: 0, y: 0 })
const isFullscreen = ref(false)
const zoomLevel = ref(1)
const currentLayout = ref('auto')

const containerSize = reactive({
    width: 800,
    height: 600
})

// 计算属性
const getNodeClass = computed(() => (node: FlowchartNode) => {
    const baseClasses = ['bg-base-100', 'hover:shadow-lg']
    
    // 根据架构类型添加特殊样式
    const architectureClasses = []
    if (node.architecture === 'rewoo') {
        architectureClasses.push('border-l-4', 'border-l-purple-500')
    } else if (node.architecture === 'llm_compiler') {
        architectureClasses.push('border-l-4', 'border-l-indigo-500')
    }
    
    // 根据节点类型添加特殊样式
    const typeClasses = []
    switch (node.type) {
        case 'rewoo_planner':
            typeClasses.push('bg-purple-50', 'border-purple-300')
            break
        case 'rewoo_worker':
            typeClasses.push('bg-blue-50', 'border-blue-300')
            break
        case 'rewoo_solver':
            typeClasses.push('bg-green-50', 'border-green-300')
            break
        case 'rewoo_variable':
            typeClasses.push('bg-yellow-50', 'border-yellow-300')
            break
        case 'dag_scheduler':
            typeClasses.push('bg-indigo-50', 'border-indigo-300')
            break
        case 'task_fetcher':
            typeClasses.push('bg-pink-50', 'border-pink-300')
            break
        case 'parallel_executor':
            typeClasses.push('bg-teal-50', 'border-teal-300')
            break
        case 'joiner':
            typeClasses.push('bg-orange-50', 'border-orange-300')
            break
        case 'dependency_resolver':
            typeClasses.push('bg-cyan-50', 'border-cyan-300')
            break
    }

    switch (node.status) {
        case 'pending':
            return [...baseClasses, ...architectureClasses, ...typeClasses, 'border-gray-300', 'text-base-content/70']
        case 'planning':
            return [...baseClasses, ...architectureClasses, ...typeClasses, 'border-blue-400', 'bg-blue-50']
        case 'running':
            return [...baseClasses, ...architectureClasses, ...typeClasses, 'border-yellow-400', 'bg-yellow-50', 'animate-pulse']
        case 'completed':
            return [...baseClasses, ...architectureClasses, ...typeClasses, 'border-green-400', 'bg-green-50']
        case 'failed':
            return [...baseClasses, ...architectureClasses, ...typeClasses, 'border-red-400', 'bg-red-50']
        case 'paused':
            return [...baseClasses, ...architectureClasses, ...typeClasses, 'border-orange-400', 'bg-orange-50']
        default:
            return [...baseClasses, ...architectureClasses, ...typeClasses]
    }
})

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
    // 初始化默认的Plan-and-Execute流程图
    const defaultNodes: FlowchartNode[] = [
        {
            id: 'start',
            name: '开始',
            description: '用户输入任务',
            status: 'completed',
            x: 50,
            y: 50,
            type: 'start',
            dependencies: []
        },
        {
            id: 'planner',
            name: '规划器',
            description: '分析任务并生成执行计划',
            status: 'completed',
            x: 50,
            y: 150,
            type: 'planner',
            dependencies: ['start']
        },
        {
            id: 'agent',
            name: '执行代理',
            description: '执行计划中的具体步骤',
            status: 'running',
            progress: 65,
            x: 50,
            y: 250,
            type: 'agent',
            dependencies: ['planner']
        },
        {
            id: 'tools',
            name: '工具调用',
            description: '调用外部工具和服务',
            status: 'running',
            progress: 30,
            x: 250,
            y: 250,
            type: 'tools',
            dependencies: ['agent']
        },
        {
            id: 'replan',
            name: '重新规划',
            description: '根据执行结果调整计划',
            status: 'pending',
            x: 450,
            y: 150,
            type: 'replan',
            dependencies: ['tools']
        },
        {
            id: 'end',
            name: '结束',
            description: '任务执行完成',
            status: 'pending',
            x: 250,
            y: 350,
            type: 'end',
            dependencies: ['tools', 'replan']
        }
    ]

    nodes.value = defaultNodes
    updateConnections()

    if (autoLayout.value) {
        performAutoLayout()
    }
}

const updateConnections = () => {
    const newConnections: FlowchartConnection[] = []

    nodes.value.forEach(node => {
        node.dependencies.forEach(depId => {
            const fromNode = nodes.value.find(n => n.id === depId)
            if (fromNode) {
                const connection: FlowchartConnection = {
                    id: `${depId}-${node.id}`,
                    from: depId,
                    to: node.id,
                    path: calculateConnectionPath(fromNode, node),
                    status: getConnectionStatus(fromNode, node)
                }
                newConnections.push(connection)
            }
        })
    })

    // 添加条件连接（重新规划循环）
    const replanNode = nodes.value.find(n => n.id === 'replan')
    const agentNode = nodes.value.find(n => n.id === 'agent')
    if (replanNode && agentNode) {
        newConnections.push({
            id: 'replan-agent',
            from: 'replan',
            to: 'agent',
            path: calculateConnectionPath(replanNode, agentNode, true),
            status: 'inactive',
            condition: 'continue'
        })
    }

    connections.value = newConnections
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

const performAutoLayout = () => {
    // 简单的自动布局算法
    const levels: { [key: number]: FlowchartNode[] } = {}
    const visited = new Set<string>()

    const assignLevel = (nodeId: string, level: number) => {
        if (visited.has(nodeId)) return
        visited.add(nodeId)

        const node = nodes.value.find(n => n.id === nodeId)
        if (!node) return

        if (!levels[level]) levels[level] = []
        levels[level].push(node)

        // 处理依赖此节点的节点
        nodes.value.forEach(n => {
            if (n.dependencies.includes(nodeId)) {
                assignLevel(n.id, level + 1)
            }
        })
    }

    // 从起始节点开始
    assignLevel('start', 0)

    // 布局节点
    Object.keys(levels).forEach(levelStr => {
        const level = parseInt(levelStr)
        const levelNodes = levels[level]
        const spacing = 200
        const startX = 50
        const startY = 50 + level * 120

        levelNodes.forEach((node, index) => {
            node.x = startX + index * spacing
            node.y = startY
        })
    })

    updateConnections()
}

const resetView = () => {
    initializeFlowchart()
}

const toggleAutoLayout = () => {
    autoLayout.value = !autoLayout.value
    if (autoLayout.value) {
        performAutoLayout()
    }
}

const setLayout = (layout: string) => {
    currentLayout.value = layout
    switch (layout) {
        case 'horizontal':
            performHorizontalLayout()
            break
        case 'vertical':
            performVerticalLayout()
            break
        case 'circular':
            performCircularLayout()
            break
        default:
            performAutoLayout()
            break
    }
}

const performHorizontalLayout = () => {
    nodes.value.forEach((node, index) => {
        node.x = 50 + index * 200
        node.y = 200
    })
    updateConnections()
}

const performVerticalLayout = () => {
    nodes.value.forEach((node, index) => {
        node.x = 200
        node.y = 50 + index * 120
    })
    updateConnections()
}

const performCircularLayout = () => {
    const centerX = containerSize.width / 2
    const centerY = containerSize.height / 2
    const radius = Math.min(centerX, centerY) - 100

    nodes.value.forEach((node, index) => {
        const angle = (index / nodes.value.length) * 2 * Math.PI
        node.x = centerX + radius * Math.cos(angle) - 100
        node.y = centerY + radius * Math.sin(angle) - 40
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

// 拖拽功能
const onNodeMouseDown = (event: MouseEvent, node: FlowchartNode) => {
    if (autoLayout.value) return

    event.preventDefault()
    draggedNode.value = node
    isDragging.value = true

    const rect = flowchartContainer.value?.getBoundingClientRect()
    if (rect) {
        dragOffset.x = event.clientX - rect.left - node.x
        dragOffset.y = event.clientY - rect.top - node.y
    }

    emit('nodeClick', node)
}

const onCanvasMouseDown = (event: MouseEvent) => {
    if (event.target === flowchartContainer.value) {
        draggedNode.value = null
    }
}

const onCanvasMouseMove = (event: MouseEvent) => {
    if (!isDragging.value || !draggedNode.value || autoLayout.value) return

    const rect = flowchartContainer.value?.getBoundingClientRect()
    if (rect) {
        draggedNode.value.x = event.clientX - rect.left - dragOffset.x
        draggedNode.value.y = event.clientY - rect.top - dragOffset.y

        // 限制在容器内
        draggedNode.value.x = Math.max(0, Math.min(draggedNode.value.x, containerSize.width - 200))
        draggedNode.value.y = Math.max(0, Math.min(draggedNode.value.y, containerSize.height - 100))

        updateConnections()
    }
}

const onCanvasMouseUp = () => {
    isDragging.value = false
    draggedNode.value = null
}

// 更新容器尺寸
const updateContainerSize = () => {
    if (flowchartContainer.value) {
        const rect = flowchartContainer.value.getBoundingClientRect()
        containerSize.width = rect.width
        containerSize.height = rect.height
    }
}

// 生命周期
onMounted(() => {
    initializeFlowchart()
    updateContainerSize()
    window.addEventListener('resize', updateContainerSize)
})

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
        nodes.value.push(node)
        updateConnections()
    },
    removeNode: (nodeId: string) => {
        nodes.value = nodes.value.filter(n => n.id !== nodeId)
        updateConnections()
    },
    resetFlowchart: initializeFlowchart
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
</style>