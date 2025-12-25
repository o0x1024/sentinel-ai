<template>
  <div class="page-content-padded space-y-6 pb-12">
    <!-- 页面标题 -->
    <div class="flex items-center justify-between">
      <div>
        <h1 class="text-3xl font-bold">{{ $t('dashboard.title') }}</h1>
        <p class="text-base-content/60 mt-1">{{ $t('dashboard.welcome') }}</p>
      </div>
      <div class="flex space-x-2">
        <button @click="refreshData" :class="{ 'loading': isLoading }" class="btn btn-primary btn-sm">
          <i v-if="!isLoading" class="fas fa-sync mr-2"></i>
          {{ $t('common.refresh') }}
        </button>
      </div>
    </div>

    <!-- 重要指标卡片 -->
    <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6">
      <StatsCard
        :value="stats.discoveredAssets"
        :label="$t('assetManagement.totalAssets')"
        :subtitle="$t('dashboard.projectsMonitored')"
        icon="fas fa-server"
        theme="primary"
      />
      
      <StatsCard
        :value="stats.vulnerabilities"
        :label="$t('dashboard.vulnerabilitiesFound')"
        :subtitle="$t('dashboard.recentVulnerabilities')"
        icon="fas fa-bug"
        theme="error"
      />
      
      <StatsCard
        :value="trafficStats.http_count"
        :label="$t('dashboard.trafficStats')"
        :subtitle="$t('dashboard.httpRequests')"
        icon="fas fa-exchange-alt"
        theme="info"
      />

      <StatsCard
        :value="totalTokensFormatted"
        :label="$t('dashboard.aiUsage')"
        :subtitle="$t('dashboard.totalTokens')"
        icon="fas fa-robot"
        theme="success"
      />
    </div>

    <!-- 核心分布图表 -->
    <div class="grid grid-cols-1 lg:grid-cols-2 gap-6">
      <!-- 漏洞严重程度分布 -->
      <div class="card bg-base-100 shadow-lg">
        <div class="card-body">
          <h3 class="card-title text-lg mb-4">
            <i class="fas fa-shield-virus mr-2 text-error"></i>
            {{ $t('dashboard.vulnerabilitySeverity') }}
          </h3>
          <div class="flex flex-col md:flex-row items-center justify-around h-64">
            <div class="w-48 h-48">
              <Doughnut v-if="vulnerabilitySeverityData.datasets[0].data.length" :data="vulnerabilitySeverityData" :options="chartOptions" />
              <div v-else class="flex items-center justify-center h-full opacity-50">
                {{ $t('common.noData') }}
              </div>
            </div>
            <div class="mt-4 md:mt-0 space-y-2">
              <div v-for="(count, idx) in vulnerabilitySeverityData.datasets[0].data" :key="idx" class="flex items-center space-x-3">
                <div :style="{ backgroundColor: vulnerabilitySeverityColors[idx] }" class="w-3 h-3 rounded-full"></div>
                <span class="text-sm font-medium w-20">{{ vulnerabilitySeverityLabels[idx] }}</span>
                <span class="text-sm opacity-70">{{ count }}</span>
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- 资产类型分布 -->
      <div class="card bg-base-100 shadow-lg">
        <div class="card-body">
          <h3 class="card-title text-lg mb-4">
            <i class="fas fa-sitemap mr-2 text-primary"></i>
            {{ $t('dashboard.vulnerabilityType') }}
          </h3>
          <div class="h-64">
            <Bar v-if="assetTypeData.datasets[0].data.length" :data="assetTypeData" :options="barChartOptions" />
            <div v-else class="flex items-center justify-center h-full opacity-50">
              {{ $t('common.noData') }}
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- 数据库与 AI 详情 -->
    <div class="grid grid-cols-1 lg:grid-cols-3 gap-6">
      <!-- 数据库概览 -->
      <div class="card bg-base-100 shadow-lg lg:col-span-1">
        <div class="card-body">
          <h3 class="card-title text-lg mb-4">
            <i class="fas fa-database mr-2 text-warning"></i>
            {{ $t('dashboard.databaseStats') }}
          </h3>
          <div class="space-y-4">
            <div class="flex justify-between items-center">
              <span class="opacity-70">{{ $t('dashboard.dbSize') }}</span>
              <span class="font-bold">{{ dbStats.db_size_formatted || '0 B' }}</span>
            </div>
            <div class="flex justify-between items-center">
              <span class="opacity-70">{{ $t('dashboard.tasks') }}</span>
              <span class="badge badge-outline">{{ dbStats.scan_tasks_count || 0 }}</span>
            </div>
            <div class="flex justify-between items-center">
              <span class="opacity-70">{{ $t('dashboard.conversations') }}</span>
              <span class="badge badge-outline">{{ dbStats.conversations_count || 0 }}</span>
            </div>
            <div class="divider my-0"></div>
            <div class="text-xs opacity-50 flex items-center">
              <i class="fas fa-history mr-1"></i>
              {{ $t('dashboard.lastBackup') || 'Last Backup' }}: {{ dbStats.last_backup || '-' }}
            </div>
          </div>
        </div>
      </div>

      <!-- AI Provider Token 使用情况 -->
      <div class="card bg-base-100 shadow-lg lg:col-span-2">
        <div class="card-body">
          <h3 class="card-title text-lg mb-4">
            <i class="fas fa-brain mr-2 text-success"></i>
            {{ $t('dashboard.aiUsage') }}
          </h3>
          <div class="overflow-x-auto">
            <table class="table table-sm w-full">
              <thead>
                <tr>
                  <th>Provider</th>
                  <th>Input Tokens</th>
                  <th>Output Tokens</th>
                  <th>Total</th>
                  <th>Cost</th>
                </tr>
              </thead>
              <tbody>
                <tr v-for="(data, provider) in aiUsageStats" :key="provider">
                  <td class="font-bold">{{ provider }}</td>
                  <td>{{ formatNumber(data.input_tokens) }}</td>
                  <td>{{ formatNumber(data.output_tokens) }}</td>
                  <td>{{ formatNumber(data.total_tokens) }}</td>
                  <td class="text-success">${{ data.cost.toFixed(4) }}</td>
                </tr>
                <tr v-if="Object.keys(aiUsageStats).length === 0">
                  <td colspan="5" class="text-center py-4 opacity-50">{{ $t('common.noData') }}</td>
                </tr>
              </tbody>
            </table>
          </div>
        </div>
      </div>
    </div>

    <!-- 最近活动 -->
    <div class="grid grid-cols-1 lg:grid-cols-2 gap-6">
      <!-- 最近扫描任务 -->
      <div class="card bg-base-100 shadow-lg">
        <div class="card-body">
          <div class="flex items-center justify-between mb-4">
            <h3 class="card-title text-lg">
              <i class="fas fa-history mr-2 text-primary"></i>
              {{ $t('dashboard.recentActivity') }}
            </h3>
            <router-link to="/scan-tasks" class="btn btn-ghost btn-xs">
              {{ $t('common.viewAll') || 'View All' }}
            </router-link>
          </div>
          <div class="space-y-3">
            <div v-for="task in recentTasks" :key="task.id" class="flex items-center justify-between p-3 bg-base-200/50 rounded-lg hover:bg-base-200 transition-colors">
              <div class="flex items-center space-x-3">
                <div :class="getTaskIconClass(task.status)" class="w-8 h-8 rounded-full flex items-center justify-center">
                  <i :class="getTaskIcon(task.status)" class="text-sm"></i>
                </div>
                <div>
                  <p class="font-medium text-sm">{{ task.name }}</p>
                  <p class="text-xs opacity-70 truncate max-w-[200px]">{{ task.target }}</p>
                </div>
              </div>
              <div class="text-right">
                <div :class="getStatusBadgeClass(task.status)" class="badge badge-xs">
                  {{ task.status }}
                </div>
                <p class="text-xs opacity-60 mt-1">{{ formatTime(task.createdAt) }}</p>
              </div>
            </div>
            <div v-if="recentTasks.length === 0" class="text-center py-8 opacity-50">
              {{ $t('common.noData') }}
            </div>
          </div>
        </div>
      </div>

      <!-- 最新发现 -->
      <div class="card bg-base-100 shadow-lg">
        <div class="card-body">
          <div class="flex items-center justify-between mb-4">
            <h3 class="card-title text-lg">
              <i class="fas fa-search mr-2 text-error"></i>
              {{ $t('dashboard.recentVulnerabilities') }}
            </h3>
            <router-link to="/vulnerabilities" class="btn btn-ghost btn-xs">
              {{ $t('common.viewAll') || 'View All' }}
            </router-link>
          </div>
          <div class="space-y-3">
            <div v-for="vuln in recentVulns.slice(0, 5)" :key="vuln.id" class="flex items-center justify-between p-3 bg-base-200/50 rounded-lg hover:bg-base-200 transition-colors">
              <div class="flex items-center space-x-3">
                <div :class="getSeverityIconClass(vuln.severity)" class="w-8 h-8 rounded-full flex items-center justify-center">
                  <i class="fas fa-bug text-sm"></i>
                </div>
                <div>
                  <p class="font-medium text-sm">{{ vuln.title }}</p>
                  <p class="text-xs opacity-70 truncate max-w-[200px]">{{ vuln.target }}</p>
                </div>
              </div>
              <div class="text-right">
                <div :class="getSeverityBadgeClass(vuln.severity)" class="badge badge-xs">
                  {{ vuln.severity }}
                </div>
                <p class="text-xs opacity-60 mt-1">{{ formatTime(vuln.discoveredAt) }}</p>
              </div>
            </div>
            <div v-if="recentVulns.length === 0" class="text-center py-8 opacity-50">
              {{ $t('common.noData') }}
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { invoke } from '@tauri-apps/api/core'
import StatsCard from '../components/Dashboard/StatsCard.vue'
import { Doughnut, Bar } from 'vue-chartjs'
import { 
  Chart as ChartJS, 
  Title, 
  Tooltip, 
  Legend, 
  ArcElement, 
  CategoryScale, 
  LinearScale, 
  BarElement 
} from 'chart.js'

// 注册 Chart.js 组件
ChartJS.register(Title, Tooltip, Legend, ArcElement, CategoryScale, LinearScale, BarElement)

const { t } = useI18n()

// 响应式数据
const isLoading = ref(false)
const stats = ref({
  discoveredAssets: 0,
  vulnerabilities: 0,
  criticalVulns: 0,
  activeScans: 0
})

const trafficStats = ref({
  http_count: 0,
  ws_connection_count: 0,
  ws_message_count: 0
})

const aiUsageStats = ref<Record<string, any>>({})
const dbStats = ref<any>({})

const assetsByType = ref<Record<string, number>>({})
const findingsBySeverity = ref({
  critical: 0,
  high: 0,
  medium: 0,
  low: 0,
  info: 0
})

const recentTasks = ref<any[]>([])
const recentVulns = ref<any[]>([])

// 图表配置
const vulnerabilitySeverityColors = ['#f87171', '#fb923c', '#fbbf24', '#60a5fa', '#9ca3af']
const vulnerabilitySeverityLabels = computed(() => [
  t('riskLevels.critical'),
  t('riskLevels.high'),
  t('riskLevels.medium'),
  t('riskLevels.low'),
  t('riskLevels.info') || 'Info'
])

const vulnerabilitySeverityData = computed(() => ({
  labels: vulnerabilitySeverityLabels.value,
  datasets: [
    {
      backgroundColor: vulnerabilitySeverityColors,
      data: [
        findingsBySeverity.value.critical,
        findingsBySeverity.value.high,
        findingsBySeverity.value.medium,
        findingsBySeverity.value.low,
        findingsBySeverity.value.info
      ]
    }
  ]
}))

const assetTypeData = computed(() => {
  const entries = Object.entries(assetsByType.value).filter(([_, count]) => count > 0)
  return {
    labels: entries.map(([type]) => t(`assetTypes.${type}`) || type),
    datasets: [
      {
        label: t('assetManagement.totalAssets'),
        backgroundColor: '#6366f1',
        data: entries.map(([_, count]) => count)
      }
    ]
  }
})

const chartOptions = {
  responsive: true,
  maintainAspectRatio: false,
  plugins: {
    legend: {
      display: false
    }
  }
}

const barChartOptions = {
  responsive: true,
  maintainAspectRatio: false,
  plugins: {
    legend: {
      display: false
    }
  },
  scales: {
    y: {
      beginAtZero: true,
      grid: {
        display: false
      }
    },
    x: {
      grid: {
        display: false
      }
    }
  }
}

// 计算属性
const totalTokensFormatted = computed(() => {
  let total = 0
  Object.values(aiUsageStats.value).forEach((s: any) => {
    total += s.total_tokens
  })
  if (total >= 1000000) return (total / 1000000).toFixed(1) + 'M'
  if (total >= 1000) return (total / 1000).toFixed(1) + 'k'
  return total.toString()
})

// 方法
const refreshData = async () => {
  isLoading.value = true
  try {
    await Promise.all([
      fetchAssetStats(),
      fetchVulnerabilityStats(),
      fetchTrafficStats(),
      fetchAiStats(),
      fetchDbStats(),
      fetchRecentData()
    ])
  } catch (error) {
    console.error('Refresh dashboard data failed:', error)
  } finally {
    isLoading.value = false
  }
}

const fetchAssetStats = async () => {
  try {
    const assetStats = await invoke<any>('get_asset_stats')
    stats.value.discoveredAssets = assetStats.total_assets || 0
    assetsByType.value = assetStats.by_type || {}
  } catch (error) {
    console.error('Failed to fetch asset stats:', error)
  }
}

const fetchVulnerabilityStats = async () => {
  try {
    // 获取各等级漏洞数量
    const severities = ['critical', 'high', 'medium', 'low', 'info']
    const counts = await Promise.all(severities.map(async (sev) => {
      const resp = await invoke<any>('count_findings', { severityFilter: sev })
      return resp.success ? resp.data : 0
    }))
    
    findingsBySeverity.value = {
      critical: counts[0],
      high: counts[1],
      medium: counts[2],
      low: counts[3],
      info: counts[4]
    }
    
    stats.value.vulnerabilities = counts.reduce((a, b) => a + b, 0)
    stats.value.criticalVulns = counts[0]
  } catch (error) {
    console.error('Failed to fetch vulnerability stats:', error)
  }
}

const fetchTrafficStats = async () => {
  try {
    const resp = await invoke<any>('get_history_stats')
    if (resp.success) {
      trafficStats.value = resp.data
    }
  } catch (error) {
    console.error('Failed to fetch traffic stats:', error)
  }
}

const fetchAiStats = async () => {
  try {
    const statsMap = await invoke<Record<string, any>>('get_ai_usage_stats')
    aiUsageStats.value = statsMap
  } catch (error) {
    console.error('Failed to fetch AI usage stats:', error)
  }
}

const fetchDbStats = async () => {
  try {
    const data = await invoke<any>('get_database_statistics')
    dbStats.value = data
  } catch (error) {
    console.error('Failed to fetch DB stats:', error)
  }
}

const fetchRecentData = async () => {
  try {
    // 最近扫描任务
    const tasks = await invoke<any[]>('get_scan_tasks')
    recentTasks.value = tasks.slice(0, 5).map(t => ({
      id: t.id,
      name: t.name,
      target: t.target,
      status: t.status,
      createdAt: new Date(t.created_at)
    }))

    // 最近漏洞
    const findingsResp = await invoke<any>('list_findings', { limit: 5, offset: 0, severityFilter: null })
    if (findingsResp.success && findingsResp.data) {
      recentVulns.value = findingsResp.data.map((f: any) => ({
        id: f.id,
        title: f.title,
        target: f.url || f.target || '-',
        severity: f.severity,
        discoveredAt: new Date(f.created_at || f.last_seen_at)
      }))
    }
  } catch (error) {
    console.error('Failed to fetch recent data:', error)
  }
}

const formatNumber = (num: number) => {
  if (!num) return '0'
  return num.toLocaleString()
}

const formatTime = (date: any) => {
  if (!date) return '-'
  const d = new Date(date)
  const now = new Date()
  const diffInMinutes = Math.floor((now.getTime() - d.getTime()) / (1000 * 60))
  
  if (diffInMinutes < 1) return t('common.justNow', '刚刚')
  if (diffInMinutes < 60) {
    return `${diffInMinutes} ${t('common.minutesAgo', '分钟前')}`
  } else if (diffInMinutes < 1440) {
    return `${Math.floor(diffInMinutes / 60)} ${t('common.hoursAgo', '小时前')}`
  } else {
    return `${Math.floor(diffInMinutes / 1440)} ${t('common.daysAgo', '天前')}`
  }
}

const getTaskIconClass = (status: string) => {
  switch (status) {
    case 'Running': return 'bg-warning/20 text-warning'
    case 'Completed': return 'bg-success/20 text-success'
    case 'Failed': return 'bg-error/20 text-error'
    case 'Pending': return 'bg-info/20 text-info'
    default: return 'bg-base-300 text-base-content'
  }
}

const getTaskIcon = (status: string) => {
  switch (status) {
    case 'Running': return 'fas fa-spinner fa-spin'
    case 'Completed': return 'fas fa-check'
    case 'Failed': return 'fas fa-times'
    case 'Pending': return 'fas fa-clock'
    default: return 'fas fa-question'
  }
}

const getStatusBadgeClass = (status: string) => {
  switch (status) {
    case 'Running': return 'badge-warning'
    case 'Completed': return 'badge-success'
    case 'Failed': return 'badge-error'
    case 'Pending': return 'badge-info'
    default: return 'badge-ghost'
  }
}

const getSeverityIconClass = (severity: string) => {
  switch (severity?.toLowerCase()) {
    case 'critical':
    case 'high': return 'bg-error/20 text-error'
    case 'medium': return 'bg-warning/20 text-warning'
    case 'low': return 'bg-info/20 text-info'
    default: return 'bg-base-300 text-base-content'
  }
}

const getSeverityBadgeClass = (severity: string) => {
  switch (severity?.toLowerCase()) {
    case 'critical':
    case 'high': return 'badge-error'
    case 'medium': return 'badge-warning'
    case 'low': return 'badge-info'
    default: return 'badge-ghost'
  }
}

onMounted(() => {
  refreshData()
})
</script>

<style scoped>
.card {
  @apply rounded-xl border border-base-200;
  transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
}

.card:hover {
  @apply shadow-xl border-primary/20;
}

.divider {
  @apply opacity-50;
}
</style>
