export type FieldControl =
  | 'enum'
  | 'string'
  | 'number'
  | 'boolean'
  | 'array-multiselect'
  | 'array-lines'
  | 'object-fields'
  | 'json'

export type SchemaProperty = {
  type?: string
  description?: string
  default?: unknown
  enum?: unknown[]
  items?: Record<string, any>
  minimum?: number
  maximum?: number
  readOnly?: boolean
  properties?: Record<string, SchemaProperty>
}

export type EditableField = {
  name: string
  path: string[]
  type: string
  description: string
  defaultValue: unknown
  required: boolean
  enumValues: unknown[]
  items?: Record<string, any>
  minimum?: number
  maximum?: number
  properties?: Record<string, SchemaProperty>
  objectFields: EditableField[]
  control: FieldControl
  typeLabel: string
  secret: boolean
  arraySuggestedOptions: string[]
}

export const PARAM_EDITOR_INVALID_KEY = '__monitorParamEditorInvalid'
export const PARAM_EDITOR_ERROR_KEY = '__monitorParamEditorErrors'
export const INJECTED_MONITOR_FIELD_NAMES = new Set([
  'targets',
  'target_objects',
  'service_targets',
  'previousSnapshots',
  '__monitorExecution',
  'monitorProgress',
  'domain',
  'domains',
  'url',
  'urls',
  'dictionary',
  'dictionary_id',
  'serviceProbeEngine',
])

const SECRET_FIELD_PATTERNS = [
  /token/i,
  /secret/i,
  /password/i,
  /passwd/i,
  /pass$/i,
  /api[_-]?key/i,
  /[_-]key$/i,
]

export const cloneValue = <T>(value: T): T => {
  if (value === undefined) {
    return value
  }
  return JSON.parse(JSON.stringify(value))
}

export const getFieldPathKey = (path: string[]) => path.join('.')

export const formatHintValue = (value: unknown) => {
  if (value === undefined) {
    return ''
  }
  if (typeof value === 'string') {
    return value
  }

  try {
    return JSON.stringify(value, null, 2)
  } catch {
    return String(value)
  }
}

export const buildTypeLabel = (property: SchemaProperty) => {
  if (Array.isArray(property.enum) && property.enum.length > 0) {
    return `enum(${property.enum.length})`
  }

  if (property.type === 'array' && property.items?.type) {
    return `array<${property.items.type}>`
  }

  return property.type || 'json'
}

const parseAvailableOptionsFromDescription = (description: string) => {
  const availableMatch = description.match(/Available:\s*([^.]*)/i)
  if (!availableMatch?.[1]) {
    return []
  }

  return Array.from(
    new Set(
      availableMatch[1]
        .split(',')
        .map(option => option.trim())
        .filter(Boolean)
    )
  )
}

export const isSecretFieldName = (fieldName: string) =>
  SECRET_FIELD_PATTERNS.some(pattern => pattern.test(fieldName))

const getFieldControl = (property: SchemaProperty): FieldControl => {
  if (Array.isArray(property.enum) && property.enum.length > 0) {
    return 'enum'
  }

  if (property.type === 'object' && property.properties && Object.keys(property.properties).length > 0) {
    return 'object-fields'
  }

  if (
    property.type === 'array'
    && property.items
    && ['string', 'number', 'integer'].includes(String(property.items.type || ''))
    && !property.items.properties
  ) {
    const suggestedOptions = parseAvailableOptionsFromDescription(String(property.description || ''))
    if (String(property.items.type || '') === 'string' && suggestedOptions.length > 0) {
      return 'array-multiselect'
    }
    return 'array-lines'
  }

  switch (property.type) {
    case 'string':
      return 'string'
    case 'number':
    case 'integer':
      return 'number'
    case 'boolean':
      return 'boolean'
    default:
      return 'json'
  }
}

const buildEditableField = (
  name: string,
  property: SchemaProperty,
  requiredFields: Set<string>,
  path: string[],
): EditableField => {
  const objectRequiredFields = new Set<string>(Array.isArray((property as any)?.required) ? (property as any).required : [])
  const objectProperties =
    property.properties && typeof property.properties === 'object'
      ? property.properties
      : {}

  const objectFields = Object.entries(objectProperties)
    .filter(([, childProperty]) => childProperty.readOnly !== true)
    .map(([childName, childProperty]) =>
      buildEditableField(childName, childProperty, objectRequiredFields, [...path, childName])
    )

  return {
    name,
    path,
    type: String(property.type || 'object'),
    description: String(property.description || '').trim(),
    defaultValue: cloneValue(property.default),
    required: requiredFields.has(name),
    enumValues: Array.isArray(property.enum) ? property.enum : [],
    items: property.items,
    minimum: property.minimum,
    maximum: property.maximum,
    properties: property.properties,
    objectFields,
    control: getFieldControl(property),
    typeLabel: buildTypeLabel(property),
    secret: isSecretFieldName(name),
    arraySuggestedOptions:
      property.type === 'array' && property.items?.type === 'string'
        ? parseAvailableOptionsFromDescription(String(property.description || ''))
        : [],
  }
}

export const buildEditableFields = (schema: any) => {
  const properties = schema?.properties
  if (!properties || typeof properties !== 'object') {
    return [] as EditableField[]
  }

  const requiredFields = new Set<string>(Array.isArray(schema?.required) ? schema.required : [])

  return Object.entries(properties)
    .filter(([name, property]) => {
      if (INJECTED_MONITOR_FIELD_NAMES.has(name)) {
        return false
      }

      return (property as SchemaProperty).readOnly !== true
    })
    .map(([name, property]) =>
      buildEditableField(name, property as SchemaProperty, requiredFields, [name])
    )
}

export const getValueAtPath = (root: Record<string, any>, path: string[]) => {
  let current: any = root
  for (const segment of path) {
    if (!current || typeof current !== 'object' || Array.isArray(current)) {
      return undefined
    }
    current = current[segment]
  }
  return current
}

const pruneEmptyAncestors = (root: Record<string, any>, path: string[]) => {
  for (let depth = path.length - 1; depth > 0; depth -= 1) {
    const parentPath = path.slice(0, depth)
    const parent = getValueAtPath(root, parentPath)
    if (!parent || typeof parent !== 'object' || Array.isArray(parent)) {
      continue
    }
    if (Object.keys(parent).length > 0) {
      continue
    }

    const grandParent = getValueAtPath(root, parentPath.slice(0, -1))
    const segment = parentPath[parentPath.length - 1]
    if (grandParent && typeof grandParent === 'object' && !Array.isArray(grandParent)) {
      delete grandParent[segment]
    }
  }
}

export const deleteValueAtPath = (root: Record<string, any>, path: string[]) => {
  if (path.length === 0) {
    return
  }

  const parent = path.length === 1 ? root : getValueAtPath(root, path.slice(0, -1))
  if (!parent || typeof parent !== 'object' || Array.isArray(parent)) {
    return
  }

  delete parent[path[path.length - 1]]
  pruneEmptyAncestors(root, path)
}

export const setValueAtPath = (root: Record<string, any>, path: string[], value: unknown) => {
  let current: any = root
  for (const segment of path.slice(0, -1)) {
    if (!current[segment] || typeof current[segment] !== 'object' || Array.isArray(current[segment])) {
      current[segment] = {}
    }
    current = current[segment]
  }
  current[path[path.length - 1]] = value
}

export const hasExplicitValueAtPath = (root: Record<string, any>, path: string[]) =>
  getValueAtPath(root, path) !== undefined

export const hasCustomizedField = (root: Record<string, any>, field: EditableField): boolean => {
  if (hasExplicitValueAtPath(root, field.path)) {
    return true
  }

  return field.objectFields.some(child => hasCustomizedField(root, child))
}

export const setPluginParamEditorState = (plugin: Record<string, any>, errors: string[]) => {
  plugin[PARAM_EDITOR_INVALID_KEY] = errors.length > 0
  plugin[PARAM_EDITOR_ERROR_KEY] = errors
}

export const clearPluginParamEditorState = (plugin: Record<string, any>) => {
  delete plugin[PARAM_EDITOR_INVALID_KEY]
  delete plugin[PARAM_EDITOR_ERROR_KEY]
}

export const buildFieldHint = (
  field: EditableField,
  t: (key: string, params?: Record<string, unknown>) => string,
) => {
  const hints: string[] = []
  if (field.description) {
    hints.push(field.description)
  }
  if (field.defaultValue !== undefined) {
    hints.push(
      t('bugBounty.monitor.pluginParamDefaultValue', {
        value: formatHintValue(field.defaultValue),
      })
    )
  }
  if (field.minimum !== undefined || field.maximum !== undefined) {
    hints.push(
      t('bugBounty.monitor.pluginParamRange', {
        min: field.minimum ?? '-',
        max: field.maximum ?? '-',
      })
    )
  }

  return hints.join(' · ')
}

export const HINT_PREVIEW_LENGTH = 160

export const isLongHint = (hint: string) =>
  hint.length > HINT_PREVIEW_LENGTH || hint.includes('\n')

export const getCollapsedHint = (hint: string) => {
  if (!isLongHint(hint)) {
    return hint
  }

  const normalized = hint.replace(/\s+/g, ' ').trim()
  if (normalized.length <= HINT_PREVIEW_LENGTH) {
    return normalized
  }

  return `${normalized.slice(0, HINT_PREVIEW_LENGTH).trimEnd()}...`
}
