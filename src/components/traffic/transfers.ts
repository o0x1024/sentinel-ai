export interface TrafficTransferRequest {
  method: string
  url: string
  headers: Record<string, string>
  body?: string
}

interface TransferEnvelope<T> {
  id: string
  payload: T
}

export const REPEATER_TRANSFER_STORAGE_KEY = 'trafficAnalysis.transfer.repeater'
export const COMPARER_TRANSFER_STORAGE_KEY = 'trafficAnalysis.transfer.comparer'

export interface TrafficComparePayload {
  name: string
  leftLabel: string
  rightLabel: string
  leftText: string
  rightText: string
}

export function queueRepeaterTransfer(request: TrafficTransferRequest): void {
  const envelope: TransferEnvelope<TrafficTransferRequest> = {
    id: `transfer-${Date.now()}-${Math.random().toString(36).slice(2, 10)}`,
    payload: request,
  }
  localStorage.setItem(REPEATER_TRANSFER_STORAGE_KEY, JSON.stringify(envelope))
}

export function queueComparerTransfer(payload: TrafficComparePayload): void {
  const envelope: TransferEnvelope<TrafficComparePayload> = {
    id: `transfer-${Date.now()}-${Math.random().toString(36).slice(2, 10)}`,
    payload,
  }
  localStorage.setItem(COMPARER_TRANSFER_STORAGE_KEY, JSON.stringify(envelope))
}

export function parseTransferEnvelope<T>(raw: string | null): TransferEnvelope<T> | null {
  if (!raw) return null

  try {
    return JSON.parse(raw) as TransferEnvelope<T>
  } catch {
    return null
  }
}
