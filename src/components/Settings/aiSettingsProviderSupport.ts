export const getEnabledProviders = (aiConfig: any) => {
  if (!aiConfig?.providers) {
    return []
  }

  return Object.keys(aiConfig.providers).filter((providerKey) => {
    const provider = aiConfig.providers[providerKey]
    return provider && provider.enabled === true
  })
}

export const getProviderModels = (aiConfig: any, providerKey: string) => {
  if (!providerKey || !aiConfig?.providers) {
    return []
  }

  const provider = Object.keys(aiConfig.providers).find((key) => key.toLowerCase() === providerKey.toLowerCase())
  if (!provider) {
    return []
  }
  return aiConfig.providers[provider]?.models || []
}

export const getProviderIcon = (provider: string) => {
  const icons: Record<string, string> = {
    OpenAI: 'fas fa-brain',
    'Azure OpenAI': 'fab fa-microsoft',
    Anthropic: 'fas fa-robot',
    Google: 'fab fa-google',
    Gemini: 'fab fa-google',
    'Google Gemini': 'fab fa-google',
    Ollama: 'fas fa-server',
    DeepSeek: 'fas fa-eye',
    EternalAI: 'fas fa-link',
    Galadriel: 'fas fa-hat-wizard',
    Moonshot: 'fas fa-moon',
    Mira: 'fas fa-compass',
    OpenRouter: 'fas fa-route',
    ModelScope: 'fas fa-cog',
    Groq: 'fas fa-bolt',
    Perplexity: 'fas fa-search',
    TogetherAI: 'fas fa-users',
    xAI: 'fas fa-atom',
    Cohere: 'fas fa-comments',
    Hyperbolic: 'fas fa-infinity',
  }
  return icons[provider] || 'fas fa-cog'
}

export const getProviderName = (provider: string) => {
  const names: Record<string, string> = {
    OpenAI: 'OpenAI',
    'Azure OpenAI': 'Azure OpenAI',
    Anthropic: 'Anthropic',
    Google: 'Google',
    Gemini: 'Gemini',
    'Google Gemini': 'Google Gemini',
    Ollama: 'Ollama',
    DeepSeek: 'DeepSeek',
    EternalAI: 'EternalAI',
    Galadriel: 'Galadriel',
    Moonshot: 'Moonshot',
    Mira: 'Mira',
    OpenRouter: 'OpenRouter',
    ModelScope: 'ModelScope',
    Groq: 'Groq',
    Perplexity: 'Perplexity',
    TogetherAI: 'TogetherAI',
    xAI: 'xAI',
    Cohere: 'Cohere',
    Hyperbolic: 'Hyperbolic',
  }
  return names[provider] || provider
}

export const rigProviderOptions = [
  { value: 'anthropic', label: 'Anthropic', description: 'Claude 系列模型' },
  { value: 'openai', label: 'OpenAI', description: 'OpenAI 及兼容 API' },
  { value: 'azure', label: 'Azure OpenAI', description: 'Azure 托管 OpenAI 服务' },
  { value: 'cohere', label: 'Cohere', description: 'Cohere 模型' },
  { value: 'deepseek', label: 'DeepSeek', description: 'DeepSeek 模型' },
  { value: 'eternalai', label: 'EternalAI', description: 'EternalAI 模型' },
  { value: 'gemini', label: 'Google Gemini', description: 'Google Gemini 模型' },
  { value: 'galadriel', label: 'Galadriel', description: 'Galadriel 模型' },
  { value: 'groq', label: 'Groq', description: 'Groq 高速推理' },
  { value: 'hyperbolic', label: 'Hyperbolic', description: 'Hyperbolic 模型' },
  { value: 'mira', label: 'Mira', description: 'Mira 模型' },
  { value: 'moonshot', label: 'Moonshot', description: 'Moonshot Kimi 模型' },
  { value: 'ollama', label: 'Ollama', description: '本地模型服务' },
  { value: 'openrouter', label: 'OpenRouter', description: '多模型路由服务' },
  { value: 'perplexity', label: 'Perplexity', description: 'Perplexity 搜索增强' },
  { value: 'togetherai', label: 'TogetherAI', description: '开源模型托管' },
  { value: 'xai', label: 'xAI', description: 'xAI Grok 模型' },
]

export const needsApiKey = (provider: string) => !['Ollama'].includes(provider)
