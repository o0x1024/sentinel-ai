<template>
  <div>
    <!-- View Mode Toggle -->
    <div v-if="selectedCategory === 'all'" class="mb-4 flex flex-wrap items-center gap-2">
      <button class="btn btn-sm" :class="pluginViewMode === 'favorited' ? 'btn-primary' : 'btn-ghost'"
        @click="$emit('update:pluginViewMode', 'favorited')">
        <i class="fas fa-star mr-1"></i>
        {{ $t('plugins.favorited', '已收藏') }}
      </button>
      <button class="btn btn-sm" :class="pluginViewMode === 'all' ? 'btn-primary' : 'btn-ghost'"
        @click="$emit('update:pluginViewMode', 'all')">
        <i class="fas fa-list mr-1"></i>
        {{ $t('plugins.allPlugins', '全部插件') }}
      </button>

      <div class="mx-1 hidden h-6 w-px bg-base-300 sm:block"></div>

      <button
        class="btn btn-sm"
        :class="selectedMainCategory === '' ? 'btn-secondary' : 'btn-ghost'"
        @click="$emit('update:mainCategory', '')"
      >
        {{ $t('plugins.allMainCategories', '全部分类') }}
      </button>
      <button
        v-for="mainCat in availableMainCategories"
        :key="mainCat.value"
        class="btn btn-sm"
        :class="selectedMainCategory === mainCat.value ? 'btn-secondary' : 'btn-ghost'"
        @click="$emit('update:mainCategory', mainCat.value)"
      >
        <i :class="mainCat.icon" class="mr-1"></i>
        {{ mainCat.label }}
      </button>
    </div>

    <!-- Search and Filter Bar -->
    <div class="flex gap-2 mb-4 flex-wrap items-center">
      <input v-model="localSearchText" type="text" :placeholder="$t('plugins.searchPlugins', '搜索插件...')"
        class="input input-bordered input-sm flex-1 min-w-48" @input="onSearchInput" />

      <!-- Tag Filter Dropdown -->
      <select v-model="localTag" class="select select-bordered select-sm" @change="onTagChange">
        <option value="">全部标签</option>
        <option v-for="tag in availableTags" :key="tag" :value="tag">
          {{ tag }}
        </option>
      </select>

      <!-- Clear Filters Button -->
      <button v-if="pluginSearchText || selectedMainCategory || selectedSubCategory || selectedTag" class="btn btn-sm btn-ghost"
        @click="$emit('clearFilters')">
        <i class="fas fa-times mr-1"></i>
        {{ $t('plugins.clearFilters', '清除筛选') }}
      </button>

      <!-- Batch Toggle Buttons -->
      <div v-if="selectedCategory === 'all'" class="ml-auto flex gap-2">
        <button class="btn btn-sm btn-success" :disabled="filteredPlugins.length === 0 || pluginBatchProcessing"
          @click="$emit('batchEnable')">
          <span v-if="batchToggling" class="loading loading-spinner"></span>
          全部开启
        </button>
        <button class="btn btn-sm btn-warning" :disabled="filteredPlugins.length === 0 || pluginBatchProcessing"
          @click="$emit('batchDisable')">
          <span v-if="batchToggling" class="loading loading-spinner"></span>
          全部停止
        </button>
      </div>
    </div>

    <div v-if="availableSubCategories.length" class="mb-4 flex flex-wrap items-center gap-2">
      <button
        class="btn btn-sm"
        :class="selectedSubCategory === '' ? 'btn-accent' : 'btn-ghost'"
        @click="$emit('update:subCategory', '')"
      >
        {{ $t('plugins.allSubCategories', '全部子分类') }}
      </button>
      <button
        v-for="subCat in availableSubCategories"
        :key="subCat"
        class="btn btn-sm"
        :class="selectedSubCategory === subCat ? 'btn-accent' : 'btn-ghost'"
        @click="$emit('update:subCategory', subCat)"
      >
        {{ subCat }}
      </button>
    </div>

    <div
      v-if="selectedPluginIds.length > 0"
      class="mb-4 flex flex-col gap-3 rounded-lg border border-base-300 bg-base-200/40 p-3 lg:flex-row lg:items-center lg:justify-between"
    >
      <div class="text-sm text-base-content/70">
        {{ $t('plugins.selectionSummary', { selected: selectedPluginIds.length, page: paginatedPlugins.length, total: filteredPlugins.length }) }}
      </div>
      <div class="flex flex-wrap items-center gap-2">
        <button
          class="btn btn-sm btn-success btn-outline"
          :disabled="selectedPluginIds.length === 0 || pluginBatchProcessing"
          @click="$emit('batchEnableSelected')"
        >
          <span v-if="batchToggling" class="loading loading-spinner loading-xs"></span>
          <i v-else class="fas fa-play mr-2"></i>
          {{ $t('plugins.batchEnableSelected', '启用选中') }}
        </button>
        <button
          class="btn btn-sm btn-warning btn-outline"
          :disabled="selectedPluginIds.length === 0 || pluginBatchProcessing"
          @click="$emit('batchDisableSelected')"
        >
          <span v-if="batchToggling" class="loading loading-spinner loading-xs"></span>
          <i v-else class="fas fa-pause mr-2"></i>
          {{ $t('plugins.batchDisableSelected', '停用选中') }}
        </button>
        <button
          class="btn btn-sm btn-error"
          v-if="canDeletePlugins"
          :disabled="selectedPluginIds.length === 0 || pluginBatchProcessing"
          @click="$emit('batchDeleteSelected')"
        >
          <span v-if="batchDeleting" class="loading loading-spinner loading-xs"></span>
          <i v-else class="fas fa-trash mr-2"></i>
          {{ $t('plugins.batchDeleteSelected', '删除选中') }}
        </button>
      </div>
    </div>

    <!-- Pagination and Page Size Control -->
    <div v-if="filteredPlugins.length > 0" class="flex justify-between items-center mb-4">
      <div class="text-sm text-base-content/70">
        {{ $t('plugins.showing', '显示') }} {{ paginationInfo.start }}-{{ paginationInfo.end }}
        {{ $t('plugins.of', '共') }} {{ paginationInfo.total }} {{ $t('plugins.items', '条') }}
      </div>
      <div class="flex items-center gap-2">
        <span class="text-sm">{{ $t('plugins.pageSize', '每页') }}:</span>
        <select :value="pageSize" @change="$emit('changePageSize', Number(($event.target as HTMLSelectElement).value))"
          class="select select-bordered select-sm">
          <option :value="10">10</option>
          <option :value="20">20</option>
          <option :value="50">50</option>
          <option :value="100">100</option>
        </select>
      </div>
    </div>

    <!-- Plugin List -->
    <div v-if="filteredPlugins.length === 0" class="alert alert-info">
      <i class="fas fa-info-circle"></i>
      <span>{{ $t('plugins.noPlugins', '暂无插件，请上传或扫描插件目录') }}</span>
    </div>

    <div v-else class="overflow-x-auto">
      <table class="table table-zebra w-full">
        <thead>
          <tr>
            <th class="w-12">
              <input
                type="checkbox"
                class="checkbox checkbox-sm"
                :checked="isAllCurrentPageSelected"
                :disabled="paginatedPlugins.length === 0 || pluginBatchProcessing"
                @change="$emit('toggleSelectAllCurrentPage')"
              />
            </th>
            <th class="w-12">{{ $t('common.status', '状态') }}</th>
            <th class="w-40">{{ $t('plugins.pluginName', '插件名称') }}</th>
            <th class="w-24">{{ $t('plugins.version', '版本') }}</th>
            <th class="w-44">{{ $t('plugins.category', '分类') }}</th>
            <th class="w-32">{{ $t('plugins.author', '作者') }}</th>
            <th class="w-48">{{ $t('plugins.tags', '标签') }}</th>
            <th class="w-80">{{ $t('common.actions', '操作') }}</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="plugin in paginatedPlugins" :key="plugin.metadata.id">
            <td>
              <input
                type="checkbox"
                class="checkbox checkbox-sm"
                :checked="isPluginSelected(plugin)"
                :disabled="pluginBatchProcessing"
                @change="$emit('toggleSelection', plugin)"
              />
            </td>
            <!-- Status Indicator -->
            <td>
              <div class="flex items-center gap-2">
                <div class="tooltip" :data-tip="getStatusText(plugin.status)">
                  <div class="w-3 h-3 rounded-full" :class="{
                    'bg-success': plugin.status === 'Enabled',
                    'bg-warning': plugin.status === 'Disabled',
                    'bg-error': plugin.status === 'Error'
                  }"></div>
                </div>
              </div>
            </td>

            <!-- Plugin Name -->
            <td>
              <div class="flex items-center gap-2">
                <div class="flex-1">
                  <div class="font-bold">{{ plugin.metadata.name }}</div>
                  <div class="text-sm text-gray-500">{{ plugin.metadata.id }}</div>
                  <div v-if="plugin.metadata.description" class="text-xs text-gray-400 mt-1">
                    {{ plugin.metadata.description }}
                  </div>
                </div>
                <!-- Active Indicator -->
                <div v-if="isToolActive(plugin.metadata.id)" class="tooltip" data-tip="使用中">
                  <div class="badge badge-success badge-sm gap-1">
                    <span class="loading loading-spinner loading-xs"></span>
                    {{ getActiveCount(plugin.metadata.id) }}
                  </div>
                </div>
              </div>
            </td>

            <!-- Version -->
            <td>
              <span class="badge badge-outline">{{ plugin.metadata.version }}</span>
            </td>

            <!-- Category -->
            <td>
              <div class="flex flex-wrap gap-2">
                <span class="badge gap-1" :class="getMainCategoryBadgeClass(plugin.metadata.main_category)">
                  <i :class="getMainCategoryIcon(plugin.metadata.main_category)"></i>
                  {{ getMainCategoryLabel(plugin.metadata.main_category) }}
                </span>
                <span class="badge badge-outline whitespace-nowrap">
                  {{ getCategoryLabel(plugin.metadata.category) }}
                </span>
              </div>
            </td>

            <!-- Author -->
            <td>{{ plugin.metadata.author || '-' }}</td>

            <!-- Tags -->
            <td>
              <div class="flex flex-wrap gap-1 max-w-xs">
                <span v-for="tag in plugin.metadata.tags.slice(0, 3)" :key="tag"
                  class="badge badge-sm badge-ghost whitespace-nowrap">
                  {{ tag }}
                </span>
                <span v-if="plugin.metadata.tags.length > 3" class="badge badge-sm badge-outline tooltip"
                  :data-tip="plugin.metadata.tags.slice(3).join(', ')">
                  +{{ plugin.metadata.tags.length - 3 }}
                </span>
              </div>
            </td>

            <!-- Action Buttons -->
            <td>
              <div class="flex gap-1 flex-wrap">
                <!-- Favorite Button -->
                <div v-if="isTrafficPluginType(plugin) || isAgentPluginType(plugin)" class="tooltip"
                  :data-tip="isPluginFavorited(plugin) ? $t('plugins.unfavorite', '取消收藏') : $t('plugins.favorite', '收藏插件')">
                  <button class="btn btn-sm btn-ghost" :disabled="pluginBatchProcessing" @click="$emit('toggleFavorite', plugin)">
                    <i :class="isPluginFavorited(plugin) ? 'fas fa-star text-yellow-500' : 'far fa-star'"></i>
                  </button>
                </div>

                <!-- Test Plugin -->
                <div class="tooltip"
                  :data-tip="isAgentPluginType(plugin) ? '测试 Agent 工具 (analyze)' : '测试流量分析 (scan_request/scan_response)'">
                  <button class="btn btn-sm btn-outline" :disabled="pluginBatchProcessing" @click="$emit('testPlugin', plugin)">
                    <i class="fas fa-vial mr-1"></i>
                  </button>
                </div>

                <div class="tooltip" :data-tip="$t('plugins.configureDefaults', '配置默认值')">
                  <button class="btn btn-sm btn-outline" :disabled="pluginBatchProcessing" @click="$emit('configureDefaults', plugin)">
                    <i class="fas fa-sliders-h mr-1"></i>
                  </button>
                </div>

                <!-- Advanced Test -->
                <div class="tooltip" :data-tip="isAgentPluginType(plugin) ? 'Agent 高级测试' : '流量分析高级测试'">
                  <button class="btn btn-sm btn-outline" :disabled="pluginBatchProcessing" @click="$emit('advancedTest', plugin)">
                    <i class="fas fa-gauge-high mr-1"></i>
                  </button>
                </div>

                <!-- Enable/Disable Toggle -->
                <button class="btn btn-sm" :class="plugin.status === 'Enabled' ? 'btn-warning' : 'btn-success'"
                  :disabled="pluginBatchProcessing"
                  @click="$emit('togglePlugin', plugin)">
                  <i :class="plugin.status === 'Enabled' ? 'fas fa-pause' : 'fas fa-play'" class="mr-1"></i>
                </button>

                <!-- View/Edit Code -->
                <button
                  v-if="canViewPluginCode"
                  class="btn btn-sm btn-info"
                  :disabled="pluginBatchProcessing"
                  @click="$emit('viewCode', plugin)"
                >
                  <i class="fas fa-code mr-1"></i>
                </button>

                <!-- Delete -->
                <button
                  v-if="canDeletePlugins"
                  class="btn btn-sm btn-error"
                  :disabled="pluginBatchProcessing"
                  @click="$emit('deletePlugin', plugin)"
                >
                  <i class="fas fa-trash"></i>
                </button>
              </div>
            </td>
          </tr>
        </tbody>
      </table>

      <!-- Pagination -->
      <div v-if="totalPages > 1" class="flex justify-center mt-4">
        <div class="join">
          <button class="join-item btn btn-sm" :disabled="currentPage === 1" @click="$emit('goToPage', 1)">
            <i class="fas fa-angle-double-left"></i>
          </button>
          <button class="join-item btn btn-sm" :disabled="currentPage === 1" @click="$emit('goToPage', currentPage - 1)">
            <i class="fas fa-angle-left"></i>
          </button>
          <template v-for="page in visiblePages" :key="page">
            <button v-if="page === '...'" class="join-item btn btn-sm btn-disabled">...</button>
            <button v-else class="join-item btn btn-sm" :class="{ 'btn-active': currentPage === page }"
              @click="$emit('goToPage', page as number)">
              {{ page }}
            </button>
          </template>
          <button class="join-item btn btn-sm" :disabled="currentPage === totalPages"
            @click="$emit('goToPage', currentPage + 1)">
            <i class="fas fa-angle-right"></i>
          </button>
          <button class="join-item btn btn-sm" :disabled="currentPage === totalPages"
            @click="$emit('goToPage', totalPages)">
            <i class="fas fa-angle-double-right"></i>
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import type { PluginRecord } from './types'
import { useActiveTools } from '@/composables/useActiveTools'

const props = defineProps<{
  selectedCategory: string
  pluginViewMode: 'favorited' | 'all'
  filteredPlugins: PluginRecord[]
  paginatedPlugins: PluginRecord[]
  selectedPluginIds: string[]
  paginationInfo: { start: number; end: number; total: number }
  currentPage: number
  pageSize: number
  totalPages: number
  pluginSearchText: string
  selectedMainCategory: string
  selectedSubCategory: string
  selectedTag: string
  availableMainCategories: Array<{ value: string; label: string; icon: string }>
  availableSubCategories: string[]
  availableTags: string[]
  batchToggling: boolean
  batchDeleting: boolean
  pluginBatchProcessing: boolean
  isAllCurrentPageSelected: boolean
  getStatusText: (status: string) => string
  getMainCategoryLabel: (mainCategory: string) => string
  getMainCategoryIcon: (mainCategory: string) => string
  getMainCategoryBadgeClass: (mainCategory: string) => string
  getCategoryLabel: (category: string) => string
  getCategoryIcon: (category: string) => string
  isPluginSelected: (plugin: PluginRecord) => boolean
  isPluginFavorited: (plugin: PluginRecord) => boolean
  isTrafficPluginType: (plugin: PluginRecord) => boolean
  isAgentPluginType: (plugin: PluginRecord) => boolean
  canDeletePlugins: boolean
  canViewPluginCode: boolean
}>()

const emit = defineEmits<{
  'update:pluginViewMode': [value: 'favorited' | 'all']
  'update:searchText': [value: string]
  'update:mainCategory': [value: string]
  'update:subCategory': [value: string]
  'update:tag': [value: string]
  clearFilters: []
  batchEnable: []
  batchDisable: []
  batchEnableSelected: []
  batchDisableSelected: []
  batchDeleteSelected: []
  changePageSize: [size: number]
  goToPage: [page: number]
  toggleSelectAllCurrentPage: []
  toggleSelection: [plugin: PluginRecord]
  toggleFavorite: [plugin: PluginRecord]
  testPlugin: [plugin: PluginRecord]
  configureDefaults: [plugin: PluginRecord]
  advancedTest: [plugin: PluginRecord]
  togglePlugin: [plugin: PluginRecord]
  viewCode: [plugin: PluginRecord]
  deletePlugin: [plugin: PluginRecord]
}>()

const localSearchText = ref(props.pluginSearchText)
const localTag = ref(props.selectedTag)

watch(() => props.pluginSearchText, (val) => { localSearchText.value = val })
watch(() => props.selectedTag, (val) => { localTag.value = val })

const onSearchInput = () => {
  emit('update:searchText', localSearchText.value)
}

const onTagChange = () => {
  emit('update:tag', localTag.value)
}

const visiblePages = computed(() => {
  const pages: (number | string)[] = []
  const total = props.totalPages
  const current = props.currentPage

  if (total <= 7) {
    for (let i = 1; i <= total; i++) pages.push(i)
  } else {
    pages.push(1)
    if (current > 3) pages.push('...')
    for (let i = Math.max(2, current - 1); i <= Math.min(total - 1, current + 1); i++) {
      pages.push(i)
    }
    if (current < total - 2) pages.push('...')
    pages.push(total)
  }
  return pages
})

// Use active tools composable
const { isToolActive, getActiveCount } = useActiveTools()
</script>
