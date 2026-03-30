<template>
  <div class="card border border-base-300 bg-base-100 shadow-sm">
    <div class="card-body gap-4">
      <div class="flex flex-col gap-2 lg:flex-row lg:items-end lg:justify-between">
        <div>
          <h3 class="card-title text-base">{{ t('bugBounty.surface.types.title') }}</h3>
          <p class="text-sm text-base-content/60">{{ t('bugBounty.surface.types.description') }}</p>
        </div>
        <div class="text-sm text-base-content/70">
          {{ t('bugBounty.surface.types.classifiedTotal') }}: {{ aggregation.total_classified }}
        </div>
      </div>

      <div v-if="loading" class="flex items-center justify-center py-10">
        <span class="loading loading-spinner loading-md"></span>
      </div>

      <div v-else-if="error" class="alert alert-error">
        <span>{{ error }}</span>
      </div>

      <div v-else-if="!aggregation.categories.length" class="text-sm text-base-content/60">
        {{ t('bugBounty.surface.types.empty') }}
      </div>

      <div v-else class="grid gap-4 md:grid-cols-2 xl:grid-cols-4 2xl:grid-cols-5">
        <section
          v-for="bucket in aggregation.categories"
          :key="bucket.category"
          class="overflow-hidden rounded-xl border border-base-300 bg-base-200/40"
        >
          <button
            class="flex w-full items-center justify-between gap-3 border-b border-base-300 bg-base-300/30 px-4 py-3 text-left"
            @click="openCategory(bucket.category)"
          >
            <span class="font-semibold text-primary">{{ formatCategory(bucket.category) }}</span>
            <span class="text-sm text-base-content/70">{{ bucket.count }}</span>
          </button>

          <div class="divide-y divide-base-300/80">
            <button
              v-for="product in bucket.top_products"
              :key="`${bucket.category}:${product.product}`"
              class="flex w-full items-center justify-between gap-3 px-4 py-3 text-left transition hover:bg-base-100"
              @click="openProduct(bucket.category, product.product)"
            >
              <span class="truncate text-sm">{{ product.product }}</span>
              <span class="shrink-0 text-xs text-base-content/60">{{ product.count }}</span>
            </button>
            <div v-if="!bucket.top_products.length" class="px-4 py-6 text-sm text-base-content/50">
              {{ t('bugBounty.surface.types.noProducts') }}
            </div>
          </div>

          <div class="border-t border-base-300 px-4 py-3">
            <button class="btn btn-ghost btn-sm w-full text-primary" @click="openCategory(bucket.category)">
              {{ t('bugBounty.surface.types.more') }}
            </button>
          </div>
        </section>
      </div>
    </div>
  </div>

  <SurfaceFingerprintAssetsModal
    :visible="showAssetsModal"
    :program-id="programId || null"
    :category="selectedCategory"
    :product="selectedProduct"
    @close="closeAssetsModal"
    @open-asset="emit('open-asset', $event)"
  />
</template>

<script setup lang="ts">
import { ref, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useI18n } from 'vue-i18n'
import SurfaceFingerprintAssetsModal from './SurfaceFingerprintAssetsModal.vue'

interface SurfaceFingerprintProductBucket {
  product: string
  count: number
}

interface SurfaceFingerprintCategoryBucket {
  category: string
  count: number
  top_products: SurfaceFingerprintProductBucket[]
}

interface SurfaceFingerprintCategoryAggregation {
  total_classified: number
  categories: SurfaceFingerprintCategoryBucket[]
}

const props = defineProps<{
  programId?: string | null
  refreshToken?: number
}>()

const emit = defineEmits<{
  (e: 'open-asset', assetId: string): void
}>()

const { t } = useI18n()

const loading = ref(false)
const error = ref('')
const aggregation = ref<SurfaceFingerprintCategoryAggregation>({
  total_classified: 0,
  categories: [],
})
const showAssetsModal = ref(false)
const selectedCategory = ref<string | null>(null)
const selectedProduct = ref<string | null>(null)

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

const loadAggregation = async () => {
  try {
    loading.value = true
    error.value = ''
    aggregation.value = await invoke<SurfaceFingerprintCategoryAggregation>(
      'surface_get_fingerprint_category_aggregation',
      {
        programId: props.programId || null,
        categoryLimit: 15,
        productLimit: 10,
      },
    )
  } catch (err) {
    error.value = err instanceof Error ? err.message : String(err)
    aggregation.value = { total_classified: 0, categories: [] }
  } finally {
    loading.value = false
  }
}

const openCategory = (category: string) => {
  selectedCategory.value = category
  selectedProduct.value = null
  showAssetsModal.value = true
}

const openProduct = (category: string, product: string) => {
  selectedCategory.value = category
  selectedProduct.value = product
  showAssetsModal.value = true
}

const closeAssetsModal = () => {
  showAssetsModal.value = false
  selectedCategory.value = null
  selectedProduct.value = null
}

watch(
  () => [props.programId, props.refreshToken],
  () => {
    loadAggregation()
  },
  { immediate: true },
)
</script>
