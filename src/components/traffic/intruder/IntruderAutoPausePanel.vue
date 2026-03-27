<template>
  <div class="rounded-lg border border-base-300">
    <div class="border-b border-base-300 bg-base-200 px-4 py-2 text-xs font-semibold uppercase tracking-wide text-base-content/70">
      {{ $t('trafficAnalysis.intruder.labels.autoPauseAttack') }}
    </div>

    <div class="space-y-3 p-4">
      <div class="flex flex-wrap items-center gap-4">
        <label class="flex items-center gap-2">
          <input
            :checked="enabled"
            type="checkbox"
            class="checkbox checkbox-sm"
            @change="$emit('update:enabled', ($event.target as HTMLInputElement).checked)"
          />
          <span>{{ $t('trafficAnalysis.intruder.labels.enableAutoPause') }}</span>
        </label>

        <select
          :value="mode"
          class="select select-bordered select-sm"
          :disabled="!enabled"
          @change="$emit('update:mode', ($event.target as HTMLSelectElement).value as 'contains' | 'missing')"
        >
          <option value="contains">{{ $t('trafficAnalysis.intruder.labels.pauseIfContains') }}</option>
          <option value="missing">{{ $t('trafficAnalysis.intruder.labels.pauseIfMissing') }}</option>
        </select>
      </div>

      <p class="text-sm text-base-content/70">
        {{ $t('trafficAnalysis.intruder.help.autoPauseHint') }}
      </p>

      <div class="grid gap-3 lg:grid-cols-[8rem_minmax(0,1fr)]">
        <div class="space-y-2">
          <button class="btn btn-sm w-full" type="button" :disabled="!enabled" @click="openCreateDialog">
            {{ $t('trafficAnalysis.intruder.actions.add') }}
          </button>
          <button class="btn btn-sm btn-ghost w-full" type="button" :disabled="!enabled || !selectedExpression" @click="openEditDialog">
            {{ $t('trafficAnalysis.intruder.actions.edit') }}
          </button>
          <button class="btn btn-sm btn-ghost w-full" type="button" :disabled="!enabled || !selectedExpression" @click="removeSelectedExpression">
            {{ $t('trafficAnalysis.intruder.actions.remove') }}
          </button>
          <button class="btn btn-sm btn-ghost w-full" type="button" :disabled="!enabled || selectedIndex <= 0" @click="moveSelectedExpression('up')">
            {{ $t('trafficAnalysis.intruder.actions.moveUp') }}
          </button>
          <button class="btn btn-sm btn-ghost w-full" type="button" :disabled="!enabled || selectedIndex < 0 || selectedIndex >= expressions.length - 1" @click="moveSelectedExpression('down')">
            {{ $t('trafficAnalysis.intruder.actions.moveDown') }}
          </button>
        </div>

        <div class="overflow-hidden rounded-lg border border-base-300">
          <table class="table table-sm">
            <thead class="bg-base-200">
              <tr>
                <th>{{ $t('trafficAnalysis.intruder.labels.expression') }}</th>
              </tr>
            </thead>
            <tbody>
              <tr
                v-for="expression in expressions"
                :key="expression"
                class="cursor-pointer"
                :class="selectedExpression === expression ? 'bg-primary/10' : ''"
                @click="selectedExpression = expression"
                @dblclick="enabled ? openEditDialog() : undefined"
              >
                <td class="max-w-0 truncate" :title="expression">{{ expression }}</td>
              </tr>
              <tr v-if="!expressions.length">
                <td class="py-10 text-center text-sm text-base-content/60">
                  {{ $t('trafficAnalysis.intruder.empty.noAutoPauseExpressions') }}
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>
    </div>

    <dialog ref="dialogRef" class="modal">
      <div class="modal-box max-w-xl">
        <h3 class="mb-4 text-lg font-semibold">
          {{ editingMode === 'create' ? $t('trafficAnalysis.intruder.labels.createAutoPauseExpression') : $t('trafficAnalysis.intruder.labels.editAutoPauseExpression') }}
        </h3>

        <label class="form-control">
          <span class="label-text">{{ $t('trafficAnalysis.intruder.labels.expression') }}</span>
          <input v-model="draftExpression" type="text" class="input input-bordered" :placeholder="$t('trafficAnalysis.intruder.placeholders.autoPauseExpression')" />
        </label>

        <div class="mt-6 flex justify-end gap-2">
          <button class="btn btn-ghost" type="button" @click="closeDialog">
            {{ $t('common.cancel') }}
          </button>
          <button class="btn btn-primary" type="button" @click="saveExpression">
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
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'

const props = defineProps<{
  enabled: boolean
  mode: 'contains' | 'missing'
  expressions: string[]
}>()

const emit = defineEmits<{
  (e: 'update:enabled', value: boolean): void
  (e: 'update:mode', value: 'contains' | 'missing'): void
  (e: 'update:expressions', value: string[]): void
}>()

useI18n()

const dialogRef = ref<HTMLDialogElement | null>(null)
const selectedExpression = ref<string | null>(props.expressions[0] ?? null)
const editingMode = ref<'create' | 'edit'>('create')
const originalExpression = ref<string | null>(null)
const draftExpression = ref('')

watch(
  () => props.expressions,
  (expressions) => {
    if (!expressions.includes(selectedExpression.value || '')) {
      selectedExpression.value = expressions[0] ?? null
    }
  },
)

const selectedIndex = computed(() => props.expressions.findIndex((expression) => expression === selectedExpression.value))

function openCreateDialog() {
  editingMode.value = 'create'
  originalExpression.value = null
  draftExpression.value = ''
  dialogRef.value?.showModal()
}

function openEditDialog() {
  if (!selectedExpression.value) return
  editingMode.value = 'edit'
  originalExpression.value = selectedExpression.value
  draftExpression.value = selectedExpression.value
  dialogRef.value?.showModal()
}

function closeDialog() {
  dialogRef.value?.close()
}

function saveExpression() {
  const value = draftExpression.value.trim()
  if (!value) return

  if (editingMode.value === 'create') {
    const nextExpressions = Array.from(new Set([...props.expressions, value]))
    emit('update:expressions', nextExpressions)
    selectedExpression.value = value
  } else if (originalExpression.value) {
    const nextExpressions = props.expressions.map((expression) => (expression === originalExpression.value ? value : expression))
    emit('update:expressions', Array.from(new Set(nextExpressions)))
    selectedExpression.value = value
  }

  closeDialog()
}

function removeSelectedExpression() {
  if (!selectedExpression.value) return
  const nextExpressions = props.expressions.filter((expression) => expression !== selectedExpression.value)
  emit('update:expressions', nextExpressions)
  selectedExpression.value = nextExpressions[Math.max(0, selectedIndex.value - 1)] ?? nextExpressions[0] ?? null
}

function moveSelectedExpression(direction: 'up' | 'down') {
  if (selectedIndex.value < 0) return
  const targetIndex = direction === 'up' ? selectedIndex.value - 1 : selectedIndex.value + 1
  if (targetIndex < 0 || targetIndex >= props.expressions.length) return

  const nextExpressions = [...props.expressions]
  const [expression] = nextExpressions.splice(selectedIndex.value, 1)
  nextExpressions.splice(targetIndex, 0, expression)
  emit('update:expressions', nextExpressions)
  selectedExpression.value = expression
}
</script>
