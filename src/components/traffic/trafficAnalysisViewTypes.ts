import type { HttpExchangeRequest } from './http/model'
import type {
  TrafficComparerDraftRequestInput,
  TrafficComparePayload,
} from './transfers'
import type { TrafficContextCandidateEvidenceSelection } from './trafficContextCandidateTypes'

export interface TrafficAnalysisViewHandle {
  createDraftFromRequest(request: HttpExchangeRequest): void
  createAttackWorkspaceFromRequest(request: HttpExchangeRequest): void
  openCompare(payload: TrafficComparePayload): void
  openDraftCompare(payload: TrafficComparerDraftRequestInput): void
  openHistoryRequest(payload: TrafficContextCandidateEvidenceSelection): Promise<void>
}
