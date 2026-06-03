const VISION_MODEL_UNSUPPORTED_PATTERNS = [
  'does not support image understanding',
  'switch to a vision-capable model in the conversation work config',
]

export const buildVisionModelUnsupportedError = (provider?: string | null, model?: string | null): string => {
  const providerName = typeof provider === 'string' ? provider.trim() : ''
  const modelName = typeof model === 'string' ? model.trim() : ''
  const target = providerName && modelName
    ? `${providerName}/${modelName}`
    : providerName || modelName || 'unknown/unknown'
  return `Current model does not support image understanding: ${target}. Switch to a vision-capable model in the conversation work config, or explicitly change image handling to local OCR.`
}

export const isVisionModelUnsupportedError = (message?: string | null): boolean => {
  const normalized = typeof message === 'string' ? message.trim().toLowerCase() : ''
  if (!normalized) return false
  return VISION_MODEL_UNSUPPORTED_PATTERNS.every((pattern) => normalized.includes(pattern))
}
