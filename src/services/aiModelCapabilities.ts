export type ModelVisionCapability = 'supported' | 'unsupported' | 'unknown'

export const inferModelSupportsVision = (provider: string, modelId: string): boolean => {
  const providerLower = provider.trim().toLowerCase()
  const modelLower = modelId.trim().toLowerCase()

  if (!modelLower || modelLower.includes('embedding')) {
    return false
  }

  const genericPatterns = [
    'vision',
    'multimodal',
    'gpt-4o',
    'gpt-4.1',
    'gpt-4-turbo',
    'claude-3',
    'claude-sonnet-4',
    'claude-opus-4',
    'gemini',
  ]
  if (genericPatterns.some((pattern) => modelLower.includes(pattern))) {
    return true
  }

  if (modelLower.includes('-vl') || modelLower.startsWith('deepseek-vl')) {
    return true
  }

  switch (providerLower) {
    case 'openai':
      return modelLower.includes('omni')
    case 'anthropic':
      return modelLower.startsWith('claude-3')
    case 'gemini':
    case 'google':
      return true
    case 'deepseek':
      return modelLower.includes('vl')
    case 'xai':
    case 'moonshot':
      return modelLower.includes('vision')
    case 'openrouter':
      return (
        modelLower.includes('vision') ||
        modelLower.includes('gpt-4o') ||
        modelLower.includes('claude-3') ||
        modelLower.includes('gemini') ||
        modelLower.includes('vl')
      )
    default:
      return false
  }
}

export const getModelVisionCapability = (
  provider: string,
  model: {
    id?: string | null
    name?: string | null
    supports_vision?: boolean | null
    vision_capability_status?: ModelVisionCapability | null
    vision_capability_source?: string | null
    vision_capability_evidence?: string | null
  } | null | undefined,
): ModelVisionCapability => {
  if (!model) return 'unknown'
  if (
    model.vision_capability_status === 'supported'
    || model.vision_capability_status === 'unsupported'
    || model.vision_capability_status === 'unknown'
  ) {
    return model.vision_capability_status
  }
  const explicit = typeof model.supports_vision === 'boolean' ? model.supports_vision : null
  const inferred = inferModelSupportsVision(
    provider,
    model.id?.trim() || model.name?.trim() || '',
  )
  if (explicit === true || inferred) {
    return 'supported'
  }
  if (explicit === false) {
    return 'unsupported'
  }
  return 'unknown'
}

export const getModelSupportsVision = (
  provider: string,
  model: {
    id?: string | null
    name?: string | null
    supports_vision?: boolean | null
  } | null | undefined,
): boolean => {
  return getModelVisionCapability(provider, model) === 'supported'
}
