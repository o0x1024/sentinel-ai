<template>
  <AppDialog class="modal" :class="{ 'modal-open': visible }">
    <div class="modal-box max-w-4xl">
      <h3 class="font-bold text-lg">{{ t('bugBounty.surface.inventory.edit.title') }}</h3>
      <div class="mt-1 flex flex-wrap items-center gap-2 text-sm text-base-content/60">
        <span>{{ asset?.display_name || asset?.asset_name || '-' }}</span>
        <span class="badge badge-outline badge-sm">
          {{ assetTypeLabel }}
        </span>
      </div>

      <form class="mt-4 space-y-5" @submit.prevent="submit">
        <section class="space-y-4">
          <div class="text-sm font-semibold text-base-content">
            {{ t('bugBounty.surface.inventory.edit.commonFields') }}
          </div>

          <div class="grid grid-cols-1 gap-4 md:grid-cols-2">
            <label class="form-control">
              <span class="label-text mb-1">{{ t('bugBounty.surface.inventory.edit.displayName') }}</span>
              <input v-model.trim="form.display_name" type="text" class="input input-bordered" />
            </label>

            <label class="form-control">
              <span class="label-text mb-1">{{ t('bugBounty.surface.inventory.edit.owner') }}</span>
              <input v-model.trim="form.owner" type="text" class="input input-bordered" />
            </label>
          </div>

          <label class="form-control">
            <span class="label-text mb-1">{{ t('bugBounty.surface.inventory.edit.description') }}</span>
            <textarea v-model.trim="form.description" class="textarea textarea-bordered min-h-24"></textarea>
          </label>

          <div class="grid grid-cols-1 gap-4 md:grid-cols-2">
            <label class="form-control">
              <span class="label-text mb-1">{{ t('bugBounty.surface.inventory.edit.status') }}</span>
              <select v-model="form.status" class="select select-bordered">
                <option value="active">active</option>
                <option value="inactive">inactive</option>
                <option value="unknown">unknown</option>
              </select>
            </label>

            <label class="form-control">
              <span class="label-text mb-1">{{ t('bugBounty.surface.inventory.edit.riskLevel') }}</span>
              <select v-model="form.risk_level" class="select select-bordered">
                <option value=""></option>
                <option value="critical">critical</option>
                <option value="high">high</option>
                <option value="medium">medium</option>
                <option value="low">low</option>
                <option value="info">info</option>
              </select>
            </label>
          </div>

          <div class="grid grid-cols-1 gap-4 md:grid-cols-2">
            <label class="form-control">
              <span class="label-text mb-1">{{ t('bugBounty.surface.inventory.edit.exposure') }}</span>
              <select v-model="form.internet_exposure" class="select select-bordered">
                <option value=""></option>
                <option value="public">public</option>
                <option value="partner">partner</option>
                <option value="internal">internal</option>
                <option value="private">private</option>
              </select>
            </label>

            <label class="form-control">
              <span class="label-text mb-1">{{ t('bugBounty.surface.inventory.edit.criticality') }}</span>
              <select v-model="form.criticality" class="select select-bordered">
                <option value=""></option>
                <option value="critical">critical</option>
                <option value="high">high</option>
                <option value="medium">medium</option>
                <option value="low">low</option>
              </select>
            </label>
          </div>
        </section>

        <section v-if="typedFields.length" class="space-y-4">
          <div class="text-sm font-semibold text-base-content">
            {{ t('bugBounty.surface.inventory.edit.typeSpecificFields') }}
          </div>

          <div class="grid grid-cols-1 gap-4 md:grid-cols-2">
            <label
              v-for="field in typedFields"
              :key="field.key"
              class="form-control"
            >
              <span class="label-text mb-1">{{ t(field.labelKey) }}</span>

              <input
                v-if="field.type === 'text' || field.type === 'number'"
                v-model="typedForm[field.key]"
                :type="field.type === 'number' ? 'number' : 'text'"
                class="input input-bordered"
              />

              <select
                v-else-if="field.type === 'select'"
                v-model="typedForm[field.key]"
                class="select select-bordered"
              >
                <option value=""></option>
                <option
                  v-for="option in field.options || []"
                  :key="option.value"
                  :value="option.value"
                >
                  {{ option.labelKey ? t(option.labelKey) : option.label || option.value }}
                </option>
              </select>
            </label>
          </div>
        </section>

        <div class="modal-action">
          <button type="button" class="btn" @click="$emit('close')">{{ t('common.cancel') }}</button>
          <button type="submit" class="btn btn-primary">{{ t('common.save') }}</button>
        </div>
      </form>
    </div>
  </AppDialog>
</template>

<script setup lang="ts">
import { computed, reactive, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import {
  buildSurfaceAssetTypedDetailsPayload,
  buildSurfaceAssetTypedForm,
  getSurfaceAssetEditSchema,
  type SurfaceAssetEditPayload,
  type SurfaceAssetEditTarget,
} from './surfaceAssetEditSupport'

const props = defineProps<{
  visible: boolean
  target: SurfaceAssetEditTarget | null
}>()

const emit = defineEmits<{
  (e: 'close'): void
  (e: 'save', payload: SurfaceAssetEditPayload): void
}>()

const { t } = useI18n()

const form = reactive({
  display_name: '',
  description: '',
  owner: '',
  status: 'active',
  internet_exposure: '',
  criticality: '',
  risk_level: '',
})

const typedForm = reactive<Record<string, string>>({})

const asset = computed(() => props.target?.asset || null)
const assetType = computed(() => String(asset.value?.asset_type || '').trim())
const assetTypeLabel = computed(() => {
  if (!assetType.value) return '-'
  return t(`bugBounty.surface.assetTypes.${assetType.value}`)
})
const typedFields = computed(() => getSurfaceAssetEditSchema(assetType.value).fields)

const normalizeOptionalString = (value: string) => {
  const trimmed = value.trim()
  return trimmed || null
}

watch(
  () => props.target,
  (target) => {
    const nextAsset = target?.asset || {}
    form.display_name = nextAsset.display_name || ''
    form.description = nextAsset.description || ''
    form.owner = nextAsset.owner || ''
    form.status = nextAsset.status || 'active'
    form.internet_exposure = nextAsset.internet_exposure || ''
    form.criticality = nextAsset.criticality || ''
    form.risk_level = nextAsset.risk_level || ''

    const nextTypedForm = buildSurfaceAssetTypedForm(nextAsset.asset_type, target?.typed_details)
    for (const key of Object.keys(typedForm)) {
      delete typedForm[key]
    }
    Object.assign(typedForm, nextTypedForm)
  },
  { immediate: true },
)

const submit = () => {
  emit('save', {
    display_name: normalizeOptionalString(form.display_name),
    description: normalizeOptionalString(form.description),
    owner: normalizeOptionalString(form.owner),
    status: normalizeOptionalString(form.status),
    internet_exposure: normalizeOptionalString(form.internet_exposure),
    criticality: normalizeOptionalString(form.criticality),
    risk_level: normalizeOptionalString(form.risk_level),
    typed_details: buildSurfaceAssetTypedDetailsPayload(
      assetType.value,
      props.target?.typed_details,
      typedForm,
    ),
  })
}
</script>
