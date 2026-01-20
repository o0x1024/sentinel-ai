<template>
  <div v-if="subagents.length > 0" class="subagent-panel ">
    <!-- Header -->
    <div class="flex items-center justify-between px-2 py-1.5">
      <div class="flex items-center gap-2">
        <i class="fas fa-robot text-primary text-sm"></i>
        <span class="font-semibold text-sm">{{ t('agent.subagents') }}</span>
        <span class="badge badge-sm badge-primary">{{ subagents.length }}</span>
      </div>
      <div class="flex items-center gap-3 text-xs">
        <span v-if="runningCount > 0" class="flex items-center gap-1 text-primary">
          <span class="w-2 h-2 rounded-full bg-primary animate-pulse"></span>
          {{ runningCount }} {{ t('agent.subagentStatus.running') }}
        </span>
        <span v-if="queuedCount > 0" class="text-warning">
          {{ queuedCount }} {{ t('agent.subagentStatus.queued') }}
        </span>
        <button 
          class="btn btn-xs btn-ghost gap-1" 
          @click="emit('toggle')"
        >
          <i class="fas" :class="isOpen ? 'fa-chevron-up' : 'fa-chevron-down'"></i>
        </button>
      </div>
    </div>

    <!-- Cards Grid -->
    <Transition name="slide-fade">
      <div v-if="isOpen" class="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 gap-2 px-2 pb-2">
        <div
          v-for="subagent in subagents"
          :key="subagent.id"
          class="subagent-card group relative bg-base-100 border border-base-300 rounded-lg p-3 cursor-pointer transition-all hover:shadow-md hover:border-primary/50"
          :class="getCardClass(subagent.status)"
          @click="emit('viewDetails', subagent.id)"
        >
          <!-- Status indicator -->
          <div class="absolute top-2 right-2">
            <div 
              class="w-2.5 h-2.5 rounded-full"
              :class="statusDotClass(subagent.status)"
            ></div>
          </div>

          <!-- Role/Title -->
          <div class="flex items-center gap-2 mb-2">
            <div 
              class="w-8 h-8 rounded-lg flex items-center justify-center text-xs"
              :class="statusBgClass(subagent.status)"
            >
              <i class="fas" :class="statusIconClass(subagent.status)"></i>
            </div>
            <div class="min-w-0 flex-1">
              <div class="text-sm font-medium truncate">
                {{ subagent.role || t('agent.subagentRoles.generic') }}
              </div>
              <div class="text-xs text-base-content/50 font-mono truncate">
                {{ subagent.id.slice(0, 8) }}
              </div>
            </div>
          </div>

          <!-- Task preview -->
          <div 
            v-if="subagent.task" 
            class="text-xs text-base-content/70 line-clamp-2 mb-2 min-h-[2.5rem]"
            :title="subagent.task"
          >
            {{ truncateTask(subagent.task) }}
          </div>
          <div v-else class="text-xs text-base-content/40 italic mb-2 min-h-[2.5rem]">
            {{ t('agent.subagentDetail.noTask') }}
          </div>

          <!-- Progress bar (for running) -->
          <div v-if="subagent.status === 'running'" class="mb-2">
            <div class="flex items-center justify-between text-xs text-base-content/50 mb-1">
              <span>{{ t('agent.subagentDetail.progress') }}</span>
              <span>{{ subagent.progress || 0 }}%</span>
            </div>
            <div class="h-1.5 bg-base-200 rounded-full overflow-hidden">
              <div 
                class="h-full bg-primary transition-all duration-300"
                :style="{ width: `${subagent.progress || 0}%` }"
              ></div>
            </div>
          </div>

          <!-- Footer info -->
          <div class="flex items-center justify-between text-xs">
            <span 
              class="badge badge-xs"
              :class="statusBadgeClass(subagent.status)"
            >
              {{ t(`agent.subagentStatus.${subagent.status}`) }}
            </span>
            <span v-if="subagent.duration" class="text-base-content/40">
              {{ formatDuration(subagent.duration) }}
            </span>
            <span v-else-if="subagent.startedAt" class="text-base-content/40">
              {{ formatRelativeTime(subagent.startedAt) }}
            </span>
          </div>

          <!-- Error indicator -->
          <div 
            v-if="subagent.status === 'failed' && subagent.error" 
            class="mt-2 text-xs text-error bg-error/10 rounded px-2 py-1 line-clamp-1"
            :title="subagent.error"
          >
            <i class="fas fa-exclamation-circle mr-1"></i>
            {{ subagent.error }}
          </div>

          <!-- Hover overlay -->
          <div class="absolute inset-0 bg-primary/5 rounded-lg opacity-0 group-hover:opacity-100 transition-opacity pointer-events-none"></div>
        </div>
      </div>
    </Transition>
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

const runningCount = computed(() => props.subagents.filter(s => s.status === 'running').length)
const queuedCount = computed(() => props.subagents.filter(s => s.status === 'queued').length)

const getCardClass = (status: SubagentStatus) => {
  if (status === 'running') return 'border-primary/30 bg-primary/5'
  if (status === 'failed') return 'border-error/30 bg-error/5'
  return ''
}

const statusBadgeClass = (status: SubagentStatus) => {
  if (status === 'running') return 'badge-primary'
  if (status === 'queued') return 'badge-warning'
  if (status === 'failed') return 'badge-error'
  return 'badge-success'
}

const statusDotClass = (status: SubagentStatus) => {
  if (status === 'running') return 'bg-primary animate-pulse'
  if (status === 'queued') return 'bg-warning'
  if (status === 'failed') return 'bg-error'
  return 'bg-success'
}

const statusBgClass = (status: SubagentStatus) => {
  if (status === 'running') return 'bg-primary/20 text-primary'
  if (status === 'queued') return 'bg-warning/20 text-warning'
  if (status === 'failed') return 'bg-error/20 text-error'
  return 'bg-success/20 text-success'
}

const statusIconClass = (status: SubagentStatus) => {
  if (status === 'running') return 'fa-spinner fa-spin'
  if (status === 'queued') return 'fa-clock'
  if (status === 'failed') return 'fa-times'
  return 'fa-check'
}

const truncateTask = (task: string) => {
  const cleaned = task.replace(/\s+/g, ' ').trim()
  const match = cleaned.match(/Subagent task:\s*(.+)/i)
  const taskPart = match ? match[1] : cleaned
  return taskPart.length > 80 ? taskPart.slice(0, 80) + '...' : taskPart
}

const formatDuration = (ms: number) => {
  if (ms < 1000) return `${ms}ms`
  if (ms < 60000) return `${(ms / 1000).toFixed(1)}s`
  const minutes = Math.floor(ms / 60000)
  const seconds = Math.round((ms % 60000) / 1000)
  return `${minutes}m ${seconds}s`
}

const formatRelativeTime = (timestamp: number) => {
  const now = Date.now()
  const diff = now - timestamp
  if (diff < 60000) return t('agent.justNow')
  if (diff < 3600000) return `${Math.floor(diff / 60000)}m`
  return `${Math.floor(diff / 3600000)}h`
}
</script>

<style scoped>
.slide-fade-enter-active,
.slide-fade-leave-active {
  transition: all 0.2s ease;
}

.slide-fade-enter-from,
.slide-fade-leave-to {
  opacity: 0;
  transform: translateY(-10px);
}

.subagent-card {
  min-height: 120px;
}
</style>
