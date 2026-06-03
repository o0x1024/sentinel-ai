export const TRAFFIC_MESSAGE_READER_PROGRESSIVE_THRESHOLD_BYTES = 128 * 1024
export const TRAFFIC_MESSAGE_READER_PROGRESSIVE_CHUNK_SIZE = 32 * 1024

function estimateTextBytes(value: string): number {
  if (!value) {
    return 0
  }

  return new Blob([value]).size
}

export function shouldProgressivelyRenderTrafficMessage(value: string): boolean {
  return estimateTextBytes(value) >= TRAFFIC_MESSAGE_READER_PROGRESSIVE_THRESHOLD_BYTES
}
