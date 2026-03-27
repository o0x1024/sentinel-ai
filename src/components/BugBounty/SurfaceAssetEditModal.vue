<template>
  <dialog class="modal" :class="{ 'modal-open': visible }">
    <div class="modal-box max-w-2xl">
      <h3 class="font-bold text-lg">{{ t('bugBounty.surface.inventory.edit.title') }}</h3>
      <p class="text-sm text-base-content/60 mt-1">
        {{ asset?.display_name || asset?.asset_name || '-' }}
      </p>

      <form class="mt-4 space-y-4" @submit.prevent="submit">
        <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
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

        <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
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

        <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
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

        <div class="modal-action">
          <button type="button" class="btn" @click="$emit('close')">{{ t('common.cancel') }}</button>
          <button type="submit" class="btn btn-primary">{{ t('common.save') }}</button>
        </div>
      </form>
    </div>
  </dialog>
</template>

<script setup lang="ts">
import { reactive, watch } from 'vue'
import { useI18n } from 'vue-i18n'

export interface SurfaceAssetEditPayload {
  display_name?: string | null
  description?: string | null
  owner?: string | null
  status?: string | null
  internet_exposure?: string | null
  criticality?: string | null
  risk_level?: string | null
}

const props = defineProps<{
  visible: boolean
  asset: any | null
}>()

const emit = defineEmits<{
  (e: 'close'): void
  (e: 'save', payload: SurfaceAssetEditPayload): void
}>()

const { t } = useI18n()

const form = reactive<SurfaceAssetEditPayload>({
  display_name: '',
  description: '',
  owner: '',
  status: 'active',
  internet_exposure: '',
  criticality: '',
  risk_level: '',
})

watch(
  () => props.asset,
  (asset) => {
    form.display_name = asset?.display_name || ''
    form.description = asset?.description || ''
    form.owner = asset?.owner || ''
    form.status = asset?.status || 'active'
    form.internet_exposure = asset?.internet_exposure || ''
    form.criticality = asset?.criticality || ''
    form.risk_level = asset?.risk_level || ''
  },
  { immediate: true },
)

const submit = () => {
  emit('save', { ...form })
}
</script>
