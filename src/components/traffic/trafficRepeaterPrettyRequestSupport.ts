function normalizeLineEndings(value: string): string {
  return value.replace(/\r\n/g, '\n').replace(/\r/g, '\n')
}

function splitRequestSections(value: string): { headerPart: string; bodyPart: string | null } {
  const normalized = normalizeLineEndings(value)
  const separatorIndex = normalized.indexOf('\n\n')

  if (separatorIndex === -1) {
    return {
      headerPart: normalized,
      bodyPart: null,
    }
  }

  return {
    headerPart: normalized.slice(0, separatorIndex),
    bodyPart: normalized.slice(separatorIndex + 2),
  }
}

export function formatRepeaterPrettyRequest(rawRequest: string): string {
  if (!rawRequest) return ''

  const { headerPart, bodyPart } = splitRequestSections(rawRequest)
  if (bodyPart == null) return headerPart

  let result = `${headerPart}\n\n`
  if (!bodyPart.trim()) return result
  if (bodyPart.includes('\n')) {
    result += bodyPart
    return result
  }

  try {
    const json = JSON.parse(bodyPart.trim())
    result += JSON.stringify(json, null, 2)
  } catch {
    result += bodyPart
  }

  return result
}

export function convertRepeaterPrettyRequestToRaw(prettyRequest: string): string {
  const normalized = normalizeLineEndings(prettyRequest)
  const lines = normalized.split('\n')
  const headerEnd = lines.findIndex((line) => line.trim() === '')

  if (headerEnd === -1) {
    return lines.join('\r\n')
  }

  const headerPart = lines.slice(0, headerEnd).join('\r\n')
  const bodyPart = lines.slice(headerEnd + 1).join('\r\n')
  return `${headerPart}\r\n\r\n${bodyPart}`
}
