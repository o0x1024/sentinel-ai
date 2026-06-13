<template>
  <AppDialog ref="dialogRef" class="modal">
    <div class="modal-box flex max-h-[90vh] w-[92vw] max-w-2xl flex-col overflow-hidden p-0">
      <div class="border-b border-base-300 px-5 py-4">
        <h3 class="text-center text-lg font-semibold">{{ $t('trafficAnalysis.codec.importDialog.title') }}</h3>
      </div>

      <div class="min-h-0 flex-1 space-y-4 overflow-y-auto px-5 py-4">
        <div class="flex flex-wrap items-center gap-3">
          <label class="btn btn-sm btn-outline cursor-pointer">
            {{ $t('trafficAnalysis.codec.importDialog.selectFile') }}
            <input
              ref="fileInputRef"
              type="file"
              accept=".json"
              class="hidden"
              @change="handleFileSelect"
            />
          </label>
          <span class="text-sm text-base-content/60">{{ $t('trafficAnalysis.codec.importDialog.orPasteJson') }}:</span>
        </div>

        <textarea
          v-model="jsonText"
          class="textarea textarea-bordered min-h-40 w-full font-mono text-sm"
          :placeholder="$t('trafficAnalysis.codec.importDialog.pastePlaceholder')"
          @input="handleJsonInput"
        />

        <p v-if="parseError" class="text-sm text-error">{{ parseError }}</p>

        <div v-if="parsedData" class="space-y-4">
          <p class="text-sm text-base-content/80">
            {{
              placeholderEntries.length > 0
                ? $t('trafficAnalysis.codec.importDialog.detectedRulesWithKeys', { count: parsedData.rules.length })
                : $t('trafficAnalysis.codec.importDialog.detectedRules', { count: parsedData.rules.length })
            }}
          </p>

          <div v-if="placeholderEntries.length > 0" class="space-y-3">
            <label
              v-for="entry in placeholderEntries"
              :key="entry.key"
              class="form-control"
            >
              <span class="label-text mb-1">
                {{ entry.key.toUpperCase() }}:
                <span v-if="entry.info.description" class="text-base-content/60">
                  ({{ entry.info.description }}<template v-if="entry.info.format">, {{ entry.info.format }}</template>)
                </span>
              </span>
              <input
                v-model="keyValues[entry.key]"
                type="text"
                class="input input-bordered font-mono"
                :placeholder="entry.info.format || $t('trafficAnalysis.codec.importDialog.keyPlaceholder')"
              />
            </label>
          </div>
        </div>

        <p v-if="importError" class="text-sm text-error">{{ importError }}</p>
      </div>

      <div class="flex items-center justify-end gap-3 border-t border-base-300 bg-base-100 px-5 py-4">
        <button class="btn btn-ghost" type="button" @click="close">{{ $t('trafficAnalysis.codec.importDialog.cancel') }}</button>
        <button
          class="btn btn-primary"
          type="button"
          :disabled="!canImport || importing"
          @click="submitImport"
        >
          {{ importing ? $t('trafficAnalysis.codec.importDialog.importing') : $t('trafficAnalysis.codec.importDialog.import') }}
        </button>
      </div>
    </div>

    <form method="dialog" class="modal-backdrop">
      <button type="button" @click="close">{{ $t('trafficAnalysis.codec.importDialog.close') }}</button>
    </form>
  </AppDialog>
</template>

<script setup lang="ts">
import { computed, reactive, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import AppDialog from '@/components/AppDialog.vue'
import { importCodecRules, parseCodecExportJson } from './trafficCodecExportImport'
import type { CodecExportData, KeyPlaceholderInfo } from './trafficCodecTypes'

const emit = defineEmits<{
  success: [importedIds: string[]]
  closed: []
}>()

const { t } = useI18n()

const dialogRef = ref<InstanceType<typeof AppDialog> | null>(null)
const fileInputRef = ref<HTMLInputElement | null>(null)
const jsonText = ref('')
const parseError = ref('')
const importError = ref('')
const importing = ref(false)
const parsedData = ref<CodecExportData | null>(null)
const keyValues = reactive<Record<string, string>>({})

const placeholderEntries = computed(() => {
  if (!parsedData.value) return []
  return Object.entries(parsedData.value.keyPlaceholders).map(([key, info]) => ({
    key,
    info: info as KeyPlaceholderInfo,
  }))
})

const canImport = computed(() => parsedData.value !== null && parsedData.value.rules.length > 0)

function resetState() {
  jsonText.value = ''
  parseError.value = ''
  importError.value = ''
  importing.value = false
  parsedData.value = null
  for (const key of Object.keys(keyValues)) {
    delete keyValues[key]
  }
  if (fileInputRef.value) {
    fileInputRef.value.value = ''
  }
}

function syncKeyValues(data: CodecExportData) {
  for (const key of Object.keys(keyValues)) {
    if (!(key in data.keyPlaceholders)) {
      delete keyValues[key]
    }
  }
  for (const key of Object.keys(data.keyPlaceholders)) {
    if (!(key in keyValues)) {
      keyValues[key] = ''
    }
  }
}

function handleJsonInput() {
  importError.value = ''
  if (!jsonText.value.trim()) {
    parseError.value = ''
    parsedData.value = null
    return
  }
  try {
    const data = parseCodecExportJson(jsonText.value)
    parsedData.value = data
    parseError.value = ''
    syncKeyValues(data)
  } catch (error) {
    parsedData.value = null
    parseError.value = error instanceof Error ? error.message : t('trafficAnalysis.codec.importDialog.jsonParseFailed')
  }
}

function handleFileSelect(event: Event) {
  const input = event.target as HTMLInputElement
  const file = input.files?.[0]
  if (!file) return

  const reader = new FileReader()
  reader.onload = () => {
    jsonText.value = String(reader.result ?? '')
    handleJsonInput()
  }
  reader.onerror = () => {
    parseError.value = t('trafficAnalysis.codec.importDialog.readFileFailed')
  }
  reader.readAsText(file)
}

async function submitImport() {
  if (!parsedData.value || !jsonText.value.trim()) return

  importError.value = ''
  importing.value = true
  try {
    const importedIds = await importCodecRules(jsonText.value, { ...keyValues })
    emit('success', importedIds)
    close()
  } catch (error) {
    importError.value = error instanceof Error ? error.message : t('trafficAnalysis.codec.importDialog.importFailed')
  } finally {
    importing.value = false
  }
}

function showModal() {
  resetState()
  dialogRef.value?.showModal()
}

function close() {
  dialogRef.value?.close()
  emit('closed')
}

defineExpose({
  showModal,
  close,
})
</script>
