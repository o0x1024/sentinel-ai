import type { SystemAgentSopDefinition } from '../systemAgentSettingsSupport'

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

export function normalizeSystemAgentSopDefinitions(
  values: unknown,
): SystemAgentSopDefinition[] {
  if (!Array.isArray(values)) {
    return []
  }

  return values
    .map(item => normalizeSystemAgentSopDefinition(item))
    .filter((item): item is SystemAgentSopDefinition => !!item)
}

export function loadLegacySystemAgentSopDefinitions(
  profileId: string,
): SystemAgentSopDefinition[] {
  if (!profileId || typeof localStorage === 'undefined') {
    return []
  }

  try {
    const raw = localStorage.getItem(`${STORAGE_PREFIX}${profileId}`)
    if (!raw) return []
    return normalizeSystemAgentSopDefinitions(JSON.parse(raw))
  } catch {
    return []
  }
}

function normalizeSystemAgentSopDefinition(
  value: unknown,
): SystemAgentSopDefinition | null {
  if (!value || typeof value !== 'object' || Array.isArray(value)) {
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
    updatedAt:
      typeof item.updatedAt === 'string' && item.updatedAt
        ? item.updatedAt
        : new Date().toISOString(),
  }
}
