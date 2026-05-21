import type { AdvancedTestResult } from './types'

export interface PluginResultDownloadFile {
  archivePath: string
  sourcePath: string
  mimeType: string
  size: number
  bytes: Uint8Array
}

interface RawPluginResultFile {
  archivePath?: unknown
  downloadPath?: unknown
  filename?: unknown
  sourcePath?: unknown
  mimeType?: unknown
  content?: unknown
  contentBase64?: unknown
}

const encoder = new TextEncoder()
const crcTable = new Uint32Array(256)

for (let index = 0; index < 256; index += 1) {
  let value = index
  for (let bit = 0; bit < 8; bit += 1) {
    value = (value & 1) ? (0xedb88320 ^ (value >>> 1)) : (value >>> 1)
  }
  crcTable[index] = value >>> 0
}

function crc32(bytes: Uint8Array): number {
  let crc = 0xffffffff
  for (const byte of bytes) {
    crc = crcTable[(crc ^ byte) & 0xff] ^ (crc >>> 8)
  }
  return (crc ^ 0xffffffff) >>> 0
}

function writeUint16(view: DataView, offset: number, value: number) {
  view.setUint16(offset, value, true)
}

function writeUint32(view: DataView, offset: number, value: number) {
  view.setUint32(offset, value >>> 0, true)
}

function bytesFromBase64(value: string): Uint8Array | null {
  try {
    const binary = atob(value)
    const bytes = new Uint8Array(binary.length)
    for (let index = 0; index < binary.length; index += 1) {
      bytes[index] = binary.charCodeAt(index)
    }
    return bytes
  } catch {
    return null
  }
}

function sanitizeArchivePath(value: unknown, fallback: string): string {
  const raw = typeof value === 'string' && value.trim() ? value.trim() : fallback
  const normalized = raw
    .replace(/\\/g, '/')
    .split('/')
    .filter(segment => segment && segment !== '.' && segment !== '..')
    .join('/')
    .replace(/[<>:"|*]/g, '_')

  return normalized || fallback
}

function bytesFromRawFile(file: RawPluginResultFile): Uint8Array | null {
  if (typeof file.contentBase64 === 'string' && file.contentBase64.trim()) {
    return bytesFromBase64(file.contentBase64.trim())
  }

  if (typeof file.content === 'string') {
    return encoder.encode(file.content)
  }

  return null
}

function collectFilesFromOutput(output: unknown): RawPluginResultFile[] {
  if (typeof output === 'string') {
    try {
      return collectFilesFromOutput(JSON.parse(output))
    } catch {
      return []
    }
  }

  if (!output || typeof output !== 'object') {
    return []
  }

  const record = output as Record<string, any>
  const candidates = [
    record.files,
    record.data?.files,
    record.output?.files,
    record.output?.data?.files,
  ]

  return candidates
    .filter(Array.isArray)
    .flatMap(files => files)
    .filter(file => file && typeof file === 'object')
}

export function collectDownloadablePluginFiles(result: AdvancedTestResult | null): PluginResultDownloadFile[] {
  if (!result) return []

  const rawFiles = [
    ...(result.outputs || []).flatMap(output => collectFilesFromOutput(output)),
    ...(result.runs || []).flatMap(run => collectFilesFromOutput(run.output)),
  ]

  const seen = new Set<string>()
  const files: PluginResultDownloadFile[] = []

  rawFiles.forEach((rawFile, index) => {
    const bytes = bytesFromRawFile(rawFile)
    if (!bytes) return

    const sourcePath = typeof rawFile.sourcePath === 'string' ? rawFile.sourcePath : ''
    const fallbackName = typeof rawFile.filename === 'string' && rawFile.filename.trim()
      ? rawFile.filename.trim()
      : `plugin-result-file-${index + 1}.txt`
    const archivePath = sanitizeArchivePath(
      rawFile.archivePath || rawFile.downloadPath || rawFile.filename,
      fallbackName,
    )

    if (seen.has(archivePath)) return
    seen.add(archivePath)

    files.push({
      archivePath,
      sourcePath,
      mimeType: typeof rawFile.mimeType === 'string' ? rawFile.mimeType : 'application/octet-stream',
      size: bytes.byteLength,
      bytes,
    })
  })

  return files
}

function dosDateTime(date: Date) {
  const year = Math.max(date.getFullYear(), 1980)
  const dosTime = (date.getHours() << 11) | (date.getMinutes() << 5) | Math.floor(date.getSeconds() / 2)
  const dosDate = ((year - 1980) << 9) | ((date.getMonth() + 1) << 5) | date.getDate()
  return { dosDate, dosTime }
}

export function buildStoredZipBlob(files: PluginResultDownloadFile[]): Blob {
  const now = dosDateTime(new Date())
  const chunks: BlobPart[] = []
  const centralDirectory: BlobPart[] = []
  let offset = 0

  for (const file of files) {
    const nameBytes = encoder.encode(file.archivePath)
    const checksum = crc32(file.bytes)
    const localHeader = new Uint8Array(30)
    const localView = new DataView(localHeader.buffer)
    writeUint32(localView, 0, 0x04034b50)
    writeUint16(localView, 4, 20)
    writeUint16(localView, 6, 0x0800)
    writeUint16(localView, 8, 0)
    writeUint16(localView, 10, now.dosTime)
    writeUint16(localView, 12, now.dosDate)
    writeUint32(localView, 14, checksum)
    writeUint32(localView, 18, file.bytes.byteLength)
    writeUint32(localView, 22, file.bytes.byteLength)
    writeUint16(localView, 26, nameBytes.byteLength)
    writeUint16(localView, 28, 0)

    chunks.push(localHeader, nameBytes, file.bytes)

    const centralHeader = new Uint8Array(46)
    const centralView = new DataView(centralHeader.buffer)
    writeUint32(centralView, 0, 0x02014b50)
    writeUint16(centralView, 4, 20)
    writeUint16(centralView, 6, 20)
    writeUint16(centralView, 8, 0x0800)
    writeUint16(centralView, 10, 0)
    writeUint16(centralView, 12, now.dosTime)
    writeUint16(centralView, 14, now.dosDate)
    writeUint32(centralView, 16, checksum)
    writeUint32(centralView, 20, file.bytes.byteLength)
    writeUint32(centralView, 24, file.bytes.byteLength)
    writeUint16(centralView, 28, nameBytes.byteLength)
    writeUint16(centralView, 30, 0)
    writeUint16(centralView, 32, 0)
    writeUint16(centralView, 34, 0)
    writeUint16(centralView, 36, 0)
    writeUint32(centralView, 38, 0)
    writeUint32(centralView, 42, offset)

    centralDirectory.push(centralHeader, nameBytes)
    offset += localHeader.byteLength + nameBytes.byteLength + file.bytes.byteLength
  }

  const centralDirectorySize = centralDirectory.reduce(
    (sum, chunk) => sum + (chunk as Uint8Array).byteLength,
    0,
  )

  const endHeader = new Uint8Array(22)
  const endView = new DataView(endHeader.buffer)
  writeUint32(endView, 0, 0x06054b50)
  writeUint16(endView, 4, 0)
  writeUint16(endView, 6, 0)
  writeUint16(endView, 8, files.length)
  writeUint16(endView, 10, files.length)
  writeUint32(endView, 12, centralDirectorySize)
  writeUint32(endView, 16, offset)
  writeUint16(endView, 20, 0)

  return new Blob([...chunks, ...centralDirectory, endHeader], { type: 'application/zip' })
}

export function makePluginResultArchiveName(pluginId: string): string {
  const normalizedPluginId = pluginId.replace(/[^a-zA-Z0-9._-]+/g, '_') || 'plugin-result'
  const stamp = new Date().toISOString().replace(/[:.]/g, '-')
  return `${normalizedPluginId}-files-${stamp}.zip`
}

export function downloadPluginResultFiles(result: AdvancedTestResult, pluginId: string): number {
  const files = collectDownloadablePluginFiles(result)
  if (files.length === 0) return 0

  const blob = buildStoredZipBlob(files)
  const url = URL.createObjectURL(blob)
  const anchor = document.createElement('a')
  anchor.href = url
  anchor.download = makePluginResultArchiveName(pluginId)
  anchor.click()
  window.setTimeout(() => URL.revokeObjectURL(url), 0)
  return files.length
}
