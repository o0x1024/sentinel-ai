<template>
  <div :class="containerClass">
    <div v-if="requestRaw" class="min-w-0 space-y-2">
      <p v-if="requestTitle" class="text-xs font-medium text-base-content/70">{{ requestTitle }}</p>
      <pre
        class="overflow-auto rounded-lg border border-base-300 bg-base-200 p-3 font-mono text-xs whitespace-pre-wrap break-all"
        :style="{ minHeight: height }"
      >{{ requestRaw }}</pre>
    </div>

    <div v-if="responseRaw" class="min-w-0 space-y-2">
      <p v-if="responseTitle" class="text-xs font-medium text-base-content/70">{{ responseTitle }}</p>
      <pre
        class="overflow-auto rounded-lg border border-base-300 bg-base-200 p-3 font-mono text-xs whitespace-pre-wrap break-all"
        :style="{ minHeight: height }"
      >{{ responseRaw }}</pre>
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

const props = withDefaults(defineProps<{
  exchange?: WorkbenchEvidenceExchange | null
  fallbackUrl?: string | null
  stateKeyPrefix: string
  height?: string
  showRequest?: boolean
  showResponse?: boolean
  requestTitle?: string
  responseTitle?: string
}>(), {
  exchange: null,
  fallbackUrl: '',
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
const containerClass = computed(() =>
  requestRaw.value && responseRaw.value ? 'grid gap-3 xl:grid-cols-2' : 'space-y-3',
)
</script>
