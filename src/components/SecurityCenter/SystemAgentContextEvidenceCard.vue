<template>
  <div class="space-y-3">
    <div>
      <p class="text-sm font-semibold text-info mb-2">Agent 判定摘要</p>
      <pre class="bg-base-300 p-3 rounded text-xs overflow-x-auto whitespace-pre-wrap break-words">{{
        evidence.evidence_snippet
      }}</pre>
    </div>
    <div v-if="agentMeta">
      <p class="text-sm font-semibold text-base-content/70 mb-2">事件元信息</p>
      <pre class="bg-base-300 p-3 rounded text-xs overflow-x-auto whitespace-pre-wrap break-words">{{
        formatStructuredValue(agentMeta)
      }}</pre>
    </div>
    <div v-if="contextPayload">
      <p class="text-sm font-semibold text-primary mb-2">触发上下文</p>
      <pre
        class="bg-base-300 p-3 rounded text-xs overflow-x-auto max-h-80 whitespace-pre-wrap break-words"
      >{{ formatStructuredValue(contextPayload) }}</pre>
    </div>
    <div v-if="contextExtractionSummary" class="space-y-2">
      <p class="text-sm font-semibold text-primary mb-2">上下文抽取命中</p>
      <div class="bg-base-300 p-3 rounded-lg space-y-3 text-sm">
        <div class="flex flex-wrap items-center gap-2">
          <span class="badge badge-info badge-sm">动作 {{ contextExtractionSummary.actionKind }}</span>
          <span class="badge badge-outline badge-sm">
            {{ contextExtractionSummary.actionSource }}
          </span>
          <span
            v-if="contextExtractionSummary.matchedAlias"
            class="badge badge-ghost badge-sm font-mono"
          >
            alias {{ contextExtractionSummary.matchedAlias }}
          </span>
        </div>
        <div v-if="contextExtractionSummary.principalMatches.length" class="space-y-1 text-xs">
          <p class="font-semibold">主体字段</p>
          <div class="flex flex-wrap gap-1">
            <span
              v-for="entry in contextExtractionSummary.principalMatches"
              :key="`principal-${entry.configuredKey}-${entry.matchedKey}-${entry.source}`"
              class="badge badge-outline badge-xs"
            >
              {{ formatContextExtractionEntry(entry) }}
            </span>
          </div>
        </div>
        <div v-if="contextExtractionSummary.resourceMatches.length" class="space-y-1 text-xs">
          <p class="font-semibold">资源字段</p>
          <div class="flex flex-wrap gap-1">
            <span
              v-for="entry in contextExtractionSummary.resourceMatches"
              :key="`resource-${entry.configuredKey}-${entry.matchedKey}-${entry.source}`"
              class="badge badge-outline badge-xs"
            >
              {{ formatContextExtractionEntry(entry) }}
            </span>
          </div>
        </div>
        <div v-if="contextExtractionSummary.authMatches.length" class="space-y-1 text-xs">
          <p class="font-semibold">认证线索</p>
          <div class="flex flex-wrap gap-1">
            <span
              v-for="entry in contextExtractionSummary.authMatches"
              :key="`auth-${entry.configuredKey}-${entry.matchedKey}-${entry.source}`"
              class="badge badge-outline badge-xs"
            >
              {{ formatContextExtractionEntry(entry) }}
            </span>
          </div>
        </div>
      </div>
    </div>
    <div v-if="semanticSummary" class="space-y-2">
      <p class="text-sm font-semibold text-secondary mb-2">语义抽象</p>
      <div class="bg-base-300 p-3 rounded-lg space-y-3 text-sm">
        <div class="flex flex-wrap items-center gap-2">
          <span class="badge badge-secondary badge-sm">
            {{ formatSemanticSource(semanticSummary.source) }}
          </span>
          <span v-if="semanticSummary.signature" class="badge badge-ghost badge-sm font-mono">
            {{ semanticSummary.signature }}
          </span>
        </div>
        <div v-if="semanticSummary.actionCandidates.length" class="space-y-1 text-xs">
          <p class="font-semibold">动作语义</p>
          <div class="flex flex-wrap gap-1">
            <span
              v-for="entry in semanticSummary.actionCandidates"
              :key="`semantic-action-${entry.kind}-${entry.source}-${entry.confidence}`"
              class="badge badge-xs"
              :class="semanticConfidenceBadgeClass(entry.confidence)"
              :title="entry.reason"
            >
              {{ formatSemanticActionEntry(entry) }}
            </span>
          </div>
        </div>
        <div v-if="semanticSummary.principalCandidates.length" class="space-y-1 text-xs">
          <p class="font-semibold">主体候选</p>
          <div class="flex flex-wrap gap-1">
            <span
              v-for="entry in semanticSummary.principalCandidates"
              :key="`semantic-principal-${entry.field}-${entry.source}-${entry.confidence}`"
              class="badge badge-xs"
              :class="semanticConfidenceBadgeClass(entry.confidence)"
              :title="entry.reason"
            >
              {{ formatSemanticCandidateEntry(entry) }}
            </span>
          </div>
        </div>
        <div v-if="semanticSummary.resourceCandidates.length" class="space-y-1 text-xs">
          <p class="font-semibold">资源候选</p>
          <div class="flex flex-wrap gap-1">
            <span
              v-for="entry in semanticSummary.resourceCandidates"
              :key="`semantic-resource-${entry.field}-${entry.source}-${entry.confidence}`"
              class="badge badge-xs"
              :class="semanticConfidenceBadgeClass(entry.confidence)"
              :title="entry.reason"
            >
              {{ formatSemanticCandidateEntry(entry) }}
            </span>
          </div>
        </div>
        <div v-if="semanticSummary.credentialCandidates.length" class="space-y-1 text-xs">
          <p class="font-semibold">凭证候选</p>
          <div class="flex flex-wrap gap-1">
            <span
              v-for="entry in semanticSummary.credentialCandidates"
              :key="`semantic-credential-${entry.field}-${entry.source}-${entry.confidence}`"
              class="badge badge-xs"
              :class="semanticConfidenceBadgeClass(entry.confidence)"
              :title="entry.reason"
            >
              {{ formatSemanticCandidateEntry(entry) }}
            </span>
          </div>
        </div>
        <div v-if="semanticSummary.stateCandidates.length" class="space-y-1 text-xs">
          <p class="font-semibold">状态候选</p>
          <div class="flex flex-wrap gap-1">
            <span
              v-for="entry in semanticSummary.stateCandidates"
              :key="`semantic-state-${entry.field}-${entry.source}-${entry.confidence}`"
              class="badge badge-xs"
              :class="semanticConfidenceBadgeClass(entry.confidence)"
              :title="entry.reason"
            >
              {{ formatSemanticCandidateEntry(entry) }}
            </span>
          </div>
        </div>
        <div v-if="semanticSummary.notes.length" class="space-y-1 text-xs">
          <p class="font-semibold">抽象备注</p>
          <ul class="list-disc list-inside space-y-1 opacity-80">
            <li v-for="note in semanticSummary.notes" :key="note">{{ note }}</li>
          </ul>
        </div>
      </div>
    </div>
    <div v-if="behaviorSummary" class="space-y-2">
      <p class="text-sm font-semibold text-info mb-2">行为特征</p>
      <div class="bg-base-300 p-3 rounded-lg space-y-2 text-sm">
        <div class="flex flex-wrap items-center gap-2">
          <span class="badge badge-info badge-sm">{{ behaviorSummary.label }}</span>
          <span class="badge badge-outline badge-sm">
            {{ behaviorSummary.effectiveModeLabel }}
          </span>
          <span v-if="behaviorSummary.usedExtension" class="badge badge-success badge-sm">
            浏览器扩展已参与
          </span>
        </div>
        <p class="text-xs opacity-80 whitespace-pre-wrap break-words">
          {{ behaviorSummary.summary }}
        </p>
        <div v-if="behaviorSummary.steps.length" class="text-xs">
          <p class="font-semibold mb-1">行为步骤</p>
          <ul class="list-disc list-inside space-y-1">
            <li v-for="step in behaviorSummary.steps" :key="step">{{ step }}</li>
          </ul>
        </div>
        <div v-if="behaviorSummary.intentHints.length" class="text-xs">
          <p class="font-semibold mb-1">意图提示</p>
          <div class="flex flex-wrap gap-1">
            <span v-for="hint in behaviorSummary.intentHints" :key="hint" class="badge badge-outline badge-xs">
              {{ hint }}
            </span>
          </div>
        </div>
        <div
          v-if="behaviorSummary.lastPageTitle || behaviorSummary.lastPageUrl"
          class="grid grid-cols-1 md:grid-cols-2 gap-2 text-xs"
        >
          <div v-if="behaviorSummary.lastPageTitle">
            <p class="font-semibold mb-1">最后页面标题</p>
            <p class="whitespace-pre-wrap break-words opacity-80">
              {{ behaviorSummary.lastPageTitle }}
            </p>
          </div>
          <div v-if="behaviorSummary.lastPageUrl">
            <p class="font-semibold mb-1">最后页面 URL</p>
            <p class="whitespace-pre-wrap break-all opacity-80">{{ behaviorSummary.lastPageUrl }}</p>
          </div>
        </div>
      </div>
    </div>
    <div v-if="skillEntries.length > 0">
      <p class="text-sm font-semibold text-accent mb-2">命中 Skills</p>
      <div class="space-y-2">
        <div v-for="skill in skillEntries" :key="skill.id" class="bg-base-300 p-3 rounded-lg space-y-2">
          <div class="flex flex-wrap items-center gap-2">
            <span class="badge badge-accent badge-sm">{{ skill.id }}</span>
            <span v-if="typeof skill.score === 'number'" class="badge badge-outline badge-sm">
              score {{ skill.score.toFixed(2) }}
            </span>
          </div>
          <p class="text-sm font-medium">{{ skill.description || skill.name || skill.id }}</p>
          <p v-if="skill.whenToUse" class="text-xs opacity-70 whitespace-pre-wrap break-words">
            {{ skill.whenToUse }}
          </p>
          <div v-if="Array.isArray(skill.reasons) && skill.reasons.length > 0" class="text-xs">
            <p class="font-semibold mb-1">推荐理由</p>
            <ul class="list-disc list-inside space-y-1">
              <li v-for="reason in skill.reasons" :key="reason">{{ reason }}</li>
            </ul>
          </div>
          <div v-if="skill.guidance" class="text-xs">
            <p class="font-semibold mb-1">Skill 指导</p>
            <pre
              class="bg-base-200 p-2 rounded whitespace-pre-wrap break-words overflow-x-auto max-h-40"
            >{{ skill.guidance }}</pre>
          </div>
        </div>
      </div>
    </div>
    <div v-if="sopEntries.length > 0">
      <p class="text-sm font-semibold text-info mb-2">命中 SOP</p>
      <div class="space-y-2">
        <div v-for="sop in sopEntries" :key="sop.id" class="bg-base-300 p-3 rounded-lg space-y-2">
          <div class="flex flex-wrap items-center gap-2">
            <span class="badge badge-info badge-sm">{{ sop.id }}</span>
            <span class="text-sm font-medium">{{ sop.name || sop.id }}</span>
          </div>
          <p v-if="sop.description" class="text-xs opacity-80 whitespace-pre-wrap break-words">
            {{ sop.description }}
          </p>
          <div v-if="sop.procedure" class="text-xs">
            <p class="font-semibold mb-1">SOP 步骤</p>
            <pre
              class="bg-base-200 p-2 rounded whitespace-pre-wrap break-words overflow-x-auto max-h-40"
            >{{ sop.procedure }}</pre>
          </div>
        </div>
      </div>
    </div>
    <div v-if="invariantEntries.length > 0">
      <p class="text-sm font-semibold text-warning mb-2">命中不变量</p>
      <div class="flex flex-col gap-2">
        <div
          v-for="invariant in invariantEntries"
          :key="invariant.id || invariant.summary"
          class="bg-base-300 p-3 rounded-lg"
        >
          <div class="flex flex-wrap items-center gap-2 mb-1">
            <span class="badge badge-warning badge-sm">{{ invariant.id || 'invariant' }}</span>
            <span v-if="invariant.severity" class="badge badge-outline badge-sm">
              {{ invariant.severity }}
            </span>
          </div>
          <p class="text-sm whitespace-pre-wrap break-words">
            {{ invariant.summary || formatStructuredValue(invariant) }}
          </p>
        </div>
      </div>
    </div>
    <div v-if="processGraph">
      <p class="text-sm font-semibold text-secondary mb-2">过程图摘要</p>
      <pre
        class="bg-base-300 p-3 rounded text-xs overflow-x-auto max-h-80 whitespace-pre-wrap break-words"
      >{{ formatStructuredValue(processGraph) }}</pre>
    </div>
    <div v-if="agentOutput">
      <p class="text-sm font-semibold text-secondary mb-2">Agent 输出</p>
      <pre
        class="bg-base-300 p-3 rounded text-xs overflow-x-auto max-h-80 whitespace-pre-wrap break-words"
      >{{ formatStructuredValue(agentOutput) }}</pre>
    </div>
    <div class="text-xs opacity-70">
      <span>记录时间: {{ formatTime(evidence.timestamp) }}</span>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import {
  formatContextExtractionEntry,
  formatSemanticActionEntry,
  formatSemanticCandidateEntry,
  getContextExtractionSummaryFromPayload,
  getSemanticAbstractionSummaryFromPayload,
  semanticConfidenceBadgeClass,
} from '../system-agent/systemAgentTrafficSummaries'
import {
  formatStructuredValue,
  getContextBehaviorSummaryFromPayload,
  getContextInvariantEntriesFromPayload,
  getContextProcessGraphFromPayload,
  getContextSkillEntriesFromPayload,
  getContextSopEntriesFromPayload,
  parseStructuredPayload,
  parseSystemAgentMeta,
} from '../system-agent/systemAgentContextEvidenceSupport'

interface EvidencePayload {
  evidence_snippet: string
  request_body?: string
  response_headers?: string
  response_body?: string
  timestamp: string
}

const props = defineProps<{
  evidence: EvidencePayload
}>()

const contextPayload = computed(() => parseStructuredPayload(props.evidence.request_body))
const agentMeta = computed(() => parseSystemAgentMeta(props.evidence.response_headers))
const agentOutput = computed(() => parseStructuredPayload(props.evidence.response_body))
const contextExtractionSummary = computed(() =>
  getContextExtractionSummaryFromPayload(contextPayload.value)
)
const semanticSummary = computed(() => getSemanticAbstractionSummaryFromPayload(contextPayload.value))
const behaviorSummary = computed(() => getContextBehaviorSummaryFromPayload(contextPayload.value))
const skillEntries = computed(() => getContextSkillEntriesFromPayload(contextPayload.value))
const sopEntries = computed(() => getContextSopEntriesFromPayload(contextPayload.value))
const invariantEntries = computed(() => getContextInvariantEntriesFromPayload(contextPayload.value))
const processGraph = computed(() => getContextProcessGraphFromPayload(contextPayload.value))

const formatSemanticSource = (source?: string | null) => {
  if (source === 'ai_augmented') return 'AI 增强'
  if (source === 'fallback') return '确定性回退'
  return source || 'unknown'
}

const formatTime = (timestamp: string) => {
  if (!timestamp) return '-'
  return new Date(timestamp).toLocaleString('zh-CN')
}
</script>
