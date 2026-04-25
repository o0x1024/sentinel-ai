import type { HttpExchangeRequest } from './http/model'
import type {
  TrafficComparerDraftRequestInput,
  TrafficComparePayload,
} from './transfers'
import type { TrafficContextCandidateEvidenceSelection } from './trafficContextCandidateTypes'

export interface TrafficAnalysisViewHandle {
  createDraftFromRequest(request: HttpExchangeRequest): Promise<void>
  createAttackWorkspaceFromRequest(request: HttpExchangeRequest): Promise<void>
  openCompare(payload: TrafficComparePayload): Promise<void>
  openDraftCompare(payload: TrafficComparerDraftRequestInput): Promise<void>
  openHistoryRequest(payload: TrafficContextCandidateEvidenceSelection): Promise<void>
}
