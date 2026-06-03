<template>
  <div>
    <!-- Stats Cards -->
    <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4 mb-6">
      <div class="stats shadow">
        <div class="stat">
          <div class="stat-figure text-warning">
            <i class="fas fa-clock text-3xl"></i>
          </div>
          <div class="stat-title">{{ $t('plugins.review.pending', '待审核') }}</div>
          <div class="stat-value text-warning">{{ reviewStats.pending }}</div>
        </div>
      </div>
      <div class="stats shadow">
        <div class="stat">
          <div class="stat-figure text-success">
            <i class="fas fa-check-circle text-3xl"></i>
          </div>
          <div class="stat-title">{{ $t('plugins.review.approved', '已批准') }}</div>
          <div class="stat-value text-success">{{ reviewStats.approved }}</div>
        </div>
      </div>
      <div class="stats shadow">
        <div class="stat">
          <div class="stat-figure text-error">
            <i class="fas fa-times-circle text-3xl"></i>
          </div>
          <div class="stat-title">{{ $t('plugins.review.rejected', '已拒绝') }}</div>
          <div class="stat-value text-error">{{ reviewStats.rejected }}</div>
        </div>
      </div>
      <div class="stats shadow">
        <div class="stat">
          <div class="stat-figure text-base-content opacity-50">
            <i class="fas fa-exclamation-triangle text-3xl"></i>
          </div>
          <div class="stat-title">{{ $t('plugins.review.failed', '验证失败') }}</div>
          <div class="stat-value">{{ reviewStats.failed }}</div>
        </div>
      </div>
    </div>

    <!-- Status Filter Buttons -->
    <div class="flex gap-2 mb-4 flex-wrap">
      <button class="btn btn-sm" :class="statusFilter === 'all' ? 'btn-primary' : 'btn-ghost'"
        @click="$emit('changeStatusFilter', 'all')">
        <i class="fas fa-list mr-1"></i>
        {{ $t('plugins.allStatus', '全部') }} ({{ reviewStats.total }})
      </button>
      <button class="btn btn-sm" :class="statusFilter === 'PendingReview' ? 'btn-warning' : 'btn-ghost'"
        @click="$emit('changeStatusFilter', 'PendingReview')">
        <i class="fas fa-clock mr-1"></i>
        {{ $t('plugins.review.pending', '待审核') }} ({{ reviewStats.pending }})
      </button>
      <button class="btn btn-sm" :class="statusFilter === 'Approved' ? 'btn-success' : 'btn-ghost'"
        @click="$emit('changeStatusFilter', 'Approved')">
        <i class="fas fa-check-circle mr-1"></i>
        {{ $t('plugins.review.approved', '已批准') }} ({{ reviewStats.approved }})
      </button>
      <button class="btn btn-sm" :class="statusFilter === 'Rejected' ? 'btn-error' : 'btn-ghost'"
        @click="$emit('changeStatusFilter', 'Rejected')">
        <i class="fas fa-times-circle mr-1"></i>
        {{ $t('plugins.review.rejected', '已拒绝') }} ({{ reviewStats.rejected }})
      </button>
      <button class="btn btn-sm" :class="statusFilter === 'ValidationFailed' ? 'btn-ghost' : 'btn-ghost'"
        @click="$emit('changeStatusFilter', 'ValidationFailed')">
        <i class="fas fa-exclamation-triangle mr-1"></i>
        {{ $t('plugins.review.failed', '验证失败') }} ({{ reviewStats.failed }})
      </button>
    </div>

    <!-- Review Actions -->
    <div class="flex gap-2 mb-4 flex-wrap">
      <input v-model="localSearchText" type="text" :placeholder="$t('plugins.searchPlugins', '搜索插件...')"
        class="input input-bordered flex-1" @input="onSearchInput" />
      <button class="btn btn-success" @click="$emit('approveSelected')" :disabled="selectedPlugins.length === 0">
        <i class="fas fa-check mr-2"></i>
        {{ $t('plugins.batchApprove', '批量批准') }} ({{ selectedPlugins.length }})
      </button>
      <button class="btn btn-error" @click="$emit('rejectSelected')" :disabled="selectedPlugins.length === 0">
        <i class="fas fa-times mr-2"></i>
        {{ $t('plugins.batchReject', '批量拒绝') }} ({{ selectedPlugins.length }})
      </button>
    </div>

    <!-- Review Plugins Table -->
    <div v-if="paginatedPlugins.length === 0" class="alert alert-info">
      <i class="fas fa-info-circle"></i>
      <span>{{ $t('plugins.noReviewPlugins', '暂无待审核的插件') }}</span>
    </div>

    <div v-else>
      <!-- Pagination Info -->
      <div class="flex justify-between items-center mb-4">
        <div class="text-sm text-base-content/70">
          {{ $t('plugins.showing', '显示') }} {{ paginationInfo.start }}-{{ paginationInfo.end }}
          {{ $t('plugins.of', '共') }} {{ paginationInfo.total }} {{ $t('plugins.items', '条') }}
        </div>
        <div class="flex items-center gap-2">
          <span class="text-sm">{{ $t('plugins.pageSize', '每页') }}:</span>
          <select :value="pageSize" @change="$emit('changePageSize', Number(($event.target as HTMLSelectElement).value))"
            class="select select-bordered select-sm">
            <option :value="5">5</option>
            <option :value="10">10</option>
            <option :value="20">20</option>
            <option :value="50">50</option>
          </select>
        </div>
      </div>

      <div class="overflow-x-auto">
        <table class="table table-zebra w-full">
          <thead>
            <tr>
              <th class="w-12">
                <input type="checkbox" class="checkbox checkbox-sm" @change="$emit('toggleSelectAll')"
                  :checked="isAllSelected" />
              </th>
              <th>{{ $t('plugins.pluginName', '插件名称') }}</th>
              <th class="w-32">{{ $t('plugins.qualityScore', '质量评分') }}</th>
              <th class="w-24">{{ $t('common.status', '状态') }}</th>
              <th class="w-24">{{ $t('plugins.model', '模型') }}</th>
              <th class="w-40">{{ $t('plugins.generatedAt', '生成时间') }}</th>
              <th class="w-48">{{ $t('common.actions', '操作') }}</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="plugin in paginatedPlugins" :key="plugin.plugin_id" class="hover">
              <td>
                <input type="checkbox" class="checkbox checkbox-sm" :checked="isPluginSelected(plugin)"
                  @change="$emit('toggleSelection', plugin)" />
              </td>
              <td>
                <div>
                  <div class="flex items-center gap-2">
                    <span class="badge badge-sm" :class="getVulnTypeBadgeClass(plugin.vuln_type)">
                      {{ plugin.vuln_type }}
                    </span>
                    <span class="font-medium">{{ plugin.plugin_name }}</span>
                  </div>
                  <div class="text-sm text-gray-500 mt-1">{{ plugin.description }}</div>
                </div>
              </td>
              <td>
                <div class="flex items-center gap-2">
                  <div class="radial-progress text-sm" :class="getScoreClass(plugin.quality_score)"
                    :style="`--value:${plugin.quality_score}; --size:2.5rem;`">
                    {{ plugin.quality_score }}
                  </div>
                </div>
              </td>
              <td>
                <span class="badge" :class="getStatusBadgeClass(plugin.status)">
                  {{ getReviewStatusText(plugin.status) }}
                </span>
              </td>
              <td>
                <span class="text-sm">{{ plugin.model }}</span>
              </td>
              <td>
                <span class="text-sm">{{ formatDate(plugin.generated_at) }}</span>
              </td>
              <td>
                <div class="flex gap-1">
                  <button class="btn btn-sm btn-info" @click="$emit('viewDetail', plugin)">
                    <i class="fas fa-eye"></i>
                  </button>
                  <button class="btn btn-sm btn-success" @click="$emit('approvePlugin', plugin)"
                    :disabled="plugin.status === 'Approved'">
                    <i class="fas fa-check"></i>
                  </button>
                  <button class="btn btn-sm btn-error" @click="$emit('rejectPlugin', plugin)"
                    :disabled="plugin.status === 'Rejected'">
                    <i class="fas fa-times"></i>
                  </button>
                </div>
              </td>
            </tr>
          </tbody>
        </table>
      </div>

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
import type { ReviewPlugin, ReviewStats } from './types'

const props = defineProps<{
  reviewStats: ReviewStats
  statusFilter: string
  searchText: string
  paginatedPlugins: ReviewPlugin[]
  paginationInfo: { start: number; end: number; total: number }
  currentPage: number
  pageSize: number
  totalPages: number
  selectedPlugins: ReviewPlugin[]
  isAllSelected: boolean
  isPluginSelected: (plugin: ReviewPlugin) => boolean
  getReviewStatusText: (status: string) => string
}>()

const emit = defineEmits<{
  'changeStatusFilter': [status: string]
  'update:searchText': [value: string]
  'approveSelected': []
  'rejectSelected': []
  'toggleSelectAll': []
  'toggleSelection': [plugin: ReviewPlugin]
  'viewDetail': [plugin: ReviewPlugin]
  'approvePlugin': [plugin: ReviewPlugin]
  'rejectPlugin': [plugin: ReviewPlugin]
  'changePageSize': [size: number]
  'goToPage': [page: number]
}>()

const localSearchText = ref(props.searchText)

watch(() => props.searchText, (val) => { localSearchText.value = val })

const onSearchInput = () => {
  emit('update:searchText', localSearchText.value)
}

const getVulnTypeBadgeClass = (vulnType: string): string => {
  const classMap: Record<string, string> = {
    'sqli': 'badge-error',
    'command_injection': 'badge-error',
    'xss': 'badge-warning',
    'idor': 'badge-info',
    'auth_bypass': 'badge-info',
    'csrf': 'badge-primary',
    'info_leak': 'badge-secondary',
    'ssrf': 'badge-accent'
  }
  return classMap[vulnType] || 'badge-ghost'
}

const getScoreClass = (score: number): string => {
  if (score >= 80) return 'text-success'
  if (score >= 60) return 'text-warning'
  return 'text-error'
}

const getStatusBadgeClass = (status: string): string => {
  const classMap: Record<string, string> = {
    'PendingReview': 'badge-warning',
    'Approved': 'badge-success',
    'Rejected': 'badge-error',
    'ValidationFailed': 'badge-ghost'
  }
  return classMap[status] || 'badge-ghost'
}

const formatDate = (dateStr: string): string => {
  try {
    const date = new Date(dateStr)
    return date.toLocaleString()
  } catch {
    return dateStr
  }
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
</script>
