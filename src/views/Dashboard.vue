<template>
  <div class="space-y-6">
    <!-- 页面标题 -->
    <div class="flex items-center justify-between">
      <h1 class="text-3xl font-bold">{{ $t('dashboard.title') }}</h1>
      <div class="flex space-x-2">
        <button @click="refreshData" class="btn btn-primary btn-sm">
          <i class="fas fa-sync mr-2"></i>
          {{ $t('common.refresh') }}
        </button>
      </div>
    </div>

    <!-- 赏金统计卡片 -->
    <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-5 gap-6">

      
      <StatsCard
        :value="stats.discoveredAssets"
        :label="$t('dashboard.vulnerabilitiesFound')"
        :subtitle="$t('dashboard.projectsMonitored')"
        icon="fas fa-server"
        theme="primary"
      />
      

      
      <StatsCard
        :value="stats.vulnerabilities"
        :label="$t('dashboard.vulnerabilitiesFound')"
        :subtitle="`${stats.criticalVulns} ${$t('common.critical')}`"
        icon="fas fa-bug"
        theme="error"
      />
      
      <StatsCard
        :value="stats.criticalVulns"
        :label="$t('vulnerabilities.criticalVulnerabilities')"
        :subtitle="$t('dashboard.recentVulnerabilities')"
        icon="fas fa-exclamation-triangle"
        theme="warning"
      />
    </div>

    <!-- ASM和漏洞展示 -->
    <div class="grid grid-cols-1 lg:grid-cols-2 gap-6">
      <!-- 攻击面管理 -->
      <div class="card bg-base-100 shadow-lg">
        <div class="card-header bg-accent text-accent-content p-4 rounded-t-xl">
          <h3 class="text-lg font-bold">
            <i class="fas fa-sitemap mr-2"></i>{{ $t('dashboard.systemStatus') }}
          </h3>
        </div>
        <div class="card-body p-4">
          <div class="tabs tabs-bordered mb-4">
            <a 
              :class="['tab', activeTab === 'subdomains' ? 'tab-active' : '']"
              @click="activeTab = 'subdomains'"
            >
              {{ $t('dashboard.scanProgress') }}
            </a>
            <a 
              :class="['tab', activeTab === 'ips' ? 'tab-active' : '']"
              @click="activeTab = 'ips'"
            >
              {{ $t('common.target') }}
            </a>
            <a 
              :class="['tab', activeTab === 'services' ? 'tab-active' : '']"
              @click="activeTab = 'services'"
            >
              {{ $t('scanTasks.scanOptions') }}
            </a>
          </div>
          
          <div class="max-h-64 overflow-y-auto">
            <div class="overflow-x-auto">
              <table class="table table-zebra w-full table-xs">
                <thead>
                  <tr v-if="activeTab === 'subdomains'">
                    <th>{{ $t('common.name') }}</th>
                    <th>{{ $t('common.target') }}</th>
                    <th>{{ $t('common.status') }}</th>
                  </tr>
                  <tr v-else-if="activeTab === 'ips'">
                    <th>{{ $t('scanTasks.targetIp') }}</th>
                    <th>{{ $t('common.description') }}</th>
                    <th>{{ $t('common.status') }}</th>
                  </tr>
                  <tr v-else>
                    <th>{{ $t('scanTasks.scanOptions') }}</th>
                    <th>{{ $t('common.type') }}</th>
                    <th>{{ $t('common.status') }}</th>
                  </tr>
                </thead>
                <tbody>
                  <tr v-for="asset in getAssetsByTab(activeTab)" :key="asset.id">
                    <td>{{ asset.primary }}</td>
                    <td>{{ asset.secondary }}</td>
                    <td>
                      <div :class="getAssetStatusClass(asset.status)" class="badge badge-xs">
                        {{ asset.status }}
                      </div>
                    </td>
                  </tr>
                </tbody>
              </table>
            </div>
          </div>
        </div>
      </div>

      <!-- 最新漏洞 -->
      <div class="card bg-base-100 shadow-lg">
        <div class="card-header bg-error text-error-content p-4 rounded-t-xl">
          <h3 class="text-lg font-bold">
            <i class="fas fa-shield-alt mr-2"></i>{{ $t('dashboard.recentVulnerabilities') }}
          </h3>
        </div>
        <div class="card-body p-4">
          <div class="space-y-3 max-h-64 overflow-y-auto">
            <div 
              v-for="vuln in recentVulns" 
              :key="vuln.id" 
              :class="getVulnAlertClass(vuln.severity)"
              class="alert"
            >
              <i :class="getVulnIcon(vuln.severity)"></i>
              <div>
                <h4 class="font-bold">{{ vuln.title }}</h4>
                <div class="text-xs">{{ vuln.target }}</div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- 图表区域 -->
    <div class="grid grid-cols-1 lg:grid-cols-2 gap-6">


      <!-- 漏洞类型分布 -->
      <div class="card bg-base-100 shadow-lg">
        <div class="card-body">
          <h3 class="card-title mb-4">
            <i class="fas fa-chart-pie mr-2 text-primary"></i>
            {{ $t('vulnerabilities.vulnerabilityType') }}
          </h3>
          <div class="space-y-3">
            <div v-for="vulnType in vulnerabilityTypes" :key="vulnType.type" class="flex items-center justify-between">
              <div class="flex items-center space-x-3">
                <div :class="vulnType.color" class="w-3 h-3 rounded-full"></div>
                <span class="text-sm">{{ vulnType.type }}</span>
              </div>
              <div class="text-right">
                <span class="text-sm font-medium">{{ vulnType.count }}</span>
                <span class="text-xs opacity-60 ml-1">({{ vulnType.percentage }}%)</span>
              </div>
            </div>
          </div>
          <div class="mt-4">
            <div class="flex w-full h-2 bg-base-200 rounded-full overflow-hidden">
              <div 
                v-for="vulnType in vulnerabilityTypes" 
                :key="vulnType.type"
                :class="vulnType.color"
                :style="{ width: vulnType.percentage + '%' }"
                class="h-full"
              ></div>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- 最近活动 -->
    <div class="grid grid-cols-1 lg:grid-cols-2 gap-6">
      <!-- 最近扫描任务 -->
      <div class="card bg-base-100 shadow-lg">
        <div class="card-body">
          <h3 class="card-title mb-4">
            <i class="fas fa-history mr-2 text-primary"></i>
            {{ $t('dashboard.recentActivity') }}
          </h3>
          <div class="space-y-3">
            <div v-for="task in recentTasks" :key="task.id" class="flex items-center justify-between p-3 bg-base-200 rounded-lg hover:bg-base-300 transition-colors">
              <div class="flex items-center space-x-3">
                <div :class="getTaskIconClass(task.status)" class="w-8 h-8 rounded-full flex items-center justify-center">
                  <i :class="getTaskIcon(task.status)" class="text-sm"></i>
                </div>
                <div>
                  <p class="font-medium text-sm">{{ task.name }}</p>
                  <p class="text-xs opacity-70">{{ task.target }}</p>
                </div>
              </div>
              <div class="text-right">
                <div :class="getStatusBadgeClass(task.status)" class="badge badge-xs">
                  {{ task.status }}
                </div>
                <p class="text-xs opacity-60 mt-1">{{ formatTime(task.createdAt) }}</p>
              </div>
            </div>
          </div>
          <div class="mt-4">
            <router-link to="/scan-tasks" class="btn btn-sm btn-outline w-full">
              <i class="fas fa-eye mr-2"></i>{{ $t('scanTasks.title') }}
            </router-link>
          </div>
        </div>
      </div>

      <!-- 最新发现 -->
      <div class="card bg-base-100 shadow-lg">
        <div class="card-body">
          <h3 class="card-title mb-4">
            <i class="fas fa-search mr-2 text-primary"></i>
            {{ $t('dashboard.recentVulnerabilities') }}
          </h3>
          <div class="space-y-3">
            <div v-for="vuln in recentVulns.slice(0, 5)" :key="vuln.id" class="flex items-center justify-between p-3 bg-base-200 rounded-lg hover:bg-base-300 transition-colors">
              <div class="flex items-center space-x-3">
                <div :class="getSeverityIconClass(vuln.severity)" class="w-8 h-8 rounded-full flex items-center justify-center">
                  <i class="fas fa-bug text-sm"></i>
                </div>
                <div>
                  <p class="font-medium text-sm">{{ vuln.title }}</p>
                  <p class="text-xs opacity-70">{{ vuln.target }}</p>
                </div>
              </div>
              <div class="text-right">
                <div :class="getSeverityBadgeClass(vuln.severity)" class="badge badge-xs">
                  {{ vuln.severity }}
                </div>
                <p class="text-xs opacity-60 mt-1">{{ formatTime(vuln.discoveredAt) }}</p>
              </div>
            </div>
          </div>
          <div class="mt-4">
            <router-link to="/vulnerabilities" class="btn btn-sm btn-outline w-full">
              <i class="fas fa-bug mr-2"></i>{{ $t('vulnerabilities.title') }}
            </router-link>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import StatsCard from '../components/Dashboard/StatsCard.vue'

const { t } = useI18n()

// 响应式数据
const activeTab = ref('subdomains')

const stats = ref({
  discoveredAssets: 156,
  vulnerabilities: 23,
  criticalVulns: 7
})

const vulnerabilityTypes = ref([
  { type: 'XSS', count: 8, percentage: 35, color: 'bg-error' },
  { type: 'SQL注入', count: 5, percentage: 22, color: 'bg-warning' },
  { type: 'CSRF', count: 3, percentage: 13, color: 'bg-info' },
  { type: '信息泄露', count: 4, percentage: 17, color: 'bg-success' },
  { type: '其他', count: 3, percentage: 13, color: 'bg-neutral' }
])

const assets = ref({
  subdomains: [
    { id: 1, primary: 'www.mgtv.com', secondary: '118.24.63.156', status: t('common.inProgress') },
    { id: 2, primary: 'api.mgtv.com', secondary: '118.24.63.157', status: t('common.inProgress') },
    { id: 3, primary: 'm.mgtv.com', secondary: '118.24.63.158', status: t('common.pending') }
  ],
  ips: [
    { id: 1, primary: '118.24.63.156', secondary: '北京', status: t('common.inProgress') },
    { id: 2, primary: '118.24.63.157', secondary: '上海', status: t('common.inProgress') },
    { id: 3, primary: '118.24.63.158', secondary: '深圳', status: t('common.failed') }
  ],
  services: [
    { id: 1, primary: 'HTTP', secondary: '80', status: 'nginx/1.18.0' },
    { id: 2, primary: 'HTTPS', secondary: '443', status: 'nginx/1.18.0' },
    { id: 3, primary: 'SSH', secondary: '22', status: 'OpenSSH_8.0' }
  ]
})

const recentTasks = ref([
  {
    id: '1',
    name: t('scanTasks.scanTypes.webScan'),
    target: 'example.com',
    status: 'Running',
    createdAt: new Date(Date.now() - 1000 * 60 * 30)
  },
  {
    id: '2',
    name: t('scanTasks.scanTypes.portScan'),
    target: 'test.example.com',
    status: 'Completed',
    createdAt: new Date(Date.now() - 1000 * 60 * 60 * 2)
  },
  {
    id: '3',
    name: t('scanTasks.scanTypes.vulnerabilityScan'),
    target: 'api.example.com',
    status: 'Failed',
    createdAt: new Date(Date.now() - 1000 * 60 * 60 * 4)
  }
])

const recentVulns = ref([
  {
    id: '1',
    title: t('vulnerabilities.types.sqli'),
    target: 'api.mgtv.com/login',
    severity: 'High',
    discoveredAt: new Date(Date.now() - 1000 * 60 * 15)
  },
  {
    id: '2',
    title: t('vulnerabilities.types.xss'),
    target: 'www.mgtv.com/search',
    severity: 'Medium',
    discoveredAt: new Date(Date.now() - 1000 * 60 * 45)
  },
  {
    id: '3',
    title: t('vulnerabilities.types.infoDisclosure'),
    target: 'admin.mgtv.com/files',
    severity: 'Low',
    discoveredAt: new Date(Date.now() - 1000 * 60 * 90)
  }
])

// 方法
const refreshData = () => {
  console.log(t('common.refresh'))
  // 这里可以调用API刷新数据
}

const getAssetsByTab = (tab: string) => {
  return assets.value[tab as keyof typeof assets.value] || []
}

const getAssetStatusClass = (status: string) => {
  switch (status) {
    case t('common.inProgress'):
      return 'badge-success'
    case t('common.pending'):
      return 'badge-warning'
    case t('common.failed'):
      return 'badge-error'
    default:
      return 'badge-ghost'
  }
}

const getVulnAlertClass = (severity: string) => {
  switch (severity) {
    case 'High':
      return 'alert-error'
    case 'Medium':
      return 'alert-warning'
    case 'Low':
      return 'alert-info'
    default:
      return 'alert-info'
  }
}

const getVulnIcon = (severity: string) => {
  switch (severity) {
    case 'High':
      return 'fas fa-exclamation-triangle'
    case 'Medium':
      return 'fas fa-exclamation-circle'
    case 'Low':
      return 'fas fa-info-circle'
    default:
      return 'fas fa-info-circle'
  }
}

const getStatusBadgeClass = (status: string) => {
  switch (status) {
    case 'Running':
      return 'badge-warning'
    case 'Completed':
      return 'badge-success'
    case 'Failed':
      return 'badge-error'
    default:
      return 'badge-ghost'
  }
}

const getSeverityBadgeClass = (severity: string) => {
  switch (severity) {
    case 'High':
      return 'badge-error'
    case 'Medium':
      return 'badge-warning'
    case 'Low':
      return 'badge-info'
    default:
      return 'badge-ghost'
  }
}

const getTaskIconClass = (status: string) => {
  switch (status) {
    case 'Running':
      return 'bg-warning/20 text-warning'
    case 'Completed':
      return 'bg-success/20 text-success'
    case 'Failed':
      return 'bg-error/20 text-error'
    default:
      return 'bg-base-300 text-base-content'
  }
}

const getTaskIcon = (status: string) => {
  switch (status) {
    case 'Running':
      return 'fas fa-spinner fa-spin'
    case 'Completed':
      return 'fas fa-check'
    case 'Failed':
      return 'fas fa-times'
    default:
      return 'fas fa-question'
  }
}

const getSeverityIconClass = (severity: string) => {
  switch (severity) {
    case 'High':
      return 'bg-error/20 text-error'
    case 'Medium':
      return 'bg-warning/20 text-warning'
    case 'Low':
      return 'bg-info/20 text-info'
    default:
      return 'bg-base-300 text-base-content'
  }
}

const formatTime = (date: Date) => {
  const now = new Date()
  const diffInMinutes = Math.floor((now.getTime() - date.getTime()) / (1000 * 60))
  
  if (diffInMinutes < 60) {
    return `${diffInMinutes}${t('common.time')}`
  } else if (diffInMinutes < 1440) {
    return `${Math.floor(diffInMinutes / 60)}${t('common.time')}`
  } else {
    return `${Math.floor(diffInMinutes / 1440)}${t('common.date')}`
  }
}

onMounted(() => {
  // 组件挂载时可以加载数据
  refreshData()
})
</script>

<style scoped>
.card-header {
  border-radius: 0.75rem 0.75rem 0 0;
}

.stat-card {
  @apply card bg-base-100 shadow-lg p-6;
}

.alert {
  padding: 0.75rem;
}
</style>