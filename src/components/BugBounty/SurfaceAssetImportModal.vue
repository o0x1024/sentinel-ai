<template>
  <dialog class="modal" :class="{ 'modal-open': visible }">
    <div class="modal-box max-w-3xl">
      <h3 class="font-bold text-lg">{{ t('bugBounty.surface.inventory.import.title') }}</h3>
      <p class="mt-1 text-sm text-base-content/60">
        {{ t('bugBounty.surface.inventory.import.description') }}
      </p>

      <form class="mt-4 space-y-4" @submit.prevent="submit">
        <div class="grid grid-cols-1 gap-4 md:grid-cols-2">
          <label class="form-control">
            <span class="label-text mb-1">{{ t('bugBounty.surface.inventory.import.program') }}</span>
            <select v-model="form.program_id" class="select select-bordered" required>
              <option value="">{{ t('bugBounty.surface.inventory.import.programPlaceholder') }}</option>
              <option v-for="program in programs" :key="program.id" :value="program.id">
                {{ program.name }}
              </option>
            </select>
          </label>

          <label class="form-control">
            <span class="label-text mb-1">{{ t('bugBounty.surface.inventory.import.assetType') }}</span>
            <select v-model="form.asset_type" class="select select-bordered" required>
              <option v-for="assetType in supportedAssetTypes" :key="assetType" :value="assetType">
                {{ t(`bugBounty.surface.assetTypes.${assetType}`) }}
              </option>
            </select>
          </label>
        </div>

        <label class="form-control">
          <span class="label-text mb-1">{{ t('bugBounty.surface.inventory.import.rawInput') }}</span>
          <textarea
            v-model.trim="form.raw_input"
            class="textarea textarea-bordered min-h-40 font-mono text-sm"
            :placeholder="rawInputPlaceholder"
            required
          ></textarea>
          <span class="label-text-alt mt-1 text-base-content/60">
            {{ t('bugBounty.surface.inventory.import.rawInputHint') }}
          </span>
        </label>

        <div class="grid grid-cols-1 gap-4 md:grid-cols-2">
          <label class="form-control">
            <span class="label-text mb-1">{{ t('bugBounty.surface.inventory.import.source') }}</span>
            <input
              v-model.trim="form.source"
              type="text"
              class="input input-bordered"
              :placeholder="t('bugBounty.surface.inventory.import.sourcePlaceholder')"
            />
          </label>

          <label class="form-control">
            <span class="label-text mb-1">{{ t('bugBounty.surface.inventory.import.owner') }}</span>
            <input
              v-model.trim="form.owner"
              type="text"
              class="input input-bordered"
              :placeholder="t('bugBounty.surface.inventory.import.ownerPlaceholder')"
            />
          </label>
        </div>

        <label class="form-control">
          <span class="label-text mb-1">{{ t('bugBounty.surface.inventory.import.descriptionLabel') }}</span>
          <textarea
            v-model.trim="form.description"
            class="textarea textarea-bordered min-h-24"
            :placeholder="t('bugBounty.surface.inventory.import.descriptionPlaceholder')"
          ></textarea>
        </label>

        <div class="grid grid-cols-1 gap-4 md:grid-cols-3">
          <label class="form-control">
            <span class="label-text mb-1">{{ t('bugBounty.surface.inventory.import.status') }}</span>
            <select v-model="form.status" class="select select-bordered">
              <option value="active">active</option>
              <option value="inactive">inactive</option>
              <option value="unknown">unknown</option>
            </select>
          </label>

          <label class="form-control">
            <span class="label-text mb-1">{{ t('bugBounty.surface.inventory.import.exposure') }}</span>
            <select v-model="form.internet_exposure" class="select select-bordered">
              <option value=""></option>
              <option value="public">public</option>
              <option value="partner">partner</option>
              <option value="internal">internal</option>
              <option value="private">private</option>
            </select>
          </label>

          <label class="form-control">
            <span class="label-text mb-1">{{ t('bugBounty.surface.inventory.import.criticality') }}</span>
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
          <button type="button" class="btn" :disabled="submitting" @click="$emit('close')">
            {{ t('common.cancel') }}
          </button>
          <button type="submit" class="btn btn-primary" :disabled="submitting || !canSubmit">
            <span v-if="submitting" class="loading loading-spinner loading-xs"></span>
            {{ t('bugBounty.surface.inventory.import.submit') }}
          </button>
        </div>
      </form>
    </div>
  </dialog>
</template>

<script setup lang="ts">
import { computed, reactive, watch } from 'vue'
import { useI18n } from 'vue-i18n'

export interface SurfaceAssetImportPayload {
  program_id: string
  asset_type: string
  raw_input: string
  source?: string | null
  owner?: string | null
  description?: string | null
  status?: string | null
  internet_exposure?: string | null
  criticality?: string | null
  risk_level?: string | null
}

const supportedAssetTypes = ['org', 'domain', 'ip', 'host', 'web'] as const

const props = defineProps<{
  visible: boolean
  submitting?: boolean
  programId?: string | null
  programs?: Array<{ id: string; name: string }>
}>()

const emit = defineEmits<{
  (e: 'close'): void
  (e: 'submit', payload: SurfaceAssetImportPayload): void
}>()

const { t } = useI18n()

const form = reactive<SurfaceAssetImportPayload>({
  program_id: '',
  asset_type: 'domain',
  raw_input: '',
  source: 'manual_import',
  owner: '',
  description: '',
  status: 'active',
  internet_exposure: 'public',
  criticality: '',
  risk_level: '',
})

const rawInputPlaceholder = computed(() => {
  switch (form.asset_type) {
    case 'org':
      return 'Example Corp\nAcme Security'
    case 'ip':
      return '1.1.1.1\n8.8.8.8'
    case 'host':
      return 'jumpbox.internal\nbastion.example.com'
    case 'web':
      return 'https://example.com\nhttps://api.example.com/v1'
    case 'domain':
    default:
      return 'example.com\napi.example.com'
  }
})

const canSubmit = computed(() => Boolean(form.program_id && form.asset_type && form.raw_input.trim()))

watch(
  () => [props.visible, props.programId] as const,
  ([visible, programId]) => {
    if (!visible) return
    form.program_id = programId || ''
    form.asset_type = 'domain'
    form.raw_input = ''
    form.source = 'manual_import'
    form.owner = ''
    form.description = ''
    form.status = 'active'
    form.internet_exposure = 'public'
    form.criticality = ''
    form.risk_level = ''
  },
  { immediate: true },
)

const submit = () => {
  emit('submit', {
    ...form,
    source: form.source?.trim() || null,
    owner: form.owner?.trim() || null,
    description: form.description?.trim() || null,
    status: form.status || null,
    internet_exposure: form.internet_exposure || null,
    criticality: form.criticality || null,
    risk_level: form.risk_level || null,
  })
}
</script>
