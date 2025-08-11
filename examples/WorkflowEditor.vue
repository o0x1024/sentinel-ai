<template>
  <div class="workflow-editor">
    <!-- 顶部工具栏 -->
    <div class="editor-toolbar">
      <div class="toolbar-left">
        <button class="btn btn-primary" @click="saveWorkflow" :disabled="!hasChanges">
          <i class="fas fa-save"></i>
          保存工作流
        </button>
        <button class="btn btn-secondary" @click="executeWorkflow" :disabled="!isValid">
          <i class="fas fa-play"></i>
          执行工作流
        </button>
        <button class="btn btn-ghost" @click="validateWorkflow">
          <i class="fas fa-check-circle"></i>
          验证
        </button>
      </div>
      
      <div class="toolbar-center">
        <span class="workflow-name">{{ workflowName }}</span>
        <div class="status-indicators">
          <span class="badge" :class="validationStatus.class">
            {{ validationStatus.text }}
          </span>
          <span class="badge badge-info" v-if="nodeCount > 0">
            {{ nodeCount }} 个节点
          </span>
        </div>
      </div>
      
      <div class="toolbar-right">
        <button class="btn btn-ghost" @click="toggleMinimap">
          <i class="fas fa-map"></i>
          小地图
        </button>
        <button class="btn btn-ghost" @click="fitToScreen">
          <i class="fas fa-expand-arrows-alt"></i>
          适应屏幕
        </button>
        <div class="zoom-controls">
          <button class="btn btn-sm btn-ghost" @click="zoomOut">
            <i class="fas fa-minus"></i>
          </button>
          <span class="zoom-level">{{ Math.round(zoomLevel * 100) }}%</span>
          <button class="btn btn-sm btn-ghost" @click="zoomIn">
            <i class="fas fa-plus"></i>
          </button>
        </div>
      </div>
    </div>
    
    <!-- 主编辑区域 -->
    <div class="editor-main">
      <!-- 左侧组件面板 -->
      <div class="component-panel" :class="{ collapsed: panelCollapsed }">
        <div class="panel-header">
          <h3>组件库</h3>
          <button class="btn btn-sm btn-ghost" @click="panelCollapsed = !panelCollapsed">
            <i :class="panelCollapsed ? 'fas fa-chevron-right' : 'fas fa-chevron-left'"></i>
          </button>
        </div>
        
        <div class="panel-content" v-show="!panelCollapsed">
          <!-- 搜索框 -->
          <div class="search-box">
            <input 
              type="text" 
              class="input input-sm input-bordered" 
              placeholder="搜索组件..."
              v-model="searchQuery"
              @input="filterComponents">
            <i class="fas fa-search search-icon"></i>
          </div>
          
          <!-- Agent分类 -->
          <div class="component-categories">
            <div 
              v-for="category in filteredCategories" 
              :key="category.name"
              class="category-section">
              <div 
                class="category-header"
                @click="toggleCategory(category.name)">
                <i :class="category.expanded ? 'fas fa-chevron-down' : 'fas fa-chevron-right'"></i>
                <span class="category-title">{{ category.displayName }}</span>
                <span class="badge badge-sm">{{ category.agents.length }}</span>
              </div>
              
              <div class="category-items" v-show="category.expanded">
                <div 
                  v-for="agent in category.agents" 
                  :key="agent.id"
                  class="component-item"
                  draggable="true"
                  @dragstart="onDragStart(agent, $event)"
                  @click="showAgentDetails(agent)">
                  <div class="item-icon">
                    <i :class="agent.icon" :style="{ color: agent.color }"></i>
                  </div>
                  <div class="item-info">
                    <div class="item-name">{{ agent.name }}</div>
                    <div class="item-description">{{ agent.description }}</div>
                    <div class="item-tags">
                      <span 
                        v-for="tag in agent.tags.slice(0, 2)" 
                        :key="tag"
                        class="tag">{{ tag }}</span>
                    </div>
                  </div>
                </div>
              </div>
            </div>
            
            <!-- 控制节点 -->
            <div class="category-section">
              <div class="category-header" @click="toggleCategory('control')">
                <i :class="controlExpanded ? 'fas fa-chevron-down' : 'fas fa-chevron-right'"></i>
                <span class="category-title">控制节点</span>
                <span class="badge badge-sm">{{ controlNodes.length }}</span>
              </div>
              
              <div class="category-items" v-show="controlExpanded">
                <div 
                  v-for="control in controlNodes" 
                  :key="control.type"
                  class="component-item control-node"
                  draggable="true"
                  @dragstart="onDragStart(control, $event)">
                  <div class="item-icon">
                    <i :class="control.icon" :style="{ color: control.color }"></i>
                  </div>
                  <div class="item-info">
                    <div class="item-name">{{ control.name }}</div>
                    <div class="item-description">{{ control.description }}</div>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
      
      <!-- 中央画布区域 -->
      <div class="canvas-container">
        <div class="canvas-wrapper" ref="canvasWrapper">
          <svg 
            class="workflow-canvas"
            :width="canvasSize.width"
            :height="canvasSize.height"
            :viewBox="viewBox"
            @mousedown="onCanvasMouseDown"
            @mousemove="onCanvasMouseMove"
            @mouseup="onCanvasMouseUp"
            @wheel="onCanvasWheel"
            @drop="onCanvasDrop"
            @dragover="onCanvasDragOver">
            
            <!-- 网格背景 -->
            <defs>
              <pattern id="grid" width="20" height="20" patternUnits="userSpaceOnUse">
                <path d="M 20 0 L 0 0 0 20" fill="none" stroke="#e5e7eb" stroke-width="1" opacity="0.5"/>
              </pattern>
              
              <!-- 箭头标记 -->
              <marker id="arrowhead" markerWidth="10" markerHeight="7" 
                      refX="9" refY="3.5" orient="auto">
                <polygon points="0 0, 10 3.5, 0 7" fill="#6b7280" />
              </marker>
            </defs>
            
            <!-- 网格 -->
            <rect width="100%" height="100%" fill="url(#grid)" />
            
            <!-- 连接线层 -->
            <g class="connections-layer">
              <WorkflowConnection 
                v-for="connection in connections" 
                :key="connection.id"
                :connection="connection"
                :selected="selectedConnectionId === connection.id"
                @select="selectConnection"
                @delete="deleteConnection"/>
            </g>
            
            <!-- 临时连接线 -->
            <g class="temp-connection-layer" v-if="tempConnection">
              <path 
                :d="getTempConnectionPath()"
                stroke="#3b82f6"
                stroke-width="2"
                fill="none"
                stroke-dasharray="5,5"
                marker-end="url(#arrowhead)"/>
            </g>
            
            <!-- 节点层 -->
            <g class="nodes-layer">
              <WorkflowNode 
                v-for="node in nodes" 
                :key="node.id"
                :node="node"
                :selected="selectedNodeId === node.id"
                :zoom-level="zoomLevel"
                @select="selectNode"
                @move="moveNode"
                @connect="startConnection"
                @delete="deleteNode"
                @duplicate="duplicateNode"/>
            </g>
            
            <!-- 选择框 -->
            <rect 
              v-if="selectionBox.visible"
              :x="selectionBox.x"
              :y="selectionBox.y"
              :width="selectionBox.width"
              :height="selectionBox.height"
              fill="rgba(59, 130, 246, 0.1)"
              stroke="#3b82f6"
              stroke-width="1"
              stroke-dasharray="3,3"/>
          </svg>
        </div>
        
        <!-- 小地图 -->
        <div class="minimap" v-show="minimapVisible">
          <svg class="minimap-svg" :width="minimapSize.width" :height="minimapSize.height">
            <!-- 小地图内容 -->
            <rect 
              class="minimap-viewport"
              :x="minimapViewport.x"
              :y="minimapViewport.y"
              :width="minimapViewport.width"
              :height="minimapViewport.height"
              fill="rgba(59, 130, 246, 0.2)"
              stroke="#3b82f6"
              stroke-width="1"/>
          </svg>
        </div>
        
        <!-- 右键菜单 -->
        <ContextMenu 
          v-if="contextMenu.visible"
          :x="contextMenu.x"
          :y="contextMenu.y"
          :items="contextMenu.items"
          @select="onContextMenuSelect"
          @close="closeContextMenu"/>
      </div>
      
      <!-- 右侧属性面板 -->
      <div class="property-panel" :class="{ collapsed: propertyPanelCollapsed }">
        <div class="panel-header">
          <h3>{{ selectedNode ? '节点属性' : selectedConnection ? '连接属性' : '属性' }}</h3>
          <button class="btn btn-sm btn-ghost" @click="propertyPanelCollapsed = !propertyPanelCollapsed">
            <i :class="propertyPanelCollapsed ? 'fas fa-chevron-left' : 'fas fa-chevron-right'"></i>
          </button>
        </div>
        
        <div class="panel-content" v-show="!propertyPanelCollapsed">
          <!-- 节点属性编辑 -->
          <NodePropertyEditor 
            v-if="selectedNode"
            :node="selectedNode"
            :available-agents="availableAgents"
            @update="updateNodeProperties"
            @validate="validateNode"/>
          
          <!-- 连接属性编辑 -->
          <ConnectionPropertyEditor 
            v-else-if="selectedConnection"
            :connection="selectedConnection"
            @update="updateConnectionProperties"/>
          
          <!-- 工作流属性编辑 -->
          <WorkflowPropertyEditor 
            v-else
            :workflow="workflowDefinition"
            @update="updateWorkflowProperties"/>
        </div>
      </div>
    </div>
    
    <!-- 底部状态栏 -->
    <div class="status-bar">
      <div class="status-left">
        <span class="status-item">
          <i class="fas fa-project-diagram"></i>
          {{ nodeCount }} 节点, {{ connectionCount }} 连接
        </span>
        <span class="status-item" v-if="selectedNode">
          <i class="fas fa-mouse-pointer"></i>
          已选择: {{ selectedNode.name }}
        </span>
      </div>
      
      <div class="status-right">
        <span class="status-item" v-if="lastSaved">
          <i class="fas fa-clock"></i>
          上次保存: {{ formatTime(lastSaved) }}
        </span>
        <span class="status-item">
          <i class="fas fa-search"></i>
          {{ Math.round(zoomLevel * 100) }}%
        </span>
      </div>
    </div>
    
    <!-- 模态对话框 -->
    <AgentDetailsModal 
      v-if="agentDetailsModal.visible"
      :agent="agentDetailsModal.agent"
      @close="agentDetailsModal.visible = false"
      @add-to-canvas="addAgentToCanvas"/>
    
    <WorkflowValidationModal 
      v-if="validationModal.visible"
      :validation-result="validationModal.result"
      @close="validationModal.visible = false"
      @fix-issues="fixValidationIssues"/>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, computed, onMounted, onUnmounted, nextTick } from 'vue'
import { useWorkflowStore } from '@/stores/workflow'
import { useAgentStore } from '@/stores/agent'
import { useDragAndDrop } from '@/composables/useDragAndDrop'
import { useWorkflowValidation } from '@/composables/useWorkflowValidation'
import { useCanvasInteraction } from '@/composables/useCanvasInteraction'

// 组件导入
import WorkflowNode from './WorkflowNode.vue'
import WorkflowConnection from './WorkflowConnection.vue'
import NodePropertyEditor from './NodePropertyEditor.vue'
import ConnectionPropertyEditor from './ConnectionPropertyEditor.vue'
import WorkflowPropertyEditor from './WorkflowPropertyEditor.vue'
import ContextMenu from './ContextMenu.vue'
import AgentDetailsModal from './AgentDetailsModal.vue'
import WorkflowValidationModal from './WorkflowValidationModal.vue'

// Props
interface Props {
  workflowId?: string
}

const props = withDefaults(defineProps<Props>(), {
  workflowId: ''
})

// Stores
const workflowStore = useWorkflowStore()
const agentStore = useAgentStore()

// Composables
const { dragState, startDrag, onDrag, endDrag } = useDragAndDrop()
const { validateWorkflow: performValidation, validateNode } = useWorkflowValidation()
const { 
  canvasSize, 
  viewBox, 
  zoomLevel, 
  zoomIn, 
  zoomOut, 
  fitToScreen,
  onCanvasMouseDown,
  onCanvasMouseMove,
  onCanvasMouseUp,
  onCanvasWheel
} = useCanvasInteraction()

// 响应式数据
const canvasWrapper = ref<HTMLElement>()
const searchQuery = ref('')
const panelCollapsed = ref(false)
const propertyPanelCollapsed = ref(false)
const minimapVisible = ref(true)
const controlExpanded = ref(true)
const hasChanges = ref(false)
const lastSaved = ref<Date | null>(null)

// 选择状态
const selectedNodeId = ref<string | null>(null)
const selectedConnectionId = ref<string | null>(null)
const tempConnection = ref<any>(null)

// 选择框
const selectionBox = reactive({
  visible: false,
  x: 0,
  y: 0,
  width: 0,
  height: 0
})

// 右键菜单
const contextMenu = reactive({
  visible: false,
  x: 0,
  y: 0,
  items: [] as any[]
})

// 模态对话框
const agentDetailsModal = reactive({
  visible: false,
  agent: null as any
})

const validationModal = reactive({
  visible: false,
  result: null as any
})

// 小地图
const minimapSize = reactive({ width: 200, height: 150 })
const minimapViewport = reactive({ x: 0, y: 0, width: 50, height: 37.5 })

// 计算属性
const workflowDefinition = computed(() => workflowStore.currentWorkflow)
const workflowName = computed(() => workflowDefinition.value?.metadata?.name || '未命名工作流')
const nodes = computed(() => workflowStore.nodes)
const connections = computed(() => workflowStore.connections)
const nodeCount = computed(() => nodes.value.length)
const connectionCount = computed(() => connections.value.length)
const selectedNode = computed(() => 
  selectedNodeId.value ? nodes.value.find(n => n.id === selectedNodeId.value) : null
)
const selectedConnection = computed(() => 
  selectedConnectionId.value ? connections.value.find(c => c.id === selectedConnectionId.value) : null
)
const availableAgents = computed(() => agentStore.agents)

// 组件分类
const agentCategories = ref([
  {
    name: 'security',
    displayName: '安全测试',
    expanded: true,
    agents: [
      {
        id: 'reconnaissance_agent',
        name: '侦察Agent',
        description: '执行目标侦察和信息收集',
        icon: 'fas fa-search',
        color: '#3b82f6',
        tags: ['reconnaissance', 'osint']
      },
      {
        id: 'scanning_agent',
        name: '扫描Agent',
        description: '执行端口扫描和服务发现',
        icon: 'fas fa-radar',
        color: '#10b981',
        tags: ['scanning', 'discovery']
      },
      {
        id: 'vulnerability_agent',
        name: '漏洞Agent',
        description: '执行漏洞扫描和检测',
        icon: 'fas fa-bug',
        color: '#f59e0b',
        tags: ['vulnerability', 'security']
      }
    ]
  },
  {
    name: 'data_analytics',
    displayName: '数据分析',
    expanded: false,
    agents: [
      {
        id: 'data_analysis_agent',
        name: '数据分析Agent',
        description: '执行数据分析和洞察生成',
        icon: 'fas fa-chart-bar',
        color: '#8b5cf6',
        tags: ['analytics', 'insights']
      },
      {
        id: 'ml_agent',
        name: '机器学习Agent',
        description: '执行机器学习模型训练和预测',
        icon: 'fas fa-brain',
        color: '#ec4899',
        tags: ['ml', 'prediction']
      }
    ]
  },
  {
    name: 'automation',
    displayName: '自动化',
    expanded: false,
    agents: [
      {
        id: 'notification_agent',
        name: '通知Agent',
        description: '发送通知和警报',
        icon: 'fas fa-bell',
        color: '#ef4444',
        tags: ['notification', 'alert']
      },
      {
        id: 'reporting_agent',
        name: '报告Agent',
        description: '生成和发布报告',
        icon: 'fas fa-file-alt',
        color: '#6b7280',
        tags: ['reporting', 'documentation']
      }
    ]
  }
])

const controlNodes = ref([
  {
    type: 'condition',
    name: '条件判断',
    description: '基于条件执行不同分支',
    icon: 'fas fa-code-branch',
    color: '#f59e0b'
  },
  {
    type: 'loop',
    name: '循环执行',
    description: '重复执行一组操作',
    icon: 'fas fa-redo',
    color: '#10b981'
  },
  {
    type: 'parallel',
    name: '并行执行',
    description: '同时执行多个操作',
    icon: 'fas fa-stream',
    color: '#8b5cf6'
  },
  {
    type: 'delay',
    name: '延时等待',
    description: '等待指定时间后继续',
    icon: 'fas fa-clock',
    color: '#6b7280'
  }
])

// 过滤后的分类
const filteredCategories = computed(() => {
  if (!searchQuery.value) return agentCategories.value
  
  return agentCategories.value.map(category => ({
    ...category,
    agents: category.agents.filter(agent => 
      agent.name.toLowerCase().includes(searchQuery.value.toLowerCase()) ||
      agent.description.toLowerCase().includes(searchQuery.value.toLowerCase()) ||
      agent.tags.some(tag => tag.toLowerCase().includes(searchQuery.value.toLowerCase()))
    )
  })).filter(category => category.agents.length > 0)
})

// 验证状态
const validationStatus = computed(() => {
  // 这里应该基于实际的验证结果
  const isValid = nodes.value.length > 0 && connections.value.length >= 0
  return {
    class: isValid ? 'badge-success' : 'badge-warning',
    text: isValid ? '有效' : '需要验证'
  }
})

const isValid = computed(() => validationStatus.value.class === 'badge-success')

// 方法
const toggleCategory = (categoryName: string) => {
  const category = agentCategories.value.find(c => c.name === categoryName)
  if (category) {
    category.expanded = !category.expanded
  } else if (categoryName === 'control') {
    controlExpanded.value = !controlExpanded.value
  }
}

const filterComponents = () => {
  // 过滤逻辑已在计算属性中实现
}

const onDragStart = (item: any, event: DragEvent) => {
  if (event.dataTransfer) {
    event.dataTransfer.setData('application/json', JSON.stringify(item))
    event.dataTransfer.effectAllowed = 'copy'
  }
}

const onCanvasDrop = (event: DragEvent) => {
  event.preventDefault()
  const data = event.dataTransfer?.getData('application/json')
  if (data) {
    const item = JSON.parse(data)
    const rect = canvasWrapper.value?.getBoundingClientRect()
    if (rect) {
      const x = (event.clientX - rect.left) / zoomLevel.value
      const y = (event.clientY - rect.top) / zoomLevel.value
      addNodeToCanvas(item, x, y)
    }
  }
}

const onCanvasDragOver = (event: DragEvent) => {
  event.preventDefault()
  if (event.dataTransfer) {
    event.dataTransfer.dropEffect = 'copy'
  }
}

const addNodeToCanvas = (item: any, x: number, y: number) => {
  const newNode = {
    id: generateId(),
    name: item.name,
    type: item.type || 'agent',
    agentId: item.id,
    x,
    y,
    width: 200,
    height: 80,
    inputs: [],
    outputs: [],
    config: {},
    status: 'idle'
  }
  
  workflowStore.addNode(newNode)
  selectNode(newNode.id)
  hasChanges.value = true
}

const selectNode = (nodeId: string) => {
  selectedNodeId.value = nodeId
  selectedConnectionId.value = null
}

const selectConnection = (connectionId: string) => {
  selectedConnectionId.value = connectionId
  selectedNodeId.value = null
}

const moveNode = (nodeId: string, deltaX: number, deltaY: number) => {
  workflowStore.moveNode(nodeId, deltaX, deltaY)
  hasChanges.value = true
}

const deleteNode = (nodeId: string) => {
  workflowStore.deleteNode(nodeId)
  if (selectedNodeId.value === nodeId) {
    selectedNodeId.value = null
  }
  hasChanges.value = true
}

const deleteConnection = (connectionId: string) => {
  workflowStore.deleteConnection(connectionId)
  if (selectedConnectionId.value === connectionId) {
    selectedConnectionId.value = null
  }
  hasChanges.value = true
}

const duplicateNode = (nodeId: string) => {
  const node = nodes.value.find(n => n.id === nodeId)
  if (node) {
    const newNode = {
      ...node,
      id: generateId(),
      name: `${node.name} (副本)`,
      x: node.x + 20,
      y: node.y + 20
    }
    workflowStore.addNode(newNode)
    selectNode(newNode.id)
    hasChanges.value = true
  }
}

const startConnection = (sourceNodeId: string, sourcePort: string, event: MouseEvent) => {
  tempConnection.value = {
    sourceNodeId,
    sourcePort,
    targetX: event.clientX,
    targetY: event.clientY
  }
}

const getTempConnectionPath = () => {
  if (!tempConnection.value) return ''
  
  const sourceNode = nodes.value.find(n => n.id === tempConnection.value.sourceNodeId)
  if (!sourceNode) return ''
  
  const startX = sourceNode.x + sourceNode.width
  const startY = sourceNode.y + sourceNode.height / 2
  const endX = tempConnection.value.targetX / zoomLevel.value
  const endY = tempConnection.value.targetY / zoomLevel.value
  
  return `M ${startX} ${startY} Q ${(startX + endX) / 2} ${startY} ${endX} ${endY}`
}

const updateNodeProperties = (nodeId: string, properties: any) => {
  workflowStore.updateNode(nodeId, properties)
  hasChanges.value = true
}

const updateConnectionProperties = (connectionId: string, properties: any) => {
  workflowStore.updateConnection(connectionId, properties)
  hasChanges.value = true
}

const updateWorkflowProperties = (properties: any) => {
  workflowStore.updateWorkflow(properties)
  hasChanges.value = true
}

const saveWorkflow = async () => {
  try {
    await workflowStore.saveWorkflow()
    hasChanges.value = false
    lastSaved.value = new Date()
  } catch (error) {
    console.error('保存工作流失败:', error)
  }
}

const executeWorkflow = async () => {
  try {
    await workflowStore.executeWorkflow()
  } catch (error) {
    console.error('执行工作流失败:', error)
  }
}

const validateWorkflow = async () => {
  const result = await performValidation(workflowDefinition.value)
  validationModal.result = result
  validationModal.visible = true
}

const showAgentDetails = (agent: any) => {
  agentDetailsModal.agent = agent
  agentDetailsModal.visible = true
}

const addAgentToCanvas = (agent: any) => {
  const centerX = canvasSize.width / 2 / zoomLevel.value
  const centerY = canvasSize.height / 2 / zoomLevel.value
  addNodeToCanvas(agent, centerX, centerY)
  agentDetailsModal.visible = false
}

const toggleMinimap = () => {
  minimapVisible.value = !minimapVisible.value
}

const onContextMenuSelect = (action: string) => {
  // 处理右键菜单选择
  closeContextMenu()
}

const closeContextMenu = () => {
  contextMenu.visible = false
}

const fixValidationIssues = (issues: any[]) => {
  // 自动修复验证问题
  validationModal.visible = false
}

const generateId = () => {
  return 'node_' + Math.random().toString(36).substr(2, 9)
}

const formatTime = (date: Date) => {
  return date.toLocaleTimeString('zh-CN', { 
    hour: '2-digit', 
    minute: '2-digit' 
  })
}

// 生命周期
onMounted(() => {
  if (props.workflowId) {
    workflowStore.loadWorkflow(props.workflowId)
  }
  
  // 加载可用的Agent
  agentStore.loadAgents()
})

onUnmounted(() => {
  // 清理资源
})
</script>

<style scoped>
.workflow-editor {
  @apply h-full flex flex-col bg-gray-50;
}

.editor-toolbar {
  @apply flex items-center justify-between px-4 py-2 bg-white border-b border-gray-200;
}

.toolbar-left,
.toolbar-right {
  @apply flex items-center gap-2;
}

.toolbar-center {
  @apply flex items-center gap-4;
}

.workflow-name {
  @apply text-lg font-semibold text-gray-800;
}

.status-indicators {
  @apply flex items-center gap-2;
}

.zoom-controls {
  @apply flex items-center gap-1 px-2 py-1 bg-gray-100 rounded;
}

.zoom-level {
  @apply text-sm font-mono text-gray-600 min-w-[3rem] text-center;
}

.editor-main {
  @apply flex-1 flex overflow-hidden;
}

.component-panel {
  @apply w-80 bg-white border-r border-gray-200 flex flex-col transition-all duration-300;
}

.component-panel.collapsed {
  @apply w-12;
}

.panel-header {
  @apply flex items-center justify-between px-4 py-3 border-b border-gray-200;
}

.panel-content {
  @apply flex-1 overflow-y-auto;
}

.search-box {
  @apply relative m-4;
}

.search-icon {
  @apply absolute right-3 top-1/2 transform -translate-y-1/2 text-gray-400;
}

.component-categories {
  @apply px-2;
}

.category-section {
  @apply mb-2;
}

.category-header {
  @apply flex items-center gap-2 px-2 py-2 text-sm font-medium text-gray-700 cursor-pointer hover:bg-gray-50 rounded;
}

.category-title {
  @apply flex-1;
}

.category-items {
  @apply ml-4 space-y-1;
}

.component-item {
  @apply flex items-center gap-3 p-3 bg-white border border-gray-200 rounded-lg cursor-pointer hover:shadow-md transition-all duration-200;
}

.component-item:hover {
  @apply border-blue-300 bg-blue-50;
}

.component-item.control-node {
  @apply border-orange-200 bg-orange-50;
}

.item-icon {
  @apply flex-shrink-0 w-8 h-8 flex items-center justify-center rounded bg-gray-100;
}

.item-info {
  @apply flex-1 min-w-0;
}

.item-name {
  @apply font-medium text-gray-900 text-sm;
}

.item-description {
  @apply text-xs text-gray-500 mt-1;
}

.item-tags {
  @apply flex gap-1 mt-2;
}

.tag {
  @apply px-2 py-1 text-xs bg-gray-200 text-gray-600 rounded;
}

.canvas-container {
  @apply flex-1 relative overflow-hidden;
}

.canvas-wrapper {
  @apply w-full h-full;
}

.workflow-canvas {
  @apply w-full h-full cursor-crosshair;
}

.minimap {
  @apply absolute top-4 right-4 bg-white border border-gray-300 rounded shadow-lg;
}

.minimap-svg {
  @apply block;
}

.property-panel {
  @apply w-80 bg-white border-l border-gray-200 flex flex-col transition-all duration-300;
}

.property-panel.collapsed {
  @apply w-12;
}

.status-bar {
  @apply flex items-center justify-between px-4 py-2 bg-white border-t border-gray-200 text-sm text-gray-600;
}

.status-left,
.status-right {
  @apply flex items-center gap-4;
}

.status-item {
  @apply flex items-center gap-1;
}

/* 连接线样式 */
.connections-layer path {
  @apply transition-all duration-200;
}

.connections-layer path:hover {
  @apply stroke-blue-500;
  stroke-width: 3;
}

/* 节点样式 */
.nodes-layer {
  @apply transition-all duration-200;
}

/* 选择框样式 */
.selection-box {
  @apply pointer-events-none;
}

/* 响应式设计 */
@media (max-width: 1024px) {
  .component-panel,
  .property-panel {
    @apply w-64;
  }
  
  .component-panel.collapsed,
  .property-panel.collapsed {
    @apply w-0;
  }
}

@media (max-width: 768px) {
  .toolbar-center {
    @apply hidden;
  }
  
  .component-panel,
  .property-panel {
    @apply absolute top-0 bottom-0 z-10 shadow-lg;
  }
  
  .component-panel {
    @apply left-0;
  }
  
  .property-panel {
    @apply right-0;
  }
}
</style>