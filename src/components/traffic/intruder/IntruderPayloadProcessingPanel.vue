<template>
  <div class="rounded-lg border border-base-300">
    <div class="border-b border-base-300 bg-base-200 px-4 py-2 text-xs font-semibold uppercase tracking-wide text-base-content/70">
      {{ $t('trafficAnalysis.intruder.labels.payloadProcessing') }}
    </div>

    <div class="space-y-3 p-4">
      <p class="text-sm text-base-content/70">
        {{ $t('trafficAnalysis.intruder.help.payloadProcessingHint') }}
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
                <th class="w-12">
                  <span class="sr-only">{{ $t('trafficAnalysis.intruder.labels.enabled') }}</span>
                </th>
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
                <td class="max-w-0 truncate text-sm" :title="describeRule(rule)">
                  {{ describeRule(rule) }}
                </td>
              </tr>
              <tr v-if="!rules.length">
                <td colspan="3" class="py-10 text-center text-sm text-base-content/60">
                  {{ $t('trafficAnalysis.intruder.empty.noPayloadProcessingRules') }}
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>
    </div>

    <AppDialog ref="dialogRef" class="modal">
      <div class="modal-box max-w-2xl">
        <h3 class="mb-4 text-lg font-semibold">
          {{ editingMode === 'create' ? $t('trafficAnalysis.intruder.labels.createRule') : $t('trafficAnalysis.intruder.labels.editRule') }}
        </h3>

        <div class="grid gap-4">
          <label class="form-control">
            <span class="label-text">{{ $t('trafficAnalysis.intruder.labels.rule') }}</span>
            <select v-model="draftRule.type" class="select select-bordered">
              <option v-for="option in ruleTypeOptions" :key="option.value" :value="option.value">
                {{ option.label }}
              </option>
            </select>
          </label>

          <div v-if="draftRule.type === 'prefix' || draftRule.type === 'suffix' || draftRule.type === 'repeat'" class="grid gap-4">
            <label class="form-control">
              <span class="label-text">{{ $t('trafficAnalysis.intruder.placeholders.ruleValue') }}</span>
              <input v-model="draftRule.replaceValue" type="text" class="input input-bordered" />
            </label>
          </div>

          <div v-else-if="draftRule.type === 'replace' || draftRule.type === 'replaceRegex'" class="grid gap-4 md:grid-cols-2">
            <label class="form-control">
              <span class="label-text">{{ $t('trafficAnalysis.intruder.placeholders.ruleMatchValue') }}</span>
              <input v-model="draftRule.matchValue" type="text" class="input input-bordered" />
            </label>
            <label class="form-control">
              <span class="label-text">{{ $t('trafficAnalysis.intruder.placeholders.ruleReplaceValue') }}</span>
              <input v-model="draftRule.replaceValue" type="text" class="input input-bordered" />
            </label>
          </div>

          <div v-else-if="draftRule.type === 'substring' || draftRule.type === 'reverseSubstring'" class="grid gap-4 md:grid-cols-2">
            <label class="form-control">
              <span class="label-text">
                {{
                  draftRule.type === 'reverseSubstring'
                    ? $t('trafficAnalysis.intruder.labels.reverseSubstringEndOffset')
                    : $t('trafficAnalysis.intruder.labels.substringStart')
                }}
              </span>
              <input
                :value="draftRule.substringStart ?? 0"
                type="number"
                min="0"
                class="input input-bordered"
                @input="updateSubstringStart(($event.target as HTMLInputElement).value)"
              />
            </label>
            <label class="form-control">
              <span class="label-text">
                {{
                  draftRule.type === 'reverseSubstring'
                    ? $t('trafficAnalysis.intruder.labels.reverseSubstringLength')
                    : $t('trafficAnalysis.intruder.labels.substringLength')
                }}
              </span>
              <input
                :value="draftRule.substringLength ?? ''"
                type="number"
                min="0"
                class="input input-bordered"
                :placeholder="
                  draftRule.type === 'reverseSubstring'
                    ? $t('trafficAnalysis.intruder.placeholders.reverseSubstringLength')
                    : $t('trafficAnalysis.intruder.placeholders.substringLength')
                "
                @input="updateSubstringLength(($event.target as HTMLInputElement).value)"
              />
            </label>
          </div>

          <div v-else-if="draftRule.type === 'decode'" class="grid gap-4">
            <label class="form-control">
              <span class="label-text">{{ $t('trafficAnalysis.intruder.labels.decodeType') }}</span>
              <select v-model="draftRule.codecType" class="select select-bordered">
                <option v-for="option in codecOptions" :key="option.value" :value="option.value">
                  {{ option.label }}
                </option>
              </select>
            </label>
          </div>

          <div v-else-if="draftRule.type === 'hash'" class="grid gap-4">
            <label class="form-control">
              <span class="label-text">{{ $t('trafficAnalysis.intruder.labels.hashAlgorithm') }}</span>
              <select v-model="draftRule.hashAlgorithm" class="select select-bordered">
                <option v-for="option in hashOptions" :key="option.value" :value="option.value">
                  {{ option.label }}
                </option>
              </select>
            </label>
          </div>

          <div v-else-if="draftRule.type === 'addRawPayload'" class="grid gap-4">
            <label class="form-control">
              <span class="label-text">{{ $t('trafficAnalysis.intruder.labels.rawPayloadPlacement') }}</span>
              <select v-model="draftRule.rawPayloadPlacement" class="select select-bordered">
                <option v-for="option in rawPayloadPlacementOptions" :key="option.value" :value="option.value">
                  {{ option.label }}
                </option>
              </select>
            </label>
          </div>

          <div v-else-if="draftRule.type === 'replaceBaseValue'" class="grid gap-4">
            <p class="text-sm text-base-content/70">
              {{ $t('trafficAnalysis.intruder.help.replaceBaseValueHint') }}
            </p>
          </div>

          <div v-else-if="draftRule.type === 'skipRegex'" class="grid gap-4">
            <label class="form-control">
              <span class="label-text">{{ $t('trafficAnalysis.intruder.labels.skipRegexPattern') }}</span>
              <input v-model="draftRule.matchValue" type="text" class="input input-bordered" />
            </label>
          </div>

          <div class="grid gap-4 md:grid-cols-[12rem_1fr]">
            <label class="form-control">
              <span class="label-text">{{ $t('trafficAnalysis.intruder.labels.condition') }}</span>
              <select v-model="draftConditionType" class="select select-bordered">
                <option v-for="option in conditionOptions" :key="option.value" :value="option.value">
                  {{ option.label }}
                </option>
              </select>
            </label>

            <label class="form-control">
              <span class="label-text">{{ $t('trafficAnalysis.intruder.placeholders.conditionValue') }}</span>
              <input
                v-model="draftRule.conditionValue"
                type="text"
                class="input input-bordered"
                :disabled="draftConditionType === 'always'"
              />
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
    </AppDialog>
  </div>
</template>

<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import {
  createDefaultPayloadProcessingRule,
  getPayloadProcessingCodecLabel,
  getPayloadProcessingConditionLabel,
  getPayloadProcessingHashLabel,
  getPayloadProcessingRawPayloadPlacementLabel,
  getPayloadProcessingRuleLabel,
} from './payloadProcessing'
import type {
  IntruderPayloadProcessingCodec,
  IntruderPayloadProcessingConditionType,
  IntruderPayloadProcessingHashAlgorithm,
  IntruderPayloadProcessingRawPayloadPlacement,
  IntruderPayloadProcessingRule,
  IntruderPayloadProcessingRuleType,
} from './types'

const props = defineProps<{
  rules: IntruderPayloadProcessingRule[]
}>()

const emit = defineEmits<{
  (e: 'update:rules', value: IntruderPayloadProcessingRule[]): void
}>()

const { t } = useI18n()
const dialogRef = ref<HTMLDialogElement | null>(null)
const selectedRuleId = ref<string | null>(props.rules[0]?.id ?? null)
const editingMode = ref<'create' | 'edit'>('create')
const editingRuleId = ref<string | null>(null)
const draftRule = ref<IntruderPayloadProcessingRule>(createDefaultPayloadProcessingRule())

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
const draftConditionType = computed<IntruderPayloadProcessingConditionType>({
  get: () => draftRule.value.conditionType ?? 'always',
  set: (value) => {
    draftRule.value = {
      ...draftRule.value,
      conditionType: value,
      conditionValue: value === 'always' ? '' : draftRule.value.conditionValue ?? '',
    }
  },
})
const ruleTypeOptions = computed(() =>
  ([
    'prefix',
    'suffix',
    'replace',
    'replaceRegex',
    'substring',
    'reverseSubstring',
    'lowercase',
    'uppercase',
    'trim',
    'base64',
    'urlEncode',
    'decode',
    'hash',
    'addRawPayload',
    'replaceBaseValue',
    'reverse',
    'removeWhitespace',
    'repeat',
    'hexEncode',
    'skipRegex',
  ] as IntruderPayloadProcessingRuleType[]).map((value) => ({
    value,
    label: getPayloadProcessingRuleLabel(value),
  })),
)
const codecOptions = computed(() =>
  (['url', 'base64', 'hex'] as IntruderPayloadProcessingCodec[]).map((value) => ({
    value,
    label: getPayloadProcessingCodecLabel(value),
  })),
)
const hashOptions = computed(() =>
  (['sha256', 'sha1'] as IntruderPayloadProcessingHashAlgorithm[]).map((value) => ({
    value,
    label: getPayloadProcessingHashLabel(value),
  })),
)
const rawPayloadPlacementOptions = computed(() =>
  (['after', 'before'] as IntruderPayloadProcessingRawPayloadPlacement[]).map((value) => ({
    value,
    label: getPayloadProcessingRawPayloadPlacementLabel(value),
  })),
)
const conditionOptions = computed(() =>
  (['always', 'contains', 'notContains', 'regex'] as IntruderPayloadProcessingConditionType[]).map((value) => ({
    value,
    label: getPayloadProcessingConditionLabel(value),
  })),
)

function updateRules(nextRules: IntruderPayloadProcessingRule[]) {
  emit('update:rules', nextRules)
}

function toggleRuleEnabled(ruleId: string, enabled: boolean) {
  updateRules(
    props.rules.map((rule) => (rule.id === ruleId ? { ...rule, enabled } : rule)),
  )
}

function openCreateDialog() {
  editingMode.value = 'create'
  editingRuleId.value = null
  draftRule.value = createDefaultPayloadProcessingRule()
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

function updateSubstringStart(value: string) {
  draftRule.value = {
    ...draftRule.value,
    substringStart: Math.max(0, Number(value) || 0),
  }
}

function updateSubstringLength(value: string) {
  draftRule.value = {
    ...draftRule.value,
    substringLength: value.trim() === '' ? null : Math.max(0, Number(value) || 0),
  }
}

function saveRule() {
  const nextRule: IntruderPayloadProcessingRule = {
    ...draftRule.value,
    conditionType: draftRule.value.conditionType ?? 'always',
    conditionValue: (draftRule.value.conditionType ?? 'always') === 'always' ? '' : draftRule.value.conditionValue ?? '',
    caseSensitive: Boolean(draftRule.value.caseSensitive),
    substringStart: Math.max(0, Number(draftRule.value.substringStart) || 0),
    substringLength: draftRule.value.substringLength == null ? null : Math.max(0, Number(draftRule.value.substringLength) || 0),
    codecType: draftRule.value.codecType ?? 'url',
    hashAlgorithm: draftRule.value.hashAlgorithm ?? 'sha256',
    rawPayloadPlacement: draftRule.value.rawPayloadPlacement ?? 'after',
  }

  if (editingMode.value === 'create') {
    updateRules([...props.rules, nextRule])
    selectedRuleId.value = nextRule.id
  } else if (editingRuleId.value) {
    updateRules(props.rules.map((rule) => (rule.id === editingRuleId.value ? nextRule : rule)))
    selectedRuleId.value = nextRule.id
  }

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

function describeRule(rule: IntruderPayloadProcessingRule): string {
  const ruleLabel = getPayloadProcessingRuleLabel(rule.type)
  const conditionType = rule.conditionType ?? 'always'
  const conditionLabel = getPayloadProcessingConditionLabel(conditionType)

  let detail = ''
  if (rule.type === 'prefix' || rule.type === 'suffix' || rule.type === 'repeat') {
    detail = rule.replaceValue || '-'
  } else if (rule.type === 'replace' || rule.type === 'replaceRegex') {
    detail = `${rule.matchValue || '-'} -> ${rule.replaceValue || '-'}`
  } else if (rule.type === 'substring') {
    const length = rule.substringLength == null ? t('trafficAnalysis.intruder.labels.substringToEnd') : String(rule.substringLength)
    detail = `${rule.substringStart ?? 0}, ${length}`
  } else if (rule.type === 'reverseSubstring') {
    const length = rule.substringLength == null ? t('trafficAnalysis.intruder.labels.substringToEnd') : String(rule.substringLength)
    detail = `${rule.substringStart ?? 0}, ${length}`
  } else if (rule.type === 'decode') {
    detail = getPayloadProcessingCodecLabel(rule.codecType ?? 'url')
  } else if (rule.type === 'hash') {
    detail = getPayloadProcessingHashLabel(rule.hashAlgorithm ?? 'sha256')
  } else if (rule.type === 'addRawPayload') {
    detail = getPayloadProcessingRawPayloadPlacementLabel(rule.rawPayloadPlacement ?? 'after')
  } else if (rule.type === 'replaceBaseValue') {
    detail = '{base}'
  } else if (rule.type === 'skipRegex') {
    detail = rule.matchValue || '-'
  }

  if (conditionType === 'always') {
    return detail ? `${ruleLabel}: ${detail}` : ruleLabel
  }

  const conditionValue = rule.conditionValue || '-'
  return detail
    ? `${ruleLabel}: ${detail} | ${conditionLabel}: ${conditionValue}`
    : `${ruleLabel} | ${conditionLabel}: ${conditionValue}`
}
</script>
