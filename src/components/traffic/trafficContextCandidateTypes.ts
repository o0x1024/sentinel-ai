import type { TrafficContextExtractionSettings } from './proxyConfigurationTypes'

export type TrafficContextCandidateCategory =
  | 'principal'
  | 'resource'
  | 'auth_header'
  | 'auth_token'
  | 'cookie_hint'
  | 'action_alias'

export interface TrafficContextDictionaryCandidate {
  key: string
  normalizedKey: string
  category: TrafficContextCandidateCategory
  suggestedCanonicalAction?: string | null
  confidence: 'low' | 'medium' | 'high'
  score: number
  evidenceCount: number
  distinctValueCount: number
  sources: string[]
  exampleValues: string[]
  exampleLocations: string[]
  ruleReasons: string[]
  aiReason?: string | null
  behaviorReason?: string | null
  behaviorEvidence: string[]
  evidenceRequests: TrafficContextCandidateEvidenceRequest[]
  conflictWithExisting: boolean
  alreadyCoveredBy?: string | null
}

export interface TrafficContextCandidateEvidenceRequest {
  requestId: number
  method: string
  url: string
  matchedLocations: string[]
}

export interface TrafficContextCandidateEvidenceSelection {
  requestId: number
  pane?: 'request' | 'response'
  matchedLocations: string[]
  searchTerms?: string[]
}

export type TrafficContextPreviewKind =
  | 'principal'
  | 'resource'
  | 'authHeader'
  | 'authToken'
  | 'cookie'
  | 'action'

export interface TrafficContextPreviewFocus {
  requestId: number | null
  kind: TrafficContextPreviewKind | null
  updatedAt: number | null
  previewVersion: string | null
}

export interface RecommendTrafficContextDictionaryCandidatesResponse {
  candidates: TrafficContextDictionaryCandidate[]
  analyzedRequestCount: number
  skippedRequestCount: number
  existingSettings: TrafficContextExtractionSettings
}

export interface TrafficContextExtractionPreviewSample {
  requestId: number
  method: string
  url: string
  addedPrincipalKeys: string[]
  addedResourceKeys: string[]
  addedAuthHeaders: string[]
  addedAuthTokens: string[]
  addedCookieKeys: string[]
  actionKindBefore?: string | null
  actionKindAfter?: string | null
}

export interface TrafficContextExtractionPreviewResponse {
  analyzedRequestCount: number
  changedRequestCount: number
  principalMatchDeltaCount: number
  resourceMatchDeltaCount: number
  authHeaderMatchDeltaCount: number
  authTokenMatchDeltaCount: number
  cookieMatchDeltaCount: number
  actionKindChangeCount: number
  samples: TrafficContextExtractionPreviewSample[]
}
