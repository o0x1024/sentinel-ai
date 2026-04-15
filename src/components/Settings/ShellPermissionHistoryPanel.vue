<template>
  <div class="card bg-base-100 shadow-sm mb-6">
    <div class="card-body gap-4">
      <div class="flex items-start justify-between gap-4">
        <div>
          <h3 class="card-title">
            <i class="fas fa-shield-halved"></i>
            {{ t('settings.agent.permissionHistory.title') }}
          </h3>
          <p class="text-sm text-base-content/70 mt-1">
            {{ t('settings.agent.permissionHistory.desc') }}
          </p>
        </div>
        <button class="btn btn-sm btn-ghost" :disabled="loading" @click="loadHistory">
          <span v-if="loading" class="loading loading-spinner loading-xs mr-2"></span>
          <i v-else class="fas fa-rotate mr-2"></i>
          {{ t('settings.agent.permissionHistory.refresh') }}
        </button>
      </div>

      <div class="rounded-xl border border-base-300 bg-base-200/30 p-4">
        <div class="text-xs font-semibold uppercase tracking-wide text-base-content/60 mb-3">
          {{ t('settings.agent.permissionHistory.filtersTitle') }}
        </div>
        <div class="grid grid-cols-1 lg:grid-cols-2 xl:grid-cols-5 gap-3">
          <label class="form-control">
            <span class="label-text text-sm">
              {{ t('settings.agent.permissionHistory.executionId') }}
            </span>
            <input
              v-model.trim="executionIdFilter"
              type="text"
              class="input input-bordered input-sm font-mono"
              :placeholder="t('settings.agent.permissionHistory.executionIdPlaceholder')"
              @keyup.enter="loadHistory"
            />
          </label>

          <label class="form-control">
            <span class="label-text text-sm">
              {{ t('settings.agent.permissionHistory.exactDate') }}
            </span>
            <input
              v-model="selectedDate"
              type="date"
              class="input input-bordered input-sm"
            />
          </label>

          <label class="form-control">
            <span class="label-text text-sm">
              {{ t('settings.agent.permissionHistory.recentDays') }}
            </span>
            <select
              v-model.number="recentDays"
              class="select select-bordered select-sm"
              :disabled="!!selectedDate"
            >
              <option :value="1">1</option>
              <option :value="3">3</option>
              <option :value="7">7</option>
              <option :value="14">14</option>
              <option :value="30">30</option>
            </select>
          </label>

          <label class="form-control">
            <span class="label-text text-sm">
              {{ t('settings.agent.permissionHistory.decision') }}
            </span>
            <select v-model="decisionFilter" class="select select-bordered select-sm">
              <option value="all">{{ t('settings.agent.permissionHistory.allDecisions') }}</option>
              <option value="allow">
                {{ t('settings.agent.permissionHistory.decisions.allow') }}
              </option>
              <option value="deny">
                {{ t('settings.agent.permissionHistory.decisions.deny') }}
              </option>
              <option value="allow_forever">
                {{ t('settings.agent.permissionHistory.decisions.allowForever') }}
              </option>
            </select>
          </label>

          <label class="form-control">
            <span class="label-text text-sm">
              {{ t('settings.agent.permissionHistory.semanticKind') }}
            </span>
            <select v-model="semanticKindFilter" class="select select-bordered select-sm">
              <option value="all">{{ t('settings.agent.permissionHistory.allSemanticKinds') }}</option>
              <option value="read_only">{{ t('tools.shell.semanticLabels.read_only') }}</option>
              <option value="mutating">{{ t('tools.shell.semanticLabels.mutating') }}</option>
              <option value="dangerous">{{ t('tools.shell.semanticLabels.dangerous') }}</option>
            </select>
          </label>
        </div>

        <div class="flex flex-wrap gap-2 mt-3">
          <button class="btn btn-sm btn-primary" :disabled="loading" @click="loadHistory">
            {{ t('settings.agent.permissionHistory.applyFilters') }}
          </button>
          <button class="btn btn-sm btn-ghost" :disabled="loading" @click="resetFilters">
            {{ t('settings.agent.permissionHistory.clearFilters') }}
          </button>
        </div>
      </div>

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
        <div
          v-for="entry in visibleHistory"
          :key="`${entry.timestamp}-${entry.session_id}-${entry.command}`"
          class="rounded-xl border border-base-300 bg-base-200/30 p-4"
        >
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
              <div v-if="entry.semantic_reason_key" class="text-base-content/70 mt-1">
                {{ t(entry.semantic_reason_key) }}
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

          <div v-if="entry.persisted_allow_rules.length > 0" class="mt-3">
            <div class="text-xs font-semibold uppercase tracking-wide text-base-content/60">
              {{ t('settings.agent.permissionHistory.persistedRules') }}
            </div>
            <div class="mt-2 flex flex-wrap gap-2">
              <span
                v-for="rule in entry.persisted_allow_rules"
                :key="`${entry.timestamp}-persisted-${rule}`"
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
                :key="`${entry.timestamp}-suggested-${item.rule}`"
                class="rounded-lg bg-base-100 px-3 py-2"
              >
                <div class="font-mono text-sm break-all">{{ item.rule }}</div>
                <div class="text-xs text-base-content/60 mt-1">{{ t(item.reason_key) }}</div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useI18n } from 'vue-i18n'

interface ShellPermissionRulePreview {
  rule: string
  reason_key: string
}

interface ShellPermissionHistoryEntry {
  timestamp: string
  session_id: string
  execution_id?: string | null
  command: string
  decision: string
  allowed: boolean
  semantic_kind: string
  semantic_code: string
  semantic_summary_key: string
  semantic_reason_key?: string | null
  suggested_allow_rules: ShellPermissionRulePreview[]
  persisted_allow_rules: string[]
}

const { t } = useI18n()

const loading = ref(false)
const history = ref<ShellPermissionHistoryEntry[]>([])
const executionIdFilter = ref('')
const selectedDate = ref('')
const recentDays = ref(7)
const decisionFilter = ref('all')
const semanticKindFilter = ref('all')

const decisionLabelKey = (decision: string) => {
  if (decision === 'allow_forever') {
    return 'allowForever'
  }
  return decision === 'deny' ? 'deny' : 'allow'
}

const decisionBadgeClass = (decision: string) => {
  if (decision === 'allow_forever') {
    return 'badge-primary'
  }
  return decision === 'deny' ? 'badge-error' : 'badge-success'
}

const semanticBadgeClass = (semanticKind: string) => {
  if (semanticKind === 'dangerous') {
    return 'badge-error'
  }
  return semanticKind === 'mutating' ? 'badge-warning' : 'badge-info'
}

const formatTimestamp = (timestamp: string) => {
  const date = new Date(timestamp)
  if (Number.isNaN(date.getTime())) {
    return timestamp
  }
  return date.toLocaleString()
}

const visibleHistory = computed(() => {
  return history.value.filter((entry) => {
    const decisionMatches =
      decisionFilter.value === 'all' || entry.decision === decisionFilter.value
    const semanticMatches =
      semanticKindFilter.value === 'all' || entry.semantic_kind === semanticKindFilter.value
    return decisionMatches && semanticMatches
  })
})

const loadHistory = async () => {
  loading.value = true
  try {
    const request = selectedDate.value
      ? {
          date: selectedDate.value,
          execution_id: executionIdFilter.value || null,
          limit: 20,
        }
      : {
          days: recentDays.value,
          execution_id: executionIdFilter.value || null,
          limit: 20,
        }
    history.value = await invoke<ShellPermissionHistoryEntry[]>('get_shell_permission_history', {
      request,
    })
  } catch (error) {
    console.error('Failed to load shell permission history:', error)
    history.value = []
  } finally {
    loading.value = false
  }
}

const resetFilters = () => {
  executionIdFilter.value = ''
  selectedDate.value = ''
  recentDays.value = 7
  decisionFilter.value = 'all'
  semanticKindFilter.value = 'all'
  loadHistory()
}

onMounted(() => {
  loadHistory()
})
</script>
