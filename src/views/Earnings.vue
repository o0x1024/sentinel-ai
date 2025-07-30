<template>
  <div class="space-y-6">
    <!-- 页面标题 -->
    <div class="flex items-center justify-between">
      <h1 class="text-3xl font-bold">{{ $t('earnings.title') }}</h1>
      <div class="flex gap-2">
        <select v-model="timeRange" class="select select-bordered">
          <option value="month">{{ $t('earnings.thisMonth') }}</option>
          <option value="quarter">{{ $t('common.thisWeek') }}</option>
          <option value="year">{{ $t('common.thisMonth') }}</option>
          <option value="all">{{ $t('common.all') }}</option>
        </select>
        <button @click="exportReport" class="btn btn-primary">
          <i class="fas fa-download mr-2"></i>{{ $t('earnings.export.exportData') }}
        </button>
      </div>
    </div>

    <!-- 收益概览 -->
    <div class="grid grid-cols-1 lg:grid-cols-3 gap-6">
      <!-- 收益趋势图 -->
      <div class="lg:col-span-2">
        <div class="card bg-base-100 shadow-lg">
          <div class="card-body">
            <h3 class="card-title mb-4">
              <i class="fas fa-chart-line mr-2 text-primary"></i>
              {{ $t('earnings.earningsHistory') }}
            </h3>
            <div class="h-64 bg-base-200 rounded-lg flex items-center justify-center">
              <div class="text-center">
                <i class="fas fa-chart-line text-4xl opacity-50 mb-2"></i>
                <p class="opacity-70">{{ $t('earnings.earningsHistory') }}</p>
                <div class="mt-4 grid grid-cols-2 gap-4 text-sm">
                  <div class="stat">
                    <div class="stat-title text-xs">{{ $t('earnings.thisMonth') }}</div>
                    <div class="stat-value text-lg text-success">+15.3%</div>
                  </div>
                  <div class="stat">
                    <div class="stat-title text-xs">{{ $t('earnings.today') }}</div>
                    <div class="stat-value text-lg text-primary">$106</div>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- 收益统计 -->
      <div>
        <div class="space-y-4">
          <StatsCard
            :value="12450"
            :label="$t('earnings.totalEarnings')"
            :subtitle="$t('common.total')"
            icon="fas fa-dollar-sign"
            theme="success"
            prefix="$"
          />
          
          <StatsCard
            :value="3200"
            :label="$t('earnings.thisMonth')"
            :subtitle="$t('common.thisMonth')"
            icon="fas fa-calendar-month"
            theme="primary"
            prefix="$"
          />
          
          <StatsCard
            :value="1850"
            :label="$t('earnings.stats.averagePerMonth')"
            :subtitle="$t('common.statistics')"
            icon="fas fa-chart-bar"
            theme="secondary"
            prefix="$"
          />
        </div>
      </div>
    </div>

    <!-- 平台收益分布 -->
    <div class="grid grid-cols-1 lg:grid-cols-2 gap-6">
      <div class="card bg-base-100 shadow-lg">
        <div class="card-body">
          <h3 class="card-title mb-4">
            <i class="fas fa-layer-group mr-2 text-primary"></i>
            {{ $t('earnings.earningsByPlatform') }}
          </h3>
          <div class="space-y-3">
            <div v-for="platform in platformEarnings" :key="platform.name" class="flex justify-between items-center">
              <span class="flex items-center gap-2">
                <div :class="platform.color" class="w-4 h-4 rounded"></div>
                {{ platform.name }}
              </span>
              <div class="text-right">
                <span class="font-bold">${{ platform.amount.toLocaleString() }}</span>
                <span class="text-xs opacity-60 ml-1">({{ platform.percentage }}%)</span>
              </div>
            </div>
          </div>
          <div class="mt-4">
            <div class="flex w-full h-3 bg-base-200 rounded-full overflow-hidden">
              <div 
                v-for="platform in platformEarnings" 
                :key="platform.name"
                :class="platform.color"
                :style="{ width: platform.percentage + '%' }"
                class="h-full"
              ></div>
            </div>
          </div>
        </div>
      </div>

      <div class="card bg-base-100 shadow-lg">
        <div class="card-body">
          <h3 class="card-title mb-4">
            <i class="fas fa-bug mr-2 text-primary"></i>
            {{ $t('earnings.earningsByVulnerability') }}
          </h3>
          <div class="space-y-3">
            <div v-for="vulnType in vulnerabilityEarnings" :key="vulnType.type" class="flex justify-between items-center">
              <div class="flex items-center gap-2">
                <i :class="vulnType.icon" class="w-4 text-center"></i>
                <span>{{ vulnType.type }}</span>
              </div>
              <span class="font-bold">${{ vulnType.amount.toLocaleString() }}</span>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- 收益详情 -->
    <div class="grid grid-cols-1 lg:grid-cols-2 gap-6">
      <!-- 最高收益记录 -->
      <div class="card bg-base-100 shadow-lg">
        <div class="card-body">
          <h3 class="card-title mb-4">
            <i class="fas fa-trophy mr-2 text-warning"></i>
            {{ $t('earnings.stats.highestPaying') }}
          </h3>
          <div class="space-y-3">
            <div v-for="record in topEarnings" :key="record.id" class="flex items-center justify-between p-3 bg-base-200 rounded-lg">
              <div class="flex items-center space-x-3">
                <div :class="getSeverityIconClass(record.severity)" class="w-10 h-10 rounded-full flex items-center justify-center">
                  <i class="fas fa-trophy text-sm"></i>
                </div>
                <div>
                  <p class="font-medium text-sm">{{ record.title }}</p>
                  <p class="text-xs opacity-70">{{ record.platform }} | {{ record.date }}</p>
                </div>
              </div>
              <div class="text-right">
                <span class="text-lg font-bold text-success">${{ record.amount.toLocaleString() }}</span>
                <p class="text-xs opacity-60">{{ record.severity }}</p>
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- 收益趋势分析 -->
      <div class="card bg-base-100 shadow-lg">
        <div class="card-body">
          <h3 class="card-title mb-4">
            <i class="fas fa-chart-area mr-2 text-info"></i>
            {{ $t('earnings.summary') }}
          </h3>
          <div class="space-y-4">
            <div class="stat">
              <div class="stat-title">{{ $t('earnings.stats.averagePerVulnerability') }}</div>
              <div class="stat-value text-primary">$541</div>
              <div class="stat-desc">{{ $t('common.thisMonth') }} +12%</div>
            </div>
            
            <div class="stat">
              <div class="stat-title">{{ $t('submissions.filters.accepted') }}</div>
              <div class="stat-value text-success">73%</div>
              <div class="stat-desc">89/122</div>
            </div>
            
            <div class="stat">
              <div class="stat-title">{{ $t('common.time') }}</div>
              <div class="stat-value text-info">2.3{{ $t('common.date') }}</div>
              <div class="stat-desc">-45%</div>
            </div>

            <div class="stat">
              <div class="stat-title">{{ $t('submissions.filters.rejected') }}</div>
              <div class="stat-value text-warning">8%</div>
              <div class="stat-desc">{{ $t('common.thisMonth') }} -3%</div>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- 月度收益表格 -->
    <div class="card bg-base-100 shadow-lg">
      <div class="card-body">
        <h3 class="card-title mb-4">
          <i class="fas fa-table mr-2 text-primary"></i>
          {{ $t('earnings.earningsByMonth') }}
        </h3>
        <div class="overflow-x-auto">
          <table class="table table-zebra">
            <thead>
              <tr>
                <th>{{ $t('common.date') }}</th>
                <th>{{ $t('submissions.allSubmissions') }}</th>
                <th>{{ $t('submissions.acceptedSubmissions') }}</th>
                <th>{{ $t('submissions.filters.accepted') }}</th>
                <th>{{ $t('earnings.amount') }}</th>
                <th>{{ $t('common.statistics') }}</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="month in monthlyData" :key="month.month">
                <td>{{ month.month }}</td>
                <td>{{ month.submissions }}</td>
                <td>{{ month.accepted }}</td>
                <td>
                  <div class="badge badge-success">{{ Math.round(month.accepted / month.submissions * 100) }}%</div>
                </td>
                <td class="font-bold">${{ month.earnings.toLocaleString() }}</td>
                <td>
                  <div :class="month.growth >= 0 ? 'text-success' : 'text-error'">
                    <i :class="month.growth >= 0 ? 'fas fa-arrow-up' : 'fas fa-arrow-down'"></i>
                    {{ Math.abs(month.growth) }}%
                  </div>
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import StatsCard from '../components/Dashboard/StatsCard.vue'
import { useI18n } from 'vue-i18n'

const { t } = useI18n()

// 响应式数据
const timeRange = ref('month')

const platformEarnings = ref([
  { name: 'HackerOne', amount: 7200, percentage: 58, color: 'bg-primary' },
  { name: 'Bugcrowd', amount: 3100, percentage: 25, color: 'bg-secondary' },
  { name: '补天', amount: 1500, percentage: 12, color: 'bg-accent' },
  { name: t('common.other'), amount: 650, percentage: 5, color: 'bg-info' }
])

const vulnerabilityEarnings = ref([
  { type: 'SQL注入', amount: 4200, icon: 'fas fa-database text-error' },
  { type: 'XSS', amount: 2800, icon: 'fas fa-code text-warning' },
  { type: 'IDOR', amount: 1900, icon: 'fas fa-key text-info' },
  { type: 'SSRF', amount: 1600, icon: 'fas fa-globe text-success' },
  { type: t('common.other'), amount: 1950, icon: 'fas fa-bug text-neutral' }
])

const topEarnings = ref([
  {
    id: 1,
    title: 'SQL注入 - 用户认证绕过',
    platform: 'HackerOne',
    amount: 5000,
    severity: 'Critical',
    date: '2024-01-15'
  },
  {
    id: 2,
    title: 'SSRF - 内网访问',
    platform: 'Bugcrowd',
    amount: 3500,
    severity: 'High',
    date: '2024-01-12'
  },
  {
    id: 3,
    title: 'XSS - 管理后台',
    platform: '补天',
    amount: 2800,
    severity: 'High',
    date: '2024-01-08'
  }
])

const monthlyData = ref([
  { month: '2024-01', submissions: 15, accepted: 12, earnings: 3200, growth: 15.3 },
  { month: '2023-12', submissions: 18, accepted: 13, earnings: 2780, growth: -8.2 },
  { month: '2023-11', submissions: 12, accepted: 9, earnings: 3025, growth: 22.1 },
  { month: '2023-10', submissions: 14, accepted: 10, earnings: 2475, growth: 5.8 },
  { month: '2023-09', submissions: 16, accepted: 11, earnings: 2340, growth: -12.3 },
  { month: '2023-08', submissions: 13, accepted: 10, earnings: 2670, growth: 8.7 }
])

// 方法
const exportReport = () => {
  console.log('导出收益报表')
  // 这里可以实现导出功能
}

const getSeverityIconClass = (severity: string) => {
  switch (severity) {
    case 'Critical':
      return 'bg-error/20 text-error'
    case 'High':
      return 'bg-warning/20 text-warning'
    case 'Medium':
      return 'bg-info/20 text-info'
    default:
      return 'bg-base-300 text-base-content'
  }
}
</script>

<style scoped>
.stat {
  padding: 1rem 0;
}

.stat-title {
  font-size: 0.875rem;
  color: rgb(107 114 128);
}

.stat-value {
  font-size: 1.5rem;
  font-weight: 700;
  line-height: 1.25;
}

.stat-desc {
  font-size: 0.75rem;
  color: rgb(107 114 128);
}
</style> 