<template>
  <div class="rounded-xl border border-base-300 bg-base-200/30 p-4">
    <div class="flex items-start justify-between gap-3">
      <div class="min-w-0 flex-1">
        <div class="flex flex-wrap items-center gap-2">
          <span class="badge badge-sm" :class="decisionBadgeClass(entry.decision)">
            {{ t(`settings.agent.permissionHistory.decisions.${decisionLabelKey(entry.decision)}`) }}
          </span>
          <span class="badge badge-sm" :class="semanticBadgeClass(entry.semantic_kind)">
            {{ t(`tools.shell.semanticLabels.${entry.semantic_kind}`) }}
          </span>
          <span class="text-xs text-base-content/60">{{ formatTimestamp(entry.timestamp) }}</span>
        </div>

        <div class="mt-3 rounded-lg bg-base-100 px-3 py-2 font-mono text-sm break-all">
          {{ entry.command }}
        </div>

        <div class="mt-3 grid grid-cols-1 lg:grid-cols-2 gap-3 text-sm">
          <div>
            <div class="font-medium">{{ t(entry.semantic_summary_key) }}</div>
            <div class="text-xs text-base-content/60 mt-1">
              {{ buildDetailSummary(entry, t) }}
            </div>
          </div>
          <div class="space-y-1 text-base-content/70">
            <div>
              <span class="font-medium text-base-content">{{ t('settings.agent.permissionHistory.semanticCode') }}:</span>
              <code class="ml-2 text-xs">{{ entry.semantic_code }}</code>
            </div>
            <div v-if="entry.execution_id">
              <span class="font-medium text-base-content">{{ t('settings.agent.permissionHistory.executionId') }}:</span>
              <code class="ml-2 text-xs break-all">{{ entry.execution_id }}</code>
            </div>
          </div>
        </div>
      </div>

      <button class="btn btn-sm btn-ghost shrink-0" @click="$emit('toggle')">
        <i class="fas" :class="expanded ? 'fa-chevron-up' : 'fa-chevron-down'"></i>
        {{
          expanded
            ? t('settings.agent.permissionHistory.hideDetails')
            : t('settings.agent.permissionHistory.showDetails')
        }}
      </button>
    </div>

    <div v-if="expanded" class="mt-3 border-t border-base-300 pt-3">
      <div v-if="entry.semantic_reason_key" class="text-sm text-base-content/70">
        {{ t(entry.semantic_reason_key) }}
      </div>

      <div v-if="entry.persisted_allow_rules.length > 0" class="mt-3">
        <div class="text-xs font-semibold uppercase tracking-wide text-base-content/60">
          {{ t('settings.agent.permissionHistory.persistedRules') }}
        </div>
        <div class="mt-2 flex flex-wrap gap-2">
          <span
            v-for="rule in entry.persisted_allow_rules"
            :key="`persisted-${rule}`"
            class="badge badge-outline badge-sm font-mono"
          >
            {{ rule }}
          </span>
        </div>
      </div>

      <div v-if="entry.suggested_allow_rules.length > 0" class="mt-3">
        <div class="text-xs font-semibold uppercase tracking-wide text-base-content/60">
          {{ t('settings.agent.permissionHistory.suggestedRules') }}
        </div>
        <div class="mt-2 space-y-2">
          <div
            v-for="item in entry.suggested_allow_rules"
            :key="`suggested-${item.rule}`"
            class="rounded-lg bg-base-100 px-3 py-2"
          >
            <div class="font-mono text-sm break-all">{{ item.rule }}</div>
            <div class="text-xs text-base-content/60 mt-1">{{ t(item.reason_key) }}</div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import {
  buildDetailSummary,
  decisionBadgeClass,
  decisionLabelKey,
  formatTimestamp,
  semanticBadgeClass,
} from './shellPermissionHistorySupport'
import type { ShellPermissionHistoryEntry } from './useShellPermissionHistoryPanel'

defineEmits<{
  toggle: []
}>()

const props = defineProps<{
  entry: ShellPermissionHistoryEntry
  expanded: boolean
}>()

const { t } = useI18n()

void props
</script>
