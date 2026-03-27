<template>
  <div class="space-y-4">
    <div class="rounded-lg border border-base-300">
      <div class="border-b border-base-300 bg-base-200 px-4 py-2 text-xs font-semibold uppercase tracking-wide text-base-content/70">
        {{ $t('trafficAnalysis.intruder.labels.resourcePool') }}
      </div>

      <div class="space-y-3 p-4">
        <p class="text-sm text-base-content/70">
          {{ $t('trafficAnalysis.intruder.help.resourcePoolHint') }}
        </p>

        <div class="grid gap-3 lg:grid-cols-[8rem_minmax(0,1fr)]">
          <div class="space-y-2">
            <button class="btn btn-sm w-full" type="button" @click="openCreateDialog">
              {{ $t('trafficAnalysis.intruder.actions.add') }}
            </button>
            <button class="btn btn-sm btn-ghost w-full" type="button" :disabled="!selectedCustomPool" @click="openEditDialog">
              {{ $t('trafficAnalysis.intruder.actions.edit') }}
            </button>
            <button class="btn btn-sm btn-ghost w-full" type="button" :disabled="!selectedCustomPool" @click="removeSelectedPool">
              {{ $t('trafficAnalysis.intruder.actions.remove') }}
            </button>
          </div>

          <div class="overflow-hidden rounded-lg border border-base-300">
            <table class="table table-sm">
              <thead class="bg-base-200">
                <tr>
                  <th class="w-12"></th>
                  <th>{{ $t('trafficAnalysis.intruder.labels.resourcePool') }}</th>
                  <th>{{ $t('trafficAnalysis.intruder.labels.concurrentRequests') }}</th>
                  <th>{{ $t('trafficAnalysis.intruder.labels.requestDelay') }}</th>
                  <th>{{ $t('trafficAnalysis.intruder.labels.randomDelay') }}</th>
                </tr>
              </thead>
              <tbody>
                <tr
                  v-for="pool in pools"
                  :key="pool.id"
                  class="cursor-pointer"
                  :class="selectedPoolId === pool.id ? 'bg-primary/10' : ''"
                  @click="selectPool(pool.id)"
                  @dblclick="pool.builtIn ? undefined : openEditDialog()"
                >
                  <td>
                    <input
                      :checked="selectedPoolId === pool.id"
                      type="radio"
                      class="radio radio-xs"
                      @click.stop
                      @change="selectPool(pool.id)"
                    />
                  </td>
                  <td>
                    <div class="flex items-center gap-2">
                      <span>{{ pool.name }}</span>
                      <span v-if="pool.builtIn" class="badge badge-outline badge-xs">{{ $t('trafficAnalysis.intruder.labels.builtIn') }}</span>
                    </div>
                  </td>
                  <td>{{ pool.concurrency }}</td>
                  <td>{{ pool.delayMs }} ms</td>
                  <td>{{ pool.randomDelayMs }} ms</td>
                </tr>
              </tbody>
            </table>
          </div>
        </div>
      </div>
    </div>

    <dialog ref="dialogRef" class="modal">
      <div class="modal-box max-w-lg">
        <h3 class="mb-4 text-lg font-semibold">
          {{ editingMode === 'create' ? $t('trafficAnalysis.intruder.labels.createResourcePool') : $t('trafficAnalysis.intruder.labels.editResourcePool') }}
        </h3>

        <div class="grid gap-4">
          <label class="form-control">
            <span class="label-text">{{ $t('trafficAnalysis.intruder.labels.resourcePool') }}</span>
            <input v-model="draftPool.name" type="text" class="input input-bordered" :placeholder="$t('trafficAnalysis.intruder.placeholders.resourcePoolName')" />
          </label>

          <label class="form-control">
            <span class="label-text">{{ $t('trafficAnalysis.intruder.labels.concurrentRequests') }}</span>
            <input v-model.number="draftPool.concurrency" type="number" min="1" max="20" class="input input-bordered" />
          </label>

          <label class="form-control">
            <span class="label-text">{{ $t('trafficAnalysis.intruder.labels.requestDelay') }}</span>
            <input v-model.number="draftPool.delayMs" type="number" min="0" max="10000" class="input input-bordered" />
          </label>

          <label class="form-control">
            <span class="label-text">{{ $t('trafficAnalysis.intruder.labels.randomDelay') }}</span>
            <input v-model.number="draftPool.randomDelayMs" type="number" min="0" max="10000" class="input input-bordered" />
          </label>
        </div>

        <div class="mt-6 flex justify-end gap-2">
          <button class="btn btn-ghost" type="button" @click="closeDialog">
            {{ $t('common.cancel') }}
          </button>
          <button class="btn btn-primary" type="button" @click="savePool">
            {{ $t('common.save') }}
          </button>
        </div>
      </div>

      <form method="dialog" class="modal-backdrop">
        <button>{{ $t('common.close') }}</button>
      </form>
    </dialog>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import type { IntruderAttackOptions, IntruderResourcePool } from './types'

interface ResourcePoolDraft {
  id?: string
  name: string
  concurrency: number
  delayMs: number
  randomDelayMs: number
}

const props = defineProps<{
  pools: IntruderResourcePool[]
  selectedPoolId: string
  attackOptions: Pick<IntruderAttackOptions, 'concurrency' | 'delayMs' | 'randomDelayMs'>
}>()

const emit = defineEmits<{
  (e: 'select:pool', poolId: string): void
  (e: 'upsert:pool', value: ResourcePoolDraft): void
  (e: 'delete:pool', poolId: string): void
}>()

const { t } = useI18n()
const dialogRef = ref<HTMLDialogElement | null>(null)
const editingMode = ref<'create' | 'edit'>('create')
const draftPool = ref<ResourcePoolDraft>(createDraftFromAttackOptions())

const selectedPool = computed(() => props.pools.find((pool) => pool.id === props.selectedPoolId) ?? null)
const selectedCustomPool = computed(() => (selectedPool.value && !selectedPool.value.builtIn ? selectedPool.value : null))

function createDraftFromAttackOptions(): ResourcePoolDraft {
  return {
    name: '',
    concurrency: props.attackOptions.concurrency,
    delayMs: props.attackOptions.delayMs,
    randomDelayMs: props.attackOptions.randomDelayMs,
  }
}

function selectPool(poolId: string) {
  emit('select:pool', poolId)
}

function openCreateDialog() {
  editingMode.value = 'create'
  draftPool.value = createDraftFromAttackOptions()
  dialogRef.value?.showModal()
}

function openEditDialog() {
  if (!selectedCustomPool.value) return
  editingMode.value = 'edit'
  draftPool.value = {
    id: selectedCustomPool.value.id,
    name: selectedCustomPool.value.name,
    concurrency: selectedCustomPool.value.concurrency,
    delayMs: selectedCustomPool.value.delayMs,
    randomDelayMs: selectedCustomPool.value.randomDelayMs,
  }
  dialogRef.value?.showModal()
}

function closeDialog() {
  dialogRef.value?.close()
}

function savePool() {
  emit('upsert:pool', {
    id: draftPool.value.id,
    name: draftPool.value.name.trim(),
    concurrency: Math.min(20, Math.max(1, draftPool.value.concurrency || 1)),
    delayMs: Math.min(10000, Math.max(0, draftPool.value.delayMs || 0)),
    randomDelayMs: Math.min(10000, Math.max(0, draftPool.value.randomDelayMs || 0)),
  })
  closeDialog()
}

function removeSelectedPool() {
  if (!selectedCustomPool.value) return
  emit('delete:pool', selectedCustomPool.value.id)
}
</script>
