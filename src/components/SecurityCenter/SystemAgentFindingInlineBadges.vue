<template>
  <template v-if="isSystemAgentFinding(finding)">
    <span
      v-if="behaviorSummary?.label"
      :class="behaviorSummary.usedExtension ? 'badge-secondary' : 'badge-neutral'"
      class="badge badge-xs"
    >
      {{ behaviorSummary.label }}
    </span>
    <span
      v-if="semanticBadge"
      :class="semanticBadge.className"
      :title="semanticBadge.title"
      class="badge badge-xs"
    >
      {{ semanticBadge.label }}
    </span>
    <span
      v-if="hypothesisBadge"
      :class="hypothesisBadge.className"
      :title="hypothesisBadge.title"
      class="badge badge-xs"
    >
      {{ hypothesisBadge.label }}
    </span>
  </template>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { getLogicHypothesisBadgeFromPayload, getSemanticBadgeFromPayload } from '../system-agent/systemAgentFindingBadges'
import { getContextBehaviorSummaryFromPayload, parseStructuredPayload } from '../system-agent/systemAgentContextEvidenceSupport'
import { isSystemAgentFinding } from './vulnerabilityFindingPresentation'
import type { Finding } from './vulnerabilityFindingTypes'

const props = defineProps<{
  finding: Finding
}>()

const systemAgentPayload = computed(() => {
  const evidence = props.finding.evidence?.find(item => item.location === 'system_agent_context')
  if (!evidence?.request_body) return null
  return parseStructuredPayload(evidence.request_body)
})

const behaviorSummary = computed(() => getContextBehaviorSummaryFromPayload(systemAgentPayload.value))
const semanticBadge = computed(() => getSemanticBadgeFromPayload(systemAgentPayload.value))
const hypothesisBadge = computed(() => getLogicHypothesisBadgeFromPayload(systemAgentPayload.value))
</script>
