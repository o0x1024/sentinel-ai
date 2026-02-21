<template>
  <div class="h-full flex flex-col">
    <div class="flex items-center justify-between p-4 border-b border-base-300">
      <div class="flex items-center gap-2">
        <i class="fas fa-shield-halved text-warning"></i>
        <h3 class="text-base font-semibold">{{ t('agent.auditFindings') }}</h3>
      </div>
      <button class="btn btn-sm btn-ghost btn-circle" @click="$emit('close')">
        <i class="fas fa-times"></i>
      </button>
    </div>

    <div class="p-4 overflow-y-auto flex-1 space-y-3">
      <div
        v-if="policyGate"
        class="rounded-lg border p-3 text-sm"
        :class="policyGate.passed ? 'border-success/30 bg-success/10 text-success' : 'border-error/30 bg-error/10 text-error'"
      >
        <div class="font-semibold">
          {{ policyGate.passed ? t('agent.policyGatePassed') : t('agent.policyGateBlocked') }}
        </div>
        <div class="text-xs mt-1 opacity-90">
          {{ policyGate.reason || (policyGate.passed ? t('agent.policyGateDefaultPass') : t('agent.policyGateDefaultBlock')) }}
        </div>
      </div>

      <div v-if="findings.length === 0" class="text-sm text-base-content/60">
        {{ t('agent.noAuditFindings') }}
      </div>

      <div
        v-for="item in findings"
        :key="item.id"
        class="border border-base-300 rounded-lg p-3 bg-base-100"
      >
        <div class="flex items-center justify-between gap-2">
          <div class="font-medium text-sm truncate" :title="item.title">{{ item.title || item.id }}</div>
          <span class="badge badge-sm" :class="severityClass(item.severity)">{{ item.severity || 'unknown' }}</span>
        </div>

        <p v-if="item.fix" class="text-xs text-base-content/70 mt-2">
          {{ t('agent.recommendedFix') }}: {{ item.fix }}
        </p>

        <div v-if="item.files && item.files.length > 0" class="mt-2 space-y-1">
          <div class="text-xs text-base-content/60">{{ t('agent.affectedFiles') }}:</div>
          <div v-for="f in item.files.slice(0, 5)" :key="f" class="text-xs font-mono text-base-content/80 break-all">
            {{ f }}
          </div>
        </div>

        <details v-if="buildHitRows(item).length > 0" class="mt-3 bg-base-200/40 rounded-md border border-base-300">
          <summary class="cursor-pointer px-3 py-2 text-xs font-medium text-base-content/80">
            {{ t('agent.hitDetails') }} ({{ buildHitRows(item).length }} {{ t('agent.times') }})
          </summary>
          <div class="p-3 pt-2 overflow-auto">
            <table class="table table-xs w-full">
              <thead>
                <tr>
                  <th>#</th>
                  <th>{{ t('agent.sourceLabel') }}</th>
                  <th>{{ t('agent.sinkLabel') }}</th>
                </tr>
              </thead>
              <tbody>
                <tr v-for="row in buildHitRows(item)" :key="row.id">
                  <td class="align-top text-xs text-base-content/60">{{ row.index }}</td>
                  <td class="align-top">
                    <div class="text-xs font-mono break-all">{{ row.source.location }}</div>
                    <div v-if="row.source.code" class="text-xs text-base-content/70 mt-1 break-all">{{ row.source.code }}</div>
                  </td>
                  <td class="align-top">
                    <div class="text-xs font-mono break-all">{{ row.sink.location }}</div>
                    <div v-if="row.sink.code" class="text-xs text-base-content/70 mt-1 break-all">{{ row.sink.code }}</div>
                  </td>
                </tr>
              </tbody>
            </table>
          </div>
        </details>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'

interface AuditFinding {
  id: string
  title?: string
  severity?: string
  fix?: string
  files?: string[]
  source?: Record<string, any>
  sink?: Record<string, any>
  hits?: Array<Record<string, any>>
  sources?: Array<Record<string, any>>
  sinks?: Array<Record<string, any>>
  source_sinks?: Array<Record<string, any>>
  trace_path?: Array<Record<string, any>>
}

interface PolicyGateResult {
  passed: boolean
  reason?: string
}

defineProps<{
  findings: AuditFinding[]
  policyGate?: PolicyGateResult | null
}>()

defineEmits<{
  (e: 'close'): void
}>()

const { t } = useI18n()

const severityClass = (severity?: string) => {
  switch ((severity || '').toLowerCase()) {
    case 'critical':
      return 'badge-error'
    case 'high':
      return 'badge-warning'
    case 'medium':
      return 'badge-info'
    case 'low':
      return 'badge-success'
    default:
      return 'badge-ghost'
  }
}

interface HitCell {
  location: string
  code?: string
}

interface HitRow {
  id: string
  index: number
  source: HitCell
  sink: HitCell
}

const pickLocation = (node?: Record<string, any>): string => {
  if (!node || typeof node !== 'object') return '-'
  const file = node.file || node.path || node.filename || node.source_file || '-'
  const line = node.line || node.line_number || node.start_line
  return line ? `${String(file)}:${String(line)}` : String(file)
}

const pickCode = (node?: Record<string, any>): string | undefined => {
  if (!node || typeof node !== 'object') return undefined
  const raw = node.code || node.snippet || node.text || node.content
  return raw ? String(raw) : undefined
}

const normalizeHitRow = (source: Record<string, any> | undefined, sink: Record<string, any> | undefined, index: number): HitRow => ({
  id: `${pickLocation(source)}->${pickLocation(sink)}#${index}`,
  index,
  source: {
    location: pickLocation(source),
    code: pickCode(source),
  },
  sink: {
    location: pickLocation(sink),
    code: pickCode(sink),
  },
})

const buildHitRows = (item: AuditFinding): HitRow[] => {
  const rows: HitRow[] = []

  const pairs = Array.isArray(item.source_sinks) ? item.source_sinks : []
  for (const pair of pairs) {
    rows.push(normalizeHitRow(pair?.source, pair?.sink, rows.length + 1))
  }

  const hits = Array.isArray(item.hits) ? item.hits : []
  for (const hit of hits) {
    const source = hit?.source || hit?.from
    const sink = hit?.sink || hit?.to
    rows.push(normalizeHitRow(source, sink, rows.length + 1))
  }

  const sources = Array.isArray(item.sources) ? item.sources : []
  const sinks = Array.isArray(item.sinks) ? item.sinks : []
  if (sources.length > 0 && sinks.length > 0) {
    const maxLen = Math.max(sources.length, sinks.length)
    for (let i = 0; i < maxLen; i += 1) {
      rows.push(normalizeHitRow(sources[i], sinks[i], rows.length + 1))
    }
  }

  if (item.source || item.sink) {
    rows.push(normalizeHitRow(item.source, item.sink, rows.length + 1))
  }

  if (Array.isArray(item.trace_path) && item.trace_path.length >= 2) {
    const traceSource = item.trace_path[0]
    const traceSink = item.trace_path[item.trace_path.length - 1]
    rows.push(normalizeHitRow(traceSource, traceSink, rows.length + 1))
  }

  const dedup = new Map<string, HitRow>()
  for (const row of rows) {
    const key = `${row.source.location}|${row.source.code || ''}|${row.sink.location}|${row.sink.code || ''}`
    if (!dedup.has(key)) {
      dedup.set(key, row)
    }
  }

  return Array.from(dedup.values()).map((row, idx) => ({ ...row, index: idx + 1 }))
}
</script>
