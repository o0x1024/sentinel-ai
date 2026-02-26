<template>
  <Teleport to="body">
    <div class="fixed inset-0 z-[100] flex items-center justify-center p-4" id="create-team-modal">
      <!-- Backdrop -->
      <div class="absolute inset-0 bg-black/50 backdrop-blur-sm" @click="emit('cancel')"></div>

      <!-- Card -->
      <div class="relative z-10 bg-base-100 rounded-2xl shadow-2xl w-full max-w-md overflow-hidden animate-in zoom-in-95 fade-in duration-200">
        <!-- Header -->
        <div class="px-5 py-4 border-b border-base-300 bg-gradient-to-r from-primary/8 to-transparent">
          <div class="flex items-center gap-3">
            <div class="w-9 h-9 rounded-xl bg-primary/15 flex items-center justify-center">
              <i class="fas fa-users text-primary"></i>
            </div>
            <div>
              <h3 class="font-bold text-base-content text-sm">使用模板创建 Team</h3>
              <p class="text-xs text-base-content/50 mt-0.5">{{ template.name }}</p>
            </div>
          </div>
        </div>

        <!-- Template preview -->
        <div class="px-5 py-3 border-b border-base-200 bg-base-50/50">
          <div class="flex flex-wrap gap-1.5">
            <div
              v-for="(m, i) in template.members"
              :key="m.id"
              class="flex items-center gap-1.5 px-2 py-1 rounded-full text-xs font-medium border"
              :style="{ backgroundColor: roleColors[i % roleColors.length] + '18', borderColor: roleColors[i % roleColors.length] + '40', color: roleColors[i % roleColors.length] }"
            >
              <div class="w-4 h-4 rounded-full flex items-center justify-center text-white text-[10px] font-bold"
                :style="{ backgroundColor: roleColors[i % roleColors.length] }">
                {{ m.name.charAt(0) }}
              </div>
              {{ m.name }}
            </div>
          </div>
        </div>

        <!-- Form -->
        <div class="px-5 py-4 space-y-4">
          <!-- Goal -->
          <div class="space-y-1.5">
            <label class="text-xs font-semibold text-base-content/70">
              团队目标 <span class="text-error">*</span>
            </label>
            <textarea
              v-model="form.goal"
              class="textarea textarea-bordered w-full text-sm resize-none h-24 leading-relaxed focus:border-primary"
              placeholder="详细描述你希望团队分析或完成的具体目标...&#10;例如：为 SaaS 产品设计一个安全可扩展的多租户权限系统"
              id="team-goal-input"
              autofocus
            ></textarea>
            <div class="text-xs text-base-content/30 flex justify-between">
              <span>建议 20-200 字，描述越清晰，团队分析越精准</span>
              <span :class="form.goal.length > 200 ? 'text-warning' : ''">{{ form.goal.length }}</span>
            </div>
          </div>

          <!-- Session name -->
          <div class="space-y-1.5">
            <label class="text-xs font-semibold text-base-content/70">会话名称</label>
            <input
              v-model="form.name"
              type="text"
              class="input input-bordered input-sm w-full focus:border-primary"
              :placeholder="autoName"
              id="team-session-name-input"
            />
          </div>

          <!-- Max rounds -->
          <div class="space-y-1.5">
            <label class="text-xs font-semibold text-base-content/70">会话总轮次（提案+审查+决策）</label>
            <div class="flex items-center gap-3">
              <input
                v-model.number="form.maxRounds"
                type="range"
                min="1"
                max="10"
                class="range range-xs range-primary flex-1"
                id="max-rounds-slider"
              />
              <div class="badge badge-primary badge-sm min-w-[2.5rem] text-center">{{ form.maxRounds }}</div>
            </div>
            <div class="flex justify-between text-xs text-base-content/30">
              <span>1（快速）</span>
              <span>10（深度）</span>
            </div>
          </div>
        </div>

        <!-- Footer -->
        <div class="px-5 py-3.5 border-t border-base-300 bg-base-50/50 flex items-center justify-between gap-3">
          <button class="btn btn-sm btn-ghost text-base-content/60" @click="emit('cancel')">
            取消
          </button>
          <button
            class="btn btn-sm btn-primary gap-2 flex-1 max-w-[160px]"
            :disabled="!form.goal.trim() || isCreating"
            @click="handleCreate"
            id="confirm-create-team-btn"
          >
            <i v-if="isCreating" class="fas fa-spinner fa-spin"></i>
            <i v-else class="fas fa-play"></i>
            {{ isCreating ? '启动中...' : '启动 Team 会话' }}
          </button>
        </div>
      </div>
    </div>
  </Teleport>
</template>

<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import { agentTeamApi } from '@/api/agentTeam'
import type { AgentTeamTemplate } from '@/types/agentTeam'

// ==================== Props / Emits ====================

const props = defineProps<{
  template: AgentTeamTemplate
  conversationId?: string
}>()

const emit = defineEmits<{
  (e: 'created', sessionId: string): void
  (e: 'cancel'): void
}>()

// ==================== State ====================

const isCreating = ref(false)
const DEFAULT_TEMPLATE_ROUNDS = 5
const MIN_TEMPLATE_ROUNDS = 1
const MAX_TEMPLATE_ROUNDS = 10

const form = ref({
  goal: '',
  name: '',
  maxRounds: extractTemplateMaxRounds(props.template.default_rounds_config),
})

const roleColors = ['#6366f1', '#8b5cf6', '#06b6d4', '#10b981', '#f59e0b']

const autoName = computed(() => {
  if (!form.value.goal.trim()) return `Team: ${props.template.name}`
  return `Team: ${form.value.goal.slice(0, 25)}${form.value.goal.length > 25 ? '...' : ''}`
})

// ==================== Actions ====================

async function handleCreate() {
  if (!form.value.goal.trim()) return
  isCreating.value = true
  try {
    const session = await agentTeamApi.createSession({
      name: form.value.name.trim() || autoName.value,
      goal: form.value.goal.trim(),
      template_id: props.template.id,
      conversation_id: props.conversationId,
      max_rounds: form.value.maxRounds,
    })
    // Kick off the run asynchronously
    agentTeamApi.startRun(session.id).catch(e =>
      console.error('[CreateTeamModal] Start run failed:', e)
    )
    emit('created', session.id)
  } catch (e) {
    console.error('[CreateTeamModal] Create failed:', e)
  } finally {
    isCreating.value = false
  }
}

watch(
  () => props.template.id,
  () => {
    form.value.maxRounds = extractTemplateMaxRounds(props.template.default_rounds_config)
  },
)

function normalizeRounds(value: unknown): number {
  const n = Number(value)
  if (!Number.isFinite(n)) return DEFAULT_TEMPLATE_ROUNDS
  const normalized = Math.trunc(n)
  return Math.max(MIN_TEMPLATE_ROUNDS, Math.min(MAX_TEMPLATE_ROUNDS, normalized))
}

function extractTemplateMaxRounds(config: unknown): number {
  if (typeof config === 'number') return normalizeRounds(config)
  if (config && typeof config === 'object') {
    const obj = config as Record<string, unknown>
    const candidate =
      obj.max_rounds ??
      obj.maxRounds ??
      obj.default_rounds ??
      obj.rounds
    return normalizeRounds(candidate)
  }
  return DEFAULT_TEMPLATE_ROUNDS
}
</script>
