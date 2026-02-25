<template>
  <div class="blackboard-panel flex flex-col h-full overflow-hidden bg-base-100">
    <!-- Header -->
    <div class="px-4 py-3 border-b border-base-300 flex items-center justify-between">
      <div class="flex items-center gap-2">
        <i class="fas fa-chalkboard text-secondary"></i>
        <h3 class="font-bold text-sm text-base-content">共享白板</h3>
        <span class="badge badge-xs badge-secondary">{{ entries.length }}</span>
      </div>
      <div class="flex items-center gap-2">
        <!-- Filter tabs -->
        <div class="flex rounded-lg overflow-hidden border border-base-300 text-xs">
          <button
            v-for="tab in tabs"
            :key="tab.value"
            class="px-2 py-1 transition-colors"
            :class="activeFilter === tab.value
              ? 'bg-secondary text-white'
              : 'bg-base-100 text-base-content/50 hover:bg-base-200'"
            @click="activeFilter = tab.value"
          >
            {{ tab.label }}
          </button>
        </div>
        <button v-if="showClose" class="btn btn-xs btn-ghost" @click="emit('close')">
          <i class="fas fa-times"></i>
        </button>
      </div>
    </div>

    <!-- Entries -->
    <div class="flex-1 overflow-y-auto p-3 space-y-2">
      <div v-if="filteredEntries.length === 0" class="text-center py-8 text-base-content/30 text-sm">
        <i class="fas fa-comment-slash text-2xl mb-2 block"></i>
        {{ activeFilter === 'all' ? '白板为空' : `暂无${filterLabel}条目` }}
      </div>

      <TransitionGroup name="bb-entry">
        <div
          v-for="entry in filteredEntries"
          :key="entry.id"
          class="bb-entry rounded-xl border p-3 transition-all"
          :class="entryClass(entry.entry_type)"
        >
          <!-- Entry header -->
          <div class="flex items-start justify-between gap-2 mb-1.5">
            <div class="flex items-center gap-1.5">
              <i :class="entryIcon(entry.entry_type)" class="text-sm flex-shrink-0"></i>
              <span class="text-xs font-semibold" :class="entryTextClass(entry.entry_type)">
                {{ entryTypeLabel(entry.entry_type) }}
              </span>
              <span v-if="entry.contributed_by" class="text-xs text-base-content/40">
                · {{ entry.contributed_by }}
              </span>
            </div>
            <div class="flex items-center gap-1 flex-shrink-0">
              <!-- Resolve button for disputes -->
              <button
                v-if="entry.entry_type === 'dispute' && !entry.is_resolved && canAnnotate"
                class="btn btn-xs btn-ghost text-success"
                @click="emit('resolve', entry.id)"
                title="标记为已解决"
              >
                <i class="fas fa-check text-xs"></i>
              </button>
              <!-- Resolved indicator -->
              <span v-if="entry.is_resolved" class="badge badge-xs badge-success">已解决</span>
            </div>
          </div>

          <!-- Title -->
          <div class="text-xs font-medium text-base-content/85 mb-1">{{ entry.title }}</div>

          <!-- Content (expandable) -->
          <div
            class="text-xs text-base-content/65 leading-relaxed cursor-pointer"
            :class="expandedEntry === entry.id ? '' : 'line-clamp-4'"
            @click="expandedEntry = expandedEntry === entry.id ? null : entry.id"
          >
            {{ entry.content }}
          </div>

          <!-- Annotation input -->
          <div v-if="annotatingEntry === entry.id" class="mt-2">
            <div class="flex gap-1.5">
              <input
                v-model="annotationText"
                type="text"
                class="input input-xs flex-1 focus:border-secondary"
                placeholder="添加批注..."
                @keydown.enter="submitAnnotation(entry.id)"
              />
              <button class="btn btn-xs btn-secondary" @click="submitAnnotation(entry.id)">
                <i class="fas fa-paper-plane text-xs"></i>
              </button>
              <button class="btn btn-xs btn-ghost" @click="annotatingEntry = null">取消</button>
            </div>
          </div>

          <!-- Footer -->
          <div class="flex items-center justify-between mt-1.5">
            <span class="text-[10px] text-base-content/30">{{ formatTime(entry.created_at) }}</span>
            <button
              v-if="canAnnotate && annotatingEntry !== entry.id"
              class="btn btn-xs btn-ghost text-base-content/30 hover:text-secondary py-0 h-auto"
              @click="startAnnotate(entry.id)"
            >
              <i class="fas fa-pen text-[10px]"></i>
              <span class="text-[10px]">批注</span>
            </button>
          </div>
        </div>
      </TransitionGroup>
    </div>

    <!-- Add manual entry -->
    <div v-if="canAnnotate" class="px-3 py-2.5 border-t border-base-300 bg-base-50/50">
      <div v-if="!showAddForm" class="flex justify-center">
        <button
          class="btn btn-xs btn-ghost text-secondary gap-1"
          @click="showAddForm = true"
        >
          <i class="fas fa-plus text-xs"></i> 添加白板条目
        </button>
      </div>
      <div v-else class="space-y-2">
        <div class="flex gap-1.5">
          <select v-model="newEntry.type" class="select select-xs bg-base-100 border-base-300 min-w-[80px]">
            <option value="consensus">共识</option>
            <option value="dispute">分歧</option>
            <option value="action_item">待办</option>
          </select>
          <input
            v-model="newEntry.title"
            type="text"
            class="input input-xs flex-1 focus:border-secondary"
            placeholder="标题..."
          />
        </div>
        <textarea
          v-model="newEntry.content"
          class="textarea textarea-xs w-full resize-none h-14 focus:border-secondary"
          placeholder="内容..."
        ></textarea>
        <div class="flex gap-1.5 justify-end">
          <button class="btn btn-xs btn-ghost" @click="showAddForm = false">取消</button>
          <button
            class="btn btn-xs btn-secondary gap-1"
            :disabled="!newEntry.title.trim() || !newEntry.content.trim()"
            @click="submitNewEntry"
          >
            <i class="fas fa-save text-xs"></i> 保存
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import type { AgentTeamBlackboardEntry } from '@/types/agentTeam'

// ==================== Props / Emits ====================

const props = defineProps<{
  entries: AgentTeamBlackboardEntry[]
  canAnnotate?: boolean
  showClose?: boolean
}>()

const emit = defineEmits<{
  (e: 'close'): void
  (e: 'resolve', entryId: string): void
  (e: 'add-entry', type: string, title: string, content: string): void
  (e: 'annotate', entryId: string, text: string): void
}>()

// ==================== State ====================

type FilterType = 'all' | 'consensus' | 'dispute' | 'action_item'
const activeFilter = ref<FilterType>('all')
const expandedEntry = ref<string | null>(null)
const annotatingEntry = ref<string | null>(null)
const annotationText = ref('')
const showAddForm = ref(false)
const newEntry = ref({ type: 'consensus', title: '', content: '' })

const tabs: Array<{ value: FilterType; label: string }> = [
  { value: 'all', label: '全部' },
  { value: 'consensus', label: '共识' },
  { value: 'dispute', label: '分歧' },
  { value: 'action_item', label: '待办' },
]

// ==================== Computed ====================

const filteredEntries = computed(() => {
  if (activeFilter.value === 'all') return props.entries
  return props.entries.filter(e => e.entry_type === activeFilter.value)
})

const filterLabel = computed(() => {
  const map: Record<string, string> = { consensus: '共识', dispute: '分歧', action_item: '待办' }
  return map[activeFilter.value] ?? ''
})

// ==================== Actions ====================

function startAnnotate(id: string) {
  annotatingEntry.value = id
  annotationText.value = ''
}

function submitAnnotation(id: string) {
  if (!annotationText.value.trim()) return
  emit('annotate', id, annotationText.value.trim())
  annotatingEntry.value = null
  annotationText.value = ''
}

function submitNewEntry() {
  if (!newEntry.value.title.trim() || !newEntry.value.content.trim()) return
  emit('add-entry', newEntry.value.type, newEntry.value.title.trim(), newEntry.value.content.trim())
  newEntry.value = { type: 'consensus', title: '', content: '' }
  showAddForm.value = false
}

// ==================== Display helpers ====================

function entryClass(type: string): string {
  if (type === 'consensus') return 'border-success/30 bg-success/5'
  if (type === 'dispute') return 'border-error/30 bg-error/5'
  return 'border-info/30 bg-info/5'
}

function entryTextClass(type: string): string {
  if (type === 'consensus') return 'text-success'
  if (type === 'dispute') return 'text-error'
  return 'text-info'
}

function entryIcon(type: string): string {
  if (type === 'consensus') return 'fas fa-handshake text-success'
  if (type === 'dispute') return 'fas fa-exclamation-triangle text-error'
  return 'fas fa-tasks text-info'
}

function entryTypeLabel(type: string): string {
  if (type === 'consensus') return '共识'
  if (type === 'dispute') return '分歧'
  return '待办'
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
.bb-entry-enter-active,
.bb-entry-leave-active {
  transition: all 0.25s ease;
}
.bb-entry-enter-from {
  opacity: 0;
  transform: translateX(-8px);
}
.bb-entry-leave-to {
  opacity: 0;
  transform: translateX(8px);
}
</style>
