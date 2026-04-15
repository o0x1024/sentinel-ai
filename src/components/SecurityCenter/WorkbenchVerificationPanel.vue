<template>
  <div class="space-y-4">
    <div
      v-if="assessmentSuggestion"
      class="rounded-lg border border-base-300 bg-base-100 p-4"
    >
      <div class="flex flex-wrap items-center justify-between gap-2">
        <div>
          <p class="text-sm font-semibold">{{ wb('verification.suggestionTitle') }}</p>
          <p class="text-xs text-base-content/60">{{ assessmentSuggestion.summary }}</p>
        </div>
        <div class="flex flex-wrap gap-2">
          <span :class="['badge', getWorkbenchConfidenceBadgeClass(assessmentSuggestion.confidence)]">
            {{ wb('overview.confidence', { value: getWorkbenchConfidenceLabel(assessmentSuggestion.confidence) }) }}
          </span>
          <span :class="['badge', getWorkbenchStatusBadgeClass(assessmentSuggestion.suggestedStatus)]">
            {{ wb('overview.suggestedStatus', { value: getWorkbenchStatusLabel(assessmentSuggestion.suggestedStatus) }) }}
          </span>
        </div>
      </div>
    </div>

    <div class="rounded-lg border border-base-300 bg-base-100 p-4">
      <div class="mb-3 flex flex-wrap items-center justify-between gap-2">
        <p class="text-sm font-semibold">{{ wb('verification.verifierRuns') }}</p>
        <span class="badge badge-outline">{{ wb('verification.count', { count: verifierRuns.length }) }}</span>
      </div>

      <div v-if="verifierRuns.length > 0" class="space-y-3">
        <div
          v-for="run in verifierRuns"
          :key="run.id"
          :class="[
            'rounded-lg p-3 space-y-2',
            selectedRunId === run.evidenceId ? 'bg-primary/10 ring-1 ring-primary/30' : 'bg-base-200/70',
          ]"
        >
          <div class="flex flex-wrap items-center justify-between gap-2">
            <div class="flex flex-wrap items-center gap-2">
              <span :class="['badge', getVerifierRunBadgeClass(run)]">
                {{ getVerifierRunStatusLabel(run) }}
              </span>
              <span v-if="run.strategy" class="badge badge-outline">
                {{ wb('verification.strategyLabel', { value: run.strategy }) }}
              </span>
              <span v-if="run.triggerEvent" class="badge badge-ghost">
                {{ wb('verification.triggerEventLabel', { value: run.triggerEvent }) }}
              </span>
              <span
                v-if="typeof run.responseStatus === 'number'"
                :class="['badge badge-sm', run.responseStatus >= 400 ? 'badge-error' : 'badge-success']"
              >
                {{ run.responseStatus }}
              </span>
            </div>
            <span class="text-xs text-base-content/60">{{ formatWorkbenchTime(run.finishedAt || run.startedAt) }}</span>
          </div>
          <p class="text-sm text-base-content/70">{{ run.summary }}</p>
          <p v-if="run.errorMessage" class="text-xs text-error">{{ run.errorMessage }}</p>
        </div>
      </div>

      <p v-else class="text-sm text-base-content/60">
        {{ wb('verification.emptyVerifierRuns') }}
      </p>
    </div>

    <div class="rounded-lg border border-base-300 bg-base-100 p-4">
      <div class="mb-3 flex flex-wrap items-center justify-between gap-2">
        <p class="text-sm font-semibold">{{ wb('verification.executionRecords') }}</p>
        <span class="badge badge-outline">{{ wb('verification.count', { count: executionRuns.length }) }}</span>
      </div>

      <div v-if="executionRuns.length > 0" class="space-y-3">
        <div
          v-for="run in executionRuns"
          :key="run.id"
          :ref="setRunRef(run.id)"
          :class="[
            'rounded-lg p-3 space-y-3',
            selectedRunId === run.id ? 'bg-primary/10 ring-1 ring-primary/30' : 'bg-base-200/70',
          ]"
        >
          <div class="flex flex-wrap items-center justify-between gap-2">
            <div class="flex flex-wrap items-center gap-2">
              <span
                :class="[
                  'badge',
                  run.status === 'completed' ? 'badge-success' : run.status === 'blocked' ? 'badge-warning' : 'badge-error',
                ]"
              >
                {{ getRunStatusLabel(run.status) }}
              </span>
              <span class="badge badge-outline">{{ run.method }}</span>
              <span class="badge badge-ghost">{{ run.targetField }}</span>
              <span v-if="selectedRunId === run.id" class="badge badge-secondary">{{ wb('verification.focused') }}</span>
            </div>
            <span class="text-xs text-base-content/60">{{ formatWorkbenchTime(run.finishedAt) }}</span>
          </div>

          <p class="text-sm text-base-content/70">{{ run.summary }}</p>
          <p class="font-mono text-xs break-all text-base-content/60">{{ run.baselineUrl }}</p>

          <div class="space-y-2">
            <div
              v-for="attempt in run.attempts"
              :key="attempt.id"
              class="rounded-lg bg-base-100 p-3"
            >
              <div class="flex flex-wrap items-center justify-between gap-2">
                <div class="flex flex-wrap items-center gap-2">
                  <span :class="['badge badge-sm', getAttemptOutcomeBadgeClass(attempt.outcome)]">
                    {{ getAttemptOutcomeLabel(attempt.outcome) }}
                  </span>
                  <span class="badge badge-primary badge-sm">{{ attempt.candidateValue }}</span>
                  <span
                    v-if="typeof attempt.responseStatus === 'number'"
                    :class="['badge badge-sm', attempt.responseStatus >= 400 ? 'badge-error' : 'badge-success']"
                  >
                    {{ attempt.responseStatus }}
                  </span>
                </div>
              </div>
              <div class="mt-2 flex flex-wrap gap-2">
                <span :class="['badge badge-sm', attempt.diff.matchedStatus ? 'badge-outline' : 'badge-warning']">
                  {{ attempt.diff.matchedStatus ? wb('verification.statusMatched') : wb('verification.statusChanged') }}
                </span>
                <span :class="['badge badge-sm', attempt.diff.matchedBody ? 'badge-outline' : 'badge-info']">
                  {{ attempt.diff.matchedBody ? wb('verification.bodyMatched') : wb('verification.bodyChanged') }}
                </span>
                <span class="badge badge-sm badge-ghost">
                  {{ wb('verification.similarity', { value: getSimilarityLevelLabel(attempt.diff.similarityLevel) }) }}
                </span>
                <span class="badge badge-sm badge-ghost">
                  {{ wb('verification.length', { baseline: attempt.diff.baselineLength, response: attempt.diff.responseLength }) }}
                </span>
              </div>
              <p class="mt-2 font-mono text-xs break-all text-base-content/70">{{ attempt.mutatedUrl }}</p>
              <div v-if="attempt.diff.changedSignals.length > 0" class="mt-2 flex flex-wrap gap-2">
                <span
                  v-for="signal in attempt.diff.changedSignals"
                  :key="signal"
                  class="badge badge-outline badge-sm"
                >
                  {{ signal }}
                </span>
              </div>
              <pre
                v-if="attempt.responseSnippet"
                class="mt-2 bg-base-200 p-3 rounded text-xs whitespace-pre-wrap break-words overflow-x-auto"
              >{{ attempt.responseSnippet }}</pre>
            </div>
          </div>
        </div>
      </div>

      <p v-else class="text-sm text-base-content/60">
        {{ wb('verification.emptyRuns') }}
      </p>
    </div>

    <div class="rounded-lg border border-base-300 bg-base-100 p-4">
      <p class="text-sm font-semibold mb-3">{{ wb('verification.currentSummary') }}</p>
      <div v-if="verificationEvidence.length > 0" class="space-y-3">
        <div
          v-for="evidence in verificationEvidence"
          :key="evidence.id"
          class="rounded-lg bg-base-200/70 p-3"
        >
          <div class="flex flex-wrap items-center justify-between gap-2">
            <div class="flex flex-wrap items-center gap-2">
              <span class="badge badge-success">{{ wb('verification.verificationRecord') }}</span>
              <span
                v-if="typeof evidence.response_status === 'number'"
                :class="['badge badge-sm', evidence.response_status >= 400 ? 'badge-error' : 'badge-success']"
              >
                {{ evidence.response_status }}
              </span>
            </div>
            <span class="text-xs text-base-content/60">{{ formatWorkbenchTime(evidence.timestamp) }}</span>
          </div>
          <pre class="mt-3 bg-base-100 p-3 rounded text-xs whitespace-pre-wrap break-words overflow-x-auto">{{
            getWorkbenchEvidenceSnippet(evidence)
          }}</pre>
          <div class="mt-3 grid gap-3 xl:grid-cols-2">
            <div
              v-if="getBaselineExchange(evidence)"
              class="rounded-lg border border-base-300 bg-base-100 p-3"
            >
              <p class="text-xs font-semibold text-base-content/70 mb-2">{{ wb('verification.baselineExchange') }}</p>
              <div class="space-y-2">
                <div class="flex flex-wrap items-center gap-2">
                  <span class="badge badge-outline">{{ getBaselineExchange(evidence)?.requestMethod || 'GET' }}</span>
                  <span
                    v-if="typeof getBaselineExchange(evidence)?.responseStatus === 'number'"
                    :class="['badge badge-sm', (getBaselineExchange(evidence)?.responseStatus || 0) >= 400 ? 'badge-error' : 'badge-success']"
                  >
                    {{ getBaselineExchange(evidence)?.responseStatus }}
                  </span>
                </div>
                <pre class="bg-base-200 p-3 rounded text-xs overflow-x-auto whitespace-pre-wrap break-words">{{ getBaselineExchange(evidence)?.requestUrl }}</pre>
                <div v-if="getBaselineExchange(evidence)?.requestHeaders">
                  <p class="text-xs text-base-content/60">{{ wb('verification.requestHeaders') }}</p>
                  <pre class="bg-base-200 p-3 rounded text-xs overflow-x-auto max-h-56 whitespace-pre-wrap break-words">{{ formatExchangePayload(getBaselineExchange(evidence)?.requestHeaders) }}</pre>
                </div>
                <div v-if="getBaselineExchange(evidence)?.requestBody">
                  <p class="text-xs text-base-content/60">{{ wb('verification.requestBody') }}</p>
                  <pre class="bg-base-200 p-3 rounded text-xs overflow-x-auto max-h-56 whitespace-pre-wrap break-words">{{ getBaselineExchange(evidence)?.requestBody }}</pre>
                </div>
                <div v-if="getBaselineExchange(evidence)?.responseHeaders">
                  <p class="text-xs text-base-content/60">{{ wb('verification.responseHeaders') }}</p>
                  <pre class="bg-base-200 p-3 rounded text-xs overflow-x-auto max-h-56 whitespace-pre-wrap break-words">{{ formatExchangePayload(getBaselineExchange(evidence)?.responseHeaders) }}</pre>
                </div>
                <div v-if="getBaselineExchange(evidence)?.responseBody">
                  <p class="text-xs text-base-content/60">{{ wb('verification.responseBody') }}</p>
                  <pre class="bg-base-200 p-3 rounded text-xs overflow-x-auto max-h-56 whitespace-pre-wrap break-words">{{ getBaselineExchange(evidence)?.responseBody }}</pre>
                </div>
              </div>
            </div>
            <div
              v-if="getReplayExchange(evidence)"
              class="rounded-lg border border-base-300 bg-base-100 p-3"
            >
              <p class="text-xs font-semibold text-base-content/70 mb-2">{{ wb('verification.replayExchange') }}</p>
              <div class="space-y-2">
                <div class="flex flex-wrap items-center gap-2">
                  <span class="badge badge-outline">{{ getReplayExchange(evidence)?.requestMethod || 'GET' }}</span>
                  <span
                    v-if="typeof getReplayExchange(evidence)?.responseStatus === 'number'"
                    :class="['badge badge-sm', (getReplayExchange(evidence)?.responseStatus || 0) >= 400 ? 'badge-error' : 'badge-success']"
                  >
                    {{ getReplayExchange(evidence)?.responseStatus }}
                  </span>
                </div>
                <pre class="bg-base-200 p-3 rounded text-xs overflow-x-auto whitespace-pre-wrap break-words">{{ getReplayExchange(evidence)?.requestUrl }}</pre>
                <div v-if="getReplayExchange(evidence)?.requestHeaders">
                  <p class="text-xs text-base-content/60">{{ wb('verification.requestHeaders') }}</p>
                  <pre class="bg-base-200 p-3 rounded text-xs overflow-x-auto max-h-56 whitespace-pre-wrap break-words">{{ formatExchangePayload(getReplayExchange(evidence)?.requestHeaders) }}</pre>
                </div>
                <div v-if="getReplayExchange(evidence)?.requestBody">
                  <p class="text-xs text-base-content/60">{{ wb('verification.requestBody') }}</p>
                  <pre class="bg-base-200 p-3 rounded text-xs overflow-x-auto max-h-56 whitespace-pre-wrap break-words">{{ getReplayExchange(evidence)?.requestBody }}</pre>
                </div>
                <div v-if="getReplayExchange(evidence)?.responseHeaders">
                  <p class="text-xs text-base-content/60">{{ wb('verification.responseHeaders') }}</p>
                  <pre class="bg-base-200 p-3 rounded text-xs overflow-x-auto max-h-56 whitespace-pre-wrap break-words">{{ formatExchangePayload(getReplayExchange(evidence)?.responseHeaders) }}</pre>
                </div>
                <div v-if="getReplayExchange(evidence)?.responseBody">
                  <p class="text-xs text-base-content/60">{{ wb('verification.responseBody') }}</p>
                  <pre class="bg-base-200 p-3 rounded text-xs overflow-x-auto max-h-56 whitespace-pre-wrap break-words">{{ getReplayExchange(evidence)?.responseBody }}</pre>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
      <p v-else class="text-sm text-base-content/60">
        {{ wb('verification.emptyVerification') }}
      </p>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, ref, watch } from 'vue'
import type {
  WorkbenchAssessmentSuggestion,
  WorkbenchCase,
  WorkbenchExecutionAttemptOutcome,
  WorkbenchExecutionSimilarityLevel,
  WorkbenchExecutionRun,
  WorkbenchExecutionRunStatus,
  WorkbenchVerifierRun,
} from './securityWorkbenchTypes'
import {
  formatWorkbenchTime,
  getWorkbenchConfidenceBadgeClass,
  getWorkbenchConfidenceLabel,
  getWorkbenchStatusBadgeClass,
  getWorkbenchStatusLabel,
} from './securityWorkbenchPresentation'
import { wb } from './securityWorkbenchLocale'
import {
  formatWorkbenchRawPayload,
  getWorkbenchEvidenceSnippet,
  getWorkbenchVerificationBaselineExchange,
  getWorkbenchVerificationReplayExchange,
} from './securityWorkbenchSystemAgentContent'
import type { Evidence } from './vulnerabilityFindingTypes'
import type { WorkbenchEvidenceExchange } from './securityWorkbenchSystemAgentContent'

const props = defineProps<{
  caseItem: WorkbenchCase
  executionRuns: WorkbenchExecutionRun[]
  verifierRuns: WorkbenchVerifierRun[]
  assessmentSuggestion: WorkbenchAssessmentSuggestion | null
  selectedRunId?: string | null
}>()

const verificationEvidence = computed(() =>
  (props.caseItem.finding.evidence || []).filter(item => item.location === 'system_agent_verification'),
)
const assessmentSuggestion = computed(() => props.assessmentSuggestion)
const runRefs = ref<Record<string, HTMLElement | null>>({})

const setRunRef = (runId: string) => (element: Element | null) => {
  runRefs.value[runId] = element instanceof HTMLElement ? element : null
}

watch(
  () => props.selectedRunId,
  async (runId) => {
    if (!runId) return
    await nextTick()
    runRefs.value[runId]?.scrollIntoView({
      behavior: 'smooth',
      block: 'center',
    })
  },
  { immediate: true },
)

const getRunStatusLabel = (status: WorkbenchExecutionRunStatus) => {
  switch (status) {
    case 'completed':
      return wb('verification.runStatus.completed')
    case 'blocked':
      return wb('verification.runStatus.blocked')
    default:
      return wb('verification.runStatus.failed')
  }
}

const getVerifierRunStatusLabel = (run: WorkbenchVerifierRun) => {
  if (run.verified) return wb('verification.verifierVerified')
  if (run.status === 'completed') return wb('verification.verifierNotVerified')
  return wb('verification.verifierFailed')
}

const getVerifierRunBadgeClass = (run: WorkbenchVerifierRun) => {
  if (run.verified) return 'badge-success'
  if (run.status === 'completed') return 'badge-warning'
  return 'badge-error'
}

const getAttemptOutcomeLabel = (outcome: WorkbenchExecutionAttemptOutcome) => {
  switch (outcome) {
    case 'changed':
      return wb('verification.changed')
    case 'same':
      return wb('verification.same')
    case 'blocked':
      return wb('verification.blocked')
    case 'not_found':
      return wb('verification.notFound')
    default:
      return wb('verification.error')
  }
}

const getAttemptOutcomeBadgeClass = (outcome: WorkbenchExecutionAttemptOutcome) => {
  switch (outcome) {
    case 'changed':
      return 'badge-success'
    case 'same':
      return 'badge-outline'
    case 'blocked':
      return 'badge-warning'
    case 'not_found':
      return 'badge-ghost'
    default:
      return 'badge-error'
  }
}

const getSimilarityLevelLabel = (level: WorkbenchExecutionSimilarityLevel) => {
  switch (level) {
    case 'high':
      return wb('verification.similarityLevel.high')
    case 'medium':
      return wb('verification.similarityLevel.medium')
    default:
      return wb('verification.similarityLevel.low')
  }
}

const getBaselineExchange = (evidence: Evidence): WorkbenchEvidenceExchange | null =>
  getWorkbenchVerificationBaselineExchange(evidence)

const getReplayExchange = (evidence: Evidence): WorkbenchEvidenceExchange | null =>
  getWorkbenchVerificationReplayExchange(evidence)

const formatExchangePayload = (raw?: string | null) => formatWorkbenchRawPayload(raw)
</script>
