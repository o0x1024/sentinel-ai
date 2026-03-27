<template>
  <section class="flex h-full min-h-0 rounded-r-lg border border-l-0 border-base-300 bg-base-100">
    <div class="min-w-0 flex-1 overflow-hidden">
      <div v-if="activeTab === 'payloads'" class="flex h-full min-h-0 flex-col">
        <div class="border-b border-base-300 px-4 py-3">
          <h3 class="text-sm font-semibold">{{ $t('trafficAnalysis.intruder.sections.payloads') }}</h3>
        </div>

        <div class="space-y-4 overflow-auto p-4 text-sm">
          <label class="form-control">
            <span class="label-text text-xs">{{ $t('trafficAnalysis.intruder.labels.payloadPosition') }}</span>
            <select v-model="selectedPayloadSetId" class="select select-bordered select-sm">
              <option v-for="payloadSet in payloadSets" :key="payloadSet.id" :value="payloadSet.id">
                {{ payloadSet.name }}
              </option>
            </select>
          </label>

          <label class="form-control">
            <span class="label-text text-xs">{{ $t('trafficAnalysis.intruder.labels.payloadType') }}</span>
            <select
              :value="activePayloadSet?.payloadType || 'simpleList'"
              class="select select-bordered select-sm"
              @change="updateActivePayloadSet({ payloadType: ($event.target as HTMLSelectElement).value as IntruderPayloadSet['payloadType'] })"
            >
              <option value="simpleList">{{ $t('trafficAnalysis.intruder.labels.simpleList') }}</option>
              <option value="numbers">{{ $t('trafficAnalysis.intruder.labels.numbers') }}</option>
              <option value="dates">{{ $t('trafficAnalysis.intruder.labels.dates') }}</option>
              <option value="runtimeFile">{{ $t('trafficAnalysis.intruder.labels.runtimeFile') }}</option>
              <option value="characterList">{{ $t('trafficAnalysis.intruder.labels.characterList') }}</option>
              <option value="nullPayloads">{{ $t('trafficAnalysis.intruder.labels.nullPayloads') }}</option>
              <option value="characterSubstitution">{{ $t('trafficAnalysis.intruder.labels.characterSubstitution') }}</option>
              <option value="usernameGenerator">{{ $t('trafficAnalysis.intruder.labels.usernameGenerator') }}</option>
            </select>
          </label>

          <div class="grid grid-cols-2 gap-3 text-xs text-base-content/70">
            <div>
              <div>{{ $t('trafficAnalysis.intruder.labels.payloadCount') }}</div>
              <div class="mt-1 font-semibold text-base-content">{{ activePayloadCount }}</div>
            </div>
            <div>
              <div>{{ $t('trafficAnalysis.intruder.labels.requestCount') }}</div>
              <div class="mt-1 font-semibold text-base-content">{{ estimatedRequests }}</div>
            </div>
          </div>

          <div class="rounded-lg border border-base-300">
            <div class="border-b border-base-300 bg-base-200 px-4 py-2 text-xs font-semibold uppercase tracking-wide text-base-content/70">
              {{ $t('trafficAnalysis.intruder.labels.payloadConfiguration') }}
            </div>

            <div v-if="activePayloadSet?.payloadType === 'simpleList'" class="grid grid-cols-[6rem_1fr] gap-3 p-3">
              <div class="space-y-2">
                <button class="btn btn-sm btn-ghost w-full justify-start" type="button" @click="pastePayloads">
                  {{ $t('trafficAnalysis.intruder.actions.paste') }}
                </button>
                <button class="btn btn-sm btn-ghost w-full justify-start" type="button" @click="deduplicatePayloads">
                  {{ $t('trafficAnalysis.intruder.actions.deduplicate') }}
                </button>
                <button class="btn btn-sm btn-ghost w-full justify-start" type="button" @click="clearPayloads">
                  {{ $t('trafficAnalysis.intruder.actions.clear') }}
                </button>
              </div>

              <textarea
                :value="activePayloadSet?.payloadsText || ''"
                class="h-64 w-full resize-none rounded-lg border border-base-300 bg-base-100 p-3 font-mono text-xs leading-6 outline-none transition focus:border-primary"
                :placeholder="$t('trafficAnalysis.intruder.placeholders.payloads')"
                @input="updateActivePayloadSet({ payloadsText: ($event.target as HTMLTextAreaElement).value })"
              ></textarea>
            </div>

            <div v-else-if="activePayloadSet?.payloadType === 'runtimeFile'" class="grid gap-3 p-4">
              <div class="flex items-center gap-2">
                <button class="btn btn-sm btn-ghost" type="button" @click="loadPayloadFile">
                  {{ $t('trafficAnalysis.intruder.actions.loadFile') }}
                </button>
                <input
                  :value="activePayloadSet.filePath"
                  type="text"
                  readonly
                  class="input input-bordered input-sm flex-1"
                  :placeholder="$t('trafficAnalysis.intruder.placeholders.payloadFile')"
                />
              </div>
              <textarea
                :value="activePayloadSet?.payloadsText || ''"
                class="h-56 w-full resize-none rounded-lg border border-base-300 bg-base-100 p-3 font-mono text-xs leading-6 outline-none"
                readonly
              ></textarea>
            </div>

            <div v-else-if="activePayloadSet?.payloadType === 'numbers'" class="grid gap-3 p-4 md:grid-cols-2">
              <label class="form-control">
                <span class="label-text text-xs">{{ $t('trafficAnalysis.intruder.labels.numberFrom') }}</span>
                <input
                  :value="activePayloadSet.numberFrom"
                  type="number"
                  class="input input-bordered input-sm"
                  @input="updateActivePayloadSet({ numberFrom: Number(($event.target as HTMLInputElement).value) || 0 })"
                />
              </label>
              <label class="form-control">
                <span class="label-text text-xs">{{ $t('trafficAnalysis.intruder.labels.numberTo') }}</span>
                <input
                  :value="activePayloadSet.numberTo"
                  type="number"
                  class="input input-bordered input-sm"
                  @input="updateActivePayloadSet({ numberTo: Number(($event.target as HTMLInputElement).value) || 0 })"
                />
              </label>
              <label class="form-control">
                <span class="label-text text-xs">{{ $t('trafficAnalysis.intruder.labels.numberStep') }}</span>
                <input
                  :value="activePayloadSet.numberStep"
                  type="number"
                  min="1"
                  class="input input-bordered input-sm"
                  @input="updateActivePayloadSet({ numberStep: Math.max(1, Number(($event.target as HTMLInputElement).value) || 1) })"
                />
              </label>
              <label class="form-control">
                <span class="label-text text-xs">{{ $t('trafficAnalysis.intruder.labels.numberPadWidth') }}</span>
                <input
                  :value="activePayloadSet.numberPadWidth"
                  type="number"
                  min="0"
                  class="input input-bordered input-sm"
                  @input="updateActivePayloadSet({ numberPadWidth: Math.max(0, Number(($event.target as HTMLInputElement).value) || 0) })"
                />
              </label>
            </div>

            <div v-else-if="activePayloadSet?.payloadType === 'dates'" class="grid gap-3 p-4 md:grid-cols-2">
              <label class="form-control">
                <span class="label-text text-xs">{{ $t('trafficAnalysis.intruder.labels.dateFrom') }}</span>
                <input
                  :value="activePayloadSet.dateFrom"
                  type="date"
                  class="input input-bordered input-sm"
                  @input="updateActivePayloadSet({ dateFrom: ($event.target as HTMLInputElement).value })"
                />
              </label>
              <label class="form-control">
                <span class="label-text text-xs">{{ $t('trafficAnalysis.intruder.labels.dateTo') }}</span>
                <input
                  :value="activePayloadSet.dateTo"
                  type="date"
                  class="input input-bordered input-sm"
                  @input="updateActivePayloadSet({ dateTo: ($event.target as HTMLInputElement).value })"
                />
              </label>
              <label class="form-control">
                <span class="label-text text-xs">{{ $t('trafficAnalysis.intruder.labels.dateStepDays') }}</span>
                <input
                  :value="activePayloadSet.dateStepDays"
                  type="number"
                  min="1"
                  class="input input-bordered input-sm"
                  @input="updateActivePayloadSet({ dateStepDays: Math.max(1, Number(($event.target as HTMLInputElement).value) || 1) })"
                />
              </label>
              <label class="form-control">
                <span class="label-text text-xs">{{ $t('trafficAnalysis.intruder.labels.dateFormat') }}</span>
                <select
                  :value="activePayloadSet.dateFormat"
                  class="select select-bordered select-sm"
                  @change="updateActivePayloadSet({ dateFormat: ($event.target as HTMLSelectElement).value as IntruderPayloadSet['dateFormat'] })"
                >
                  <option value="yyyy-MM-dd">yyyy-MM-dd</option>
                  <option value="yyyyMMdd">yyyyMMdd</option>
                  <option value="MM/dd/yyyy">MM/dd/yyyy</option>
                </select>
              </label>
            </div>

            <div v-else-if="activePayloadSet?.payloadType === 'characterList'" class="grid gap-3 p-4">
              <label class="form-control">
                <span class="label-text text-xs">{{ $t('trafficAnalysis.intruder.labels.characters') }}</span>
                <textarea
                  :value="activePayloadSet.characterList"
                  class="h-40 w-full resize-none rounded-lg border border-base-300 bg-base-100 p-3 font-mono text-xs leading-6 outline-none transition focus:border-primary"
                  :placeholder="$t('trafficAnalysis.intruder.placeholders.characterList')"
                  @input="updateActivePayloadSet({ characterList: ($event.target as HTMLTextAreaElement).value })"
                ></textarea>
              </label>
            </div>

            <div v-else-if="activePayloadSet?.payloadType === 'nullPayloads'" class="grid gap-3 p-4 md:grid-cols-2">
              <label class="form-control">
                <span class="label-text text-xs">{{ $t('trafficAnalysis.intruder.labels.nullCount') }}</span>
                <input
                  :value="activePayloadSet.nullCount"
                  type="number"
                  min="0"
                  max="100000"
                  class="input input-bordered input-sm"
                  @input="updateActivePayloadSet({ nullCount: Math.max(0, Number(($event.target as HTMLInputElement).value) || 0) })"
                />
              </label>
              <label class="form-control">
                <span class="label-text text-xs">{{ $t('trafficAnalysis.intruder.labels.nullValue') }}</span>
                <input
                  :value="activePayloadSet.nullValue"
                  type="text"
                  class="input input-bordered input-sm"
                  :placeholder="$t('trafficAnalysis.intruder.placeholders.nullValue')"
                  @input="updateActivePayloadSet({ nullValue: ($event.target as HTMLInputElement).value })"
                />
              </label>
            </div>

            <div v-else-if="activePayloadSet?.payloadType === 'characterSubstitution'" class="grid gap-3 p-4">
              <label class="form-control">
                <span class="label-text text-xs">{{ $t('trafficAnalysis.intruder.labels.sourcePayloads') }}</span>
                <textarea
                  :value="activePayloadSet.substitutionSource"
                  class="h-32 w-full resize-none rounded-lg border border-base-300 bg-base-100 p-3 font-mono text-xs leading-6 outline-none transition focus:border-primary"
                  :placeholder="$t('trafficAnalysis.intruder.placeholders.substitutionSource')"
                  @input="updateActivePayloadSet({ substitutionSource: ($event.target as HTMLTextAreaElement).value })"
                ></textarea>
              </label>
              <label class="form-control">
                <span class="label-text text-xs">{{ $t('trafficAnalysis.intruder.labels.substitutionRules') }}</span>
                <textarea
                  :value="activePayloadSet.substitutionRules"
                  class="h-28 w-full resize-none rounded-lg border border-base-300 bg-base-100 p-3 font-mono text-xs leading-6 outline-none transition focus:border-primary"
                  :placeholder="$t('trafficAnalysis.intruder.placeholders.substitutionRules')"
                  @input="updateActivePayloadSet({ substitutionRules: ($event.target as HTMLTextAreaElement).value })"
                ></textarea>
              </label>
            </div>

            <div v-else-if="activePayloadSet?.payloadType === 'usernameGenerator'" class="grid gap-3 p-4">
              <label class="form-control">
                <span class="label-text text-xs">{{ $t('trafficAnalysis.intruder.labels.firstNames') }}</span>
                <textarea
                  :value="activePayloadSet.usernameFirstNames"
                  class="h-24 w-full resize-none rounded-lg border border-base-300 bg-base-100 p-3 font-mono text-xs leading-6 outline-none transition focus:border-primary"
                  :placeholder="$t('trafficAnalysis.intruder.placeholders.usernameFirstNames')"
                  @input="updateActivePayloadSet({ usernameFirstNames: ($event.target as HTMLTextAreaElement).value })"
                ></textarea>
              </label>
              <label class="form-control">
                <span class="label-text text-xs">{{ $t('trafficAnalysis.intruder.labels.lastNames') }}</span>
                <textarea
                  :value="activePayloadSet.usernameLastNames"
                  class="h-24 w-full resize-none rounded-lg border border-base-300 bg-base-100 p-3 font-mono text-xs leading-6 outline-none transition focus:border-primary"
                  :placeholder="$t('trafficAnalysis.intruder.placeholders.usernameLastNames')"
                  @input="updateActivePayloadSet({ usernameLastNames: ($event.target as HTMLTextAreaElement).value })"
                ></textarea>
              </label>
              <label class="form-control">
                <span class="label-text text-xs">{{ $t('trafficAnalysis.intruder.labels.usernameFormats') }}</span>
                <textarea
                  :value="activePayloadSet.usernameFormats"
                  class="h-24 w-full resize-none rounded-lg border border-base-300 bg-base-100 p-3 font-mono text-xs leading-6 outline-none transition focus:border-primary"
                  :placeholder="$t('trafficAnalysis.intruder.placeholders.usernameFormats')"
                  @input="updateActivePayloadSet({ usernameFormats: ($event.target as HTMLTextAreaElement).value })"
                ></textarea>
              </label>
            </div>
          </div>

          <div class="rounded-lg border border-base-300">
            <div class="border-b border-base-300 bg-base-200 px-4 py-2 text-xs font-semibold uppercase tracking-wide text-base-content/70">
              {{ $t('trafficAnalysis.intruder.labels.payloadEncoding') }}
            </div>
            <div class="space-y-3 p-4">
              <label class="flex items-center gap-2">
                <input
                  :checked="activePayloadSet?.urlEncode || false"
                  type="checkbox"
                  class="checkbox checkbox-sm"
                  @change="updateActivePayloadSet({ urlEncode: ($event.target as HTMLInputElement).checked })"
                />
                <span>{{ $t('trafficAnalysis.intruder.labels.urlEncodePayloads') }}</span>
              </label>
            </div>
          </div>

          <IntruderPayloadProcessingPanel
            :rules="payloadProcessingRules"
            @update:rules="$emit('update:payloadProcessingRules', $event)"
          />
        </div>
      </div>

      <div v-else-if="activeTab === 'resourcePool'" class="flex h-full min-h-0 flex-col">
        <div class="border-b border-base-300 px-4 py-3">
          <h3 class="text-sm font-semibold">{{ $t('trafficAnalysis.intruder.sections.resourcePool') }}</h3>
        </div>

        <div class="space-y-4 overflow-auto p-4 text-sm">
          <IntruderResourcePoolPanel
            :pools="resourcePoolPresets"
            :selected-pool-id="selectedResourcePoolId"
            :attack-options="{
              concurrency: attackOptions.concurrency,
              delayMs: attackOptions.delayMs,
              randomDelayMs: attackOptions.randomDelayMs,
            }"
            @select:pool="$emit('selectResourcePoolPreset', $event)"
            @upsert:pool="$emit('upsertResourcePool', $event)"
            @delete:pool="$emit('deleteResourcePool', $event)"
          />
        </div>
      </div>

      <div v-else class="flex h-full min-h-0 flex-col">
        <div class="border-b border-base-300 px-4 py-3">
          <h3 class="text-sm font-semibold">{{ $t('trafficAnalysis.intruder.sections.settings') }}</h3>
        </div>

        <div class="space-y-4 overflow-auto p-4 text-sm">
          <IntruderRequestHeadersPanel
            :attack-options="{
              updateContentLength: attackOptions.updateContentLength,
              setConnectionClose: attackOptions.setConnectionClose,
            }"
            @update:options="updateAttackOptions"
          />

          <IntruderErrorHandlingPanel
            :attack-options="{
              retryCount: attackOptions.retryCount,
              retryPauseMs: attackOptions.retryPauseMs,
              timeoutSecs: attackOptions.timeoutSecs,
              maxRequests: attackOptions.maxRequests,
            }"
            @update:options="updateAttackOptions"
          />

          <IntruderAttackResultsSettingsPanel
            :attack-options="{
              storeRequests: attackOptions.storeRequests,
              storeResponses: attackOptions.storeResponses,
              makeUnmodifiedBaseline: attackOptions.makeUnmodifiedBaseline,
              denialOfServiceMode: attackOptions.denialOfServiceMode,
              storeFullPayloads: attackOptions.storeFullPayloads,
            }"
            @update:options="updateAttackOptions"
          />

          <IntruderGrepMatchPanel
            :rules="grepMatchRules"
            @update:rules="$emit('update:grepMatchRules', $event)"
          />

          <IntruderGrepExtractPanel
            :rules="grepExtractRules"
            @update:rules="$emit('update:grepExtractRules', $event)"
          />

          <IntruderAutoPausePanel
            :enabled="attackOptions.autoPauseEnabled"
            :mode="attackOptions.autoPauseMode"
            :expressions="attackOptions.autoPauseExpressions"
            @update:enabled="updateOption('autoPauseEnabled', $event)"
            @update:mode="updateOption('autoPauseMode', $event)"
            @update:expressions="updateAutoPauseExpressions"
          />
        </div>
      </div>
    </div>

    <div class="flex w-12 flex-col border-l border-base-300 bg-base-200">
      <button
        class="flex-1 border-b border-base-300 px-1 text-xs font-medium tracking-wide transition"
        :class="activeTab === 'payloads' ? 'bg-base-100 text-primary' : 'text-base-content/70 hover:bg-base-300'"
        type="button"
        @click="$emit('update:activeTab', 'payloads')"
      >
        <span class="side-label">{{ $t('trafficAnalysis.intruder.sections.payloads') }}</span>
      </button>
      <button
        class="flex-1 border-b border-base-300 px-1 text-xs font-medium tracking-wide transition"
        :class="activeTab === 'resourcePool' ? 'bg-base-100 text-primary' : 'text-base-content/70 hover:bg-base-300'"
        type="button"
        @click="$emit('update:activeTab', 'resourcePool')"
      >
        <span class="side-label">{{ $t('trafficAnalysis.intruder.sections.resourcePool') }}</span>
      </button>
      <button
        class="flex-1 px-1 text-xs font-medium tracking-wide transition"
        :class="activeTab === 'settings' ? 'bg-base-100 text-primary' : 'text-base-content/70 hover:bg-base-300'"
        type="button"
        @click="$emit('update:activeTab', 'settings')"
      >
        <span class="side-label">{{ $t('trafficAnalysis.intruder.sections.settings') }}</span>
      </button>
    </div>
  </section>
</template>

<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { open } from '@tauri-apps/plugin-dialog'
import { readTextFile } from '@tauri-apps/plugin-fs'
import { dialog } from '@/composables/useDialog'
import IntruderAttackResultsSettingsPanel from './IntruderAttackResultsSettingsPanel.vue'
import IntruderAutoPausePanel from './IntruderAutoPausePanel.vue'
import IntruderErrorHandlingPanel from './IntruderErrorHandlingPanel.vue'
import IntruderGrepExtractPanel from './IntruderGrepExtractPanel.vue'
import IntruderGrepMatchPanel from './IntruderGrepMatchPanel.vue'
import IntruderPayloadProcessingPanel from './IntruderPayloadProcessingPanel.vue'
import IntruderRequestHeadersPanel from './IntruderRequestHeadersPanel.vue'
import IntruderResourcePoolPanel from './IntruderResourcePoolPanel.vue'
import { expandPayloadSet, parsePayloadLines } from './payloads'
import type {
  IntruderAttackOptions,
  IntruderGrepExtractRule,
  IntruderGrepMatchRule,
  IntruderPayloadProcessingRule,
  IntruderPayloadSet,
  IntruderResourcePool,
} from './types'

const props = defineProps<{
  activeTab: 'payloads' | 'resourcePool' | 'settings'
  payloadSets: IntruderPayloadSet[]
  payloadProcessingRules: IntruderPayloadProcessingRule[]
  grepMatchRules: IntruderGrepMatchRule[]
  grepExtractRules: IntruderGrepExtractRule[]
  resourcePoolPresets: IntruderResourcePool[]
  selectedResourcePoolId: string
  attackOptions: IntruderAttackOptions
  estimatedRequests: number
}>()

const emit = defineEmits<{
  (e: 'update:activeTab', value: 'payloads' | 'resourcePool' | 'settings'): void
  (e: 'update:attackOptions', value: IntruderAttackOptions): void
  (e: 'updatePayloadSet', id: string, patch: Partial<IntruderPayloadSet>): void
  (e: 'update:payloadProcessingRules', value: IntruderPayloadProcessingRule[]): void
  (e: 'update:grepMatchRules', value: IntruderGrepMatchRule[]): void
  (e: 'update:grepExtractRules', value: IntruderGrepExtractRule[]): void
  (e: 'selectResourcePoolPreset', presetId: string): void
  (e: 'upsertResourcePool', value: { id?: string; name: string; concurrency: number; delayMs: number; randomDelayMs: number }): void
  (e: 'deleteResourcePool', resourcePoolId: string): void
}>()

const selectedPayloadSetId = ref(props.payloadSets[0]?.id ?? '')

watch(
  () => props.payloadSets,
  (payloadSets) => {
    if (!payloadSets.some((payloadSet) => payloadSet.id === selectedPayloadSetId.value)) {
      selectedPayloadSetId.value = payloadSets[0]?.id ?? ''
    }
  },
  { deep: true },
)

const activePayloadSet = computed(() => props.payloadSets.find((payloadSet) => payloadSet.id === selectedPayloadSetId.value) ?? props.payloadSets[0] ?? null)
const activePayloadCount = computed(() => (activePayloadSet.value ? expandPayloadSet(activePayloadSet.value).length : 0))

function updateOption<K extends keyof IntruderAttackOptions>(key: K, value: IntruderAttackOptions[K]) {
  emit('update:attackOptions', {
    ...props.attackOptions,
    [key]: value,
  })
}

function updateAttackOptions(patch: Partial<IntruderAttackOptions>) {
  emit('update:attackOptions', {
    ...props.attackOptions,
    ...patch,
  })
}

function updateAutoPauseExpressions(expressions: string[]) {
  emit('update:attackOptions', {
    ...props.attackOptions,
    autoPauseExpressions: expressions,
    autoPauseExpression: expressions[0] ?? '',
  })
}

function updateActivePayloadSet(patch: Partial<IntruderPayloadSet>) {
  if (!activePayloadSet.value) return
  emit('updatePayloadSet', activePayloadSet.value.id, patch)
}

async function pastePayloads() {
  try {
    const text = await navigator.clipboard.readText()
    updateActivePayloadSet({ payloadsText: text })
  } catch {
    dialog.toast.error('Clipboard read failed')
  }
}

async function loadPayloadFile() {
  if (!activePayloadSet.value) return

  try {
    const selected = await open({
      multiple: false,
      directory: false,
      filters: [{ name: 'Text', extensions: ['txt', 'lst', 'csv', 'log'] }],
    })
    if (!selected || Array.isArray(selected)) return

    const content = await readTextFile(selected)
    updateActivePayloadSet({
      filePath: selected,
      payloadsText: content,
    })
  } catch (error) {
    console.error('Failed to load payload file', error)
    dialog.toast.error('Failed to load payload file')
  }
}

function clearPayloads() {
  updateActivePayloadSet({ payloadsText: '' })
}

function deduplicatePayloads() {
  if (!activePayloadSet.value || activePayloadSet.value.payloadType !== 'simpleList') return
  const deduped = Array.from(new Set(parsePayloadLines(activePayloadSet.value.payloadsText)))
  updateActivePayloadSet({ payloadsText: deduped.join('\n') })
}
</script>

<style scoped>
.side-label {
  display: inline-block;
  writing-mode: vertical-rl;
  transform: rotate(180deg);
  letter-spacing: 0.06em;
}
</style>
