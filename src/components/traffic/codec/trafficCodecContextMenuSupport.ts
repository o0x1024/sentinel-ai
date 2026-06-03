import type { TrafficContextSubmenu } from '../trafficContextSubmenuSupport'
import { buildTrafficContextSubmenu } from '../trafficContextSubmenuSupport'
import type { TrafficCodecRule, CodecRequestMeta } from './trafficCodecTypes'

export function buildTrafficCodecContextSubmenu(options: {
  matchedRules: TrafficCodecRule[]
  codecViewEnabled: boolean
  onToggleRule: (ruleId: string) => void
  onToggleView: () => void
  onCreateRule: () => void
  onManageRules: () => void
}): TrafficContextSubmenu | null {
  const items = [
    ...options.matchedRules.map(rule => ({
      key: `codec-rule-${rule.id}`,
      iconClass: rule.enabled ? 'fas fa-toggle-on text-success' : 'fas fa-toggle-off text-base-content/50',
      labelKey: rule.name,
      onClick: () => options.onToggleRule(rule.id),
    })),
    {
      key: 'codec-toggle-view',
      iconClass: options.codecViewEnabled ? 'fas fa-eye text-info' : 'fas fa-eye-slash',
      labelKey: options.codecViewEnabled ? 'codecViewOn' : 'codecViewOff',
      onClick: options.onToggleView,
    },
    {
      key: 'codec-create-rule',
      iconClass: 'fas fa-plus text-success',
      labelKey: 'createCodecRule',
      onClick: options.onCreateRule,
    },
    {
      key: 'codec-manage-rules',
      iconClass: 'fas fa-cog text-base-content/70',
      labelKey: 'manageCodecRules',
      onClick: options.onManageRules,
    },
  ]

  return buildTrafficContextSubmenu({
    key: 'transparent-codec',
    triggerLabelKey: 'transparentCodec',
    triggerIconClass: 'fas fa-key text-warning',
    items,
  })
}

export function extractCodecMetaFromUrl(url: string, method: string, headers: Record<string, string>): CodecRequestMeta {
  let host = ''
  let path = ''
  try {
    const parsed = new URL(url)
    host = parsed.hostname
    path = parsed.pathname
  } catch { /* ignore */ }
  return {
    host,
    path,
    method,
    contentType: headers['content-type'] ?? headers['Content-Type'] ?? '',
  }
}

export function extractCodecMetaFromRawRequest(rawRequest: string, host: string): CodecRequestMeta {
  const firstLine = rawRequest.split(/\r\n|\r|\n/)[0]?.trim() || ''
  const methodMatch = firstLine.match(/^(\w+)\s+(\S+)/)
  const method = methodMatch?.[1] || 'GET'
  let path = methodMatch?.[2] || '/'

  if (path.startsWith('http://') || path.startsWith('https://')) {
    try {
      const parsed = new URL(path)
      host = parsed.hostname || host
      path = parsed.pathname
    } catch { /* ignore */ }
  } else if (!path.startsWith('/')) {
    path = `/${path}`
  }

  const headers: Record<string, string> = {}
  const lines = rawRequest.split(/\r\n|\r|\n/)
  for (let i = 1; i < lines.length; i++) {
    const line = lines[i]
    if (line.trim() === '') break
    const colonIdx = line.indexOf(':')
    if (colonIdx > 0) {
      headers[line.slice(0, colonIdx).trim()] = line.slice(colonIdx + 1).trim()
    }
  }

  const url = host ? `http://${host}${path}` : path
  return extractCodecMetaFromUrl(url, method, headers)
}
