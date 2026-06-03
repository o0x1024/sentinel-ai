<template>
  <div class="space-y-6 text-sm">
    <p class="text-sm text-base-content/70">
      {{ $t('trafficAnalysis.intruder.help.resourcePoolHint') }}
    </p>

    <section class="space-y-3">
      <label class="flex items-center gap-2">
        <input
          :checked="mode === 'existing'"
          type="radio"
          class="radio radio-sm"
          @change="switchToExistingMode"
        />
        <span>{{ $t('trafficAnalysis.intruder.labels.useExistingResourcePool') }}</span>
      </label>

      <div class="overflow-hidden border border-base-300 bg-base-100">
        <table class="w-full border-collapse text-left text-xs">
          <thead class="bg-base-200/80 text-base-content/70">
            <tr>
              <th class="border-b border-base-300 px-3 py-1.5 font-medium">{{ $t('trafficAnalysis.intruder.labels.selected') }}</th>
              <th class="border-b border-base-300 px-3 py-1.5 font-medium">{{ $t('trafficAnalysis.intruder.labels.resourcePool') }}</th>
              <th class="border-b border-base-300 px-3 py-1.5 font-medium">{{ $t('trafficAnalysis.intruder.labels.concurrentRequests') }}</th>
              <th class="border-b border-base-300 px-3 py-1.5 font-medium">{{ $t('trafficAnalysis.intruder.labels.requestDelay') }}</th>
              <th class="border-b border-base-300 px-3 py-1.5 font-medium">{{ $t('trafficAnalysis.intruder.labels.randomDelay') }}</th>
              <th class="border-b border-base-300 px-3 py-1.5 font-medium">{{ $t('trafficAnalysis.intruder.labels.delayIncrement') }}</th>
              <th class="border-b border-base-300 px-3 py-1.5 font-medium">{{ $t('trafficAnalysis.intruder.labels.autoThrottle') }}</th>
            </tr>
          </thead>
          <tbody>
            <tr
              v-for="pool in pools"
              :key="pool.id"
              class="cursor-pointer transition hover:bg-base-200/50"
              :class="mode === 'existing' && selectedPoolId === pool.id ? 'bg-base-200/60' : ''"
              @click="selectPool(pool.id)"
            >
              <td class="border-b border-base-300 px-3 py-1 align-middle">
                <input
                  :checked="mode === 'existing' && selectedPoolId === pool.id"
                  type="radio"
                  class="radio radio-xs"
                  @click.stop
                  @change="selectPool(pool.id)"
                />
              </td>
              <td class="border-b border-base-300 px-3 py-1 align-middle">{{ pool.name }}</td>
              <td class="border-b border-base-300 px-3 py-1 align-middle">{{ pool.concurrencyEnabled ? pool.concurrency : '' }}</td>
              <td class="border-b border-base-300 px-3 py-1 align-middle">{{ pool.delayEnabled ? `${pool.delayMs}` : '' }}</td>
              <td class="border-b border-base-300 px-3 py-1 align-middle">{{ pool.delayEnabled && pool.randomDelayEnabled ? `${pool.randomDelayMs}` : '' }}</td>
              <td class="border-b border-base-300 px-3 py-1 align-middle">{{ pool.delayEnabled && pool.delayIncrementEnabled ? `${pool.delayIncrementMs}` : '' }}</td>
              <td class="border-b border-base-300 px-3 py-1 align-middle">{{ pool.autoThrottleEnabled ? $t('trafficAnalysis.intruder.labels.yes') : '' }}</td>
            </tr>
          </tbody>
        </table>
      </div>
    </section>

    <section class="space-y-4">
      <label class="flex items-center gap-2">
        <input
          :checked="mode === 'create'"
          type="radio"
          class="radio radio-sm"
          @change="startCreateMode"
        />
        <span>{{ $t('trafficAnalysis.intruder.labels.createNewResourcePool') }}</span>
      </label>

      <div class="space-y-4">
        <label class="flex items-center gap-3">
          <span class="w-20 shrink-0">{{ $t('trafficAnalysis.intruder.labels.name') }}:</span>
          <input
            v-model="draft.name"
            type="text"
            class="input input-bordered input-sm flex-1"
            :placeholder="$t('trafficAnalysis.intruder.placeholders.resourcePoolName')"
          />
        </label>

        <div class="flex flex-wrap items-center gap-3">
          <label class="flex items-center gap-3">
            <input v-model="draft.concurrencyEnabled" type="checkbox" class="checkbox checkbox-sm" />
            <span>{{ $t('trafficAnalysis.intruder.labels.maximumConcurrentRequests') }}:</span>
          </label>
          <input
            v-model.number="draft.concurrency"
            type="number"
            min="1"
            max="999"
            class="input input-bordered input-sm w-24"
            :disabled="!draft.concurrencyEnabled"
          />
        </div>

        <div class="space-y-3">
          <div class="flex flex-wrap items-center gap-3">
            <label class="flex items-center gap-3">
              <input v-model="draft.delayEnabled" type="checkbox" class="checkbox checkbox-sm" />
              <span>{{ $t('trafficAnalysis.intruder.labels.delayBetweenRequests') }}:</span>
            </label>
            <input
              v-model.number="draft.delayMs"
              type="number"
              min="0"
              max="60000"
              class="input input-bordered input-sm w-24"
              :disabled="!draft.delayEnabled"
            />
            <span class="text-xs text-base-content/70">{{ $t('trafficAnalysis.intruder.labels.milliseconds') }}</span>
          </div>

          <div class="space-y-2 pl-8">
            <label class="flex items-center gap-2">
              <input
                :checked="delayMode === 'fixed'"
                type="radio"
                class="radio radio-xs"
                :disabled="!draft.delayEnabled"
                @change="setDelayMode('fixed')"
              />
              <span>{{ $t('trafficAnalysis.intruder.labels.fixed') }}</span>
            </label>

            <div class="flex flex-wrap items-center gap-3">
              <label class="flex items-center gap-3">
                <input
                  :checked="delayMode === 'random'"
                  type="radio"
                  class="radio radio-xs"
                  :disabled="!draft.delayEnabled"
                  @change="setDelayMode('random')"
                />
                <span>{{ $t('trafficAnalysis.intruder.labels.withRandomVariations') }}</span>
              </label>
              <input
                v-model.number="draft.randomDelayMs"
                type="number"
                min="0"
                max="60000"
                class="input input-bordered input-sm w-24"
                :disabled="!draft.delayEnabled || delayMode !== 'random'"
              />
              <span class="text-xs text-base-content/70">{{ $t('trafficAnalysis.intruder.labels.milliseconds') }}</span>
            </div>

            <div class="flex flex-wrap items-center gap-3">
              <label class="flex items-center gap-3">
                <input
                  :checked="delayMode === 'increment'"
                  type="radio"
                  class="radio radio-xs"
                  :disabled="!draft.delayEnabled"
                  @change="setDelayMode('increment')"
                />
                <span>{{ $t('trafficAnalysis.intruder.labels.increaseDelayInIncrementsOf') }}</span>
              </label>
              <input
                v-model.number="draft.delayIncrementMs"
                type="number"
                min="0"
                max="60000"
                class="input input-bordered input-sm w-24"
                :disabled="!draft.delayEnabled || delayMode !== 'increment'"
              />
              <span class="text-xs text-base-content/70">{{ $t('trafficAnalysis.intruder.labels.milliseconds') }}</span>
            </div>
          </div>
        </div>

        <div class="space-y-3">
          <label class="flex items-center gap-3">
            <input v-model="draft.autoThrottleEnabled" type="checkbox" class="checkbox checkbox-sm" />
            <span>{{ $t('trafficAnalysis.intruder.labels.automaticThrottling') }}</span>
          </label>

          <div class="space-y-2 pl-8">
            <label class="flex items-center gap-2">
              <input v-model="draftStatusSelections.use429" type="checkbox" class="checkbox checkbox-xs" :disabled="!draft.autoThrottleEnabled" />
              <span>429</span>
            </label>
            <label class="flex items-center gap-2">
              <input v-model="draftStatusSelections.use503" type="checkbox" class="checkbox checkbox-xs" :disabled="!draft.autoThrottleEnabled" />
              <span>503</span>
            </label>
            <label class="flex items-center gap-2">
              <input v-model="draftStatusSelections.useOther" type="checkbox" class="checkbox checkbox-xs" :disabled="!draft.autoThrottleEnabled" />
              <span>{{ $t('trafficAnalysis.intruder.labels.other') }}</span>
            </label>
            <div class="pl-6">
              <div class="text-xs text-base-content/70">{{ $t('trafficAnalysis.intruder.labels.csvFormatExample') }}</div>
              <input
                v-model="draftStatusSelections.otherStatusesText"
                type="text"
                class="input input-bordered input-sm mt-1 max-w-xs"
                :disabled="!draft.autoThrottleEnabled || !draftStatusSelections.useOther"
              />
            </div>
          </div>
        </div>

        <div v-if="mode === 'existing' && selectedCustomPool" class="flex flex-wrap items-center gap-2 pt-1">
          <button
            class="btn btn-ghost btn-sm text-error"
            type="button"
            @click="removeSelectedPool"
          >
            {{ $t('trafficAnalysis.intruder.actions.deleteResourcePool') }}
          </button>
        </div>
      </div>
    </section>
  </div>
</template>

<script setup lang="ts">
import { computed, reactive, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { buildIntruderResourcePoolAutoName } from './storage'
import type { IntruderAttackOptions, IntruderResourcePool } from './types'

interface ResourcePoolDraft {
  id?: string
  name: string
  concurrencyEnabled: boolean
  concurrency: number
  delayEnabled: boolean
  delayMs: number
  randomDelayEnabled: boolean
  randomDelayMs: number
  delayIncrementEnabled: boolean
  delayIncrementMs: number
  autoThrottleEnabled: boolean
  autoThrottleStatusCodes: number[]
}

const props = defineProps<{
  pools: IntruderResourcePool[]
  selectedPoolId: string
  attackOptions: Pick<
    IntruderAttackOptions,
    'concurrency' | 'delayMs' | 'randomDelayMs' | 'delayIncrementMs' | 'autoThrottleEnabled' | 'autoThrottleStatusCodes'
  >
}>()

const emit = defineEmits<{
  (e: 'select:pool', poolId: string): void
  (e: 'update:attack-options', value: Pick<
    IntruderAttackOptions,
    'concurrency' | 'delayMs' | 'randomDelayMs' | 'delayIncrementMs' | 'autoThrottleEnabled' | 'autoThrottleStatusCodes'
  >): void
  (e: 'upsert:pool', value: ResourcePoolDraft): void
  (e: 'delete:pool', poolId: string): void
}>()

useI18n()
const mode = ref<'existing' | 'create'>('existing')
const draftStatusSelections = reactive({
  use429: true,
  use503: true,
  useOther: false,
  otherStatusesText: '',
})
const lastAutoGeneratedName = ref('')
const draft = ref<ResourcePoolDraft>(createDraftFromAttackOptions())
const syncingDraft = ref(false)

const selectedPool = computed(() => props.pools.find((pool) => pool.id === props.selectedPoolId) ?? null)
const selectedCustomPool = computed(() => (selectedPool.value && !selectedPool.value.builtIn ? selectedPool.value : null))
const delayMode = computed<'fixed' | 'random' | 'increment'>(() => {
  if (!draft.value.delayEnabled) return 'fixed'
  if (draft.value.delayIncrementEnabled) return 'increment'
  if (draft.value.randomDelayEnabled) return 'random'
  return 'fixed'
})

watch(
  () => props.attackOptions,
  () => {
    if (mode.value === 'existing') {
      applyDraft(createDraftFromAttackOptions())
    }
  },
  { deep: true },
)

watch(
  () => selectedCustomPool.value,
  (pool) => {
    if (!pool || mode.value !== 'create') return
    if (!draft.value.name.trim() || pool.name.trim() !== draft.value.name.trim()) return
    if (draft.value.id === pool.id) return
    applyDraft({
      ...draft.value,
      id: pool.id,
    })
  },
  { deep: true },
)

watch(
  draft,
  () => {
    handleDraftMutation()
  },
  { deep: true, flush: 'sync' },
)

watch(
  draftStatusSelections,
  () => {
    handleDraftMutation()
  },
  { deep: true, flush: 'sync' },
)

function getNormalizedAttackOptions() {
  return {
    concurrency: Math.max(1, Number(props.attackOptions?.concurrency) || 1),
    delayMs: Math.max(0, Number(props.attackOptions?.delayMs) || 0),
    randomDelayMs: Math.max(0, Number(props.attackOptions?.randomDelayMs) || 0),
    delayIncrementMs: Math.max(0, Number(props.attackOptions?.delayIncrementMs) || 0),
    autoThrottleEnabled: Boolean(props.attackOptions?.autoThrottleEnabled),
    autoThrottleStatusCodes: Array.isArray(props.attackOptions?.autoThrottleStatusCodes)
      ? props.attackOptions.autoThrottleStatusCodes
      : [],
  }
}

function createDraftFromAttackOptions(): ResourcePoolDraft {
  const attackOptions = getNormalizedAttackOptions()
  const nextDraft = {
    name: buildAutoGeneratedDraftName(undefined),
    concurrencyEnabled: true,
    concurrency: attackOptions.concurrency,
    delayEnabled: true,
    delayMs: attackOptions.delayMs,
    randomDelayEnabled: attackOptions.randomDelayMs > 0,
    randomDelayMs: attackOptions.randomDelayMs,
    delayIncrementEnabled: attackOptions.delayIncrementMs > 0,
    delayIncrementMs: attackOptions.delayIncrementMs,
    autoThrottleEnabled: attackOptions.autoThrottleEnabled,
    autoThrottleStatusCodes: [...attackOptions.autoThrottleStatusCodes],
  }
  lastAutoGeneratedName.value = buildAutoGeneratedDraftName(undefined)
  return nextDraft
}

function applyDraft(nextDraft: ResourcePoolDraft) {
  syncingDraft.value = true
  draft.value = nextDraft
  syncStatusSelectionsFromCodes(nextDraft.autoThrottleStatusCodes)
  syncingDraft.value = false
}

function syncStatusSelectionsFromCodes(codes: number[]) {
  const normalized = new Set(codes)
  draftStatusSelections.use429 = normalized.has(429)
  draftStatusSelections.use503 = normalized.has(503)
  const otherCodes = Array.from(normalized).filter((code) => code !== 429 && code !== 503)
  draftStatusSelections.useOther = otherCodes.length > 0
  draftStatusSelections.otherStatusesText = otherCodes.join(',')
}

function parseOtherStatusCodes(input: string): number[] {
  return input
    .split(',')
    .map((value) => Number.parseInt(value.trim(), 10))
    .filter((value) => Number.isInteger(value) && value >= 100 && value <= 999 && value !== 429 && value !== 503)
}

function buildSelectedStatusCodes(): number[] {
  const codes = new Set<number>()
  if (draftStatusSelections.use429) codes.add(429)
  if (draftStatusSelections.use503) codes.add(503)
  if (draftStatusSelections.useOther) {
    parseOtherStatusCodes(draftStatusSelections.otherStatusesText).forEach((code) => codes.add(code))
  }
  return Array.from(codes).sort((left, right) => left - right)
}

function buildAttackOptionsFromDraft() {
  const autoThrottleStatusCodes = draft.value.autoThrottleEnabled ? buildSelectedStatusCodes() : []
  return {
    concurrency: Math.min(999, Math.max(1, Number(draft.value.concurrency) || 1)),
    delayMs: draft.value.delayEnabled ? Math.min(60000, Math.max(0, Number(draft.value.delayMs) || 0)) : 0,
    randomDelayMs: draft.value.delayEnabled && draft.value.randomDelayEnabled
      ? Math.min(60000, Math.max(0, Number(draft.value.randomDelayMs) || 0))
      : 0,
    delayIncrementMs: draft.value.delayEnabled && draft.value.delayIncrementEnabled
      ? Math.min(60000, Math.max(0, Number(draft.value.delayIncrementMs) || 0))
      : 0,
    autoThrottleEnabled: draft.value.autoThrottleEnabled,
    autoThrottleStatusCodes,
  }
}

function buildPoolDraftForPersist(): ResourcePoolDraft {
  const attackOptions = buildAttackOptionsFromDraft()
  return {
    id: draft.value.id,
    name: draft.value.name.trim() || buildAutoGeneratedDraftName(draft.value.id),
    concurrencyEnabled: draft.value.concurrencyEnabled,
    concurrency: attackOptions.concurrency,
    delayEnabled: draft.value.delayEnabled,
    delayMs: attackOptions.delayMs,
    randomDelayEnabled: draft.value.delayEnabled && draft.value.randomDelayEnabled,
    randomDelayMs: attackOptions.randomDelayMs,
    delayIncrementEnabled: draft.value.delayEnabled && draft.value.delayIncrementEnabled,
    delayIncrementMs: attackOptions.delayIncrementMs,
    autoThrottleEnabled: attackOptions.autoThrottleEnabled,
    autoThrottleStatusCodes: attackOptions.autoThrottleStatusCodes,
  }
}

function buildAutoGeneratedDraftName(currentPoolId?: string): string {
  return buildIntruderResourcePoolAutoName(props.pools, currentPoolId)
}

function syncAutoGeneratedDraftName() {
  const nextAutoName = buildAutoGeneratedDraftName(draft.value.id)
  const currentName = draft.value.name.trim()
  const shouldReplace = !currentName || currentName === lastAutoGeneratedName.value
  lastAutoGeneratedName.value = nextAutoName

  if (!shouldReplace || currentName === nextAutoName) {
    return
  }

  syncingDraft.value = true
  draft.value = {
    ...draft.value,
    name: nextAutoName,
  }
  syncingDraft.value = false
}

function startCreateMode() {
  if (mode.value === 'create') return
  mode.value = 'create'
  const nextDraft = {
    ...draft.value,
    id: undefined,
    name: buildAutoGeneratedDraftName(undefined),
  }
  lastAutoGeneratedName.value = nextDraft.name
  applyDraft({
    ...nextDraft,
  })
}

function switchToExistingMode() {
  mode.value = 'existing'
  if (selectedPool.value) {
    emit('select:pool', selectedPool.value.id)
    return
  }
  applyDraft(createDraftFromAttackOptions())
}

function handleDraftMutation() {
  if (syncingDraft.value) return
  if (mode.value !== 'create') {
    startCreateMode()
  }

  syncAutoGeneratedDraftName()

  emit('update:attack-options', buildAttackOptionsFromDraft())

  const nextPool = buildPoolDraftForPersist()
  emit('upsert:pool', nextPool)
}

function setDelayMode(value: 'fixed' | 'random' | 'increment') {
  draft.value = {
    ...draft.value,
    randomDelayEnabled: value === 'random',
    delayIncrementEnabled: value === 'increment',
    randomDelayMs: value === 'random' ? Math.max(0, draft.value.randomDelayMs || 0) : 0,
    delayIncrementMs: value === 'increment' ? Math.max(0, draft.value.delayIncrementMs || 0) : 0,
  }
}

function selectPool(poolId: string) {
  mode.value = 'existing'
  emit('select:pool', poolId)
}

function removeSelectedPool() {
  if (!selectedCustomPool.value) return
  emit('delete:pool', selectedCustomPool.value.id)
  applyDraft(createDraftFromAttackOptions())
  mode.value = 'existing'
}
</script>

<style scoped>
table tbody tr {
  height: 1.9rem;
}
</style>
