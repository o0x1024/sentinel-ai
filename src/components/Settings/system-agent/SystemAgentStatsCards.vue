<template>
  <div class="space-y-3">
    <div class="flex flex-wrap items-center justify-between gap-3">
      <div class="text-sm font-medium text-base-content/70">运行结果统计</div>
      <div class="join">
        <button
          v-for="option in statsWindowOptions"
          :key="option.value"
          class="join-item btn btn-xs"
          :class="modelValue === option.value ? 'btn-primary' : 'btn-ghost'"
          @click="$emit('update:modelValue', option.value)"
        >
          {{ option.label }}
        </button>
      </div>
    </div>

    <div class="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-4 gap-3">
      <div
        v-for="card in statCards"
        :key="card.title"
        class="stat bg-base-200/60 rounded-lg border border-base-300"
      >
        <div class="stat-title text-xs">{{ card.title }}</div>
        <div class="stat-value text-lg">{{ card.value }}</div>
        <div class="stat-desc">{{ card.desc }}</div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'

import type {
  SystemAgentFindingSummary,
  SystemAgentProfileVersionPayload,
  SystemAgentRunPayload,
} from '../systemAgentSettingsSupport'
import type { SystemAgentStatsMode } from './systemAgentRegistry'

const props = defineProps<{
  runs: SystemAgentRunPayload[]
  recentFindings: SystemAgentFindingSummary[]
  versions: SystemAgentProfileVersionPayload[]
  totalFindingsCount: number
  falsePositiveFindingsCount: number
  modelValue: '24h' | '7d' | '30d'
  statsMode: SystemAgentStatsMode
}>()

defineEmits<{
  'update:modelValue': [value: '24h' | '7d' | '30d']
}>()

const statsWindowOptions = [
  { value: '24h' as const, label: '24h' },
  { value: '7d' as const, label: '7天' },
  { value: '30d' as const, label: '30天' },
]

const statsWindowDescription = computed(() => {
  if (props.modelValue === '7d') return '近 7 天'
  if (props.modelValue === '30d') return '近 30 天'
  return '近 24 小时'
})

const statsWindowStart = computed(() => {
  const now = Date.now()
  if (props.modelValue === '7d') return now - 7 * 24 * 60 * 60 * 1000
  if (props.modelValue === '30d') return now - 30 * 24 * 60 * 60 * 1000
  return now - 24 * 60 * 60 * 1000
})

const windowedRecentFindings = computed(() => {
  return props.recentFindings.filter(finding => {
    const seenAt = new Date(finding.last_seen_at).getTime()
    return Number.isFinite(seenAt) && seenAt >= statsWindowStart.value
  })
})

const agentStats = computed(() => {
  const recentRunsWindow = props.runs.filter(run => {
    const startedAt = new Date(run.startedAt).getTime()
    return Number.isFinite(startedAt) && startedAt >= statsWindowStart.value
  })
  const terminalRuns = recentRunsWindow.filter(run =>
    run.status === 'completed' || run.status === 'failed' || run.status === 'dead_letter',
  )
  const completedRuns = terminalRuns.filter(run => run.status === 'completed').length
  const runSuccessRate = terminalRuns.length > 0
    ? `${Math.round((completedRuns / terminalRuns.length) * 100)}%`
    : '0%'

  return {
    recentRuns: recentRunsWindow.length,
    runSuccessRate,
    totalFindings: props.totalFindingsCount,
    recentVerifiedFindings: windowedRecentFindings.value.filter(finding => hasVerificationEvidence(finding)).length,
    falsePositiveFindings: props.falsePositiveFindingsCount,
    versionCount: props.versions.length,
    latestVersionAt: props.versions[0]?.createdAt || null,
  }
})

const statCards = computed(() => {
  const cards = [
    {
      title: '最近运行',
      value: agentStats.value.recentRuns,
      desc: statsWindowDescription.value,
    },
    {
      title: '运行成功率',
      value: agentStats.value.runSuccessRate,
      desc: statsWindowDescription.value,
    },
  ]

  if (props.statsMode === 'findings') {
    cards.push(
      {
        title: '累计发现',
        value: agentStats.value.totalFindings,
        desc: '当前 Agent',
      },
      {
        title: '最近已验证',
        value: agentStats.value.recentVerifiedFindings,
        desc: `${statsWindowDescription.value}内样本`,
      },
      {
        title: '误报数',
        value: agentStats.value.falsePositiveFindings,
        desc: '累计状态统计',
      },
    )
    return cards
  }

  cards.push(
    {
      title: '配置版本',
      value: agentStats.value.versionCount,
      desc: '自动保存快照',
    },
    {
      title: '最近更新',
      value: agentStats.value.latestVersionAt ? formatDate(agentStats.value.latestVersionAt) : '-',
      desc: '最近一次配置变更',
    },
  )
  return cards
})

function hasVerificationEvidence(finding: SystemAgentFindingSummary) {
  return !!finding.evidence?.some(evidence => evidence.location === 'system_agent_verification')
}

function formatDate(value?: string | null) {
  if (!value) return '-'
  try {
    return new Date(value).toLocaleString()
  } catch {
    return value
  }
}
</script>
