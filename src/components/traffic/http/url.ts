import type { HttpEndpoint, HttpScheme } from './model'

export function getDefaultPort(scheme: HttpScheme): number {
  return scheme === 'https' ? 443 : 80
}

export function buildHostHeaderValue(endpoint: HttpEndpoint): string {
  const defaultPort = getDefaultPort(endpoint.scheme)
  return endpoint.port === defaultPort ? endpoint.host : `${endpoint.host}:${endpoint.port}`
}

export function buildAbsoluteUrl(endpoint: HttpEndpoint, target: string): string {
  if (target.startsWith('http://') || target.startsWith('https://')) {
    return target
  }

  const normalizedTarget = target.startsWith('/') ? target : `/${target}`
  const defaultPort = getDefaultPort(endpoint.scheme)
  const portSuffix = endpoint.port === defaultPort ? '' : `:${endpoint.port}`
  return `${endpoint.scheme}://${endpoint.host}${portSuffix}${normalizedTarget}`
}

export function endpointFromUrl(url: string): HttpEndpoint {
  const parsed = new URL(url)
  const scheme = parsed.protocol === 'http:' ? 'http' : 'https'
  const port = parsed.port ? Number.parseInt(parsed.port, 10) : getDefaultPort(scheme)

  return {
    scheme,
    host: parsed.hostname,
    port,
  }
}
