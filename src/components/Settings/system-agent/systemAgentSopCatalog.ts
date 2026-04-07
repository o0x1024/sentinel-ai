export interface SystemAgentSopDefinition {
  id: string
  name: string
  description: string
  procedure: string
  updatedAt: string
}

const STORAGE_PREFIX = 'system-agent-sop-catalog:'

export function createEmptySystemAgentSopDefinition(): SystemAgentSopDefinition {
  return {
    id: '',
    name: '',
    description: '',
    procedure: '',
    updatedAt: new Date().toISOString(),
  }
}

export function loadSystemAgentSopDefinitions(profileId: string): SystemAgentSopDefinition[] {
  if (!profileId || typeof localStorage === 'undefined') {
    return []
  }

  try {
    const raw = localStorage.getItem(`${STORAGE_PREFIX}${profileId}`)
    if (!raw) return []
    const parsed = JSON.parse(raw)
    if (!Array.isArray(parsed)) return []
    return parsed
      .map(item => normalizeSystemAgentSopDefinition(item))
      .filter((item): item is SystemAgentSopDefinition => !!item)
  } catch {
    return []
  }
}

export function saveSystemAgentSopDefinitions(
  profileId: string,
  definitions: SystemAgentSopDefinition[],
) {
  if (!profileId || typeof localStorage === 'undefined') {
    return
  }

  localStorage.setItem(`${STORAGE_PREFIX}${profileId}`, JSON.stringify(definitions))
}

function normalizeSystemAgentSopDefinition(value: unknown): SystemAgentSopDefinition | null {
  if (!value || typeof value !== 'object') {
    return null
  }

  const item = value as Partial<SystemAgentSopDefinition>
  const id = typeof item.id === 'string' ? item.id.trim() : ''
  if (!id) {
    return null
  }

  return {
    id,
    name: typeof item.name === 'string' ? item.name.trim() : '',
    description: typeof item.description === 'string' ? item.description.trim() : '',
    procedure: typeof item.procedure === 'string' ? item.procedure : '',
    updatedAt: typeof item.updatedAt === 'string' && item.updatedAt
      ? item.updatedAt
      : new Date().toISOString(),
  }
}
