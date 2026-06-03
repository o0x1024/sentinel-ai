<template>
  <AppDialog ref="dialogRef" class="modal">
    <div class="modal-box flex max-h-[90vh] w-[92vw] max-w-2xl flex-col overflow-hidden p-0">
      <div class="border-b border-base-300 px-5 py-4">
        <h3 class="text-center text-lg font-semibold">导入 Codec 规则</h3>
      </div>

      <div class="min-h-0 flex-1 space-y-4 overflow-y-auto px-5 py-4">
        <div class="flex flex-wrap items-center gap-3">
          <label class="btn btn-sm btn-outline cursor-pointer">
            选择文件
            <input
              ref="fileInputRef"
              type="file"
              accept=".json"
              class="hidden"
              @change="handleFileSelect"
            />
          </label>
          <span class="text-sm text-base-content/60">或粘贴 JSON:</span>
        </div>

        <textarea
          v-model="jsonText"
          class="textarea textarea-bordered min-h-40 w-full font-mono text-sm"
          placeholder='{"format":"sentinel-codec-rules", ...}'
          @input="handleJsonInput"
        />

        <p v-if="parseError" class="text-sm text-error">{{ parseError }}</p>

        <div v-if="parsedData" class="space-y-4">
          <p class="text-sm text-base-content/80">
            检测到 {{ parsedData.rules.length }} 条规则
            <template v-if="placeholderEntries.length > 0">, 需要填写以下密钥:</template>
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
                :placeholder="entry.info.format || '请输入密钥'"
              />
            </label>
          </div>
        </div>

        <p v-if="importError" class="text-sm text-error">{{ importError }}</p>
      </div>

      <div class="flex items-center justify-end gap-3 border-t border-base-300 bg-base-100 px-5 py-4">
        <button class="btn btn-ghost" type="button" @click="close">取消</button>
        <button
          class="btn btn-primary"
          type="button"
          :disabled="!canImport || importing"
          @click="submitImport"
        >
          {{ importing ? '导入中...' : '导入' }}
        </button>
      </div>
    </div>

    <form method="dialog" class="modal-backdrop">
      <button type="button" @click="close">关闭</button>
    </form>
  </AppDialog>
</template>

<script setup lang="ts">
import { computed, reactive, ref } from 'vue'
import AppDialog from '@/components/AppDialog.vue'
import { importCodecRules, parseCodecExportJson } from './trafficCodecExportImport'
import type { CodecExportData, KeyPlaceholderInfo } from './trafficCodecTypes'

const emit = defineEmits<{
  success: [importedIds: string[]]
  closed: []
}>()

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
    parseError.value = error instanceof Error ? error.message : 'JSON 解析失败'
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
    parseError.value = '读取文件失败'
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
    importError.value = error instanceof Error ? error.message : '导入失败'
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
