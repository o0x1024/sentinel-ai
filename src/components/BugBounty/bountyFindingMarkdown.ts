export function formatBountyFindingMarkdownList(jsonValue?: string | null) {
  if (!jsonValue) {
    return ''
  }

  const parsed = JSON.parse(jsonValue)
  if (!Array.isArray(parsed)) {
    return ''
  }

  return parsed
    .filter(item => typeof item === 'string' && item.trim())
    .join('\n\n')
}

