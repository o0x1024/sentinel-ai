<template>
  <Teleport to="body">
    <div
      v-if="show"
      class="fixed inset-0 z-[200] flex items-center justify-center p-4"
      @click.self="closeModal"
    >
      <!-- Backdrop -->
      <div class="absolute inset-0 bg-black/60 backdrop-blur-sm" @click="closeModal" />

      <!-- Modal -->
      <div
        class="relative w-full max-w-2xl max-h-[90vh] overflow-hidden flex flex-col rounded-2xl shadow-2xl border border-base-300 bg-base-100 text-base-content"
      >
        <!-- Header -->
        <div class="relative px-6 py-5 border-b border-base-300 overflow-hidden bg-gradient-to-r from-primary/10 to-secondary/5">
          <div class="relative flex items-center gap-3">
            <div class="w-10 h-10 rounded-xl flex items-center justify-center bg-primary/15 text-primary">
              <i class="fas fa-wand-magic-sparkles text-lg"></i>
            </div>
            <div>
              <h2 class="text-lg font-bold">AI 生成模板</h2>
              <p class="text-xs text-base-content/60">描述你的场景，AI 自动设计专属的多 Agent 协作团队</p>
            </div>
            <button
              class="ml-auto btn btn-ghost btn-xs text-base-content/50 hover:text-base-content"
              @click="closeModal"
            >
              <i class="fas fa-times text-sm"></i>
            </button>
          </div>
        </div>

        <!-- Content -->
        <div class="flex-1 overflow-y-auto">
          <!-- Step 1: Describe -->
          <div v-if="step === 'input'" class="p-6 space-y-5">
            <!-- Description textarea -->
            <div>
              <label class="block text-sm font-medium text-base-content/80 mb-2">
                场景描述 <span class="text-error">*</span>
              </label>
              <textarea
                id="ai-template-description"
                v-model="description"
                rows="5"
                placeholder="例如：我需要一个安全代码审计团队，包含漏洞挖掘、业务逻辑分析和合规审查三个视角，专注于 Web 应用和 API 安全..."
                class="textarea textarea-bordered w-full rounded-xl px-4 py-3 text-sm resize-none outline-none transition-all"
                :class="description.length > 0 ? 'border-primary/50' : 'border-base-300'"
                :disabled="isGenerating"
                @input="onDescriptionInput"
              ></textarea>
              <div class="flex justify-between mt-1.5 text-xs text-base-content/50">
                <span>描述越详细，生成质量越高</span>
                <span :class="description.length > 500 ? 'text-warning' : ''">{{ description.length }}/500</span>
              </div>
            </div>

            <!-- Options row -->
            <div class="grid grid-cols-2 gap-4">
              <!-- Domain -->
              <div>
                <label class="block text-xs font-medium text-base-content/70 mb-1.5">领域</label>
                <select
                  id="ai-template-domain"
                  v-model="domain"
                  class="select select-bordered w-full rounded-lg text-sm"
                  :disabled="isGenerating"
                >
                  <option value="">自动推断</option>
                  <option value="product">产品研发</option>
                  <option value="security">安全运营</option>
                  <option value="audit">代码审计</option>
                  <option value="redblue">红蓝对抗</option>
                  <option value="ops">运维变更</option>
                  <option value="custom">自定义</option>
                </select>
              </div>

              <!-- Agent count -->
              <div>
                <label class="block text-xs font-medium text-base-content/70 mb-1.5">
                  Agent 数量：<span class="text-primary font-bold">{{ roleCount }}</span>
                </label>
                <div class="flex items-center gap-2">
                  <span class="text-xs text-base-content/45">2</span>
                  <input
                    id="ai-template-role-count"
                    v-model.number="roleCount"
                    type="range"
                    min="2"
                    max="6"
                    step="1"
                    class="flex-1 range range-xs range-primary"
                    :disabled="isGenerating"
                  />
                  <span class="text-xs text-base-content/45">6</span>
                </div>
                <div class="mt-1 text-xs text-base-content/50">建议 3-4 个 Agent 获得最佳讨论质量</div>
              </div>
            </div>

            <!-- Quick examples -->
            <div>
              <div class="text-xs text-base-content/60 mb-2">快速示例</div>
              <div class="flex flex-wrap gap-2">
                <button
                  v-for="eg in quickExamples"
                  :key="eg.label"
                  class="px-2.5 py-1 rounded-lg text-xs border border-base-300 text-base-content/70 transition-all hover:border-primary/40 hover:text-primary hover:bg-primary/5"
                  :disabled="isGenerating"
                  @click="applyExample(eg)"
                >
                  {{ eg.label }}
                </button>
              </div>
            </div>
          </div>

          <!-- Step 2: Generating -->
          <div v-else-if="step === 'generating'" class="p-6 flex flex-col items-center justify-center gap-6 py-12">
            <!-- Animated spinner -->
            <div class="relative w-20 h-20 rounded-full border-2 border-base-300 border-t-primary animate-spin flex items-center justify-center">
              <div class="absolute inset-3 rounded-full flex items-center justify-center bg-base-100">
                <i class="fas fa-brain text-2xl text-primary"></i>
              </div>
            </div>
            <div class="text-center">
              <div class="text-base font-semibold text-base-content/85 mb-1">AI 正在设计团队架构...</div>
              <div class="text-sm text-base-content/60">{{ generatingHint }}</div>
            </div>
            <!-- Progress dots -->
            <div class="flex gap-1.5">
              <div v-for="i in 4" :key="i" class="w-2 h-2 rounded-full animate-pulse bg-primary"
                :style="{ animationDelay: `${(i - 1) * 0.2}s` }"></div>
            </div>
          </div>

          <!-- Step 3: Preview result -->
          <div v-else-if="step === 'preview' && generated" class="p-6 space-y-5">
            <!-- Template info -->
            <div class="rounded-xl p-4 border border-base-300 bg-base-200/40">
              <div class="flex items-start gap-3">
                <div class="w-8 h-8 rounded-lg flex items-center justify-center flex-shrink-0 bg-primary/15 text-primary">
                  <i class="fas fa-users text-sm"></i>
                </div>
                <div class="min-w-0">
                  <div class="text-base font-bold leading-tight flex items-center gap-2 flex-wrap">
                    <span id="generated-template-name">{{ generated.name }}</span>
                    <span class="badge badge-sm badge-primary">{{ domainLabel(generated.domain) }}</span>
                  </div>
                  <div class="text-sm text-base-content/70 mt-1">{{ generated.description }}</div>
                </div>
              </div>
            </div>

            <!-- Agents -->
            <div>
              <div class="text-xs font-semibold text-base-content/60 uppercase tracking-wider mb-3 flex items-center gap-2">
                <i class="fas fa-user-group"></i>
                Agent 设计 ({{ generated.agents.length }})
              </div>
              <div class="space-y-3">
                <div
                  v-for="(agent, i) in generated.agents"
                  :key="i"
                  class="rounded-xl border overflow-hidden transition-all"
                  :class="expandedMember === i ? 'bg-base-200/60 border-primary/30' : 'bg-base-100 border-base-300 hover:bg-base-200/30'"
                >
                  <!-- Agent header -->
                  <div
                    class="flex items-center gap-3 px-4 py-3 cursor-pointer"
                    @click="expandedMember = expandedMember === i ? null : i"
                  >
                    <div class="w-7 h-7 rounded-lg flex items-center justify-center text-xs font-bold flex-shrink-0"
                      :style="{ background: memberColors[i % memberColors.length] }">
                      {{ i + 1 }}
                    </div>
                    <div class="flex-1 min-w-0">
                      <div class="text-sm font-semibold text-base-content/90">{{ agent.name }}</div>
                      <div class="text-xs text-base-content/55 truncate">{{ agent.responsibility }}</div>
                    </div>
                    <div class="flex items-center gap-1.5 flex-shrink-0">
                      <span class="badge badge-xs badge-ghost">
                        {{ styleLabel(agent.decision_style) }}
                      </span>
                      <i class="fas text-base-content/40 text-xs transition-transform"
                        :class="expandedMember === i ? 'fa-chevron-up' : 'fa-chevron-down'"></i>
                    </div>
                  </div>

                  <!-- Agent detail -->
                  <div v-if="expandedMember === i" class="px-4 pb-4 border-t border-base-300">
                    <div class="mt-3 text-xs text-base-content/70 leading-relaxed whitespace-pre-wrap bg-base-200/60 rounded-lg p-3 font-mono">{{ agent.system_prompt }}</div>
                    <div class="mt-2 flex items-center gap-3 text-xs text-base-content/55">
                      <span>风险偏好: <span class="text-base-content/75">{{ riskLabel(agent.risk_preference) }}</span></span>
                      <span>权重: <span class="text-base-content/75">{{ agent.weight }}</span></span>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </div>

          <!-- Error state -->
          <div v-else-if="step === 'error'" class="p-6 flex flex-col items-center gap-4 py-10">
            <div class="w-14 h-14 rounded-2xl flex items-center justify-center bg-error/15">
              <i class="fas fa-triangle-exclamation text-2xl text-error"></i>
            </div>
            <div class="text-center">
              <div class="font-semibold text-base-content/90 mb-1">生成失败</div>
              <div class="text-sm text-base-content/65">{{ errorMsg }}</div>
            </div>
            <button class="btn btn-sm btn-error btn-outline" @click="step = 'input'">重新描述</button>
          </div>
        </div>

        <!-- Footer -->
        <div class="px-6 py-4 border-t border-base-300 bg-base-50/50 flex items-center gap-3">
          <!-- Back -->
          <button
            v-if="step === 'preview' || step === 'error'"
            class="btn btn-sm btn-ghost text-base-content/60 gap-2"
            @click="step = 'input'"
          >
            <i class="fas fa-arrow-left text-xs"></i>
            重新生成
          </button>

          <div class="flex-1" />

          <!-- Cancel -->
          <button class="btn btn-sm btn-ghost text-base-content/55" @click="closeModal">取消</button>

          <!-- Generate -->
          <button
            v-if="step === 'input'"
            class="btn btn-sm btn-primary gap-2 font-semibold"
            :disabled="!description.trim() || isGenerating"
            @click="generate"
            id="ai-generate-template-btn"
          >
            <i class="fas fa-wand-magic-sparkles text-xs"></i>
            AI 生成
          </button>

          <!-- Save -->
          <button
            v-if="step === 'preview'"
            class="btn btn-sm btn-success gap-2 font-semibold"
            :disabled="isSaving"
            @click="saveTemplate"
            id="ai-save-template-btn"
          >
            <i v-if="isSaving" class="fas fa-spinner animate-spin text-xs"></i>
            <i v-else class="fas fa-floppy-disk text-xs"></i>
            {{ isSaving ? '保存中...' : '保存模板' }}
          </button>
        </div>
      </div>
    </div>
  </Teleport>
</template>

<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'

// ==================== Types ====================

interface GeneratedAgent {
  name: string
  responsibility: string
  system_prompt: string
  decision_style: string
  risk_preference: string
  weight: number
}

interface GeneratedTemplate {
  name: string
  description: string
  domain: string
  agents: GeneratedAgent[]
  raw_json: string
}

// ==================== Props / Emits ====================

const props = defineProps<{
  show: boolean
}>()

const emit = defineEmits<{
  (e: 'close'): void
  (e: 'saved', template: any): void
}>()

// ==================== State ====================

const step = ref<'input' | 'generating' | 'preview' | 'error'>('input')
const description = ref('')
const domain = ref('')
const roleCount = ref(3)
const generated = ref<GeneratedTemplate | null>(null)
const isGenerating = ref(false)
const isSaving = ref(false)
const errorMsg = ref('')
const expandedMember = ref<number | null>(0)

const generatingHints = [
  '正在分析场景特征...',
  '正在设计 Agent 职责分工...',
  '正在编写专属 System Prompt...',
  '正在优化 Agent 互补性...',
  '即将完成，稍等片刻...',
]
const generatingHint = ref(generatingHints[0])
let hintInterval: ReturnType<typeof setInterval> | null = null

const memberColors = [
  'linear-gradient(135deg, hsl(271 70% 55%), hsl(290 60% 50%))',
  'linear-gradient(135deg, hsl(210 80% 50%), hsl(225 70% 55%))',
  'linear-gradient(135deg, hsl(142 65% 40%), hsl(160 60% 45%))',
  'linear-gradient(135deg, hsl(38 90% 50%), hsl(25 85% 55%))',
  'linear-gradient(135deg, hsl(0 70% 55%), hsl(15 75% 50%))',
  'linear-gradient(135deg, hsl(180 60% 40%), hsl(195 70% 45%))',
]

// ==================== Quick examples ====================

const quickExamples = [
  {
    label: '🔍 代码安全审计',
    description: '我需要一个专注于 Web 应用安全审计的团队，涵盖 SQL 注入、XSS、逻辑漏洞等维度，同时考虑业务影响和修复可行性。',
    domain: 'audit',
    roleCount: 3,
  },
  {
    label: '🚨 应急响应',
    description: '设计一个安全事件应急响应团队，能快速进行初步研判、攻击链分析、取证溯源，并输出合规通报和整改建议。',
    domain: 'security',
    roleCount: 3,
  },
  {
    label: '🏗️ 微服务架构评审',
    description: '我需要一个技术评审团队，对微服务架构方案进行多维度评估，包括系统设计合理性、性能扩展性、运维复杂度和技术债务。',
    domain: 'product',
    roleCount: 4,
  },
  {
    label: '📋 合规风险评估',
    description: '针对 GDPR/等保三级合规场景，设计一个评审团队，包含法律合规、技术实现、风险评估和业务影响四个维度。',
    domain: 'audit',
    roleCount: 4,
  },
  {
    label: '⚔️ 红蓝演练复盘',
    description: '红蓝对抗演练结束后，设计一个复盘团队，红队汇报攻击路径和成果，蓝队分析检测响应有效性，管理层提取经验教训。',
    domain: 'redblue',
    roleCount: 3,
  },
]

// ==================== Methods ====================

function onDescriptionInput() {
  if (description.value.length > 500) {
    description.value = description.value.slice(0, 500)
  }
}

function applyExample(eg: typeof quickExamples[0]) {
  description.value = eg.description
  domain.value = eg.domain
  roleCount.value = eg.roleCount
}

async function generate() {
  if (!description.value.trim()) return
  step.value = 'generating'
  isGenerating.value = true
  errorMsg.value = ''

  // Rotate hints
  let hintIdx = 0
  hintInterval = setInterval(() => {
    hintIdx = (hintIdx + 1) % generatingHints.length
    generatingHint.value = generatingHints[hintIdx]
  }, 1800)

  try {
    const result = await invoke<GeneratedTemplate>('agent_team_generate_template', {
      request: {
        description: description.value.trim(),
        domain: domain.value || null,
        agent_count: roleCount.value,
      },
    })
    generated.value = result
    expandedMember.value = 0
    step.value = 'preview'
  } catch (e: any) {
    errorMsg.value = e?.toString() ?? '未知错误'
    step.value = 'error'
  } finally {
    isGenerating.value = false
    if (hintInterval) {
      clearInterval(hintInterval)
      hintInterval = null
    }
  }
}

async function saveTemplate() {
  if (!generated.value) return
  isSaving.value = true
  try {
    const saved = await invoke<any>('agent_team_save_generated_template', {
      generated: generated.value,
    })
    emit('saved', saved)
    closeModal()
  } catch (e: any) {
    errorMsg.value = e?.toString() ?? '保存失败'
    step.value = 'error'
  } finally {
    isSaving.value = false
  }
}

function closeModal() {
  if (isGenerating.value) return
  emit('close')
  // Reset after close animation
  setTimeout(() => {
    step.value = 'input'
    description.value = ''
    domain.value = ''
    roleCount.value = 3
    generated.value = null
    errorMsg.value = ''
    expandedMember.value = null
  }, 200)
}

// ==================== Display helpers ====================

function domainLabel(d: string): string {
  const map: Record<string, string> = {
    product: '产品研发', security: '安全运营', audit: '代码审计',
    redblue: '红蓝对抗', ops: '运维变更', custom: '自定义',
  }
  return map[d] ?? d
}

function styleLabel(s: string): string {
  const map: Record<string, string> = {
    conservative: '保守', balanced: '平衡', aggressive: '激进',
    pragmatic: '务实', risk_aware: '风险意识',
  }
  return map[s] ?? s
}

function riskLabel(r: string): string {
  const map: Record<string, string> = { low: '低', medium: '中', high: '高' }
  return map[r] ?? r
}

// Reset when modal opens
watch(() => props.show, (v) => {
  if (v) {
    step.value = 'input'
    generated.value = null
    errorMsg.value = ''
  }
})
</script>

<style scoped></style>
