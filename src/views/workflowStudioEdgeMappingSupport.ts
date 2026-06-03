import type { EdgeDef, JsonSchema, NodeCatalogItem } from '@/types/workflow'

type StudioNodeLike = {
  id: string
  type: string
  name?: string
  params?: Record<string, any>
}

export type EdgeMappingOption = {
  path: string
  label: string
  description?: string
}

const normalizePathPrefix = (prefix: string) => prefix.trim().replace(/\.$/, '')

const joinPath = (prefix: string, key: string | number) => {
  const normalized = normalizePathPrefix(prefix)
  return normalized ? `${normalized}.${key}` : String(key)
}

const collectValuePaths = (
  value: unknown,
  prefix = '',
  options: EdgeMappingOption[] = [],
  seen = new Set<string>(),
): EdgeMappingOption[] => {
  const push = (path: string, description?: string) => {
    if (!path || seen.has(path)) return
    seen.add(path)
    options.push({ path, label: path, description })
  }

  if (Array.isArray(value)) {
    value.forEach((entry, index) => {
      const path = joinPath(prefix, index)
      push(path, '数组项')
      collectValuePaths(entry, path, options, seen)
    })
    return options
  }

  if (value && typeof value === 'object') {
    Object.entries(value as Record<string, unknown>).forEach(([key, entry]) => {
      const path = joinPath(prefix, key)
      const description = Array.isArray(entry)
        ? '数组'
        : entry && typeof entry === 'object'
          ? '对象'
          : typeof entry
      push(path, description)
      collectValuePaths(entry, path, options, seen)
    })
  }

  return options
}

const collectSchemaPaths = (
  schema: JsonSchema | undefined,
  prefix = '',
  options: EdgeMappingOption[] = [],
  seen = new Set<string>(),
) => {
  if (!schema || !schema.properties) return options

  Object.entries(schema.properties).forEach(([key, child]) => {
    const path = joinPath(prefix, key)
    if (!seen.has(path)) {
      seen.add(path)
      options.push({
        path,
        label: path,
        description: child.description || child.type || '字段',
      })
    }

    if (child.type === 'object' && child.properties) {
      collectSchemaPaths(child, path, options, seen)
    }
  })

  return options
}

const tryParseRawNodeValue = (node: StudioNodeLike | undefined) => {
  if (!node || node.type !== 'raw') return null
  const rawType = String(node.params?.raw_type || 'json')
  const rawValue = String(node.params?.value || '').trim()
  if (!rawValue) return null
  if (rawType === 'text') return rawValue

  try {
    return JSON.parse(rawValue)
  } catch {
    return null
  }
}

export const buildTargetPathOptions = (
  edge: EdgeDef | null,
  nodes: StudioNodeLike[],
  catalogIndex: Map<string, NodeCatalogItem>,
): EdgeMappingOption[] => {
  if (!edge) return []
  const targetNode = nodes.find((node) => node.id === edge.to_node)
  if (!targetNode) return []

  const schema = catalogIndex.get(targetNode.type)?.params_schema
  return collectSchemaPaths(schema)
}

export const buildSourcePathOptions = ({
  edge,
  nodes,
  catalogIndex,
  stepResults,
}: {
  edge: EdgeDef | null
  nodes: StudioNodeLike[]
  catalogIndex: Map<string, NodeCatalogItem>
  stepResults: Record<string, any>
}): EdgeMappingOption[] => {
  if (!edge) return []
  const sourceNode = nodes.find((node) => node.id === edge.from_node)
  if (!sourceNode) return []

  const options: EdgeMappingOption[] = []
  const seen = new Set<string>()

  const runtimeValue = edge.source_scope === 'output'
    ? stepResults[edge.from_node]
    : sourceNode.params

  if (runtimeValue !== undefined) {
    collectValuePaths(runtimeValue, '', options, seen)
  }

  if (edge.source_scope === 'input') {
    const inputSchema = catalogIndex.get(sourceNode.type)?.params_schema
    collectSchemaPaths(inputSchema, '', options, seen)
  } else {
    const rawValue = tryParseRawNodeValue(sourceNode)
    if (rawValue !== null) {
      collectValuePaths(rawValue, '', options, seen)
    }
  }

  return options
}
