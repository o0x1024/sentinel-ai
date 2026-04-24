<template>
  <div class="space-y-3 p-3">
    <div class="grid gap-3 md:grid-cols-[10rem_minmax(0,1fr)]">
    <div class="space-y-2">
      <button
        class="btn btn-sm btn-outline w-full justify-center"
        type="button"
        data-testid="simple-list-paste"
        @click="pastePayloads"
      >
        {{ t('trafficAnalysis.intruder.actions.paste') }}
      </button>
      <button class="btn btn-sm btn-outline w-full justify-center" type="button" @click="loadPayloadsFromFile">
        {{ t('trafficAnalysis.intruder.actions.loadFile') }}
      </button>
      <button
        class="btn btn-sm btn-outline w-full justify-center"
        type="button"
        :disabled="selectedPayloadIndex === null"
        @click="removeSelectedPayload"
      >
        {{ t('trafficAnalysis.intruder.actions.remove') }}
      </button>
      <button
        class="btn btn-sm btn-outline w-full justify-center"
        type="button"
        :disabled="payloadItems.length === 0"
        @click="clearPayloads"
      >
        {{ t('trafficAnalysis.intruder.actions.clear') }}
      </button>
      <button
        class="btn btn-sm btn-outline w-full justify-center"
        type="button"
        :disabled="payloadItems.length < 2"
        @click="deduplicatePayloads"
      >
        {{ t('trafficAnalysis.intruder.actions.deduplicate') }}
      </button>
    </div>

    <div class="min-w-0 space-y-3">
      <div class="overflow-hidden rounded-lg border border-base-300 bg-base-100">
        <div class="max-h-64 overflow-auto">
          <table class="table table-pin-rows table-sm">
            <thead>
              <tr class="bg-base-200/80 text-[10px] uppercase tracking-wide text-base-content/60">
                <th>{{ t('trafficAnalysis.intruder.labels.payloads') }}</th>
              </tr>
            </thead>
            <tbody v-if="payloadItems.length">
              <tr
                v-for="(payload, index) in payloadItems"
                :key="`${index}-${payload}`"
                data-testid="simple-list-payload-row"
                class="cursor-pointer"
                :class="selectedPayloadIndex === index ? 'bg-primary/10' : 'hover:bg-base-200/40'"
                @click="selectPayload(index)"
              >
                <td class="p-0 align-middle">
                  <input
                    :value="payload"
                    type="text"
                    class="w-full border-0 bg-transparent px-2 py-0.5 font-mono text-[11px] leading-4 outline-none"
                    :class="selectedPayloadIndex === index ? 'ring-1 ring-inset ring-primary/50' : ''"
                    @focus="selectPayload(index)"
                    @input="updatePayload(index, ($event.target as HTMLInputElement).value)"
                  />
                </td>
              </tr>
            </tbody>
          </table>

          <div
            v-if="!payloadItems.length"
            class="flex h-40 items-center justify-center px-4 text-sm text-base-content/50"
          >
            {{ t('trafficAnalysis.intruder.empty.noArrayItems') }}
          </div>
        </div>
      </div>
    </div>
    </div>

    <div class="grid gap-3 md:grid-cols-[10rem_minmax(0,1fr)]">
      <button class="btn btn-sm btn-outline w-full" type="button" data-testid="simple-list-add" @click="addPayload">
        {{ t('trafficAnalysis.intruder.actions.add') }}
      </button>
      <input
        v-model="newPayloadValue"
        type="text"
        data-testid="simple-list-add-input"
        class="input input-bordered input-sm w-full font-mono"
        :placeholder="t('trafficAnalysis.intruder.placeholders.arrayItem')"
        @keydown.enter.prevent="addPayload"
      />
    </div>

    <button
      class="btn btn-sm btn-primary w-full"
      type="button"
      data-testid="simple-list-open-library"
      @click="payloadLibraryOpen = true"
    >
      {{ t('trafficAnalysis.intruder.actions.openPayloadLibrary') }}
    </button>

    <IntruderPayloadSourcePicker
      :open="payloadLibraryOpen"
      :payload-set="payloadSet"
      :payload-sets="payloadSets"
      :attack-type="attackType"
      :positions="positions"
      :request-text="requestText"
      @close="payloadLibraryOpen = false"
      @import="handleLibraryImport"
      @apply-template="emit('apply-template', $event)"
    />
  </div>
</template>

<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'
import { readTextFile } from '@tauri-apps/plugin-fs'
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { dialog } from '@/composables/useDialog'
import IntruderPayloadSourcePicker from './IntruderPayloadSourcePicker.vue'
import { applyIntruderPayloadImportMode, type IntruderPayloadImportMode } from './intruderPayloadLibrarySupport'
import { parsePayloadLines } from './payloads'
import type { IntruderAttackType, IntruderPayloadSet, IntruderPosition } from './types'

const props = defineProps<{
  payloadSet: IntruderPayloadSet
  payloadSets: IntruderPayloadSet[]
  attackType: IntruderAttackType
  positions: IntruderPosition[]
  requestText: string
}>()

const emit = defineEmits<{
  (e: 'update', patch: Partial<IntruderPayloadSet>): void
  (e: 'apply-template', value: {
    sourceRef: string
    attackType: IntruderAttackType
    sets: Array<{ name: string; payloadsText: string }>
  }): void
}>()

const { t } = useI18n()
const selectedPayloadIndex = ref<number | null>(null)
const newPayloadValue = ref('')
const payloadLibraryOpen = ref(false)

const payloadItems = computed(() => parsePayloadLines(props.payloadSet.payloadsText))

watch(payloadItems, (items) => {
  if (selectedPayloadIndex.value === null) {
    return
  }

  if (selectedPayloadIndex.value >= items.length) {
    selectedPayloadIndex.value = items.length ? items.length - 1 : null
  }
})

function emitPayloadItems(items: string[]) {
  emit('update', {
    payloadsText: items.join('\n'),
  })
}

function appendPayloadItems(items: string[]) {
  if (!items.length) {
    return
  }

  emitPayloadItems([...payloadItems.value, ...items])
  if (selectedPayloadIndex.value === null) {
    selectedPayloadIndex.value = 0
  }
}

function selectPayload(index: number) {
  selectedPayloadIndex.value = index
}

function updatePayload(index: number, value: string) {
  const nextItems = [...payloadItems.value]
  nextItems[index] = value
  emitPayloadItems(nextItems)
}

function addPayload() {
  const nextValue = newPayloadValue.value.trim()
  if (!nextValue) {
    return
  }

  const nextItems = [...payloadItems.value, nextValue]
  emitPayloadItems(nextItems)
  selectedPayloadIndex.value = nextItems.length - 1
  newPayloadValue.value = ''
}

function handleLibraryImport(value: {
  mode: IntruderPayloadImportMode
  payloads: string[]
  sourceRef: string
}) {
  const result = applyIntruderPayloadImportMode(payloadItems.value, value.payloads, value.mode)
  emit('update', {
    payloadsText: result.nextPayloadsText,
  })
}

async function pastePayloads() {
  try {
    const response = await invoke<{ success: boolean; data: string; error?: string }>('read_traffic_clipboard_text')
    if (!response?.success) {
      throw new Error(response?.error || t('trafficAnalysis.repeater.messages.cannotReadClipboard'))
    }

    appendPayloadItems(parsePayloadLines(response.data || ''))
  } catch (error) {
    console.error('Failed to read Intruder payload clipboard content', error)
    dialog.toast.error(error instanceof Error ? error.message : t('trafficAnalysis.repeater.messages.cannotReadClipboard'))
  }
}

async function loadPayloadsFromFile() {
  try {
    const selected = await open({
      multiple: false,
      directory: false,
      filters: [{ name: 'Text', extensions: ['txt', 'lst', 'csv', 'log'] }],
    })
    if (!selected || Array.isArray(selected)) {
      return
    }

    const content = await readTextFile(selected)
    appendPayloadItems(parsePayloadLines(content))
  } catch (error) {
    console.error('Failed to load Intruder payload file', error)
    dialog.toast.error(t('trafficAnalysis.intruder.messages.payloadFileLoadFailed'))
  }
}

function removeSelectedPayload() {
  if (selectedPayloadIndex.value === null) {
    return
  }

  const nextItems = payloadItems.value.filter((_, index) => index !== selectedPayloadIndex.value)
  emitPayloadItems(nextItems)
  if (!nextItems.length) {
    selectedPayloadIndex.value = null
    return
  }

  selectedPayloadIndex.value = Math.min(selectedPayloadIndex.value, nextItems.length - 1)
}

function clearPayloads() {
  emitPayloadItems([])
  selectedPayloadIndex.value = null
}

function deduplicatePayloads() {
  emitPayloadItems(Array.from(new Set(payloadItems.value)))
}
</script>

<style scoped>
:deep(.table thead th) {
  padding-top: 0.25rem;
  padding-bottom: 0.25rem;
  font-size: 10px;
}

:deep(.table tbody td) {
  padding-top: 0;
  padding-bottom: 0;
}

:deep(.table tbody tr) {
  height: 1.5rem;
}
</style>
