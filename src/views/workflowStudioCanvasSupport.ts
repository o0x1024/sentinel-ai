import type { NodeCatalogItem, WorkflowGraph, NodeDef, EdgeDef } from '@/types/workflow'

type StudioNode = {
  id: string
  type: string
  name: string
  x: number
  y: number
  params?: Record<string, any>
}

type StudioEdge = Partial<EdgeDef> & {
  from_node: string
  to_node: string
}

type FlowchartApi = {
  resetFlowchart: () => void
  addNode: (node: any) => void
  addConnectionEdge: (edge: EdgeDef) => void
}

const toJsonSchemaType = (portType?: string) => {
  switch ((portType || 'string').toLowerCase()) {
    case 'integer':
    case 'int':
    case 'number':
      return 'number'
    case 'boolean':
    case 'bool':
      return 'boolean'
    case 'array':
      return 'array'
    case 'object':
    case 'json':
      return 'object'
    default:
      return 'string'
  }
}

const buildSchemaFromNode = (node: NodeDef) => {
  const params = node.params || {}
  if (params.input_schema && typeof params.input_schema === 'object') {
    return params.input_schema as Record<string, any>
  }

  const properties: Record<string, any> = {}
  const required: string[] = []

  for (const port of node.input_ports || []) {
    if (['in', 'flow', 'trigger', 'input'].includes(port.id)) continue

    properties[port.id] = {
      type: toJsonSchemaType(port.port_type),
      description: port.name || port.id,
    }

    if (port.required) {
      required.push(port.id)
    }
  }

  if (!Object.keys(properties).length) return null

  const schema: Record<string, any> = {
    type: 'object',
    properties,
  }
  if (required.length > 0) {
    schema.required = required
  }
  return schema
}

const buildInputSchema = (nodeDefs: NodeDef[], edgeDefs: EdgeDef[]) => {
  const startLikeNodes = nodeDefs.filter((node) =>
    ['start', 'trigger', 'input', 'webhook', 'trigger_schedule'].includes(node.node_type),
  )

  for (const node of startLikeNodes) {
    const schema = buildSchemaFromNode(node)
    if (schema) return schema
  }

  const indegree = new Map<string, number>()
  nodeDefs.forEach((node) => indegree.set(node.id, 0))
  edgeDefs.forEach((edge) => indegree.set(edge.to_node, (indegree.get(edge.to_node) || 0) + 1))

  for (const node of nodeDefs.filter((entry) => (indegree.get(entry.id) || 0) === 0)) {
    const schema = buildSchemaFromNode(node)
    if (schema) return schema
  }

  for (const node of nodeDefs) {
    const schema = buildSchemaFromNode(node)
    if (schema) return schema
  }

  return null
}

const buildOutputSchema = (nodeDefs: NodeDef[], edgeDefs: EdgeDef[]) => {
  const outgoing = new Map<string, number>()
  nodeDefs.forEach((node) => outgoing.set(node.id, 0))
  edgeDefs.forEach((edge) => outgoing.set(edge.from_node, (outgoing.get(edge.from_node) || 0) + 1))

  const schemas: Record<string, any>[] = []

  for (const node of nodeDefs.filter((entry) => (outgoing.get(entry.id) || 0) === 0)) {
    const params = node.params || {}
    if (params.output_schema && typeof params.output_schema === 'object') {
      schemas.push(params.output_schema)
      continue
    }

    const properties: Record<string, any> = {}
    const required: string[] = []
    const ports = node.output_ports?.length ? node.output_ports : (node.input_ports || [])

    for (const port of ports) {
      if (['out', 'flow'].includes(port.id)) continue
      properties[port.id] = {
        type: toJsonSchemaType(port.port_type),
        description: port.name || port.id,
      }
      if (port.required) {
        required.push(port.id)
      }
    }

    if (!Object.keys(properties).length) continue

    const schema: Record<string, any> = {
      type: 'object',
      properties,
    }
    if (required.length > 0) {
      schema.required = required
    }
    schemas.push(schema)
  }

  if (schemas.length === 1) return schemas[0]
  if (schemas.length > 1) return { oneOf: schemas }
  return null
}

export const buildWorkflowStudioGraph = ({
  workflowId,
  workflowName,
  workflowVersion,
  workflowIsTool,
  nodes,
  edgesDetailed,
  fallbackEdges,
  catalogIndex,
  unnamedWorkflowLabel,
  inputPortLabel,
  outputPortLabel,
}: {
  workflowId: string
  workflowName: string
  workflowVersion: string
  workflowIsTool: boolean
  nodes: StudioNode[]
  edgesDetailed: StudioEdge[]
  fallbackEdges: StudioEdge[]
  catalogIndex: Map<string, NodeCatalogItem>
  unnamedWorkflowLabel: string
  inputPortLabel: string
  outputPortLabel: string
}): WorkflowGraph => {
  const nodeDefs: NodeDef[] = nodes.map((node) => ({
    id: node.id,
    node_type: node.type,
    node_name: node.name,
    x: Math.round(node.x),
    y: Math.round(node.y),
    params: node.params || {},
    input_ports: (() => {
      const item = catalogIndex.get(node.type)
      return item?.input_ports?.length
        ? item.input_ports
        : [{ id: 'in', name: inputPortLabel, port_type: 'Json', required: false }]
    })(),
    output_ports: (() => {
      const item = catalogIndex.get(node.type)
      return item?.output_ports?.length
        ? item.output_ports
        : [{ id: 'out', name: outputPortLabel, port_type: 'Json', required: false }]
    })(),
  }))

  const normalizedEdges = edgesDetailed.length ? edgesDetailed : fallbackEdges
  const edgeDefs: EdgeDef[] = normalizedEdges.map((edge, index) => ({
    id: edge.id || `e_${index}_${edge.from_node}_${edge.to_node}`,
    from_node: edge.from_node,
    from_port: edge.from_port || 'out',
    to_node: edge.to_node,
    to_port: edge.to_port || 'in',
    source_scope: edge.source_scope || 'output',
    source_path: edge.source_path,
    target_path: edge.target_path,
    merge_mode: edge.merge_mode || 'replace',
  }))

  let inputSchema = buildInputSchema(nodeDefs, edgeDefs)
  let outputSchema = buildOutputSchema(nodeDefs, edgeDefs)

  if (workflowIsTool) {
    if (!inputSchema) {
      inputSchema = {
        type: 'object',
        properties: {
          inputs: {
            type: 'object',
            description: 'Workflow input parameters',
          },
        },
      }
    }

    if (!outputSchema) {
      outputSchema = {
        type: 'object',
        properties: {
          result: {
            type: 'object',
            description: 'Workflow execution result',
          },
        },
      }
    }
  }

  const graph: Record<string, any> = {
    id: workflowId,
    name: workflowName || unnamedWorkflowLabel,
    version: workflowVersion || 'v1.0.0',
    nodes: nodeDefs,
    edges: edgeDefs,
    variables: [],
    credentials: [],
  }

  if (inputSchema) {
    graph.input_schema = inputSchema
  }
  if (outputSchema) {
    graph.output_schema = outputSchema
  }

  return graph as WorkflowGraph
}

export const applyWorkflowStudioGraphToCanvas = (flowRef: FlowchartApi | null, graph: WorkflowGraph) => {
  flowRef?.resetFlowchart()

  graph.nodes.forEach((node) => {
    flowRef?.addNode({
      id: node.id,
      name: node.node_name,
      description: node.node_type,
      status: 'pending',
      x: node.x,
      y: node.y,
      type: node.node_type,
      dependencies: [],
      params: node.params || {},
      metadata: { input_ports: node.input_ports || [], output_ports: node.output_ports || [] },
    })
  })

  graph.edges.forEach((edge) => {
    flowRef?.addConnectionEdge(edge)
  })
}
