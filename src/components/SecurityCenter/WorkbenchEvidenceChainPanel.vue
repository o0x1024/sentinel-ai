<template>
  <div class="space-y-3">
    <div
      v-for="evidence in evidences"
      :key="evidence.id"
      :ref="setEvidenceRef(evidence.id)"
      :class="[
        'rounded-lg border bg-base-100 p-4 space-y-3',
        selectedEvidenceId === evidence.id
          ? 'border-primary ring-1 ring-primary/30'
          : 'border-base-300',
      ]"
    >
      <div class="flex flex-wrap items-center justify-between gap-2">
        <div class="flex flex-wrap items-center gap-2">
          <span class="badge badge-outline">{{ evidence.method }}</span>
          <span class="badge badge-ghost">{{
            getWorkbenchEvidenceLocationLabel(evidence.location)
          }}</span>
          <span v-if="caseItem.baselineEvidenceId === evidence.id" class="badge badge-primary">
            {{ wb('evidence.baseline') }}
          </span>
          <span v-if="selectedEvidenceId === evidence.id" class="badge badge-secondary">
            {{ wb('evidence.focused') }}
          </span>
        </div>
        <div class="flex items-center gap-2">
          <span class="text-xs text-base-content/60">{{
            formatWorkbenchTime(evidence.timestamp)
          }}</span>
          <SecurityEvidenceTransferActions
            :evidence="evidence"
            size="xs"
            direction="down"
            :messages="transferMessages"
          />
          <button
            v-if="caseItem.baselineEvidenceId !== evidence.id"
            class="btn btn-xs btn-outline"
            @click="$emit('set-baseline', evidence.id)"
          >
            {{ wb('evidence.setBaseline') }}
          </button>
        </div>
      </div>

      <p class="font-mono text-xs break-all text-base-content/70">{{ evidence.url }}</p>
      <pre
        class="bg-base-200 p-3 rounded text-xs whitespace-pre-wrap break-words overflow-x-auto"
        >{{ getWorkbenchEvidenceSnippet(evidence) }}</pre
      >

      <div
        v-if="hasWorkbenchEvidenceExchange(evidence) && getWorkbenchEvidenceExchange(evidence)"
        class="space-y-2"
      >
        <div class="flex flex-wrap gap-2 text-xs text-base-content/60">
          <span class="badge badge-outline">{{ wb('evidence.rawExchange') }}</span>
          <span class="font-mono"
            >{{ getWorkbenchEvidenceExchange(evidence)?.requestMethod }}
            {{ getWorkbenchEvidenceExchange(evidence)?.requestUrl }}</span
          >
          <span
            v-if="typeof getWorkbenchEvidenceExchange(evidence)?.responseStatus === 'number'"
            :class="[
              'badge badge-sm',
              (getWorkbenchEvidenceExchange(evidence)?.responseStatus || 0) >= 400
                ? 'badge-error'
                : 'badge-success',
            ]"
          >
            {{
              wb('evidence.responseStatusLabel', {
                value: getWorkbenchEvidenceExchange(evidence)?.responseStatus,
              })
            }}
          </span>
        </div>

        <details
          v-if="hasRequestPayload(evidence)"
          class="rounded-lg border border-base-300 bg-base-100"
        >
          <summary class="cursor-pointer px-3 py-2 text-sm font-medium">
            {{ wb('evidence.requestPanel') }}
          </summary>
          <div class="border-t border-base-300 px-3 py-3">
            <SecurityEvidenceRawExchangePanel
              :exchange="getWorkbenchEvidenceExchange(evidence)"
              :fallback-url="evidence.url"
              :state-key-prefix="`security-workbench:evidence:${evidence.id}:request`"
              :height="rawPanelHeight"
              :show-response="false"
            />
          </div>
        </details>

        <details
          v-if="hasResponsePayload(evidence)"
          class="rounded-lg border border-base-300 bg-base-100"
        >
          <summary class="cursor-pointer px-3 py-2 text-sm font-medium">
            {{ wb('evidence.responsePanel') }}
          </summary>
          <div class="border-t border-base-300 px-3 py-3">
            <SecurityEvidenceRawExchangePanel
              :exchange="getWorkbenchEvidenceExchange(evidence)"
              :fallback-url="evidence.url"
              :state-key-prefix="`security-workbench:evidence:${evidence.id}:response`"
              :height="rawPanelHeight"
              :show-request="false"
            />
          </div>
        </details>
      </div>
    </div>
    <div
      v-if="evidences.length === 0"
      class="rounded-lg border border-dashed border-base-300 p-6 text-center text-sm text-base-content/60"
    >
      {{ wb('evidence.empty') }}
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, ref, watch } from 'vue'
import type { WorkbenchCase } from './securityWorkbenchTypes'
import type { Evidence } from './vulnerabilityFindingTypes'
import SecurityEvidenceRawExchangePanel from './SecurityEvidenceRawExchangePanel.vue'
import SecurityEvidenceTransferActions from './SecurityEvidenceTransferActions.vue'
import { formatWorkbenchTime } from './securityWorkbenchPresentation'
import { wb } from './securityWorkbenchLocale'
import {
  getWorkbenchEvidenceLocationLabel,
  getWorkbenchEvidenceExchange,
  getWorkbenchEvidenceSnippet,
  hasWorkbenchEvidenceExchange,
} from './securityWorkbenchSystemAgentContent'
const props = defineProps<{
  caseItem: WorkbenchCase
  selectedEvidenceId?: string | null
}>()

defineEmits<{
  'set-baseline': [evidenceId: string]
}>()

const evidences = computed(() => props.caseItem.finding.evidence || [])
const evidenceRefs = ref<Record<string, HTMLElement | null>>({})
const transferMessages = computed(() => ({
  triggerLabel: wb('evidence.sendTo'),
  sendToRepeater: wb('evidence.sendToRepeater'),
  sendToIntruder: wb('evidence.sendToIntruder'),
  noTransferableRequest: wb('evidence.noTransferableRequest'),
  sentToRepeater: wb('evidence.sentToRepeater'),
  sentToIntruder: wb('evidence.sentToIntruder'),
  transferFailed: wb('evidence.transferFailed', { error: '{error}' }),
}))
const rawPanelHeight = '18rem'

const hasRequestPayload = (evidence: Evidence) => {
  const exchange = getWorkbenchEvidenceExchange(evidence)
  return Boolean(exchange?.requestUrl || exchange?.requestHeaders || exchange?.requestBody)
}

const hasResponsePayload = (evidence: Evidence) => {
  const exchange = getWorkbenchEvidenceExchange(evidence)
  return Boolean(
    typeof exchange?.responseStatus === 'number' ||
      exchange?.responseHeaders ||
      exchange?.responseBody
  )
}

const setEvidenceRef = (evidenceId: string) => (element: Element | null) => {
  evidenceRefs.value[evidenceId] = element instanceof HTMLElement ? element : null
}

watch(
  () => props.selectedEvidenceId,
  async evidenceId => {
    if (!evidenceId) return
    await nextTick()
    evidenceRefs.value[evidenceId]?.scrollIntoView({
      behavior: 'smooth',
      block: 'center',
    })
  },
  { immediate: true }
)
</script>
