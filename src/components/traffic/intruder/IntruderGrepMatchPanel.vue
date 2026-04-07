<template>
  <div class="rounded-lg border border-base-300">
    <div class="border-b border-base-300 bg-base-200 px-4 py-2 text-xs font-semibold uppercase tracking-wide text-base-content/70">
      {{ $t('trafficAnalysis.intruder.labels.grepMatch') }}
    </div>

    <div class="space-y-3 p-4">
      <p class="text-sm text-base-content/70">
        {{ $t('trafficAnalysis.intruder.help.grepMatchHint') }}
      </p>

      <div class="grid gap-3 lg:grid-cols-[8rem_minmax(0,1fr)]">
        <div class="space-y-2">
          <button class="btn btn-sm w-full" type="button" @click="openCreateDialog">
            {{ $t('trafficAnalysis.intruder.actions.add') }}
          </button>
          <button class="btn btn-sm btn-ghost w-full" type="button" :disabled="!selectedRule" @click="openEditDialog">
            {{ $t('trafficAnalysis.intruder.actions.edit') }}
          </button>
          <button class="btn btn-sm btn-ghost w-full" type="button" :disabled="!selectedRule" @click="removeSelectedRule">
            {{ $t('trafficAnalysis.intruder.actions.remove') }}
          </button>
          <button class="btn btn-sm btn-ghost w-full" type="button" :disabled="selectedRuleIndex <= 0" @click="moveSelectedRule('up')">
            {{ $t('trafficAnalysis.intruder.actions.moveUp') }}
          </button>
          <button class="btn btn-sm btn-ghost w-full" type="button" :disabled="selectedRuleIndex < 0 || selectedRuleIndex >= rules.length - 1" @click="moveSelectedRule('down')">
            {{ $t('trafficAnalysis.intruder.actions.moveDown') }}
          </button>
        </div>

        <div class="overflow-hidden rounded-lg border border-base-300">
          <table class="table table-sm">
            <thead class="bg-base-200">
              <tr>
                <th class="w-12"></th>
                <th class="w-28">{{ $t('trafficAnalysis.intruder.labels.enabled') }}</th>
                <th>{{ $t('trafficAnalysis.intruder.labels.rule') }}</th>
              </tr>
            </thead>
            <tbody>
              <tr
                v-for="rule in rules"
                :key="rule.id"
                class="cursor-pointer"
                :class="selectedRuleId === rule.id ? 'bg-primary/10' : ''"
                @click="selectedRuleId = rule.id"
                @dblclick="openEditDialog"
              >
                <td>
                  <input
                    :checked="rule.enabled"
                    type="checkbox"
                    class="checkbox checkbox-xs"
                    @click.stop
                    @change="toggleRuleEnabled(rule.id, ($event.target as HTMLInputElement).checked)"
                  />
                </td>
                <td>{{ rule.enabled ? $t('trafficAnalysis.intruder.labels.enabled') : '-' }}</td>
                <td class="max-w-0 truncate" :title="describeRule(rule)">
                  {{ describeRule(rule) }}
                </td>
              </tr>
              <tr v-if="!rules.length">
                <td colspan="3" class="py-10 text-center text-sm text-base-content/60">
                  {{ $t('trafficAnalysis.intruder.empty.noGrepMatchRules') }}
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>
    </div>

    <dialog ref="dialogRef" class="modal">
      <div class="modal-box max-w-2xl">
        <h3 class="mb-4 text-lg font-semibold">
          {{ editingMode === 'create' ? $t('trafficAnalysis.intruder.labels.createGrepMatchRule') : $t('trafficAnalysis.intruder.labels.editGrepMatchRule') }}
        </h3>

        <div class="grid gap-4">
          <label class="form-control">
            <span class="label-text">{{ $t('trafficAnalysis.intruder.placeholders.grepName') }}</span>
            <input v-model="draftRule.name" type="text" class="input input-bordered" />
          </label>
          <label class="form-control">
            <span class="label-text">{{ $t('trafficAnalysis.intruder.placeholders.grepPattern') }}</span>
            <input v-model="draftRule.pattern" type="text" class="input input-bordered" />
          </label>

          <div class="flex flex-wrap gap-4">
            <label class="form-control flex-1">
              <span class="label-text">{{ $t('trafficAnalysis.intruder.labels.patternType') }}</span>
              <select v-model="draftRule.patternType" class="select select-bordered">
                <option value="literal">{{ $t('trafficAnalysis.intruder.labels.literalPattern') }}</option>
                <option value="regex">{{ $t('trafficAnalysis.intruder.labels.regexPattern') }}</option>
              </select>
            </label>
          </div>

          <div class="flex flex-wrap gap-4">
            <label class="flex items-center gap-2">
              <input v-model="draftRule.enabled" type="checkbox" class="checkbox checkbox-sm" />
              <span>{{ $t('trafficAnalysis.intruder.labels.enabled') }}</span>
            </label>
            <label class="flex items-center gap-2">
              <input v-model="draftRule.caseSensitive" type="checkbox" class="checkbox checkbox-sm" />
              <span>{{ $t('trafficAnalysis.intruder.labels.caseSensitive') }}</span>
            </label>
            <label class="flex items-center gap-2">
              <input v-model="draftRule.invert" type="checkbox" class="checkbox checkbox-sm" />
              <span>{{ $t('trafficAnalysis.intruder.labels.invert') }}</span>
            </label>
            <label class="flex items-center gap-2">
              <input v-model="draftRule.excludeHeaders" type="checkbox" class="checkbox checkbox-sm" />
              <span>{{ $t('trafficAnalysis.intruder.labels.excludeResponseHeaders') }}</span>
            </label>
          </div>
        </div>

        <div class="mt-6 flex justify-end gap-2">
          <button class="btn btn-ghost" type="button" @click="closeDialog">
            {{ $t('common.cancel') }}
          </button>
          <button class="btn btn-primary" type="button" @click="saveRule">
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
import { createDefaultGrepMatchRule } from './analysis'
import type { IntruderGrepMatchRule } from './types'

const props = defineProps<{
  rules: IntruderGrepMatchRule[]
}>()

const emit = defineEmits<{
  (e: 'update:rules', value: IntruderGrepMatchRule[]): void
}>()

const { t } = useI18n()
const dialogRef = ref<HTMLDialogElement | null>(null)
const selectedRuleId = ref<string | null>(props.rules[0]?.id ?? null)
const editingMode = ref<'create' | 'edit'>('create')
const editingRuleId = ref<string | null>(null)
const draftRule = ref<IntruderGrepMatchRule>(createDefaultGrepMatchRule())

watch(
  () => props.rules,
  (rules) => {
    if (!rules.some((rule) => rule.id === selectedRuleId.value)) {
      selectedRuleId.value = rules[0]?.id ?? null
    }
  },
  { deep: true },
)

const selectedRuleIndex = computed(() => props.rules.findIndex((rule) => rule.id === selectedRuleId.value))
const selectedRule = computed(() => props.rules.find((rule) => rule.id === selectedRuleId.value) ?? null)

function updateRules(nextRules: IntruderGrepMatchRule[]) {
  emit('update:rules', nextRules)
}

function toggleRuleEnabled(ruleId: string, enabled: boolean) {
  updateRules(props.rules.map((rule) => (rule.id === ruleId ? { ...rule, enabled } : rule)))
}

function openCreateDialog() {
  editingMode.value = 'create'
  editingRuleId.value = null
  draftRule.value = createDefaultGrepMatchRule()
  dialogRef.value?.showModal()
}

function openEditDialog() {
  if (!selectedRule.value) return
  editingMode.value = 'edit'
  editingRuleId.value = selectedRule.value.id
  draftRule.value = { ...selectedRule.value }
  dialogRef.value?.showModal()
}

function closeDialog() {
  dialogRef.value?.close()
}

function saveRule() {
  const nextRule = { ...draftRule.value }
  if (editingMode.value === 'create') {
    updateRules([...props.rules, nextRule])
  } else if (editingRuleId.value) {
    updateRules(props.rules.map((rule) => (rule.id === editingRuleId.value ? nextRule : rule)))
  }
  selectedRuleId.value = nextRule.id
  closeDialog()
}

function removeSelectedRule() {
  if (!selectedRule.value) return
  const nextRules = props.rules.filter((rule) => rule.id !== selectedRule.value?.id)
  updateRules(nextRules)
  selectedRuleId.value = nextRules[Math.max(0, selectedRuleIndex.value - 1)]?.id ?? nextRules[0]?.id ?? null
}

function moveSelectedRule(direction: 'up' | 'down') {
  if (selectedRuleIndex.value < 0) return
  const targetIndex = direction === 'up' ? selectedRuleIndex.value - 1 : selectedRuleIndex.value + 1
  if (targetIndex < 0 || targetIndex >= props.rules.length) return

  const nextRules = [...props.rules]
  const [rule] = nextRules.splice(selectedRuleIndex.value, 1)
  nextRules.splice(targetIndex, 0, rule)
  updateRules(nextRules)
  selectedRuleId.value = rule.id
}

function describeRule(rule: IntruderGrepMatchRule): string {
  const fragments = [rule.name || t('trafficAnalysis.intruder.labels.grepMatch'), rule.pattern || '-']
  if (rule.patternType === 'regex') fragments.push(t('trafficAnalysis.intruder.labels.regexPattern'))
  if (rule.caseSensitive) fragments.push(t('trafficAnalysis.intruder.labels.caseSensitive'))
  if (rule.excludeHeaders) fragments.push(t('trafficAnalysis.intruder.labels.excludeResponseHeaders'))
  if (rule.invert) fragments.push(t('trafficAnalysis.intruder.labels.invert'))
  return fragments.join(' | ')
}
</script>
