<template>
  <div class="subagent-panel bg-base-100/80 border border-base-300 rounded-xl px-4 py-3 mb-3 shadow-sm">
    <div class="flex items-center justify-between gap-3">
      <div class="flex items-center gap-2">
        <i class="fas fa-robot text-primary"></i>
        <span class="font-semibold text-sm">{{ t('agent.subagents') }}</span>
        <span class="badge badge-sm">{{ totalCount }}</span>
      </div>
      <div class="flex items-center gap-2 text-xs">
        <span class="text-success">{{ t('agent.subagentStatus.running') }}: {{ runningCount }}</span>
        <span class="text-warning">{{ t('agent.subagentStatus.queued') }}: {{ queuedCount }}</span>
        <span class="text-base-content/60">{{ t('agent.subagentStatus.completed') }}: {{ completedCount }}</span>
        <button class="btn btn-xs btn-ghost" @click="emit('toggle')">
          {{ isOpen ? t('agent.collapse') : t('agent.expand') }}
        </button>
      </div>
    </div>

    <div v-if="isOpen" class="mt-3">
      <div v-if="subagents.length === 0" class="text-xs text-base-content/60">
        {{ t('agent.noSubagents') }}
      </div>
      <div v-else class="grid grid-cols-1 lg:grid-cols-2 gap-2">
        <div
          v-for="subagent in subagents"
          :key="subagent.id"
          class="border border-base-200 rounded-lg px-3 py-2 bg-base-100 flex flex-col gap-2 hover:border-primary/50 transition-colors"
        >
          <div class="flex items-center justify-between text-sm">
            <div class="flex items-center gap-2">
              <span class="font-mono text-xs text-base-content/50" :title="subagent.id">
                {{ subagent.id.slice(0, 8) }}...
              </span>
              <span class="font-semibold text-primary">
                {{ subagent.role || t('agent.subagentRoles.generic') }}
              </span>
            </div>
            <div class="flex items-center gap-2">
              <span
                class="badge badge-xs"
                :class="statusBadgeClass(subagent.status)"
              >
                {{ t(`agent.subagentStatus.${subagent.status}`) }}
              </span>
              <button
                class="btn btn-xs btn-ghost btn-circle"
                :title="t('agent.viewSubagentDetails')"
                @click="emit('viewDetails', subagent.id)"
              >
                <i class="fas fa-external-link-alt text-xs"></i>
              </button>
            </div>
          </div>
          
          <!-- Task description -->
          <div v-if="subagent.task" class="text-xs text-base-content/80 line-clamp-2">
            <i class="fas fa-tasks mr-1 text-base-content/40"></i>
            {{ subagent.task }}
          </div>
          
          <!-- Progress bar -->
          <div class="flex items-center gap-2">
            <div class="flex-1 h-1.5 bg-base-200 rounded-full overflow-hidden">
              <div
                class="h-full transition-all duration-300"
                :class="subagent.status === 'failed' ? 'bg-error' : 'bg-primary'"
                :style="{ width: `${subagent.progress || 0}%` }"
              ></div>
            </div>
            <span class="text-xs text-base-content/60 w-8 text-right">{{ subagent.progress || 0 }}%</span>
          </div>
          
          <!-- Summary / Output preview -->
          <div v-if="subagent.summary" class="text-xs text-base-content/70 line-clamp-3 bg-base-200/50 rounded px-2 py-1">
            {{ subagent.summary }}
          </div>
          
          <!-- Error message -->
          <div v-if="subagent.status === 'failed' && subagent.error" class="text-xs text-error bg-error/10 rounded px-2 py-1">
            <i class="fas fa-exclamation-circle mr-1"></i>
            {{ subagent.error }}
          </div>
          
          <!-- Tools used -->
          <div v-if="subagent.tools && subagent.tools.length > 0" class="flex flex-wrap gap-1 text-xs">
            <span class="text-base-content/60">{{ t('agent.subagentTools') }}:</span>
            <span
              v-for="tool in subagent.tools.slice(0, 5)"
              :key="tool"
              class="px-2 py-0.5 rounded-md bg-base-200 text-base-content/70"
            >
              {{ tool }}
            </span>
            <span v-if="subagent.tools.length > 5" class="text-base-content/50">
              +{{ subagent.tools.length - 5 }}
            </span>
          </div>
          
          <!-- Timing info -->
          <div class="flex items-center justify-between text-xs text-base-content/40">
            <span v-if="subagent.startedAt">
              <i class="fas fa-clock mr-1"></i>
              {{ formatTime(subagent.startedAt) }}
            </span>
            <span v-if="subagent.duration">
              {{ formatDuration(subagent.duration) }}
            </span>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'

type SubagentStatus = 'running' | 'queued' | 'completed' | 'failed'

interface SubagentItem {
  id: string
  role?: string
  status: SubagentStatus
  progress?: number
  tools?: string[]
  parentId: string
  summary?: string
  task?: string
  error?: string
  startedAt?: number
  duration?: number
}

const props = defineProps<{
  subagents: SubagentItem[]
  isOpen: boolean
}>()

const emit = defineEmits<{
  (e: 'toggle'): void
  (e: 'viewDetails', subagentId: string): void
}>()

const { t } = useI18n()

const totalCount = computed(() => props.subagents.length)
const runningCount = computed(() => props.subagents.filter(s => s.status === 'running').length)
const queuedCount = computed(() => props.subagents.filter(s => s.status === 'queued').length)
const completedCount = computed(() => props.subagents.filter(s => s.status === 'completed').length)

const statusBadgeClass = (status: SubagentStatus) => {
  if (status === 'running') return 'badge-primary'
  if (status === 'queued') return 'badge-warning'
  if (status === 'failed') return 'badge-error'
  return 'badge-ghost'
}

const formatTime = (timestamp: number) => {
  return new Date(timestamp).toLocaleTimeString()
}

const formatDuration = (ms: number) => {
  if (ms < 1000) return `${ms}ms`
  if (ms < 60000) return `${(ms / 1000).toFixed(1)}s`
  return `${(ms / 60000).toFixed(1)}m`
}
</script>
