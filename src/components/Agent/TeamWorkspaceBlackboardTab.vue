<template>
  <div class="flex-1 overflow-auto p-3">
    <div v-if="blackboardEntries.length === 0" class="text-sm text-base-content/50">
      {{ t('agent.teamWorkspaceNoBlackboardEntries') }}
    </div>
    <div v-else class="space-y-2">
      <div
        v-for="entry in blackboardEntries"
        :key="entry.id"
        class="rounded-lg border border-base-300 bg-base-100 p-2"
      >
        <div class="flex items-center justify-between gap-2">
          <span class="badge badge-xs" :class="badgeClass(entry.entry_type)">
            {{ t(labelKey(entry.entry_type)) }}
          </span>
          <span class="text-[11px] text-base-content/45">{{ formatTimestamp(entry.created_at) }}</span>
        </div>
        <div class="mt-1 text-[11px] text-base-content/60">
          {{ t('agent.teamBlackboardAgentLabel') }}: {{ resolveAgentName(entry.agent_id) }} ·
          {{ t('agent.teamBlackboardTaskLabel') }}: {{ entry.task_id || '-' }}
        </div>
        <template v-if="isArtifactRefEntry(entry)">
          <div class="mt-1 rounded bg-base-200/60 p-1.5 text-[11px] whitespace-pre-wrap break-words">
            {{ artifactSummary(entry) || entry.content || '—' }}
          </div>
          <div v-if="artifactPath(entry)" class="mt-1 text-[11px] text-base-content/65 break-all">
            {{ t('agent.teamArtifactFileLabel') }}: {{ artifactPath(entry) }}
          </div>
          <div v-if="artifactContainerPath(entry)" class="mt-0.5 text-[11px] text-base-content/55 break-all">
            {{ t('agent.teamArtifactContainerLabel') }}: {{ artifactContainerPath(entry) }}
          </div>
          <div v-if="artifactHostPath(entry)" class="mt-0.5 text-[11px] text-base-content/55 break-all">
            {{ t('agent.teamArtifactHostLabel') }}: {{ artifactHostPath(entry) }}
          </div>
          <div v-if="artifactBytes(entry) !== null" class="mt-0.5 text-[11px] text-base-content/55">
            {{ t('agent.teamArtifactSizeLabel') }}: {{ formatBytes(artifactBytes(entry) ?? 0) }}
          </div>
        </template>
        <div v-else class="mt-1 rounded bg-base-200/60 p-1.5 text-[11px] whitespace-pre-wrap break-words">
          {{ entry.content || '—' }}
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import type { TeamBlackboardEntry } from '@/types/agentTeam'

defineProps<{
  blackboardEntries: TeamBlackboardEntry[]
  formatTimestamp: (value?: string | null) => string
  resolveAgentName: (agentId?: string | null) => string
  badgeClass: (entryType?: string | null) => string
  labelKey: (entryType?: string | null) => string
  isArtifactRefEntry: (entry: TeamBlackboardEntry) => boolean
  artifactSummary: (entry: TeamBlackboardEntry) => string
  artifactPath: (entry: TeamBlackboardEntry) => string
  artifactContainerPath: (entry: TeamBlackboardEntry) => string
  artifactHostPath: (entry: TeamBlackboardEntry) => string
  artifactBytes: (entry: TeamBlackboardEntry) => number | null
  formatBytes: (bytes: number) => string
}>()

const { t } = useI18n()
</script>
