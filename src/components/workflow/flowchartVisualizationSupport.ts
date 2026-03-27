import type { EdgeDef } from '@/types/workflow'

export type NodeStatus = 'pending' | 'planning' | 'running' | 'completed' | 'failed' | 'paused' | 'cancelled'

export interface FlowchartNode {
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

export interface FlowchartConnection {
  id: string
  edgeId: string
  from: string
  to: string
  path: string
  status: 'inactive' | 'active' | 'completed' | 'failed'
  label?: string
  labelX?: number
  labelY?: number
  condition?: string
}

export interface HistoryState {
  nodes: FlowchartNode[]
  edges: EdgeDef[]
}

export type FlowchartConnectionStatus = FlowchartConnection['status']

export const createFlowchartHistoryState = (
  nodes: FlowchartNode[],
  edges: EdgeDef[],
): HistoryState => ({
  nodes: JSON.parse(JSON.stringify(nodes)),
  edges: JSON.parse(JSON.stringify(edges)),
})

export const restoreFlowchartHistoryState = (state: HistoryState) => ({
  nodes: JSON.parse(JSON.stringify(state.nodes)) as FlowchartNode[],
  edges: JSON.parse(JSON.stringify(state.edges)) as EdgeDef[],
})

export const getFlowchartNodeClass = (node: FlowchartNode) => {
  const baseClasses = ['bg-base-100', 'hover:shadow-lg']
  switch (node.status) {
    case 'pending':
      return [...baseClasses, 'border-gray-300', 'text-base-content/70']
    case 'planning':
      return [...baseClasses, 'border-blue-400', 'bg-blue-50']
    case 'running':
      return [...baseClasses, 'border-yellow-400', 'bg-yellow-50', 'animate-pulse']
    case 'completed':
      return [...baseClasses, 'border-green-400', 'bg-green-50']
    case 'failed':
      return [...baseClasses, 'border-red-400', 'bg-red-50']
    case 'paused':
      return [...baseClasses, 'border-orange-400', 'bg-orange-50']
    default:
      return baseClasses
  }
}

export const getFlowchartStatusIndicatorClass = (status: NodeStatus) => {
  switch (status) {
    case 'pending': return 'bg-gray-400'
    case 'planning': return 'bg-blue-400 animate-pulse'
    case 'running': return 'bg-yellow-400 animate-pulse'
    case 'completed': return 'bg-green-400'
    case 'failed': return 'bg-red-400'
    case 'paused': return 'bg-orange-400'
    default: return 'bg-gray-400'
  }
}

export const getFlowchartStatusBadgeClass = (status: NodeStatus) => {
  switch (status) {
    case 'pending': return 'badge-ghost'
    case 'planning': return 'badge-info'
    case 'running': return 'badge-warning'
    case 'completed': return 'badge-success'
    case 'failed': return 'badge-error'
    case 'paused': return 'badge-secondary'
    case 'cancelled': return 'badge-neutral'
    default: return 'badge-ghost'
  }
}

export const getFlowchartConnectionClass = (connection: FlowchartConnection) => {
  switch (connection.status) {
    case 'active': return 'stroke-warning'
    case 'completed': return 'stroke-success'
    case 'failed': return 'stroke-error'
    default: return 'stroke-base-content/30'
  }
}

export const getFlowchartStatusText = (status: NodeStatus, t: (key: string) => string): string => {
  switch (status) {
    case 'pending': return t('trafficAnalysis.workflowStudio.flowchart.status.pending')
    case 'planning': return t('trafficAnalysis.workflowStudio.flowchart.status.planning')
    case 'running': return t('trafficAnalysis.workflowStudio.flowchart.status.running')
    case 'completed': return t('trafficAnalysis.workflowStudio.flowchart.status.completed')
    case 'failed': return t('trafficAnalysis.workflowStudio.flowchart.status.failed')
    case 'paused': return t('trafficAnalysis.workflowStudio.flowchart.status.paused')
    case 'cancelled': return t('trafficAnalysis.workflowStudio.flowchart.status.cancelled')
    default: return status
  }
}

export const getFlowchartNodeIcon = (nodeType: string): string => {
  if (nodeType === 'branch') return '🔀'
  if (nodeType === 'merge') return '🔗'
  if (nodeType === 'retry') return '🔄'
  if (nodeType === 'raw') return '🧾'
  if (nodeType === 'trigger') return '⚡'
  if (nodeType === 'output') return '📤'
  return ''
}

export const calculateFlowchartConnectionPath = (from: FlowchartNode, to: FlowchartNode, curved = false): string => {
  const NODE_WIDTH = 180
  const NODE_HEIGHT = 80

  const fromX = from.x + NODE_WIDTH
  const fromY = from.y + NODE_HEIGHT / 2
  const toX = to.x
  const toY = to.y + NODE_HEIGHT / 2

  if (curved) {
    const midX = (fromX + toX) / 2 + 100
    const midY = Math.min(fromY, toY) - 50
    return `M ${fromX} ${fromY} Q ${midX} ${midY} ${toX} ${toY}`
  }

  const dx = toX - fromX
  const controlOffset = Math.min(Math.abs(dx) * 0.5, 80)
  return `M ${fromX} ${fromY} C ${fromX + controlOffset} ${fromY}, ${toX - controlOffset} ${toY}, ${toX} ${toY}`
}

export const getFlowchartConnectionStatus = (from: FlowchartNode, to: FlowchartNode): FlowchartConnectionStatus => {
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

const compactEdgePath = (path?: string) => {
  const trimmed = path?.trim()
  if (!trimmed) return ''
  const segments = trimmed.split('.').filter(Boolean)
  if (segments.length <= 2) return segments.join('.')
  return `${segments[0]}...${segments[segments.length - 1]}`
}

export const describeFlowchartEdgeMapping = (edge: EdgeDef) => {
  const target = compactEdgePath(edge.target_path) || '?'
  const source = compactEdgePath(edge.source_path)
  const arrow = edge.source_scope === 'input' ? '⇢' : '↦'

  if (!source) return `${arrow}${target}`
  return `${source}${arrow}${target}`
}

export const calculateFlowchartConnectionLabelPosition = (from: FlowchartNode, to: FlowchartNode) => {
  const NODE_WIDTH = 180
  const NODE_HEIGHT = 80

  const fromX = from.x + NODE_WIDTH
  const fromY = from.y + NODE_HEIGHT / 2
  const toX = to.x
  const toY = to.y + NODE_HEIGHT / 2

  return {
    x: (fromX + toX) / 2,
    y: (fromY + toY) / 2 - 12,
  }
}

export const arrangeFlowchartNodes = (nodes: FlowchartNode[]) => {
  if (!nodes.length) return

  const levelMap: Record<string, number> = {}

  const computeLevel = (node: FlowchartNode, seen: Set<string>): number => {
    if (levelMap[node.id] !== undefined) return levelMap[node.id]
    if (!node.dependencies?.length || seen.has(node.id)) {
      levelMap[node.id] = 0
      return 0
    }

    seen.add(node.id)
    let maxDependencyLevel = 0
    node.dependencies.forEach((dependencyId) => {
      const dependency = nodes.find((entry) => entry.id === dependencyId)
      if (!dependency) return
      maxDependencyLevel = Math.max(maxDependencyLevel, computeLevel(dependency, seen))
    })
    levelMap[node.id] = maxDependencyLevel + 1
    return levelMap[node.id]
  }

  nodes.forEach((node) => computeLevel(node, new Set()))

  const groupedByLevel: Record<number, FlowchartNode[]> = {}
  Object.entries(levelMap).forEach(([id, level]) => {
    const node = nodes.find((entry) => entry.id === id)
    if (!node) return
    if (!groupedByLevel[level]) groupedByLevel[level] = []
    groupedByLevel[level].push(node)
  })

  const HORIZONTAL_SPACING = 200
  const VERTICAL_SPACING = 140
  const START_X = 50
  const START_Y = 50

  Object.keys(groupedByLevel)
    .map((level) => Number.parseInt(level, 10))
    .sort((left, right) => left - right)
    .forEach((level, rowIndex) => {
      groupedByLevel[level].forEach((node, columnIndex) => {
        node.x = START_X + columnIndex * HORIZONTAL_SPACING
        node.y = START_Y + rowIndex * VERTICAL_SPACING
      })
    })
}
