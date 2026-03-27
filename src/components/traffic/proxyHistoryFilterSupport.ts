import type {
  ProxyHistoryFilterCache,
  ProxyHistoryFilterConfig,
  ProxyRequest,
} from './proxyHistoryTypes'

export const buildDefaultProxyHistoryFilterConfig = (): ProxyHistoryFilterConfig => ({
  requestType: {
    showOnlyInScope: false,
    showOnlyWithParams: false,
    hideWithoutResponse: false,
  },
  mimeType: {
    html: true,
    script: true,
    xml: true,
    css: false,
    otherText: true,
    images: false,
    flash: true,
    otherBinary: false,
  },
  statusCode: {
    s2xx: true,
    s3xx: true,
    s4xx: true,
    s5xx: true,
  },
  search: {
    term: '',
    regex: false,
    caseSensitive: false,
    negative: false,
  },
  extension: {
    showOnlyEnabled: false,
    showOnly: '',
    hideEnabled: true,
    hide: 'js,gif,jpg,png,css,ico,woff,woff2,ttf,svg',
  },
  annotation: {
    showOnlyWithNotes: false,
    showOnlyHighlighted: false,
  },
  listener: {
    port: '',
  },
  bambdaExpression: '',
})

export const hasParams = (url: string): boolean => {
  try {
    const urlObj = new URL(url)
    return urlObj.search.length > 0
  } catch {
    return url.includes('?')
  }
}

export const getExtension = (url: string): string => {
  try {
    const urlObj = new URL(url)
    const pathname = urlObj.pathname
    const parts = pathname.split('.')
    if (parts.length > 1) {
      return parts[parts.length - 1].split(/[?#]/)[0].toLowerCase()
    }
    return ''
  } catch {
    return ''
  }
}

export const getMimeTypeCategory = (request: ProxyRequest): string => {
  let contentType = ''

  if (request.response_headers) {
    try {
      const headers = JSON.parse(request.response_headers)
      contentType = (headers['content-type'] || headers['Content-Type'] || '').toLowerCase()
    } catch {
      // ignore
    }
  }

  if (!contentType) {
    const ext = getExtension(request.url).toLowerCase()
    const extMap: Record<string, string> = {
      html: 'html',
      htm: 'html',
      js: 'script',
      mjs: 'script',
      xml: 'xml',
      css: 'css',
      png: 'image',
      jpg: 'image',
      jpeg: 'image',
      gif: 'image',
      webp: 'image',
      svg: 'image',
      ico: 'image',
      swf: 'flash',
      txt: 'text',
      json: 'text',
      woff: 'binary',
      woff2: 'binary',
      ttf: 'binary',
      eot: 'binary',
      pdf: 'binary',
      zip: 'binary',
    }
    return extMap[ext] || 'unknown'
  }

  if (contentType.includes('html')) return 'html'
  if (contentType.includes('javascript') || contentType.includes('ecmascript')) return 'script'
  if (contentType.includes('xml')) return 'xml'
  if (contentType.includes('css')) return 'css'
  if (contentType.includes('image')) return 'image'
  if (contentType.includes('flash') || contentType.includes('shockwave')) return 'flash'
  if (contentType.includes('text') || contentType.includes('json')) return 'text'
  if (
    contentType.includes('octet-stream') ||
    contentType.includes('binary') ||
    contentType.includes('font') ||
    contentType.includes('application')
  ) {
    return 'binary'
  }

  return 'unknown'
}

export const buildProxyHistoryFilterCache = (
  config: ProxyHistoryFilterConfig,
): ProxyHistoryFilterCache => {
  let searchRegex: RegExp | null = null
  if (config.search.term && config.search.regex) {
    try {
      searchRegex = new RegExp(config.search.term, config.search.caseSensitive ? '' : 'i')
    } catch {
      searchRegex = null
    }
  }

  let showExts: Set<string> | null = null
  if (config.extension.showOnlyEnabled && config.extension.showOnly) {
    showExts = new Set(config.extension.showOnly.toLowerCase().split(',').map((ext) => ext.trim()))
  }

  let hideExts: Set<string> | null = null
  if (config.extension.hideEnabled && config.extension.hide) {
    hideExts = new Set(config.extension.hide.toLowerCase().split(',').map((ext) => ext.trim()))
  }

  return {
    config,
    searchRegex,
    searchTerm: config.search.term
      ? (config.search.caseSensitive ? config.search.term : config.search.term.toLowerCase())
      : '',
    showExts,
    hideExts,
  }
}

export const filterProxyRequests = (
  requests: ProxyRequest[],
  config: ProxyHistoryFilterConfig,
  cache: ProxyHistoryFilterCache,
) =>
  requests.filter((request) => {
    if (config.requestType.showOnlyWithParams && !hasParams(request.url)) {
      return false
    }
    if (config.requestType.hideWithoutResponse && request.status_code === 0) {
      return false
    }

    const code = request.status_code
    if (code !== 0) {
      if (code >= 200 && code < 300 && !config.statusCode.s2xx) return false
      if (code >= 300 && code < 400 && !config.statusCode.s3xx) return false
      if (code >= 400 && code < 500 && !config.statusCode.s4xx) return false
      if (code >= 500 && code < 600 && !config.statusCode.s5xx) return false
    }

    const mime = getMimeTypeCategory(request)
    if (mime === 'html' && !config.mimeType.html) return false
    if (mime === 'script' && !config.mimeType.script) return false
    if (mime === 'xml' && !config.mimeType.xml) return false
    if (mime === 'css' && !config.mimeType.css) return false
    if (mime === 'image' && !config.mimeType.images) return false
    if (mime === 'flash' && !config.mimeType.flash) return false
    if (mime === 'text' && !config.mimeType.otherText) return false
    if (mime === 'binary' && !config.mimeType.otherBinary) return false

    const ext = getExtension(request.url).toLowerCase()
    if (cache.showExts && ext && !cache.showExts.has(ext)) {
      return false
    }
    if (cache.hideExts && ext && cache.hideExts.has(ext)) {
      return false
    }

    if (cache.searchTerm) {
      const text = `${request.url} ${request.host}`
      let match = false

      if (cache.searchRegex) {
        match = cache.searchRegex.test(text)
      } else if (config.search.caseSensitive) {
        match = text.includes(cache.searchTerm)
      } else {
        match = text.toLowerCase().includes(cache.searchTerm)
      }

      if (config.search.negative ? match : !match) {
        return false
      }
    }

    if (config.listener.port) {
      const listenerPort = request.listener || ''
      if (!listenerPort.includes(config.listener.port)) {
        return false
      }
    }

    return true
  })

export const hasActiveProxyHistoryFilters = (config: ProxyHistoryFilterConfig) => {
  const def = buildDefaultProxyHistoryFilterConfig()
  return (
    config.requestType.showOnlyWithParams !== def.requestType.showOnlyWithParams ||
    config.requestType.hideWithoutResponse !== def.requestType.hideWithoutResponse ||
    !config.statusCode.s2xx ||
    !config.statusCode.s3xx ||
    !config.statusCode.s4xx ||
    !config.statusCode.s5xx ||
    !config.mimeType.html ||
    !config.mimeType.script ||
    !config.mimeType.xml ||
    config.mimeType.css ||
    config.mimeType.images ||
    !config.mimeType.otherText ||
    config.mimeType.otherBinary ||
    config.search.term !== '' ||
    config.extension.showOnlyEnabled ||
    config.listener.port !== ''
  )
}

export const buildProxyHistoryFilterSummary = (config: ProxyHistoryFilterConfig) => {
  const parts: string[] = []
  const hiddenMime: string[] = []

  if (!config.mimeType.css) hiddenMime.push('CSS')
  if (!config.mimeType.images) hiddenMime.push('image')
  if (!config.mimeType.otherBinary) hiddenMime.push('binary')
  if (hiddenMime.length > 0) {
    parts.push(`Hiding ${hiddenMime.join(', ')}`)
  }

  if (config.extension.hideEnabled && config.extension.hide) {
    parts.push('hiding extensions')
  }
  if (config.search.term) {
    parts.push(`search: "${config.search.term}"`)
  }

  if (parts.length === 0) {
    return 'Filter settings: Showing all content'
  }
  return `Filter settings: ${parts.join(' and ')}`
}

export const showAllProxyHistoryFilters = (config: ProxyHistoryFilterConfig): ProxyHistoryFilterConfig => ({
  ...config,
  mimeType: {
    html: true,
    script: true,
    xml: true,
    css: true,
    otherText: true,
    images: true,
    flash: true,
    otherBinary: true,
  },
  statusCode: {
    s2xx: true,
    s3xx: true,
    s4xx: true,
    s5xx: true,
  },
  extension: {
    ...config.extension,
    showOnlyEnabled: false,
    hideEnabled: false,
  },
  requestType: {
    ...config.requestType,
    showOnlyInScope: false,
    showOnlyWithParams: false,
    hideWithoutResponse: false,
  },
  annotation: {
    ...config.annotation,
    showOnlyWithNotes: false,
    showOnlyHighlighted: false,
  },
})

export const hideAllProxyHistoryFilters = (config: ProxyHistoryFilterConfig): ProxyHistoryFilterConfig => ({
  ...config,
  mimeType: {
    html: false,
    script: false,
    xml: false,
    css: false,
    otherText: false,
    images: false,
    flash: false,
    otherBinary: false,
  },
})

export const convertProxyHistoryFilterConfigToBambda = (config: ProxyHistoryFilterConfig) => {
  const conditions: string[] = []
  const enabledMimes = Object.entries(config.mimeType)
    .filter(([, enabled]) => enabled)
    .map(([type]) => type)

  if (enabledMimes.length > 0 && enabledMimes.length < 8) {
    const mimeConditions = enabledMimes
      .map((type) => {
        const mimeMap: Record<string, string> = {
          html: 'text/html',
          script: 'javascript',
          xml: 'xml',
          css: 'text/css',
          otherText: 'text/',
          images: 'image/',
          flash: 'flash',
          otherBinary: 'application/octet-stream',
        }
        return `response.mimeType().contains('${mimeMap[type]}')`
      })
      .join(' || ')
    conditions.push(`(${mimeConditions})`)
  }

  const statusCodes: string[] = []
  if (config.statusCode.s2xx) statusCodes.push('response.statusCode() >= 200 && response.statusCode() < 300')
  if (config.statusCode.s3xx) statusCodes.push('response.statusCode() >= 300 && response.statusCode() < 400')
  if (config.statusCode.s4xx) statusCodes.push('response.statusCode() >= 400 && response.statusCode() < 500')
  if (config.statusCode.s5xx) statusCodes.push('response.statusCode() >= 500 && response.statusCode() < 600')

  if (statusCodes.length > 0 && statusCodes.length < 4) {
    conditions.push(`(${statusCodes.join(' || ')})`)
  }

  if (config.search.term) {
    const target = config.search.caseSensitive ? 'request.url()' : 'request.url().toLowerCase()'
    const method = config.search.regex ? 'matches' : 'contains'
    const searchTerm = config.search.caseSensitive ? config.search.term : config.search.term.toLowerCase()
    const condition = `${target}.${method}('${searchTerm}')`
    conditions.push(config.search.negative ? `!${condition}` : condition)
  }

  if (config.extension.hideEnabled && config.extension.hide) {
    const exts = config.extension.hide.split(',').map((ext) => ext.trim())
    const extConditions = exts.map((ext) => `!request.url().endsWith('.${ext}')`).join(' && ')
    conditions.push(`(${extConditions})`)
  }

  return conditions.length > 0 ? conditions.join(' && ') : '// No filters configured'
}

export const mergeStoredProxyHistoryFilterConfig = (saved: unknown): ProxyHistoryFilterConfig => ({
  ...buildDefaultProxyHistoryFilterConfig(),
  ...(saved as Partial<ProxyHistoryFilterConfig>),
})
