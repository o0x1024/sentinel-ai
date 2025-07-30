<template>
  <div class="space-y-6">
    <!-- 页面标题 -->
    <div class="flex items-center justify-between">
      <h1 class="text-3xl font-bold">{{ $t('submissions.title') }}</h1>
      <div class="flex gap-2">
        <select v-model="statusFilter" class="select select-bordered">
          <option value="all">{{ $t('common.all') }}</option>
          <option value="pending">{{ $t('common.pending') }}</option>
          <option value="confirmed">{{ $t('submissions.filters.accepted') }}</option>
          <option value="fixed">{{ $t('vulnerabilities.filters.fixed') }}</option>
          <option value="duplicate">{{ $t('submissions.filters.rejected') }}</option>
          <option value="invalid">{{ $t('vulnerabilities.filters.notExploitable') }}</option>
        </select>
        <button @click="createSubmission" class="btn btn-primary">
          <i class="fas fa-plus mr-2"></i>{{ $t('submissions.newSubmission') }}
        </button>
      </div>
    </div>

    <!-- 提交统计 -->
    <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6">
      <StatsCard
        :value="submissions.length"
        :label="$t('common.total')"
        :subtitle="$t('submissions.allSubmissions')"
        icon="fas fa-paper-plane"
        theme="primary"
      />
      
      <StatsCard
        :value="getStatusCount('confirmed')"
        :label="$t('submissions.filters.accepted')"
        :subtitle="$t('vulnerabilities.filters.inProgress')"
        icon="fas fa-check-circle"
        theme="success"
      />
      
      <StatsCard
        :value="getStatusCount('pending')"
        :label="$t('common.pending')"
        :subtitle="$t('vulnerabilities.filters.inProgress')"
        icon="fas fa-clock"
        theme="warning"
      />
      
      <StatsCard
        :value="getStatusCount('duplicate')"
        :label="$t('submissions.filters.rejected')"
        :subtitle="$t('common.noData')"
        icon="fas fa-copy"
        theme="error"
      />
    </div>

    <!-- 提交记录列表 -->
    <div class="grid grid-cols-1 gap-4">
      <div 
        v-for="submission in filteredSubmissions" 
        :key="submission.id" 
        :class="getSubmissionBorderClass(submission.status)"
        class="card bg-base-100 shadow-lg border-l-4"
      >
        <div class="card-body">
          <div class="flex justify-between items-start">
            <div class="flex-1">
              <div class="flex items-center gap-2 mb-2">
                <h3 class="card-title">{{ submission.title }}</h3>
                <div :class="getStatusBadgeClass(submission.status)" class="badge">
                  {{ getStatusText(submission.status) }}
                </div>
                <div :class="getSeverityBadgeClass(submission.severity)" class="badge">
                  {{ submission.severity }}
                </div>
                <div class="badge badge-outline">{{ submission.platform }}</div>
              </div>
              
              <p class="text-sm opacity-70 mb-2">
                {{ $t('common.target') }}: {{ submission.target }} | {{ $t('vulnerabilities.vulnerabilityType') }}: {{ submission.parameter }}
              </p>
              
              <div class="flex gap-4 text-xs mb-2">
                <span>{{ $t('submissions.submissionDate') }}: {{ formatDate(submission.submittedAt) }}</span>
                <span v-if="submission.responseTime">{{ $t('common.time') }}: {{ submission.responseTime }}</span>
                <span v-if="submission.fixedAt">{{ $t('vulnerabilities.filters.fixed') }}: {{ formatDate(submission.fixedAt) }}</span>
              </div>
              
              <div class="flex items-center gap-2">
                <span class="text-lg font-bold" :class="getRewardClass(submission.reward)">
                  {{ submission.reward > 0 ? `$${submission.reward.toLocaleString()}` : $t('common.pending') }}
                </span>
                <span class="text-xs opacity-60">{{ getRewardLabel(submission.status) }}</span>
              </div>
            </div>
            
            <div class="flex flex-col gap-2 ml-4">
              <button @click="viewDetails(submission)" class="btn btn-ghost btn-sm">
                <i class="fas fa-eye mr-1"></i>{{ $t('common.details') }}
              </button>
              <button 
                v-if="canEdit(submission)" 
                @click="editSubmission(submission)" 
                class="btn btn-ghost btn-sm"
              >
                <i class="fas fa-edit mr-1"></i>{{ $t('common.edit') }}
              </button>
              <button 
                v-if="canResubmit(submission)" 
                @click="resubmit(submission)" 
                class="btn btn-ghost btn-sm"
              >
                <i class="fas fa-redo mr-1"></i>{{ $t('submissions.form.submit') }}
              </button>
            </div>
          </div>
          
          <!-- 进度条 -->
          <div v-if="submission.status === 'pending'" class="mt-4">
            <div class="flex justify-between text-xs mb-1">
              <span>{{ $t('common.progress') }}</span>
              <span>{{ getProgressText(submission) }}</span>
            </div>
            <progress 
              class="progress progress-primary w-full" 
              :value="getProgressValue(submission)" 
              max="100"
            ></progress>
          </div>
          
          <!-- 时间线 -->
          <div v-if="submission.timeline.length > 0" class="mt-4">
            <div class="collapse collapse-arrow">
              <input type="checkbox" />
              <div class="collapse-title text-sm font-medium">
                <i class="fas fa-history mr-2"></i>{{ $t('submissions.submissionDate') }}
              </div>
              <div class="collapse-content">
                <div class="timeline timeline-vertical">
                  <div 
                    v-for="(event, index) in submission.timeline" 
                    :key="index"
                    class="timeline-item"
                  >
                    <div class="timeline-marker">
                      <i :class="getTimelineIcon(event.type)" class="text-xs"></i>
                    </div>
                    <div class="timeline-content">
                      <div class="text-sm font-medium">{{ event.title }}</div>
                      <div class="text-xs opacity-70">{{ formatDate(event.date) }}</div>
                      <div v-if="event.description" class="text-xs mt-1">{{ event.description }}</div>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- 空状态 -->
    <div v-if="filteredSubmissions.length === 0" class="text-center py-12">
      <i class="fas fa-inbox text-6xl opacity-30 mb-4"></i>
      <h3 class="text-lg font-medium mb-2">{{ $t('common.noData') }}</h3>
      <p class="text-sm opacity-70 mb-4">
        {{ statusFilter === 'all' ? $t('submissions.allSubmissions') : getStatusText(statusFilter) }}
      </p>
      <button @click="createSubmission" class="btn btn-primary">
        <i class="fas fa-plus mr-2"></i>{{ $t('submissions.newSubmission') }}
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import StatsCard from '../components/Dashboard/StatsCard.vue'
import { useI18n } from 'vue-i18n'

const { t } = useI18n()

// 类型定义
interface Timeline {
  type: string
  title: string
  date: Date
  description?: string
}

interface Submission {
  id: string
  title: string
  target: string
  parameter: string
  platform: string
  status: string
  severity: string
  submittedAt: Date
  responseTime?: string
  fixedAt?: Date
  reward: number
  timeline: Timeline[]
}

// 响应式数据
const statusFilter = ref('all')

const submissions = ref<Submission[]>([
  {
    id: '1',
    title: 'SQL注入漏洞 - TechCorp',
    target: 'api.techcorp.com/login',
    parameter: 'username',
    platform: 'HackerOne',
    status: 'confirmed',
    severity: 'High',
    submittedAt: new Date('2024-01-15T14:30:00'),
    responseTime: '2小时',
    reward: 2500,
    timeline: [
      { type: 'submit', title: '提交漏洞报告', date: new Date('2024-01-15T14:30:00') },
      { type: 'review', title: '开始审核', date: new Date('2024-01-15T15:00:00') },
      { type: 'confirm', title: '漏洞确认', date: new Date('2024-01-15T16:30:00'), description: '高危SQL注入，影响用户认证' }
    ]
  },
  {
    id: '2',
    title: 'XSS漏洞 - FinanceApp',
    target: 'app.finance.com/search',
    parameter: 'q',
    platform: 'Bugcrowd',
    status: 'pending',
    severity: 'Medium',
    submittedAt: new Date('2024-01-16T09:15:00'),
    reward: 0,
    timeline: [
      { type: 'submit', title: '提交漏洞报告', date: new Date('2024-01-16T09:15:00') },
      { type: 'review', title: '开始审核', date: new Date('2024-01-16T10:00:00') }
    ]
  },
  {
    id: '3',
    title: 'IDOR漏洞 - SocialApp',
    target: 'api.social.com/user',
    parameter: 'user_id',
    platform: '补天',
    status: 'duplicate',
    severity: 'Medium',
    submittedAt: new Date('2024-01-14T16:45:00'),
    responseTime: '1天',
    reward: 0,
    timeline: [
      { type: 'submit', title: '提交漏洞报告', date: new Date('2024-01-14T16:45:00') },
      { type: 'review', title: '开始审核', date: new Date('2024-01-15T09:00:00') },
      { type: 'duplicate', title: '标记为重复', date: new Date('2024-01-15T16:45:00'), description: '该漏洞已被其他研究者提交' }
    ]
  },
  {
    id: '4',
    title: 'SSRF漏洞 - CloudService',
    target: 'api.cloudservice.com/webhook',
    parameter: 'url',
    platform: 'HackerOne',
    status: 'fixed',
    severity: 'High',
    submittedAt: new Date('2024-01-10T11:20:00'),
    responseTime: '6小时',
    fixedAt: new Date('2024-01-12T14:30:00'),
    reward: 3500,
    timeline: [
      { type: 'submit', title: '提交漏洞报告', date: new Date('2024-01-10T11:20:00') },
      { type: 'review', title: '开始审核', date: new Date('2024-01-10T12:00:00') },
      { type: 'confirm', title: '漏洞确认', date: new Date('2024-01-10T17:20:00') },
      { type: 'fix', title: '漏洞修复', date: new Date('2024-01-12T14:30:00'), description: '已修复SSRF漏洞，更新了URL验证逻辑' }
    ]
  }
])

// 计算属性
const filteredSubmissions = computed(() => {
  if (statusFilter.value === 'all') {
    return submissions.value
  }
  return submissions.value.filter(s => s.status === statusFilter.value)
})

// 方法
const getStatusCount = (status: string) => {
  return submissions.value.filter(s => s.status === status).length
}

const getSubmissionBorderClass = (status: string) => {
  switch (status) {
    case 'confirmed':
      return 'border-success'
    case 'pending':
      return 'border-warning'
    case 'fixed':
      return 'border-info'
    case 'duplicate':
      return 'border-error'
    case 'invalid':
      return 'border-neutral'
    default:
      return 'border-base-300'
  }
}

const getStatusBadgeClass = (status: string) => {
  switch (status) {
    case 'confirmed':
      return 'badge-success'
    case 'pending':
      return 'badge-warning'
    case 'fixed':
      return 'badge-info'
    case 'duplicate':
      return 'badge-error'
    case 'invalid':
      return 'badge-neutral'
    default:
      return 'badge-ghost'
  }
}

const getSeverityBadgeClass = (severity: string) => {
  switch (severity) {
    case 'Critical':
      return 'badge-error'
    case 'High':
      return 'badge-warning'
    case 'Medium':
      return 'badge-info'
    case 'Low':
      return 'badge-success'
    default:
      return 'badge-ghost'
  }
}

const getStatusText = (status: string) => {
  switch (status) {
    case 'pending': return t('common.pending')
    case 'confirmed': return t('submissions.filters.accepted')
    case 'fixed': return t('vulnerabilities.filters.fixed')
    case 'duplicate': return t('submissions.filters.rejected')
    case 'invalid': return t('vulnerabilities.filters.notExploitable')
    case 'all': return t('common.all')
    default: return status
  }
}

const getRewardClass = (reward: number) => {
  if (reward === 0) return 'text-base-content'
  if (reward >= 3000) return 'text-success'
  if (reward >= 1000) return 'text-warning'
  return 'text-info'
}

const getRewardLabel = (status: string) => {
  switch (status) {
    case 'confirmed':
      return t('submissions.reward')
    case 'fixed':
      return t('submissions.reward')
    case 'pending':
      return t('common.pending')
    case 'duplicate':
      return t('submissions.filters.rejected')
    case 'invalid':
      return t('vulnerabilities.filters.notExploitable')
    default:
      return t('common.pending')
  }
}

const canEdit = (submission: Submission) => {
  return submission.status === 'pending'
}

const canResubmit = (submission: Submission) => {
  return submission.status === 'duplicate' || submission.status === 'invalid'
}

const getProgressValue = (submission: Submission) => {
  // 根据提交状态和时间线计算进度
  const events = submission.timeline.length
  if (events === 1) return 25  // 仅提交
  if (events === 2) return 50  // 开始审核
  if (events >= 3) return 75   // 进一步处理
  return 25
}

const getProgressText = (submission: Submission) => {
  const events = submission.timeline.length
  if (events === 1) return t('submissions.form.submit')
  if (events === 2) return t('vulnerabilities.filters.inProgress')
  if (events >= 3) return t('vulnerabilities.filters.inProgress')
  return t('submissions.form.submit')
}

const getTimelineIcon = (type: string) => {
  switch (type) {
    case 'submit':
      return 'fas fa-paper-plane text-primary'
    case 'review':
      return 'fas fa-search text-info'
    case 'confirm':
      return 'fas fa-check text-success'
    case 'fix':
      return 'fas fa-wrench text-warning'
    case 'duplicate':
      return 'fas fa-copy text-error'
    default:
      return 'fas fa-circle text-base-content'
  }
}

const formatDate = (date: Date) => {
  return date.toLocaleString('zh-CN', {
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit'
  })
}

const createSubmission = () => {
  console.log('创建新提交')
  // 这里可以打开新建提交的模态框或导航到新页面
}

const viewDetails = (submission: Submission) => {
  console.log('查看详情:', submission.id)
  // 这里可以打开详情模态框或导航到详情页面
}

const editSubmission = (submission: Submission) => {
  console.log('编辑提交:', submission.id)
  // 这里可以打开编辑模态框
}

const resubmit = (submission: Submission) => {
  console.log('重新提交:', submission.id)
  // 这里可以处理重新提交逻辑
}
</script>

<style scoped>
.timeline {
  position: relative;
}

.timeline-item {
  display: flex;
  align-items: flex-start;
  margin-bottom: 1rem;
  position: relative;
}

.timeline-marker {
  width: 1.5rem;
  height: 1.5rem;
  border-radius: 50%;
  background: var(--fallback-b1, oklch(var(--b1)));
  border: 2px solid var(--fallback-bc, oklch(var(--bc)));
  display: flex;
  align-items: center;
  justify-content: center;
  margin-right: 1rem;
  flex-shrink: 0;
  z-index: 1;
}

.timeline-item:not(:last-child) .timeline-marker::after {
  content: '';
  position: absolute;
  top: 1.5rem;
  left: 50%;
  transform: translateX(-50%);
  width: 2px;
  height: 2rem;
  background: var(--fallback-bc, oklch(var(--bc)/0.2));
}

.timeline-content {
  flex: 1;
}

.collapse-title {
  padding: 0.5rem 0;
}
</style> 