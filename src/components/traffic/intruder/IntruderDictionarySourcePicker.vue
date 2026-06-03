<template>
  <Teleport to="body">
    <div v-if="open" class="modal modal-open" @click.self="$emit('close')">
      <div class="modal-box max-w-4xl">
        <div class="mb-4 flex items-center justify-between">
          <div>
            <h3 class="text-base font-semibold">
              {{ mode === 'default' ? $t('trafficAnalysis.intruder.labels.selectDefaultDictionary') : $t('trafficAnalysis.intruder.labels.selectDictionary') }}
            </h3>
            <p class="mt-1 text-xs text-base-content/60">
              {{ mode === 'default' ? $t('trafficAnalysis.intruder.help.defaultDictionaryPickerHint') : $t('trafficAnalysis.intruder.help.dictionaryPickerHint') }}
            </p>
          </div>
          <button class="btn btn-ghost btn-xs" type="button" @click="$emit('close')">✕</button>
        </div>

        <div v-if="mode === 'dictionary'" class="grid gap-3 md:grid-cols-[minmax(0,1fr)_15rem]">
          <input
            v-model="searchTerm"
            type="text"
            class="input input-bordered input-sm"
            :placeholder="$t('trafficAnalysis.intruder.placeholders.searchDictionary')"
          />
          <select v-model="dictionaryType" class="select select-bordered select-sm">
            <option value="">{{ $t('trafficAnalysis.intruder.labels.allDictionaryTypes') }}</option>
            <option v-for="dictType in dictionaryTypeOptions" :key="dictType" :value="dictType">
              {{ getDictionaryTypeLabel(dictType) }}
            </option>
          </select>
        </div>

        <div v-else class="grid gap-3 md:grid-cols-[minmax(0,1fr)_auto]">
          <select v-model="defaultDictionaryType" class="select select-bordered select-sm">
            <option value="">{{ $t('trafficAnalysis.intruder.labels.pleaseSelect') }}</option>
            <option v-for="dictType in dictionaryTypeOptions" :key="dictType" :value="dictType">
              {{ getDictionaryTypeLabel(dictType) }}
            </option>
          </select>
          <div class="badge badge-outline self-center">
            {{ activeDefaultDictionaryId ? activeDefaultDictionaryId : $t('trafficAnalysis.intruder.labels.noDefaultDictionaryConfigured') }}
          </div>
        </div>

        <div v-if="pickerError" class="mt-3 rounded-lg border border-error/30 bg-error/10 px-3 py-2 text-sm text-error">
          {{ pickerError }}
        </div>

        <div v-if="mode === 'dictionary'" class="mt-4 grid gap-4 lg:grid-cols-[minmax(0,1.4fr)_minmax(0,1fr)]">
          <div class="rounded-lg border border-base-300">
            <div class="overflow-auto">
              <table class="table table-sm">
                <thead>
                  <tr>
                    <th class="w-12"></th>
                    <th>{{ $t('trafficAnalysis.intruder.labels.dictionary') }}</th>
                    <th class="w-36">{{ $t('trafficAnalysis.intruder.labels.dictionaryType') }}</th>
                    <th class="w-24 text-right">{{ $t('dictionary.wordCount') }}</th>
                  </tr>
                </thead>
                <tbody>
                  <tr v-if="loading">
                    <td colspan="4" class="py-8 text-center text-sm text-base-content/60">
                      <span class="loading loading-spinner loading-sm"></span>
                    </td>
                  </tr>
                  <tr
                    v-for="dictionary in dictionaries"
                    v-else
                    :key="dictionary.id"
                    class="cursor-pointer"
                    :class="[
                      focusedDictionaryId === dictionary.id ? 'bg-primary/10' : '',
                      dictionary.is_active === false ? 'opacity-70' : '',
                    ]"
                    @click="focusDictionary(dictionary.id)"
                  >
                    <td>
                      <input
                        :checked="selectedDictionaryIds.includes(dictionary.id)"
                        type="checkbox"
                        class="checkbox checkbox-sm"
                        @click.stop
                        @change="toggleDictionary(dictionary.id, ($event.target as HTMLInputElement).checked)"
                      />
                    </td>
                    <td>
                      <div class="flex flex-wrap items-center gap-2">
                        <div class="font-medium">{{ dictionary.name }}</div>
                        <span v-if="dictionary.is_active === false" class="badge badge-warning badge-outline badge-xs">
                          {{ $t('common.disabled', '已禁用') }}
                        </span>
                      </div>
                      <div class="text-xs font-mono text-base-content/60">{{ dictionary.id }}</div>
                    </td>
                    <td class="text-xs text-base-content/70">
                      {{ getDictionaryTypeLabel(dictionary.dict_type) }}
                    </td>
                    <td class="text-right text-sm">
                      {{ dictionary.word_count || 0 }}
                    </td>
                  </tr>
                  <tr v-if="!loading && !dictionaries.length">
                    <td colspan="4" class="py-8 text-center text-sm text-base-content/60">
                      {{ $t('trafficAnalysis.intruder.empty.noDictionaries') }}
                    </td>
                  </tr>
                </tbody>
              </table>
            </div>

            <div class="flex items-center justify-between border-t border-base-300 px-4 py-3 text-xs text-base-content/60">
              <span>
                {{ $t('trafficAnalysis.intruder.labels.dictionaryPageSummary', {
                  page,
                  total: totalPages,
                  count: total,
                }) }}
              </span>
              <div class="join">
                <button class="join-item btn btn-xs" type="button" :disabled="page <= 1 || loading" @click="page -= 1">
                  {{ $t('common.previous') }}
                </button>
                <button class="join-item btn btn-xs" type="button" :disabled="page >= totalPages || loading" @click="page += 1">
                  {{ $t('common.next') }}
                </button>
              </div>
            </div>
          </div>

          <div class="rounded-lg border border-base-300 bg-base-100 p-4">
            <div class="text-sm font-medium">{{ $t('trafficAnalysis.intruder.labels.dictionaryWordPreview') }}</div>
            <div v-if="previewLoading" class="mt-3 text-sm text-base-content/60">
              {{ $t('trafficAnalysis.intruder.labels.loadingDictionaryPreview') }}
            </div>
            <div v-else-if="previewError" class="mt-3 text-sm text-error">
              {{ previewError }}
            </div>
            <div v-else-if="!focusedDictionary" class="mt-3 text-sm text-base-content/60">
              {{ $t('trafficAnalysis.intruder.empty.noDictionarySelected') }}
            </div>
            <div v-else-if="!previewWords.length" class="mt-3 text-sm text-base-content/60">
              {{ $t('trafficAnalysis.intruder.empty.noDictionaryPreviewWords') }}
            </div>
            <div v-else class="mt-3 flex flex-wrap gap-2">
              <span
                v-for="(word, index) in previewWords"
                :key="`${focusedDictionary.id}-${index}-${word}`"
                class="badge badge-outline"
              >
                {{ word }}
              </span>
            </div>
          </div>
        </div>

        <div v-else class="mt-4 rounded-lg border border-base-300 bg-base-100 p-4">
          <div class="text-sm font-medium">{{ $t('trafficAnalysis.intruder.labels.dictionaryWordPreview') }}</div>
          <div v-if="previewLoading" class="mt-3 text-sm text-base-content/60">
            {{ $t('trafficAnalysis.intruder.labels.loadingDictionaryPreview') }}
          </div>
          <div v-else-if="previewError" class="mt-3 text-sm text-error">
            {{ previewError }}
          </div>
          <div v-else-if="!defaultDictionaryType" class="mt-3 text-sm text-base-content/60">
            {{ $t('trafficAnalysis.intruder.labels.selectDictionaryTypeFirst') }}
          </div>
          <div v-else-if="!activeDefaultDictionaryId" class="mt-3 text-sm text-base-content/60">
            {{ $t('trafficAnalysis.intruder.labels.noDefaultDictionaryConfigured') }}
          </div>
          <div v-else-if="!previewWords.length" class="mt-3 text-sm text-base-content/60">
            {{ $t('trafficAnalysis.intruder.empty.noDictionaryPreviewWords') }}
          </div>
          <div v-else class="mt-3 flex flex-wrap gap-2">
            <span
              v-for="(word, index) in previewWords"
              :key="`${defaultDictionaryType}-${index}-${word}`"
              class="badge badge-outline"
            >
              {{ word }}
            </span>
          </div>
        </div>

        <div class="modal-action">
          <button
            class="btn btn-primary btn-sm"
            type="button"
            :disabled="mode === 'dictionary' ? selectedDictionaryIds.length === 0 : !defaultDictionaryType || !activeDefaultDictionaryId"
            @click="applySelection"
          >
            {{ $t('trafficAnalysis.intruder.actions.add') }}
          </button>
          <button class="btn btn-outline btn-sm" type="button" @click="$emit('close')">
            {{ $t('common.cancel', '取消') }}
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
  getIntruderDefaultDictionaryMap,
  getIntruderDictionaryTypeTranslationKey,
  INTRUDER_DICTIONARY_TYPE_OPTIONS,
  listIntruderDictionariesPaged,
  listIntruderDictionaryWords,
  type IntruderDictionarySummary,
} from './intruderDictionaries'
import type { IntruderDictionarySource } from './types'

const PREVIEW_LIMIT = 12
const PAGE_SIZE = 20

const props = defineProps<{
  open: boolean
  mode: 'dictionary' | 'default'
}>()

const emit = defineEmits<{
  (e: 'close'): void
  (e: 'append', value: IntruderDictionarySource[]): void
}>()

const { t } = useI18n()

const loading = ref(false)
const pickerError = ref('')
const dictionaries = ref<IntruderDictionarySummary[]>([])
const focusedDictionaryId = ref('')
const selectedDictionaryIds = ref<string[]>([])
const searchTerm = ref('')
const dictionaryType = ref('')
const page = ref(1)
const total = ref(0)
const defaultDictionaryType = ref('')
const defaultDictionaryMap = ref<Record<string, string>>({})
const previewLoading = ref(false)
const previewError = ref('')
const previewWords = ref<string[]>([])
let previewRequestToken = 0

const dictionaryTypeOptions = [...INTRUDER_DICTIONARY_TYPE_OPTIONS]
const totalPages = computed(() => Math.max(1, Math.ceil(total.value / PAGE_SIZE)))
const focusedDictionary = computed(() =>
  dictionaries.value.find((item) => item.id === focusedDictionaryId.value) || null,
)
const activeDefaultDictionaryId = computed(() =>
  defaultDictionaryType.value ? defaultDictionaryMap.value[defaultDictionaryType.value] || '' : '',
)

watch(
  () => props.open,
  async (open) => {
    if (!open) {
      return
    }

    resetState()
    await loadDefaultDictionaryMap()
    if (props.mode === 'dictionary') {
      await refreshDictionaries()
    }
  },
)

watch([searchTerm, dictionaryType], async () => {
  if (!props.open || props.mode !== 'dictionary') {
    return
  }

  page.value = 1
  await refreshDictionaries()
})

watch(page, async (value, previousValue) => {
  if (!props.open || props.mode !== 'dictionary' || value === previousValue) {
    return
  }

  await refreshDictionaries()
})

watch(focusedDictionaryId, async (value, previousValue) => {
  if (!props.open || props.mode !== 'dictionary' || value === previousValue) {
    return
  }

  await loadDictionaryPreview(value)
})

watch(defaultDictionaryType, async (value, previousValue) => {
  if (!props.open || props.mode !== 'default' || value === previousValue) {
    return
  }

  await loadDefaultDictionaryPreview()
})

function resetState() {
  loading.value = false
  pickerError.value = ''
  dictionaries.value = []
  focusedDictionaryId.value = ''
  selectedDictionaryIds.value = []
  searchTerm.value = ''
  dictionaryType.value = ''
  page.value = 1
  total.value = 0
  defaultDictionaryType.value = ''
  previewLoading.value = false
  previewError.value = ''
  previewWords.value = []
}

async function loadDefaultDictionaryMap() {
  try {
    defaultDictionaryMap.value = await getIntruderDefaultDictionaryMap()
  } catch (error) {
    console.error('Failed to load Intruder default dictionary map', error)
    defaultDictionaryMap.value = {}
  }
}

async function refreshDictionaries() {
  loading.value = true
  pickerError.value = ''

  try {
    const pageResult = await listIntruderDictionariesPaged({
      dictType: dictionaryType.value || null,
      searchTerm: searchTerm.value || null,
      offset: (page.value - 1) * PAGE_SIZE,
      limit: PAGE_SIZE,
    })
    dictionaries.value = pageResult.items
    total.value = pageResult.total
    if (!dictionaries.value.some((item) => item.id === focusedDictionaryId.value)) {
      focusedDictionaryId.value = dictionaries.value[0]?.id || ''
    }
    if (!focusedDictionaryId.value) {
      previewWords.value = []
      previewError.value = ''
    }
  } catch (error) {
    console.error('Failed to load dictionaries for Intruder picker', error)
    pickerError.value = error instanceof Error ? error.message : 'Failed to load dictionaries'
    dictionaries.value = []
    total.value = 0
  } finally {
    loading.value = false
  }
}

function focusDictionary(dictionaryId: string) {
  focusedDictionaryId.value = dictionaryId
}

function toggleDictionary(dictionaryId: string, checked: boolean) {
  if (checked) {
    if (!selectedDictionaryIds.value.includes(dictionaryId)) {
      selectedDictionaryIds.value = [...selectedDictionaryIds.value, dictionaryId]
    }
    focusedDictionaryId.value = dictionaryId
    return
  }

  selectedDictionaryIds.value = selectedDictionaryIds.value.filter((item) => item !== dictionaryId)
}

async function loadDictionaryPreview(dictionaryId: string) {
  previewLoading.value = true
  previewError.value = ''
  const currentToken = ++previewRequestToken

  try {
    previewWords.value = dictionaryId
      ? await listIntruderDictionaryWords(dictionaryId, PREVIEW_LIMIT)
      : []
  } catch (error) {
    console.error('Failed to load dictionary preview words', error)
    if (currentToken !== previewRequestToken) {
      return
    }
    previewWords.value = []
    previewError.value = error instanceof Error ? error.message : 'Failed to load dictionary preview'
  } finally {
    if (currentToken === previewRequestToken) {
      previewLoading.value = false
    }
  }
}

async function loadDefaultDictionaryPreview() {
  previewLoading.value = true
  previewError.value = ''
  const currentToken = ++previewRequestToken

  try {
    previewWords.value = activeDefaultDictionaryId.value
      ? await listIntruderDictionaryWords(activeDefaultDictionaryId.value, PREVIEW_LIMIT)
      : []
  } catch (error) {
    console.error('Failed to load default dictionary preview words', error)
    if (currentToken !== previewRequestToken) {
      return
    }
    previewWords.value = []
    previewError.value = error instanceof Error ? error.message : 'Failed to load default dictionary preview'
  } finally {
    if (currentToken === previewRequestToken) {
      previewLoading.value = false
    }
  }
}

function applySelection() {
  if (props.mode === 'default') {
    const dictType = defaultDictionaryType.value.trim()
    if (!dictType || !activeDefaultDictionaryId.value) {
      return
    }

    emit('append', [{
      type: 'default_dictionary',
      dictType,
    }])
    emit('close')
    return
  }

  const selectedSources = selectedDictionaryIds.value.map((dictionaryId) => {
    const dictionary = dictionaries.value.find((item) => item.id === dictionaryId)
    return {
      type: 'dictionary' as const,
      dictionaryId,
      dictionaryName: dictionary?.name,
    }
  })
  emit('append', selectedSources)
  emit('close')
}

function getDictionaryTypeLabel(dictType: string): string {
  const translationKey = getIntruderDictionaryTypeTranslationKey(dictType)
  return t(`dictionary.types.${translationKey}`, dictType)
}
</script>
