<template>
  <div class="space-y-3">
    <SecurityEvidenceSnippetFields
      :location="locationLabel"
      :technique="techniqueLabel"
      :target-path="targetPathLabel"
      :reference-status="referenceStatusLabel"
      :probe-status="probeStatusLabel"
    />

    <div :class="containerClass">
      <div v-if="requestRaw" class="min-w-0 space-y-2">
        <p v-if="requestTitle" class="text-xs font-medium text-base-content/70">
          {{ requestTitle }}
        </p>
        <SecurityEvidenceHighlightTermList
          v-if="requestHighlightTerms.length"
          label="PoC"
          tone="request"
          :terms="requestHighlightTerms"
        />
        <pre
          class="overflow-auto rounded-lg border border-base-300 bg-base-200 p-3 font-mono text-xs whitespace-pre-wrap break-all"
          :style="{ minHeight: height }"
          v-html="requestHtml"
        ></pre>
      </div>

      <div v-if="responseRaw" class="min-w-0 space-y-2">
        <p v-if="responseTitle" class="text-xs font-medium text-base-content/70">
          {{ responseTitle }}
        </p>
        <SecurityEvidenceHighlightTermList
          v-if="responseHighlightTerms.length"
          label="命中值"
          tone="response"
          :terms="responseHighlightTerms"
        />
        <pre
          class="overflow-auto rounded-lg border border-base-300 bg-base-200 p-3 font-mono text-xs whitespace-pre-wrap break-all"
          :style="{ minHeight: height }"
          v-html="responseHtml"
        ></pre>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import SecurityEvidenceHighlightTermList from './SecurityEvidenceHighlightTermList.vue'
import SecurityEvidenceSnippetFields from './SecurityEvidenceSnippetFields.vue'
import type { WorkbenchEvidenceExchange } from './securityWorkbenchSystemAgentContent'
import {
  buildSecurityEvidenceRawRequest,
  buildSecurityEvidenceRawResponse,
} from './securityEvidenceHttpSupport'
import {
  buildSecurityEvidenceRequestHtml,
  buildSecurityEvidenceResponseHtml,
  extractSecurityEvidenceHighlightBuckets,
  extractSecurityEvidenceSnippetFields,
} from './securityEvidenceHighlightSupport'

const props = withDefaults(
  defineProps<{
    exchange?: WorkbenchEvidenceExchange | null
    fallbackUrl?: string | null
    evidenceSnippet?: string | null
    stateKeyPrefix: string
    height?: string
    showRequest?: boolean
    showResponse?: boolean
    requestTitle?: string
    responseTitle?: string
  }>(),
  {
    exchange: null,
    fallbackUrl: '',
    evidenceSnippet: '',
    height: '18rem',
    showRequest: true,
    showResponse: true,
    requestTitle: '',
    responseTitle: '',
  }
)

const requestRaw = computed(() =>
  props.showRequest ? buildSecurityEvidenceRawRequest(props.exchange, props.fallbackUrl) : ''
)
const responseRaw = computed(() =>
  props.showResponse ? buildSecurityEvidenceRawResponse(props.exchange) : ''
)
const highlightBuckets = computed(() =>
  extractSecurityEvidenceHighlightBuckets(props.evidenceSnippet)
)
const snippetFields = computed(() => extractSecurityEvidenceSnippetFields(props.evidenceSnippet))
const requestHighlightTerms = computed(() => highlightBuckets.value.requestTerms)
const responseHighlightTerms = computed(() => highlightBuckets.value.responseTerms)
const locationLabel = computed(() => snippetFields.value.location || '')
const techniqueLabel = computed(() => snippetFields.value.technique || '')
const targetPathLabel = computed(() => snippetFields.value.targetPath || '')
const referenceStatusLabel = computed(() => snippetFields.value.referenceStatus || '')
const probeStatusLabel = computed(() => snippetFields.value.probeStatus || '')
const requestHtml = computed(() =>
  buildSecurityEvidenceRequestHtml(requestRaw.value, props.evidenceSnippet)
)
const responseHtml = computed(() =>
  buildSecurityEvidenceResponseHtml(responseRaw.value, props.evidenceSnippet)
)
const containerClass = computed(() =>
  requestRaw.value && responseRaw.value ? 'grid gap-3 xl:grid-cols-2' : 'space-y-3'
)
</script>

<style scoped>
.security-evidence-hit {
  border-radius: 0.25rem;
  padding: 0 0.12rem;
  font-weight: 700;
}

.security-evidence-hit--request {
  background: color-mix(in srgb, oklch(var(--wa)) 28%, transparent);
  color: inherit;
}

.security-evidence-hit--response {
  background: color-mix(in srgb, oklch(var(--su)) 28%, transparent);
  color: inherit;
}
</style>
