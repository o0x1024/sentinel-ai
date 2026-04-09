<template>
  <div class="space-y-4">
    <div class="rounded-lg border border-base-300 bg-base-100 p-4">
      <div class="mb-3 flex flex-wrap items-center justify-between gap-2">
        <div>
          <p class="text-sm font-semibold">{{ wb('drafts.title') }}</p>
          <p class="text-xs text-base-content/60">{{ wb('drafts.summary') }}</p>
        </div>
        <span class="badge badge-outline">{{ wb('drafts.count', { count: drafts.length }) }}</span>
      </div>

      <div v-if="drafts.length > 0" class="space-y-3">
        <div
          v-for="draft in drafts"
          :key="draft.id"
          :ref="setDraftRef(draft.id)"
          :class="[
            'rounded-lg border bg-base-200/50 p-4 space-y-3',
            selectedDraftId === draft.id ? 'border-primary ring-1 ring-primary/30' : 'border-base-300',
          ]"
        >
          <div class="flex flex-wrap items-center gap-2">
            <span
              :class="[
                'badge',
                draft.severity === 'high' ? 'badge-error' : draft.severity === 'medium' ? 'badge-warning' : 'badge-ghost',
              ]"
            >
              {{ wb(`priority.${draft.severity}`) }}
            </span>
            <span :class="draft.readOnly ? 'badge badge-success' : 'badge badge-warning'">
              {{ draft.readOnly ? wb('drafts.readonlyPreferred') : wb('drafts.manualConfirm') }}
            </span>
            <span class="badge badge-outline">{{ getExecutionDraftStatusLabel(draft.status) }}</span>
            <span v-if="selectedDraftId === draft.id" class="badge badge-secondary">{{ wb('drafts.focused') }}</span>
          </div>

          <div>
            <p class="font-medium">{{ draft.title }}</p>
            <p class="mt-1 text-sm text-base-content/70">{{ draft.targetMethod }} {{ draft.targetUrl }}</p>
            <p class="mt-2 text-xs text-base-content/60">{{ draft.rationale }}</p>
          </div>

          <div class="flex flex-wrap gap-2">
            <span
              v-for="value in draft.candidateValues"
              :key="value"
              class="badge badge-primary"
            >
              {{ value }}
            </span>
          </div>

          <div class="flex flex-wrap items-center justify-between gap-3">
            <label class="form-control">
              <span class="label-text text-xs text-base-content/60 mb-1">{{ wb('drafts.status') }}</span>
              <select
                class="select select-bordered select-sm"
                :value="draft.status"
                @change="onStatusChange(draft.id, $event)"
              >
                <option value="draft">draft</option>
                <option value="ready">ready</option>
                <option value="paused">paused</option>
                <option value="archived">archived</option>
              </select>
            </label>
            <div class="flex flex-wrap gap-2">
              <button
                class="btn btn-sm btn-primary"
                :disabled="executingDraftId === draft.id"
                @click="$emit('execute-draft', draft.id, draft.readOnly)"
              >
                {{
                  executingDraftId === draft.id
                    ? wb('drafts.executing')
                    : draft.readOnly
                      ? wb('drafts.executeReadonly')
                      : wb('drafts.executeWithConfirm')
                }}
              </button>
              <button class="btn btn-sm btn-outline" @click="copyDraft(draft)">{{ wb('drafts.copySummary') }}</button>
            </div>
          </div>
        </div>
      </div>

      <p v-else class="text-sm text-base-content/60">
        {{ wb('drafts.empty') }}
      </p>
    </div>
  </div>
</template>

<script setup lang="ts">
import { nextTick, ref, watch } from 'vue'
import { dialog } from '@/composables/useDialog'
import type { WorkbenchExecutionDraft, WorkbenchExecutionDraftStatus } from './securityWorkbenchTypes'
import { wb } from './securityWorkbenchLocale'

const props = defineProps<{
  drafts: WorkbenchExecutionDraft[]
  executingDraftId: string | null
  selectedDraftId?: string | null
}>()

const emit = defineEmits<{
  'update-status': [draftId: string, status: WorkbenchExecutionDraftStatus]
  'execute-draft': [draftId: string, readOnly: boolean]
}>()

const draftRefs = ref<Record<string, HTMLElement | null>>({})

const getExecutionDraftStatusLabel = (status: WorkbenchExecutionDraftStatus) => wb(`drafts.statusLabel.${status}`)

const setDraftRef = (draftId: string) => (element: Element | null) => {
  draftRefs.value[draftId] = element instanceof HTMLElement ? element : null
}

watch(
  () => props.selectedDraftId,
  async (draftId) => {
    if (!draftId) return
    await nextTick()
    draftRefs.value[draftId]?.scrollIntoView({
      behavior: 'smooth',
      block: 'center',
    })
  },
  { immediate: true },
)

const onStatusChange = (draftId: string, event: Event) => {
  const value = (event.target as HTMLSelectElement).value as WorkbenchExecutionDraftStatus
  emit('update-status', draftId, value)
}

const copyDraft = async (draft: WorkbenchExecutionDraft) => {
  const text = [
    wb('drafts.formatted.title', { title: draft.title }),
    wb('drafts.formatted.status', { value: getExecutionDraftStatusLabel(draft.status) }),
    wb('drafts.formatted.readonly', { value: draft.readOnly ? wb('common.yes') : wb('common.no') }),
    wb('drafts.formatted.targetRequest', { value: `${draft.targetMethod} ${draft.targetUrl}` }),
    wb('drafts.formatted.targetField', { value: draft.targetField }),
    wb('drafts.formatted.candidateValues', { value: draft.candidateValues.join(', ') }),
    '',
    wb('drafts.formatted.steps'),
    ...draft.steps.map((step, index) => `${index + 1}. ${step.title} - ${step.detail}`),
    '',
    wb('drafts.formatted.stopConditions'),
    ...draft.stopConditions.map((item, index) => `${index + 1}. ${item.detail}`),
  ].join('\n')

  try {
    await navigator.clipboard.writeText(text)
    dialog.toast.success(wb('drafts.copySuccess'))
  } catch (error) {
    console.error('Failed to copy execution draft', error)
    dialog.toast.error(wb('drafts.copyFailed'))
  }
}
</script>
