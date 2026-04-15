export type HttpScheme = 'http' | 'https'
export type HttpVersion = 'HTTP/1.0' | 'HTTP/1.1' | 'HTTP/2' | 'AUTO'

export interface HttpHeaderEntry {
  name: string
  value: string
}

export interface HttpEndpoint {
  scheme: HttpScheme
  host: string
  port: number
  sniHost?: string
}

export interface HttpRequestDraft {
  method: string
  target: string
  versionPreference: HttpVersion
  headers: HttpHeaderEntry[]
  bodyText: string
}

export interface HttpExchangeRequest {
  endpoint: HttpEndpoint
  absoluteUrl: string
  request: HttpRequestDraft
}

export interface HttpReplayResponse {
  statusCode: number
  versionObserved?: Exclude<HttpVersion, 'AUTO'>
  statusText?: string
  headers: HttpHeaderEntry[]
  bodyText: string
  rawText: string
  responseTimeMs: number
}
