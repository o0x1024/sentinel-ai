<template>
  <div v-if="visible" class="modal modal-open z-50">
    <div class="modal-box max-w-5xl">
      <div class="flex items-start justify-between gap-4">
        <div>
          <h3 class="text-lg font-semibold">{{ t('bugBounty.surface.runs.detailTitle') }}</h3>
          <p class="mt-1 text-xs text-base-content/60 font-mono break-all">{{ detail?.run?.id || runId || '-' }}</p>
        </div>
        <button class="btn btn-sm btn-ghost" @click="$emit('close')">✕</button>
      </div>

      <div v-if="loading" class="py-12 flex items-center justify-center">
        <span class="loading loading-spinner loading-md"></span>
      </div>

      <div v-else-if="error" class="alert alert-error mt-4">
        <span>{{ error }}</span>
      </div>

      <div v-else-if="detail" class="mt-4 space-y-6">
        <section class="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-6 gap-3">
          <div class="rounded-lg border border-base-300 bg-base-200/50 p-3">
            <div class="text-xs text-base-content/60">{{ t('bugBounty.surface.runs.status') }}</div>
            <div class="mt-1 font-medium">{{ formatStatus(detail.run.status) }}</div>
          </div>
          <div class="rounded-lg border border-base-300 bg-base-200/50 p-3">
            <div class="text-xs text-base-content/60">{{ t('bugBounty.surface.runs.plugin') }}</div>
            <div class="mt-1 font-medium break-all">{{ detail.run.plugin_id || '-' }}</div>
          </div>
          <div class="rounded-lg border border-base-300 bg-base-200/50 p-3">
            <div class="text-xs text-base-content/60">{{ t('bugBounty.surface.runs.trigger') }}</div>
            <div class="mt-1 font-medium break-all">{{ detail.run.trigger_source }}</div>
          </div>
          <div class="rounded-lg border border-base-300 bg-base-200/50 p-3">
            <div class="text-xs text-base-content/60">{{ t('bugBounty.surface.runs.startedAt') }}</div>
            <div class="mt-1 font-medium">{{ formatTime(detail.run.started_at) }}</div>
          </div>
          <div class="rounded-lg border border-base-300 bg-base-200/50 p-3">
            <div class="text-xs text-base-content/60">{{ t('bugBounty.surface.runs.completedAt') }}</div>
            <div class="mt-1 font-medium">{{ formatTime(detail.run.completed_at) }}</div>
          </div>
          <div class="rounded-lg border border-base-300 bg-base-200/50 p-3">
            <div class="text-xs text-base-content/60">{{ t('bugBounty.surface.columns.imported') }}</div>
            <div class="mt-1 font-medium">{{ detail.run.imported_asset_count || 0 }}</div>
          </div>
        </section>

        <section class="card bg-base-100 border border-base-300">
          <div class="card-body">
            <h4 class="card-title text-base">{{ t('bugBounty.surface.runs.observations') }}</h4>
            <div v-if="Object.keys(detail.by_artifact || {}).length" class="flex flex-wrap gap-2">
              <span
                v-for="(count, artifactType) in detail.by_artifact"
                :key="artifactType"
                class="badge badge-outline badge-lg"
              >
                {{ artifactType }}: {{ count }}
              </span>
            </div>
            <div v-else class="text-sm text-base-content/60">{{ t('bugBounty.surface.runs.noObservations') }}</div>
          </div>
        </section>

        <section class="card bg-base-100 border border-base-300">
          <div class="card-body">
            <h4 class="card-title text-base">{{ t('bugBounty.surface.runs.linkedAssets') }}</h4>
            <div v-if="detail.assets.length" class="space-y-2">
              <button
                v-for="asset in detail.assets"
                :key="asset.id"
                class="w-full rounded-lg border border-base-300 bg-base-200/50 px-3 py-2 text-left hover:border-primary/40"
                @click="$emit('open-asset', asset.id)"
              >
                <div class="flex items-center justify-between gap-3">
                  <span class="badge badge-outline badge-sm">{{ formatAssetType(asset.asset_type) }}</span>
                  <span class="text-xs text-base-content/60">{{ formatTime(asset.last_seen_at) }}</span>
                </div>
                <div class="mt-2 font-mono text-xs break-all">{{ asset.asset_name }}</div>
                <div class="text-sm truncate">{{ asset.display_name || '-' }}</div>
              </button>
            </div>
            <div v-else class="text-sm text-base-content/60">{{ t('bugBounty.surface.runs.noLinkedAssets') }}</div>
          </div>
        </section>

        <section class="card bg-base-100 border border-base-300">
          <div class="card-body">
            <h4 class="card-title text-base">{{ t('bugBounty.surface.runs.changes') }}</h4>
            <div v-if="detail.changes.length" class="space-y-2">
              <div
                v-for="change in detail.changes"
                :key="change.id"
                class="rounded-lg border border-base-300 px-3 py-2"
              >
                <div class="text-xs text-base-content/60">{{ change.change_type }}</div>
                <div class="mt-1 text-sm">{{ change.summary }}</div>
                <div class="mt-1 text-xs text-base-content/60">{{ formatTime(change.detected_at) }}</div>
              </div>
            </div>
            <div v-else class="text-sm text-base-content/60">{{ t('bugBounty.surface.runs.noChanges') }}</div>
          </div>
        </section>
      </div>
    </div>
    <div class="modal-backdrop" @click="$emit('close')"></div>
  </div>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useI18n } from 'vue-i18n'

const props = defineProps<{
  visible: boolean
  runId?: string | null
}>()

defineEmits<{
  (e: 'close'): void
  (e: 'open-asset', assetId: string): void
}>()

const { t } = useI18n()

const loading = ref(false)
const error = ref('')
const detail = ref<any | null>(null)

const formatTime = (value?: string | null) => {
  if (!value) return '-'
  const date = new Date(value)
  if (Number.isNaN(date.getTime())) return value
  return date.toLocaleString()
}

const formatStatus = (value?: string) => {
  if (!value) return '-'
  const key = `bugBounty.surface.status.${value}`
  const translated = t(key)
  return translated === key ? value : translated
}

const formatAssetType = (value?: string) => {
  if (!value) return '-'
  const key = `bugBounty.surface.assetTypes.${value}`
  const translated = t(key)
  return translated === key ? value : translated
}

const loadDetail = async () => {
  if (!props.visible || !props.runId) return
  try {
    loading.value = true
    error.value = ''
    detail.value = await invoke('surface_get_discovery_run_detail', { runId: props.runId })
  } catch (err) {
    error.value = err instanceof Error ? err.message : String(err)
    detail.value = null
  } finally {
    loading.value = false
  }
}

watch(
  () => [props.visible, props.runId],
  () => {
    loadDetail()
  },
  { immediate: true },
)
</script>
