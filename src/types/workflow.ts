export type PortType = 'String' | 'Integer' | 'Float' | 'Boolean' | 'Json' | 'Array' | 'Object' | 'Artifact'

export interface JsonSchema {
  type?: string
  properties?: Record<string, JsonSchema>
  items?: JsonSchema
  enum?: string[]
  required?: string[]
  additional_properties?: boolean
}

export interface PortDef {
  id: string
  name: string
  port_type: PortType
  required: boolean
}

export interface VariableDef {
  name: string
  var_type: PortType
  default?: any
}

export interface CredentialRef {
  name: string
  provider: string
  ref_id?: string
}

export interface NodeDef {
  id: string
  node_type: string
  node_name: string
  x: number
  y: number
  params: Record<string, any>
  input_ports: PortDef[]
  output_ports: PortDef[]
}

export interface EdgeDef {
  id: string
  from_node: string
  from_port: string
  to_node: string
  to_port: string
}

export interface WorkflowGraph {
  id: string
  name: string
  version: string
  nodes: NodeDef[]
  edges: EdgeDef[]
  variables: VariableDef[]
  credentials: CredentialRef[]
}

export interface NodeCatalogItem {
  node_type: string
  label: string
  category: string
  params_schema: JsonSchema
  input_ports: PortDef[]
  output_ports: PortDef[]
}

export interface WorkflowValidationIssue {
  code: string
  message: string
  node_id?: string
  edge_id?: string
}

export function validate_workflow_graph(graph: WorkflowGraph): WorkflowValidationIssue[] {
  const issues: WorkflowValidationIssue[] = []
  const node_ids = new Set(graph.nodes.map(n => n.id))
  const node_port_map: Record<string, { inputs: Set<string>, outputs: Set<string> }> = {}
  graph.nodes.forEach(n => {
    node_port_map[n.id] = {
      inputs: new Set(n.input_ports.map(p => p.id)),
      outputs: new Set(n.output_ports.map(p => p.id)),
    }
  })

  graph.edges.forEach(e => {
    if (!node_ids.has(e.from_node)) {
      issues.push({ code: 'edge_from_missing', message: `from_node not found: ${e.from_node}`, edge_id: e.id })
    } else if (!node_port_map[e.from_node].outputs.has(e.from_port)) {
      issues.push({ code: 'edge_from_port_missing', message: `from_port not found: ${e.from_port}`, edge_id: e.id })
    }
    if (!node_ids.has(e.to_node)) {
      issues.push({ code: 'edge_to_missing', message: `to_node not found: ${e.to_node}`, edge_id: e.id })
    } else if (!node_port_map[e.to_node].inputs.has(e.to_port)) {
      issues.push({ code: 'edge_to_port_missing', message: `to_port not found: ${e.to_port}`, edge_id: e.id })
    }
  })

  const indegree: Record<string, number> = {}
  const adj: Record<string, string[]> = {}
  graph.nodes.forEach(n => { indegree[n.id] = 0; adj[n.id] = [] })
  graph.edges.forEach(e => { indegree[e.to_node] = (indegree[e.to_node] ?? 0) + 1; adj[e.from_node].push(e.to_node) })
  const q: string[] = []
  Object.keys(indegree).forEach(id => { if (indegree[id] === 0) q.push(id) })
  let visited = 0
  while (q.length) {
    const u = q.shift() as string
    visited += 1
    for (const v of adj[u]) {
      indegree[v] -= 1
      if (indegree[v] === 0) q.push(v)
    }
  }
  if (visited !== graph.nodes.length) {
    issues.push({ code: 'cycle_detected', message: 'graph contains cycle' })
  }

  graph.nodes.forEach(n => {
    const required_inputs = n.input_ports.filter(p => p.required).map(p => p.id)
    const has_inputs = new Set(graph.edges.filter(e => e.to_node === n.id).map(e => e.to_port))
    required_inputs.forEach(pid => {
      if (!has_inputs.has(pid)) {
        issues.push({ code: 'missing_input', message: `required input missing: ${pid}`, node_id: n.id })
      }
    })
  })

  return issues
}
