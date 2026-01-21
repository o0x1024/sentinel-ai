<template>
  <div class="space-y-6">
    <!-- Overview Cards -->
    <div class="grid grid-cols-2 md:grid-cols-4 gap-4">
      <div class="stat bg-base-200 rounded-lg">
        <div class="stat-figure text-success">
          <i class="fas fa-dollar-sign text-3xl"></i>
        </div>
        <div class="stat-title">{{ t('bugBounty.statistics.totalEarnings') }}</div>
        <div class="stat-value text-success">${{ totalEarnings.toFixed(0) }}</div>
        <div class="stat-desc">{{ t('bugBounty.statistics.fromAccepted') }}</div>
      </div>
      
      <div class="stat bg-base-200 rounded-lg">
        <div class="stat-figure text-primary">
          <i class="fas fa-check-circle text-3xl"></i>
        </div>
        <div class="stat-title">{{ t('bugBounty.statistics.acceptRate') }}</div>
        <div class="stat-value text-primary">{{ acceptRate.toFixed(1) }}%</div>
        <div class="stat-desc">{{ acceptedCount }}/{{ totalSubmissions }} {{ t('bugBounty.statistics.submissions') }}</div>
      </div>
      
      <div class="stat bg-base-200 rounded-lg">
        <div class="stat-figure text-error">
          <i class="fas fa-bug text-3xl"></i>
        </div>
        <div class="stat-title">{{ t('bugBounty.statistics.criticalFindings') }}</div>
        <div class="stat-value text-error">{{ criticalFindings }}</div>
        <div class="stat-desc">{{ highFindings }} {{ t('bugBounty.severity.high') }}</div>
      </div>
      
      <div class="stat bg-base-200 rounded-lg">
        <div class="stat-figure text-info">
          <i class="fas fa-clock text-3xl"></i>
        </div>
        <div class="stat-title">{{ t('bugBounty.statistics.avgResponse') }}</div>
        <div class="stat-value text-info">{{ avgResponseTime }}h</div>
        <div class="stat-desc">{{ t('bugBounty.statistics.responseTime') }}</div>
      </div>
    </div>

    <!-- Charts Row -->
    <div class="grid grid-cols-1 lg:grid-cols-2 gap-6">
      <!-- Severity Distribution -->
      <div class="card bg-base-100 shadow-lg border border-base-200">
        <div class="card-body">
          <h3 class="card-title text-base">
            <i class="fas fa-chart-pie text-primary mr-2"></i>
            {{ t('bugBounty.statistics.severityDistribution') }}
          </h3>
          <div class="h-64 flex items-center justify-center">
            <div class="w-56 h-56">
              <Doughnut v-if="hasSeverityData" :data="severityChartData" :options="doughnutOptions" />
              <div v-else class="flex items-center justify-center h-full text-base-content/50">
                <div class="text-center">
                  <i class="fas fa-chart-pie text-4xl mb-2 opacity-30"></i>
                  <p>{{ t('common.noData') }}</p>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- Submission Status -->
      <div class="card bg-base-100 shadow-lg border border-base-200">
        <div class="card-body">
          <h3 class="card-title text-base">
            <i class="fas fa-tasks text-info mr-2"></i>
            {{ t('bugBounty.statistics.statusDistribution') }}
          </h3>
          <div class="h-64 flex items-center justify-center">
            <div class="w-56 h-56">
              <Doughnut v-if="hasStatusData" :data="statusChartData" :options="doughnutOptions" />
              <div v-else class="flex items-center justify-center h-full text-base-content/50">
                <div class="text-center">
                  <i class="fas fa-tasks text-4xl mb-2 opacity-30"></i>
                  <p>{{ t('common.noData') }}</p>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- Vulnerability Types Bar Chart -->
    <div class="card bg-base-100 shadow-lg border border-base-200">
      <div class="card-body">
        <h3 class="card-title text-base">
          <i class="fas fa-chart-bar text-warning mr-2"></i>
          {{ t('bugBounty.statistics.vulnTypeDistribution') }}
        </h3>
        <div class="h-64">
          <Bar v-if="hasVulnTypeData" :data="vulnTypeChartData" :options="barOptions" />
          <div v-else class="flex items-center justify-center h-full text-base-content/50">
            <div class="text-center">
              <i class="fas fa-chart-bar text-4xl mb-2 opacity-30"></i>
              <p>{{ t('common.noData') }}</p>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- Monthly Earnings Trend -->
    <div class="card bg-base-100 shadow-lg border border-base-200">
      <div class="card-body">
        <h3 class="card-title text-base">
          <i class="fas fa-chart-line text-success mr-2"></i>
          {{ t('bugBounty.statistics.earningsTrend') }}
        </h3>
        <div class="h-64">
          <Line v-if="hasEarningsData" :data="earningsChartData" :options="lineOptions" />
          <div v-else class="flex items-center justify-center h-full text-base-content/50">
            <div class="text-center">
              <i class="fas fa-chart-line text-4xl mb-2 opacity-30"></i>
              <p>{{ t('bugBounty.statistics.noEarningsYet') }}</p>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- Program Leaderboard -->
    <div class="card bg-base-100 shadow-lg border border-base-200">
      <div class="card-body">
        <h3 class="card-title text-base">
          <i class="fas fa-trophy text-warning mr-2"></i>
          {{ t('bugBounty.statistics.programLeaderboard') }}
        </h3>
        <div v-if="programStats.length > 0" class="overflow-x-auto">
          <table class="table table-sm">
            <thead>
              <tr>
                <th>#</th>
                <th>{{ t('bugBounty.table.program') }}</th>
                <th>{{ t('bugBounty.statistics.findings') }}</th>
                <th>{{ t('bugBounty.statistics.submissions') }}</th>
                <th>{{ t('bugBounty.statistics.accepted') }}</th>
                <th>{{ t('bugBounty.statistics.earnings') }}</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="(program, index) in programStats" :key="program.id" class="hover">
                <td>
                  <span v-if="index === 0" class="text-warning"><i class="fas fa-medal"></i></span>
                  <span v-else-if="index === 1" class="text-base-content/60"><i class="fas fa-medal"></i></span>
                  <span v-else-if="index === 2" class="text-orange-400"><i class="fas fa-medal"></i></span>
                  <span v-else>{{ index + 1 }}</span>
                </td>
                <td class="font-medium">{{ program.name }}</td>
                <td>{{ program.findings_count }}</td>
                <td>{{ program.submissions_count }}</td>
                <td>
                  <span class="badge badge-success badge-sm">{{ program.accepted_count }}</span>
                </td>
                <td class="text-success font-medium">${{ program.total_earnings.toFixed(0) }}</td>
              </tr>
            </tbody>
          </table>
        </div>
        <div v-else class="text-center py-8 text-base-content/50">
          <i class="fas fa-trophy text-4xl mb-2 opacity-30"></i>
          <p>{{ t('common.noData') }}</p>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { invoke } from '@tauri-apps/api/core'
import { Doughnut, Bar, Line } from 'vue-chartjs'
import {
  Chart as ChartJS,
  Title,
  Tooltip,
  Legend,
  ArcElement,
  CategoryScale,
  LinearScale,
  BarElement,
  PointElement,
  LineElement,
  Filler
} from 'chart.js'

ChartJS.register(Title, Tooltip, Legend, ArcElement, CategoryScale, LinearScale, BarElement, PointElement, LineElement, Filler)

const { t } = useI18n()

const props = defineProps<{
  findingStats: any
  submissionStats: any
  programs: any[]
  findings: any[]
  submissions: any[]
}>()

// Computed stats
const totalEarnings = computed(() => {
  return props.submissions.reduce((sum, s) => sum + (s.reward_amount || 0) + (s.bonus_amount || 0), 0)
})

const acceptedCount = computed(() => {
  return props.submissions.filter(s => ['accepted', 'resolved'].includes(s.status?.toLowerCase())).length
})

const totalSubmissions = computed(() => props.submissions.length)

const acceptRate = computed(() => {
  if (totalSubmissions.value === 0) return 0
  return (acceptedCount.value / totalSubmissions.value) * 100
})

const criticalFindings = computed(() => props.findingStats?.by_severity?.critical || 0)
const highFindings = computed(() => props.findingStats?.by_severity?.high || 0)

const avgResponseTime = computed(() => {
  const withResponse = props.submissions.filter(s => s.response_time_hours)
  if (withResponse.length === 0) return 0
  const total = withResponse.reduce((sum, s) => sum + s.response_time_hours, 0)
  return Math.round(total / withResponse.length)
})

// Chart data
const hasSeverityData = computed(() => {
  const data = props.findingStats?.by_severity
  return data && Object.values(data).some((v: any) => v > 0)
})

const severityChartData = computed(() => ({
  labels: [
    t('bugBounty.severity.critical'),
    t('bugBounty.severity.high'),
    t('bugBounty.severity.medium'),
    t('bugBounty.severity.low'),
    t('bugBounty.severity.info')
  ],
  datasets: [{
    data: [
      props.findingStats?.by_severity?.critical || 0,
      props.findingStats?.by_severity?.high || 0,
      props.findingStats?.by_severity?.medium || 0,
      props.findingStats?.by_severity?.low || 0,
      props.findingStats?.by_severity?.info || 0,
    ],
    backgroundColor: ['#ef4444', '#f97316', '#3b82f6', '#22c55e', '#6b7280'],
    borderWidth: 0,
  }]
}))

const hasStatusData = computed(() => {
  return props.submissions.length > 0
})

const statusChartData = computed(() => {
  const statusCounts: Record<string, number> = {}
  props.submissions.forEach(s => {
    const status = s.status || 'draft'
    statusCounts[status] = (statusCounts[status] || 0) + 1
  })
  
  const statusLabels: Record<string, string> = {
    draft: t('bugBounty.submissionStatus.draft'),
    submitted: t('bugBounty.submissionStatus.submitted'),
    triaged: t('bugBounty.submissionStatus.triaged'),
    accepted: t('bugBounty.submissionStatus.accepted'),
    rejected: t('bugBounty.submissionStatus.rejected'),
    duplicate: t('bugBounty.submissionStatus.duplicate'),
    informative: t('bugBounty.submissionStatus.informative'),
    resolved: t('bugBounty.submissionStatus.resolved'),
  }
  
  const statusColors: Record<string, string> = {
    draft: '#6b7280',
    submitted: '#3b82f6',
    triaged: '#8b5cf6',
    accepted: '#22c55e',
    rejected: '#ef4444',
    duplicate: '#f97316',
    informative: '#64748b',
    resolved: '#10b981',
  }

  return {
    labels: Object.keys(statusCounts).map(s => statusLabels[s] || s),
    datasets: [{
      data: Object.values(statusCounts),
      backgroundColor: Object.keys(statusCounts).map(s => statusColors[s] || '#6b7280'),
      borderWidth: 0,
    }]
  }
})

const hasVulnTypeData = computed(() => props.findings.length > 0)

const vulnTypeChartData = computed(() => {
  const typeCounts: Record<string, number> = {}
  props.findings.forEach(f => {
    const type = f.finding_type || 'Other'
    typeCounts[type] = (typeCounts[type] || 0) + 1
  })
  
  const sorted = Object.entries(typeCounts).sort((a, b) => b[1] - a[1]).slice(0, 10)
  
  return {
    labels: sorted.map(([type]) => type),
    datasets: [{
      label: t('bugBounty.statistics.count'),
      data: sorted.map(([, count]) => count),
      backgroundColor: '#3b82f6',
      borderRadius: 4,
    }]
  }
})

const hasEarningsData = computed(() => {
  return props.submissions.some(s => s.reward_amount || s.bonus_amount)
})

const earningsChartData = computed(() => {
  // Group by month
  const monthlyEarnings: Record<string, number> = {}
  
  props.submissions.forEach(s => {
    if (!s.submitted_at && !s.created_at) return
    const date = new Date(s.submitted_at || s.created_at)
    const monthKey = `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, '0')}`
    const earnings = (s.reward_amount || 0) + (s.bonus_amount || 0)
    monthlyEarnings[monthKey] = (monthlyEarnings[monthKey] || 0) + earnings
  })
  
  const sortedMonths = Object.keys(monthlyEarnings).sort()
  const last6Months = sortedMonths.slice(-6)
  
  // Calculate cumulative
  let cumulative = 0
  const cumulativeData = last6Months.map(month => {
    cumulative += monthlyEarnings[month]
    return cumulative
  })
  
  return {
    labels: last6Months.map(m => {
      const [year, month] = m.split('-')
      return `${month}/${year.slice(2)}`
    }),
    datasets: [
      {
        label: t('bugBounty.statistics.monthlyEarnings'),
        data: last6Months.map(m => monthlyEarnings[m]),
        backgroundColor: 'rgba(34, 197, 94, 0.2)',
        borderColor: '#22c55e',
        borderWidth: 2,
        fill: true,
        tension: 0.3,
      },
      {
        label: t('bugBounty.statistics.cumulativeEarnings'),
        data: cumulativeData,
        backgroundColor: 'transparent',
        borderColor: '#3b82f6',
        borderWidth: 2,
        borderDash: [5, 5],
        tension: 0.3,
      }
    ]
  }
})

// Program stats
const programStats = computed(() => {
  const stats = props.programs.map(p => {
    const programFindings = props.findings.filter(f => f.program_id === p.id)
    const programSubmissions = props.submissions.filter(s => s.program_id === p.id)
    const accepted = programSubmissions.filter(s => ['accepted', 'resolved'].includes(s.status?.toLowerCase()))
    const earnings = programSubmissions.reduce((sum, s) => sum + (s.reward_amount || 0) + (s.bonus_amount || 0), 0)
    
    return {
      id: p.id,
      name: p.name,
      findings_count: programFindings.length,
      submissions_count: programSubmissions.length,
      accepted_count: accepted.length,
      total_earnings: earnings,
    }
  })
  
  return stats.sort((a, b) => b.total_earnings - a.total_earnings).slice(0, 10)
})

// Chart options
const doughnutOptions = {
  responsive: true,
  maintainAspectRatio: false,
  plugins: {
    legend: {
      position: 'right' as const,
      labels: {
        boxWidth: 12,
        padding: 8,
        font: { size: 11 }
      }
    }
  },
  cutout: '60%',
}

const barOptions = {
  responsive: true,
  maintainAspectRatio: false,
  plugins: {
    legend: { display: false }
  },
  scales: {
    y: {
      beginAtZero: true,
      ticks: { stepSize: 1 }
    }
  }
}

const lineOptions = {
  responsive: true,
  maintainAspectRatio: false,
  plugins: {
    legend: {
      position: 'top' as const,
      labels: {
        boxWidth: 12,
        padding: 8,
        font: { size: 11 }
      }
    }
  },
  scales: {
    y: {
      beginAtZero: true,
      ticks: {
        callback: (value: any) => `$${value}`
      }
    }
  }
}
</script>
