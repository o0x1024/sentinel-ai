<template>
  <div
    v-if="pluginId"
    class="min-w-0 rounded-lg border border-base-300/70 bg-base-200/60 p-3 space-y-3 overflow-x-hidden"
  >
    <div class="flex items-center justify-between gap-2">
      <div class="text-xs font-semibold text-base-content/80">
        {{ t('bugBounty.monitor.pluginRuntimeParams') }}
      </div>
      <div class="flex items-center gap-2">
        <button
          v-if="hasCustomizedParams"
          type="button"
          class="btn btn-ghost btn-xs text-primary"
          @click="resetAllModifiedParams"
        >
          {{ t('bugBounty.monitor.resetAllModifiedParams') }}
        </button>
        <span v-if="loading" class="loading loading-spinner loading-xs"></span>
      </div>
    </div>

    <div v-if="loadError" class="text-xs text-warning">
      {{ loadError }}
    </div>

    <div v-if="!loadError && defaultConfigLoadError" class="text-xs text-warning">
      {{ defaultConfigLoadError }}
    </div>

    <template v-if="!loadError">
      <div
        v-if="editableFields.length > 0"
        class="flex flex-wrap items-center justify-between gap-2 rounded-lg border border-base-300/60 bg-base-100/70 px-3 py-2"
      >
        <label class="label cursor-pointer justify-start gap-2 py-0">
          <input
            v-model="showOnlyChanged"
            type="checkbox"
            class="checkbox checkbox-xs checkbox-primary"
          />
          <span class="text-xs text-base-content/70">
            {{ t('bugBounty.monitor.showChangedOnly') }}
          </span>
        </label>
        <div v-if="validationErrors.length > 0" class="text-xs text-warning">
          {{ t('bugBounty.monitor.pluginParamsValidationFailed') }}
        </div>
      </div>

      <div v-if="editableFields.length === 0" class="text-xs text-base-content/60">
        {{ t('bugBounty.monitor.noEditablePluginParams') }}
      </div>

      <div v-else class="space-y-3">
        <MonitorPluginParamsField
          v-for="field in editableFields"
          :key="getFieldPathKey(field.path)"
          :field="field"
          :params="pluginParams"
          :default-params="pluginDefaultParams"
          :json-editor-values="jsonEditorValues"
          :field-errors="fieldErrors"
          :expanded-hint-fields="expandedHintFields"
          :show-only-changed="showOnlyChanged"
        />
      </div>

      <div class="rounded-lg border border-base-300/70 bg-base-100/70 p-3 space-y-2">
        <div class="text-xs font-semibold text-base-content/80">
          {{ t('bugBounty.monitor.injectedRuntimeParams') }}
        </div>
        <div class="text-xs text-base-content/60">
          {{ t('bugBounty.monitor.injectedRuntimeParamsHint') }}
        </div>
        <div class="flex flex-wrap gap-2">
          <span
            v-for="item in injectedParamLabels"
            :key="item"
            class="rounded-full border border-base-300 bg-base-100 px-2 py-1 text-xs text-base-content/70"
          >
            {{ item }}
          </span>
        </div>
      </div>
    </template>
  </div>
</template>

<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core'
import { computed, reactive, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { buildResolvedPluginDefaultConfig } from '@/services/pluginDefaultConfig'
import MonitorPluginParamsField from './MonitorPluginParamsField.vue'
import {
  buildEditableFields,
  clearPluginParamEditorState,
  getFieldPathKey,
  getValueAtPath,
  hasCustomizedField,
  setPluginParamEditorState,
  type EditableField,
} from './monitorPluginParamsSupport'

const props = defineProps<{
  plugin: Record<string, any>
  monitorType?: string
}>()

const { t } = useI18n()

const schema = ref<any>(null)
const loading = ref(false)
const loadError = ref('')
const defaultConfigLoadError = ref('')
const showOnlyChanged = ref(false)
const expandedHintFields = reactive<Record<string, boolean>>({})
const fieldErrors = reactive<Record<string, string>>({})
const jsonEditorValues = reactive<Record<string, string>>({})
const pluginDefaultParams = ref<Record<string, unknown>>({})

const schemaCache = new Map<string, any>()

const pluginId = computed(() => String(props.plugin?.plugin_id || '').trim())

const pluginParams = computed(() => {
  if (
    !props.plugin.plugin_params
    || typeof props.plugin.plugin_params !== 'object'
    || Array.isArray(props.plugin.plugin_params)
  ) {
    props.plugin.plugin_params = {}
  }

  return props.plugin.plugin_params as Record<string, unknown>
})

const editableFields = computed<EditableField[]>(() => buildEditableFields(schema.value))

const validationErrors = computed(() =>
  Object.entries(fieldErrors)
    .filter(([, message]) => Boolean(message))
    .map(([path, message]) => `${path}: ${message}`)
)

const hasCustomizedParams = computed(() =>
  editableFields.value.some(field => hasCustomizedField(pluginParams.value as Record<string, any>, field))
)

const getInheritedDefaultValue = (field: EditableField) => {
  const pluginDefaultValue = getValueAtPath(
    pluginDefaultParams.value as Record<string, any>,
    field.path,
  )
  return pluginDefaultValue === undefined ? field.defaultValue : pluginDefaultValue
}

const syncJsonEditors = () => {
  const validFieldKeys = new Set<string>()

  for (const field of editableFields.value) {
    if (field.control !== 'json') {
      continue
    }

    const pathKey = getFieldPathKey(field.path)
    validFieldKeys.add(pathKey)
    const explicitValue = getValueAtPath(pluginParams.value as Record<string, any>, field.path)
    const sourceValue = explicitValue === undefined ? getInheritedDefaultValue(field) : explicitValue
    jsonEditorValues[pathKey] =
      sourceValue === undefined ? '' : JSON.stringify(sourceValue, null, 2)
  }

  for (const key of Object.keys(jsonEditorValues)) {
    if (!validFieldKeys.has(key)) {
      delete jsonEditorValues[key]
    }
  }
}

const clearValidationState = () => {
  for (const key of Object.keys(fieldErrors)) {
    delete fieldErrors[key]
  }
  clearPluginParamEditorState(props.plugin)
}

const updateValidationState = () => {
  setPluginParamEditorState(props.plugin, validationErrors.value)
}

const loadPluginSchema = async (nextPluginId: string) => {
  clearValidationState()

  if (!nextPluginId) {
    schema.value = null
    loadError.value = ''
    return { type: 'object', properties: {} }
  }

  if (schemaCache.has(nextPluginId)) {
    const cachedSchema = schemaCache.get(nextPluginId) || { type: 'object', properties: {} }
    schema.value = cachedSchema
    loadError.value = ''
    syncJsonEditors()
    return cachedSchema
  }

  loadError.value = ''

  try {
    const response = await invoke<any>('get_plugin_input_schema', { pluginId: nextPluginId })
    const responseSchema = response?.success ? response.data : response
    const resolvedSchema =
      responseSchema && typeof responseSchema === 'object'
        ? responseSchema
        : { type: 'object', properties: {} }

    schemaCache.set(nextPluginId, resolvedSchema)
    schema.value = resolvedSchema
    syncJsonEditors()
    return resolvedSchema
  } catch (error) {
    schema.value = { type: 'object', properties: {} }
    loadError.value =
      error instanceof Error
        ? error.message
        : t('bugBounty.monitor.pluginParamsLoadFailed')
    return { type: 'object', properties: {} }
  }
}

const loadPluginDefaultParams = async (nextPluginId: string, nextSchema: any) => {
  if (!nextPluginId) {
    pluginDefaultParams.value = {}
    defaultConfigLoadError.value = ''
    syncJsonEditors()
    return
  }

  defaultConfigLoadError.value = ''

  try {
    const resolvedConfig = await buildResolvedPluginDefaultConfig(nextPluginId, nextSchema)
    pluginDefaultParams.value = resolvedConfig
    syncJsonEditors()
  } catch (error) {
    pluginDefaultParams.value = {}
    defaultConfigLoadError.value =
      error instanceof Error
        ? error.message
        : t('bugBounty.monitor.pluginParamsLoadFailed')
  }
}

const resetAllModifiedParams = () => {
  props.plugin.plugin_params = {}
  clearValidationState()
  syncJsonEditors()
}

const injectedParamLabels = computed(() => {
  const labels = [
    'targets[]',
    'target_objects[]',
    'domain',
    'domains',
    'url',
    'urls',
    'service_targets[]',
    '__monitorExecution',
  ]

  if (props.monitorType === 'dns' || props.monitorType === 'ip') {
    return labels
  }

  return labels
})

watch(
  pluginId,
  async nextPluginId => {
    loading.value = true
    try {
      const resolvedSchema = await loadPluginSchema(nextPluginId)
      await loadPluginDefaultParams(nextPluginId, resolvedSchema)
    } finally {
      loading.value = false
    }
  },
  { immediate: true }
)

watch(
  editableFields,
  () => {
    syncJsonEditors()
  },
  { deep: true }
)

watch(
  validationErrors,
  () => {
    updateValidationState()
  },
  { deep: true, immediate: true }
)
</script>
