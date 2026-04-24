import type {
  ProxyHistoryFilterCache,
  ProxyHistoryFilterConfig,
  ProxyRequest,
} from './proxyHistoryTypes'
import type { ProxyScopeRule } from './proxyConfigurationTypes'
import { getProxyHistoryDerived } from './proxyHistoryDerivedSupport'
import { isProxyRequestInScope } from './proxyScopeSupport'

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
  return getProxyHistoryDerived(request).mimeTypeCategory
}

export const buildProxyHistoryFilterCache = (
  config: ProxyHistoryFilterConfig,
  scopeRules: { includeRules: ProxyScopeRule[]; excludeRules: ProxyScopeRule[] } | null = null,
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
    scopeRules,
  }
}

export const filterProxyRequests = (
  requests: ProxyRequest[],
  config: ProxyHistoryFilterConfig,
  cache: ProxyHistoryFilterCache,
) => {
  const filtered: ProxyRequest[] = []

  requests.forEach((request) => {
    if (matchesProxyHistoryRequest(request, config, cache)) {
      filtered.push(request)
    }
  })
  return filtered
}

export const matchesProxyHistoryRequest = (
  request: ProxyRequest,
  config: ProxyHistoryFilterConfig,
  cache: ProxyHistoryFilterCache,
) => {
  const derived = getProxyHistoryDerived(request)

  if (
    config.requestType.showOnlyInScope
    && (
      !cache.scopeRules
      || !isProxyRequestInScope(
        request,
        cache.scopeRules.includeRules,
        cache.scopeRules.excludeRules,
      )
    )
  ) {
    return false
  }

  if (config.requestType.showOnlyWithParams && !derived.hasParams) {
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

  const mime = derived.mimeTypeCategory
  if (mime === 'html' && !config.mimeType.html) return false
  if (mime === 'script' && !config.mimeType.script) return false
  if (mime === 'xml' && !config.mimeType.xml) return false
  if (mime === 'css' && !config.mimeType.css) return false
  if (mime === 'image' && !config.mimeType.images) return false
  if (mime === 'flash' && !config.mimeType.flash) return false
  if (mime === 'text' && !config.mimeType.otherText) return false
  if (mime === 'binary' && !config.mimeType.otherBinary) return false

  const ext = derived.extension
  if (cache.showExts && ext && !cache.showExts.has(ext)) {
    return false
  }
  if (cache.hideExts && ext && cache.hideExts.has(ext)) {
    return false
  }

  if (cache.searchTerm) {
    let match = false

    if (cache.searchRegex) {
      match = cache.searchRegex.test(derived.searchText)
    } else if (config.search.caseSensitive) {
      match = derived.searchText.includes(cache.searchTerm)
    } else {
      match = derived.searchTextLower.includes(cache.searchTerm)
    }

    if (config.search.negative ? match : !match) {
      return false
    }
  }

  if (config.listener.port && !derived.listenerValue.includes(config.listener.port)) {
    return false
  }

  return true
}

export const hasActiveProxyHistoryFilters = (config: ProxyHistoryFilterConfig) => {
  return (
    config.requestType.showOnlyInScope ||
    config.requestType.showOnlyWithParams ||
    config.requestType.hideWithoutResponse ||
    !config.statusCode.s2xx ||
    !config.statusCode.s3xx ||
    !config.statusCode.s4xx ||
    !config.statusCode.s5xx ||
    !config.mimeType.html ||
    !config.mimeType.script ||
    !config.mimeType.xml ||
    !config.mimeType.css ||
    !config.mimeType.images ||
    !config.mimeType.flash ||
    !config.mimeType.otherText ||
    !config.mimeType.otherBinary ||
    config.search.term !== '' ||
    (config.extension.showOnlyEnabled && config.extension.showOnly.trim() !== '') ||
    (config.extension.hideEnabled && config.extension.hide.trim() !== '') ||
    config.annotation.showOnlyWithNotes ||
    config.annotation.showOnlyHighlighted ||
    config.listener.port !== '' ||
    config.bambdaExpression.trim() !== ''
  )
}

export const buildProxyHistoryFilterSummary = (config: ProxyHistoryFilterConfig) => {
  const parts: string[] = []
  const hiddenContent: string[] = []

  if (!config.mimeType.html) hiddenContent.push('HTML')
  if (!config.mimeType.otherText) hiddenContent.push('text')
  if (!config.mimeType.script) hiddenContent.push('script')
  if (!config.mimeType.css) hiddenContent.push('CSS')
  if (!config.mimeType.images) hiddenContent.push('image')
  if (!config.mimeType.xml) hiddenContent.push('XML')
  if (!config.mimeType.flash) hiddenContent.push('Flash')
  if (!config.mimeType.otherBinary) hiddenContent.push('binary')
  if (hiddenContent.length > 0) {
    parts.push(`Hiding ${hiddenContent.join(' and ')} content`)
  }

  if (config.extension.showOnlyEnabled && config.extension.showOnly.trim()) {
    parts.push('showing only specific extensions')
  }
  if (config.extension.hideEnabled && config.extension.hide.trim()) {
    parts.push('hiding specific extensions')
  }
  if (config.requestType.showOnlyInScope) {
    parts.push('showing only in-scope items')
  }
  if (config.requestType.showOnlyWithParams) {
    parts.push('showing only requests with parameters')
  }
  if (config.requestType.hideWithoutResponse) {
    parts.push('hiding items without responses')
  }
  if (!config.statusCode.s2xx || !config.statusCode.s3xx || !config.statusCode.s4xx || !config.statusCode.s5xx) {
    parts.push('filtering specific status codes')
  }
  if (config.search.term) {
    parts.push(`searching for "${config.search.term}"`)
  }
  if (config.annotation.showOnlyWithNotes) {
    parts.push('showing only items with notes')
  }
  if (config.annotation.showOnlyHighlighted) {
    parts.push('showing only highlighted items')
  }
  if (config.listener.port) {
    parts.push(`matching listener port "${config.listener.port}"`)
  }
  if (config.bambdaExpression.trim()) {
    parts.push('using a custom Bambda expression')
  }

  if (parts.length === 0) {
    return 'Showing all content'
  }
  return parts.join('; ')
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
  search: {
    term: '',
    regex: false,
    caseSensitive: false,
    negative: false,
  },
  listener: {
    port: '',
  },
  bambdaExpression: '',
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
