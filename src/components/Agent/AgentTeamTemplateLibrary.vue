<template>
  <div class="template-library flex flex-col h-full overflow-hidden">
    <!-- Header -->
    <div class="flex items-center justify-between px-4 py-3 border-b border-base-300 bg-base-100/80 backdrop-blur-sm">
      <div class="flex items-center gap-2">
        <i class="fas fa-layer-group text-primary"></i>
        <h2 class="text-sm font-bold text-base-content">Team 模板库</h2>
        <span class="badge badge-sm badge-primary">{{ filteredTemplates.length }}</span>
      </div>
      <div class="flex items-center gap-2">
        <!-- AI Generate -->
        <button
          class="btn btn-xs btn-primary gap-1"
          @click="showAiGenerateModal = true"
          id="ai-generate-template-btn-library"
          title="使用 AI 自动生成模板"
        >
          <i class="fas fa-wand-magic-sparkles text-xs"></i>
          AI 生成
        </button>
        <button
          class="btn btn-xs btn-primary gap-1"
          @click="showCreateModal = true"
          id="create-template-btn"
        >
          <i class="fas fa-plus"></i>
          <span>新建模板</span>
        </button>
        <button
          class="btn btn-xs btn-ghost"
          @click="emit('close')"
          title="关闭"
        >
          <i class="fas fa-times"></i>
        </button>
      </div>
    </div>

    <!-- Search & Filter Bar -->
    <div class="px-4 py-2.5 border-b border-base-300 bg-base-50/50 flex items-center gap-2">
      <div class="relative flex-1">
        <i class="fas fa-search absolute left-2.5 top-1/2 -translate-y-1/2 text-base-content/30 text-xs"></i>
        <input
          v-model="searchQuery"
          type="text"
          class="input input-xs w-full pl-7 pr-3 bg-base-100 border-base-300 focus:border-primary"
          placeholder="搜索模板名称..."
          id="template-search-input"
        />
      </div>
      <!-- Domain filter -->
      <select
        v-model="domainFilter"
        class="select select-xs bg-base-100 border-base-300 focus:border-primary min-w-[80px]"
        id="domain-filter-select"
      >
        <option value="">所有领域</option>
        <option value="product">产品</option>
        <option value="security">安全</option>
        <option value="ops">运维</option>
        <option value="audit">审计</option>
        <option value="custom">自定义</option>
      </select>
      <!-- System/Custom filter -->
      <div class="flex rounded-lg overflow-hidden border border-base-300">
        <button
          class="px-2 py-1 text-xs transition-colors"
          :class="typeFilter === 'all' ? 'bg-primary text-white' : 'bg-base-100 text-base-content/60 hover:bg-base-200'"
          @click="typeFilter = 'all'"
        >全部</button>
        <button
          class="px-2 py-1 text-xs transition-colors"
          :class="typeFilter === 'system' ? 'bg-primary text-white' : 'bg-base-100 text-base-content/60 hover:bg-base-200'"
          @click="typeFilter = 'system'"
        >内置</button>
        <button
          class="px-2 py-1 text-xs transition-colors"
          :class="typeFilter === 'custom' ? 'bg-primary text-white' : 'bg-base-100 text-base-content/60 hover:bg-base-200'"
          @click="typeFilter = 'custom'"
        >自定义</button>
      </div>
    </div>

    <!-- Template Grid -->
    <div class="flex-1 overflow-y-auto p-4">
      <!-- Loading -->
      <div v-if="isLoading" class="flex items-center justify-center h-40 text-base-content/40">
        <i class="fas fa-spinner fa-spin mr-2"></i> 加载中...
      </div>

      <!-- Empty state -->
      <div v-else-if="filteredTemplates.length === 0" class="flex flex-col items-center justify-center h-40 gap-3 text-base-content/40">
        <i class="fas fa-folder-open text-3xl"></i>
        <p class="text-sm">{{ searchQuery || domainFilter ? '没有匹配的模板' : '还没有模板，创建第一个吧' }}</p>
        <button v-if="!searchQuery && !domainFilter" class="btn btn-sm btn-primary gap-1" @click="showCreateModal = true">
          <i class="fas fa-plus"></i> 新建模板
        </button>
      </div>

      <!-- Grid -->
      <div v-else class="grid gap-3">
        <TransitionGroup name="template-list">
          <div
            v-for="tpl in filteredTemplates"
            :key="tpl.id"
            class="template-card group relative p-4 rounded-xl border-2 bg-base-100 transition-all hover:shadow-md cursor-pointer"
            :class="selectedId === tpl.id ? 'border-primary shadow-primary/20 shadow-md' : 'border-base-300 hover:border-primary/40'"
            @click="selectedId = tpl.id"
            :id="`template-card-${tpl.id}`"
          >
            <!-- Top row -->
            <div class="flex items-start justify-between gap-2 mb-2">
              <div class="flex items-center gap-2 flex-1 min-w-0">
                <!-- Domain icon -->
                <div class="w-8 h-8 rounded-lg flex items-center justify-center flex-shrink-0"
                  :class="domainBgClass(tpl.domain)">
                  <i :class="domainIcon(tpl.domain)" class="text-sm"></i>
                </div>
                <div class="min-w-0">
                  <h3 class="font-semibold text-sm text-base-content truncate leading-tight">
                    {{ tpl.name }}
                  </h3>
                  <div class="flex items-center gap-1 mt-0.5">
                    <span v-if="tpl.is_system" class="badge badge-xs badge-info">内置</span>
                    <span class="badge badge-xs badge-ghost capitalize">{{ tpl.domain }}</span>
                    <span class="badge badge-xs badge-ghost">{{ tpl.members.length }} 角色</span>
                  </div>
                </div>
              </div>

              <!-- Action buttons (show on hover) -->
              <div class="flex items-center gap-1 opacity-0 group-hover:opacity-100 transition-opacity flex-shrink-0">
                <button
                  class="btn btn-xs btn-ghost text-base-content/60"
                  @click.stop="handleClone(tpl)"
                  title="复制模板"
                >
                  <i class="fas fa-copy text-xs"></i>
                </button>
                <button
                  v-if="!tpl.is_system"
                  class="btn btn-xs btn-ghost text-base-content/60 hover:text-error"
                  @click.stop="handleDelete(tpl)"
                  title="删除模板"
                >
                  <i class="fas fa-trash text-xs"></i>
                </button>
                <button
                  class="btn btn-xs btn-ghost text-base-content/60"
                  @click.stop="handleEditTemplate(tpl)"
                  title="编辑模板"
                >
                  <i class="fas fa-edit text-xs"></i>
                </button>
              </div>
            </div>

            <!-- Description -->
            <p v-if="tpl.description" class="text-xs text-base-content/55 mb-2.5 line-clamp-2 leading-relaxed">
              {{ tpl.description }}
            </p>

            <!-- Member chips -->
            <div class="flex flex-wrap gap-1 mb-3">
              <span
                v-for="m in tpl.members"
                :key="m.id"
                class="badge badge-xs px-2 py-0.5 font-normal"
                :style="{ backgroundColor: memberColor(m.name) + '20', color: memberColor(m.name), borderColor: memberColor(m.name) + '40' }"
              >
                {{ m.name }}
              </span>
            </div>

            <!-- Footer: Use template button -->
            <div class="flex items-center justify-between">
              <span class="text-xs text-base-content/30">
                {{ formatDate(tpl.updated_at) }}
              </span>
              <button
                class="btn btn-xs btn-primary gap-1 opacity-0 group-hover:opacity-100 transition-opacity"
                :class="selectedId === tpl.id ? 'opacity-100' : ''"
                @click.stop="handleUseTemplate(tpl)"
                :id="`use-template-${tpl.id}`"
              >
                <i class="fas fa-play text-xs"></i>
                使用此模板
              </button>
            </div>
          </div>
        </TransitionGroup>
      </div>
    </div>

    <!-- Create Template Modal -->
    <Teleport to="body">
      <div v-if="showCreateModal" class="fixed inset-0 z-[100] flex items-center justify-center p-4">
        <div class="absolute inset-0 bg-black/40 backdrop-blur-sm" @click="showCreateModal = false"></div>
        <div class="relative z-10 bg-base-100 rounded-2xl shadow-2xl w-full max-w-lg overflow-hidden">
          <AgentTeamSettings
            @save="handleTemplateSaved"
            @cancel="showCreateModal = false"
          />
        </div>
      </div>
    </Teleport>

    <!-- Edit Template Modal -->
    <Teleport to="body">
      <div v-if="editingTemplate" class="fixed inset-0 z-[100] flex items-center justify-center p-4">
        <div class="absolute inset-0 bg-black/40 backdrop-blur-sm" @click="editingTemplate = null"></div>
        <div class="relative z-10 bg-base-100 rounded-2xl shadow-2xl w-full max-w-lg overflow-hidden">
          <AgentTeamSettings
            :template="editingTemplate"
            @save="handleTemplateSaved"
            @cancel="editingTemplate = null"
          />
        </div>
      </div>
    </Teleport>

    <!-- AI Generate Template Modal -->
    <AIGenerateTemplateModal
      :show="showAiGenerateModal"
      @close="showAiGenerateModal = false"
      @saved="handleAiTemplateSaved"
    />

    <!-- Use Template Modal -->
    <Teleport to="body">
      <CreateTeamFromTemplateModal
        v-if="usingTemplate"
        :template="usingTemplate"
        :conversation-id="conversationId"
        @created="handleSessionCreated"
        @cancel="usingTemplate = null"
      />
    </Teleport>

    <!-- Delete confirm toast -->
    <Teleport to="body">
      <div v-if="deletingTemplate" class="fixed bottom-4 left-1/2 -translate-x-1/2 z-[200] animate-in slide-in-from-bottom-4">
        <div class="bg-base-100 border border-error/40 shadow-xl rounded-xl px-4 py-3 flex items-center gap-4 min-w-[320px]">
          <i class="fas fa-exclamation-triangle text-error"></i>
          <div class="flex-1 text-sm">
            <span class="font-medium">删除「{{ deletingTemplate.name }}」?</span>
            <span class="text-base-content/50 ml-1">此操作不可撤销</span>
          </div>
          <div class="flex gap-2">
            <button class="btn btn-xs btn-ghost" @click="deletingTemplate = null">取消</button>
            <button class="btn btn-xs btn-error" @click="confirmDelete" :disabled="isDeleting">
              <i v-if="isDeleting" class="fas fa-spinner fa-spin mr-1"></i>
              确认删除
            </button>
          </div>
        </div>
      </div>
    </Teleport>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { agentTeamApi } from '@/api/agentTeam'
import type { AgentTeamTemplate } from '@/types/agentTeam'
import AgentTeamSettings from './AgentTeamSettings.vue'
import CreateTeamFromTemplateModal from './CreateTeamFromTemplateModal.vue'
import AIGenerateTemplateModal from './AIGenerateTemplateModal.vue'

// ==================== Props / Emits ====================

const props = defineProps<{
  conversationId?: string
}>()

const emit = defineEmits<{
  (e: 'close'): void
  (e: 'session-created', sessionId: string): void
  (e: 'templates-updated', templateId?: string): void
}>()

// ==================== State ====================

const templates = ref<AgentTeamTemplate[]>([])
const isLoading = ref(true)
const searchQuery = ref('')
const domainFilter = ref('')
const typeFilter = ref<'all' | 'system' | 'custom'>('all')
const selectedId = ref<string | null>(null)

const showCreateModal = ref(false)
const showAiGenerateModal = ref(false)
const editingTemplate = ref<AgentTeamTemplate | null>(null)
const usingTemplate = ref<AgentTeamTemplate | null>(null)
const deletingTemplate = ref<AgentTeamTemplate | null>(null)
const isDeleting = ref(false)

// ==================== Computed ====================

const filteredTemplates = computed(() => {
  return templates.value.filter(t => {
    if (typeFilter.value === 'system' && !t.is_system) return false
    if (typeFilter.value === 'custom' && t.is_system) return false
    if (domainFilter.value && t.domain !== domainFilter.value) return false
    if (searchQuery.value) {
      const q = searchQuery.value.toLowerCase()
      return t.name.toLowerCase().includes(q) || (t.description ?? '').toLowerCase().includes(q)
    }
    return true
  })
})

// ==================== Lifecycle ====================

onMounted(loadTemplates)

// ==================== Data ====================

async function loadTemplates() {
  isLoading.value = true
  try {
    let list = await agentTeamApi.listTemplates()
    if (list.length === 0) {
      await agentTeamApi.seedBuiltinTemplates()
      list = await agentTeamApi.listTemplates()
    }
    templates.value = list
  } catch (e) {
    console.error('[TemplateLibrary] Failed to load:', e)
  } finally {
    isLoading.value = false
  }
}

// ==================== Actions ====================

function handleUseTemplate(tpl: AgentTeamTemplate) {
  usingTemplate.value = tpl
}

function handleEditTemplate(tpl: AgentTeamTemplate) {
  editingTemplate.value = tpl
}

async function handleClone(tpl: AgentTeamTemplate) {
  try {
    const created = await agentTeamApi.createTemplate({
      name: `${tpl.name} (副本)`,
      description: tpl.description,
      domain: tpl.domain,
      members: tpl.members.map(m => ({
        name: m.name,
        responsibility: m.responsibility,
        system_prompt: m.system_prompt,
        decision_style: m.decision_style,
        risk_preference: m.risk_preference,
        weight: m.weight,
        sort_order: m.sort_order,
      })),
    })
    selectedId.value = created.id
    await loadTemplates()
    emit('templates-updated', created.id)
  } catch (e) {
    console.error('[TemplateLibrary] Clone failed:', e)
  }
}

function handleDelete(tpl: AgentTeamTemplate) {
  deletingTemplate.value = tpl
}

async function confirmDelete() {
  if (!deletingTemplate.value) return
  isDeleting.value = true
  try {
    await agentTeamApi.deleteTemplate(deletingTemplate.value.id)
    templates.value = templates.value.filter(t => t.id !== deletingTemplate.value!.id)
    deletingTemplate.value = null
    emit('templates-updated')
  } catch (e) {
    console.error('[TemplateLibrary] Delete failed:', e)
  } finally {
    isDeleting.value = false
  }
}

async function handleTemplateSaved(template: AgentTeamTemplate) {
  showCreateModal.value = false
  editingTemplate.value = null
  if (template?.id) {
    selectedId.value = template.id
  }
  await loadTemplates()
  emit('templates-updated', template?.id)
}

async function handleAiTemplateSaved(template: AgentTeamTemplate) {
  templates.value.unshift(template)
  selectedId.value = template.id
  emit('templates-updated', template.id)
}

function handleSessionCreated(sessionId: string) {
  usingTemplate.value = null
  emit('session-created', sessionId)
  emit('close')
}

// ==================== Display helpers ====================

function domainIcon(domain: string): string {
  const map: Record<string, string> = {
    product: 'fas fa-box text-primary',
    security: 'fas fa-shield-halved text-error',
    ops: 'fas fa-server text-warning',
    audit: 'fas fa-search text-info',
    custom: 'fas fa-sliders text-secondary',
  }
  return map[domain] ?? 'fas fa-users text-base-content/60'
}

function domainBgClass(domain: string): string {
  const map: Record<string, string> = {
    product: 'bg-primary/10',
    security: 'bg-error/10',
    ops: 'bg-warning/10',
    audit: 'bg-info/10',
    custom: 'bg-secondary/10',
  }
  return map[domain] ?? 'bg-base-200'
}

const MEMBER_COLORS = ['#6366f1', '#8b5cf6', '#06b6d4', '#10b981', '#f59e0b', '#ef4444']
function memberColor(name: string): string {
  let hash = 0
  for (let i = 0; i < name.length; i++) hash = (hash + name.charCodeAt(i)) % MEMBER_COLORS.length
  return MEMBER_COLORS[hash]
}

function formatDate(dateStr: string): string {
  try {
    const d = new Date(dateStr)
    return d.toLocaleDateString('zh-CN', { month: 'short', day: 'numeric' })
  } catch {
    return ''
  }
}
</script>

<style scoped>
.template-list-enter-active,
.template-list-leave-active {
  transition: all 0.2s ease;
}
.template-list-enter-from {
  opacity: 0;
  transform: translateY(6px);
}
.template-list-leave-to {
  opacity: 0;
}
</style>
