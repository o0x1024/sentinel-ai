import { parser } from '@lezer/json'
import type { SyntaxNode } from '@lezer/common'

function normalizeLineEndings(value: string): string {
  return value.replace(/\r\n/g, '\n').replace(/\r/g, '\n')
}

export function normalizeRepeaterPrettyRequestLineEndings(rawRequest: string): string {
  return normalizeLineEndings(rawRequest)
}

function splitRequestSections(value: string): { headerPart: string; bodyPart: string | null } {
  const normalized = normalizeLineEndings(value)
  const separatorIndex = normalized.indexOf('\n\n')

  if (separatorIndex === -1) {
    return {
      headerPart: normalized,
      bodyPart: null,
    }
  }

  return {
    headerPart: normalized.slice(0, separatorIndex),
    bodyPart: normalized.slice(separatorIndex + 2),
  }
}

function isParseErrorNode(node: SyntaxNode): boolean {
  return node.type.isError || node.type.name === '⚠'
}

function hasParseErrors(node: SyntaxNode): boolean {
  if (isParseErrorNode(node)) return true

  for (let child = node.firstChild; child; child = child.nextSibling) {
    if (hasParseErrors(child)) return true
  }

  return false
}

function indent(depth: number): string {
  return '  '.repeat(depth)
}

function formatJsonNode(node: SyntaxNode, source: string, depth: number): string {
  switch (node.type.name) {
    case 'Object':
      return formatJsonObject(node, source, depth)
    case 'Array':
      return formatJsonArray(node, source, depth)
    case 'Property':
      return formatJsonProperty(node, source, depth)
    case 'PropertyName':
    case 'String':
    case 'Number':
    case 'True':
    case 'False':
    case 'Null':
      return source.slice(node.from, node.to)
    default:
      return source.slice(node.from, node.to)
  }
}

function formatJsonObject(node: SyntaxNode, source: string, depth: number): string {
  const properties: string[] = []

  for (let child = node.firstChild; child; child = child.nextSibling) {
    if (child.type.name === 'Property') {
      properties.push(`${indent(depth + 1)}${formatJsonProperty(child, source, depth + 1)}`)
    }
  }

  if (properties.length === 0) return '{}'
  return `{\n${properties.join(',\n')}\n${indent(depth)}}`
}

function formatJsonArray(node: SyntaxNode, source: string, depth: number): string {
  const values: string[] = []

  for (let child = node.firstChild; child; child = child.nextSibling) {
    if (child.type.name === '[' || child.type.name === ']' || child.type.name === ',') {
      continue
    }
    values.push(`${indent(depth + 1)}${formatJsonNode(child, source, depth + 1)}`)
  }

  if (values.length === 0) return '[]'
  return `[\n${values.join(',\n')}\n${indent(depth)}]`
}

function formatJsonProperty(node: SyntaxNode, source: string, depth: number): string {
  let propertyName: SyntaxNode | null = null
  let propertyValue: SyntaxNode | null = null

  for (let child = node.firstChild; child; child = child.nextSibling) {
    if (child.type.name === 'PropertyName') {
      propertyName = child
      continue
    }
    if (child.type.name === ':') continue
    propertyValue = child
  }

  if (!propertyName || !propertyValue) {
    return source.slice(node.from, node.to)
  }

  return `${formatJsonNode(propertyName, source, depth)}:${formatJsonNode(propertyValue, source, depth)}`
}

function formatJsonBodyIfPossible(bodyPart: string): string | null {
  const trimmed = bodyPart.trim()
  if (!trimmed) return null
  if (!trimmed.startsWith('{') && !trimmed.startsWith('[')) return null

  try {
    JSON.parse(trimmed)
  } catch {
    return null
  }

  const tree = parser.parse(trimmed)
  const rootNode = tree.topNode.firstChild
  if (!rootNode || hasParseErrors(rootNode)) return null

  return formatJsonNode(rootNode, trimmed, 0)
}

export function formatRepeaterPrettyRequest(rawRequest: string): string {
  if (!rawRequest) return ''

  const { headerPart, bodyPart } = splitRequestSections(rawRequest)
  if (bodyPart == null) return headerPart

  let result = `${headerPart}\n\n`
  if (!bodyPart.trim()) return result
  result += formatJsonBodyIfPossible(bodyPart) ?? bodyPart

  return result
}

export function convertRepeaterPrettyRequestToRaw(prettyRequest: string): string {
  const normalized = normalizeLineEndings(prettyRequest)
  const lines = normalized.split('\n')
  const headerEnd = lines.findIndex((line) => line.trim() === '')

  if (headerEnd === -1) {
    return lines.join('\r\n')
  }

  const headerPart = lines.slice(0, headerEnd).join('\r\n')
  const bodyPart = lines.slice(headerEnd + 1).join('\r\n')
  return `${headerPart}\r\n\r\n${bodyPart}`
}
