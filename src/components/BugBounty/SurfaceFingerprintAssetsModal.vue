<template>
  <Teleport to="body">
    <div v-if="visible" class="fixed inset-0 z-[70]">
      <div class="absolute inset-0 bg-black/45 backdrop-blur-sm" @click="$emit('close')"></div>

      <div class="relative flex min-h-full items-start justify-center overflow-y-auto px-4 py-20 md:px-6 md:py-24">
        <div class="modal-box relative flex max-h-[calc(100vh-6rem)] w-full max-w-6xl flex-col overflow-hidden p-0 md:max-h-[calc(100vh-8rem)]">
          <div class="sticky top-0 z-10 border-b border-base-300 bg-base-100/95 px-6 py-4 backdrop-blur">
            <div class="flex items-start justify-between gap-4">
              <div class="min-w-0">
                <h3 class="text-lg font-semibold">{{ modalTitle }}</h3>
                <p class="mt-1 text-xs text-base-content/60">
                  {{ t('bugBounty.surface.types.classifiedTotal') }}: {{ response.total }}
                </p>
              </div>
              <button class="btn btn-sm btn-ghost shrink-0" @click="$emit('close')">✕</button>
            </div>
          </div>

          <div v-if="loading" class="flex flex-1 items-center justify-center py-12">
            <span class="loading loading-spinner loading-md"></span>
          </div>

          <div v-else-if="error" class="flex-1 overflow-y-auto px-6 py-4">
            <div class="alert alert-error">
              <span>{{ error }}</span>
            </div>
          </div>

          <div v-else class="min-h-0 flex-1 overflow-y-auto px-6 pb-6">
            <div class="overflow-x-auto pt-4">
              <table class="table table-sm">
                <thead>
                  <tr>
                    <th>{{ t('bugBounty.surface.columns.type') }}</th>
                    <th>{{ t('bugBounty.surface.columns.name') }}</th>
                    <th>{{ t('bugBounty.surface.types.matchedProducts') }}</th>
                    <th>{{ t('bugBounty.surface.types.matchedRules') }}</th>
                    <th>{{ t('bugBounty.surface.columns.count') }}</th>
                    <th>{{ t('bugBounty.surface.types.matchedAt') }}</th>
                  </tr>
                </thead>
                <tbody>
                  <tr
                    v-for="item in response.items"
                    :key="item.asset.id"
                    class="cursor-pointer hover"
                    @click="$emit('open-asset', item.asset.id)"
                  >
                    <td>
                      <span class="badge badge-outline badge-sm">{{ formatAssetType(item.asset.asset_type) }}</span>
                    </td>
                    <td class="max-w-[24rem]">
                      <div class="font-medium">{{ item.asset.display_name || item.asset.asset_name }}</div>
                      <div class="truncate font-mono text-xs text-base-content/60">
                        {{ primaryValue(item) }}
                      </div>
                    </td>
                    <td class="max-w-[16rem]">
                      <div class="flex flex-wrap gap-1">
                        <span
                          v-for="product in item.matched_products.slice(0, 4)"
                          :key="product"
                          class="badge badge-primary badge-outline badge-sm"
                        >
                          {{ product }}
                        </span>
                        <span v-if="!item.matched_products.length" class="text-xs text-base-content/50">-</span>
                      </div>
                    </td>
                    <td class="max-w-[16rem]">
                      <div class="text-xs text-base-content/80">
                        {{ item.matched_rule_names.slice(0, 3).join(' / ') || '-' }}
                      </div>
                    </td>
                    <td>{{ item.fingerprint_count }}</td>
                    <td>{{ formatTime(item.last_observed_at || item.asset.last_seen_at) }}</td>
                  </tr>
                </tbody>
              </table>
            </div>

            <div v-if="!response.items.length" class="pt-4 text-sm text-base-content/60">
              {{ t('bugBounty.surface.types.drilldownEmpty') }}
            </div>

            <div v-if="response.total > 0" class="flex flex-col gap-3 pt-4 sm:flex-row sm:items-center sm:justify-between">
              <div class="text-sm text-base-content/70">
                {{ t('bugBounty.surface.inventory.pageInfo', { page, total: pageCount }) }}
              </div>
              <div class="join">
                <button class="join-item btn btn-sm" :disabled="page <= 1 || loading" @click="setPage(1)">
                  {{ t('bugBounty.surface.inventory.firstPage') }}
                </button>
                <button class="join-item btn btn-sm" :disabled="page <= 1 || loading" @click="setPage(page - 1)">
                  {{ t('common.previous') }}
                </button>
                <button class="join-item btn btn-sm" :disabled="page >= pageCount || loading" @click="setPage(page + 1)">
                  {{ t('common.next') }}
                </button>
                <button class="join-item btn btn-sm" :disabled="page >= pageCount || loading" @click="setPage(pageCount)">
                  {{ t('bugBounty.surface.inventory.lastPage') }}
                </button>
              </div>
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

interface SurfaceFingerprintAssetInventoryItem {
  asset: Record<string, any>
  typed_details: Record<string, any> | null
  matched_products: string[]
  matched_vendors: string[]
  matched_rule_names: string[]
  fingerprint_count: number
  last_observed_at: string | null
}

interface SurfaceFingerprintAssetInventoryResponse {
  items: SurfaceFingerprintAssetInventoryItem[]
  total: number
}

const props = defineProps<{
  visible: boolean
  programId?: string | null
  category: string | null
  product?: string | null
}>()

defineEmits<{
  (e: 'close'): void
  (e: 'open-asset', assetId: string): void
}>()

const { t } = useI18n()

const loading = ref(false)
const error = ref('')
const response = ref<SurfaceFingerprintAssetInventoryResponse>({ items: [], total: 0 })
const page = ref(1)
const pageSize = 10

const pageCount = computed(() => Math.max(1, Math.ceil(response.value.total / pageSize)))
const modalTitle = computed(() => {
  const categoryLabel = formatCategory(props.category)
  if (props.product) {
    return `${categoryLabel} / ${props.product}`
  }
  return `${categoryLabel} / ${t('bugBounty.surface.types.allProducts')}`
})

const formatCategory = (value?: string | null) => {
  if (!value) return '-'
  const key = `bugBounty.surface.fingerprintCategories.${value}`
  const translated = t(key)
  if (translated !== key) return translated
  return value
    .split('_')
    .filter(Boolean)
    .map((item) => item.charAt(0).toUpperCase() + item.slice(1))
    .join(' ')
}

const formatAssetType = (value?: string | null) => {
  if (!value) return '-'
  const key = `bugBounty.surface.assetTypes.${value}`
  const translated = t(key)
  return translated === key ? value : translated
}

const formatTime = (value?: string | null) => {
  if (!value) return '-'
  const date = new Date(value)
  if (Number.isNaN(date.getTime())) return value
  return date.toLocaleString()
}

const primaryValue = (item: SurfaceFingerprintAssetInventoryItem) => {
  const details = item.typed_details || {}
  return (
    details.canonical_url ||
    details.application_service_name ||
    details.product_name ||
    details.hostname ||
    details.fqdn ||
    details.ip_address ||
    item.asset.asset_name ||
    '-'
  )
}

const loadAssets = async () => {
  if (!props.visible || !props.category) return

  try {
    loading.value = true
    error.value = ''
    response.value = await invoke<SurfaceFingerprintAssetInventoryResponse>('surface_list_fingerprint_assets', {
      filter: {
        program_id: props.programId || null,
        category: props.category,
        product: props.product || null,
        limit: pageSize,
        offset: (page.value - 1) * pageSize,
      },
    })
  } catch (err) {
    error.value = err instanceof Error ? err.message : t('bugBounty.surface.types.drilldownLoadFailed')
    response.value = { items: [], total: 0 }
  } finally {
    loading.value = false
  }
}

const setPage = (nextPage: number) => {
  page.value = Math.min(Math.max(1, nextPage), pageCount.value)
}

watch(
  () => [props.visible, props.programId, props.category, props.product],
  () => {
    page.value = 1
    loadAssets()
  },
  { immediate: true },
)

watch(page, () => {
  loadAssets()
})
</script>
