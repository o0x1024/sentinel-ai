<template>
  <div class="page-content-padded safe-top space-y-6">
    <!-- 页面标题和操作按钮 -->
    <div class="flex items-center justify-between">
      <h1 class="text-3xl font-bold">{{ $t('assetManagement.title', '资产管理') }}</h1>
      <div class="flex space-x-2">
        <button @click="refreshAssets" class="btn btn-primary btn-sm">
          <i class="fas fa-sync-alt mr-2"></i>
          {{ $t('common.refresh', '刷新') }}
        </button>
        <button @click="showImportModal = true" class="btn btn-secondary btn-sm">
          <i class="fas fa-upload mr-2"></i>
          {{ $t('assetManagement.import', '导入资产') }}
        </button>
      </div>
    </div>

    <!-- 统计卡片 -->
    <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
      <div class="stat bg-base-200 rounded-lg">
        <div class="stat-figure text-primary">
          <i class="fas fa-globe text-2xl"></i>
        </div>
        <div class="stat-title">{{ $t('assetManagement.totalAssets', '总资产数') }}</div>
        <div class="stat-value text-primary">{{ stats.total }}</div>
        <div class="stat-desc">{{ $t('assetManagement.lastUpdated', '最后更新') }}: {{ formatTime(stats.lastUpdated) }}</div>
      </div>
      
      <div class="stat bg-base-200 rounded-lg">
        <div class="stat-figure text-success">
          <i class="fas fa-check-circle text-2xl"></i>
        </div>
        <div class="stat-title">{{ $t('assetManagement.activeAssets', '活跃资产') }}</div>
        <div class="stat-value text-success">{{ stats.active }}</div>
        <div class="stat-desc">{{ ((stats.active / stats.total) * 100).toFixed(1) }}% {{ $t('common.ofTotal', '占总数') }}</div>
      </div>
      
      <div class="stat bg-base-200 rounded-lg">
        <div class="stat-figure text-warning">
          <i class="fas fa-exclamation-triangle text-2xl"></i>
        </div>
        <div class="stat-title">{{ $t('assetManagement.highRiskAssets', '高风险资产') }}</div>
        <div class="stat-value text-warning">{{ stats.highRisk }}</div>
        <div class="stat-desc">{{ $t('assetManagement.needsAttention', '需要关注') }}</div>
      </div>
      
      <div class="stat bg-base-200 rounded-lg">
        <div class="stat-figure text-info">
          <i class="fas fa-clock text-2xl"></i>
        </div>
        <div class="stat-title">{{ $t('assetManagement.recentlyDiscovered', '最近发现') }}</div>
        <div class="stat-value text-info">{{ stats.recent }}</div>
        <div class="stat-desc">{{ $t('assetManagement.last24Hours', '过去24小时') }}</div>
      </div>
    </div>

    <!-- 筛选和搜索 -->
    <div class="card bg-base-100 shadow-md">
      <div class="card-body">
        <div class="grid grid-cols-1 md:grid-cols-4 gap-4">
          <!-- 搜索框 -->
          <div class="form-control">
            <label class="label">
              <span class="label-text">{{ $t('common.search', '搜索') }}</span>
            </label>
            <input 
              v-model="searchQuery" 
              type="text" 
              :placeholder="$t('assetManagement.searchPlaceholder', '搜索资产名称、值或标签...')"
              class="input input-bordered input-sm"
              @input="debouncedSearch"
            />
          </div>
          
          <!-- 资产类型筛选 -->
          <div class="form-control">
            <label class="label">
              <span class="label-text">{{ $t('assetManagement.assetType', '资产类型') }}</span>
            </label>
            <select v-model="selectedAssetType" class="select select-bordered select-sm" @change="applyFilters">
              <option value="">{{ $t('common.all', '全部') }}</option>
              <option v-for="type in assetTypes" :key="type" :value="type">
                {{ $t(`assetTypes.${type}`, type) }}
              </option>
            </select>
          </div>
          
          <!-- 风险等级筛选 -->
          <div class="form-control">
            <label class="label">
              <span class="label-text">{{ $t('assetManagement.riskLevel', '风险等级') }}</span>
            </label>
            <select v-model="selectedRiskLevel" class="select select-bordered select-sm" @change="applyFilters">
              <option value="">{{ $t('common.all', '全部') }}</option>
              <option v-for="level in riskLevels" :key="level" :value="level">
                {{ $t(`riskLevels.${level}`, level) }}
              </option>
            </select>
          </div>
          
          <!-- 状态筛选 -->
          <div class="form-control">
            <label class="label">
              <span class="label-text">{{ $t('common.status', '状态') }}</span>
            </label>
            <select v-model="selectedStatus" class="select select-bordered select-sm" @change="applyFilters">
              <option value="">{{ $t('common.all', '全部') }}</option>
              <option v-for="status in assetStatuses" :key="status" :value="status">
                {{ $t(`assetStatuses.${status}`, status) }}
              </option>
            </select>
          </div>
        </div>
      </div>
    </div>

    <!-- 资产列表 -->
    <div class="card bg-base-100 shadow-md">
      <div class="card-body">
        <div class="flex items-center justify-between mb-4">
          <h2 class="card-title">{{ $t('assetManagement.assetList', '资产列表') }}</h2>
          <div class="flex items-center space-x-2">
            <span class="text-sm text-base-content/70">
              {{ $t('common.showing', '显示') }} {{ filteredAssets.length }} {{ $t('common.of', '条，共') }} {{ assets.length }} {{ $t('common.items', '条') }}
            </span>
          </div>
        </div>
        
        <!-- 加载状态 -->
        <div v-if="loading" class="flex justify-center py-8">
          <span class="loading loading-spinner loading-lg"></span>
        </div>
        
        <!-- 资产表格 -->
        <div v-else class="overflow-x-auto">
          <table class="table table-zebra">
            <thead>
              <tr>
                <th>{{ $t('assetManagement.name', '名称') }}</th>
                <th>{{ $t('assetManagement.type', '类型') }}</th>
                <th>{{ $t('assetManagement.value', '值') }}</th>
                <th>{{ $t('assetManagement.riskLevel', '风险等级') }}</th>
                <th>{{ $t('common.status', '状态') }}</th>
                <th>{{ $t('assetManagement.lastSeen', '最后发现') }}</th>
                <th>{{ $t('common.actions', '操作') }}</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="asset in paginatedAssets" :key="asset.id">
                <td>
                  <div class="flex items-center space-x-2">
                    <i :class="getAssetIcon(asset.asset_type)" class="text-lg"></i>
                    <span class="font-medium">{{ asset.name }}</span>
                  </div>
                </td>
                <td>
                  <span class="badge badge-outline">{{ $t(`assetTypes.${asset.asset_type}`, asset.asset_type) }}</span>
                </td>
                <td>
                  <code class="text-sm bg-base-200 px-2 py-1 rounded">{{ asset.value }}</code>
                </td>
                <td>
                  <span class="badge" :class="getRiskLevelClass(asset.risk_level)">
                    {{ $t(`riskLevels.${asset.risk_level}`, asset.risk_level) }}
                  </span>
                </td>
                <td>
                  <span class="badge" :class="getStatusClass(asset.status)">
                    {{ $t(`assetStatuses.${asset.status}`, asset.status) }}
                  </span>
                </td>
                <td>
                  <span class="text-sm text-base-content/70">
                    {{ asset.last_seen ? formatTime(new Date(asset.last_seen)) : $t('common.never', '从未') }}
                  </span>
                </td>
                <td>
                  <div class="flex space-x-1">
                    <button @click="viewAssetDetail(asset)" class="btn btn-ghost btn-xs" :title="$t('common.view', '查看')">
                      <i class="fas fa-eye"></i>
                    </button>
                    <button @click="editAsset(asset)" class="btn btn-ghost btn-xs" :title="$t('common.edit', '编辑')">
                      <i class="fas fa-edit"></i>
                    </button>
                    <button @click="deleteAsset(asset)" class="btn btn-ghost btn-xs text-error" :title="$t('common.delete', '删除')">
                      <i class="fas fa-trash"></i>
                    </button>
                  </div>
                </td>
              </tr>
            </tbody>
          </table>
          
          <!-- 空状态 -->
          <div v-if="filteredAssets.length === 0" class="text-center py-8">
            <i class="fas fa-search text-4xl text-base-content/30 mb-4"></i>
            <p class="text-base-content/70">{{ $t('assetManagement.noAssetsFound', '未找到匹配的资产') }}</p>
          </div>
        </div>
        
        <!-- 分页 -->
        <div v-if="totalPages > 1" class="flex justify-center mt-4">
          <div class="join">
            <button 
              class="join-item btn btn-sm" 
              :disabled="currentPage === 1"
              @click="currentPage = 1"
            >
              <i class="fas fa-angle-double-left"></i>
            </button>
            <button 
              class="join-item btn btn-sm" 
              :disabled="currentPage === 1"
              @click="currentPage--"
            >
              <i class="fas fa-angle-left"></i>
            </button>
            
            <span class="join-item btn btn-sm btn-active">
              {{ currentPage }} / {{ totalPages }}
            </span>
            
            <button 
              class="join-item btn btn-sm" 
              :disabled="currentPage === totalPages"
              @click="currentPage++"
            >
              <i class="fas fa-angle-right"></i>
            </button>
            <button 
              class="join-item btn btn-sm" 
              :disabled="currentPage === totalPages"
              @click="currentPage = totalPages"
            >
              <i class="fas fa-angle-double-right"></i>
            </button>
          </div>
        </div>
      </div>
    </div>

    <!-- 资产详情模态框 -->
    <div v-if="showDetailModal" class="modal modal-open">
      <div class="modal-box max-w-4xl">
        <h3 class="font-bold text-lg mb-4">{{ $t('assetManagement.assetDetail', '资产详情') }}</h3>
        
        <div v-if="selectedAsset" class="space-y-4">
          <!-- 基本信息 -->
          <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
            <div class="form-control">
              <label class="label">
                <span class="label-text font-medium">{{ $t('assetManagement.name', '名称') }}</span>
              </label>
              <div class="text-lg">{{ selectedAsset.name }}</div>
            </div>
            
            <div class="form-control">
              <label class="label">
                <span class="label-text font-medium">{{ $t('assetManagement.type', '类型') }}</span>
              </label>
              <span class="badge badge-outline badge-lg">{{ $t(`assetTypes.${selectedAsset.asset_type}`, selectedAsset.asset_type) }}</span>
            </div>
            
            <div class="form-control">
              <label class="label">
                <span class="label-text font-medium">{{ $t('assetManagement.value', '值') }}</span>
              </label>
              <code class="bg-base-200 px-3 py-2 rounded text-sm">{{ selectedAsset.value }}</code>
            </div>
            
            <div class="form-control">
              <label class="label">
                <span class="label-text font-medium">{{ $t('assetManagement.riskLevel', '风险等级') }}</span>
              </label>
              <span class="badge badge-lg" :class="getRiskLevelClass(selectedAsset.risk_level)">
                {{ $t(`riskLevels.${selectedAsset.risk_level}`, selectedAsset.risk_level) }}
              </span>
            </div>
          </div>
          
          <!-- 描述 -->
          <div v-if="selectedAsset.description" class="form-control">
            <label class="label">
              <span class="label-text font-medium">{{ $t('common.description', '描述') }}</span>
            </label>
            <div class="bg-base-200 p-3 rounded">{{ selectedAsset.description }}</div>
          </div>
          
          <!-- 标签 -->
          <div v-if="selectedAsset.tags && selectedAsset.tags.length > 0" class="form-control">
            <label class="label">
              <span class="label-text font-medium">{{ $t('assetManagement.tags', '标签') }}</span>
            </label>
            <div class="flex flex-wrap gap-2">
              <span v-for="tag in selectedAsset.tags" :key="tag" class="badge badge-secondary">
                {{ tag }}
              </span>
            </div>
          </div>
          
          <!-- 元数据 -->
          <div v-if="selectedAsset.metadata" class="form-control">
            <label class="label">
              <span class="label-text font-medium">{{ $t('assetManagement.metadata', '元数据') }}</span>
            </label>
            <div class="bg-base-200 p-3 rounded">
              <pre class="text-sm">{{ JSON.stringify(selectedAsset.metadata, null, 2) }}</pre>
            </div>
          </div>
        </div>
        
        <div class="modal-action">
          <button @click="showDetailModal = false" class="btn">
            {{ $t('common.close', '关闭') }}
          </button>
        </div>
      </div>
    </div>

    <!-- 导入资产模态框 -->
    <div v-if="showImportModal" class="modal modal-open">
      <div class="modal-box">
        <h3 class="font-bold text-lg mb-4">{{ $t('assetManagement.importAssets', '导入资产') }}</h3>
        
        <div class="space-y-4">
          <div class="form-control">
            <label class="label">
              <span class="label-text">{{ $t('assetManagement.importFormat', '导入格式') }}</span>
            </label>
            <select v-model="importFormat" class="select select-bordered">
              <option value="json">JSON</option>
              <option value="csv">CSV</option>
            </select>
          </div>
          
          <div class="form-control">
            <label class="label">
              <span class="label-text">{{ $t('assetManagement.selectFile', '选择文件') }}</span>
            </label>
            <input 
              type="file" 
              class="file-input file-input-bordered" 
              :accept="importFormat === 'json' ? '.json' : '.csv'"
              @change="handleFileSelect"
            />
          </div>
          
          <div v-if="importPreview.length > 0" class="form-control">
            <label class="label">
              <span class="label-text">{{ $t('assetManagement.preview', '预览') }} ({{ importPreview.length }} {{ $t('common.items', '条') }})</span>
            </label>
            <div class="bg-base-200 p-3 rounded max-h-40 overflow-y-auto">
              <pre class="text-sm">{{ JSON.stringify(importPreview.slice(0, 3), null, 2) }}</pre>
              <div v-if="importPreview.length > 3" class="text-center text-sm text-base-content/70 mt-2">
                ... {{ $t('assetManagement.andMore', '还有') }} {{ importPreview.length - 3 }} {{ $t('common.items', '条') }}
              </div>
            </div>
          </div>
        </div>
        
        <div class="modal-action">
          <button @click="showImportModal = false" class="btn">
            {{ $t('common.cancel', '取消') }}
          </button>
          <button 
            @click="importAssets" 
            class="btn btn-primary" 
            :disabled="importPreview.length === 0 || importing"
          >
            <span v-if="importing" class="loading loading-spinner loading-sm mr-2"></span>
            {{ importing ? $t('common.importing', '导入中...') : $t('common.import', '导入') }}
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { invoke } from '@tauri-apps/api/core'
import { useToast } from '../composables/useToast'

// 类型定义
interface Asset {
  id: string
  name: string
  value: string
  asset_type: string
  risk_level: string
  status: string
  description?: string
  tags?: string[]
  metadata?: any
  last_seen?: string
  created_at?: string
}

interface AssetFilter {
  asset_types?: string[] | null
  risk_levels?: string[] | null
  statuses?: string[] | null
  search?: string | null
}

// 初始化
const { t } = useI18n()
const toast = useToast()

// 响应式数据
const loading = ref(false)
const assets = ref<Asset[]>([])
const searchQuery = ref('')
const selectedAssetType = ref('')
const selectedRiskLevel = ref('')
const selectedStatus = ref('')
const currentPage = ref(1)
const pageSize = ref(20)

// 模态框状态
const showDetailModal = ref(false)
const showImportModal = ref(false)
const selectedAsset = ref<Asset | null>(null)

// 导入相关
const importFormat = ref('json')
const importPreview = ref<any[]>([])
const importing = ref(false)

// 选项数据
const assetTypes = ref<string[]>([])
const riskLevels = ref<string[]>([])
const assetStatuses = ref<string[]>([])

// 统计数据
const stats = ref({
  total: 0,
  active: 0,
  highRisk: 0,
  recent: 0,
  lastUpdated: new Date()
})

// 计算属性
const filteredAssets = computed(() => {
  let filtered = assets.value
  
  // 搜索过滤
  if (searchQuery.value) {
    const query = searchQuery.value.toLowerCase()
    filtered = filtered.filter(asset => 
      asset.name.toLowerCase().includes(query) ||
      asset.value.toLowerCase().includes(query) ||
      (asset.tags && asset.tags.some(tag => tag.toLowerCase().includes(query)))
    )
  }
  
  // 类型过滤
  if (selectedAssetType.value) {
    filtered = filtered.filter(asset => asset.asset_type === selectedAssetType.value)
  }
  
  // 风险等级过滤
  if (selectedRiskLevel.value) {
    filtered = filtered.filter(asset => asset.risk_level === selectedRiskLevel.value)
  }
  
  // 状态过滤
  if (selectedStatus.value) {
    filtered = filtered.filter(asset => asset.status === selectedStatus.value)
  }
  
  return filtered
})

const totalPages = computed(() => {
  return Math.ceil(filteredAssets.value.length / pageSize.value)
})

const paginatedAssets = computed(() => {
  const start = (currentPage.value - 1) * pageSize.value
  const end = start + pageSize.value
  return filteredAssets.value.slice(start, end)
})

// 防抖搜索
let searchTimeout: NodeJS.Timeout | null = null
const debouncedSearch = () => {
  if (searchTimeout) clearTimeout(searchTimeout)
  searchTimeout = setTimeout(() => {
    currentPage.value = 1
  }, 300)
}

// 方法
const loadAssets = async () => {
  try {
    loading.value = true
    const filter: AssetFilter = {
      asset_types: selectedAssetType.value ? [selectedAssetType.value] : null,
      risk_levels: selectedRiskLevel.value ? [selectedRiskLevel.value] : null,
      statuses: selectedStatus.value ? [selectedStatus.value] : null,
      search: searchQuery.value || null
    }
    const result = await invoke<Asset[]>('list_assets', {
      filter,
      limit: null,
      offset: null
    })
    assets.value = result
    updateStats()
  } catch (error) {
    console.error('Failed to load assets:', error)
    toast.error(t('assetManagement.loadFailed', '加载资产失败'))
  } finally {
    loading.value = false
  }
}

const loadOptions = async () => {
  try {
    const [types, risks, statuses] = await Promise.all([
      invoke<string[]>('get_asset_types'),
      invoke<string[]>('get_risk_levels'),
      invoke<string[]>('get_asset_statuses')
    ])
    assetTypes.value = types
    riskLevels.value = risks
    assetStatuses.value = statuses
  } catch (error) {
    console.error('Failed to load options:', error)
  }
}

const updateStats = () => {
  stats.value.total = assets.value.length
  stats.value.active = assets.value.filter(asset => asset.status === 'Active').length
  stats.value.highRisk = assets.value.filter(asset => asset.risk_level === 'High').length
  
  const yesterday = new Date()
  yesterday.setDate(yesterday.getDate() - 1)
  stats.value.recent = assets.value.filter(asset => 
    asset.created_at && new Date(asset.created_at) > yesterday
  ).length
  
  stats.value.lastUpdated = new Date()
}

const refreshAssets = () => {
  loadAssets()
}

const applyFilters = () => {
  currentPage.value = 1
  loadAssets()
}

const viewAssetDetail = (asset: Asset) => {
  selectedAsset.value = asset
  showDetailModal.value = true
}

const editAsset = (asset: Asset) => {
  // TODO: 实现编辑功能
  toast.info(t('common.featureComingSoon', '功能即将推出'))
}

const deleteAsset = async (asset: Asset) => {
  if (confirm(t('assetManagement.confirmDelete', '确定要删除这个资产吗？'))) {
    try {
      await invoke('delete_asset', { assetId: asset.id })
      toast.success(t('assetManagement.deleteSuccess', '资产删除成功'))
      loadAssets()
    } catch (error) {
      console.error('Failed to delete asset:', error)
      toast.error(t('assetManagement.deleteFailed', '删除资产失败'))
    }
  }
}

const handleFileSelect = (event: Event) => {
  const target = event.target as HTMLInputElement
  const file = target.files?.[0]
  if (!file) return
  
  const reader = new FileReader()
  reader.onload = (e) => {
    try {
      let data
      if (importFormat.value === 'json') {
        data = JSON.parse(e.target?.result as string)
      } else {
        // TODO: 实现CSV解析
        toast.warning(t('assetManagement.csvNotSupported', 'CSV格式暂不支持'))
        return
      }
      
      if (Array.isArray(data)) {
        importPreview.value = data
      } else {
        importPreview.value = [data]
      }
    } catch (error) {
      console.error('Failed to parse file:', error)
      toast.error(t('assetManagement.parseFileFailed', '解析文件失败'))
    }
  }
  reader.readAsText(file)
}

const importAssets = async () => {
  try {
    importing.value = true
    await invoke('import_assets', { assets: importPreview.value })
    toast.success(t('assetManagement.importSuccess', '资产导入成功'))
    showImportModal.value = false
    importPreview.value = []
    loadAssets()
  } catch (error) {
    console.error('Failed to import assets:', error)
    toast.error(t('assetManagement.importFailed', '导入资产失败'))
  } finally {
    importing.value = false
  }
}

// 工具函数
const getAssetIcon = (type: string) => {
  const icons: Record<string, string> = {
    domain: 'fas fa-globe',
    subdomain: 'fas fa-sitemap',
    ip: 'fas fa-server',
    port: 'fas fa-door-open',
    service: 'fas fa-cogs',
    website: 'fas fa-window-maximize',
    api: 'fas fa-code',
    certificate: 'fas fa-certificate',
    fingerprint: 'fas fa-fingerprint',
    vulnerability: 'fas fa-bug',
    technology: 'fas fa-microchip'
  }
  return icons[type] || 'fas fa-question-circle'
}

const getRiskLevelClass = (level: string) => {
  const classes: Record<string, string> = {
    High: 'badge-error',
    Medium: 'badge-warning',
    Low: 'badge-success',
    Unknown: 'badge-ghost'
  }
  return classes[level] || 'badge-ghost'
}

const getStatusClass = (status: string) => {
  const classes: Record<string, string> = {
    Active: 'badge-success',
    Inactive: 'badge-ghost',
    Archived: 'badge-neutral'
  }
  return classes[status] || 'badge-ghost'
}

const formatTime = (date: Date) => {
  const now = new Date()
  const diffInMinutes = Math.floor((now.getTime() - date.getTime()) / (1000 * 60))
  
  if (diffInMinutes < 60) {
    return `${diffInMinutes}${t('common.minutesAgo', '分钟前')}`
  } else if (diffInMinutes < 1440) {
    return `${Math.floor(diffInMinutes / 60)}${t('common.hoursAgo', '小时前')}`
  } else {
    return `${Math.floor(diffInMinutes / 1440)}${t('common.daysAgo', '天前')}`
  }
}

// 生命周期
onMounted(() => {
  loadOptions()
  loadAssets()
})
</script>

<style scoped>
/* 自定义样式 */
.table th {
  @apply bg-base-200 font-semibold;
}

.stat {
  @apply border border-base-300;
}

code {
  @apply font-mono;
}

pre {
  @apply font-mono text-xs;
}
</style>