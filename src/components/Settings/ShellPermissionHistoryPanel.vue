<template>
  <div class="card bg-base-100 shadow-sm mb-6">
    <div class="card-body gap-4">
      <ShellPermissionHistoryHeader :loading="loading" @refresh="refreshHistory" />

      <ShellPermissionHistoryFilters
        :loading="loading"
        :execution-id-filter="executionIdFilter"
        :command-search-filter="commandSearchFilter"
        :selected-date="selectedDate"
        :recent-days="recentDays"
        :decision-filter="decisionFilter"
        :semantic-kind-filter="semanticKindFilter"
        @update:execution-id-filter="executionIdFilter = $event"
        @update:command-search-filter="commandSearchFilter = $event"
        @update:selected-date="selectedDate = $event"
        @update:recent-days="recentDays = $event"
        @update:decision-filter="decisionFilter = $event"
        @update:semantic-kind-filter="semanticKindFilter = $event"
        @apply="applyFilters"
        @reset="resetFilters"
      />

      <div v-if="loading && history.length === 0" class="py-10 text-center text-base-content/60">
        <span class="loading loading-spinner loading-md"></span>
        <div class="mt-3 text-sm">{{ t('settings.agent.permissionHistory.loading') }}</div>
      </div>

      <div
        v-else-if="visibleHistory.length === 0"
        class="rounded-xl border border-dashed border-base-300 px-4 py-8 text-center text-sm text-base-content/60"
      >
        {{
          history.length === 0
            ? t('settings.agent.permissionHistory.empty')
            : t('settings.agent.permissionHistory.emptyFiltered')
        }}
      </div>

      <div v-else class="space-y-3">
        <ShellPermissionHistoryEntryCard
          v-for="entry in visibleHistory"
          :key="entryKey(entry)"
          :entry="entry"
          :expanded="isExpanded(entry)"
          @toggle="toggleExpandedAndPersist(entry)"
        />

        <div class="flex flex-wrap items-center justify-between gap-3 pt-1">
          <div class="text-sm text-base-content/60">
            {{
              t('settings.agent.permissionHistory.showingCount', {
                visible: visibleHistory.length,
                loaded: history.length,
              })
            }}
          </div>

          <button
            v-if="hasMoreHistory"
            class="btn btn-sm btn-outline"
            :disabled="loading"
            @click="loadMoreHistory"
          >
            <span v-if="loading" class="loading loading-spinner loading-xs mr-2"></span>
            {{ t('settings.agent.permissionHistory.loadMore') }}
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import ShellPermissionHistoryEntryCard from './ShellPermissionHistoryEntryCard.vue'
import ShellPermissionHistoryFilters from './ShellPermissionHistoryFilters.vue'
import ShellPermissionHistoryHeader from './ShellPermissionHistoryHeader.vue'
import { useShellPermissionHistoryPanel } from './useShellPermissionHistoryPanel'

const { t } = useI18n()
const {
  loading,
  history,
  visibleHistory,
  hasMoreHistory,
  executionIdFilter,
  commandSearchFilter,
  selectedDate,
  recentDays,
  decisionFilter,
  semanticKindFilter,
  entryKey,
  isExpanded,
  loadMoreHistory,
  toggleExpandedAndPersist,
  refreshHistory,
  applyFilters,
  resetFilters,
} = useShellPermissionHistoryPanel()
</script>
