<template>
  <Teleport to="body">
    <div v-if="visible" class="fixed inset-0 z-[70]">
      <div class="absolute inset-0 bg-black/45 backdrop-blur-sm" @click="$emit('close')"></div>

      <div class="relative flex min-h-full items-start justify-center overflow-y-auto px-4 py-20 md:px-6 md:py-24">
        <div class="modal-box relative flex max-h-[calc(100vh-6rem)] w-full max-w-5xl flex-col overflow-hidden p-0 md:max-h-[calc(100vh-8rem)]">
          <div class="sticky top-0 z-10 border-b border-base-300 bg-base-100/95 px-6 py-4 backdrop-blur">
            <div class="flex items-start justify-between gap-4">
              <div class="min-w-0">
                <h3 class="text-lg font-semibold">{{ title }}</h3>
                <p class="mt-1 text-xs text-base-content/60">{{ totalCount }}</p>
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
            <div class="pt-4">
              <div class="overflow-x-auto">
                <table class="table table-sm">
                  <thead>
                    <tr>
                      <th>{{ t('bugBounty.surface.columns.type') }}</th>
                      <th>{{ t('bugBounty.surface.columns.count') }}</th>
                    </tr>
                  </thead>
                  <tbody>
                    <tr v-for="item in typeCounts" :key="item.type">
                      <td><span class="badge badge-outline badge-sm">{{ formatAssetType(item.type) }}</span></td>
                      <td>{{ item.count }}</td>
                    </tr>
                  </tbody>
                </table>
              </div>
              <div v-if="!typeCounts.length" class="text-sm text-base-content/60">{{ t('bugBounty.surface.inventory.empty') }}</div>
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
  title: string
  programId?: string | null
  statusFilter?: 'all' | 'active'
}>()

defineEmits<{
  (e: 'close'): void
}>()

const { t } = useI18n()

const loading = ref(false)
const error = ref('')
const assets = ref<any[]>([])
const typeCounts = computed(() => {
  const counts = new Map<string, number>()
  for (const asset of assets.value) {
    const type = String(asset.asset_type || 'unknown')
    counts.set(type, (counts.get(type) || 0) + 1)
  }
  return [...counts.entries()]
    .map(([type, count]) => ({ type, count }))
    .sort((a, b) => b.count - a.count || a.type.localeCompare(b.type))
})
const totalCount = computed(() => typeCounts.value.reduce((sum, item) => sum + item.count, 0))

const formatAssetType = (value?: string) => {
  if (!value) return '-'
  const key = `bugBounty.surface.assetTypes.${value}`
  const translated = t(key)
  return translated === key ? value : translated
}

const loadAssets = async () => {
  if (!props.visible) return
  try {
    loading.value = true
    error.value = ''
    if (props.statusFilter === 'active') {
      const rows = await invoke<any[]>('surface_list_assets', {
        filter: {
          program_id: props.programId || null,
          asset_type: null,
          status: 'active',
          search: null,
          limit: null,
          offset: null,
        },
      })
      assets.value = rows
      return
    }

    const overview = await invoke<any>('surface_get_overview', {
      programId: props.programId || null,
    })

    const byType = overview?.by_type || {}
    assets.value = Object.entries(byType).flatMap(([asset_type, count]) =>
      Array.from({ length: Number(count || 0) }, () => ({ asset_type })),
    )
  } catch (err) {
    error.value = err instanceof Error ? err.message : String(err)
    assets.value = []
  } finally {
    loading.value = false
  }
}

watch(
  () => [props.visible, props.programId, props.statusFilter],
  () => {
    loadAssets()
  },
  { immediate: true },
)
</script>
