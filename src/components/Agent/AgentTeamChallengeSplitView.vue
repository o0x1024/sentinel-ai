<template>
  <div class="challenge-split-view flex flex-col h-full overflow-hidden bg-base-100">
    <!-- Header -->
    <div class="px-4 py-3 border-b border-base-300 flex items-center justify-between bg-gradient-to-r from-secondary/5 to-transparent">
      <div class="flex items-center gap-2">
        <i class="fas fa-columns text-secondary"></i>
        <h3 class="font-bold text-sm text-base-content">Challenge 审查视图</h3>
        <span v-if="divergenceScore !== null" class="badge badge-sm" :class="divergenceBadgeClass">
          分歧度 {{ (divergenceScore! * 100).toFixed(0) }}%
        </span>
      </div>
      <div class="flex items-center gap-2">
        <button
          class="btn btn-xs btn-ghost gap-1"
          :class="viewMode === 'split' ? 'text-primary' : 'text-base-content/50'"
          @click="viewMode = 'split'"
        >
          <i class="fas fa-columns text-xs"></i> 分屏
        </button>
        <button
          class="btn btn-xs btn-ghost gap-1"
          :class="viewMode === 'diff' ? 'text-primary' : 'text-base-content/50'"
          @click="viewMode = 'diff'"
        >
          <i class="fas fa-code-compare text-xs"></i> Diff
        </button>
        <button v-if="showClose" class="btn btn-xs btn-ghost" @click="emit('close')">
          <i class="fas fa-times"></i>
        </button>
      </div>
    </div>

    <!-- Divergence bar -->
    <div v-if="divergenceScore !== null" class="px-4 py-2 border-b border-base-200 bg-base-50/50">
      <div class="flex items-center gap-2">
        <span class="text-xs text-base-content/50 flex-shrink-0">团队分歧度</span>
        <div class="flex-1 h-2 bg-base-300 rounded-full overflow-hidden">
          <div
            class="h-full rounded-full transition-all duration-700"
            :class="divergenceBarClass"
            :style="{ width: `${divergenceScore! * 100}%` }"
          ></div>
        </div>
        <span class="text-xs font-bold flex-shrink-0" :class="divergenceTextClass">
          {{ (divergenceScore! * 100).toFixed(0) }}%
        </span>
        <span class="text-xs text-base-content/40 flex-shrink-0">
          (阈值 {{ (threshold * 100).toFixed(0) }}%)
        </span>
      </div>
    </div>

    <!-- Proposal + Reviews -->
    <div class="flex-1 overflow-hidden flex flex-col min-h-0">
      <!-- Original proposal (collapsed) -->
      <div
        class="border-b border-base-200 bg-base-50/50"
        v-if="proposalMessage"
      >
        <div
          class="px-4 py-2 flex items-center gap-2 cursor-pointer hover:bg-base-100 transition-colors"
          @click="showProposal = !showProposal"
        >
          <div class="w-6 h-6 rounded-full bg-primary/20 flex items-center justify-center flex-shrink-0">
            <span class="text-primary text-xs font-bold">{{ proposalMessage.member_name?.charAt(0) || 'P' }}</span>
          </div>
          <div class="flex-1 min-w-0">
            <span class="text-xs font-medium text-base-content/70">{{ proposalMessage.member_name || '提案角色' }} 的提案</span>
            <span class="text-xs text-base-content/40 ml-2">{{ formatTime(proposalMessage.timestamp) }}</span>
          </div>
          <i class="fas text-xs text-base-content/30" :class="showProposal ? 'fa-chevron-up' : 'fa-chevron-down'"></i>
        </div>
        <div v-if="showProposal" class="px-4 pb-3">
          <div class="text-xs text-base-content/75 leading-relaxed bg-base-100 rounded-lg p-3 border border-base-300 max-h-40 overflow-y-auto">
            <pre class="whitespace-pre-wrap font-sans">{{ proposalMessage.content }}</pre>
          </div>
        </div>
      </div>

      <!-- Split / Diff view -->
      <div class="flex-1 overflow-hidden min-h-0">
        <!-- Split view -->
        <div v-if="viewMode === 'split'" class="flex h-full overflow-hidden">
          <div
            v-for="(review, idx) in reviewMessages"
            :key="review.id"
            class="review-col flex flex-col overflow-hidden border-r border-base-200 last:border-r-0"
            :style="{ width: `${100 / reviewMessages.length}%` }"
          >
            <!-- Reviewer header -->
            <div
              class="px-3 py-2 border-b border-base-200 flex items-center gap-2 flex-shrink-0"
              :style="{ backgroundColor: reviewerColors[idx % reviewerColors.length] + '15' }"
            >
              <div
                class="w-6 h-6 rounded-full flex items-center justify-center text-white text-xs font-bold flex-shrink-0"
                :style="{ backgroundColor: reviewerColors[idx % reviewerColors.length] }"
              >
                {{ review.member_name?.charAt(0) || '?' }}
              </div>
              <div class="min-w-0">
                <div class="text-xs font-semibold text-base-content/80 truncate">{{ review.member_name }}</div>
                <div class="text-[10px] text-base-content/40">{{ formatTime(review.timestamp) }}</div>
              </div>
              <!-- Sentiment indicator -->
              <div class="ml-auto flex-shrink-0">
                <span
                  class="badge badge-xs"
                  :class="sentimentClass(review.content)"
                >{{ sentimentLabel(review.content) }}</span>
              </div>
            </div>

            <!-- Review content -->
            <div class="flex-1 overflow-y-auto p-3">
              <div class="text-xs text-base-content/75 leading-relaxed">
                <pre class="whitespace-pre-wrap font-sans break-words">{{ review.content }}</pre>
              </div>
            </div>
          </div>

          <!-- Empty state -->
          <div v-if="reviewMessages.length === 0" class="flex-1 flex items-center justify-center text-base-content/30 text-sm">
            <i class="fas fa-comment-slash mr-2"></i> 本轮暂无审查意见
          </div>
        </div>

        <!-- Diff view - show commonalities and differences -->
        <div v-else class="flex-1 overflow-y-auto p-4 space-y-3">
          <!-- Agreements -->
          <div v-if="agreements.length > 0">
            <div class="flex items-center gap-2 mb-2">
              <i class="fas fa-check-circle text-success text-sm"></i>
              <span class="text-xs font-bold text-success uppercase tracking-wider">达成共识 ({{ agreements.length }})</span>
            </div>
            <div class="space-y-1.5">
              <div
                v-for="(item, i) in agreements"
                :key="i"
                class="text-xs p-2.5 rounded-lg bg-success/8 border border-success/25 text-base-content/75 leading-relaxed"
              >
                {{ item }}
              </div>
            </div>
          </div>

          <!-- Disagreements -->
          <div v-if="disagreements.length > 0">
            <div class="flex items-center gap-2 mb-2 mt-4">
              <i class="fas fa-exclamation-triangle text-warning text-sm"></i>
              <span class="text-xs font-bold text-warning uppercase tracking-wider">存在分歧 ({{ disagreements.length }})</span>
            </div>
            <div class="space-y-1.5">
              <div
                v-for="(item, i) in disagreements"
                :key="i"
                class="text-xs p-2.5 rounded-lg bg-warning/8 border border-warning/25 text-base-content/75 leading-relaxed"
              >
                {{ item }}
              </div>
            </div>
          </div>

          <!-- All reviews condensed -->
          <div v-if="agreements.length === 0 && disagreements.length === 0 && reviewMessages.length > 0" class="space-y-3">
            <div
              v-for="(review, idx) in reviewMessages"
              :key="review.id"
              class="p-3 rounded-xl border"
              :style="{ borderColor: reviewerColors[idx % reviewerColors.length] + '50' }"
            >
              <div class="flex items-center gap-2 mb-1.5">
                <div
                  class="w-5 h-5 rounded-full flex items-center justify-center text-white text-[10px] font-bold"
                  :style="{ backgroundColor: reviewerColors[idx % reviewerColors.length] }"
                >{{ review.member_name?.charAt(0) }}</div>
                <span class="text-xs font-medium text-base-content/70">{{ review.member_name }}</span>
              </div>
              <p class="text-xs text-base-content/65 leading-relaxed line-clamp-5">{{ review.content }}</p>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import type { AgentTeamMessage } from '@/types/agentTeam'

// ==================== Props / Emits ====================

const props = defineProps<{
  messages: AgentTeamMessage[]
  roundId?: string | null
  divergenceScore?: number | null
  threshold?: number
  showClose?: boolean
}>()

const emit = defineEmits<{
  (e: 'close'): void
}>()

// ==================== State ====================

const viewMode = ref<'split' | 'diff'>('split')
const showProposal = ref(false)

// ==================== Computed ====================

const proposalMessage = computed(() =>
  props.messages.find(m => m.role === 'assistant' && (!props.roundId || m.round_id !== props.roundId))
)

const reviewMessages = computed(() =>
  props.messages.filter(m =>
    m.role === 'assistant' &&
    m.round_id === props.roundId &&
    m.member_id !== proposalMessage.value?.member_id
  )
)

const threshold = computed(() => props.threshold ?? 0.4)

const reviewerColors = ['#6366f1', '#8b5cf6', '#06b6d4', '#10b981', '#f59e0b', '#ef4444']

const divergenceBadgeClass = computed(() => {
  const s = props.divergenceScore ?? 0
  if (s >= 0.6) return 'badge-error'
  if (s >= 0.35) return 'badge-warning'
  return 'badge-success'
})

const divergenceBarClass = computed(() => {
  const s = props.divergenceScore ?? 0
  if (s >= 0.6) return 'bg-gradient-to-r from-warning to-error'
  if (s >= 0.35) return 'bg-warning'
  return 'bg-success'
})

const divergenceTextClass = computed(() => {
  const s = props.divergenceScore ?? 0
  if (s >= 0.6) return 'text-error'
  if (s >= 0.35) return 'text-warning'
  return 'text-success'
})

// Simple heuristic: extract bullet/numbered points from reviews to find agreement/disagreement patterns
const agreements = computed(() => {
  if (reviewMessages.value.length < 2) return []
  // Find sentences containing positive sentiment across all reviews
  const positiveTerms = ['同意', '赞同', '支持', '可行', '合理', 'agree', 'support', 'viable', 'reasonable']
  const result: string[] = []
  for (const msg of reviewMessages.value) {
    const sentences = msg.content.split(/[。！\n.!]+/).filter(s => s.trim().length > 10)
    for (const sentence of sentences.slice(0, 5)) {
      if (positiveTerms.some(t => sentence.toLowerCase().includes(t))) {
        const trimmed = sentence.trim().slice(0, 80)
        if (!result.includes(trimmed)) result.push(trimmed)
      }
    }
  }
  return result.slice(0, 4)
})

const disagreements = computed(() => {
  if (reviewMessages.value.length < 2) return []
  const negativeTerms = ['担忧', '风险', '问题', '不足', '建议修改', '反对', 'concern', 'risk', 'issue', 'lack', 'missing']
  const result: string[] = []
  for (const msg of reviewMessages.value) {
    const sentences = msg.content.split(/[。！\n.!]+/).filter(s => s.trim().length > 10)
    for (const sentence of sentences.slice(0, 5)) {
      if (negativeTerms.some(t => sentence.toLowerCase().includes(t))) {
        const trimmed = sentence.trim().slice(0, 80)
        if (!result.includes(trimmed)) result.push(trimmed)
      }
    }
  }
  return result.slice(0, 4)
})

// ==================== Sentiment helpers ====================

function sentimentClass(content: string): string {
  const lower = content.toLowerCase()
  const pos = ['同意', '支持', '可行', 'agree', 'support'].filter(t => lower.includes(t)).length
  const neg = ['反对', '问题', '风险', '不可行', 'concern', 'risk'].filter(t => lower.includes(t)).length
  if (pos > neg) return 'badge-success'
  if (neg > pos) return 'badge-warning'
  return 'badge-ghost'
}

function sentimentLabel(content: string): string {
  const lower = content.toLowerCase()
  const pos = ['同意', '支持', '可行', 'agree', 'support'].filter(t => lower.includes(t)).length
  const neg = ['反对', '问题', '风险', '不可行', 'concern', 'risk'].filter(t => lower.includes(t)).length
  if (pos > neg) return '支持'
  if (neg > pos) return '保留'
  return '中立'
}

function formatTime(ts: string): string {
  try {
    return new Date(ts).toLocaleTimeString('zh-CN', { hour: '2-digit', minute: '2-digit' })
  } catch {
    return ''
  }
}
</script>

<style scoped>
.review-col:last-child {
  border-right: none;
}
</style>
