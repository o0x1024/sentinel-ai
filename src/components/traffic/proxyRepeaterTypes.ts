import type { HttpExchangeRequest, HttpReplayResponse } from './http/model'

export interface ReplayCommandResponse<T> {
  success: boolean
  data?: T
  error?: string
}

export type RepeaterRequestTab = 'pretty' | 'raw' | 'hex'
export type RepeaterResponseTab = 'pretty' | 'raw' | 'hex' | 'render'

export interface RepeaterTab {
  id: string
  name: string
  sourceRequestId: number | null
  targetHost: string
  targetPort: number
  useTls: boolean
  overrideSni: boolean
  sniHost: string
  initialRawRequest: string
  rawRequest: string
  prettyRequest: string
  lastCompletedRawResponse: string
  previousRawResponse: string
  rawResponse: string
  requestTab: RepeaterRequestTab
  responseTab: RepeaterResponseTab
  response: HttpReplayResponse | null
  isSending: boolean
  modified: boolean
}

export interface RepeaterSavedTab {
  id: string
  name: string
  sourceRequestId: number | null
  targetHost: string
  targetPort: number
  useTls: boolean
  overrideSni: boolean
  sniHost: string
  rawRequest: string
  requestTab: RepeaterRequestTab
  responseTab: RepeaterResponseTab
  initialRawRequest?: string
}

export interface RepeaterSavedState {
  tabs: RepeaterSavedTab[]
  activeTabIndex: number
}

export interface RepeaterCompareLabels {
  defaultName: string
  requestVersions: string
  responseVersions: string
  originalRequest: string
  currentRequest: string
  previousResponse: string
  currentResponse: string
}

export interface RepeaterTabCreationOptions {
  request?: HttpExchangeRequest
  fallbackNameIndex: number
  generateId: () => string
  defaultRequestTab: RepeaterRequestTab
  defaultResponseTab: RepeaterResponseTab
}
