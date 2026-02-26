<template>
  <div class="h-full flex flex-col">
    <div class="flex items-center justify-between px-4 py-3 border-b border-base-300 bg-base-100/80 backdrop-blur-sm">
      <div class="flex items-center gap-2">
        <i class="fas fa-comments text-info"></i>
        <h2 class="text-sm font-bold text-base-content">Team 会话</h2>
        <span class="badge badge-sm badge-info">{{ sessions.length }}</span>
      </div>
      <div class="flex items-center gap-2">
        <button class="btn btn-xs btn-primary" title="新建 Team 会话" @click="emit('create')">
          <i class="fas fa-plus mr-1"></i>新建
        </button>
        <button class="btn btn-xs btn-ghost" title="刷新" @click="emit('refresh')">
          <i class="fas fa-rotate-right"></i>
        </button>
        <button class="btn btn-xs btn-ghost" title="关闭" @click="emit('close')">
          <i class="fas fa-times"></i>
        </button>
      </div>
    </div>

    <div class="p-3 border-b border-base-300">
      <input
        v-model="searchQuery"
        type="text"
        class="input input-sm input-bordered w-full"
        placeholder="搜索 Team 会话..."
      />
      <div class="flex flex-wrap gap-1.5 mt-2">
        <button
          v-for="opt in filterOptions"
          :key="opt.value"
          class="btn btn-xs"
          :class="statusFilter === opt.value ? 'btn-info' : 'btn-ghost'"
          @click="statusFilter = opt.value"
        >
          {{ opt.label }}
        </button>
      </div>
    </div>

    <div class="flex-1 overflow-y-auto p-3 space-y-2">
      <div v-if="loading" class="text-center py-8 text-base-content/50 text-sm">
        <i class="fas fa-spinner fa-spin mr-2"></i>加载中...
      </div>
      <div v-else-if="filteredSessions.length === 0" class="text-center py-10 text-base-content/40 text-sm">
        {{ searchQuery.trim() ? '没有匹配会话' : '暂无 Team 会话' }}
      </div>
      <button
        v-for="s in filteredSessions"
        :key="s.id"
        class="w-full text-left p-3 rounded-xl border transition-all group"
        :class="currentSessionId === s.id
          ? 'border-info bg-info/8'
          : 'border-base-300 hover:border-info/40 hover:bg-base-50'"
        @click="emit('select', s.id)"
      >
        <div class="flex items-start justify-between gap-2">
          <div class="min-w-0">
            <div class="text-sm font-semibold text-base-content truncate">{{ s.name }}</div>
            <div class="text-xs text-base-content/55 line-clamp-2 mt-0.5">{{ s.goal || '无目标描述' }}</div>
          </div>
          <div class="flex items-center gap-1">
            <button
              class="btn btn-ghost btn-xs opacity-0 group-hover:opacity-100"
              title="重命名"
              @click.stop="emit('rename', s)"
            >
              <i class="fas fa-pen"></i>
            </button>
            <button
              v-if="s.state !== 'ARCHIVED'"
              class="btn btn-ghost btn-xs opacity-0 group-hover:opacity-100"
              title="归档"
              @click.stop="emit('archive', s)"
            >
              <i class="fas fa-box-archive"></i>
            </button>
            <button
              v-if="s.state === 'ARCHIVED'"
              class="btn btn-ghost btn-xs text-info opacity-0 group-hover:opacity-100"
              title="恢复"
              @click.stop="emit('restore', s)"
            >
              <i class="fas fa-rotate-left"></i>
            </button>
            <button
              class="btn btn-ghost btn-xs text-error opacity-0 group-hover:opacity-100"
              title="删除"
              @click.stop="emit('delete', s)"
            >
              <i class="fas fa-trash"></i>
            </button>
            <span class="badge badge-xs" :class="stateBadgeClass(s.state)">{{ stateLabel(s.state) }}</span>
          </div>
        </div>
        <div class="text-xs text-base-content/45 mt-2">
          第 {{ s.current_round }}/{{ s.max_rounds }} 轮 · 更新于 {{ formatDateTime(s.updated_at) }}
        </div>
      </button>
      <div v-if="!loading && filteredSessions.length > 0" class="pt-2 text-center">
        <button
          v-if="hasMore"
          class="btn btn-xs btn-outline"
          :disabled="loadingMore"
          @click="emit('load-more')"
        >
          <i v-if="loadingMore" class="fas fa-spinner fa-spin mr-1"></i>
          {{ loadingMore ? '加载中...' : '加载更多' }}
        </button>
        <span v-else class="text-xs text-base-content/40">已加载全部会话</span>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'
import type { AgentTeamSession } from '@/types/agentTeam'

const props = defineProps<{
  sessions: AgentTeamSession[]
  currentSessionId?: string | null
  loading?: boolean
  hasMore?: boolean
  loadingMore?: boolean
}>()

const emit = defineEmits<{
  (e: 'create'): void
  (e: 'refresh'): void
  (e: 'close'): void
  (e: 'select', sessionId: string): void
  (e: 'rename', session: AgentTeamSession): void
  (e: 'archive', session: AgentTeamSession): void
  (e: 'restore', session: AgentTeamSession): void
  (e: 'delete', session: AgentTeamSession): void
  (e: 'load-more'): void
}>()

const searchQuery = ref('')
const statusFilter = ref<'active' | 'all' | 'running' | 'completed' | 'failed' | 'pending' | 'archived'>('active')

const filterOptions = [
  { value: 'active', label: '活跃' },
  { value: 'archived', label: '已归档' },
  { value: 'all', label: '全部' },
  { value: 'running', label: '运行中' },
  { value: 'completed', label: '已完成' },
  { value: 'failed', label: '失败' },
  { value: 'pending', label: '待启动' },
] as const

const filteredSessions = computed(() => {
  const keyword = searchQuery.value.trim().toLowerCase()
  return props.sessions.filter((s) => {
    const byText = !keyword
      || (s.name || '').toLowerCase().includes(keyword)
      || (s.goal || '').toLowerCase().includes(keyword)
    if (!byText) return false
    if (statusFilter.value === 'active') return s.state !== 'ARCHIVED'
    if (statusFilter.value === 'archived') return s.state === 'ARCHIVED'
    if (statusFilter.value === 'all') return true
    if (statusFilter.value === 'completed') return s.state === 'COMPLETED'
    if (statusFilter.value === 'failed') return s.state === 'FAILED'
    if (statusFilter.value === 'pending') return s.state === 'PENDING'
    if (statusFilter.value === 'running') {
      return ['INITIALIZING', 'PROPOSING', 'CHALLENGING', 'CONVERGENCE_CHECK', 'REVISING', 'DECIDING', 'ARTIFACT_GENERATION'].includes(s.state)
    }
    return true
  })
})

function stateLabel(state: string): string {
  const map: Record<string, string> = {
    PENDING: '待启动',
    INITIALIZING: '初始化',
    PROPOSING: '提案中',
    CHALLENGING: '审查中',
    CONVERGENCE_CHECK: '检验收敛',
    REVISING: '修订中',
    DECIDING: '决策中',
    ARTIFACT_GENERATION: '生成产物',
    COMPLETED: '已完成',
    FAILED: '失败',
    ARCHIVED: '已归档',
    SUSPENDED_FOR_HUMAN: '待人工',
  }
  return map[state] ?? state
}

function stateBadgeClass(state: string): string {
  if (state === 'COMPLETED') return 'badge-success'
  if (state === 'FAILED') return 'badge-error'
  if (state === 'ARCHIVED') return 'badge-neutral'
  if (state === 'SUSPENDED_FOR_HUMAN') return 'badge-warning'
  if (['PROPOSING', 'DECIDING', 'CHALLENGING', 'ARTIFACT_GENERATION'].includes(state)) return 'badge-primary'
  return 'badge-ghost'
}

function formatDateTime(timestamp: string): string {
  try {
    const d = new Date(timestamp)
    return d.toLocaleString('zh-CN', { month: '2-digit', day: '2-digit', hour: '2-digit', minute: '2-digit' })
  } catch {
    return ''
  }
}
</script>
