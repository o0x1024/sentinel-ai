export const HEX_VIEW_CHUNK_SIZE = 16 * 1024
export const CHUNKED_HEX_VIEW_THRESHOLD_BYTES = 64 * 1024

function estimateTextBytes(value: string): number {
  if (!value) {
    return 0
  }

  return new Blob([value]).size
}

export function shouldChunkHexView(value: string): boolean {
  return estimateTextBytes(value) >= CHUNKED_HEX_VIEW_THRESHOLD_BYTES
}

export function convertTextToHexChunk(
  value: string,
  start: number,
  end: number,
): string {
  let hex = ''
  for (let index = start; index < end; index += 1) {
    hex += `${value.charCodeAt(index).toString(16).padStart(2, '0')} `
    if ((index + 1) % 16 === 0) {
      hex += '\n'
    }
  }
  return hex
}
