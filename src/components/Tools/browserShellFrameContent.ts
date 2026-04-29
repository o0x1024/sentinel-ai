import type { BrowserShellFrame } from '@/api/browserShell'

const utf8Decoder = new TextDecoder()

function normalizeTerminalText(raw: string) {
  return raw
    .replace(/\r\n/g, '\n')
    .replace(/\r/g, '\n')
    .replace(/\0/g, '')
}

function decodeBase64Payload(payloadBase64: string) {
  const binary = window.atob(payloadBase64)
  const bytes = new Uint8Array(binary.length)
  for (let index = 0; index < binary.length; index += 1) {
    bytes[index] = binary.charCodeAt(index)
  }
  return utf8Decoder.decode(bytes)
}

export function decodeBrowserShellFrameText(frame: BrowserShellFrame) {
  const payloadBase64 = frame.payloadBase64?.trim()
  if (!payloadBase64) return null

  try {
    return normalizeTerminalText(decodeBase64Payload(payloadBase64))
  } catch {
    return null
  }
}

export function resolveBrowserShellFrameContent(
  frame: BrowserShellFrame,
  preferFullPayload: boolean,
) {
  const decodedPayload = decodeBrowserShellFrameText(frame)
  if (preferFullPayload && decodedPayload && decodedPayload.trim().length > 0) {
    return decodedPayload
  }

  const preview = frame.textPreview?.trim()
  if (preview) {
    return frame.textPreview || ''
  }

  if (decodedPayload && decodedPayload.trim().length > 0) {
    return decodedPayload
  }

  if (frame.frameType === 'binary') {
    return '[binary frame]'
  }
  return '[empty frame]'
}

export function browserShellFrameHasFullPayload(frame: BrowserShellFrame) {
  return Boolean(frame.payloadBase64?.trim())
}
