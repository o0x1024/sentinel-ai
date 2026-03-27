<template>
  <div>
    <button class="btn btn-sm btn-ghost" type="button" @click="openDialog">
      <i class="fas fa-save"></i>
      {{ $t('trafficAnalysis.intruder.labels.savedAttack') }}
    </button>

    <dialog ref="dialogRef" class="modal">
      <div class="modal-box max-w-4xl">
        <h3 class="mb-4 text-lg font-semibold">{{ $t('trafficAnalysis.intruder.labels.savedAttack') }}</h3>

        <div class="grid gap-4 lg:grid-cols-[11rem_minmax(0,1fr)]">
          <div class="space-y-2">
            <button class="btn btn-sm w-full" type="button" @click="emitSave">
              {{ $t('trafficAnalysis.intruder.actions.saveAttack') }}
            </button>
            <button class="btn btn-sm btn-ghost w-full" type="button" :disabled="!selectedTemplateId" @click="emitOverwrite">
              {{ $t('trafficAnalysis.intruder.actions.overwriteAttack') }}
            </button>
            <button class="btn btn-sm btn-ghost w-full" type="button" :disabled="!selectedTemplateId" @click="emitRename">
              {{ $t('trafficAnalysis.intruder.actions.renameAttack') }}
            </button>
            <button class="btn btn-sm btn-ghost w-full" type="button" :disabled="!selectedTemplateId" @click="loadSelectedTemplate">
              {{ $t('trafficAnalysis.intruder.actions.loadAttack') }}
            </button>
            <button class="btn btn-sm btn-ghost w-full" type="button" :disabled="!selectedTemplateId" @click="emitDelete">
              {{ $t('trafficAnalysis.intruder.actions.deleteAttack') }}
            </button>
          </div>

          <div class="space-y-3">
            <p class="text-sm text-base-content/70">
              {{ $t('trafficAnalysis.intruder.help.attackTemplatesHint') }}
            </p>

            <label class="form-control">
              <span class="label-text">{{ $t('trafficAnalysis.intruder.actions.search') }}</span>
              <input
                v-model="searchQuery"
                type="text"
                class="input input-bordered"
                :placeholder="$t('trafficAnalysis.intruder.placeholders.searchTemplates')"
              />
            </label>

            <label class="form-control">
              <span class="label-text">{{ $t('trafficAnalysis.intruder.labels.templateName') }}</span>
              <input
                v-model="draftName"
                type="text"
                class="input input-bordered"
                :placeholder="$t('trafficAnalysis.intruder.placeholders.attackTemplateName')"
              />
            </label>

            <div class="overflow-hidden rounded-lg border border-base-300">
              <table class="table table-sm">
                <thead class="bg-base-200">
                  <tr>
                    <th>{{ $t('trafficAnalysis.intruder.labels.templateName') }}</th>
                    <th>{{ $t('trafficAnalysis.intruder.labels.attackType') }}</th>
                    <th>{{ $t('trafficAnalysis.intruder.labels.target') }}</th>
                  </tr>
                </thead>
                <tbody>
                  <tr
                    v-for="template in filteredTemplates"
                    :key="template.id"
                    class="cursor-pointer"
                    :class="selectedTemplateId === template.id ? 'bg-primary/10' : ''"
                    @click="selectTemplate(template.id)"
                    @dblclick="emitLoad(template.id)"
                  >
                    <td class="max-w-0 truncate" :title="template.name">{{ template.name }}</td>
                    <td>{{ attackTypeLabel(template.attackType) }}</td>
                    <td class="max-w-0 truncate" :title="targetLabel(template)">{{ targetLabel(template) }}</td>
                  </tr>
                  <tr v-if="!filteredTemplates.length">
                    <td colspan="3" class="py-10 text-center text-sm text-base-content/60">
                      {{ $t('trafficAnalysis.intruder.empty.noAttackTemplates') }}
                    </td>
                  </tr>
                </tbody>
              </table>
            </div>
          </div>
        </div>

        <div class="mt-6 flex justify-end">
          <button class="btn btn-ghost" type="button" @click="closeDialog">
            {{ $t('common.close') }}
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
import type { IntruderAttackType } from './types'
import type { IntruderAttackTemplate } from './storage'

const props = defineProps<{
  templates: IntruderAttackTemplate[]
  selectedTemplateId: string
  defaultName: string
}>()

const emit = defineEmits<{
  (e: 'update:selectedTemplateId', value: string): void
  (e: 'save', name: string): void
  (e: 'overwrite', value: { id: string; name: string }): void
  (e: 'rename', value: { id: string; name: string }): void
  (e: 'load', templateId: string): void
  (e: 'delete', templateId: string): void
}>()

const { t } = useI18n()
const dialogRef = ref<HTMLDialogElement | null>(null)
const draftName = ref(props.defaultName)
const searchQuery = ref('')

const selectedTemplate = computed(() => props.templates.find((template) => template.id === props.selectedTemplateId) ?? null)
const filteredTemplates = computed(() => {
  const query = searchQuery.value.trim().toLowerCase()
  if (!query) return props.templates
  return props.templates.filter((template) => {
    const target = targetLabel(template).toLowerCase()
    return (
      template.name.toLowerCase().includes(query)
      || template.attackType.toLowerCase().includes(query)
      || target.includes(query)
    )
  })
})

watch(
  () => props.selectedTemplateId,
  () => {
    draftName.value = selectedTemplate.value?.name ?? props.defaultName
  },
)

watch(
  () => props.defaultName,
  (value) => {
    if (!selectedTemplate.value) {
      draftName.value = value
    }
  },
)

function openDialog() {
  draftName.value = selectedTemplate.value?.name ?? props.defaultName
  searchQuery.value = ''
  dialogRef.value?.showModal()
}

function closeDialog() {
  dialogRef.value?.close()
}

function selectTemplate(templateId: string) {
  emit('update:selectedTemplateId', templateId)
}

function emitSave() {
  const name = draftName.value.trim()
  if (!name) return
  emit('save', name)
}

function emitOverwrite() {
  const name = draftName.value.trim()
  if (!name || !props.selectedTemplateId) return
  emit('overwrite', { id: props.selectedTemplateId, name })
}

function emitRename() {
  const name = draftName.value.trim()
  if (!name || !props.selectedTemplateId) return
  emit('rename', { id: props.selectedTemplateId, name })
}

function emitLoad(templateId = props.selectedTemplateId) {
  if (!templateId) return
  emit('load', templateId)
  closeDialog()
}

function loadSelectedTemplate() {
  emitLoad(props.selectedTemplateId)
}

function emitDelete() {
  if (!props.selectedTemplateId) return
  emit('delete', props.selectedTemplateId)
}

function targetLabel(template: IntruderAttackTemplate): string {
  const protocol = template.target.useTls ? 'https' : 'http'
  return `${protocol}://${template.target.host}:${template.target.port}`
}

function attackTypeLabel(type: IntruderAttackType): string {
  return t(`trafficAnalysis.intruder.attackTypes.${type}`)
}
</script>
