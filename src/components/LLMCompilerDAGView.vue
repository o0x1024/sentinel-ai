<template>
  <div class="llm-compiler-dag-view">
    <!-- DAG控制面板 -->
    <div class="card bg-base-100 shadow-xl mb-6">
      <div class="card-body">
        <div class="flex justify-between items-center mb-4">
          <h2 class="card-title">LLMCompiler DAG 可视化</h2>
          <div class="flex gap-2">
            <div class="badge" :class="dagStatus.active ? 'badge-success' : 'badge-warning'">
              {{ dagStatus.active ? 'DAG运行中' : 'DAG待机' }}
            </div>
            <button 
              class="btn btn-sm btn-outline"
              @click="refreshDAG"
              :disabled="refreshing"
            >
              <span v-if="refreshing" class="loading loading-spinner loading-sm"></span>
              刷新DAG
            </button>
          </div>
        </div>
        
        <!-- DAG统计 -->
        <div class="grid grid-cols-1 md:grid-cols-5 gap-4 mb-4">
          <div class="stat bg-base-200 rounded-lg">
            <div class="stat-title">总任务数</div>
            <div class="stat-value text-lg">{{ dagStatistics.totalTasks }}</div>
          </div>
          <div class="stat bg-base-200 rounded-lg">
            <div class="stat-title">并发执行</div>
            <div class="stat-value text-lg text-info">{{ dagStatistics.concurrentTasks }}</div>
          </div>
          <div class="stat bg-base-200 rounded-lg">
            <div class="stat-title">已完成</div>
            <div class="stat-value text-lg text-success">{{ dagStatistics.completedTasks }}</div>
          </div>
          <div class="stat bg-base-200 rounded-lg">
            <div class="stat-title">执行效率</div>
            <div class="stat-value text-lg text-primary">{{ dagStatistics.efficiency }}%</div>
          </div>
          <div class="stat bg-base-200 rounded-lg">
            <div class="stat-title">平均并发度</div>
            <div class="stat-value text-lg">{{ dagStatistics.avgConcurrency.toFixed(1) }}</div>
          </div>
        </div>
        
        <!-- 快速操作 -->
        <div class="flex gap-2">
          <button 
            class="btn btn-primary btn-sm"
            @click="startDAGExecution"
            :disabled="!dagStatus.ready || dagStatus.active"
          >
            <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M14.828 14.828a4 4 0 01-5.656 0M9 10h1m4 0h1" />
            </svg>
            启动DAG执行
          </button>
          <button 
            class="btn btn-warning btn-sm"
            @click="pauseDAGExecution"
            :disabled="!dagStatus.active"
          >
            <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 9v6m4-6v6" />
            </svg>
            暂停执行
          </button>
          <button 
            class="btn btn-error btn-sm"
            @click="stopDAGExecution"
            :disabled="!dagStatus.active"
          >
            <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 10l4 4 4-4" />
            </svg>
            停止执行
          </button>
          <button 
            class="btn btn-outline btn-sm"
            @click="showDAGConfig = !showDAGConfig"
          >
            配置DAG
          </button>
        </div>
      </div>
    </div>
    
    <!-- DAG任务图 -->
    <div class="card bg-base-100 shadow-xl mb-6">
      <div class="card-body">
        <div class="flex justify-between items-center mb-4">
          <h3 class="card-title">任务依赖图</h3>
          <div class="flex gap-2">
            <!-- 布局选择 -->
            <div class="dropdown dropdown-end">
              <div tabindex="0" role="button" class="btn btn-sm btn-outline">
                <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 6h16M4 12h16M4 18h16" />
                </svg>
                {{ layoutOptions[currentLayout] }}
              </div>
              <ul tabindex="0" class="dropdown-content z-[1] menu p-2 shadow bg-base-100 rounded-box w-40">
                <li v-for="(label, key) in layoutOptions" :key="key">
                  <a @click="setLayout(key)">{{ label }}</a>
                </li>
              </ul>
            </div>
            
            <!-- 缩放控制 -->
            <div class="join">
              <button class="btn btn-sm join-item" @click="zoomOut">
                <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0zM13 10H7" />
                </svg>
              </button>
              <button class="btn btn-sm join-item" @click="resetZoom">
                {{ Math.round(zoomLevel * 100) }}%
              </button>
              <button class="btn btn-sm join-item" @click="zoomIn">
                <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0zM10 7v3m0 0v3m0-3h3m-3 0H7" />
                </svg>
              </button>
            </div>
            
            <!-- 全屏切换 -->
            <button class="btn btn-sm btn-outline" @click="toggleFullscreen">
              <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 8V4m0 0h4M4 4l5 5m11-1V4m0 0h-4m4 0l-5 5M4 16v4m0 0h4m-4 0l5-5m11 5l-5-5m5 5v-4m0 4h-4" />
              </svg>
            </button>
          </div>
        </div>
        
        <!-- DAG可视化容器 -->
        <div 
          ref="dagContainer"
          class="dag-visualization bg-base-200 rounded-lg p-4 min-h-[500px] relative overflow-hidden"
          :class="{ 'fullscreen': isFullscreen }"
          @wheel="onWheel"
          @mousedown="onCanvasMouseDown"
          @mousemove="onCanvasMouseMove"
          @mouseup="onCanvasMouseUp"
        >
          <!-- SVG画布 -->
          <svg 
            class="absolute inset-0 w-full h-full"
            :viewBox="`0 0 ${canvasSize.width} ${canvasSize.height}`"
            :style="{ transform: `scale(${zoomLevel}) translate(${panOffset.x}px, ${panOffset.y}px)` }"
          >
            <defs>
              <!-- 箭头标记 -->
              <marker id="arrowhead" markerWidth="10" markerHeight="7" refX="9" refY="3.5" orient="auto">
                <polygon points="0 0, 10 3.5, 0 7" class="fill-primary" />
              </marker>
              
              <!-- 依赖线样式 -->
              <marker id="dependency-arrow" markerWidth="8" markerHeight="6" refX="7" refY="3" orient="auto">
                <polygon points="0 0, 8 3, 0 6" class="fill-info" />
              </marker>
            </defs>
            
            <!-- 依赖连接线 -->
            <g class="dependencies">
              <path 
                v-for="edge in dagEdges" 
                :key="edge.id"
                :d="edge.path"
                :class="[
                  'stroke-2 fill-none transition-all duration-200',
                  getDependencyClass(edge)
                ]"
                :marker-end="edge.type === 'dependency' ? 'url(#dependency-arrow)' : 'url(#arrowhead)'"
              />
            </g>
            
            <!-- 任务节点 -->
            <g class="task-nodes">
              <g 
                v-for="node in dagNodes" 
                :key="node.id"
                :transform="`translate(${node.x}, ${node.y})`"
                class="task-node cursor-pointer"
                @click="selectNode(node)"
                @mousedown="onNodeMouseDown($event, node)"
              >
                <!-- 节点背景 -->
                <rect 
                  :width="node.width"
                  :height="node.height"
                  :rx="8"
                  :class="[
                    'transition-all duration-200',
                    getNodeClass(node)
                  ]"
                />
                
                <!-- 节点内容 -->
                <text 
                  :x="node.width / 2"
                  :y="20"
                  text-anchor="middle"
                  class="text-sm font-semibold fill-current"
                >
                  {{ node.name }}
                </text>
                
                <text 
                  :x="node.width / 2"
                  :y="35"
                  text-anchor="middle"
                  class="text-xs fill-current opacity-70"
                >
                  {{ node.type }}
                </text>
                
                <!-- 状态指示器 -->
                <circle 
                  :cx="node.width - 15"
                  :cy="15"
                  :r="6"
                  :class="getStatusIndicatorClass(node.status)"
                />
                
                <!-- 进度条 -->
                <rect 
                  v-if="node.progress !== undefined && node.status === 'running'"
                  :x="5"
                  :y="node.height - 8"
                  :width="(node.width - 10) * (node.progress / 100)"
                  :height="3"
                  class="fill-primary"
                  rx="1.5"
                />
                
                <!-- 并发指示器 -->
                <g v-if="node.concurrent">
                  <rect 
                    x="5"
                    y="5"
                    width="20"
                    height="12"
                    rx="2"
                    class="fill-info opacity-80"
                  />
                  <text 
                    x="15"
                    y="13"
                    text-anchor="middle"
                    class="text-xs fill-white font-bold"
                  >
                    ∥
                  </text>
                </g>
              </g>
            </g>
          </svg>
          
          <!-- 空状态 -->
          <div v-if="dagNodes.length === 0" class="absolute inset-0 flex items-center justify-center">
            <div class="text-center text-base-content/60">
              <svg xmlns="http://www.w3.org/2000/svg" class="h-16 w-16 mx-auto mb-4 opacity-50" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13.828 10.172a4 4 0 00-5.656 0l-4 4a4 4 0 105.656 5.656l1.102-1.101m-.758-4.899a4 4 0 005.656 0l4-4a4 4 0 00-5.656-5.656l-1.1 1.1" />
              </svg>
              <p class="text-lg mb-2">暂无DAG任务</p>
              <p class="text-sm">启动LLMCompiler会话后将显示任务图</p>
            </div>
          </div>
        </div>
        
        <!-- 图例 -->
        <div class="mt-4 p-3 bg-base-200 rounded-lg">
          <h4 class="font-semibold text-sm mb-2">状态图例</h4>
          <div class="grid grid-cols-2 md:grid-cols-4 gap-4 text-xs">
            <div class="flex items-center gap-2">
              <div class="w-4 h-4 rounded border-2 border-gray-400 bg-gray-100"></div>
              <span>待执行</span>
            </div>
            <div class="flex items-center gap-2">
              <div class="w-4 h-4 rounded border-2 border-blue-400 bg-blue-100"></div>
              <span>准备中</span>
            </div>
            <div class="flex items-center gap-2">
              <div class="w-4 h-4 rounded border-2 border-yellow-400 bg-yellow-100"></div>
              <span>执行中</span>
            </div>
            <div class="flex items-center gap-2">
              <div class="w-4 h-4 rounded border-2 border-green-400 bg-green-100"></div>
              <span>已完成</span>
            </div>
            <div class="flex items-center gap-2">
              <div class="w-4 h-4 rounded border-2 border-red-400 bg-red-100"></div>
              <span>失败</span>
            </div>
            <div class="flex items-center gap-2">
              <div class="w-4 h-4 rounded border-2 border-orange-400 bg-orange-100"></div>
              <span>已暂停</span>
            </div>
            <div class="flex items-center gap-2">
              <div class="w-4 h-4 rounded border-2 border-info bg-info/20"></div>
              <span>并发执行</span>
            </div>
            <div class="flex items-center gap-2">
              <div class="w-4 h-1 bg-primary rounded"></div>
              <span>依赖关系</span>
            </div>
          </div>
        </div>
      </div>
    </div>
    
    <!-- 并发执行监控 -->
    <div class="card bg-base-100 shadow-xl mb-6">
      <div class="card-body">
        <h3 class="card-title mb-4">并发执行监控</h3>
        
        <!-- 并发统计 -->
        <div class="grid grid-cols-1 md:grid-cols-4 gap-4 mb-4">
          <div class="stat bg-base-200 rounded-lg">
            <div class="stat-title">当前并发数</div>
            <div class="stat-value text-lg text-info">{{ concurrentStats.current }}</div>
            <div class="stat-desc">/ {{ concurrentStats.max }} 最大</div>
          </div>
          <div class="stat bg-base-200 rounded-lg">
            <div class="stat-title">线程池使用率</div>
            <div class="stat-value text-lg">{{ Math.round(concurrentStats.threadPoolUsage * 100) }}%</div>
          </div>
          <div class="stat bg-base-200 rounded-lg">
            <div class="stat-title">队列长度</div>
            <div class="stat-value text-lg">{{ concurrentStats.queueLength }}</div>
          </div>
          <div class="stat bg-base-200 rounded-lg">
            <div class="stat-title">平均等待时间</div>
            <div class="stat-value text-lg">{{ formatDuration(concurrentStats.avgWaitTime) }}</div>
          </div>
        </div>
        
        <!-- 并发任务列表 -->
        <div class="overflow-x-auto">
          <table class="table table-sm">
            <thead>
              <tr>
                <th>任务ID</th>
                <th>任务名称</th>
                <th>状态</th>
                <th>开始时间</th>
                <th>执行时间</th>
                <th>进度</th>
                <th>操作</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="task in runningTasks" :key="task.id">
                <td>
                  <code class="text-xs">{{ task.id.substring(0, 8) }}</code>
                </td>
                <td>{{ task.name }}</td>
                <td>
                  <div class="badge badge-sm" :class="getTaskStatusBadge(task.status)">{{ task.status }}</div>
                </td>
                <td class="text-xs">{{ formatTime(task.startTime) }}</td>
                <td class="text-xs">{{ formatDuration(task.executionTime) }}</td>
                <td>
                  <div class="flex items-center gap-2">
                    <progress 
                      class="progress progress-primary w-16 h-2" 
                      :value="task.progress" 
                      max="100"
                    ></progress>
                    <span class="text-xs">{{ Math.round(task.progress) }}%</span>
                  </div>
                </td>
                <td>
                  <button 
                    class="btn btn-xs btn-error"
                    @click="cancelTask(task.id)"
                    :disabled="task.status === 'completed'"
                  >
                    取消
                  </button>
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>
    </div>
    
    <!-- Joiner决策历史 -->
    <div class="card bg-base-100 shadow-xl">
      <div class="card-body">
        <h3 class="card-title mb-4">Joiner 决策历史</h3>
        
        <div class="space-y-3">
          <div 
            v-for="decision in joinerDecisions" 
            :key="decision.id"
            class="border rounded-lg p-4"
          >
            <div class="flex justify-between items-start mb-2">
              <div>
                <div class="font-semibold">{{ getDecisionTypeText(decision.type) }}</div>
                <div class="text-sm text-base-content/70">{{ decision.reason }}</div>
              </div>
              <div class="text-right">
                <div class="badge badge-sm" :class="getDecisionResultBadge(decision.result)">{{ decision.result }}</div>
                <div class="text-xs text-base-content/60 mt-1">{{ formatTime(decision.timestamp) }}</div>
              </div>
            </div>
            
            <!-- 决策详情 -->
            <div class="collapse collapse-arrow bg-base-200">
              <input type="checkbox" /> 
              <div class="collapse-title text-sm font-medium">
                决策详情
              </div>
              <div class="collapse-content">
                <div class="text-sm space-y-2">
                  <div><strong>触发条件:</strong> {{ decision.trigger }}</div>
                  <div><strong>考虑因素:</strong> {{ decision.factors.join(', ') }}</div>
                  <div><strong>影响任务:</strong> {{ decision.affectedTasks.join(', ') }}</div>
                  <div v-if="decision.metrics"><strong>性能指标:</strong> {{ JSON.stringify(decision.metrics) }}</div>
                </div>
              </div>
            </div>
          </div>
        </div>
        
        <!-- 空状态 -->
        <div v-if="joinerDecisions.length === 0" class="text-center py-8 text-base-content/60">
          <svg xmlns="http://www.w3.org/2000/svg" class="h-12 w-12 mx-auto mb-2 opacity-50" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9.663 17h4.673M12 3v1m6.364 1.636l-.707.707M21 12h-1M4 12H3m3.343-5.657l-.707-.707m2.828 9.9a5 5 0 117.072 0l-.548.547A3.374 3.374 0 0014 18.469V19a2 2 0 11-4 0v-.531c0-.895-.356-1.754-.988-2.386l-.548-.547z" />
          </svg>
          <p>暂无Joiner决策记录</p>
          <p class="text-sm">启动LLMCompiler会话后将显示决策历史</p>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, computed, onMounted, onUnmounted, nextTick } from 'vue'

// 类型定义
interface DAGStatus {
  active: boolean
  ready: boolean
  paused: boolean
}

interface DAGStatistics {
  totalTasks: number
  concurrentTasks: number
  completedTasks: number
  efficiency: number
  avgConcurrency: number
}

interface DAGNode {
  id: string
  name: string
  type: string
  status: 'pending' | 'ready' | 'running' | 'completed' | 'failed' | 'paused'
  progress?: number
  x: number
  y: number
  width: number
  height: number
  concurrent: boolean
  dependencies: string[]
  metadata?: Record<string, any>
}

interface DAGEdge {
  id: string
  from: string
  to: string
  type: 'dependency' | 'data-flow'
  path: string
  status: 'inactive' | 'active' | 'completed'
}

interface ConcurrentStats {
  current: number
  max: number
  threadPoolUsage: number
  queueLength: number
  avgWaitTime: number
}

interface RunningTask {
  id: string
  name: string
  status: string
  startTime: Date
  executionTime: number
  progress: number
}

interface JoinerDecision {
  id: string
  type: 'continue' | 'wait' | 'replan' | 'optimize'
  reason: string
  result: 'success' | 'failed' | 'pending'
  timestamp: Date
  trigger: string
  factors: string[]
  affectedTasks: string[]
  metrics?: Record<string, any>
}

// Props
interface Props {
  sessionId?: string
}

const props = defineProps<Props>()

// Emits
const emit = defineEmits<{
  dagStarted: [sessionId: string]
  dagPaused: []
  dagStopped: []
  taskCancelled: [taskId: string]
  nodeSelected: [node: DAGNode]
}>()

// 响应式数据
const dagStatus = ref<DAGStatus>({
  active: false,
  ready: true,
  paused: false
})

const dagStatistics = ref<DAGStatistics>({
  totalTasks: 0,
  concurrentTasks: 0,
  completedTasks: 0,
  efficiency: 0,
  avgConcurrency: 0
})

const dagNodes = ref<DAGNode[]>([])
const dagEdges = ref<DAGEdge[]>([])
const runningTasks = ref<RunningTask[]>([])
const joinerDecisions = ref<JoinerDecision[]>([])

const concurrentStats = ref<ConcurrentStats>({
  current: 0,
  max: 8,
  threadPoolUsage: 0,
  queueLength: 0,
  avgWaitTime: 0
})

// UI状态
const refreshing = ref(false)
const showDAGConfig = ref(false)
const isFullscreen = ref(false)
const selectedNode = ref<DAGNode | null>(null)

// 画布控制
const dagContainer = ref<HTMLElement>()
const canvasSize = reactive({ width: 800, height: 600 })
const zoomLevel = ref(1)
const panOffset = reactive({ x: 0, y: 0 })
const isDragging = ref(false)
const dragStart = reactive({ x: 0, y: 0 })

// 布局选项
const layoutOptions = {
  'hierarchical': '层次布局',
  'force': '力导向布局',
  'circular': '环形布局',
  'grid': '网格布局'
} as const

type LayoutType = keyof typeof layoutOptions
const currentLayout = ref<LayoutType>('hierarchical')

// 方法
const startDAGExecution = async () => {
  try {
    dagStatus.value.active = true
    emit('dagStarted', `dag_${Date.now()}`)
  } catch (error) {
    console.error('启动DAG执行失败:', error)
  }
}

const pauseDAGExecution = () => {
  dagStatus.value.paused = true
  emit('dagPaused')
}

const stopDAGExecution = () => {
  dagStatus.value.active = false
  dagStatus.value.paused = false
  emit('dagStopped')
}

const refreshDAG = async () => {
  refreshing.value = true
  try {
    // 调用后端API刷新DAG数据
    await new Promise(resolve => setTimeout(resolve, 1000)) // 模拟API调用
  } finally {
    refreshing.value = false
  }
}

const cancelTask = (taskId: string) => {
  const index = runningTasks.value.findIndex(task => task.id === taskId)
  if (index > -1) {
    runningTasks.value.splice(index, 1)
    emit('taskCancelled', taskId)
  }
}

const selectNode = (node: DAGNode) => {
  selectedNode.value = node
  emit('nodeSelected', node)
}

// 画布控制方法
const setLayout = (layout: LayoutType) => {
  currentLayout.value = layout
  // 重新计算节点位置
  calculateNodePositions()
}

const zoomIn = () => {
  zoomLevel.value = Math.min(zoomLevel.value * 1.2, 3)
}

const zoomOut = () => {
  zoomLevel.value = Math.max(zoomLevel.value / 1.2, 0.3)
}

const resetZoom = () => {
  zoomLevel.value = 1
  panOffset.x = 0
  panOffset.y = 0
}

const toggleFullscreen = () => {
  isFullscreen.value = !isFullscreen.value
}

const onWheel = (event: WheelEvent) => {
  event.preventDefault()
  if (event.deltaY < 0) {
    zoomIn()
  } else {
    zoomOut()
  }
}

const onCanvasMouseDown = (event: MouseEvent) => {
  isDragging.value = true
  dragStart.x = event.clientX - panOffset.x
  dragStart.y = event.clientY - panOffset.y
}

const onCanvasMouseMove = (event: MouseEvent) => {
  if (isDragging.value) {
    panOffset.x = event.clientX - dragStart.x
    panOffset.y = event.clientY - dragStart.y
  }
}

const onCanvasMouseUp = () => {
  isDragging.value = false
}

const onNodeMouseDown = (event: MouseEvent, node: DAGNode) => {
  event.stopPropagation()
  selectNode(node)
}

const calculateNodePositions = () => {
  // 根据当前布局算法计算节点位置
  // 这里是简化实现，实际应该使用专业的图布局算法
  const nodes = dagNodes.value
  if (nodes.length === 0) return
  
  switch (currentLayout.value) {
    case 'hierarchical':
      calculateHierarchicalLayout()
      break
    case 'force':
      calculateForceLayout()
      break
    case 'circular':
      calculateCircularLayout()
      break
    case 'grid':
      calculateGridLayout()
      break
  }
  
  // 重新计算边的路径
  calculateEdgePaths()
}

const calculateHierarchicalLayout = () => {
  // 层次布局实现
  const nodes = dagNodes.value
  const levels: string[][] = []
  const visited = new Set<string>()
  
  // 简化的层次分析
  nodes.forEach((node, index) => {
    const level = Math.floor(index / 3)
    if (!levels[level]) levels[level] = []
    levels[level].push(node.id)
  })
  
  // 设置节点位置
  levels.forEach((levelNodes, levelIndex) => {
    levelNodes.forEach((nodeId, nodeIndex) => {
      const node = nodes.find(n => n.id === nodeId)
      if (node) {
        node.x = 100 + nodeIndex * 200
        node.y = 100 + levelIndex * 150
      }
    })
  })
}

const calculateForceLayout = () => {
  // 力导向布局的简化实现
  // 实际应该使用D3.js或其他专业库
}

const calculateCircularLayout = () => {
  // 环形布局实现
  const nodes = dagNodes.value
  const centerX = canvasSize.width / 2
  const centerY = canvasSize.height / 2
  const radius = Math.min(centerX, centerY) - 100
  
  nodes.forEach((node, index) => {
    const angle = (2 * Math.PI * index) / nodes.length
    node.x = centerX + radius * Math.cos(angle) - node.width / 2
    node.y = centerY + radius * Math.sin(angle) - node.height / 2
  })
}

const calculateGridLayout = () => {
  // 网格布局实现
  const nodes = dagNodes.value
  const cols = Math.ceil(Math.sqrt(nodes.length))
  
  nodes.forEach((node, index) => {
    const row = Math.floor(index / cols)
    const col = index % cols
    node.x = 100 + col * 200
    node.y = 100 + row * 150
  })
}

const calculateEdgePaths = () => {
  // 计算边的SVG路径
  dagEdges.value.forEach(edge => {
    const fromNode = dagNodes.value.find(n => n.id === edge.from)
    const toNode = dagNodes.value.find(n => n.id === edge.to)
    
    if (fromNode && toNode) {
      const fromX = fromNode.x + fromNode.width / 2
      const fromY = fromNode.y + fromNode.height
      const toX = toNode.x + toNode.width / 2
      const toY = toNode.y
      
      // 简单的直线连接，实际可以使用贝塞尔曲线
      edge.path = `M ${fromX} ${fromY} L ${toX} ${toY}`
    }
  })
}

// 格式化方法
const formatDuration = (ms: number) => {
  if (ms < 1000) return `${ms}ms`
  if (ms < 60000) return `${(ms / 1000).toFixed(1)}s`
  return `${(ms / 60000).toFixed(1)}m`
}

const formatTime = (date: Date) => {
  return date.toLocaleTimeString()
}

// 样式类方法
const getNodeClass = (node: DAGNode) => {
  const baseClass = 'stroke-2 transition-all duration-200'
  const statusClasses = {
    pending: 'fill-gray-100 stroke-gray-400',
    ready: 'fill-blue-100 stroke-blue-400',
    running: 'fill-yellow-100 stroke-yellow-400',
    completed: 'fill-green-100 stroke-green-400',
    failed: 'fill-red-100 stroke-red-400',
    paused: 'fill-orange-100 stroke-orange-400'
  }
  
  let classes = `${baseClass} ${statusClasses[node.status] || statusClasses.pending}`
  
  if (selectedNode.value?.id === node.id) {
    classes += ' ring-2 ring-primary'
  }
  
  return classes
}

const getStatusIndicatorClass = (status: string) => {
  const classes = {
    pending: 'fill-gray-400',
    ready: 'fill-blue-400',
    running: 'fill-yellow-400',
    completed: 'fill-green-400',
    failed: 'fill-red-400',
    paused: 'fill-orange-400'
  }
  return classes[status as keyof typeof classes] || classes.pending
}

const getDependencyClass = (edge: DAGEdge) => {
  const baseClass = 'transition-all duration-200'
  const statusClasses = {
    inactive: 'stroke-gray-400',
    active: 'stroke-blue-400',
    completed: 'stroke-green-400'
  }
  return `${baseClass} ${statusClasses[edge.status] || statusClasses.inactive}`
}

const getTaskStatusBadge = (status: string) => {
  const badges = {
    pending: 'badge-warning',
    ready: 'badge-info',
    running: 'badge-primary',
    completed: 'badge-success',
    failed: 'badge-error',
    paused: 'badge-warning'
  }
  return badges[status as keyof typeof badges] || 'badge-ghost'
}

const getDecisionTypeText = (type: string) => {
  const texts = {
    continue: '继续执行',
    wait: '等待依赖',
    replan: '重新规划',
    optimize: '优化调度'
  }
  return texts[type as keyof typeof texts] || type
}

const getDecisionResultBadge = (result: string) => {
  const badges = {
    success: 'badge-success',
    failed: 'badge-error',
    pending: 'badge-warning'
  }
  return badges[result as keyof typeof badges] || 'badge-ghost'
}

// 生命周期
onMounted(() => {
  // 初始化画布大小
  if (dagContainer.value) {
    const rect = dagContainer.value.getBoundingClientRect()
    canvasSize.width = rect.width
    canvasSize.height = rect.height
  }
  
  // 加载初始数据
  refreshDAG()
})

// 监听窗口大小变化
const handleResize = () => {
  if (dagContainer.value) {
    const rect = dagContainer.value.getBoundingClientRect()
    canvasSize.width = rect.width
    canvasSize.height = rect.height
  }
}

onMounted(() => {
  window.addEventListener('resize', handleResize)
})

onUnmounted(() => {
  window.removeEventListener('resize', handleResize)
})
</script>

<style scoped>
.llm-compiler-dag-view {
  @apply space-y-6;
}

.dag-visualization {
  @apply relative;
}

.dag-visualization.fullscreen {
  @apply fixed inset-0 z-50 bg-base-100;
}

.task-node:hover {
  @apply opacity-80;
}

.task-node.selected {
  @apply ring-2 ring-primary;
}

.dependencies path:hover {
  @apply stroke-2;
}
</style>