<template>
  <AppModal :open="open" box-class="max-w-5xl" top-aligned @close="$emit('close')">
    <div class="space-y-4">
      <div class="flex items-start justify-between gap-3">
        <div>
          <h3 class="text-lg font-semibold">上下文词典候选</h3>
          <p class="text-sm text-base-content/70">
            基于历史记录推荐主体字段、资源主键、认证字段和动作别名，确认后会合并回当前上下文抽取词典。
          </p>
        </div>
        <button class="btn btn-sm btn-ghost" @click="$emit('close')">关闭</button>
      </div>

      <div v-if="loading" class="flex items-center justify-center rounded-lg border border-base-300 bg-base-200/40 px-4 py-12">
        <span class="loading loading-spinner loading-md mr-3"></span>
        <span class="text-sm text-base-content/70">正在分析历史记录并生成候选...</span>
      </div>

      <div v-else-if="result" class="space-y-4">
        <div class="rounded-lg border border-base-300 bg-base-200/40 p-3 text-sm text-base-content/70">
          分析了 {{ result.analyzedRequestCount }} 条请求
          <span v-if="result.skippedRequestCount > 0">，跳过 {{ result.skippedRequestCount }} 条缺失记录</span>
          ，当前共推荐 {{ visibleCandidates.length }} 个候选。
        </div>

        <div
          v-if="previewResult"
          class="rounded-lg border border-info/30 bg-info/5 p-3 space-y-3"
        >
          <div class="text-sm font-medium text-info">应用前预览命中变化</div>
          <div class="flex flex-wrap gap-2 text-xs text-base-content/80">
            <span>分析 {{ previewResult.analyzedRequestCount }} 条请求</span>
            <span>命中变化 {{ previewResult.changedRequestCount }} 条</span>
            <button
              v-if="previewResult.principalMatchDeltaCount > 0"
              type="button"
              class="badge badge-outline badge-xs transition-colors border-success/40 bg-success/10 text-success hover:border-success hover:bg-success/15"
              :class="activePreviewKind === 'principal' ? 'ring-2 ring-success/30' : ''"
              @click="openPreviewCategorySample('principal')"
            >
              主体 +{{ previewResult.principalMatchDeltaCount }}
            </button>
            <button
              v-if="previewResult.resourceMatchDeltaCount > 0"
              type="button"
              class="badge badge-outline badge-xs transition-colors border-primary/40 bg-primary/10 text-primary hover:border-primary hover:bg-primary/15"
              :class="activePreviewKind === 'resource' ? 'ring-2 ring-primary/30' : ''"
              @click="openPreviewCategorySample('resource')"
            >
              资源 +{{ previewResult.resourceMatchDeltaCount }}
            </button>
            <button
              v-if="previewResult.authHeaderMatchDeltaCount > 0"
              type="button"
              class="badge badge-outline badge-xs transition-colors border-warning/40 bg-warning/10 text-warning hover:border-warning hover:bg-warning/15"
              :class="activePreviewKind === 'authHeader' ? 'ring-2 ring-warning/30' : ''"
              @click="openPreviewCategorySample('authHeader')"
            >
              认证头 +{{ previewResult.authHeaderMatchDeltaCount }}
            </button>
            <button
              v-if="previewResult.authTokenMatchDeltaCount > 0"
              type="button"
              class="badge badge-outline badge-xs transition-colors border-secondary/40 bg-secondary/10 text-secondary hover:border-secondary hover:bg-secondary/15"
              :class="activePreviewKind === 'authToken' ? 'ring-2 ring-secondary/30' : ''"
              @click="openPreviewCategorySample('authToken')"
            >
              Token +{{ previewResult.authTokenMatchDeltaCount }}
            </button>
            <button
              v-if="previewResult.cookieMatchDeltaCount > 0"
              type="button"
              class="badge badge-outline badge-xs transition-colors border-accent/40 bg-accent/10 text-accent hover:border-accent hover:bg-accent/15"
              :class="activePreviewKind === 'cookie' ? 'ring-2 ring-accent/30' : ''"
              @click="openPreviewCategorySample('cookie')"
            >
              Cookie +{{ previewResult.cookieMatchDeltaCount }}
            </button>
            <button
              v-if="previewResult.actionKindChangeCount > 0"
              type="button"
              class="badge badge-outline badge-xs transition-colors border-info/40 bg-info/10 text-info hover:border-info hover:bg-info/15"
              :class="activePreviewKind === 'action' ? 'ring-2 ring-info/30' : ''"
              @click="openPreviewCategorySample('action')"
            >
              动作变化 {{ previewResult.actionKindChangeCount }}
            </button>
          </div>

          <div v-if="previewResult.samples.length > 0" class="space-y-2">
            <div class="text-xs text-base-content/60">样例请求</div>
            <div
              v-for="sample in previewResult.samples"
              :key="sample.requestId"
              :ref="(element) => setPreviewSampleRef(sample.requestId, element)"
              class="rounded-lg border border-base-300 bg-base-100 p-2 text-xs space-y-1 cursor-pointer transition-colors hover:border-info/40 hover:bg-base-100/90"
              :class="activePreviewSampleRequestId === sample.requestId ? 'ring-2 ring-info/50 border-info/40 bg-info/5' : ''"
              role="button"
              tabindex="0"
              @click="openPreviewSample(sample, buildPreviewSampleEvidenceSelection(sample), null)"
              @keydown.enter.prevent="openPreviewSample(sample, buildPreviewSampleEvidenceSelection(sample), null)"
              @keydown.space.prevent="openPreviewSample(sample, buildPreviewSampleEvidenceSelection(sample), null)"
            >
              <div class="flex items-start justify-between gap-2">
                <div class="font-mono break-all min-w-0 flex-1">{{ sample.method }} {{ sample.url }}</div>
                <span
                  v-if="activePreviewSampleRequestId === sample.requestId"
                  class="badge badge-info badge-xs shrink-0"
                >
                  已定位
                </span>
              </div>
              <div class="flex flex-wrap gap-1">
                <button
                  v-for="item in sampleSummaryBadges(sample)"
                  :key="item.label"
                  type="button"
                  class="badge badge-outline badge-xs transition-colors"
                  :class="[item.toneClass, getPreviewSampleBadgeActiveClass(sample.requestId, item.kind)]"
                  @click.stop="openPreviewSample(sample, item.payload, item.kind)"
                  @keydown.enter.stop.prevent="openPreviewSample(sample, item.payload, item.kind)"
                  @keydown.space.stop.prevent="openPreviewSample(sample, item.payload, item.kind)"
                >
                  {{ item.label }}
                </button>
              </div>
            </div>
          </div>
        </div>

        <div v-if="visibleCandidates.length > 0" class="max-h-[65vh] space-y-4 overflow-y-auto pr-1">
          <div
            v-for="group in groupedCandidates"
            :key="group.category"
            class="space-y-3"
          >
            <div class="flex items-center justify-between gap-2">
              <div class="flex items-center gap-2">
                <h4 class="text-sm font-semibold">{{ group.label }}</h4>
                <span class="badge badge-outline badge-sm">{{ group.items.length }}</span>
              </div>
            </div>

            <div class="space-y-3">
              <label
                v-for="candidate in group.items"
                :key="buildTrafficContextCandidateId(candidate)"
                class="block rounded-lg border border-base-300 bg-base-100 p-3"
                :class="candidate.alreadyCoveredBy ? 'opacity-60' : 'cursor-pointer hover:border-primary/40'"
              >
                <div class="flex items-start gap-3">
                  <input
                    type="checkbox"
                    class="checkbox checkbox-sm mt-0.5"
                    :checked="selectedCandidateIds.includes(buildTrafficContextCandidateId(candidate))"
                    :disabled="Boolean(candidate.alreadyCoveredBy)"
                    @change="toggleCandidate(candidate)"
                  />
                  <div class="min-w-0 flex-1 space-y-2">
                    <div class="flex flex-wrap items-center gap-2">
                      <span class="font-medium">{{ candidate.key }}</span>
                      <span
                        class="badge badge-sm"
                        :class="getTrafficContextCandidateConfidenceBadgeClass(candidate.confidence)"
                      >
                        {{ candidate.confidence }}
                      </span>
                      <span class="badge badge-outline badge-sm">score {{ candidate.score }}</span>
                      <span
                        v-if="candidate.category === 'action_alias' && candidate.suggestedCanonicalAction"
                        class="badge badge-info badge-sm"
                      >
                        {{ candidate.suggestedCanonicalAction }} = {{ candidate.key }}
                      </span>
                      <span v-if="candidate.alreadyCoveredBy" class="badge badge-ghost badge-sm">
                        已覆盖
                      </span>
                    </div>

                    <div class="flex flex-wrap gap-2 text-xs text-base-content/70">
                      <span>命中 {{ candidate.evidenceCount }} 条请求</span>
                      <span>不同值 {{ candidate.distinctValueCount }}</span>
                      <span>来源 {{ candidate.sources.join(', ') || '-' }}</span>
                    </div>

                    <div v-if="candidate.exampleValues.length" class="space-y-1">
                      <div class="text-xs text-base-content/50">示例值</div>
                      <div class="space-y-1">
                        <div
                          v-for="value in candidate.exampleValues"
                          :key="value"
                          class="w-full rounded-md border border-base-300 bg-base-200/40 px-2 py-0.5 font-mono text-[10px] leading-4 whitespace-pre-wrap break-all select-text"
                        >
                          {{ value }}
                        </div>
                      </div>
                    </div>

                    <div v-if="candidate.ruleReasons.length" class="text-xs text-base-content/70">
                      {{ candidate.ruleReasons.join('；') }}
                    </div>

                    <div v-if="candidate.behaviorReason" class="text-xs text-info">
                      {{ candidate.behaviorReason }}
                    </div>

                    <div v-if="candidate.behaviorEvidence.length" class="flex flex-wrap gap-1">
                      <span
                        v-for="item in candidate.behaviorEvidence"
                        :key="item"
                        class="badge badge-info badge-outline badge-xs"
                      >
                        {{ item }}
                      </span>
                    </div>

                    <div v-if="candidate.exampleLocations.length" class="text-xs text-base-content/50">
                      位置示例：{{ candidate.exampleLocations.join('，') }}
                    </div>

                    <div v-if="candidate.evidenceRequests.length" class="space-y-1">
                      <div class="text-xs text-base-content/50">证据请求</div>
                      <div
                        v-for="request in candidate.evidenceRequests"
                        :key="`${candidate.key}-${request.requestId}`"
                        class="rounded border border-base-300 bg-base-200/40 px-2 py-1 text-xs space-y-1 cursor-pointer transition-colors hover:border-primary/40 hover:bg-base-200"
                        role="button"
                        tabindex="0"
                        @click.stop="emit('open-evidence-request', { requestId: request.requestId, matchedLocations: request.matchedLocations })"
                        @keydown.enter.stop.prevent="emit('open-evidence-request', { requestId: request.requestId, matchedLocations: request.matchedLocations })"
                        @keydown.space.stop.prevent="emit('open-evidence-request', { requestId: request.requestId, matchedLocations: request.matchedLocations })"
                      >
                        <div class="font-mono break-all">{{ request.method }} {{ request.url }}</div>
                        <div
                          v-if="request.matchedLocations.length"
                          class="text-[11px] text-base-content/50"
                        >
                          命中位置：{{ request.matchedLocations.join('，') }}
                        </div>
                      </div>
                    </div>

                    <div v-if="candidate.alreadyCoveredBy" class="text-xs text-base-content/50">
                      已被当前词典覆盖：{{ candidate.alreadyCoveredBy }}
                    </div>
                  </div>
                </div>
              </label>
            </div>
          </div>
        </div>

        <div
          v-else
          class="rounded-lg border border-dashed border-base-300 bg-base-200/30 px-4 py-10 text-center text-sm text-base-content/60"
        >
          当前范围内没有生成足够稳定的候选。建议缩小到同一业务流程、同一 host 或同一路径组后重试。
        </div>
      </div>

      <div class="modal-action mt-2 flex items-center justify-between">
        <div class="flex items-center gap-2">
          <button
            class="btn btn-sm btn-ghost"
            :disabled="selectedCandidateIds.length === 0 || previewing"
            @click="$emit('preview')"
          >
            {{ previewing ? '预览中' : '预览命中变化' }}
          </button>
          <button
            class="btn btn-sm btn-ghost"
            :disabled="!result || selectableCandidates.length === 0"
            @click="selectHighConfidence"
          >
            全选高置信度
          </button>
          <button
            class="btn btn-sm btn-ghost"
            :disabled="selectedCandidateIds.length === 0"
            @click="$emit('update:selectedCandidateIds', [])"
          >
            清空选择
          </button>
        </div>
        <button
          class="btn btn-sm btn-primary"
          :disabled="selectedCandidateIds.length === 0 || applying"
          @click="$emit('apply')"
        >
          {{ applying ? '应用中' : `应用 ${selectedCandidateIds.length} 项到上下文词典` }}
        </button>
      </div>
    </div>
  </AppModal>
</template>

<script setup lang="ts">
import { computed, nextTick, ref, watch } from 'vue'

import AppModal from '@/components/AppModal.vue'

import {
  buildTrafficContextCandidateId,
  getTrafficContextCandidateConfidenceBadgeClass,
  sortTrafficContextCandidates,
  trafficContextCandidateCategoryLabels,
  trafficContextCandidateCategoryOrder,
} from './trafficContextCandidateSupport'
import {
  buildTrafficContextPreviewVersion,
  isTrafficContextPreviewFocusFresh,
} from './trafficContextPreviewFocusSupport'
import type {
  TrafficContextCandidateEvidenceSelection,
  TrafficContextPreviewFocus,
  TrafficContextPreviewKind,
  RecommendTrafficContextDictionaryCandidatesResponse,
  TrafficContextDictionaryCandidate,
  TrafficContextExtractionPreviewSample,
  TrafficContextExtractionPreviewResponse,
} from './trafficContextCandidateTypes'

const props = defineProps<{
  open: boolean
  loading: boolean
  applying?: boolean
  previewing?: boolean
  result: RecommendTrafficContextDictionaryCandidatesResponse | null
  previewResult?: TrafficContextExtractionPreviewResponse | null
  selectedCandidateIds: string[]
  preferredPreviewFocus?: TrafficContextPreviewFocus | null
}>()

const emit = defineEmits<{
  (event: 'close'): void
  (event: 'apply'): void
  (event: 'preview'): void
  (event: 'open-evidence-request', payload: TrafficContextCandidateEvidenceSelection): void
  (event: 'update:preferredPreviewFocus', value: TrafficContextPreviewFocus): void
  (event: 'update:selectedCandidateIds', value: string[]): void
}>()

const visibleCandidates = computed(() =>
  sortTrafficContextCandidates(props.result?.candidates || []),
)

const selectableCandidates = computed(() =>
  visibleCandidates.value.filter(candidate => !candidate.alreadyCoveredBy),
)

const groupedCandidates = computed(() =>
  trafficContextCandidateCategoryOrder
    .map(category => ({
      category,
      label: trafficContextCandidateCategoryLabels[category],
      items: visibleCandidates.value.filter(candidate => candidate.category === category),
    }))
    .filter(group => group.items.length > 0),
)
const activePreviewSampleRequestId = ref<number | null>(null)
const activePreviewKind = ref<PreviewSampleBadgeKind | null>(null)
const previewSampleRefs = ref(new Map<number, HTMLElement>())
const currentPreviewVersion = computed(() => buildTrafficContextPreviewVersion(props.previewResult))

interface PreviewSampleBadge {
  label: string
  payload: TrafficContextCandidateEvidenceSelection
  toneClass: string
  kind: PreviewSampleBadgeKind
}

type PreviewSampleBadgeKind = TrafficContextPreviewKind

function toggleCandidate(candidate: TrafficContextDictionaryCandidate) {
  const candidateId = buildTrafficContextCandidateId(candidate)
  const next = new Set(props.selectedCandidateIds)
  if (next.has(candidateId)) {
    next.delete(candidateId)
  } else {
    next.add(candidateId)
  }
  emit('update:selectedCandidateIds', [...next])
}

function selectHighConfidence() {
  emit(
    'update:selectedCandidateIds',
    selectableCandidates.value
      .filter(candidate => candidate.confidence === 'high')
      .map(buildTrafficContextCandidateId),
  )
}

function sampleSummaryBadges(sample: TrafficContextExtractionPreviewSample): PreviewSampleBadge[] {
  const badges: PreviewSampleBadge[] = []
  sample.addedPrincipalKeys.forEach(key => badges.push(buildPreviewSampleBadge(sample, `主体 +${key}`, [key], 'principal')))
  sample.addedResourceKeys.forEach(key => badges.push(buildPreviewSampleBadge(sample, `资源 +${key}`, [key], 'resource')))
  sample.addedAuthHeaders.forEach(key => badges.push(buildPreviewSampleBadge(sample, `认证头 +${key}`, [key], 'authHeader')))
  sample.addedAuthTokens.forEach(key => badges.push(buildPreviewSampleBadge(sample, `Token +${key}`, [key], 'authToken')))
  sample.addedCookieKeys.forEach(key => badges.push(buildPreviewSampleBadge(sample, `Cookie +${key}`, [key], 'cookie')))
  if (
    sample.actionKindBefore
    && sample.actionKindAfter
    && sample.actionKindBefore !== sample.actionKindAfter
  ) {
    badges.push(
      buildPreviewSampleBadge(
        sample,
        `动作 ${sample.actionKindBefore} -> ${sample.actionKindAfter}`,
        [sample.actionKindAfter, sample.actionKindBefore].filter(Boolean),
        'action',
      ),
    )
  }
  return badges
}

function buildPreviewSampleBadge(
  sample: TrafficContextExtractionPreviewSample,
  label: string,
  searchTerms: string[],
  kind: PreviewSampleBadgeKind,
): PreviewSampleBadge {
  return {
    label,
    payload: {
      requestId: sample.requestId,
      pane: 'request',
      matchedLocations: [],
      searchTerms: [...new Set(searchTerms.filter(Boolean))],
    },
    toneClass: getPreviewSampleBadgeToneClass(kind),
    kind,
  }
}

function getPreviewSampleBadgeToneClass(
  kind: PreviewSampleBadgeKind,
) {
  switch (kind) {
    case 'principal':
      return 'border-success/40 bg-success/10 text-success hover:border-success hover:bg-success/15'
    case 'resource':
      return 'border-primary/40 bg-primary/10 text-primary hover:border-primary hover:bg-primary/15'
    case 'authHeader':
      return 'border-warning/40 bg-warning/10 text-warning hover:border-warning hover:bg-warning/15'
    case 'authToken':
      return 'border-secondary/40 bg-secondary/10 text-secondary hover:border-secondary hover:bg-secondary/15'
    case 'cookie':
      return 'border-accent/40 bg-accent/10 text-accent hover:border-accent hover:bg-accent/15'
    case 'action':
      return 'border-info/40 bg-info/10 text-info hover:border-info hover:bg-info/15'
    default:
      return 'hover:border-info hover:bg-info/10'
  }
}

function getPreviewSampleBadgeActiveClass(
  requestId: number,
  kind: PreviewSampleBadgeKind,
) {
  if (activePreviewSampleRequestId.value !== requestId || activePreviewKind.value !== kind) {
    return ''
  }

  switch (kind) {
    case 'principal':
      return 'ring-2 ring-success/30 border-success bg-success/20 shadow-sm'
    case 'resource':
      return 'ring-2 ring-primary/30 border-primary bg-primary/20 shadow-sm'
    case 'authHeader':
      return 'ring-2 ring-warning/30 border-warning bg-warning/20 shadow-sm'
    case 'authToken':
      return 'ring-2 ring-secondary/30 border-secondary bg-secondary/20 shadow-sm'
    case 'cookie':
      return 'ring-2 ring-accent/30 border-accent bg-accent/20 shadow-sm'
    case 'action':
      return 'ring-2 ring-info/30 border-info bg-info/20 shadow-sm'
    default:
      return ''
  }
}

function openPreviewCategorySample(kind: PreviewSampleBadgeKind) {
  const sample = props.previewResult?.samples.find((item) => previewSampleHasKind(item, kind))
  if (!sample) {
    return
  }
  void openPreviewSample(sample, buildPreviewSampleCategorySelection(sample, kind), kind)
}

function previewSampleHasKind(sample: TrafficContextExtractionPreviewSample, kind: PreviewSampleBadgeKind) {
  switch (kind) {
    case 'principal':
      return sample.addedPrincipalKeys.length > 0
    case 'resource':
      return sample.addedResourceKeys.length > 0
    case 'authHeader':
      return sample.addedAuthHeaders.length > 0
    case 'authToken':
      return sample.addedAuthTokens.length > 0
    case 'cookie':
      return sample.addedCookieKeys.length > 0
    case 'action':
      return Boolean(
        sample.actionKindBefore
        && sample.actionKindAfter
        && sample.actionKindBefore !== sample.actionKindAfter,
      )
    default:
      return false
  }
}

function buildPreviewSampleCategorySelection(
  sample: TrafficContextExtractionPreviewSample,
  kind: PreviewSampleBadgeKind,
): TrafficContextCandidateEvidenceSelection {
  switch (kind) {
    case 'principal':
      return buildPreviewSampleEvidenceSelectionFromTerms(sample, sample.addedPrincipalKeys)
    case 'resource':
      return buildPreviewSampleEvidenceSelectionFromTerms(sample, sample.addedResourceKeys)
    case 'authHeader':
      return buildPreviewSampleEvidenceSelectionFromTerms(sample, sample.addedAuthHeaders)
    case 'authToken':
      return buildPreviewSampleEvidenceSelectionFromTerms(sample, sample.addedAuthTokens)
    case 'cookie':
      return buildPreviewSampleEvidenceSelectionFromTerms(sample, sample.addedCookieKeys)
    case 'action':
      return buildPreviewSampleEvidenceSelectionFromTerms(
        sample,
        [sample.actionKindAfter, sample.actionKindBefore].filter(Boolean),
      )
    default:
      return buildPreviewSampleEvidenceSelection(sample)
  }
}

function buildPreviewSampleEvidenceSelectionFromTerms(
  sample: TrafficContextExtractionPreviewSample,
  searchTerms: string[],
): TrafficContextCandidateEvidenceSelection {
  return {
    requestId: sample.requestId,
    pane: 'request',
    matchedLocations: [],
    searchTerms: [...new Set(searchTerms.filter(Boolean))],
  }
}

function buildPreviewSampleEvidenceSelection(
  sample: TrafficContextExtractionPreviewSample,
): TrafficContextCandidateEvidenceSelection {
  const searchTerms = [
    ...sample.addedPrincipalKeys,
    ...sample.addedResourceKeys,
    ...sample.addedAuthHeaders,
    ...sample.addedAuthTokens,
    ...sample.addedCookieKeys,
  ]

  if (
    sample.actionKindAfter
    && sample.actionKindAfter !== sample.actionKindBefore
  ) {
    searchTerms.push(sample.actionKindAfter)
  }

  return buildPreviewSampleEvidenceSelectionFromTerms(sample, searchTerms)
}

async function openPreviewSample(
  sample: TrafficContextExtractionPreviewSample,
  payload: TrafficContextCandidateEvidenceSelection,
  kind: PreviewSampleBadgeKind | null,
) {
  activePreviewSampleRequestId.value = sample.requestId
  activePreviewKind.value = kind
  emit('update:preferredPreviewFocus', {
    requestId: sample.requestId,
    kind,
    updatedAt: Date.now(),
    previewVersion: currentPreviewVersion.value,
  })
  await nextTick()
  previewSampleRefs.value.get(sample.requestId)?.scrollIntoView({
    block: 'nearest',
    behavior: 'smooth',
  })
  emit('open-evidence-request', payload)
}

function setPreviewSampleRef(requestId: number, element: unknown) {
  if (!(element instanceof HTMLElement)) {
    previewSampleRefs.value.delete(requestId)
    return
  }
  previewSampleRefs.value.set(requestId, element)
}

watch(
  () => [
    props.previewResult?.samples.map(sample => sample.requestId).join('|') || '',
    props.preferredPreviewFocus?.requestId || 0,
    props.preferredPreviewFocus?.kind || '',
    props.preferredPreviewFocus?.updatedAt || 0,
    props.preferredPreviewFocus?.previewVersion || '',
    currentPreviewVersion.value || '',
  ] as const,
  async () => {
    previewSampleRefs.value.clear()

    if (!isTrafficContextPreviewFocusFresh(props.preferredPreviewFocus)) {
      activePreviewSampleRequestId.value = null
      activePreviewKind.value = null
      return
    }

    const preferredRequestId = props.preferredPreviewFocus?.requestId || null
    const preferredKind = props.preferredPreviewFocus?.kind || null
    const canRestoreExactSample = Boolean(
      preferredRequestId
      && props.preferredPreviewFocus?.previewVersion
      && currentPreviewVersion.value
      && props.preferredPreviewFocus.previewVersion === currentPreviewVersion.value,
    )
    const exactSample = canRestoreExactSample && preferredRequestId
      ? props.previewResult?.samples.find((sample) => sample.requestId === preferredRequestId) || null
      : null
    const nextSample = exactSample || (
      preferredKind
        ? props.previewResult?.samples.find((sample) => previewSampleHasKind(sample, preferredKind)) || null
        : null
    )

    if (!nextSample) {
      activePreviewSampleRequestId.value = null
      activePreviewKind.value = null
      return
    }

    activePreviewSampleRequestId.value = nextSample.requestId
    activePreviewKind.value = preferredKind && previewSampleHasKind(nextSample, preferredKind)
      ? preferredKind
      : null
    await nextTick()
    previewSampleRefs.value.get(nextSample.requestId)?.scrollIntoView({
      block: 'nearest',
      behavior: 'smooth',
    })
  },
)
</script>
