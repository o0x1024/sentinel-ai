<template>
  <div class="flex-1 overflow-auto p-3">
    <div v-if="members.length === 0" class="text-sm text-base-content/50">
      {{ t('agent.teamWorkspaceNoAgents') }}
    </div>
    <div v-else class="space-y-2">
      <div
        v-for="member in members"
        :key="member.id"
        class="rounded-lg border border-base-300 bg-base-100 p-2"
      >
        <div class="flex items-center justify-between">
          <div class="text-sm font-medium">{{ member.name }}</div>
          <span class="badge badge-xs" :class="agentStatusBadgeClass(member.id, member.name, member.is_active)">
            {{ t(agentStatusI18nKey(member.id, member.name, member.is_active)) }}
          </span>
        </div>
        <div class="mt-1 text-[11px] text-base-content/55">
          {{ member.responsibility || t('agent.teamWorkspaceNoResponsibility') }}
        </div>
        <div class="mt-1 text-[11px] text-base-content/50">
          {{ t('agent.teamAgentTokensLabel') }}: {{ member.token_usage }} ·
          {{ t('agent.teamAgentToolsLabel') }}: {{ member.tool_calls_count }}
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import type { AgentTeamMember } from '@/types/agentTeam'

const props = defineProps<{
  members: AgentTeamMember[]
  agentStatusI18nKey: (memberId?: string, memberName?: string, isActive?: boolean) => string
  agentStatusBadgeClass: (memberId?: string, memberName?: string, isActive?: boolean) => string
}>()

const { t } = useI18n()
const members = computed(() => props.members || [])
</script>
