<template>
  <div class="agent-team-settings flex flex-col h-full overflow-hidden max-h-[85vh]">
    <!-- Header -->
    <div class="px-5 py-4 border-b border-base-300 flex items-center justify-between bg-gradient-to-r from-secondary/8 to-transparent">
      <div class="flex items-center gap-2.5">
        <div class="w-8 h-8 rounded-lg bg-secondary/15 flex items-center justify-center">
          <i class="fas fa-sliders text-secondary text-sm"></i>
        </div>
        <div>
          <h3 class="font-bold text-sm text-base-content">{{ isEditMode ? '编辑模板' : '新建 Team 模板' }}</h3>
          <p class="text-xs text-base-content/40">{{ isEditMode ? '修改团队角色配置' : '定义多角色协作团队' }}</p>
        </div>
      </div>
      <button class="btn btn-xs btn-ghost text-base-content/50" @click="emit('cancel')">
        <i class="fas fa-times"></i>
      </button>
    </div>

    <!-- Scrollable content -->
    <div class="flex-1 overflow-y-auto">
      <!-- Basic Info -->
      <div class="px-5 py-4 border-b border-base-200 space-y-3">
        <h4 class="text-xs font-bold text-base-content/60 uppercase tracking-wider">基本信息</h4>
        
        <div class="grid grid-cols-2 gap-3">
          <div class="col-span-2 space-y-1">
            <label class="text-xs font-medium text-base-content/70">模板名称 <span class="text-error">*</span></label>
            <input
              v-model="form.name"
              type="text"
              class="input input-bordered input-sm w-full focus:border-secondary"
              placeholder="例：产品研发四角色团队"
              id="template-name-input"
            />
          </div>
          <div class="space-y-1">
            <label class="text-xs font-medium text-base-content/70">领域</label>
            <select
              v-model="form.domain"
              class="select select-bordered select-sm w-full focus:border-secondary"
              id="template-domain-select"
            >
              <option value="product">产品研发</option>
              <option value="security">安全评估</option>
              <option value="ops">运维架构</option>
              <option value="audit">代码审计</option>
              <option value="custom">自定义</option>
            </select>
          </div>
          <div class="space-y-1">
            <label class="text-xs font-medium text-base-content/70">默认总轮次（提案+审查+决策）</label>
            <input
              v-model.number="form.defaultRounds"
              type="number"
              min="1"
              max="10"
              class="input input-bordered input-sm w-full focus:border-secondary"
            />
          </div>
          <div class="col-span-2 space-y-1">
            <label class="text-xs font-medium text-base-content/70">模板描述</label>
            <textarea
              v-model="form.description"
              class="textarea textarea-bordered textarea-sm w-full resize-none h-16 focus:border-secondary"
              placeholder="简要描述此团队模板的用途和适用场景..."
            ></textarea>
          </div>
        </div>
      </div>

      <!-- Members -->
      <div class="px-5 py-4 space-y-3">
        <div class="flex items-center justify-between">
          <h4 class="text-xs font-bold text-base-content/60 uppercase tracking-wider">角色配置 ({{ form.members.length }})</h4>
          <button
            class="btn btn-xs btn-secondary btn-outline gap-1"
            @click="addMember"
            id="add-member-btn"
          >
            <i class="fas fa-plus text-xs"></i>
            添加角色
          </button>
        </div>

        <!-- Member list -->
        <div class="space-y-3">
          <TransitionGroup name="member-list">
            <div
              v-for="(member, idx) in form.members"
              :key="member._key"
              class="member-row border border-base-300 rounded-xl overflow-visible"
            >
              <!-- Member header -->
              <div
                class="flex items-center gap-2 px-3 py-2 cursor-pointer hover:bg-base-50 transition-colors"
                :class="expandedMember === idx ? 'bg-base-100 border-b border-base-200' : 'bg-base-50/50'"
                @click="expandedMember = expandedMember === idx ? -1 : idx"
              >
                <!-- Color dot -->
                <div
                  class="w-5 h-5 rounded-full flex items-center justify-center text-white text-[10px] font-bold flex-shrink-0"
                  :style="{ backgroundColor: roleColors[idx % roleColors.length] }"
                >
                  {{ member.name.charAt(0) || '?' }}
                </div>
                <span class="text-sm font-medium flex-1 min-w-0 truncate">
                  {{ member.name || `角色 ${idx + 1}` }}
                </span>
                <span class="text-xs text-base-content/40 truncate max-w-[120px]">{{ member.responsibility || '' }}</span>
                <div class="flex items-center gap-1 ml-2 flex-shrink-0">
                  <button
                    v-if="idx > 0"
                    class="btn btn-ghost btn-xs text-base-content/40 hover:text-base-content p-0 w-5 h-5 min-h-0"
                    @click.stop="moveMember(idx, -1)"
                    title="上移"
                  >↑</button>
                  <button
                    v-if="idx < form.members.length - 1"
                    class="btn btn-ghost btn-xs text-base-content/40 hover:text-base-content p-0 w-5 h-5 min-h-0"
                    @click.stop="moveMember(idx, 1)"
                    title="下移"
                  >↓</button>
                  <button
                    class="btn btn-ghost btn-xs text-base-content/30 hover:text-error p-0 w-5 h-5 min-h-0"
                    @click.stop="removeMember(idx)"
                    title="删除角色"
                  >
                    <i class="fas fa-times text-[10px]"></i>
                  </button>
                  <i
                    class="fas text-[10px] text-base-content/30 transition-transform"
                    :class="expandedMember === idx ? 'fa-chevron-up' : 'fa-chevron-down'"
                  ></i>
                </div>
              </div>

              <!-- Member form (expanded) -->
              <div v-if="expandedMember === idx" class="p-3 space-y-2.5 bg-base-100">
                <div class="grid grid-cols-2 gap-2">
                  <div class="space-y-1">
                    <label class="text-xs text-base-content/60">角色名称 *</label>
                    <input
                      v-model="member.name"
                      type="text"
                      class="input input-bordered input-xs w-full focus:border-secondary"
                      placeholder="例：产品经理"
                    />
                  </div>
                  <div class="space-y-1">
                    <label class="text-xs text-base-content/60">权重</label>
                    <input
                      v-model.number="member.weight"
                      type="number"
                      step="0.1"
                      min="0.1"
                      max="3"
                      class="input input-bordered input-xs w-full focus:border-secondary"
                    />
                  </div>
                </div>
                <div class="space-y-1">
                  <label class="text-xs text-base-content/60">职责描述</label>
                  <input
                    v-model="member.responsibility"
                    type="text"
                    class="input input-bordered input-xs w-full focus:border-secondary"
                    placeholder="例：负责需求分析和产品路线图"
                  />
                </div>
                <div class="space-y-1">
                  <label class="text-xs text-base-content/60">System Prompt</label>
                  <textarea
                    v-model="member.system_prompt"
                    class="textarea textarea-bordered textarea-xs w-full resize-none h-20 focus:border-secondary text-xs"
                    placeholder="定义角色行为的系统提示词..."
                  ></textarea>
                </div>
                <div class="grid grid-cols-2 gap-2">
                  <div class="space-y-1">
                    <label class="text-xs text-base-content/60">决策风格</label>
                    <select
                      v-model="member.decision_style"
                      class="select select-bordered select-xs w-full focus:border-secondary"
                    >
                      <option value="">不限</option>
                      <option value="conservative">保守 (conservative)</option>
                      <option value="balanced">平衡 (balanced)</option>
                      <option value="aggressive">激进 (aggressive)</option>
                      <option value="pragmatic">务实 (pragmatic)</option>
                      <option value="risk-aware">风险意识 (risk-aware)</option>
                    </select>
                  </div>
                  <div class="space-y-1">
                    <label class="text-xs text-base-content/60">风险偏好</label>
                    <select
                      v-model="member.risk_preference"
                      class="select select-bordered select-xs w-full focus:border-secondary"
                    >
                      <option value="">不限</option>
                      <option value="low">低风险 (low)</option>
                      <option value="medium">中等 (medium)</option>
                      <option value="high">高风险 (high)</option>
                    </select>
                  </div>
                </div>
                <div class="space-y-1">
                  <label class="text-xs text-base-content/60">角色模型（留空=使用默认 LLM）</label>
                  <SearchableSelect
                    :model-value="member.model_override"
                    :options="memberModelOptions"
                    placeholder="跟随默认模型"
                    search-placeholder="搜索模型或提供商..."
                    no-results-text="无匹配模型"
                    size="sm"
                    direction="up"
                    group-by="description"
                    :auto-width="false"
                    align="justify"
                    @update:model-value="member.model_override = $event"
                  />
                  <p class="text-[11px] text-base-content/45">
                    当前: {{ member.model_override || '默认 LLM（AI Settings）' }}
                  </p>
                </div>
              </div>
            </div>
          </TransitionGroup>

          <div v-if="form.members.length === 0" class="text-center py-6 text-base-content/30 text-sm border-2 border-dashed border-base-300 rounded-xl">
            <i class="fas fa-user-plus text-2xl mb-2 block"></i>
            点击「添加角色」开始定义团队成员
          </div>
        </div>
      </div>
    </div>

    <!-- Footer -->
    <div class="px-5 py-3 border-t border-base-300 bg-base-50/50 flex items-center justify-between gap-3">
      <div class="text-xs text-base-content/40">
        {{ form.members.length }} 个角色
        <span v-if="form.members.length < 2" class="text-warning ml-1">（至少需要 2 个角色）</span>
      </div>
      <div class="flex gap-2">
        <button class="btn btn-sm btn-ghost text-base-content/60" @click="emit('cancel')">取消</button>
        <button
          class="btn btn-sm btn-secondary gap-1"
          :disabled="!isFormValid || isSaving"
          @click="handleSave"
          id="save-template-btn"
        >
          <i v-if="isSaving" class="fas fa-spinner fa-spin"></i>
          <i v-else class="fas fa-save"></i>
          {{ isSaving ? '保存中...' : (isEditMode ? '保存更改' : '创建模板') }}
        </button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { agentTeamApi } from '@/api/agentTeam'
import type { AgentTeamTemplate } from '@/types/agentTeam'
import SearchableSelect from '@/components/SearchableSelect.vue'

// ==================== Props / Emits ====================

const props = defineProps<{
  template?: AgentTeamTemplate | null
}>()

const emit = defineEmits<{
  (e: 'save', template: AgentTeamTemplate): void
  (e: 'cancel'): void
}>()

// ==================== State ====================

interface MemberForm {
  _key: string
  name: string
  responsibility: string
  system_prompt: string
  decision_style: string
  risk_preference: string
  model_override: string
  weight: number
  sort_order: number
}

interface ModelOption {
  value: string
  label: string
  description: string
}

const form = ref({
  name: '',
  description: '',
  domain: 'product',
  defaultRounds: 5,
  members: [] as MemberForm[],
})

const expandedMember = ref(-1)
const isSaving = ref(false)
const modelOptions = ref<ModelOption[]>([])

const roleColors = ['#6366f1', '#8b5cf6', '#06b6d4', '#10b981', '#f59e0b', '#ef4444', '#ec4899']
const DEFAULT_TEMPLATE_ROUNDS = 5
const MIN_TEMPLATE_ROUNDS = 1
const MAX_TEMPLATE_ROUNDS = 10

// ==================== Computed ====================

const isEditMode = computed(() => !!props.template)

const isFormValid = computed(() =>
  form.value.name.trim().length > 0 && form.value.members.length >= 2 && form.value.members.every(m => m.name.trim().length > 0)
)

const memberModelOptions = computed<ModelOption[]>(() => [
  {
    value: '',
    label: '跟随默认模型',
    description: '默认',
  },
  ...modelOptions.value,
])

// ==================== Lifecycle ====================

onMounted(() => {
  loadModelOptions()
  if (props.template) {
    form.value.name = props.template.name
    form.value.description = props.template.description ?? ''
    form.value.domain = props.template.domain
    form.value.defaultRounds = extractTemplateDefaultRounds(props.template.default_rounds_config)
    form.value.members = props.template.members.map((m, i) => ({
      _key: `${m.id}-${i}`,
      name: m.name,
      responsibility: m.responsibility ?? '',
      system_prompt: m.system_prompt ?? '',
      decision_style: m.decision_style ?? '',
      risk_preference: m.risk_preference ?? '',
      model_override: parseMemberModelOverride(m.output_schema),
      weight: m.weight,
      sort_order: m.sort_order,
    }))
    if (form.value.members.length > 0) expandedMember.value = 0
  }
})

// ==================== Actions ====================

function addMember() {
  const key = `new-${Date.now()}`
  form.value.members.push({
    _key: key,
    name: '',
    responsibility: '',
    system_prompt: '',
    decision_style: 'balanced',
    risk_preference: 'medium',
    model_override: '',
    weight: 1.0,
    sort_order: form.value.members.length,
  })
  expandedMember.value = form.value.members.length - 1
}

function removeMember(idx: number) {
  form.value.members.splice(idx, 1)
  if (expandedMember.value >= form.value.members.length) {
    expandedMember.value = form.value.members.length - 1
  }
}

function moveMember(idx: number, dir: -1 | 1) {
  const arr = form.value.members
  const target = idx + dir
  if (target < 0 || target >= arr.length) return
  ;[arr[idx], arr[target]] = [arr[target], arr[idx]]
}

async function handleSave() {
  if (!isFormValid.value) return
  isSaving.value = true
  try {
    const normalizedRounds = normalizeTemplateRounds(form.value.defaultRounds)
    form.value.defaultRounds = normalizedRounds
    const memberPayload = form.value.members.map((m, i) => ({
      name: m.name.trim(),
      responsibility: m.responsibility || undefined,
      system_prompt: m.system_prompt || undefined,
      decision_style: m.decision_style || undefined,
      risk_preference: m.risk_preference || undefined,
      output_schema: buildMemberOutputSchema(m.model_override),
      weight: m.weight,
      sort_order: i,
    }))

    if (isEditMode.value && props.template) {
      await agentTeamApi.updateTemplate(props.template.id, {
        name: form.value.name.trim(),
        description: form.value.description || undefined,
        domain: form.value.domain,
        default_rounds_config: {
          max_rounds: normalizedRounds,
        },
        members: memberPayload,
      })
      const updated = await agentTeamApi.getTemplate(props.template.id)
      emit('save', updated!)
    } else {
      const created = await agentTeamApi.createTemplate({
        name: form.value.name.trim(),
        description: form.value.description || undefined,
        domain: form.value.domain,
        default_rounds_config: {
          max_rounds: normalizedRounds,
        },
        members: memberPayload,
      })
      emit('save', created)
    }
  } catch (e) {
    console.error('[AgentTeamSettings] Save failed:', e)
  } finally {
    isSaving.value = false
  }
}

function normalizeProviderName(provider: string) {
  const names: Record<string, string> = {
    openai: 'OpenAI',
    anthropic: 'Anthropic',
    gemini: 'Gemini',
    deepseek: 'DeepSeek',
    moonshot: 'Moonshot',
    ollama: 'Ollama',
    openrouter: 'OpenRouter',
    groq: 'Groq',
    perplexity: 'Perplexity',
    xai: 'xAI',
    cohere: 'Cohere',
    lmstudio: 'LM Studio',
    modelscope: 'ModelScope',
  }
  return names[provider.toLowerCase()] || provider
}

function extractModelId(item: any): string {
  if (!item) return ''
  if (typeof item === 'string') return item
  if (typeof item.id === 'string') return item.id
  if (typeof item.name === 'string') return item.name
  return ''
}

async function loadModelOptions() {
  try {
    const aiConfig = await invoke<any>('get_ai_config')
    const providers = aiConfig?.providers && typeof aiConfig.providers === 'object'
      ? aiConfig.providers
      : {}

    const options: ModelOption[] = []

    Object.entries(providers).forEach(([providerKey, providerValue]) => {
      const cfg = providerValue as any
      if (cfg?.enabled === false) return
      const provider = String(cfg?.provider || providerKey || '').trim().toLowerCase()
      if (!provider) return

      const ids: string[] = (Array.isArray(cfg?.models) ? cfg.models : [])
        .map(extractModelId)
        .filter((v: string) => !!v)
      if (typeof cfg?.default_model === 'string' && cfg.default_model.trim()) {
        ids.push(cfg.default_model.trim())
      }

      Array.from(new Set<string>(ids)).forEach((modelId: string) => {
        options.push({
          value: `${provider}/${modelId}`,
          label: modelId,
          description: normalizeProviderName(provider),
        })
      })
    })

    modelOptions.value = options.sort((a, b) => a.value.localeCompare(b.value))
  } catch (e) {
    console.warn('[AgentTeamSettings] Failed to load model options:', e)
    modelOptions.value = []
  }
}

function parseMemberModelOverride(outputSchema: any): string {
  if (!outputSchema || typeof outputSchema !== 'object') return ''
  if (typeof outputSchema.llm_model === 'string' && outputSchema.llm_model.includes('/')) {
    return outputSchema.llm_model
  }
  const provider = typeof outputSchema.model_provider === 'string'
    ? outputSchema.model_provider.trim()
    : ''
  const modelName = typeof outputSchema.model_name === 'string'
    ? outputSchema.model_name.trim()
    : ''
  if (provider && modelName) return `${provider}/${modelName}`
  return ''
}

function buildMemberOutputSchema(modelOverride: string) {
  if (!modelOverride || !modelOverride.includes('/')) return undefined
  const [provider, ...modelNameParts] = modelOverride.split('/')
  const modelName = modelNameParts.join('/').trim()
  if (!provider.trim() || !modelName) return undefined
  return {
    model_provider: provider.trim().toLowerCase(),
    model_name: modelName,
    llm_model: `${provider.trim().toLowerCase()}/${modelName}`,
  }
}

function normalizeTemplateRounds(value: unknown): number {
  const n = Number(value)
  if (!Number.isFinite(n)) return DEFAULT_TEMPLATE_ROUNDS
  const normalized = Math.trunc(n)
  return Math.max(MIN_TEMPLATE_ROUNDS, Math.min(MAX_TEMPLATE_ROUNDS, normalized))
}

function extractTemplateDefaultRounds(config: unknown): number {
  if (typeof config === 'number') {
    return normalizeTemplateRounds(config)
  }
  if (config && typeof config === 'object') {
    const obj = config as Record<string, unknown>
    const candidate =
      obj.max_rounds ??
      obj.maxRounds ??
      obj.default_rounds ??
      obj.rounds
    return normalizeTemplateRounds(candidate)
  }
  return DEFAULT_TEMPLATE_ROUNDS
}
</script>

<style scoped>
.member-list-enter-active,
.member-list-leave-active {
  transition: all 0.2s ease;
}
.member-list-enter-from {
  opacity: 0;
  transform: translateY(-4px);
}
.member-list-leave-to {
  opacity: 0;
}
</style>
