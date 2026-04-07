import type { IntruderRequestProcessorTrace } from './plugins'
import type { IntruderPayloadSourceSummaryEntry } from './intruderPayloadSourceSummary'
import {
  buildRequestPreviewDiff,
  buildRequestPreviewTraceDiffs,
  type RequestPreviewChangeKind,
  type RequestPreviewDiff,
} from './requestPreview'

interface BuildRequestPreviewDebugReportOptions {
  payloadSummary: string
  payloadSources?: IntruderPayloadSourceSummaryEntry[]
  originalRequest: string
  finalRequest: string
  traces: IntruderRequestProcessorTrace[]
}

function formatStatus(changed: boolean): string {
  return changed ? 'changed' : 'unchanged'
}

function formatChangeKind(kind: RequestPreviewChangeKind): string {
  return kind.toUpperCase()
}

function formatValue(value: string): string {
  return value || '(empty)'
}

function appendParameterSection(lines: string[], title: string, diff: RequestPreviewDiff['queryParameterDiff']) {
  lines.push(`${title}: ${diff.changes.length} changed, ${diff.unchangedCount} unchanged`)

  if (!diff.changes.length) {
    return
  }

  for (const change of diff.changes) {
    lines.push(`- [${formatChangeKind(change.kind)}] ${change.label}: ${formatValue(change.before)} -> ${formatValue(change.after)}`)
  }
}

function appendHeaderSection(lines: string[], diff: RequestPreviewDiff) {
  lines.push(`Header changes: ${diff.headerChanges.length} changed, ${diff.unchangedHeaderCount} unchanged`)

  if (!diff.headerChanges.length) {
    return
  }

  for (const change of diff.headerChanges) {
    lines.push(`- [${formatChangeKind(change.kind)}] ${change.label}`)
    lines.push(`  before: ${formatValue(change.before)}`)
    lines.push(`  after: ${formatValue(change.after)}`)
  }
}

function appendDiffSummary(lines: string[], title: string, diff: RequestPreviewDiff) {
  lines.push(title)
  lines.push(`- Request line: ${formatStatus(diff.requestLineChanged)}`)
  lines.push(`- Body: ${formatStatus(diff.bodyChanged)}`)
  appendHeaderSection(lines, diff)
  appendParameterSection(lines, 'Query parameters', diff.queryParameterDiff)
  appendParameterSection(lines, 'Form parameters', diff.formParameterDiff)

  if (diff.requestLineChanged) {
    lines.push(`Request line before: ${formatValue(diff.requestLineBefore)}`)
    lines.push(`Request line after: ${formatValue(diff.requestLineAfter)}`)
  }

  if (diff.bodyChanged) {
    lines.push('Body before:')
    lines.push(diff.bodyBefore || '(empty)')
    lines.push('Body after:')
    lines.push(diff.bodyAfter || '(empty)')
  }
}

function formatOutput(output: unknown): string {
  if (output == null) {
    return '(none)'
  }

  if (typeof output === 'string') {
    return output
  }

  try {
    return JSON.stringify(output, null, 2)
  } catch {
    return String(output)
  }
}

function formatPayloadSource(source: IntruderPayloadSourceSummaryEntry['sources'][number]): string {
  if (source.type === 'default_dictionary') {
    return `Default dictionary (${source.dictType || 'unknown'})`
  }

  if (source.dictionaryName) {
    return `${source.dictionaryName} (${source.dictionaryId || 'unknown'})`
  }

  return source.dictionaryId || 'unknown'
}

export function buildRequestPreviewDebugReport(
  options: BuildRequestPreviewDebugReportOptions,
): string {
  const { payloadSummary, payloadSources = [], originalRequest, finalRequest, traces } = options
  const lines: string[] = []
  const overallDiff = buildRequestPreviewDiff(originalRequest, finalRequest)
  const traceDiffs = buildRequestPreviewTraceDiffs(
    originalRequest,
    traces.map((trace) => trace.requestText),
  )

  lines.push('Intruder Request Preview Debug Report')
  lines.push(`Payload summary: ${payloadSummary || '(empty)'}`)
  if (payloadSources.length > 0) {
    lines.push('Payload sources:')
    payloadSources.forEach((entry) => {
      lines.push(`- ${entry.payloadSetName}: ${entry.sources.map(formatPayloadSource).join(', ')}`)
    })
  }
  lines.push('')

  appendDiffSummary(lines, 'Overall diff', overallDiff)
  lines.push('')
  lines.push('Original request:')
  lines.push(originalRequest || '(empty)')
  lines.push('')
  lines.push('Final request:')
  lines.push(finalRequest || '(empty)')

  if (!traces.length) {
    return lines.join('\n')
  }

  lines.push('')
  lines.push('Processor trace')

  traces.forEach((trace, index) => {
    const traceDiff = traceDiffs[index]?.diff ?? buildRequestPreviewDiff(index === 0 ? originalRequest : traces[index - 1].requestText, trace.requestText)
    lines.push('')
    lines.push(`${index + 1}. ${trace.pluginId}`)
    appendDiffSummary(lines, 'Processor diff', traceDiff)
    lines.push('Request after processor:')
    lines.push(trace.requestText || '(empty)')
    lines.push('Raw output:')
    lines.push(formatOutput(trace.output))
  })

  return lines.join('\n')
}
