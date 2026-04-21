<template>
  <div class="flex-1 overflow-auto p-3">
    <div v-if="sessionMessages.length === 0" class="text-sm text-base-content/50">
      {{ t('agent.teamWorkspaceNoInboxMessages') }}
    </div>
    <div v-else class="space-y-2">
      <div
        v-for="msg in sessionMessages"
        :key="msg.id"
        class="rounded-lg border border-base-300 bg-base-100 p-2"
      >
        <div class="flex items-center justify-between gap-2">
          <div class="text-xs font-semibold">{{ msg.role }}</div>
          <span class="text-[11px] text-base-content/45">{{ formatTimestamp(msg.timestamp) }}</span>
        </div>
        <div class="mt-1 text-[11px] text-base-content/60">
          {{ msg.member_name || msg.member_id || t('agent.systemMessageLabel') }}
        </div>
        <div class="mt-1 rounded bg-base-200/60 p-1.5 text-[11px] whitespace-pre-wrap break-words">
          {{ msg.content || '—' }}
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import type { AgentTeamMessage } from '@/types/agentTeam'

defineProps<{
  sessionMessages: AgentTeamMessage[]
  formatTimestamp: (value?: string | null) => string
}>()

const { t } = useI18n()
</script>
