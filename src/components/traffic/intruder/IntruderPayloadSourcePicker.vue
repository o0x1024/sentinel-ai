<template>
  <Teleport to="body">
    <div v-if="open" class="modal modal-open" @click.self="$emit('close')">
      <div class="modal-box max-w-6xl">
        <div class="flex items-start justify-between gap-4">
          <div>
            <h3 class="text-base font-semibold">{{ t('trafficAnalysis.intruder.labels.payloadLibrary') }}</h3>
            <p class="mt-1 text-xs text-base-content/60">
              {{ t('trafficAnalysis.intruder.help.payloadLibraryHint') }}
            </p>
          </div>
          <button class="btn btn-ghost btn-xs" type="button" @click="$emit('close')">✕</button>
        </div>

        <div class="mt-4 flex flex-wrap gap-2">
          <button
            v-for="tab in tabs"
            :key="tab.id"
            class="btn btn-sm"
            :class="activeTab === tab.id ? 'btn-primary' : 'btn-outline'"
            type="button"
            @click="activeTab = tab.id"
          >
            {{ tab.label }}
          </button>
        </div>

        <div class="mt-4 grid gap-3 md:grid-cols-[minmax(0,1fr)_12rem_auto]">
          <input
            v-model.trim="searchTerm"
            type="text"
            class="input input-bordered input-sm"
            :placeholder="t('trafficAnalysis.intruder.placeholders.searchPayloadLibrary')"
          />
          <select
            v-if="activeTab === 'dictionary'"
            v-model="dictionaryType"
            class="select select-bordered select-sm"
          >
            <option value="">{{ t('trafficAnalysis.intruder.labels.allDictionaryTypes') }}</option>
            <option v-for="dictType in dictionaryTypeOptions" :key="dictType" :value="dictType">
              {{ getDictionaryTypeLabel(dictType) }}
            </option>
          </select>
          <select
            v-else-if="activeTab === 'default'"
            v-model="defaultDictionaryType"
            class="select select-bordered select-sm"
          >
            <option value="">{{ t('trafficAnalysis.intruder.labels.pleaseSelect') }}</option>
            <option v-for="dictType in dictionaryTypeOptions" :key="dictType" :value="dictType">
              {{ getDictionaryTypeLabel(dictType) }}
            </option>
          </select>
          <div v-else class="hidden md:block"></div>
          <label class="flex items-center gap-2">
            <input
              :checked="showFavoritesOnly"
              type="checkbox"
              class="checkbox checkbox-sm"
              @change="showFavoritesOnly = ($event.target as HTMLInputElement).checked"
            />
            <span class="text-xs text-base-content/70">{{ t('trafficAnalysis.intruder.labels.favoritesOnly') }}</span>
          </label>
        </div>

        <div v-if="pickerError" class="mt-3 rounded-lg border border-error/30 bg-error/10 px-3 py-2 text-sm text-error">
          {{ pickerError }}
        </div>

        <div class="mt-4 grid gap-4 lg:grid-cols-[minmax(0,1fr)_minmax(20rem,26rem)]">
          <div class="rounded-lg border border-base-300 bg-base-100">
            <div class="border-b border-base-300 px-4 py-3 text-xs text-base-content/60">
              {{ sourceListSummary }}
            </div>

            <div class="max-h-[32rem] overflow-auto">
              <div
                v-for="source in visibleSources"
                :key="source.ref"
                class="border-b border-base-300/70 px-4 py-3 last:border-b-0"
                :class="selectedSourceRef === source.ref ? 'bg-primary/10' : ''"
              >
                <div class="flex items-start gap-3">
                  <button
                    class="btn btn-ghost btn-xs mt-0.5"
                    type="button"
                    :data-testid="`payload-library-favorite-${source.ref}`"
                    @click.stop="toggleFavorite(source.ref)"
                  >
                    {{ isFavorite(source.ref) ? '★' : '☆' }}
                  </button>
                  <button
                    class="min-w-0 flex-1 text-left"
                    type="button"
                    :data-testid="`payload-library-source-${source.ref}`"
                    @click="selectSource(source.ref)"
                  >
                    <div class="flex flex-wrap items-center gap-2">
                      <div class="font-medium">{{ source.label }}</div>
                      <span v-if="source.recommended" class="badge badge-success badge-outline badge-xs">
                        {{ t('trafficAnalysis.intruder.labels.recommended') }}
                      </span>
                      <span v-if="isRecent(source.ref)" class="badge badge-outline badge-xs">
                        {{ t('trafficAnalysis.intruder.labels.recent') }}
                      </span>
                      <span v-if="source.kind === 'template'" class="badge badge-primary badge-outline badge-xs">
                        {{ t(`trafficAnalysis.intruder.attackTypes.${source.recommendedAttackType}`) }}
                      </span>
                    </div>
                    <div class="mt-1 text-xs text-base-content/60">
                      {{ source.description }}
                    </div>
                    <div class="mt-2 flex flex-wrap gap-2 text-[11px]">
                      <span v-for="tag in source.tags.slice(0, 6)" :key="`${source.ref}-${tag}`" class="badge badge-ghost badge-sm">
                        {{ tag }}
                      </span>
                    </div>
                  </button>
                </div>
              </div>

              <div v-if="!visibleSources.length" class="px-4 py-10 text-center text-sm text-base-content/60">
                {{ t('trafficAnalysis.intruder.empty.noPayloadLibrarySources') }}
              </div>
            </div>
          </div>

          <div class="rounded-lg border border-base-300 bg-base-100 p-4">
            <div v-if="selectedSource" class="space-y-4">
              <div>
                <div class="flex flex-wrap items-center gap-2">
                  <div class="text-base font-semibold">{{ selectedSource.label }}</div>
                  <span class="badge badge-outline">{{ selectedSource.count }}</span>
                </div>
                <div class="mt-1 text-sm text-base-content/70">{{ selectedSource.description }}</div>
              </div>

              <div class="grid grid-cols-2 gap-3 text-xs text-base-content/70">
                <div>
                  <div>{{ t('trafficAnalysis.intruder.labels.currentRequestEstimate') }}</div>
                  <div class="mt-1 font-semibold text-base-content">{{ currentEstimatedRequests }}</div>
                </div>
                <div>
                  <div>{{ t('trafficAnalysis.intruder.labels.importedRequestEstimate') }}</div>
                  <div class="mt-1 font-semibold text-base-content">{{ importedEstimatedRequests }}</div>
                </div>
                <div>
                  <div>{{ t('trafficAnalysis.intruder.labels.currentPayloadCount') }}</div>
                  <div class="mt-1 font-semibold text-base-content">{{ currentItems.length }}</div>
                </div>
                <div>
                  <div>{{ t('trafficAnalysis.intruder.labels.importedPayloadCount') }}</div>
                  <div class="mt-1 font-semibold text-base-content">{{ importedItems.length }}</div>
                </div>
              </div>

              <div v-if="selectedSource.kind !== 'template'" class="grid gap-3 md:grid-cols-2">
                <label class="form-control">
                  <span class="label-text text-xs">{{ t('trafficAnalysis.intruder.labels.importMode') }}</span>
                  <select v-model="importMode" class="select select-bordered select-sm">
                    <option value="append">{{ t('trafficAnalysis.intruder.importModes.append') }}</option>
                    <option value="replace">{{ t('trafficAnalysis.intruder.importModes.replace') }}</option>
                    <option value="mergeDeduplicate">{{ t('trafficAnalysis.intruder.importModes.mergeDeduplicate') }}</option>
                  </select>
                </label>
                <label v-if="selectedSource.kind !== 'builtIn'" class="form-control">
                  <span class="label-text text-xs">{{ t('trafficAnalysis.intruder.labels.dictionaryPayloadLimit') }}</span>
                  <input
                    v-model.number="importLimit"
                    type="number"
                    min="1"
                    max="5000"
                    class="input input-bordered input-sm"
                  />
                </label>
              </div>

              <div class="rounded-lg border border-base-300 bg-base-50 p-3">
                <div class="text-xs font-semibold uppercase tracking-wide text-base-content/60">
                  {{ t('trafficAnalysis.intruder.labels.recommendationReason') }}
                </div>
                <div class="mt-2 text-sm text-base-content/70">
                  <template v-if="selectedSource.kind === 'template'">
                    {{ t('trafficAnalysis.intruder.help.payloadTemplateHint') }}
                  </template>
                  <template v-else>
                    {{ recommendationSummary }}
                  </template>
                </div>
              </div>

              <div class="rounded-lg border border-base-300 p-3">
                <div class="text-sm font-medium">{{ t('trafficAnalysis.intruder.labels.payloadPreview') }}</div>
                <div class="mt-2 flex flex-wrap gap-2">
                  <span
                    v-for="(payload, index) in previewItems"
                    :key="`${selectedSource.ref}-${index}-${payload}`"
                    class="badge badge-outline"
                  >
                    {{ payload }}
                  </span>
                </div>
              </div>

              <div class="rounded-lg border border-base-300 p-3">
                <div class="text-sm font-medium">{{ t('trafficAnalysis.intruder.labels.payloadQuality') }}</div>
                <div class="mt-3 grid grid-cols-2 gap-3 text-xs text-base-content/70">
                  <div>
                    <div>{{ t('trafficAnalysis.intruder.labels.uniquePayloads') }}</div>
                    <div class="mt-1 font-semibold text-base-content">{{ qualityStats.unique }}</div>
                  </div>
                  <div>
                    <div>{{ t('trafficAnalysis.intruder.labels.duplicatePayloads') }}</div>
                    <div class="mt-1 font-semibold text-base-content">{{ qualityStats.duplicates }}</div>
                  </div>
                  <div>
                    <div>{{ t('trafficAnalysis.intruder.labels.averageLength') }}</div>
                    <div class="mt-1 font-semibold text-base-content">{{ qualityStats.averageLength }}</div>
                  </div>
                  <div>
                    <div>{{ t('trafficAnalysis.intruder.labels.weakPayloadRatio') }}</div>
                    <div class="mt-1 font-semibold text-base-content">{{ qualityStats.weakRatio }}</div>
                  </div>
                  <div>
                    <div>{{ t('trafficAnalysis.intruder.labels.dirtyPayloads') }}</div>
                    <div class="mt-1 font-semibold text-base-content">{{ qualityStats.dirtyCount }}</div>
                  </div>
                  <div>
                    <div>{{ t('trafficAnalysis.intruder.labels.unicodePayloads') }}</div>
                    <div class="mt-1 font-semibold text-base-content">{{ qualityStats.charsetDistribution.withUnicode }}</div>
                  </div>
                </div>
              </div>

              <div v-if="selectedSource.kind === 'template'" class="rounded-lg border border-base-300 p-3">
                <div class="text-sm font-medium">{{ t('trafficAnalysis.intruder.labels.payloadTemplateSets') }}</div>
                <div class="mt-3 space-y-2">
                  <div
                    v-for="templateSet in selectedTemplateSets"
                    :key="`${selectedSource.ref}-${templateSet.name}`"
                    class="rounded-lg border border-base-300 px-3 py-2 text-sm"
                  >
                    <div class="font-medium">{{ templateSet.name }}</div>
                    <div class="mt-1 text-xs text-base-content/60">{{ templateSet.sourceId }}</div>
                  </div>
                </div>
              </div>
            </div>

            <div v-else class="py-10 text-center text-sm text-base-content/60">
              {{ t('trafficAnalysis.intruder.empty.noPayloadLibrarySourceSelected') }}
            </div>
          </div>
        </div>

        <div class="modal-action">
          <button
            v-if="selectedSource?.kind === 'template'"
            class="btn btn-primary btn-sm"
            type="button"
            :disabled="selectedTemplateSets.length === 0"
            @click="applyTemplate"
          >
            {{ t('trafficAnalysis.intruder.actions.applyTemplate') }}
          </button>
          <button
            v-else
            class="btn btn-primary btn-sm"
            type="button"
            :disabled="importedItems.length === 0"
            @click="applyImport"
          >
            {{ t('trafficAnalysis.intruder.actions.importPayloads') }}
          </button>
          <button class="btn btn-outline btn-sm" type="button" @click="$emit('close')">
            {{ t('common.cancel', '取消') }}
          </button>
        </div>
      </div>
    </div>
  </Teleport>
</template>

<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import {
  buildIntruderPayloadPreview,
  buildIntruderPayloadTemplateApplicationSets,
  estimateIntruderRequestsAfterImport,
  getIntruderPayloadSetItems,
  type IntruderPayloadImportMode,
} from './intruderPayloadLibrarySupport'
import {
  getIntruderBuiltInPayloadListItems,
  getIntruderPayloadQualityStats,
  INTRUDER_BUILT_IN_PAYLOAD_LISTS,
  INTRUDER_PAYLOAD_TEMPLATES,
  matchesIntruderBuiltInPayloadList,
  rankIntruderBuiltInPayloadLists,
} from './intruderBuiltInPayloadLists'
import {
  getIntruderDefaultDictionaryMap,
  getIntruderDictionaryTypeTranslationKey,
  INTRUDER_DICTIONARY_TYPE_OPTIONS,
  listIntruderDictionariesPaged,
  listIntruderDictionaryWords,
  type IntruderDictionarySummary,
} from './intruderDictionaries'
import {
  getIntruderPayloadLibraryPreferences,
  markIntruderPayloadLibraryRecent,
  toggleIntruderPayloadLibraryFavorite,
} from './intruderPayloadLibraryPreferences'
import type { IntruderAttackType, IntruderPayloadSet, IntruderPosition } from './types'

type PayloadLibraryTab = 'builtIn' | 'dictionary' | 'default' | 'template'
type PayloadSourceKind = 'builtIn' | 'dictionary' | 'default' | 'template'

interface PayloadLibrarySourceItem {
  ref: string
  kind: PayloadSourceKind
  label: string
  description: string
  count: number
  tags: string[]
  recommended: boolean
  recommendedAttackType?: IntruderAttackType
}

interface DictionaryImportSource {
  dictionaryId: string
  dictionaryName: string
  dictType: string
}

const DICTIONARY_PAGE_SIZE = 50

const props = defineProps<{
  open: boolean
  payloadSet: IntruderPayloadSet
  payloadSets: IntruderPayloadSet[]
  attackType: IntruderAttackType
  positions: IntruderPosition[]
  requestText: string
}>()

const emit = defineEmits<{
  (e: 'close'): void
  (e: 'import', value: { mode: IntruderPayloadImportMode; payloads: string[]; sourceRef: string }): void
  (e: 'apply-template', value: {
    sourceRef: string
    attackType: IntruderAttackType
    sets: Array<{ name: string; payloadsText: string }>
  }): void
}>()

const { t } = useI18n()

const tabs = computed(() => [
  { id: 'builtIn' as const, label: t('trafficAnalysis.intruder.labels.builtInLists') },
  { id: 'dictionary' as const, label: t('trafficAnalysis.intruder.labels.appDictionary') },
  { id: 'default' as const, label: t('trafficAnalysis.intruder.labels.defaultDictionary') },
  { id: 'template' as const, label: t('trafficAnalysis.intruder.labels.payloadTemplates') },
])

const activeTab = ref<PayloadLibraryTab>('builtIn')
const searchTerm = ref('')
const showFavoritesOnly = ref(false)
const importMode = ref<IntruderPayloadImportMode>('append')
const selectedSourceRef = ref('')
const pickerError = ref('')
const dictionaryType = ref('')
const defaultDictionaryType = ref('')
const dictionaryPage = ref(1)
const importLimit = ref(200)
const dictionaries = ref<IntruderDictionarySummary[]>([])
const dictionariesTotal = ref(0)
const defaultDictionaryMap = ref<Record<string, string>>({})
const preferences = ref(getIntruderPayloadLibraryPreferences())
const selectedDictionarySource = ref<DictionaryImportSource | null>(null)
const selectedDefaultDictionaryId = ref('')
const resolvedPreviewItems = ref<string[]>([])

const dictionaryTypeOptions = [...INTRUDER_DICTIONARY_TYPE_OPTIONS]
const currentItems = computed(() => getIntruderPayloadSetItems(props.payloadSet))
const currentEstimatedRequests = computed(() =>
  estimateIntruderRequestsAfterImport({
    attackType: props.attackType,
    positionsLength: props.positions.length,
    payloadSets: props.payloadSets,
    activePayloadSetId: props.payloadSet.id,
    nextPayloadsText: props.payloadSet.payloadsText,
  }),
)
const activePayloadSetIndex = computed(() =>
  Math.max(0, props.payloadSets.findIndex((payloadSet) => payloadSet.id === props.payloadSet.id)),
)
const activePositionValues = computed(() => {
  if (props.attackType === 'sniper' || props.attackType === 'batteringRam') {
    return props.positions.map((position) => position.value)
  }

  return props.positions[activePayloadSetIndex.value]
    ? [props.positions[activePayloadSetIndex.value].value]
    : props.positions.map((position) => position.value)
})
const recommendedListIds = computed(() =>
  rankIntruderBuiltInPayloadLists({
    attackType: props.attackType,
    requestText: props.requestText,
    positionValues: activePositionValues.value,
  }),
)
const builtInSources = computed<PayloadLibrarySourceItem[]>(() =>
  INTRUDER_BUILT_IN_PAYLOAD_LISTS
    .filter((definition) => matchesIntruderBuiltInPayloadList(definition, searchTerm.value))
    .map((definition): PayloadLibrarySourceItem => ({
      ref: `builtin:${definition.id}`,
      kind: 'builtIn',
      label: t(definition.labelKey),
      description: t(definition.descriptionKey),
      count: definition.items.length,
      tags: definition.tags,
      recommended: recommendedListIds.value.includes(definition.id),
    }))
    .sort((left, right) => compareSourceItems(left, right, preferences.value, recommendedListIds.value.map((id) => `builtin:${id}`))),
)
const dictionarySources = computed<PayloadLibrarySourceItem[]>(() =>
  dictionaries.value
    .filter((dictionary) => {
      if (!searchTerm.value.trim()) {
        return true
      }
      const haystack = [
        dictionary.id,
        dictionary.name,
        dictionary.description || '',
        dictionary.dict_type,
      ].join(' ').toLowerCase()
      return haystack.includes(searchTerm.value.trim().toLowerCase())
    })
    .map((dictionary): PayloadLibrarySourceItem => ({
      ref: `dictionary:${dictionary.id}`,
      kind: 'dictionary',
      label: dictionary.name,
      description: dictionary.description || `${getDictionaryTypeLabel(dictionary.dict_type)} · ${dictionary.id}`,
      count: dictionary.word_count || 0,
      tags: [getDictionaryTypeLabel(dictionary.dict_type)],
      recommended: false,
    }))
    .sort((left, right) => compareSourceItems(left, right, preferences.value, [])),
)
const defaultSources = computed<PayloadLibrarySourceItem[]>(() =>
  dictionaryTypeOptions
    .filter((dictType) => !defaultDictionaryType.value || dictType === defaultDictionaryType.value)
    .filter((dictType) => {
      if (!searchTerm.value.trim()) {
        return true
      }
      const haystack = `${dictType} ${getDictionaryTypeLabel(dictType)}`.toLowerCase()
      return haystack.includes(searchTerm.value.trim().toLowerCase())
    })
    .map((dictType): PayloadLibrarySourceItem => ({
      ref: `default:${dictType}`,
      kind: 'default',
      label: getDictionaryTypeLabel(dictType),
      description: defaultDictionaryMap.value[dictType]
        ? `${t('trafficAnalysis.intruder.labels.defaultDictionaryLabel', { type: getDictionaryTypeLabel(dictType) })} · ${defaultDictionaryMap.value[dictType]}`
        : t('trafficAnalysis.intruder.labels.noDefaultDictionaryConfigured'),
      count: 0,
      tags: [dictType],
      recommended: false,
    }))
    .sort((left, right) => compareSourceItems(left, right, preferences.value, [])),
)
const templateSources = computed<PayloadLibrarySourceItem[]>(() =>
  INTRUDER_PAYLOAD_TEMPLATES
    .filter((template) => {
      if (!searchTerm.value.trim()) {
        return true
      }
      const haystack = `${template.id} ${template.tags.join(' ')} ${template.recommendedPositionHints.join(' ')}`.toLowerCase()
      return haystack.includes(searchTerm.value.trim().toLowerCase())
    })
    .map((template): PayloadLibrarySourceItem => ({
      ref: `template:${template.id}`,
      kind: 'template',
      label: t(template.labelKey),
      description: t(template.descriptionKey),
      count: template.sets.length,
      tags: template.tags,
      recommended: template.recommendedPositionHints.some((hint) => activePositionValues.value.join(' ').toLowerCase().includes(hint)),
      recommendedAttackType: template.recommendedAttackType,
    }))
    .sort((left, right) => compareSourceItems(left, right, preferences.value, [])),
)
const visibleSources = computed(() => {
  const sourceMap: Record<PayloadLibraryTab, PayloadLibrarySourceItem[]> = {
    builtIn: builtInSources.value,
    dictionary: dictionarySources.value,
    default: defaultSources.value,
    template: templateSources.value,
  }

  const nextSources = sourceMap[activeTab.value]
  if (!showFavoritesOnly.value) {
    return nextSources
  }

  return nextSources.filter((item) => isFavorite(item.ref))
})
const selectedSource = computed(() =>
  visibleSources.value.find((item) => item.ref === selectedSourceRef.value)
    || builtInSources.value.find((item) => item.ref === selectedSourceRef.value)
    || dictionarySources.value.find((item) => item.ref === selectedSourceRef.value)
    || defaultSources.value.find((item) => item.ref === selectedSourceRef.value)
    || templateSources.value.find((item) => item.ref === selectedSourceRef.value)
    || null,
)
const importedItems = computed(() => {
  if (!selectedSource.value) {
    return []
  }
  if (selectedSource.value.kind === 'template') {
    return []
  }
  return resolvedPreviewItems.value
})
const importedEstimatedRequests = computed(() => {
  if (!selectedSource.value || selectedSource.value.kind === 'template') {
    return currentEstimatedRequests.value
  }

  const nextPayloadsText = buildNextPayloadsText(importedItems.value)
  return estimateIntruderRequestsAfterImport({
    attackType: props.attackType,
    positionsLength: props.positions.length,
    payloadSets: props.payloadSets,
    activePayloadSetId: props.payloadSet.id,
    nextPayloadsText,
  })
})
const previewItems = computed(() => buildIntruderPayloadPreview(importedItems.value))
const qualityStats = computed(() => getIntruderPayloadQualityStats(importedItems.value))
const selectedTemplateSets = computed(() => {
  if (!selectedSource.value || selectedSource.value.kind !== 'template') {
    return []
  }
  const templateId = selectedSource.value.ref.replace('template:', '')
  const template = INTRUDER_PAYLOAD_TEMPLATES.find((item) => item.id === templateId)
  if (!template) {
    return []
  }
  return buildIntruderPayloadTemplateApplicationSets(template.sets.map((item) => ({
    name: t(item.nameKey),
    sourceId: item.sourceId,
  })))
})
const sourceListSummary = computed(() => {
  if (activeTab.value === 'dictionary') {
    return t('trafficAnalysis.intruder.labels.dictionaryPageSummary', {
      page: dictionaryPage.value,
      total: Math.max(1, Math.ceil(dictionariesTotal.value / DICTIONARY_PAGE_SIZE)),
      count: dictionariesTotal.value,
    })
  }

  return t('trafficAnalysis.intruder.labels.payloadLibraryCount', {
    count: visibleSources.value.length,
  })
})
const recommendationSummary = computed(() => {
  if (!selectedSource.value) {
    return ''
  }

  const hints = activePositionValues.value.join(', ') || t('trafficAnalysis.intruder.labels.detectedPositions')
  return t('trafficAnalysis.intruder.help.payloadLibraryRecommendationHint', {
    attackType: t(`trafficAnalysis.intruder.attackTypes.${props.attackType}`),
    positions: hints,
  })
})

watch(
  () => props.open,
  async (open) => {
    if (!open) {
      return
    }

    preferences.value = getIntruderPayloadLibraryPreferences()
    pickerError.value = ''
    await Promise.all([
      refreshDictionaries(),
      refreshDefaultDictionaryMap(),
    ])
    ensureDefaultSelection()
    await refreshSelectedSourcePreview()
  },
  { immediate: true },
)

watch([activeTab, searchTerm, dictionaryType, defaultDictionaryType, importLimit], async () => {
  if (!props.open) {
    return
  }

  if (activeTab.value === 'dictionary') {
    await refreshDictionaries()
  }
  if (activeTab.value === 'default' && defaultDictionaryType.value) {
    selectedSourceRef.value = `default:${defaultDictionaryType.value}`
  }
  ensureDefaultSelection()
  await refreshSelectedSourcePreview()
})

watch(selectedSourceRef, async () => {
  await refreshSelectedSourcePreview()
})

function compareSourceItems(
  left: PayloadLibrarySourceItem,
  right: PayloadLibrarySourceItem,
  currentPreferences: { favorites: string[]; recent: string[] },
  recommendedRefs: string[],
): number {
  const score = (source: PayloadLibrarySourceItem) => {
    let value = 0
    if (currentPreferences.favorites.includes(source.ref)) value += 100
    if (recommendedRefs.includes(source.ref)) value += 50
    if (currentPreferences.recent.includes(source.ref)) value += 25 - currentPreferences.recent.indexOf(source.ref)
    if (source.recommended) value += 10
    return value
  }

  return score(right) - score(left) || left.label.localeCompare(right.label)
}

function ensureDefaultSelection() {
  if (visibleSources.value.some((item) => item.ref === selectedSourceRef.value)) {
    return
  }

  selectedSourceRef.value = visibleSources.value[0]?.ref ?? ''
}

async function refreshDictionaries() {
  if (activeTab.value !== 'dictionary') {
    return
  }

  try {
    const page = await listIntruderDictionariesPaged({
      dictType: dictionaryType.value || null,
      searchTerm: searchTerm.value || null,
      offset: (dictionaryPage.value - 1) * DICTIONARY_PAGE_SIZE,
      limit: DICTIONARY_PAGE_SIZE,
    })
    dictionaries.value = page.items
    dictionariesTotal.value = page.total
  } catch (error) {
    console.error('Failed to load payload library dictionaries', error)
    pickerError.value = error instanceof Error ? error.message : 'Failed to load dictionaries'
  }
}

async function refreshDefaultDictionaryMap() {
  try {
    defaultDictionaryMap.value = await getIntruderDefaultDictionaryMap()
  } catch (error) {
    console.error('Failed to load default dictionary map', error)
    pickerError.value = error instanceof Error ? error.message : 'Failed to load default dictionaries'
  }
}

async function refreshSelectedSourcePreview() {
  resolvedPreviewItems.value = []
  selectedDictionarySource.value = null
  selectedDefaultDictionaryId.value = ''

  const source = selectedSource.value
  if (!source) {
    return
  }

  if (source.kind === 'builtIn') {
    resolvedPreviewItems.value = getIntruderBuiltInPayloadListItems(source.ref.replace('builtin:', ''))
    return
  }

  if (source.kind === 'template') {
    return
  }

  try {
    if (source.kind === 'dictionary') {
      const dictionaryId = source.ref.replace('dictionary:', '')
      const dictionary = dictionaries.value.find((item) => item.id === dictionaryId)
      if (!dictionary) {
        return
      }
      selectedDictionarySource.value = {
        dictionaryId,
        dictionaryName: dictionary.name,
        dictType: dictionary.dict_type,
      }
      resolvedPreviewItems.value = await listIntruderDictionaryWords(dictionaryId, importLimit.value)
      return
    }

    const dictType = source.ref.replace('default:', '')
    const dictionaryId = defaultDictionaryMap.value[dictType] || ''
    selectedDefaultDictionaryId.value = dictionaryId
    if (!dictionaryId) {
      resolvedPreviewItems.value = []
      return
    }
    resolvedPreviewItems.value = await listIntruderDictionaryWords(dictionaryId, importLimit.value)
  } catch (error) {
    console.error('Failed to load payload source preview', error)
    pickerError.value = error instanceof Error ? error.message : 'Failed to load payload source'
  }
}

function selectSource(sourceRef: string) {
  selectedSourceRef.value = sourceRef
}

function isFavorite(sourceRef: string): boolean {
  return preferences.value.favorites.includes(sourceRef)
}

function isRecent(sourceRef: string): boolean {
  return preferences.value.recent.includes(sourceRef)
}

function toggleFavorite(sourceRef: string) {
  preferences.value = toggleIntruderPayloadLibraryFavorite(sourceRef)
}

function applyImport() {
  if (!selectedSource.value || selectedSource.value.kind === 'template') {
    return
  }

  preferences.value = markIntruderPayloadLibraryRecent(selectedSource.value.ref)
  emit('import', {
    mode: importMode.value,
    payloads: importedItems.value,
    sourceRef: selectedSource.value.ref,
  })
  emit('close')
}

function applyTemplate() {
  if (!selectedSource.value || selectedSource.value.kind !== 'template') {
    return
  }

  const templateId = selectedSource.value.ref.replace('template:', '')
  const template = INTRUDER_PAYLOAD_TEMPLATES.find((item) => item.id === templateId)
  if (!template) {
    return
  }

  preferences.value = markIntruderPayloadLibraryRecent(selectedSource.value.ref)
  emit('apply-template', {
    sourceRef: selectedSource.value.ref,
    attackType: template.recommendedAttackType,
    sets: selectedTemplateSets.value.map((item) => ({
      name: item.name,
      payloadsText: item.payloadsText,
    })),
  })
  emit('close')
}

function buildNextPayloadsText(payloads: string[]): string {
  if (importMode.value === 'replace') {
    return payloads.join('\n')
  }

  if (importMode.value === 'mergeDeduplicate') {
    return Array.from(new Set([...currentItems.value, ...payloads])).join('\n')
  }

  return [...currentItems.value, ...payloads].join('\n')
}

function getDictionaryTypeLabel(dictType: string): string {
  const translationKey = getIntruderDictionaryTypeTranslationKey(dictType)
  return t(`dictionary.types.${translationKey}`, dictType)
}
</script>
