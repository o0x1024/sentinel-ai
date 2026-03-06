<template>
  <dialog ref="dialogRef" class="modal" @close="handleClose">
    <div class="modal-box max-w-5xl w-11/12 h-[85vh] flex flex-col overflow-hidden p-0">
      <div class="px-5 py-4 border-b border-base-300 flex items-center justify-between gap-3">
        <div>
          <h3 class="font-semibold text-lg">{{ t('aiAssistant.turnLogsTitle', 'Turn 日志') }}</h3>
          <p class="text-sm text-base-content/60">
            {{ t('aiAssistant.turnLogsDescription', '查看按轮聚合的对话与工具调用日志') }}
          </p>
        </div>
        <button class="btn btn-sm btn-circle btn-ghost" @click="closeDialog">
          <i class="fas fa-times"></i>
        </button>
      </div>

      <div class="px-5 py-4 border-b border-base-300 grid grid-cols-1 md:grid-cols-[180px_1fr_1fr_110px] gap-3">
        <label class="form-control">
          <span class="label-text text-xs">{{ t('aiAssistant.turnLogsDate', '日期') }}</span>
          <input v-model="date" type="date" class="input input-sm input-bordered" />
        </label>
        <label class="form-control">
          <span class="label-text text-xs">{{ t('aiAssistant.turnLogsConversation', '会话标题') }}</span>
          <SearchableSelect
            v-model="conversationId"
            size="sm"
            :options="conversationOptions"
            :placeholder="t('aiAssistant.turnLogsConversationPlaceholder', '选择会话标题')"
            :search-placeholder="t('aiAssistant.turnLogsConversationSearchPlaceholder', '搜索会话标题')"
            :no-results-text="t('aiAssistant.turnLogsConversationNoResults', '没有匹配的会话')"
          />
        </label>
        <label class="form-control">
          <span class="label-text text-xs">{{ t('aiAssistant.turnLogsSession', 'Turn Session') }}</span>
          <input
            v-model.trim="sessionId"
            type="text"
            class="input input-sm input-bordered"
            :placeholder="t('aiAssistant.turnLogsSessionPlaceholder', '按 session_id 过滤')"
          />
        </label>
        <label class="form-control">
          <span class="label-text text-xs">{{ t('aiAssistant.turnLogsLimit', '数量') }}</span>
          <input v-model.number="limit" type="number" min="1" max="500" class="input input-sm input-bordered" />
        </label>
        <div class="md:col-span-4 flex items-center gap-2 justify-end">
          <button class="btn btn-sm btn-outline" @click="applyActiveSessionFilter">
            {{ t('aiAssistant.turnLogsUseCurrentSession', '使用当前会话') }}
          </button>
          <button class="btn btn-sm btn-primary gap-2" :disabled="loading" @click="loadLogs">
            <span v-if="loading" class="loading loading-spinner loading-xs"></span>
            <i v-else class="fas fa-rotate"></i>
            {{ t('aiAssistant.turnLogsRefresh', '刷新') }}
          </button>
        </div>
      </div>

      <div class="flex-1 overflow-y-auto px-5 py-4 space-y-3 bg-base-200/20">
        <div v-if="error" class="alert alert-error text-sm">
          {{ error }}
        </div>

        <div v-else-if="loading" class="h-full flex items-center justify-center text-base-content/60">
          <span class="loading loading-spinner loading-md"></span>
        </div>

        <div v-else-if="logs.length === 0" class="h-full flex items-center justify-center text-base-content/50 text-sm">
          {{ t('aiAssistant.turnLogsEmpty', '当前筛选条件下没有日志') }}
        </div>

        <details
          v-for="entry in logs"
          :key="`${entry.session_id}-${entry.timestamp}`"
          class="collapse collapse-arrow bg-base-100 border border-base-300"
          @toggle="handleToggle(entry, $event)"
        >
          <summary class="collapse-title pr-12">
            <div class="flex flex-wrap items-center gap-2 text-sm pr-28">
              <span class="badge badge-outline">#{{ entry.turn ?? '-' }}</span>
              <span class="badge" :class="statusBadgeClass(entry.status)">
                {{ entry.status || 'unknown' }}
              </span>
              <span class="font-medium">{{ entry.provider }} / {{ entry.model }}</span>
              <span class="text-base-content/60">{{ formatTime(entry.timestamp) }}</span>
              <span class="text-base-content/60 truncate max-w-[360px]">{{ entry.session_id }}</span>
            </div>
            <button
              class="btn btn-xs btn-outline absolute right-12 top-1/2 -translate-y-1/2 z-10"
              @click.stop="openConversation(entry)"
            >
              {{ t('aiAssistant.turnLogsOpenConversation', '打开会话') }}
            </button>
          </summary>
          <div class="collapse-content space-y-4 text-sm">
            <div class="grid grid-cols-1 md:grid-cols-3 gap-3">
              <div class="stat bg-base-200/50 rounded-box px-4 py-3">
                <div class="stat-title text-xs">{{ t('aiAssistant.turnLogsDuration', '耗时') }}</div>
                <div class="stat-value text-lg">{{ formatDuration(entry.duration_ms) }}</div>
              </div>
              <div class="stat bg-base-200/50 rounded-box px-4 py-3">
                <div class="stat-title text-xs">{{ t('aiAssistant.turnLogsTokens', 'Tokens') }}</div>
                <div class="stat-value text-lg">
                  {{ formatTokens(entry.input_tokens, entry.output_tokens) }}
                </div>
              </div>
              <div class="stat bg-base-200/50 rounded-box px-4 py-3">
                <div class="stat-title text-xs">{{ t('aiAssistant.turnLogsTools', '工具调用') }}</div>
                <div class="stat-value text-lg">{{ entry.tool_call_count }}</div>
              </div>
            </div>

            <div class="grid grid-cols-1 xl:grid-cols-2 gap-4">
              <section class="space-y-2">
                <h4 class="font-medium">{{ t('aiAssistant.turnLogsUserPrompt', '用户输入') }} Preview</h4>
                <pre class="bg-base-200/60 rounded-lg p-3 whitespace-pre-wrap break-words">{{ entry.user_request_preview || '-' }}</pre>
              </section>
              <section class="space-y-2">
                <h4 class="font-medium">{{ t('aiAssistant.turnLogsAssistantResponse', '助手回复') }} Preview</h4>
                <pre class="bg-base-200/60 rounded-lg p-3 whitespace-pre-wrap break-words">{{ entry.assistant_response_preview || '-' }}</pre>
              </section>
            </div>

            <div v-if="isDetailLoading(entry.session_id)" class="flex items-center gap-2 text-base-content/60">
              <span class="loading loading-spinner loading-sm"></span>
              <span>{{ t('aiAssistant.turnLogsLoadingDetail', '正在加载详情') }}</span>
            </div>

            <div v-else-if="getDetailError(entry.session_id)" class="alert alert-error text-sm">
              {{ getDetailError(entry.session_id) }}
            </div>

            <template v-else-if="getDetail(entry.session_id)">
              <section v-if="getDetail(entry.session_id)?.summary?.reasoning" class="space-y-2">
                <h4 class="font-medium">{{ t('aiAssistant.turnLogsReasoning', '推理内容') }}</h4>
                <pre class="bg-base-200/60 rounded-lg p-3 whitespace-pre-wrap break-words">{{ stringify(getDetail(entry.session_id)?.summary?.reasoning) }}</pre>
              </section>

              <div class="grid grid-cols-1 xl:grid-cols-2 gap-4">
                <section class="space-y-2">
                  <h4 class="font-medium">{{ t('aiAssistant.turnLogsUserPrompt', '用户输入') }}</h4>
                  <pre class="bg-base-200/60 rounded-lg p-3 whitespace-pre-wrap break-words">{{ stringify(getDetail(entry.session_id)?.summary?.user_request) }}</pre>
                </section>
                <section class="space-y-2">
                  <h4 class="font-medium">{{ t('aiAssistant.turnLogsAssistantResponse', '助手回复') }}</h4>
                  <pre class="bg-base-200/60 rounded-lg p-3 whitespace-pre-wrap break-words">{{ stringify(getDetail(entry.session_id)?.summary?.assistant_response) }}</pre>
                </section>
              </div>

              <section v-if="Array.isArray(getDetail(entry.session_id)?.summary?.tool_calls) && getDetail(entry.session_id)?.summary?.tool_calls.length > 0" class="space-y-3">
              <h4 class="font-medium">{{ t('aiAssistant.turnLogsToolCalls', '工具调用详情') }}</h4>
              <div
                v-for="(toolCall, index) in getDetail(entry.session_id)?.summary?.tool_calls"
                :key="`${entry.session_id}-tool-${index}`"
                class="rounded-lg border border-base-300 bg-base-200/40 p-3 space-y-2"
              >
                <div class="flex flex-wrap items-center gap-2">
                  <span class="badge badge-outline">{{ toolCall.tool_name || `tool-${index + 1}` }}</span>
                  <span class="badge" :class="toolCall.success === false ? 'badge-error' : 'badge-success'">
                    {{ toolCall.success === false ? 'failed' : 'success' }}
                  </span>
                  <span class="text-base-content/60 text-xs">{{ toolCall.tool_call_id }}</span>
                </div>
                <div class="grid grid-cols-1 xl:grid-cols-2 gap-3">
                  <div>
                    <div class="text-xs uppercase tracking-wide text-base-content/60 mb-1">arguments</div>
                    <pre class="bg-base-100 rounded p-3 whitespace-pre-wrap break-words">{{ pretty(toolCall.arguments ?? toolCall.arguments_raw) }}</pre>
                  </div>
                  <div>
                    <div class="text-xs uppercase tracking-wide text-base-content/60 mb-1">result</div>
                    <pre class="bg-base-100 rounded p-3 whitespace-pre-wrap break-words">{{ pretty(toolCall.result ?? toolCall.result_raw) }}</pre>
                  </div>
                </div>
              </div>
              </section>
            </template>
          </div>
        </details>
      </div>
    </div>
    <form method="dialog" class="modal-backdrop" @click.prevent="closeDialog">
      <button>close</button>
    </form>
  </dialog>
</template>

<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { invoke } from '@tauri-apps/api/core'
import {
  getAiTurnLogDetail,
  getAiTurnLogs,
  type AiTurnLogEntry,
  type AiTurnLogSummaryEntry,
} from '@/api/aiLogs'
import SearchableSelect from '@/components/SearchableSelect.vue'

interface ConversationOptionSource {
  id: string
  title?: string | null
  updated_at?: string | null
}

const props = defineProps<{
  modelValue: boolean
  activeSessionId?: string | null
  conversationOptions?: ConversationOptionSource[]
}>()

const emit = defineEmits<{
  (e: 'update:modelValue', value: boolean): void
  (e: 'openConversation', payload: AiTurnLogSummaryEntry): void
}>()

const { t } = useI18n()
const dialogRef = ref<HTMLDialogElement | null>(null)
const logs = ref<AiTurnLogSummaryEntry[]>([])
const detailBySessionId = ref<Record<string, AiTurnLogEntry | null>>({})
const detailLoadingBySessionId = ref<Record<string, boolean>>({})
const detailErrorBySessionId = ref<Record<string, string>>({})
const loading = ref(false)
const error = ref('')
const date = ref(new Date().toISOString().slice(0, 10))
const conversationId = ref('')
const sessionId = ref('')
const limit = ref(50)
const initializedSessionFilter = ref(false)
const availableConversations = ref<ConversationOptionSource[]>([])

const normalizedActiveSessionId = computed(() => String(props.activeSessionId || '').trim())
const normalizedConversationFilter = computed(() => String(conversationId.value || '').trim())

const formatConversationLabel = (conversation: ConversationOptionSource) => {
  const id = String(conversation.id || '').trim()
  const title = String(conversation.title || '').trim() || t('agent.unnamedConversation')
  return title || id
}

const mergeConversations = (...lists: Array<ConversationOptionSource[] | undefined>) => {
  const merged = new Map<string, ConversationOptionSource>()

  for (const list of lists) {
    for (const item of list || []) {
      const id = String(item?.id || '').trim()
      if (!id) continue
      const existing = merged.get(id)
      if (!existing) {
        merged.set(id, { id, title: item.title, updated_at: item.updated_at })
        continue
      }

      if (!existing.title && item.title) {
        existing.title = item.title
      }
      if (!existing.updated_at && item.updated_at) {
        existing.updated_at = item.updated_at
      }
    }
  }

  return Array.from(merged.values()).sort((a, b) => {
    const aTime = new Date(String(a.updated_at || 0)).getTime()
    const bTime = new Date(String(b.updated_at || 0)).getTime()
    return bTime - aTime
  })
}

const conversationOptions = computed(() => {
  const merged = mergeConversations(props.conversationOptions, availableConversations.value)
  const options = [
    {
      value: '',
      label: t('aiAssistant.turnLogsConversationAll', '全部会话'),
      description: '',
    },
    ...merged.map(item => ({
      value: item.id,
      label: formatConversationLabel(item),
      description: '',
    })),
  ]

  if (
    normalizedConversationFilter.value &&
    !options.some(option => option.value === normalizedConversationFilter.value)
  ) {
    options.push({
      value: normalizedConversationFilter.value,
      label: normalizedConversationFilter.value,
      description: normalizedConversationFilter.value,
    })
  }

  return options
})

const closeDialog = () => {
  dialogRef.value?.close()
}

const handleClose = () => {
  emit('update:modelValue', false)
}

const applyActiveSessionFilter = () => {
  if (normalizedActiveSessionId.value) {
    conversationId.value = normalizedActiveSessionId.value
  }
}

const loadConversationOptions = async () => {
  try {
    const conversations = await invoke<ConversationOptionSource[]>('get_ai_conversations')
    availableConversations.value = Array.isArray(conversations) ? conversations : []
  } catch (error) {
    console.error('Failed to load conversation options for turn logs:', error)
  }
}

const openConversation = (entry: AiTurnLogSummaryEntry) => {
  emit('openConversation', entry)
  closeDialog()
}

const getDetail = (sessionId: string) => detailBySessionId.value[sessionId] || null
const isDetailLoading = (sessionId: string) => detailLoadingBySessionId.value[sessionId] === true
const getDetailError = (sessionId: string) => detailErrorBySessionId.value[sessionId] || ''

const ensureDetailLoaded = async (entry: AiTurnLogSummaryEntry) => {
  if (detailBySessionId.value[entry.session_id] || detailLoadingBySessionId.value[entry.session_id]) {
    return
  }
  detailLoadingBySessionId.value[entry.session_id] = true
  detailErrorBySessionId.value[entry.session_id] = ''
  try {
    const detail = await getAiTurnLogDetail(date.value, entry.session_id)
    detailBySessionId.value[entry.session_id] = detail
    if (!detail) {
      detailErrorBySessionId.value[entry.session_id] = t('aiAssistant.turnLogsDetailMissing', '未找到该条 turn 的详情')
    }
  } catch (e: any) {
    detailErrorBySessionId.value[entry.session_id] = e?.message || String(e)
  } finally {
    detailLoadingBySessionId.value[entry.session_id] = false
  }
}

const handleToggle = async (entry: AiTurnLogSummaryEntry, event: Event) => {
  const target = event.target as HTMLDetailsElement | null
  if (!target?.open) return
  await ensureDetailLoaded(entry)
}

const loadLogs = async () => {
  loading.value = true
  error.value = ''
  detailBySessionId.value = {}
  detailLoadingBySessionId.value = {}
  detailErrorBySessionId.value = {}
  try {
    logs.value = await getAiTurnLogs({
      date: date.value,
      conversationId: conversationId.value || undefined,
      sessionId: sessionId.value || undefined,
      limit: limit.value || 50,
    })
  } catch (e: any) {
    error.value = e?.message || String(e)
  } finally {
    loading.value = false
  }
}

const stringify = (value: unknown) => {
  if (typeof value === 'string') return value
  if (value == null) return ''
  return JSON.stringify(value, null, 2)
}

const pretty = (value: unknown) => {
  if (typeof value === 'string') {
    try {
      return JSON.stringify(JSON.parse(value), null, 2)
    } catch {
      return value
    }
  }
  return stringify(value)
}

const formatTime = (value: string) => {
  const dateObj = new Date(value)
  if (Number.isNaN(dateObj.getTime())) return value
  return dateObj.toLocaleString()
}

const formatDuration = (value: unknown) => {
  const ms = Number(value || 0)
  if (!Number.isFinite(ms) || ms <= 0) return '-'
  if (ms < 1000) return `${ms} ms`
  return `${(ms / 1000).toFixed(2)} s`
}

const formatTokens = (input?: unknown, output?: unknown) => {
  const inCount = Number(input || 0)
  const outCount = Number(output || 0)
  return `${inCount}/${outCount}`
}

const statusBadgeClass = (status?: string) => {
  switch (status) {
    case 'completed':
      return 'badge-success'
    case 'error':
    case 'empty_response':
      return 'badge-error'
    case 'interrupted':
      return 'badge-warning'
    default:
      return 'badge-ghost'
  }
}

watch(
  () => props.modelValue,
  async (open) => {
    if (open) {
      if (!dialogRef.value?.open) dialogRef.value?.showModal()
      if (!initializedSessionFilter.value && normalizedActiveSessionId.value) {
        conversationId.value = normalizedActiveSessionId.value
        initializedSessionFilter.value = true
      }
      await Promise.all([loadConversationOptions(), loadLogs()])
      return
    }

    if (dialogRef.value?.open) {
      dialogRef.value.close()
    }
  }
)
</script>
