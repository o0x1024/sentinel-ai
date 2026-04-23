<template>
  <div class="space-y-3">
    <div
      v-if="locationLabel || techniqueLabel || targetPathLabel || referenceStatusLabel || probeStatusLabel"
      class="flex flex-wrap gap-2"
    >
      <span v-if="locationLabel" class="badge badge-ghost badge-sm">
        Location: {{ locationLabel }}
      </span>
      <span v-if="techniqueLabel" class="badge badge-primary badge-sm">
        Technique: {{ techniqueLabel }}
      </span>
      <span
        v-if="targetPathLabel"
        class="badge badge-outline badge-sm font-mono max-w-full break-all"
      >
        Target Path: {{ targetPathLabel }}
      </span>
      <span
        v-if="referenceStatusLabel"
        :class="['badge badge-sm', getStatusBadgeClass(referenceStatusLabel)]"
      >
        Reference: {{ referenceStatusLabel }}
      </span>
      <span
        v-if="probeStatusLabel"
        :class="['badge badge-sm', getStatusBadgeClass(probeStatusLabel)]"
      >
        Probe: {{ probeStatusLabel }}
      </span>
    </div>

    <div :class="containerClass">
    <div v-if="requestRaw" class="min-w-0 space-y-2">
      <p v-if="requestTitle" class="text-xs font-medium text-base-content/70">{{ requestTitle }}</p>
      <div v-if="requestHighlightTerms.length" class="flex flex-wrap gap-2">
        <span class="badge badge-warning badge-sm">PoC</span>
        <span
          v-for="term in requestHighlightTerms"
          :key="`request:${term}`"
          class="badge badge-outline badge-sm font-mono max-w-full break-all"
        >
          {{ term }}
        </span>
      </div>
      <pre
        class="overflow-auto rounded-lg border border-base-300 bg-base-200 p-3 font-mono text-xs whitespace-pre-wrap break-all"
        :style="{ minHeight: height }"
        v-html="requestHtml"
      ></pre>
    </div>

    <div v-if="responseRaw" class="min-w-0 space-y-2">
      <p v-if="responseTitle" class="text-xs font-medium text-base-content/70">{{ responseTitle }}</p>
      <div v-if="responseHighlightTerms.length" class="flex flex-wrap gap-2">
        <span class="badge badge-success badge-sm">命中值</span>
        <span
          v-for="term in responseHighlightTerms"
          :key="`response:${term}`"
          class="badge badge-outline badge-sm font-mono max-w-full break-all"
        >
          {{ term }}
        </span>
      </div>
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

const props = withDefaults(defineProps<{
  exchange?: WorkbenchEvidenceExchange | null
  fallbackUrl?: string | null
  evidenceSnippet?: string | null
  stateKeyPrefix: string
  height?: string
  showRequest?: boolean
  showResponse?: boolean
  requestTitle?: string
  responseTitle?: string
}>(), {
  exchange: null,
  fallbackUrl: '',
  evidenceSnippet: '',
  height: '18rem',
  showRequest: true,
  showResponse: true,
  requestTitle: '',
  responseTitle: '',
})

const requestRaw = computed(() =>
  props.showRequest ? buildSecurityEvidenceRawRequest(props.exchange, props.fallbackUrl) : '',
)
const responseRaw = computed(() =>
  props.showResponse ? buildSecurityEvidenceRawResponse(props.exchange) : '',
)
const highlightBuckets = computed(() => extractSecurityEvidenceHighlightBuckets(props.evidenceSnippet))
const snippetFields = computed(() => extractSecurityEvidenceSnippetFields(props.evidenceSnippet))
const requestHighlightTerms = computed(() => highlightBuckets.value.requestTerms)
const responseHighlightTerms = computed(() => highlightBuckets.value.responseTerms)
const locationLabel = computed(() => snippetFields.value.location || '')
const techniqueLabel = computed(() => snippetFields.value.technique || '')
const targetPathLabel = computed(() => snippetFields.value.targetPath || '')
const referenceStatusLabel = computed(() => snippetFields.value.referenceStatus || '')
const probeStatusLabel = computed(() => snippetFields.value.probeStatus || '')
const requestHtml = computed(() => buildSecurityEvidenceRequestHtml(requestRaw.value, props.evidenceSnippet))
const responseHtml = computed(() => buildSecurityEvidenceResponseHtml(responseRaw.value, props.evidenceSnippet))
const containerClass = computed(() =>
  requestRaw.value && responseRaw.value ? 'grid gap-3 xl:grid-cols-2' : 'space-y-3',
)

function getStatusBadgeClass(status: string) {
  const numericStatus = Number(status)
  if (Number.isFinite(numericStatus)) {
    return numericStatus >= 400 ? 'badge-error' : 'badge-success'
  }
  return 'badge-outline'
}
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
