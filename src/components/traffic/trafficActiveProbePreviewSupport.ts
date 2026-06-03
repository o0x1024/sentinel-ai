import type { ActiveProbeEntry } from './trafficActiveProbeTypes'

export interface ActiveProbeSelectionRange {
  from: number
  to: number
}

export interface ActiveProbeTargetHighlight {
  parameterName: string
  parameterPath: string
  location: 'query' | 'body'
  locationLabel: string
  probeValue: string
  selection: ActiveProbeSelectionRange | null
}

function buildLocationLabel(location: 'query' | 'body') {
  return location === 'query' ? 'Query 参数' : 'Body 参数'
}

function extractParameterName(parameterPath: string) {
  const normalized = parameterPath.trim()
  if (!normalized) {
    return ''
  }

  const withoutIndexes = normalized.replace(/\[\d+\]/g, '')
  const segments = withoutIndexes.split('.').filter(Boolean)
  return segments[segments.length - 1] || normalized
}

function findProbeSelection(rawRequest: string, probeValue: string): ActiveProbeSelectionRange | null {
  if (!rawRequest || !probeValue) {
    return null
  }

  const candidates = [
    probeValue,
    JSON.stringify(probeValue),
    encodeURIComponent(probeValue),
  ]

  for (const candidate of candidates) {
    if (!candidate) {
      continue
    }

    const index = rawRequest.indexOf(candidate)
    if (index >= 0) {
      return {
        from: index,
        to: index + candidate.length,
      }
    }
  }

  return null
}

export function resolveActiveProbeTargetHighlight(
  rawRequest: string,
  entry: ActiveProbeEntry | null,
): ActiveProbeTargetHighlight | null {
  if (!entry?.target_path || !entry.probe_value) {
    return null
  }

  const location = entry.target_location === 'query' ? 'query' : 'body'
  const parameterPath = entry.target_path

  return {
    parameterName: entry.target_name || extractParameterName(parameterPath),
    parameterPath,
    location,
    locationLabel: buildLocationLabel(location),
    probeValue: entry.probe_value,
    selection: findProbeSelection(rawRequest, entry.probe_value),
  }
}
