<template>
  <div class="space-y-4">
    <div
      v-if="assessmentSuggestion"
      class="rounded-lg border border-info/30 bg-info/5 p-4 space-y-3"
    >
      <div class="flex flex-wrap items-center justify-between gap-2">
        <div>
          <p class="text-sm font-semibold">{{ wb('overview.executionSuggestion') }}</p>
          <p class="text-xs text-base-content/60">{{ assessmentSuggestion.summary }}</p>
        </div>
        <div class="flex flex-wrap gap-2">
          <span
            :class="['badge', getWorkbenchConfidenceBadgeClass(assessmentSuggestion.confidence)]"
          >
            {{ wb('overview.confidence', { value: getWorkbenchConfidenceLabel(assessmentSuggestion.confidence) }) }}
          </span>
          <span :class="['badge', getWorkbenchStatusBadgeClass(assessmentSuggestion.suggestedStatus)]">
            {{ wb('overview.suggestedStatus', { value: getWorkbenchStatusLabel(assessmentSuggestion.suggestedStatus) }) }}
          </span>
        </div>
      </div>

      <p class="text-sm">{{ assessmentSuggestion.title }}</p>

      <div class="flex flex-wrap gap-2">
        <span
          v-for="signal in assessmentSuggestion.signals"
          :key="signal"
          class="badge badge-outline badge-sm"
        >
          {{ signal }}
        </span>
      </div>

      <div class="flex flex-wrap gap-2">
        <button class="btn btn-sm btn-outline" @click="applySuggestionToDraft">
          {{ wb('overview.applyToDraft') }}
        </button>
        <button class="btn btn-sm btn-info" @click="applySuggestionAndSave">
          {{ wb('overview.applyAndSave') }}
        </button>
        <button class="btn btn-sm btn-secondary" :disabled="syncingFinding" @click="syncFindingWithSuggestion">
          {{ wb('overview.syncWithSuggestion') }}
        </button>
      </div>
    </div>

    <div class="grid gap-4 md:grid-cols-2 xl:grid-cols-4">
      <div class="rounded-lg border border-base-300 bg-base-200/60 p-4">
        <p class="text-xs text-base-content/60 mb-1">{{ wb('overview.caseStatus') }}</p>
        <div class="flex items-center gap-2">
          <span :class="getWorkbenchStatusBadgeClass(caseItem.status)" class="badge">
            {{ getWorkbenchStatusLabel(caseItem.status) }}
          </span>
        </div>
      </div>
      <div class="rounded-lg border border-base-300 bg-base-200/60 p-4">
        <p class="text-xs text-base-content/60 mb-1">{{ wb('overview.priority') }}</p>
        <span class="badge badge-outline">{{ wb(`priority.${caseItem.priority}`) }}</span>
      </div>
      <div class="rounded-lg border border-base-300 bg-base-200/60 p-4">
        <p class="text-xs text-base-content/60 mb-1">{{ wb('overview.lastActivity') }}</p>
        <p class="text-sm">{{ formatWorkbenchTime(caseItem.lastActivityAt) }}</p>
      </div>
    </div>

    <div
      v-if="systemAgentStatuses.length"
      class="rounded-lg border border-base-300 bg-base-100 p-4 space-y-3"
    >
      <div>
        <p class="text-sm font-semibold">{{ wb('overview.agentPolicyTitle') }}</p>
        <p class="text-xs text-base-content/60">{{ wb('overview.agentPolicySummary') }}</p>
      </div>
      <div class="grid gap-3 lg:grid-cols-2">
        <div
          v-for="agent in systemAgentStatuses"
          :key="agent.profileId"
          class="rounded-lg border border-base-300 bg-base-200/60 p-4 space-y-3"
        >
          <div class="flex flex-wrap items-center justify-between gap-2">
            <div>
              <p class="text-sm font-semibold">{{ getAgentLabel(agent.profileId) }}</p>
              <p class="text-xs text-base-content/60">{{ agent.profileId }}</p>
            </div>
            <span :class="['badge', agent.enabled ? 'badge-success' : 'badge-ghost']">
              {{ agent.enabled ? wb('overview.agentEnabled') : wb('overview.agentDisabled') }}
            </span>
          </div>
          <div class="flex flex-wrap gap-2">
            <span :class="['badge badge-outline', agent.autoMode ? 'badge-success' : 'badge-ghost']">
              {{ wb('overview.autoMode') }}: {{ agent.autoMode ? wb('overview.enabled') : wb('overview.disabled') }}
            </span>
            <span :class="['badge badge-outline', agent.allowActiveReplay ? 'badge-warning' : 'badge-ghost']">
              {{ wb('overview.activeReplay') }}: {{ agent.allowActiveReplay ? wb('overview.enabled') : wb('overview.disabled') }}
            </span>
            <span :class="['badge badge-outline', agent.shadowMode ? 'badge-info' : 'badge-ghost']">
              {{ wb('overview.shadowMode') }}: {{ agent.shadowMode ? wb('overview.enabled') : wb('overview.disabled') }}
            </span>
            <span class="badge badge-outline">
              {{ wb('overview.scopeHosts') }}: {{ agent.scopeHosts.length }}
            </span>
          </div>
        </div>
      </div>
    </div>

    <div class="rounded-lg border border-base-300 bg-base-100 p-4 space-y-3">
      <div class="grid gap-3 md:grid-cols-2 xl:grid-cols-2">
        <label class="form-control">
          <span class="label-text text-xs text-base-content/60 mb-1">{{ wb('overview.caseStatus') }}</span>
          <select v-model="statusDraft" class="select select-bordered select-sm">
            <option value="new">{{ wb('status.new') }}</option>
            <option value="investigating">{{ wb('status.investigating') }}</option>
            <option value="awaiting_verification">{{ wb('status.awaiting_verification') }}</option>
            <option value="verified">{{ wb('status.verified') }}</option>
            <option value="false_positive">{{ wb('status.false_positive') }}</option>
            <option value="archived">{{ wb('status.archived') }}</option>
          </select>
        </label>
        <label class="form-control">
          <span class="label-text text-xs text-base-content/60 mb-1">{{ wb('overview.priority') }}</span>
          <select v-model="priorityDraft" class="select select-bordered select-sm">
            <option value="low">{{ wb('priority.low') }}</option>
            <option value="medium">{{ wb('priority.medium') }}</option>
            <option value="high">{{ wb('priority.high') }}</option>
          </select>
        </label>
      </div>

      <div>
        <p class="text-xs text-base-content/60 mb-1">{{ wb('overview.caseTitle') }}</p>
        <p class="text-lg font-semibold break-words">{{ findingTitle }}</p>
      </div>
      <div class="grid gap-4 lg:grid-cols-[minmax(0,2fr),minmax(0,1fr)]">
        <div>
          <p class="text-xs text-base-content/60 mb-1">{{ wb('overview.findingSummary') }}</p>
          <div class="rounded-lg bg-base-200/70 p-3 space-y-2">
            <div class="flex flex-wrap gap-2">
              <span class="badge badge-outline">{{ caseItem.finding.vulnType }}</span>
              <span class="badge badge-ghost">{{ caseItem.finding.pluginId }}</span>
              <span class="badge badge-info">{{ getWorkbenchConfidenceLabel(caseItem.finding.confidence) }}</span>
            </div>
            <p class="text-sm whitespace-pre-wrap break-words">{{ findingDescription }}</p>
          </div>
        </div>
        <div>
          <p class="text-xs text-base-content/60 mb-1">{{ wb('overview.currentConclusion') }}</p>
          <textarea
            v-model="conclusionDraft"
            class="textarea textarea-bordered min-h-[140px] w-full"
            :placeholder="wb('overview.conclusionPlaceholder')"
          />
          <div class="mt-3 flex flex-wrap gap-2">
            <button class="btn btn-sm btn-outline" @click="saveMetadata">{{ wb('overview.saveCaseInfo') }}</button>
            <button class="btn btn-sm btn-primary" @click="saveConclusion">{{ wb('overview.saveConclusion') }}</button>
            <button class="btn btn-sm btn-secondary" :disabled="syncingFinding" @click="syncFinding">
              <span v-if="syncingFinding" class="loading loading-spinner loading-xs mr-1"></span>
              {{ wb('overview.syncToFinding') }}
            </button>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import type {
  WorkbenchAssessmentSuggestion,
  WorkbenchCase,
  WorkbenchCaseStatus,
  WorkbenchSystemAgentStatus,
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
  getWorkbenchFindingDescription,
  getWorkbenchFindingTitle,
} from './securityWorkbenchSystemAgentContent'

const props = defineProps<{
  caseItem: WorkbenchCase
  syncingFinding: boolean
  assessmentSuggestion: WorkbenchAssessmentSuggestion | null
  systemAgentStatuses: WorkbenchSystemAgentStatus[]
}>()

const emit = defineEmits<{
  'save-conclusion': [value: string]
  'save-metadata': [patch: {
    status: WorkbenchCaseStatus
    priority: 'low' | 'medium' | 'high'
  }]
  'sync-finding': []
  'sync-finding-with-suggestion': []
}>()

const conclusionDraft = ref(props.caseItem.currentConclusion)
const statusDraft = ref<WorkbenchCaseStatus>(props.caseItem.status)
const priorityDraft = ref<'low' | 'medium' | 'high'>(props.caseItem.priority)
const assessmentSuggestion = computed(() => props.assessmentSuggestion)
const systemAgentStatuses = computed(() => props.systemAgentStatuses || [])
const findingTitle = computed(() => getWorkbenchFindingTitle(props.caseItem.finding))
const findingDescription = computed(() => getWorkbenchFindingDescription(props.caseItem.finding))

watch(
  () => props.caseItem,
  value => {
    conclusionDraft.value = value.currentConclusion
    statusDraft.value = value.status
    priorityDraft.value = value.priority
  },
  { deep: true },
)

const saveConclusion = () => {
  emit('save-conclusion', conclusionDraft.value)
}

const saveMetadata = () => {
  emit('save-metadata', {
    status: statusDraft.value,
    priority: priorityDraft.value,
  })
}

const applySuggestionToDraft = () => {
  if (!props.assessmentSuggestion) return
  statusDraft.value = props.assessmentSuggestion.suggestedStatus
  conclusionDraft.value = props.assessmentSuggestion.suggestedConclusion
}

const applySuggestionAndSave = () => {
  applySuggestionToDraft()
  saveMetadata()
  saveConclusion()
}

const syncFinding = () => {
  emit('sync-finding')
}

const syncFindingWithSuggestion = () => {
  emit('sync-finding-with-suggestion')
}

const getAgentLabel = (profileId: string) => {
  if (profileId === 'traffic_logic_triage') return wb('overview.agentLogic')
  if (profileId === 'traffic_active_verifier') return wb('overview.agentVerifier')
  return profileId
}
</script>
