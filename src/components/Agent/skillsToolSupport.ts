import { parseStructuredToolPayload } from './toolRenderSupport'

export type SkillsToolAction = 'invoke' | 'fork' | 'read_file' | 'unknown'

export type ParsedSkillsToolPayload = {
  action: SkillsToolAction
  skillId: string
  skillName: string
  description?: string
  content?: string
  referencedFiles: string[]
  warnings: string[]
}

export type ParsedSkillsToolArgs = {
  skill?: string
  skillArgs?: string
}

export type SkillActivityMetadata = {
  kind?: string
  mode?: string
  skill_id?: string
  skill_name?: string
  referenced_files?: string[]
  result_preview?: string
}

const SKILLS_TOOL_NAME = 'skills'

export const isSkillsToolName = (toolName: unknown): boolean => {
  return String(toolName || '').trim().toLowerCase() === SKILLS_TOOL_NAME
}

export const parseSkillsToolArgs = (args: unknown): ParsedSkillsToolArgs => {
  const parsed = parseStructuredToolPayload(args)
  if (!parsed || typeof parsed !== 'object' || Array.isArray(parsed)) {
    return {}
  }

  const record = parsed as Record<string, unknown>
  const skill = typeof record.skill === 'string' ? record.skill.trim() : undefined
  const skillArgs =
    typeof record.args === 'string'
      ? record.args
      : typeof record.skill_args === 'string'
        ? record.skill_args
        : undefined

  return {
    skill: skill || undefined,
    skillArgs,
  }
}

const normalizeSkillsAction = (value: unknown): SkillsToolAction => {
  const action = String(value || '').trim().toLowerCase()
  if (action === 'invoke' || action === 'fork' || action === 'read_file') {
    return action
  }
  return 'unknown'
}

const normalizeStringList = (value: unknown): string[] => {
  if (!Array.isArray(value)) return []
  return value
    .filter((item): item is string => typeof item === 'string')
    .map((item) => item.trim())
    .filter(Boolean)
}

const normalizeReferencedFiles = (value: unknown): string[] => normalizeStringList(value)

export const parseSkillsToolResult = (result: unknown): ParsedSkillsToolPayload | null => {
  const parsed = parseStructuredToolPayload(result)
  if (!parsed || typeof parsed !== 'object' || Array.isArray(parsed)) {
    return null
  }

  const record = parsed as Record<string, unknown>
  const skill =
    record.skill && typeof record.skill === 'object' && !Array.isArray(record.skill)
      ? (record.skill as Record<string, unknown>)
      : null

  const skillId =
    (typeof skill?.id === 'string' && skill.id.trim()) ||
    (typeof record.skill_id === 'string' && record.skill_id.trim()) ||
    ''
  const skillName =
    (typeof skill?.name === 'string' && skill.name.trim()) ||
    skillId ||
    'unknown'
  const description =
    typeof skill?.description === 'string' ? skill.description.trim() : undefined
  const content = typeof record.content === 'string' ? record.content : undefined

  if (!skillId && !skillName) {
    return null
  }

  return {
    action: normalizeSkillsAction(record.action),
    skillId: skillId || skillName,
    skillName,
    description,
    content,
    referencedFiles: normalizeReferencedFiles(record.referenced_files),
    warnings: normalizeStringList(record.warnings),
  }
}

export const resolveSkillsDisplayFromMessage = (params: {
  toolName?: unknown
  toolArgs?: unknown
  toolResult?: unknown
}): ParsedSkillsToolPayload | null => {
  if (!isSkillsToolName(params.toolName)) {
    return null
  }

  const parsedResult = parseSkillsToolResult(params.toolResult)
  if (parsedResult) {
    return parsedResult
  }

  const parsedArgs = parseSkillsToolArgs(params.toolArgs)
  if (!parsedArgs.skill) {
    return null
  }

  return {
    action: 'unknown',
    skillId: parsedArgs.skill,
    skillName: parsedArgs.skill,
    referencedFiles: [],
    warnings: [],
  }
}

export const parseSkillActivityMetadata = (
  metadata: unknown,
): SkillActivityMetadata | null => {
  if (!metadata || typeof metadata !== 'object' || Array.isArray(metadata)) {
    return null
  }
  return metadata as SkillActivityMetadata
}

export const isSkillActivityMessage = (message: {
  type?: string
  metadata?: unknown
}): boolean => {
  if (message.type !== 'system') return false
  const metadata = parseSkillActivityMetadata(message.metadata)
  return metadata?.kind === 'skill_loaded' || metadata?.kind === 'skill_forked'
}

export type SkillFileBlock = {
  path: string
  content: string
}

const SKILL_FILE_BLOCK_RE = /<file path="([^"]+)">\s*([\s\S]*?)\s*<\/file>/gi
const SKILL_BODY_RE = /<skill>\s*([\s\S]*?)\s*<\/skill>/i

export function parseSkillFileBlocks(content: string): SkillFileBlock[] {
  if (!content.trim()) return []

  const blocks: SkillFileBlock[] = []
  const seen = new Set<string>()
  for (const match of content.matchAll(SKILL_FILE_BLOCK_RE)) {
    const path = match[1]?.trim()
    const body = match[2]?.trim()
    if (!path || !body || seen.has(path)) continue
    seen.add(path)
    blocks.push({ path, content: body })
  }
  return blocks
}

export function extractSkillInvokeMarkdown(content: string): string {
  const trimmed = content.trim()
  if (!trimmed) return ''

  const skillMatch = trimmed.match(SKILL_BODY_RE)
  if (skillMatch?.[1]?.trim()) {
    return skillMatch[1].trim()
  }

  const loadedPrefix = /^Skill loaded:[^\n]*\n*/i
  const forkPrefix = /^Skill "[^"]+" completed \(forked execution\)\.\n*(?:Result:\n*)?/i
  return trimmed.replace(loadedPrefix, '').replace(forkPrefix, '').trim()
}

export function resolveSkillFileSections(
  content?: string,
  referencedFiles?: string[],
): SkillFileBlock[] {
  const normalizedContent = content?.trim() || ''
  if (!normalizedContent) return []

  const fileBlocks = parseSkillFileBlocks(normalizedContent)
  if (fileBlocks.length > 0) return fileBlocks

  const invokeMarkdown = extractSkillInvokeMarkdown(normalizedContent)
  if (!invokeMarkdown) return []

  const fallbackPath =
    referencedFiles?.find((file) => String(file || '').trim().length > 0)?.trim() || 'SKILL.md'

  return [{ path: fallbackPath, content: invokeMarkdown }]
}

export const compactSkillResultPreview = (content: string, maxChars = 500): string => {
  const trimmed = content.trim()
  if (trimmed.length <= maxChars) return trimmed
  return `${trimmed.slice(0, maxChars)}…`
}

const SKILLS_ERROR_MARKERS = [
  'Skills operation failed:',
  'Skill lookup failed:',
  'Skill not found:',
  'Invalid arguments:',
]

export const isSkillsInvokeToolSuccess = (result: unknown): boolean => {
  const parsed = parseSkillsToolResult(result)
  if (!parsed || parsed.action !== 'invoke') {
    return false
  }
  const content = parsed.content?.trim() || ''
  return content.startsWith('Skill loaded:') && content.includes('<skill>')
}

export const isSkillsForkToolSuccess = (result: unknown): boolean => {
  const parsed = parseSkillsToolResult(result)
  if (!parsed || parsed.action !== 'fork') {
    return false
  }
  const content = parsed.content || ''
  return content.includes('forked execution') || content.startsWith('Skill "')
}

export const isSkillsReadFileSuccess = (result: unknown): boolean => {
  const parsed = parseSkillsToolResult(result)
  if (!parsed || parsed.action !== 'read_file') {
    return false
  }
  const content = parsed.content || ''
  return content.includes('<file path=')
}

export const isSkillsToolSuccess = (result: unknown): boolean => {
  return isSkillsInvokeToolSuccess(result) || isSkillsForkToolSuccess(result) || isSkillsReadFileSuccess(result)
}

export const extractSkillsToolErrorMessage = (
  error: unknown,
  result: unknown,
): string | undefined => {
  if (isSkillsToolSuccess(result)) {
    return undefined
  }
  if (typeof error === 'string' && error.trim()) {
    return error.trim()
  }

  const parsed = parseStructuredToolPayload(result)
  const candidates: string[] = []

  const collectText = (value: unknown) => {
    if (typeof value !== 'string' || !value.trim()) return
    candidates.push(value.trim())
  }

  if (typeof parsed === 'string') {
    collectText(parsed)
  } else if (Array.isArray(parsed)) {
    for (const item of parsed) {
      if (item && typeof item === 'object' && !Array.isArray(item)) {
        collectText((item as Record<string, unknown>).text)
      }
    }
  } else if (parsed && typeof parsed === 'object') {
    collectText((parsed as Record<string, unknown>).text)
    collectText((parsed as Record<string, unknown>).error)
  }

  for (const candidate of candidates) {
    for (const marker of SKILLS_ERROR_MARKERS) {
      const index = candidate.indexOf(marker)
      if (index >= 0) {
        return candidate.slice(index).replace(/\s+/g, ' ').trim()
      }
    }
    if (/toolset error|tool execution failed|toolcallerror/i.test(candidate)) {
      return candidate.replace(/\s+/g, ' ').trim()
    }
  }

  return undefined
}
