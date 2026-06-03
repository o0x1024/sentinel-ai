<template>
  <AppDialog :open="open" class="modal">
    <div class="modal-box w-11/12 max-w-5xl max-h-[90vh] overflow-y-auto">
      <div class="flex items-start justify-between gap-4">
        <div>
          <h3 class="text-lg font-bold">
            {{ $t('plugins.defaultConfigTitle', '插件默认值配置') }}
          </h3>
          <div class="mt-1 text-sm text-base-content/70">
            {{ plugin?.metadata.name || '-' }}
            <span class="ml-2 font-mono text-xs">{{ plugin?.metadata.id || '' }}</span>
          </div>
        </div>
        <button class="btn btn-sm btn-circle btn-ghost" @click="$emit('close')">✕</button>
      </div>

      <div class="mt-4 space-y-4">
        <div class="rounded-lg border border-base-300 bg-base-200/60 p-3 text-sm text-base-content/70">
          {{ $t('plugins.defaultConfigHint', '这里配置的是插件默认输入。其它地方调用该插件时，会在显式输入缺失时自动继承这些值。') }}
        </div>

        <div v-if="loading" class="alert alert-info">
          <span class="loading loading-spinner"></span>
          <span>{{ $t('plugins.loadingDefaultConfig', '正在加载默认配置...') }}</span>
        </div>

        <div class="grid gap-4 lg:grid-cols-[minmax(0,1fr)_20rem]">
          <div class="space-y-4">
            <div class="flex items-center justify-between gap-3">
              <div class="tabs tabs-boxed tabs-sm">
                <button
                  type="button"
                  class="tab"
                  :class="{ 'tab-active': editorMode === 'form' }"
                  @click="editorMode = 'form'"
                >
                  {{ $t('plugins.formMode', '图形化') }}
                </button>
                <button
                  type="button"
                  class="tab"
                  :class="{ 'tab-active': editorMode === 'json' }"
                  @click="editorMode = 'json'"
                >
                  {{ $t('plugins.jsonMode', 'JSON') }}
                </button>
              </div>

              <button
                v-if="hasEditableSchema"
                type="button"
                class="btn btn-ghost btn-xs"
                @click="resetToLoadedConfig"
              >
                {{ $t('plugins.resetConfig', '重置') }}
              </button>
            </div>

            <div v-if="editorMode === 'form' && hasEditableSchema" class="space-y-3">
              <div class="rounded-lg border border-base-300/70 bg-base-100/70 p-3">
                <div class="text-xs text-base-content/60">
                  {{ $t('plugins.formModeHint', '优先使用图形化配置；只有在复杂场景下再切换到 JSON。') }}
                </div>
              </div>

              <div class="space-y-3">
                <MonitorPluginParamsField
                  v-for="field in editableFields"
                  :key="getFieldPathKey(field.path)"
                  :field="field"
                  :params="paramValues"
                  :json-editor-values="jsonEditorValues"
                  :field-errors="fieldErrors"
                  :expanded-hint-fields="expandedHintFields"
                />
              </div>
            </div>

            <div v-else-if="editorMode === 'json'" class="space-y-3">
              <label class="form-control">
                <span class="label-text text-sm font-medium">
                  {{ $t('plugins.defaultConfigJson', '默认配置 JSON') }}
                </span>
                <textarea
                  v-model="rawConfigText"
                  class="textarea textarea-bordered min-h-[24rem] font-mono text-xs"
                  spellcheck="false"
                />
              </label>
              <div class="text-xs text-base-content/60">
                {{ $t('plugins.advancedJsonHint', '高级模式：直接编辑插件默认配置 JSON。') }}
              </div>
            </div>

            <div v-else class="rounded-lg border border-base-300 bg-base-200 p-4 text-sm text-base-content/70">
              {{ $t('plugins.noEditablePluginParams', '当前插件没有可图形化编辑的参数，可切换到 JSON 模式。') }}
            </div>

            <div v-if="combinedError" class="alert alert-error">
              <i class="fas fa-exclamation-circle"></i>
              <span>{{ combinedError }}</span>
            </div>
          </div>

          <div class="space-y-3">
            <div class="rounded-lg border border-base-300 p-3">
              <div class="text-sm font-medium">
                {{ $t('plugins.schemaPreview', '输入 Schema 预览') }}
              </div>
              <pre class="mt-2 max-h-[24rem] overflow-auto whitespace-pre-wrap break-all rounded bg-base-200/60 p-3 text-xs">{{ schemaPreview }}</pre>
            </div>
          </div>
        </div>
      </div>

      <div class="modal-action">
        <button class="btn btn-sm" @click="$emit('close')">
          {{ $t('common.cancel', '取消') }}
        </button>
        <button class="btn btn-primary btn-sm" :disabled="loading || saving || hasErrors" @click="handleSave">
          <span v-if="saving" class="loading loading-spinner loading-xs"></span>
          {{ saving ? $t('common.saving', '保存中...') : $t('common.save', '保存') }}
        </button>
      </div>
    </div>
    <form method="dialog" class="modal-backdrop">
      <button @click="$emit('close')">close</button>
    </form>
  </AppDialog>
</template>

<script setup lang="ts">
import { computed, reactive, ref, watch } from 'vue'
import MonitorPluginParamsField from '@/components/PluginManagement/MonitorPluginParamsField.vue'
import {
  buildEditableFields,
  cloneValue,
  getFieldPathKey,
} from '@/components/PluginManagement/monitorPluginParamsSupport'
import type { PluginRecord } from './types'

const props = defineProps<{
  open: boolean
  plugin: PluginRecord | null
  modelValue: string
  schema: any
  loading: boolean
  saving: boolean
  error: string
}>()

const emit = defineEmits<{
  'update:modelValue': [value: string]
  close: []
  save: []
}>()

const editorMode = ref<'form' | 'json'>('form')
const rawConfigText = ref('{}')
const localError = ref('')
const lastLoadedConfigText = ref('{}')
const paramValues = ref<Record<string, any>>({})
const fieldErrors = reactive<Record<string, string>>({})
const jsonEditorValues = reactive<Record<string, string>>({})
const expandedHintFields = reactive<Record<string, boolean>>({})

const editableSchema = computed(() => {
  if (props.schema && typeof props.schema === 'object' && !Array.isArray(props.schema)) {
    return props.schema
  }
  return { type: 'object', properties: {} }
})

const editableFields = computed(() => buildEditableFields(editableSchema.value))
const hasEditableSchema = computed(() => editableFields.value.length > 0)
const schemaPreview = computed(() => JSON.stringify(editableSchema.value, null, 2))
const hasErrors = computed(() => {
  if (localError.value.trim()) {
    return true
  }
  return Object.values(fieldErrors).some(Boolean)
})
const combinedError = computed(() => localError.value || props.error)

const clearEditorErrors = () => {
  localError.value = ''
  for (const key of Object.keys(fieldErrors)) {
    delete fieldErrors[key]
  }
  for (const key of Object.keys(jsonEditorValues)) {
    delete jsonEditorValues[key]
  }
}

const normalizeConfigObject = (value: unknown) => {
  if (!value || typeof value !== 'object' || Array.isArray(value)) {
    return {}
  }
  return cloneValue(value as Record<string, any>)
}

const parseRawConfigText = () => {
  const trimmed = rawConfigText.value.trim()
  if (!trimmed) {
    return {}
  }

  const parsed = JSON.parse(trimmed)
  if (!parsed || typeof parsed !== 'object' || Array.isArray(parsed)) {
    throw new Error('插件默认配置必须是 JSON 对象')
  }
  return normalizeConfigObject(parsed)
}

const applyParsedConfigToForm = (config: Record<string, any>) => {
  paramValues.value = normalizeConfigObject(config)
}

const serializeParams = () => normalizeConfigObject(paramValues.value)

const initializeForm = () => {
  clearEditorErrors()
  lastLoadedConfigText.value = props.modelValue || '{}'

  let parsedConfig: Record<string, any> = {}
  try {
    parsedConfig = props.modelValue.trim() ? parseJsonObject(props.modelValue) : {}
  } catch {
    localError.value = '插件默认配置 JSON 无法解析'
  }

  applyParsedConfigToForm(parsedConfig)
  rawConfigText.value = JSON.stringify(parsedConfig, null, 2)
  editorMode.value = hasEditableSchema.value ? 'form' : 'json'
}

const resetToLoadedConfig = () => {
  try {
    const parsed = parseJsonObject(lastLoadedConfigText.value)
    clearEditorErrors()
    applyParsedConfigToForm(parsed)
    rawConfigText.value = JSON.stringify(parsed, null, 2)
    editorMode.value = hasEditableSchema.value ? 'form' : 'json'
  } catch {
    localError.value = '无法重置到已加载配置'
  }
}

const handleSave = () => {
  try {
    const nextConfig = editorMode.value === 'json' ? parseRawConfigText() : serializeParams()
    const nextText = JSON.stringify(nextConfig, null, 2)
    emit('update:modelValue', nextText)
    emit('save')
  } catch (error) {
    localError.value = error instanceof Error ? error.message : '保存插件默认配置失败'
  }
}

watch(
  () => [props.open, props.schema, props.modelValue] as const,
  ([open]) => {
    if (!open) {
      return
    }
    initializeForm()
  },
  { immediate: true }
)

watch(editorMode, (mode, previousMode) => {
  if (mode === previousMode) {
    return
  }

  if (mode === 'json') {
    try {
      clearEditorErrors()
      rawConfigText.value = JSON.stringify(serializeParams(), null, 2)
    } catch {
      localError.value = '无法将当前图形化配置转换为 JSON'
    }
    return
  }

  try {
    const parsed = parseRawConfigText()
    clearEditorErrors()
    applyParsedConfigToForm(parsed)
  } catch (error) {
    localError.value = error instanceof Error ? error.message : '插件默认配置 JSON 无法解析'
  }
})

function parseJsonObject(raw: string) {
  const parsed = JSON.parse(raw)
  if (!parsed || typeof parsed !== 'object' || Array.isArray(parsed)) {
    throw new Error('Plugin config must be a JSON object')
  }
  return normalizeConfigObject(parsed)
}
</script>
