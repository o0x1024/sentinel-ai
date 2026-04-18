import type { HttpExchangeRequest } from './http/model'
import type {
  TrafficComparerDraftRequestInput,
  TrafficComparePayload,
} from './transfers'
import type { TrafficContextCandidateEvidenceSelection } from './trafficContextCandidateTypes'

export interface TrafficAnalysisViewHandle {
  sendToRepeater(request: HttpExchangeRequest): void
  sendToIntruder(request: HttpExchangeRequest): void
  sendToComparer(payload: TrafficComparePayload): void
  sendDraftRequestToComparer(payload: TrafficComparerDraftRequestInput): void
  openHistoryRequest(payload: TrafficContextCandidateEvidenceSelection): Promise<void>
}
