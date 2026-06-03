<template>
  <div
    v-if="!showOnlyChanged || hasCustomizedField(params, field)"
    class="form-control"
    :class="depth > 0 ? 'rounded-lg border border-base-300/60 bg-base-100/70 p-3' : ''"
  >
    <div class="flex items-start justify-between gap-2">
      <label class="label py-1 min-w-0">
        <span class="label-text-alt">
          {{ field.name }}
          <span v-if="field.required" class="text-error">*</span>
        </span>
        <span v-if="field.typeLabel" class="label-text-alt text-base-content/50 ml-2">
          {{ field.typeLabel }}
        </span>
      </label>
      <button
        v-if="hasExplicitValueAtPath(params, field.path)"
        type="button"
        class="btn btn-ghost btn-xs text-primary"
        @click="resetField"
      >
        {{ t('bugBounty.monitor.resetToDefault') }}
      </button>
    </div>

    <select
      v-if="field.control === 'enum'"
      :value="getEnumIndex()"
      class="select select-sm select-bordered"
      @change="setEnumValue(($event.target as HTMLSelectElement).value)"
    >
      <option value="">{{ t('bugBounty.monitor.usePluginDefaultParamValue') }}</option>
      <option
        v-for="(option, index) in field.enumValues"
        :key="`${pathKey}-${index}`"
        :value="String(index)"
      >
        {{ option }}
      </option>
    </select>

    <div
      v-else-if="field.control === 'string'"
      class="flex items-center gap-2"
    >
      <input
        :value="getStringValue()"
        :type="field.secret && !secretVisible ? 'password' : 'text'"
        class="input input-sm input-bordered flex-1 min-w-0"
        :placeholder="getStringPlaceholder()"
        @input="setStringValue(($event.target as HTMLInputElement).value)"
      />
      <button
        v-if="field.secret"
        type="button"
        class="btn btn-ghost btn-xs"
        @click="secretVisible = !secretVisible"
      >
        {{ secretVisible ? t('bugBounty.monitor.hideSecretValue') : t('bugBounty.monitor.showSecretValue') }}
      </button>
    </div>

    <input
      v-else-if="field.control === 'number'"
      :value="getNumberTextValue()"
      :type="field.type === 'integer' ? 'number' : 'text'"
      class="input input-sm input-bordered"
      :placeholder="getNumberPlaceholder()"
      @input="setNumberValue(($event.target as HTMLInputElement).value)"
    />

    <label
      v-else-if="field.control === 'boolean'"
      class="label cursor-pointer justify-start gap-3 rounded border border-base-300 bg-base-100 px-3 py-2"
    >
      <input
        :checked="getBooleanValue()"
        type="checkbox"
        class="checkbox checkbox-sm checkbox-primary"
        @change="setBooleanValue(($event.target as HTMLInputElement).checked)"
      />
      <span class="text-sm">{{ t('bugBounty.monitor.enableBooleanParam') }}</span>
    </label>

    <div v-else-if="field.control === 'array-multiselect'" class="space-y-2 rounded-lg border border-base-300/70 bg-base-100/70 p-3">
      <div class="flex items-center justify-between gap-2">
        <div class="text-xs text-base-content/60">
          {{ t('bugBounty.monitor.multiSelectHint') }}
        </div>
        <div class="flex items-center gap-2">
          <button type="button" class="btn btn-ghost btn-xs" @click="selectAllSuggestedOptions">
            {{ t('bugBounty.monitor.selectAllOptions') }}
          </button>
          <button type="button" class="btn btn-ghost btn-xs" @click="clearArrayValue">
            {{ t('common.clear') }}
          </button>
        </div>
      </div>
      <div class="grid gap-2 sm:grid-cols-2">
        <label
          v-for="option in field.arraySuggestedOptions"
          :key="`${pathKey}-${option}`"
          class="label cursor-pointer justify-start gap-3 rounded border border-base-300 bg-base-100 px-3 py-2"
        >
          <input
            :checked="getArraySelection().includes(option)"
            type="checkbox"
            class="checkbox checkbox-sm checkbox-primary"
            @change="toggleArrayOption(option, ($event.target as HTMLInputElement).checked)"
          />
          <span class="text-sm break-all">{{ option }}</span>
        </label>
      </div>
    </div>

    <textarea
      v-else-if="field.control === 'array-lines'"
      :value="getArrayLinesValue()"
      class="textarea textarea-bordered textarea-sm min-h-24 font-mono"
      spellcheck="false"
      :placeholder="getArrayPlaceholder()"
      @input="setArrayLinesValue(($event.target as HTMLTextAreaElement).value)"
    />

    <div
      v-else-if="field.control === 'object-fields'"
      class="space-y-3 rounded-lg border border-base-300/70 bg-base-100/70 p-3"
    >
      <div class="text-xs text-base-content/60">
        {{ t('bugBounty.monitor.structuredObjectHint') }}
      </div>
      <MonitorPluginParamsField
        v-for="childField in field.objectFields"
        :key="getFieldPathKey(childField.path)"
        :field="childField"
        :params="params"
        :default-params="defaultParams"
        :json-editor-values="jsonEditorValues"
        :field-errors="fieldErrors"
        :expanded-hint-fields="expandedHintFields"
        :show-only-changed="showOnlyChanged"
        :depth="depth + 1"
      />
    </div>

    <div
      v-else-if="field.control === 'dictionary-picker'"
      class="space-y-2"
    >
      <div class="flex items-center gap-2">
        <select
          v-if="!dictManualMode"
          :value="getExplicitValue() ?? ''"
          class="select select-sm select-bordered flex-1 min-w-0"
          @change="setDictionaryValue(($event.target as HTMLSelectElement).value)"
        >
          <option value="">{{ t('bugBounty.monitor.usePluginDefaultParamValue', '使用插件默认值') }}</option>
          <option
            v-for="dict in filteredDictionaries"
            :key="dict.id"
            :value="dict.id"
          >
            {{ dict.name }} ({{ dict.dict_type }}) [{{ dict.word_count }}]
          </option>
        </select>
        <input
          v-else
          :value="getStringValue()"
          type="text"
          class="input input-sm input-bordered flex-1 min-w-0"
          :placeholder="t('bugBounty.monitor.dictionaryIdPlaceholder', '输入字典 ID 或名称')"
          @input="setStringValue(($event.target as HTMLInputElement).value)"
        />
        <button
          type="button"
          class="btn btn-ghost btn-xs whitespace-nowrap"
          @click="dictManualMode = !dictManualMode"
        >
          {{ dictManualMode ? t('bugBounty.monitor.dictionaryPickerMode', '选择') : t('bugBounty.monitor.dictionaryManualMode', '手动输入') }}
        </button>
      </div>
      <div v-if="dictLoading" class="text-xs text-base-content/50">
        {{ t('bugBounty.monitor.loadingDictionaries', '加载字典列表...') }}
      </div>
      <div v-if="dictLoadError" class="text-xs text-warning">
        {{ dictLoadError }}
      </div>
      <div v-if="selectedDictionaryInfo && !dictManualMode" class="text-xs text-base-content/60">
        {{ selectedDictionaryInfo }}
      </div>
    </div>

    <textarea
      v-else
      :value="getJsonEditorText()"
      class="textarea textarea-bordered textarea-sm min-h-28 font-mono"
      spellcheck="false"
      :placeholder="getJsonPlaceholder()"
      @input="setJsonEditorText(($event.target as HTMLTextAreaElement).value)"
    />

    <label class="label py-1">
      <span class="label-text-alt text-base-content/60 w-full whitespace-pre-wrap break-all leading-5">
        {{ displayedHint }}
      </span>
    </label>
    <div v-if="longHint" class="mt-1">
      <button
        type="button"
        class="btn btn-ghost btn-xs h-auto min-h-0 px-0 text-primary"
        @click="toggleHintExpanded"
      >
        {{ hintExpanded ? t('common.collapse') : t('common.expand') }}
      </button>
    </div>

    <div v-if="fieldErrors[pathKey]" class="text-xs text-warning">
      {{ fieldErrors[pathKey] }}
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { invoke } from '@tauri-apps/api/core'
import {
  buildFieldHint,
  cloneValue,
  deleteValueAtPath,
  getCollapsedHint,
  getFieldPathKey,
  getValueAtPath,
  hasCustomizedField,
  hasExplicitValueAtPath,
  isLongHint,
  setValueAtPath,
  type EditableField,
} from './monitorPluginParamsSupport'

type DictionaryItem = {
  id: string
  name: string
  dict_type: string
  word_count: number
  is_active: boolean
  description?: string
}

defineOptions({ name: 'MonitorPluginParamsField' })

const props = withDefaults(
  defineProps<{
    field: EditableField
    params: Record<string, any>
    defaultParams?: Record<string, any>
    jsonEditorValues: Record<string, string>
    fieldErrors: Record<string, string>
    expandedHintFields: Record<string, boolean>
    showOnlyChanged?: boolean
    depth?: number
  }>(),
  {
    defaultParams: () => ({}),
    showOnlyChanged: false,
    depth: 0,
  }
)

const { t } = useI18n()
const secretVisible = ref(false)

const dictManualMode = ref(false)
const dictLoading = ref(false)
const dictLoadError = ref('')
const allDictionaries = ref<DictionaryItem[]>([])

const filteredDictionaries = computed(() => {
  const dictType = props.field.dictionaryType
  const list = allDictionaries.value.filter(d => d.is_active !== false)
  if (!dictType) return list
  return list.filter(d => d.dict_type === dictType)
})

const selectedDictionaryInfo = computed(() => {
  const currentId = getExplicitValue()
  if (!currentId || typeof currentId !== 'string') return ''
  const dict = allDictionaries.value.find(d => d.id === currentId)
  if (!dict) return ''
  const desc = dict.description ? ` — ${dict.description}` : ''
  return `${dict.name} (${dict.dict_type}, ${dict.word_count} ${t('dictionary.wordCount', '词条')})${desc}`
})

const setDictionaryValue = (value: string) => {
  if (!value) {
    resetField()
    return
  }
  delete props.fieldErrors[pathKey.value]
  setValueAtPath(props.params, props.field.path, value)
}

const loadDictionaries = async () => {
  if (allDictionaries.value.length > 0) return
  dictLoading.value = true
  dictLoadError.value = ''
  try {
    const result = await invoke<DictionaryItem[]>('get_dictionaries', {
      dict_type: props.field.dictionaryType || null,
      service_type: null,
      category: null,
      is_builtin: null,
      is_active: null,
      search_term: null,
    })
    allDictionaries.value = result
  } catch (err) {
    dictLoadError.value = err instanceof Error ? err.message : String(err)
  } finally {
    dictLoading.value = false
  }
}

watch(
  () => props.field.control,
  (control) => {
    if (control === 'dictionary-picker') {
      loadDictionaries()
    }
  },
  { immediate: true },
)

const pathKey = computed(() => getFieldPathKey(props.field.path))
const fullHint = computed(() => buildFieldHint(props.field, t))
const longHint = computed(() => isLongHint(fullHint.value))
const hintExpanded = computed(() => Boolean(props.expandedHintFields[pathKey.value]))
const displayedHint = computed(() => {
  if (!longHint.value || hintExpanded.value) {
    return fullHint.value
  }
  return getCollapsedHint(fullHint.value)
})

const getInheritedDefaultValue = () => {
  const pluginDefaultValue = getValueAtPath(props.defaultParams, props.field.path)
  return pluginDefaultValue === undefined ? props.field.defaultValue : pluginDefaultValue
}

const hasPluginDefaultValue = () =>
  getValueAtPath(props.defaultParams, props.field.path) !== undefined

const formatDefaultPlaceholderValue = (value: unknown) => {
  if (value === undefined || value === null) {
    return ''
  }

  if (props.field.secret) {
    return '********'
  }

  if (typeof value === 'string' || typeof value === 'number' || typeof value === 'boolean') {
    return String(value)
  }

  try {
    return JSON.stringify(value)
  } catch {
    return String(value)
  }
}

const getExplicitValue = () => getValueAtPath(props.params, props.field.path)

const getEffectiveValue = () => {
  const explicitValue = getExplicitValue()
  return explicitValue === undefined ? getInheritedDefaultValue() : explicitValue
}

const resetField = () => {
  deleteValueAtPath(props.params, props.field.path)
  delete props.fieldErrors[pathKey.value]
  delete props.jsonEditorValues[pathKey.value]
}

const toggleHintExpanded = () => {
  props.expandedHintFields[pathKey.value] = !props.expandedHintFields[pathKey.value]
}

const getStringValue = () => {
  const value = getExplicitValue()
  return value === undefined || value === null ? '' : String(value)
}

const setStringValue = (value: string) => {
  if (value === '') {
    resetField()
    return
  }
  delete props.fieldErrors[pathKey.value]
  setValueAtPath(props.params, props.field.path, value)
}

const getStringPlaceholder = () =>
  (() => {
    const inheritedDefaultValue = getInheritedDefaultValue()
    if (inheritedDefaultValue === undefined) {
      return t('bugBounty.monitor.usePluginDefaultParamValue')
    }

    const placeholder = formatDefaultPlaceholderValue(inheritedDefaultValue)
    if (placeholder.trim()) {
      return placeholder
    }

    return props.field.secret ? '********' : t('bugBounty.monitor.usePluginDefaultParamValue')
  })()

const getEnumIndex = () => {
  const currentValue = getEffectiveValue()
  if (currentValue === undefined) {
    return ''
  }

  const matchedIndex = props.field.enumValues.findIndex(
    option => JSON.stringify(option) === JSON.stringify(currentValue)
  )
  return matchedIndex >= 0 ? String(matchedIndex) : ''
}

const setEnumValue = (rawValue: string) => {
  if (rawValue === '') {
    resetField()
    return
  }

  const index = Number.parseInt(rawValue, 10)
  if (Number.isNaN(index) || index < 0 || index >= props.field.enumValues.length) {
    return
  }

  delete props.fieldErrors[pathKey.value]
  setValueAtPath(props.params, props.field.path, cloneValue(props.field.enumValues[index]))
}

const getNumberTextValue = () => {
  const value = getExplicitValue()
  return value === undefined || value === null ? '' : String(value)
}

const getNumberPlaceholder = () =>
  (() => {
    const inheritedDefaultValue = getInheritedDefaultValue()
    if (inheritedDefaultValue === undefined) {
      return t('bugBounty.monitor.usePluginDefaultParamValue')
    }

    const placeholder = formatDefaultPlaceholderValue(inheritedDefaultValue)
    if (placeholder.trim()) {
      return placeholder
    }

    return props.field.secret ? '********' : t('bugBounty.monitor.usePluginDefaultParamValue')
  })()

const setNumberValue = (rawValue: string) => {
  const value = rawValue.trim()
  if (!value) {
    resetField()
    return
  }

  const parsed =
    props.field.type === 'integer' ? Number.parseInt(value, 10) : Number.parseFloat(value)
  if (Number.isNaN(parsed)) {
    props.fieldErrors[pathKey.value] = t('bugBounty.monitor.pluginParamNumberInvalid')
    return
  }

  if (props.field.minimum !== undefined && parsed < props.field.minimum) {
    props.fieldErrors[pathKey.value] = t('bugBounty.monitor.pluginParamOutOfRange')
    return
  }
  if (props.field.maximum !== undefined && parsed > props.field.maximum) {
    props.fieldErrors[pathKey.value] = t('bugBounty.monitor.pluginParamOutOfRange')
    return
  }

  delete props.fieldErrors[pathKey.value]
  setValueAtPath(props.params, props.field.path, parsed)
}

const getBooleanValue = () => {
  const value = getEffectiveValue()
  return Boolean(value)
}

const setBooleanValue = (checked: boolean) => {
  delete props.fieldErrors[pathKey.value]
  setValueAtPath(props.params, props.field.path, checked)
}

const getArraySelection = () => {
  const value = getEffectiveValue()
  return Array.isArray(value) ? value.map(item => String(item)) : []
}

const selectAllSuggestedOptions = () => {
  delete props.fieldErrors[pathKey.value]
  setValueAtPath(props.params, props.field.path, [...props.field.arraySuggestedOptions])
}

const clearArrayValue = () => {
  resetField()
}

const toggleArrayOption = (option: string, checked: boolean) => {
  const selected = new Set(getArraySelection())
  if (checked) {
    selected.add(option)
  } else {
    selected.delete(option)
  }

  if (selected.size === 0) {
    resetField()
    return
  }

  delete props.fieldErrors[pathKey.value]
  setValueAtPath(props.params, props.field.path, Array.from(selected))
}

const getArrayLinesValue = () => {
  const value = getEffectiveValue()
  const source = Array.isArray(value) ? value : []
  return source.map(item => String(item)).join('\n')
}

const getArrayPlaceholder = () => {
  const inheritedDefaultValue = getInheritedDefaultValue()
  if (Array.isArray(inheritedDefaultValue) && inheritedDefaultValue.length > 0) {
    return inheritedDefaultValue.map(item => String(item)).join('\n')
  }
  return t('bugBounty.monitor.pluginParamArrayPlaceholder')
}

const setArrayLinesValue = (rawValue: string) => {
  const lines = rawValue
    .split('\n')
    .map(item => item.trim())
    .filter(Boolean)

  if (lines.length === 0) {
    resetField()
    return
  }

  const itemType = String(props.field.items?.type || 'string')
  const parsedItems = lines.map(item => {
    if (itemType === 'integer') {
      return Number.parseInt(item, 10)
    }
    if (itemType === 'number') {
      return Number.parseFloat(item)
    }
    return item
  })

  if (parsedItems.some(item => Number.isNaN(item))) {
    props.fieldErrors[pathKey.value] = t('bugBounty.monitor.pluginParamsJsonInvalid')
    return
  }

  delete props.fieldErrors[pathKey.value]
  setValueAtPath(props.params, props.field.path, parsedItems)
}

const getJsonEditorText = () => {
  if (Object.prototype.hasOwnProperty.call(props.jsonEditorValues, pathKey.value)) {
    return props.jsonEditorValues[pathKey.value]
  }

  const value = getEffectiveValue()
  return value === undefined ? '' : JSON.stringify(value, null, 2)
}

const getJsonPlaceholder = () => {
  const inheritedDefaultValue = getInheritedDefaultValue()
  if (inheritedDefaultValue !== undefined) {
    return JSON.stringify(inheritedDefaultValue, null, 2)
  }
  return '{}'
}

const setJsonEditorText = (text: string) => {
  props.jsonEditorValues[pathKey.value] = text

  const trimmed = text.trim()
  if (!trimmed) {
    resetField()
    return
  }

  try {
    setValueAtPath(props.params, props.field.path, JSON.parse(trimmed))
    delete props.fieldErrors[pathKey.value]
  } catch {
    props.fieldErrors[pathKey.value] = t('bugBounty.monitor.pluginParamsJsonInvalid')
  }
}
</script>
