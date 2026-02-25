<template>
  <div class="agent-team-timeline flex flex-col h-full overflow-hidden bg-base-100">
    <!-- Header -->
    <div class="px-4 py-3 border-b border-base-300 flex items-center justify-between">
      <div class="flex items-center gap-2">
        <i class="fas fa-code-branch text-primary"></i>
        <h3 class="font-bold text-sm text-base-content">Discussion Timeline</h3>
      </div>
      <button class="btn btn-xs btn-ghost" @click="emit('close')" v-if="showClose">
        <i class="fas fa-times"></i>
      </button>
    </div>

    <!-- Timeline content -->
    <div class="flex-1 overflow-y-auto px-4 py-3 space-y-2">
      <div v-if="rounds.length === 0" class="text-center py-8 text-base-content/40 text-sm">
        <i class="fas fa-hourglass-start text-2xl mb-2 block"></i>
        会话尚未开始
      </div>

      <!-- Round groups -->
      <div
        v-for="(round, rIdx) in rounds"
        :key="round.id"
        class="round-group"
      >
        <!-- Round header -->
        <div
          class="flex items-center gap-2 py-1.5 px-2 rounded-lg cursor-pointer hover:bg-base-200/50 transition-colors"
          @click="toggleRound(round.id)"
        >
          <!-- Phase color dot -->
          <div
            class="w-2.5 h-2.5 rounded-full flex-shrink-0"
            :class="phaseColor(round.phase)"
          ></div>
          <!-- Connector line -->
          <div v-if="rIdx < rounds.length - 1" class="absolute left-[18px] mt-3 w-px h-full bg-base-300"></div>

          <div class="flex-1 min-w-0">
            <div class="flex items-center gap-2">
              <span class="text-xs font-semibold text-base-content/80">
                第 {{ round.round_number }} 轮 · {{ phaseLabel(round.phase) }}
              </span>
              <span class="badge badge-xs" :class="roundStatusClass(round.status)">
                {{ roundStatusLabel(round.status) }}
              </span>
            </div>
            <div class="flex items-center gap-3 text-xs text-base-content/40 mt-0.5">
              <span v-if="round.started_at">{{ formatTime(round.started_at) }}</span>
              <span v-if="round.divergence_score !== null && round.divergence_score !== undefined">
                分歧度:
                <span :class="divergenceColor(round.divergence_score)">
                  {{ (round.divergence_score * 100).toFixed(0) }}%
                </span>
              </span>
            </div>
          </div>
          <i
            class="fas text-xs text-base-content/30 transition-transform flex-shrink-0"
            :class="expandedRounds.has(round.id) ? 'fa-chevron-up' : 'fa-chevron-down'"
          ></i>
        </div>

        <!-- Messages in this round (collapsed by default for older rounds) -->
        <div v-if="expandedRounds.has(round.id)" class="pl-5 space-y-1.5 mt-1">
          <div
            v-for="msg in messagesForRound(round.id)"
            :key="msg.id"
            class="flex gap-2"
          >
            <!-- Role avatar -->
            <div
              class="w-5 h-5 rounded-full flex items-center justify-center text-[9px] font-bold flex-shrink-0 mt-0.5"
              :class="roleAvatarClass(msg.member_name)"
            >
              {{ msg.member_name?.charAt(0) || 'S' }}
            </div>
            <div class="flex-1 min-w-0">
              <div class="flex items-center gap-1.5 mb-0.5">
                <span class="text-xs font-medium text-base-content/70">{{ msg.member_name || '系统' }}</span>
                <span class="text-xs text-base-content/30">{{ formatTime(msg.timestamp) }}</span>
                <span v-if="msg.token_count" class="text-xs text-base-content/20">{{ msg.token_count }}t</span>
              </div>
              <div
                class="text-xs text-base-content/65 leading-relaxed bg-base-200/40 rounded-lg px-2.5 py-1.5 cursor-pointer hover:bg-base-200/70 transition-colors"
                :class="expandedMsg === msg.id ? 'line-clamp-none' : 'line-clamp-3'"
                @click="expandedMsg = expandedMsg === msg.id ? null : msg.id"
              >
                {{ msg.content }}
              </div>
            </div>
          </div>

          <!-- Empty round -->
          <div v-if="messagesForRound(round.id).length === 0" class="text-xs text-base-content/30 pl-1 py-1">
            暂无消息
          </div>
        </div>

        <!-- Divergence bar -->
        <div
          v-if="round.divergence_score !== null && round.divergence_score !== undefined"
          class="pl-5 mt-1"
        >
          <div class="flex items-center gap-2">
            <div class="flex-1 h-1 bg-base-300 rounded-full overflow-hidden">
              <div
                class="h-full rounded-full transition-all duration-500"
                :class="divergenceBarClass(round.divergence_score)"
                :style="{ width: `${round.divergence_score * 100}%` }"
              ></div>
            </div>
            <span class="text-[10px] text-base-content/40" :class="divergenceColor(round.divergence_score)">
              {{ divergenceLabel(round.divergence_score) }}
            </span>
          </div>
        </div>
      </div>

      <!-- Current activity -->
      <div v-if="isRunning && currentMemberName" class="flex items-center gap-2 py-2 pl-2">
        <div class="w-2.5 h-2.5 rounded-full bg-primary animate-pulse flex-shrink-0"></div>
        <div class="flex items-center gap-1.5">
          <div
            class="w-5 h-5 rounded-full bg-primary/20 flex items-center justify-center text-[9px] font-bold text-primary"
          >
            {{ currentMemberName.charAt(0) }}
          </div>
          <span class="text-xs text-primary/80 font-medium">{{ currentMemberName }}</span>
          <span class="text-xs text-base-content/40">正在思考...</span>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import type { AgentTeamMessage } from '@/types/agentTeam'

// ==================== Types ====================

interface RoundSummary {
  id: string
  round_number: number
  phase: string
  status: string
  divergence_score?: number | null
  started_at?: string | null
  completed_at?: string | null
}

// ==================== Props / Emits ====================

const props = defineProps<{
  rounds: RoundSummary[]
  messages: AgentTeamMessage[]
  isRunning?: boolean
  currentMemberName?: string | null
  showClose?: boolean
}>()

const emit = defineEmits<{
  (e: 'close'): void
}>()

// ==================== State ====================

const expandedRounds = ref<Set<string>>(new Set())
const expandedMsg = ref<string | null>(null)

// Auto-expand the latest round
onMounted(() => {
  if (props.rounds.length > 0) {
    expandedRounds.value.add(props.rounds[props.rounds.length - 1].id)
  }
})

// ==================== Helpers ====================

function toggleRound(id: string) {
  if (expandedRounds.value.has(id)) {
    expandedRounds.value.delete(id)
  } else {
    expandedRounds.value.add(id)
  }
}

function messagesForRound(roundId: string): AgentTeamMessage[] {
  return props.messages.filter(m => m.round_id === roundId)
}

function phaseColor(phase: string): string {
  const map: Record<string, string> = {
    proposing: 'bg-primary',
    challenging: 'bg-secondary',
    convergence_check: 'bg-info',
    revising: 'bg-warning',
    deciding: 'bg-accent',
    artifact_generation: 'bg-success',
  }
  return map[phase] ?? 'bg-base-300'
}

function phaseLabel(phase: string): string {
  const map: Record<string, string> = {
    proposing: '提案',
    challenging: '审查',
    convergence_check: '收敛检验',
    revising: '修订',
    deciding: '决策',
    artifact_generation: '产物生成',
  }
  return map[phase] ?? phase
}

function roundStatusClass(status: string): string {
  if (status === 'completed') return 'badge-success'
  if (status === 'running') return 'badge-primary badge-outline'
  if (status === 'failed') return 'badge-error'
  return 'badge-ghost'
}

function roundStatusLabel(status: string): string {
  if (status === 'completed') return '已完成'
  if (status === 'running') return '进行中'
  if (status === 'failed') return '失败'
  return '待开始'
}

function divergenceColor(score: number): string {
  if (score >= 0.6) return 'text-error font-medium'
  if (score >= 0.35) return 'text-warning  font-medium'
  return 'text-success font-medium'
}

function divergenceBarClass(score: number): string {
  if (score >= 0.6) return 'bg-error'
  if (score >= 0.35) return 'bg-warning'
  return 'bg-success'
}

function divergenceLabel(score: number): string {
  if (score >= 0.6) return '高分歧'
  if (score >= 0.35) return '中分歧'
  return '低分歧'
}

const AVATAR_COLORS = [
  'bg-primary/20 text-primary',
  'bg-secondary/20 text-secondary',
  'bg-accent/20 text-accent',
  'bg-success/20 text-success',
  'bg-warning/20 text-warning',
  'bg-info/20 text-info',
]

function roleAvatarClass(name?: string): string {
  if (!name) return 'bg-base-300 text-base-content/50'
  const idx = name.charCodeAt(0) % AVATAR_COLORS.length
  return AVATAR_COLORS[idx]
}

function formatTime(ts?: string | null): string {
  if (!ts) return ''
  try {
    return new Date(ts).toLocaleTimeString('zh-CN', { hour: '2-digit', minute: '2-digit', second: '2-digit' })
  } catch {
    return ''
  }
}
</script>

<style scoped>
.round-group {
  position: relative;
}
</style>
