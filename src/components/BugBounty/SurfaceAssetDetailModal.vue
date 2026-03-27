<template>
  <Teleport to="body">
    <div v-if="visible" class="fixed inset-0 z-[70]">
      <div class="absolute inset-0 bg-black/45 backdrop-blur-sm" @click="$emit('close')"></div>

      <div class="relative flex min-h-full items-start justify-center overflow-y-auto px-4 py-20 md:px-6 md:py-24">
        <div class="modal-box relative flex max-h-[calc(100vh-6rem)] w-full max-w-5xl flex-col overflow-hidden p-0 md:max-h-[calc(100vh-8rem)]">
          <div class="sticky top-0 z-10 border-b border-base-300 bg-base-100/95 px-6 py-4 backdrop-blur">
            <div class="flex items-start justify-between gap-4">
              <div class="min-w-0">
                <div class="flex items-center gap-2">
                  <h3 class="text-lg font-semibold break-all">
                    {{ detail?.asset?.display_name || detail?.asset?.asset_name || t('bugBounty.surface.detail.titleFallback') }}
                  </h3>
                  <span v-if="detail?.asset?.asset_type" class="badge badge-outline badge-sm shrink-0">
                    {{ formatAssetType(detail.asset.asset_type) }}
                  </span>
                </div>
                <p class="mt-1 font-mono text-xs text-base-content/60 break-all">
                  {{ detail?.asset?.asset_name || '-' }}
                </p>
              </div>
              <button class="btn btn-sm btn-ghost shrink-0" @click="$emit('close')">✕</button>
            </div>
          </div>

          <div v-if="loading" class="flex flex-1 items-center justify-center px-6 py-12">
            <span class="loading loading-spinner loading-md"></span>
          </div>

          <div v-else-if="error" class="flex-1 overflow-y-auto px-6 py-4">
            <div class="alert alert-error">
              <span>{{ error }}</span>
            </div>
          </div>

          <div v-else-if="detail" class="min-h-0 flex-1 overflow-y-auto px-6 pb-6">
            <div class="space-y-6 py-4">
              <section class="grid grid-cols-1 gap-3 md:grid-cols-2 xl:grid-cols-4">
                <div class="rounded-lg border border-base-300 bg-base-200/50 p-3">
                  <div class="text-xs text-base-content/60">{{ t('bugBounty.surface.detail.status') }}</div>
                  <div class="mt-1 font-medium">{{ formatStatus(detail.asset.status) }}</div>
                </div>
                <div class="rounded-lg border border-base-300 bg-base-200/50 p-3">
                  <div class="text-xs text-base-content/60">{{ t('bugBounty.surface.detail.exposure') }}</div>
                  <div class="mt-1 font-medium">{{ detail.asset.internet_exposure || '-' }}</div>
                </div>
                <div class="rounded-lg border border-base-300 bg-base-200/50 p-3">
                  <div class="text-xs text-base-content/60">{{ t('bugBounty.surface.detail.source') }}</div>
                  <div class="mt-1 font-medium">{{ detail.asset.source || '-' }}</div>
                </div>
                <div class="rounded-lg border border-base-300 bg-base-200/50 p-3">
                  <div class="text-xs text-base-content/60">{{ t('bugBounty.surface.detail.lastSeen') }}</div>
                  <div class="mt-1 font-medium">{{ formatTime(detail.asset.last_seen_at) }}</div>
                </div>
              </section>

              <section class="card border border-base-300 bg-base-100">
                <div class="card-body">
                  <h4 class="card-title text-base">{{ t('bugBounty.surface.detail.typedDetails') }}</h4>
                  <div v-if="typedEntries.length" class="grid grid-cols-1 gap-3 md:grid-cols-2">
                    <div v-for="[key, value] in typedEntries" :key="key" class="rounded-lg border border-base-300 px-3 py-2">
                      <div class="text-xs text-base-content/60">{{ key }}</div>
                      <div class="mt-1 break-all whitespace-pre-wrap text-sm">{{ formatValue(value) }}</div>
                    </div>
                  </div>
                  <div v-else class="text-sm text-base-content/60">{{ t('bugBounty.surface.detail.noTypedDetails') }}</div>
                </div>
              </section>

              <section class="card border border-base-300 bg-base-100">
                <div class="card-body">
                  <h4 class="card-title text-base">{{ t('bugBounty.surface.detail.relations') }}</h4>
                  <div class="overflow-x-auto">
                    <table class="table table-sm">
                      <thead>
                        <tr>
                          <th>{{ t('bugBounty.surface.detail.direction') }}</th>
                          <th>{{ t('bugBounty.surface.detail.relation') }}</th>
                          <th>{{ t('bugBounty.surface.detail.peerType') }}</th>
                          <th>{{ t('bugBounty.surface.detail.peerAsset') }}</th>
                        </tr>
                      </thead>
                      <tbody>
                        <tr v-for="relation in detail.relations" :key="relation.relation.id">
                          <td>{{ formatDirection(relation.direction) }}</td>
                          <td>{{ relation.relation.relation_type }}</td>
                          <td>{{ formatAssetType(relation.peer_asset.asset_type) }}</td>
                          <td class="font-mono text-xs break-all">
                            {{ relation.peer_asset.display_name || relation.peer_asset.asset_name }}
                          </td>
                        </tr>
                      </tbody>
                    </table>
                  </div>
                  <div v-if="!detail.relations.length" class="text-sm text-base-content/60">{{ t('bugBounty.surface.detail.noRelations') }}</div>
                </div>
              </section>

              <section class="grid grid-cols-1 gap-4 xl:grid-cols-3">
                <div class="card border border-base-300 bg-base-100">
                  <div class="card-body">
                    <h4 class="card-title text-base">{{ t('bugBounty.surface.detail.fingerprints') }}</h4>
                    <div v-if="detail.fingerprints.length" class="space-y-2">
                      <div v-for="fingerprint in detail.fingerprints" :key="fingerprint.id" class="rounded-lg border border-base-300 px-3 py-2">
                        <div class="text-xs text-base-content/60">
                          {{ fingerprint.fingerprint_type }} / {{ fingerprint.fingerprint_key || '-' }}
                        </div>
                        <div class="mt-1 break-all text-sm">{{ fingerprint.fingerprint_value }}</div>
                      </div>
                    </div>
                    <div v-else class="text-sm text-base-content/60">{{ t('bugBounty.surface.detail.noFingerprints') }}</div>
                  </div>
                </div>

                <div class="card border border-base-300 bg-base-100">
                  <div class="card-body">
                    <h4 class="card-title text-base">{{ t('bugBounty.surface.detail.evidence') }}</h4>
                    <div v-if="detail.evidence.length" class="space-y-2">
                      <div v-for="evidence in detail.evidence" :key="evidence.id" class="rounded-lg border border-base-300 px-3 py-2">
                        <div class="text-xs text-base-content/60">{{ evidence.evidence_type }}</div>
                        <div class="mt-1 text-sm">{{ evidence.title || '-' }}</div>
                        <div class="mt-1 whitespace-pre-wrap break-all text-xs text-base-content/70">
                          {{ evidence.content_text || formatJsonSnippet(evidence.content_json) }}
                        </div>
                      </div>
                    </div>
                    <div v-else class="text-sm text-base-content/60">{{ t('bugBounty.surface.detail.noEvidence') }}</div>
                  </div>
                </div>

                <div class="card border border-base-300 bg-base-100">
                  <div class="card-body">
                    <h4 class="card-title text-base">{{ t('bugBounty.surface.detail.changes') }}</h4>
                    <div v-if="detail.changes.length" class="space-y-2">
                      <div v-for="change in detail.changes" :key="change.id" class="rounded-lg border border-base-300 px-3 py-2">
                        <div class="text-xs text-base-content/60">{{ change.change_type }}</div>
                        <div class="mt-1 text-sm">{{ change.summary }}</div>
                        <div class="mt-1 text-xs text-base-content/60">{{ formatTime(change.detected_at) }}</div>
                      </div>
                    </div>
                    <div v-else class="text-sm text-base-content/60">{{ t('bugBounty.surface.detail.noChanges') }}</div>
                  </div>
                </div>
              </section>
            </div>
          </div>
        </div>
      </div>
    </div>
  </Teleport>
</template>

<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useI18n } from 'vue-i18n'

const props = defineProps<{
  visible: boolean
  assetId?: string | null
}>()

const { t } = useI18n()

defineEmits<{
  (e: 'close'): void
}>()

const loading = ref(false)
const error = ref('')
const detail = ref<any | null>(null)

const typedEntries = computed(() => {
  const typed = detail.value?.typed_details
  if (!typed || typeof typed !== 'object') return []
  return Object.entries(typed).filter(([key, value]) => key !== 'asset_id' && value !== null && value !== '')
})

const formatTime = (value?: string) => {
  if (!value) return '-'
  const date = new Date(value)
  if (Number.isNaN(date.getTime())) return value
  return date.toLocaleString()
}

const formatValue = (value: unknown) => {
  if (value === null || value === undefined || value === '') return '-'
  if (typeof value === 'object') {
    try {
      return JSON.stringify(value, null, 2)
    } catch {
      return String(value)
    }
  }
  return String(value)
}

const formatJsonSnippet = (value?: string | null) => {
  if (!value) return '-'
  return value.length > 180 ? `${value.slice(0, 180)}...` : value
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

const formatDirection = (value?: string) => {
  if (!value) return '-'
  const key = `bugBounty.surface.relationDirection.${value}`
  const translated = t(key)
  return translated === key ? value : translated
}

const loadDetail = async () => {
  if (!props.visible || !props.assetId) return
  try {
    loading.value = true
    error.value = ''
    detail.value = await invoke('surface_get_asset_detail', { assetId: props.assetId })
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err)
    error.value = `${t('bugBounty.surface.detail.loadFailed')}: ${message}`
    detail.value = null
  } finally {
    loading.value = false
  }
}

watch(
  () => [props.visible, props.assetId],
  () => {
    loadDetail()
  },
  { immediate: true },
)
</script>
