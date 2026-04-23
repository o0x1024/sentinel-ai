<template>
  <div class="rounded-lg border border-base-300/70 bg-base-200/60 p-3 space-y-3">
    <div class="text-xs font-semibold text-base-content/80">
      {{ t('bugBounty.monitor.subdomainBruteDictionarySection') }}
    </div>

    <div class="form-control">
      <label class="label py-1">
        <span class="label-text-alt">{{
          t('bugBounty.monitor.subdomainBruteDictionarySource')
        }}</span>
      </label>
      <select
        v-model="selectedDictionaryId"
        class="select select-sm select-bordered"
        :disabled="loading"
      >
        <option value="">{{ t('bugBounty.monitor.subdomainBruteFollowDefaultDictionary') }}</option>
        <option
          v-for="dictionary in orderedDictionaries"
          :key="dictionary.id"
          :value="dictionary.id"
        >
          {{ formatDictionaryLabel(dictionary) }}
        </option>
      </select>
      <label class="label py-1">
        <span class="label-text-alt text-base-content/60">{{ defaultDictionaryHint }}</span>
      </label>
    </div>
    <div class="form-control">
      <label class="label py-1">
        <span class="label-text-alt">{{
          t('bugBounty.monitor.subdomainBruteInlineDictionary')
        }}</span>
      </label>
      <textarea
        v-model="inlineDictionaryText"
        class="textarea textarea-bordered textarea-sm min-h-28"
        spellcheck="false"
        :placeholder="t('bugBounty.monitor.subdomainBruteInlineDictionaryPlaceholder')"
      />
      <label class="label py-1">
        <span class="label-text-alt text-base-content/60">
          {{ t('bugBounty.monitor.subdomainBruteInlineDictionaryHint') }}
        </span>
      </label>
    </div>

    <div v-if="loadError" class="text-xs text-warning">
      {{ loadError }}
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { getDefaultId } from '../../services/dictionary'
import {
  listMonitorSubdomainDictionaries,
  normalizeSubdomainBruteInlineWords,
  type MonitorSubdomainDictionarySummary,
} from './monitorSubdomainBruteSupport'

const props = defineProps<{
  plugin: Record<string, any>
}>()

const { t } = useI18n()

const dictionaries = ref<MonitorSubdomainDictionarySummary[]>([])
const defaultDictionaryId = ref<string | null>(null)
const loading = ref(false)
const loadError = ref('')

const ensurePluginParams = () => {
  if (!props.plugin.plugin_params || typeof props.plugin.plugin_params !== 'object') {
    props.plugin.plugin_params = {}
  }

  return props.plugin.plugin_params as Record<string, unknown>
}

const selectedDictionaryId = computed({
  get: () => String(props.plugin?.plugin_params?.dictionary_id || '').trim(),
  set: (value: string) => {
    const params = ensurePluginParams()
    const trimmed = String(value || '').trim()
    if (trimmed) {
      params.dictionary_id = trimmed
      return
    }
    delete params.dictionary_id
  },
})

const inlineDictionaryText = computed({
  get: () => normalizeSubdomainBruteInlineWords(props.plugin?.plugin_params?.dictionary).join('\n'),
  set: (value: string) => {
    const params = ensurePluginParams()
    const words = normalizeSubdomainBruteInlineWords(value)
    if (words.length > 0) {
      params.dictionary = words
      return
    }
    delete params.dictionary
  },
})

const resolvedDefaultDictionary = computed(
  () => dictionaries.value.find(dictionary => dictionary.id === defaultDictionaryId.value) ?? null
)

const orderedDictionaries = computed(() => {
  if (!defaultDictionaryId.value) {
    return dictionaries.value
  }

  return [...dictionaries.value].sort((left, right) => {
    const leftIsDefault = left.id === defaultDictionaryId.value
    const rightIsDefault = right.id === defaultDictionaryId.value
    if (leftIsDefault === rightIsDefault) {
      return left.name.localeCompare(right.name)
    }
    return leftIsDefault ? -1 : 1
  })
})

const defaultDictionaryHint = computed(() => {
  if (defaultDictionaryId.value) {
    return t('bugBounty.monitor.subdomainBruteDictionaryPickerHint', {
      name: resolvedDefaultDictionary.value?.name || defaultDictionaryId.value,
    })
  }

  return t('bugBounty.monitor.subdomainBruteDictionaryPickerHintNoDefault')
})

const formatDictionaryLabel = (dictionary: MonitorSubdomainDictionarySummary) => {
  const count = Math.max(0, Number(dictionary.word_count) || 0)
  const defaultSuffix =
    dictionary.id === defaultDictionaryId.value
      ? ` · ${t('bugBounty.monitor.subdomainBruteDefaultDictionaryTag')}`
      : ''
  return `${dictionary.name} (${count})${defaultSuffix}`
}

const loadDictionaryOptions = async () => {
  loading.value = true
  loadError.value = ''

  try {
    const [items, defaultId] = await Promise.all([
      listMonitorSubdomainDictionaries(),
      getDefaultId('subdomain'),
    ])

    dictionaries.value = items
    defaultDictionaryId.value = defaultId

    const params = ensurePluginParams()
    delete params.dictionary_limit
  } catch (error) {
    dictionaries.value = []
    defaultDictionaryId.value = null
    loadError.value =
      error instanceof Error
        ? error.message
        : t('bugBounty.monitor.subdomainBruteDictionaryLoadFailed')
  } finally {
    loading.value = false
  }
}

onMounted(async () => {
  await loadDictionaryOptions()
})
</script>
