<template>
  <div class="grid gap-4 p-4">
    <div class="flex flex-wrap gap-2">
      <button class="btn btn-sm btn-outline" type="button" @click="pickerMode = 'dictionary'">
        {{ $t('trafficAnalysis.intruder.actions.addDictionarySource') }}
      </button>
      <button class="btn btn-sm btn-outline" type="button" @click="pickerMode = 'default'">
        {{ $t('trafficAnalysis.intruder.actions.addDefaultDictionarySource') }}
      </button>
      <button
        class="btn btn-sm btn-ghost"
        type="button"
        :disabled="!normalizedDictionaryConfig.sources.length || previewLoading"
        @click="refreshPreview"
      >
        <span v-if="previewLoading" class="loading loading-spinner loading-xs"></span>
        {{ $t('trafficAnalysis.intruder.actions.refreshDictionaryPreview') }}
      </button>
      <button
        class="btn btn-sm btn-ghost"
        type="button"
        :disabled="!normalizedDictionaryConfig.sources.length"
        @click="clearSources"
      >
        {{ $t('trafficAnalysis.intruder.actions.clearDictionarySources') }}
      </button>
    </div>

    <div class="flex flex-wrap gap-2 rounded-lg border border-base-300 bg-base-100 p-3 min-h-16">
      <span
        v-for="(source, index) in normalizedDictionaryConfig.sources"
        :key="`${source.type}-${source.dictionaryId || source.dictType || index}`"
        class="badge badge-primary badge-outline gap-2 py-3"
      >
        <span class="max-w-64 truncate">{{ formatSourceLabel(source) }}</span>
        <button
          class="btn btn-ghost btn-xs min-h-0 h-4 w-4 p-0"
          type="button"
          @click="removeSource(index)"
        >
          ×
        </button>
      </span>

      <span v-if="!normalizedDictionaryConfig.sources.length" class="text-xs text-base-content/50">
        {{ $t('trafficAnalysis.intruder.empty.noDictionarySources') }}
      </span>
    </div>

    <div class="grid gap-3 md:grid-cols-2">
      <label class="form-control">
        <span class="label-text text-xs">{{ $t('trafficAnalysis.intruder.labels.dictionaryPayloadLimit') }}</span>
        <input
          :value="normalizedDictionaryConfig.limit"
          type="number"
          min="1"
          max="50000"
          class="input input-bordered input-sm"
          @input="updateLimit(($event.target as HTMLInputElement).value)"
        />
      </label>

      <label class="flex items-center gap-2 pt-7">
        <input
          :checked="normalizedDictionaryConfig.deduplicate"
          type="checkbox"
          class="checkbox checkbox-sm"
          @change="updateDeduplicate(($event.target as HTMLInputElement).checked)"
        />
        <span>{{ $t('trafficAnalysis.intruder.labels.deduplicateDictionaryPayloads') }}</span>
      </label>
    </div>

    <div class="grid grid-cols-2 gap-3 text-xs text-base-content/70">
      <div>
        <div>{{ $t('trafficAnalysis.intruder.labels.payloadCount') }}</div>
        <div class="mt-1 font-semibold text-base-content">{{ cachedPayloadCount }}</div>
      </div>
      <div>
        <div>{{ $t('trafficAnalysis.intruder.labels.dictionarySourceCount') }}</div>
        <div class="mt-1 font-semibold text-base-content">{{ normalizedDictionaryConfig.sources.length }}</div>
      </div>
    </div>

    <div class="rounded-lg border border-base-300 bg-base-100 p-4">
      <div class="flex items-center justify-between gap-3">
        <div>
          <div class="text-sm font-medium">{{ $t('trafficAnalysis.intruder.labels.dictionaryWordPreview') }}</div>
          <div class="mt-1 text-xs text-base-content/60">
            {{ $t('trafficAnalysis.intruder.help.dictionaryPayloadPreviewHint') }}
          </div>
        </div>
        <div class="badge badge-outline">
          {{ $t('trafficAnalysis.intruder.labels.cachedPayloads') }}: {{ cachedPayloadCount }}
        </div>
      </div>

      <div v-if="previewError" class="mt-3 rounded-lg border border-error/30 bg-error/10 px-3 py-2 text-sm text-error">
        {{ previewError }}
      </div>
      <div v-else-if="previewLoading" class="mt-3 text-sm text-base-content/60">
        {{ $t('trafficAnalysis.intruder.labels.loadingDictionaryPreview') }}
      </div>
      <div v-else-if="!previewWords.length" class="mt-3 text-sm text-base-content/60">
        {{ normalizedDictionaryConfig.sources.length ? $t('trafficAnalysis.intruder.empty.noDictionaryPreviewWords') : $t('trafficAnalysis.intruder.empty.noDictionarySources') }}
      </div>
      <div v-else class="mt-3 flex flex-wrap gap-2">
        <span
          v-for="(word, index) in previewWords.slice(0, 24)"
          :key="`${index}-${word}`"
          class="badge badge-outline"
        >
          {{ word }}
        </span>
      </div>
    </div>

    <IntruderDictionarySourcePicker
      :open="pickerMode !== null"
      :mode="pickerMode || 'dictionary'"
      @append="appendSources"
      @close="pickerMode = null"
    />
  </div>
</template>

<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import IntruderDictionarySourcePicker from './IntruderDictionarySourcePicker.vue'
import { parsePayloadLines } from './payloads'
import {
  deduplicateIntruderDictionarySources,
  normalizeIntruderDictionaryPayloadConfig,
  resolveIntruderDictionaryPayloads,
} from './intruderAppDictionaryPayloads'
import { getIntruderDictionaryTypeTranslationKey } from './intruderDictionaries'
import type { IntruderDictionarySource, IntruderPayloadSet } from './types'

const props = defineProps<{
  payloadSet: IntruderPayloadSet
}>()

const emit = defineEmits<{
  (e: 'update', value: Partial<IntruderPayloadSet>): void
}>()

const { t } = useI18n()

const pickerMode = ref<'dictionary' | 'default' | null>(null)
const previewLoading = ref(false)
const previewError = ref('')
const previewWords = ref<string[]>([])
let previewRequestToken = 0

const normalizedDictionaryConfig = computed(() =>
  normalizeIntruderDictionaryPayloadConfig(props.payloadSet.dictionaryConfig),
)
const cachedPayloadCount = computed(() => parsePayloadLines(props.payloadSet.payloadsText).length)

watch(
  normalizedDictionaryConfig,
  async () => {
    await refreshPreview()
  },
  { immediate: true, deep: true },
)

async function refreshPreview() {
  previewError.value = ''
  const currentToken = ++previewRequestToken
  const config = normalizedDictionaryConfig.value

  if (!config.sources.length) {
    previewWords.value = []
    emitCachedPayloads([])
    previewLoading.value = false
    return
  }

  previewLoading.value = true
  try {
    const payloads = await resolveIntruderDictionaryPayloads(config)
    if (currentToken !== previewRequestToken) {
      return
    }
    previewWords.value = payloads
    emitCachedPayloads(payloads)
  } catch (error) {
    console.error('Failed to resolve Intruder app dictionary payloads', error)
    if (currentToken !== previewRequestToken) {
      return
    }
    previewWords.value = []
    previewError.value = error instanceof Error ? error.message : 'Failed to load dictionary payloads'
    emitCachedPayloads([])
  } finally {
    if (currentToken === previewRequestToken) {
      previewLoading.value = false
    }
  }
}

function emitCachedPayloads(payloads: string[]) {
  const nextPayloadsText = payloads.join('\n')
  if (props.payloadSet.payloadsText === nextPayloadsText) {
    return
  }

  emit('update', {
    payloadsText: nextPayloadsText,
  })
}

function appendSources(sources: IntruderDictionarySource[]) {
  const nextSources = deduplicateIntruderDictionarySources([
    ...normalizedDictionaryConfig.value.sources,
    ...sources,
  ])
  emitDictionaryConfig({
    ...normalizedDictionaryConfig.value,
    sources: nextSources,
  })
  pickerMode.value = null
}

function removeSource(index: number) {
  emitDictionaryConfig({
    ...normalizedDictionaryConfig.value,
    sources: normalizedDictionaryConfig.value.sources.filter((_, itemIndex) => itemIndex !== index),
  })
}

function clearSources() {
  emitDictionaryConfig({
    ...normalizedDictionaryConfig.value,
    sources: [],
  })
}

function updateLimit(rawValue: string) {
  emitDictionaryConfig({
    ...normalizedDictionaryConfig.value,
    limit: Math.max(1, Number(rawValue) || 1),
  })
}

function updateDeduplicate(deduplicate: boolean) {
  emitDictionaryConfig({
    ...normalizedDictionaryConfig.value,
    deduplicate,
  })
}

function emitDictionaryConfig(value: IntruderPayloadSet['dictionaryConfig']) {
  emit('update', {
    dictionaryConfig: normalizeIntruderDictionaryPayloadConfig(value),
  })
}

function formatSourceLabel(source: IntruderDictionarySource): string {
  if (source.type === 'default_dictionary') {
    return t('trafficAnalysis.intruder.labels.defaultDictionaryLabel', {
      type: getDictionaryTypeLabel(source.dictType || ''),
    })
  }

  return source.dictionaryName
    ? `${source.dictionaryName} (${source.dictionaryId || ''})`
    : source.dictionaryId || ''
}

function getDictionaryTypeLabel(dictType: string): string {
  const translationKey = getIntruderDictionaryTypeTranslationKey(dictType)
  return t(`dictionary.types.${translationKey}`, dictType)
}
</script>
