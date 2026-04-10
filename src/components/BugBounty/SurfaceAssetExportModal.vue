<template>
  <AppDialog class="modal" :class="{ 'modal-open': visible }">
    <div class="modal-box max-w-2xl">
      <h3 class="font-bold text-lg">{{ t('bugBounty.surface.inventory.export.title') }}</h3>
      <p class="mt-1 text-sm text-base-content/60">
        {{ t('bugBounty.surface.inventory.export.description') }}
      </p>

      <div class="mt-4 space-y-4">
        <div v-if="programName" class="rounded-lg border border-base-300 bg-base-200/50 p-3 text-sm">
          <span class="font-medium">{{ t('bugBounty.surface.inventory.export.program') }}:</span>
          {{ programName }}
        </div>

        <label class="form-control">
          <span class="label-text mb-1">{{ t('bugBounty.surface.inventory.export.assetType') }}</span>
          <select v-model="form.exportType" class="select select-bordered">
            <option
              v-for="option in exportTypeOptions"
              :key="option.value"
              :value="option.value"
            >
              {{ option.label }}
            </option>
          </select>
        </label>

        <div class="form-control">
          <span class="label-text mb-2">{{ t('bugBounty.surface.inventory.export.format') }}</span>
          <div class="flex flex-wrap gap-3">
            <label class="label cursor-pointer justify-start gap-2 rounded-lg border border-base-300 px-3 py-2">
              <input v-model="form.format" type="radio" class="radio radio-sm" value="csv" />
              <span class="label-text">CSV</span>
            </label>
            <label class="label cursor-pointer justify-start gap-2 rounded-lg border border-base-300 px-3 py-2">
              <input v-model="form.format" type="radio" class="radio radio-sm" value="json" />
              <span class="label-text">JSON</span>
            </label>
          </div>
        </div>

        <div class="rounded-lg border border-dashed border-base-300 bg-base-200/30 p-3 text-sm text-base-content/70">
          {{ t('bugBounty.surface.inventory.export.scopeHint') }}
        </div>
      </div>

      <div class="modal-action">
        <button type="button" class="btn" :disabled="exporting" @click="$emit('close')">
          {{ t('common.cancel') }}
        </button>
        <button type="button" class="btn btn-primary" :disabled="exporting" @click="submit">
          <span v-if="exporting" class="loading loading-spinner loading-xs"></span>
          {{ t('bugBounty.surface.inventory.export.submit') }}
        </button>
      </div>
    </div>
  </AppDialog>
</template>

<script setup lang="ts">
import { computed, reactive, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import type { SurfaceAssetExportType } from './surfaceAssetExportSupport'

export interface SurfaceAssetExportPayload {
  exportType: SurfaceAssetExportType
  format: 'csv' | 'json'
}

const props = defineProps<{
  visible: boolean
  exporting: boolean
  programName?: string | null
  currentAssetType?: string | null
  availableAssetTypes?: string[]
  initialExportType?: SurfaceAssetExportType
}>()

const emit = defineEmits<{
  close: []
  submit: [payload: SurfaceAssetExportPayload]
}>()

const { t } = useI18n()

const defaultExportType = () => props.initialExportType || (props.currentAssetType ? 'current' : 'all')

const form = reactive<SurfaceAssetExportPayload>({
  exportType: defaultExportType(),
  format: 'csv',
})

const exportTypeOptions = computed(() => {
  const options: Array<{ value: SurfaceAssetExportType; label: string }> = []

  if (props.currentAssetType) {
    options.push({
      value: 'current',
      label: t('bugBounty.surface.inventory.export.currentType', {
        type: t(`bugBounty.surface.assetTypes.${props.currentAssetType}`),
      }),
    })
  }

  options.push(
    { value: 'all', label: t('bugBounty.surface.inventory.export.allAssets') },
    { value: 'api', label: t('bugBounty.surface.inventory.export.apiAssets') },
  )

  const dynamicTypes = Array.from(new Set(props.availableAssetTypes || []))
  dynamicTypes.forEach(type => {
    options.push({
      value: type as SurfaceAssetExportType,
      label: t(`bugBounty.surface.assetTypes.${type}`),
    })
  })

  return options.filter((option, index, list) =>
    list.findIndex(candidate => candidate.value === option.value) === index,
  )
})

watch(
  () => [props.visible, props.initialExportType, props.currentAssetType],
  ([visible]) => {
    if (!visible) return
    form.exportType = defaultExportType()
    form.format = 'csv'
  },
)

const submit = () => {
  emit('submit', {
    exportType: form.exportType,
    format: form.format,
  })
}
</script>
