<template>
  <div class="border border-base-300 rounded-lg overflow-hidden">
    <div class="px-4 py-3 bg-base-200">
      <div class="font-semibold text-sm">Skill 命中统计</div>
      <div class="text-xs text-base-content/60 mt-1">基于最近发现中的 Agent 上下文 evidence 聚合。</div>
    </div>
    <div v-if="skillStats.length === 0" class="p-4 text-sm text-base-content/60">
      当前时间窗口内没有可统计的 skill 命中数据。
    </div>
    <div v-else class="p-4 space-y-3">
      <div class="grid grid-cols-1 md:grid-cols-3 gap-3">
        <div class="stat bg-base-200/60 rounded-lg border border-base-300">
          <div class="stat-title text-xs">命中技能数</div>
          <div class="stat-value text-lg">{{ skillStats.length }}</div>
          <div class="stat-desc">当前窗口</div>
        </div>
        <div class="stat bg-base-200/60 rounded-lg border border-base-300">
          <div class="stat-title text-xs">总命中次数</div>
          <div class="stat-value text-lg">{{ totalHits }}</div>
          <div class="stat-desc">来自 system_agent_context</div>
        </div>
        <div class="stat bg-base-200/60 rounded-lg border border-base-300">
          <div class="stat-title text-xs">Top Skill</div>
          <div class="stat-value text-sm">{{ skillStats[0]?.id || '-' }}</div>
          <div class="stat-desc">{{ skillStats[0]?.hits || 0 }} 次</div>
        </div>
      </div>

      <div class="space-y-2">
        <div
          v-for="skill in skillStats"
          :key="skill.id"
          class="bg-base-200/60 border border-base-300 rounded-lg p-3"
        >
          <div class="flex items-center justify-between gap-3">
            <div class="min-w-0">
              <div class="font-medium text-sm break-words">{{ skill.name || skill.id }}</div>
              <div class="text-xs text-base-content/60 break-words">{{ skill.description || '无描述' }}</div>
            </div>
            <div class="badge badge-accent badge-sm shrink-0">{{ skill.hits }} 次</div>
          </div>
          <div v-if="skill.sampleReasons.length > 0" class="text-xs text-base-content/70 mt-2">
            <span class="font-semibold">示例理由：</span>
            {{ skill.sampleReasons.join('；') }}
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'

import type { SystemAgentFindingSummary } from '../systemAgentSettingsSupport'

const props = defineProps<{
  findings: SystemAgentFindingSummary[]
  windowStart: number
}>()

interface SkillStatEntry {
  id: string
  name?: string
  description?: string
  hits: number
  sampleReasons: string[]
}

const windowedFindings = computed(() => {
  return props.findings.filter(finding => {
    const seenAt = new Date(finding.last_seen_at).getTime()
    return Number.isFinite(seenAt) && seenAt >= props.windowStart
  })
})

const skillStats = computed<SkillStatEntry[]>(() => {
  const entries = new Map<string, SkillStatEntry>()

  for (const finding of windowedFindings.value) {
    for (const evidence of finding.evidence ?? []) {
      if (evidence.location !== 'system_agent_context' || !evidence.request_body) {
        continue
      }
      const payload = parseJson(evidence.request_body)
      const skills = Array.isArray(payload?.logicSkillContext) ? payload.logicSkillContext : []
      for (const skill of skills) {
        const id = typeof skill?.id === 'string' ? skill.id : null
        if (!id) continue
        const existing = entries.get(id) ?? {
          id,
          name: typeof skill?.name === 'string' ? skill.name : undefined,
          description: typeof skill?.description === 'string' ? skill.description : undefined,
          hits: 0,
          sampleReasons: [],
        }
        existing.hits += 1
        const reasons = Array.isArray(skill?.reasons)
          ? skill.reasons.filter((item: unknown): item is string => typeof item === 'string')
          : []
        for (const reason of reasons) {
          if (!existing.sampleReasons.includes(reason) && existing.sampleReasons.length < 3) {
            existing.sampleReasons.push(reason)
          }
        }
        entries.set(id, existing)
      }
    }
  }

  return Array.from(entries.values()).sort((left, right) => right.hits - left.hits)
})

const totalHits = computed(() => skillStats.value.reduce((sum, item) => sum + item.hits, 0))

function parseJson(raw?: string | null) {
  if (!raw) return null
  try {
    return JSON.parse(raw)
  } catch {
    return null
  }
}
</script>
