<template>
  <Teleport to="body">
    <div v-if="visible" class="fixed inset-0 z-[70]">
      <div class="absolute inset-0 bg-black/45 backdrop-blur-sm" @click="$emit('close')"></div>

      <div
        class="relative flex min-h-full items-start justify-center overflow-y-auto px-4 py-20 md:px-6 md:py-24"
        @click.self="$emit('close')"
      >
        <div class="modal-box relative flex max-h-[calc(100vh-6rem)] w-full max-w-6xl flex-col overflow-hidden p-0 md:max-h-[calc(100vh-8rem)]">
          <div class="sticky top-0 z-10 border-b border-base-300 bg-base-100/95 px-6 py-4 backdrop-blur">
            <div class="flex items-start justify-between gap-4">
              <div class="min-w-0">
                <h3 class="text-lg font-semibold">{{ t('bugBounty.surface.inventory.faviconAssets.title') }}</h3>
                <p class="mt-1 break-all font-mono text-xs text-base-content/60">
                  {{ faviconHash }}
                </p>
                <p class="mt-1 text-xs text-base-content/60">
                  {{ t('bugBounty.surface.types.classifiedTotal') }}: {{ total }}
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
                    <th>{{ t('bugBounty.surface.inventory.fields.canonicalUrl') }}</th>
                    <th>{{ t('bugBounty.surface.inventory.fields.siteTitle') }}</th>
                    <th>{{ t('bugBounty.surface.inventory.fields.httpStatusCode') }}</th>
                    <th>{{ t('bugBounty.surface.columns.lastSeen') }}</th>
                  </tr>
                </thead>
                <tbody>
                  <tr
                    v-for="item in response.items"
                    :key="item.asset.id"
                    class="cursor-pointer hover"
                    @click="$emit('open-asset', item.asset.id)"
                  >
                    <td class="max-w-[28rem] font-mono text-xs break-all">
                      {{ item.typed_details?.canonical_url || item.asset.asset_name || '-' }}
                    </td>
                    <td class="max-w-[20rem]">
                      <div class="truncate">{{ item.typed_details?.site_title || item.asset.display_name || '-' }}</div>
                    </td>
                    <td>{{ item.typed_details?.http_status_code ?? '-' }}</td>
                    <td>{{ formatTime(item.asset.last_seen_at) }}</td>
                  </tr>
                </tbody>
              </table>
            </div>

            <div v-if="!response.items.length" class="pt-4 text-sm text-base-content/60">
              {{ t('bugBounty.surface.inventory.faviconAssets.empty') }}
            </div>

            <div v-if="response.items.length > 0 || total > 0" class="flex flex-col gap-3 pt-4 sm:flex-row sm:items-center sm:justify-between">
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
                <button class="join-item btn btn-sm" :disabled="!response.has_next || loading" @click="setPage(page + 1)">
                  {{ t('common.next') }}
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

interface SurfaceInventoryItem {
  asset: Record<string, any>
  typed_details: Record<string, any> | null
}

interface SurfaceInventoryResponse {
  items: SurfaceInventoryItem[]
  total?: number | null
  has_next: boolean
  next_cursor?: SurfaceInventoryCursor | null
}

interface SurfaceInventoryCursor {
  last_seen_at: string
  id: string
}

const props = defineProps<{
  visible: boolean
  programId?: string | null
  faviconHash: string | null
}>()

defineEmits<{
  (e: 'close'): void
  (e: 'open-asset', assetId: string): void
}>()

const { t } = useI18n()

const loading = ref(false)
const error = ref('')
const response = ref<SurfaceInventoryResponse>({ items: [], total: null, has_next: false, next_cursor: null })
const total = ref(0)
const page = ref(1)
const pageSize = 10
const cursorByPage = ref<Record<number, SurfaceInventoryCursor | null>>({ 1: null })
const nextCursorByPage = ref<Record<number, SurfaceInventoryCursor | null>>({})

const pageCount = computed(() => Math.max(1, Math.ceil(total.value / pageSize)))

const formatTime = (value?: string | null) => {
  if (!value) return '-'
  const date = new Date(value)
  if (Number.isNaN(date.getTime())) return value
  return date.toLocaleString()
}

const loadAssets = async () => {
  const faviconHash = props.faviconHash?.trim()
  if (!props.visible || !faviconHash) return

  try {
    loading.value = true
    error.value = ''
    const cursor = cursorByPage.value[page.value] || null
    response.value = await invoke<SurfaceInventoryResponse>('surface_list_inventory', {
      filter: {
        program_id: props.programId || null,
        asset_type: 'web',
        status: null,
        search: null,
        favicon_hash: faviconHash,
        service_name: null,
        transport_protocol: null,
        view_state: null,
        is_favorite: null,
        column_filters: null,
        limit: pageSize,
        offset: null,
      },
      cursor,
    })
    if (response.value.has_next && response.value.next_cursor) {
      nextCursorByPage.value[page.value] = response.value.next_cursor
      cursorByPage.value[page.value + 1] = response.value.next_cursor
    }
  } catch (err) {
    error.value = err instanceof Error ? err.message : t('bugBounty.surface.inventory.faviconAssets.loadFailed')
    response.value = { items: [], total: null, has_next: false, next_cursor: null }
  } finally {
    loading.value = false
  }
}

const setPage = (nextPage: number) => {
  const targetPage = Math.max(1, nextPage)
  if (targetPage > page.value && !nextCursorByPage.value[page.value]) return
  if (targetPage === 1) {
    cursorByPage.value = { 1: null }
    nextCursorByPage.value = {}
  }
  page.value = Math.min(targetPage, Math.max(page.value + 1, pageCount.value))
}

const loadTotal = async () => {
  const faviconHash = props.faviconHash?.trim()
  if (!props.visible || !faviconHash) return
  try {
    total.value = await invoke<number>('surface_count_assets', {
      filter: {
        program_id: props.programId || null,
        asset_type: 'web',
        status: null,
        search: null,
        favicon_hash: faviconHash,
        has_favicon_hash: null,
        http_status_code: null,
        service_name: null,
        transport_protocol: null,
        view_state: null,
        is_favorite: null,
        column_filters: null,
        limit: null,
        offset: null,
      },
    })
  } catch (err) {
    total.value = 0
  }
}

watch(
  () => [props.visible, props.programId, props.faviconHash],
  () => {
    page.value = 1
    total.value = 0
    cursorByPage.value = { 1: null }
    nextCursorByPage.value = {}
    loadAssets()
    loadTotal()
  },
  { immediate: true },
)

watch(page, () => {
  loadAssets()
})
</script>
