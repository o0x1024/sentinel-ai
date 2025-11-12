<template>
  <div class="space-y-6">
    <!-- 操作栏 -->
    <div class="flex items-center justify-between">
      <h2 class="text-xl font-semibold">{{ $t('assetManagement.title') }}</h2>
      <div class="flex space-x-2">
        <button @click="refreshAssets" class="btn btn-outline btn-sm">
          <svg class="w-4 h-4 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"></path>
          </svg>
          {{ $t('common.refresh') }}
        </button>
      </div>
    </div>

    <!-- 筛选 -->
    <div class="bg-base-100 rounded-lg p-4 shadow-sm border border-base-300">
      <div class="grid grid-cols-1 md:grid-cols-4 gap-4">
        <div class="form-control">
          <input 
            v-model="searchQuery" 
            type="text" 
            :placeholder="$t('common.search') + '...'"
            class="input input-bordered input-sm"
            @input="applyFilters"
          />
        </div>
        <div class="form-control">
          <select v-model="selectedAssetType" class="select select-bordered select-sm" @change="applyFilters">
            <option value="">{{ $t('assetManagement.allTypes') }}</option>
            <option v-for="type in assetTypes" :key="type" :value="type">
              {{ type }}
            </option>
          </select>
        </div>
        <div class="form-control">
          <select v-model="selectedRiskLevel" class="select select-bordered select-sm" @change="applyFilters">
            <option value="">{{ $t('assetManagement.allRiskLevels') }}</option>
            <option value="High">{{ $t('assetManagement.riskLevel.high') }}</option>
            <option value="Medium">{{ $t('assetManagement.riskLevel.medium') }}</option>
            <option value="Low">{{ $t('assetManagement.riskLevel.low') }}</option>
          </select>
        </div>
        <div class="form-control">
          <select v-model="selectedStatus" class="select select-bordered select-sm" @change="applyFilters">
            <option value="">{{ $t('assetManagement.allStatuses') }}</option>
            <option value="Active">{{ $t('assetManagement.status.active') }}</option>
            <option value="Inactive">{{ $t('assetManagement.status.inactive') }}</option>
          </select>
        </div>
      </div>
    </div>

    <!-- 资产列表 -->
    <div class="bg-base-100 rounded-lg shadow-sm border border-base-300 overflow-hidden">
      <div v-if="loading" class="flex justify-center py-8">
        <span class="loading loading-spinner loading-lg"></span>
      </div>

      <div v-else-if="filteredAssets.length > 0" class="overflow-x-auto">
        <table class="table table-zebra">
          <thead>
            <tr>
              <th>{{ $t('assetManagement.name') }}</th>
              <th>{{ $t('assetManagement.type') }}</th>
              <th>{{ $t('assetManagement.value') }}</th>
              <th>{{ $t('assetManagement.riskLevel.title') }}</th>
              <th>{{ $t('common.status') }}</th>
              <th>{{ $t('assetManagement.lastSeen') }}</th>
              <th>{{ $t('common.actions') }}</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="asset in paginatedAssets" :key="asset.id">
              <td>
                <div class="flex items-center space-x-2">
                  <svg class="w-4 h-4" fill="currentColor" viewBox="0 0 20 20">
                    <path fill-rule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zm3.707-9.293a1 1 0 00-1.414-1.414L9 10.586 7.707 9.293a1 1 0 00-1.414 1.414l2 2a1 1 0 001.414 0l4-4z" clip-rule="evenodd"></path>
                  </svg>
                  <span class="font-medium">{{ asset.name }}</span>
                </div>
              </td>
              <td>
                <span class="badge badge-outline badge-sm">{{ asset.asset_type }}</span>
              </td>
              <td>
                <code class="text-xs bg-base-200 px-2 py-1 rounded">{{ asset.value }}</code>
              </td>
              <td>
                <span class="badge badge-sm" :class="getRiskLevelClass(asset.risk_level)">
                  {{ asset.risk_level }}
                </span>
              </td>
              <td>
                <span class="badge badge-sm" :class="getStatusClass(asset.status)">
                  {{ asset.status }}
                </span>
              </td>
              <td class="text-sm opacity-70">
                {{ asset.last_seen ? formatTime(asset.last_seen) : '-' }}
              </td>
              <td>
                <div class="flex space-x-1">
                  <button @click="viewAssetDetail(asset)" class="btn btn-ghost btn-xs">
                    <svg class="w-3 h-3" fill="currentColor" viewBox="0 0 20 20">
                      <path d="M10 12a2 2 0 100-4 2 2 0 000 4z"></path>
                      <path fill-rule="evenodd" d="M.458 10C1.732 5.943 5.522 3 10 3s8.268 2.943 9.542 7c-1.274 4.057-5.064 7-9.542 7S1.732 14.057.458 10zM14 10a4 4 0 11-8 0 4 4 0 018 0z" clip-rule="evenodd"></path>
                    </svg>
                  </button>
                </div>
              </td>
            </tr>
          </tbody>
        </table>

        <!-- 分页 -->
        <div class="flex justify-center py-4">
          <div class="join">
            <button @click="currentPage--" :disabled="currentPage === 1" class="join-item btn btn-sm">«</button>
            <button class="join-item btn btn-sm">{{ currentPage }} / {{ totalPages }}</button>
            <button @click="currentPage++" :disabled="currentPage === totalPages" class="join-item btn btn-sm">»</button>
          </div>
        </div>
      </div>

      <div v-else class="text-center py-8 text-base-content/50">
        <svg class="w-16 h-16 mx-auto mb-2 opacity-30" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M20 13V6a2 2 0 00-2-2H6a2 2 0 00-2 2v7m16 0v5a2 2 0 01-2 2H6a2 2 0 01-2-2v-5m16 0h-2.586a1 1 0 00-.707.293l-2.414 2.414a1 1 0 01-.707.293h-3.172a1 1 0 01-.707-.293l-2.414-2.414A1 1 0 006.586 13H4"></path>
        </svg>
        <p class="text-sm">{{ $t('assetManagement.noAssets') }}</p>
      </div>
    </div>

    <!-- 资产详情模态框 -->
    <div v-if="showDetailModal && selectedAsset" class="modal modal-open">
      <div class="modal-box">
        <h3 class="font-bold text-lg mb-4">{{ $t('assetManagement.assetDetail') }}</h3>
        <div class="space-y-2">
          <p><strong>{{ $t('assetManagement.name') }}:</strong> {{ selectedAsset.name }}</p>
          <p><strong>{{ $t('assetManagement.type') }}:</strong> {{ selectedAsset.asset_type }}</p>
          <p><strong>{{ $t('assetManagement.value') }}:</strong> {{ selectedAsset.value }}</p>
          <p><strong>{{ $t('assetManagement.riskLevel.title') }}:</strong> {{ selectedAsset.risk_level }}</p>
          <p><strong>{{ $t('common.status') }}:</strong> {{ selectedAsset.status }}</p>
        </div>
        <div class="modal-action">
          <button @click="showDetailModal = false" class="btn btn-sm">{{ $t('common.close') }}</button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue';
import { useI18n } from 'vue-i18n';
import { invoke } from '@tauri-apps/api/core';

const { t } = useI18n();
const emit = defineEmits<{
  'stats-updated': [stats: { total: number; active: number }]
}>();

interface Asset {
  id: string;
  name: string;
  value: string;
  asset_type: string;
  risk_level: string;
  status: string;
  last_seen?: string;
}

const loading = ref(false);
const assets = ref<Asset[]>([]);
const searchQuery = ref('');
const selectedAssetType = ref('');
const selectedRiskLevel = ref('');
const selectedStatus = ref('');
const currentPage = ref(1);
const pageSize = ref(20);
const showDetailModal = ref(false);
const selectedAsset = ref<Asset | null>(null);

const assetTypes = ref<string[]>([]);

const filteredAssets = computed(() => {
  let filtered = assets.value;
  
  if (searchQuery.value) {
    const query = searchQuery.value.toLowerCase();
    filtered = filtered.filter(asset => 
      asset.name.toLowerCase().includes(query) ||
      asset.value.toLowerCase().includes(query)
    );
  }
  
  if (selectedAssetType.value) {
    filtered = filtered.filter(asset => asset.asset_type === selectedAssetType.value);
  }
  
  if (selectedRiskLevel.value) {
    filtered = filtered.filter(asset => asset.risk_level === selectedRiskLevel.value);
  }
  
  if (selectedStatus.value) {
    filtered = filtered.filter(asset => asset.status === selectedStatus.value);
  }
  
  return filtered;
});

const totalPages = computed(() => {
  return Math.ceil(filteredAssets.value.length / pageSize.value);
});

const paginatedAssets = computed(() => {
  const start = (currentPage.value - 1) * pageSize.value;
  const end = start + pageSize.value;
  return filteredAssets.value.slice(start, end);
});

const loadAssets = async () => {
  try {
    loading.value = true;
    const result = await invoke<Asset[]>('list_assets', {
      filter: null,
      limit: null,
      offset: null
    });
    assets.value = result;
    updateStats();
    
    // 提取资产类型
    const types = new Set(assets.value.map(a => a.asset_type));
    assetTypes.value = Array.from(types);
  } catch (error) {
    console.error('Failed to load assets:', error);
    assets.value = [];
  } finally {
    loading.value = false;
  }
};

const updateStats = () => {
  const stats = {
    total: assets.value.length,
    active: assets.value.filter(a => a.status === 'Active').length
  };
  emit('stats-updated', stats);
};

const refreshAssets = () => {
  loadAssets();
};

const applyFilters = () => {
  currentPage.value = 1;
};

const viewAssetDetail = (asset: Asset) => {
  selectedAsset.value = asset;
  showDetailModal.value = true;
};

const getRiskLevelClass = (level: string) => {
  const classes: Record<string, string> = {
    High: 'badge-error',
    Medium: 'badge-warning',
    Low: 'badge-success',
    Unknown: 'badge-ghost'
  };
  return classes[level] || 'badge-ghost';
};

const getStatusClass = (status: string) => {
  const classes: Record<string, string> = {
    Active: 'badge-success',
    Inactive: 'badge-ghost',
    Archived: 'badge-neutral'
  };
  return classes[status] || 'badge-ghost';
};

const formatTime = (dateStr: string) => {
  if (!dateStr) return '-';
  const date = new Date(dateStr);
  const now = new Date();
  const diffInMinutes = Math.floor((now.getTime() - date.getTime()) / (1000 * 60));
  
  if (diffInMinutes < 60) return `${diffInMinutes}${t('common.minutesAgo')}`;
  if (diffInMinutes < 1440) return `${Math.floor(diffInMinutes / 60)}${t('common.hoursAgo')}`;
  return `${Math.floor(diffInMinutes / 1440)}${t('common.daysAgo')}`;
};

const handleRefresh = () => {
  refreshAssets();
};

onMounted(() => {
  loadAssets();
  window.addEventListener('security-center-refresh', handleRefresh);
});

onUnmounted(() => {
  window.removeEventListener('security-center-refresh', handleRefresh);
});
</script>
