<template>
  <div class="card bg-base-100 border border-base-300 shadow-sm">
    <div class="card-body gap-4">
      <div class="flex flex-col gap-3 lg:flex-row lg:items-center lg:justify-between">
        <div>
          <h3 class="card-title text-base">
            <i class="fas fa-terminal text-primary mr-2"></i>
            Browser Shell Bridge
          </h3>
          <p class="text-sm text-base-content/70">
            查看扩展捕获到的第三方 WebSocket Shell 会话、最近帧和待审批写入请求。
          </p>
        </div>
        <div class="flex flex-wrap items-center gap-2">
          <span class="badge badge-ghost">会话 {{ sessions.length }}</span>
          <span class="badge badge-success">在线 {{ connectedSessionCount }}</span>
          <span class="badge badge-warning">待审批 {{ pendingApprovalCount }}</span>
          <span
            v-if="boundSessionId"
            class="badge badge-primary"
          >
            已绑定 {{ boundSessionId }}
          </span>
          <span
            v-if="boundSessionId && directWriteEnabled"
            class="badge badge-warning"
          >
            AI 直写已授权
          </span>
          <button
            v-if="showToolTestActions"
            class="btn btn-sm btn-outline btn-primary"
            @click="openToolTest({ action: 'list_sessions' })"
          >
            <i class="fas fa-flask mr-1"></i>
            测试 list_sessions
          </button>
          <button
            class="btn btn-sm btn-outline"
            :disabled="refreshLoading"
            @click="refreshAll(false)"
          >
            <i :class="['fas', refreshLoading ? 'fa-spinner fa-spin' : 'fa-sync-alt']"></i>
            刷新
          </button>
        </div>
      </div>

      <div v-if="initialLoading" class="flex items-center gap-2 text-sm text-base-content/70">
        <span class="loading loading-spinner loading-sm"></span>
        <span>正在加载浏览器 shell bridge...</span>
      </div>
      <div v-else-if="sessions.length === 0" class="rounded-lg border border-dashed border-base-300 px-4 py-6 text-sm text-base-content/60">
        当前还没有发现第三方浏览器 shell 会话。先在目标网页中打开终端，并确保 Chrome 扩展的 shell bridge 已启用。
      </div>
      <div v-else class="grid gap-4 xl:grid-cols-[360px_minmax(0,1fr)]">
        <div class="space-y-3">
          <div class="text-sm font-medium text-base-content/70">已发现会话</div>
          <div class="max-h-[720px] space-y-2 overflow-y-auto pr-1">
            <button
              v-for="session in sessions"
              :key="session.id"
              class="w-full rounded-xl border p-3 text-left transition-colors"
              :class="session.id === selectedSessionId ? 'border-primary bg-primary/5' : 'border-base-300 bg-base-100 hover:border-primary/40'"
              @click="selectSession(session.id)"
            >
              <div class="flex items-start justify-between gap-2">
                <div class="min-w-0">
                  <div class="truncate font-medium">
                    {{ session.pageTitle || session.pageUrl }}
                  </div>
                  <div class="mt-1 text-xs text-base-content/60 truncate">
                    {{ session.pageUrl }}
                  </div>
                </div>
                <div class="flex flex-col items-end gap-1">
                  <span :class="session.connected ? 'badge badge-success badge-sm' : 'badge badge-ghost badge-sm'">
                    {{ session.connected ? 'connected' : 'closed' }}
                  </span>
                  <span :class="session.writable ? 'badge badge-primary badge-sm' : 'badge badge-ghost badge-sm'">
                    {{ session.writable ? 'writable' : 'read-only' }}
                  </span>
                </div>
              </div>
              <div class="mt-2 flex flex-wrap gap-1 text-[11px] text-base-content/60">
                <span class="badge badge-ghost badge-xs">{{ session.terminalKind }}</span>
                <span v-if="session.protocol" class="badge badge-ghost badge-xs">{{ session.protocol }}</span>
                <span class="badge badge-ghost badge-xs">tab {{ session.tabId ?? '-' }}</span>
                <span class="badge badge-ghost badge-xs">frame {{ session.frameId ?? '-' }}</span>
              </div>
              <div class="mt-2 text-[11px] text-base-content/50 truncate">
                {{ session.wsUrl }}
              </div>
              <div class="mt-1 text-[11px] text-base-content/50">
                {{ formatTimestamp(session.lastSeenAt) }}
              </div>
            </button>
          </div>
        </div>

        <div v-if="selectedSession" class="space-y-4">
          <div class="rounded-xl border border-base-300 bg-base-100 p-4">
            <div class="flex flex-col gap-3 lg:flex-row lg:items-start lg:justify-between">
              <div class="min-w-0">
                <div class="flex items-center gap-2">
                  <h4 class="font-semibold">{{ selectedSession.pageTitle || selectedSession.pageUrl }}</h4>
                  <span :class="selectedSession.connected ? 'badge badge-success badge-sm' : 'badge badge-ghost badge-sm'">
                    {{ selectedSession.connected ? 'connected' : 'closed' }}
                  </span>
                </div>
                <div class="mt-2 text-sm text-base-content/70 break-all">
                  {{ selectedSession.pageUrl }}
                </div>
                <div class="mt-1 text-xs text-base-content/50 break-all">
                  {{ selectedSession.wsUrl }}
                </div>
                <div class="mt-3 flex flex-wrap gap-2 text-xs">
                  <span class="badge badge-ghost">session {{ selectedSession.id }}</span>
                  <span class="badge badge-ghost">{{ selectedSession.terminalKind }}</span>
                  <span v-if="selectedSession.protocol" class="badge badge-ghost">{{ selectedSession.protocol }}</span>
                </div>
              </div>
              <div class="flex flex-wrap gap-2">
                <button
                  class="btn btn-xs"
                  :class="selectedSession.id === boundSessionId ? 'btn-primary' : 'btn-outline btn-primary'"
                  @click="bindSelectedSession(selectedSession.id)"
                >
                  <i class="fas fa-link mr-1"></i>
                  {{ selectedSession.id === boundSessionId ? '已绑定到当前对话' : '绑定到当前对话' }}
                </button>
                <button
                  v-if="selectedSession.id === boundSessionId"
                  class="btn btn-xs btn-outline"
                  @click="clearBoundSession"
                >
                  <i class="fas fa-link-slash mr-1"></i>
                  解除绑定
                </button>
                <label
                  v-if="selectedSession.id === boundSessionId"
                  class="label cursor-pointer gap-2 rounded-lg border border-warning/30 bg-warning/5 px-2 py-1"
                >
                  <span class="label-text text-xs">允许 AI 直接写入</span>
                  <input
                    :checked="directWriteEnabled"
                    type="checkbox"
                    class="toggle toggle-warning toggle-xs"
                    @change="toggleDirectWrite(($event.target as HTMLInputElement).checked)"
                  />
                </label>
                <button
                  v-if="showToolTestActions"
                  class="btn btn-xs btn-outline btn-primary"
                  @click="openToolTest({
                    action: 'read_frames',
                    session_id: selectedSession.id,
                    limit: 80,
                  })"
                >
                  <i class="fas fa-flask mr-1"></i>
                  测试 read_frames
                </button>
                <button class="btn btn-xs btn-outline" @click="copySessionId(selectedSession.id)">
                  <i class="fas fa-copy mr-1"></i>
                  复制会话 ID
                </button>
                <button class="btn btn-xs btn-outline" @click="loadFrames(selectedSession.id, false)">
                  <i :class="['fas', framesLoading ? 'fa-spinner fa-spin' : 'fa-stream']"></i>
                  刷新帧
                </button>
              </div>
            </div>
          </div>

          <div class="grid gap-4 2xl:grid-cols-[minmax(0,1fr)_380px]">
            <div class="rounded-xl border border-base-300 bg-base-100 p-4">
              <div class="flex items-center justify-between gap-3">
                <div>
                  <h4 class="font-semibold">最近终端帧</h4>
                  <p class="text-sm text-base-content/60">优先展示完整帧内容，便于确认命令是否被截断、串行或异常回显。</p>
                </div>
                <div class="flex items-center gap-2">
                  <span class="badge badge-ghost">{{ frames.length }} 帧</span>
                  <label class="label cursor-pointer gap-2 py-0">
                    <span class="label-text text-xs">完整帧</span>
                    <input
                      v-model="preferFullFrameContent"
                      type="checkbox"
                      class="toggle toggle-primary toggle-xs"
                    />
                  </label>
                </div>
              </div>
              <div v-if="framesLoading" class="mt-4 flex items-center gap-2 text-sm text-base-content/70">
                <span class="loading loading-spinner loading-sm"></span>
                <span>正在加载最近帧...</span>
              </div>
              <div v-else-if="frames.length === 0" class="mt-4 text-sm text-base-content/60">
                当前会话还没有可显示的最近帧。
              </div>
              <div v-else class="mt-4 max-h-[520px] space-y-2 overflow-y-auto pr-1">
                <div
                  v-for="frame in frames"
                  :key="frame.id"
                  class="rounded-lg border border-base-300 bg-base-200/40 p-3"
                >
                  <div class="flex flex-wrap items-center gap-2 text-xs">
                    <span :class="frame.direction === 'out' ? 'badge badge-warning badge-sm' : 'badge badge-info badge-sm'">
                      {{ frame.direction }}
                    </span>
                    <span class="badge badge-ghost badge-sm">{{ frame.frameType }}</span>
                    <span
                      v-if="frameHasFullPayload(frame)"
                      class="badge badge-primary badge-sm"
                    >
                      payload
                    </span>
                    <span class="text-base-content/50">{{ formatTimestamp(frame.receivedAt) }}</span>
                  </div>
                  <pre class="mt-2 whitespace-pre-wrap break-words font-mono text-xs text-base-content/80">{{ frameContent(frame) }}</pre>
                </div>
              </div>
            </div>

            <div class="space-y-4">
              <div class="rounded-xl border border-base-300 bg-base-100 p-4">
                <div class="flex items-center justify-between gap-3">
                  <div>
                    <h4 class="font-semibold">排队写入</h4>
                    <p class="text-sm text-base-content/60">直接向当前第三方 shell 会话写入命令，默认先进入审批。</p>
                  </div>
                  <label class="label cursor-pointer gap-2 py-0">
                    <span class="label-text text-sm">需要审批</span>
                    <input v-model="requiresApproval" type="checkbox" class="toggle toggle-warning toggle-sm" />
                  </label>
                </div>
                <div class="mt-3 space-y-3">
                  <textarea
                    v-model="writeDraft"
                    class="textarea textarea-bordered h-32 w-full font-mono text-sm"
                    placeholder="例如: ls -la\n"
                    spellcheck="false"
                  ></textarea>
                  <div class="flex justify-end gap-2">
                    <button
                      v-if="showToolTestActions"
                      class="btn btn-sm btn-outline btn-primary"
                      :disabled="!canQueueWrite"
                      @click="openToolTest({
                        action: 'queue_write',
                        session_id: selectedSession.id,
                        input_text: writeDraft,
                        requires_approval: requiresApproval,
                      })"
                    >
                      <i class="fas fa-flask mr-1"></i>
                      测试 queue_write
                    </button>
                    <button
                      class="btn btn-primary btn-sm"
                      :disabled="queueSubmitting || !canQueueWrite"
                      @click="queueWrite"
                    >
                      <i :class="['fas', queueSubmitting ? 'fa-spinner fa-spin' : 'fa-paper-plane']"></i>
                      提交写入
                    </button>
                  </div>
                </div>
              </div>

              <div class="rounded-xl border border-base-300 bg-base-100 p-4">
                <div class="flex items-center justify-between gap-3">
                  <div>
                    <h4 class="font-semibold">待审批写入</h4>
                    <p class="text-sm text-base-content/60">只有审批通过后，扩展才会把输入发回原始 WebSocket 连接。</p>
                  </div>
                  <span class="badge badge-warning">{{ selectedPendingRequests.length }}</span>
                </div>
                <div v-if="selectedPendingRequests.length === 0" class="mt-4 text-sm text-base-content/60">
                  当前会话没有待审批写入。
                </div>
                <div v-else class="mt-4 space-y-3">
                  <div
                    v-for="request in selectedPendingRequests"
                    :key="request.requestId"
                    class="rounded-lg border border-warning/30 bg-warning/5 p-3"
                  >
                    <div class="flex items-center justify-between gap-3">
                      <div class="text-xs text-base-content/60">
                        {{ formatTimestamp(request.createdAt) }}
                      </div>
                      <div class="flex gap-2">
                        <button
                          v-if="showToolTestActions"
                          class="btn btn-xs btn-outline btn-primary"
                          @click="openToolTest({
                            action: 'respond_write',
                            request_id: request.requestId,
                            allowed: true,
                          })"
                        >
                          <i class="fas fa-flask mr-1"></i>
                          测试
                        </button>
                        <button
                          class="btn btn-xs btn-success"
                          :disabled="requestActionId === request.requestId"
                          @click="respondWrite(request.requestId, true)"
                        >
                          <i :class="['fas', requestActionId === request.requestId ? 'fa-spinner fa-spin' : 'fa-check']"></i>
                          通过
                        </button>
                        <button
                          class="btn btn-xs btn-error"
                          :disabled="requestActionId === request.requestId"
                          @click="respondWrite(request.requestId, false)"
                        >
                          <i class="fas fa-xmark"></i>
                          拒绝
                        </button>
                      </div>
                    </div>
                    <pre class="mt-2 whitespace-pre-wrap break-words font-mono text-xs">{{ request.inputText }}</pre>
                  </div>
                </div>
              </div>

              <div class="rounded-xl border border-base-300 bg-base-100 p-4">
                <div class="flex items-center justify-between gap-3">
                  <div>
                    <h4 class="font-semibold">最近写入请求</h4>
                    <p class="text-sm text-base-content/60">查看当前会话的请求状态和失败原因。</p>
                  </div>
                  <span class="badge badge-ghost">{{ selectedRequests.length }}</span>
                </div>
                <div v-if="selectedRequests.length === 0" class="mt-4 text-sm text-base-content/60">
                  当前会话还没有写入请求。
                </div>
                <div v-else class="mt-4 max-h-[320px] space-y-2 overflow-y-auto pr-1">
                  <div
                    v-for="request in selectedRequests"
                    :key="request.requestId"
                    class="rounded-lg border border-base-300 bg-base-200/40 p-3"
                  >
                    <div class="flex items-center justify-between gap-2">
                      <span :class="writeStatusBadgeClass(request.status)">{{ request.status }}</span>
                      <span class="text-xs text-base-content/50">{{ formatTimestamp(request.updatedAt) }}</span>
                    </div>
                    <pre class="mt-2 whitespace-pre-wrap break-words font-mono text-xs">{{ request.inputText }}</pre>
                    <div v-if="request.error" class="mt-2 text-xs text-error break-all">
                      {{ request.error }}
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { dialog } from '@/composables/useDialog'
import { useBrowserShell } from '@/composables/useBrowserShell'
import {
  browserShellFrameHasFullPayload,
  resolveBrowserShellFrameContent,
} from './browserShellFrameContent'
import {
  getBrowserShellFrames,
  listBrowserShellSessions,
  listBrowserShellWriteRequests,
  queueBrowserShellWrite,
  respondBrowserShellWrite,
  type BrowserShellFrame,
  type BrowserShellSession,
  type BrowserShellWriteRequest,
  type BrowserShellWriteStatus,
} from '@/api/browserShell'

const emit = defineEmits<{
  (e: 'open-tool-test', initialParams: Record<string, unknown>): void
}>()
const props = withDefaults(defineProps<{
  autoAuthorizeDirectWriteOnBind?: boolean
  showToolTestActions?: boolean
}>(), {
  autoAuthorizeDirectWriteOnBind: false,
  showToolTestActions: true,
})
const browserShell = useBrowserShell()

const sessions = ref<BrowserShellSession[]>([])
const frames = ref<BrowserShellFrame[]>([])
const writeRequests = ref<BrowserShellWriteRequest[]>([])
const selectedSessionId = ref('')
const writeDraft = ref('')
const requiresApproval = ref(true)
const preferFullFrameContent = ref(true)
const initialLoading = ref(true)
const refreshLoading = ref(false)
const framesLoading = ref(false)
const queueSubmitting = ref(false)
const requestActionId = ref('')

let refreshTimer: number | null = null
let refreshInFlight = false

const selectedSession = computed(() =>
  sessions.value.find(session => session.id === selectedSessionId.value) ?? null,
)
const boundSessionId = computed(() => browserShell.currentSessionId.value)
const directWriteEnabled = computed(() => browserShell.directWriteEnabled.value)

const connectedSessionCount = computed(() =>
  sessions.value.filter(session => session.connected).length,
)

const pendingApprovalCount = computed(() =>
  writeRequests.value.filter(request => request.status === 'pending_approval').length,
)

const selectedRequests = computed(() =>
  writeRequests.value
    .filter(request => request.sessionId === selectedSessionId.value)
    .slice(0, 12),
)

const selectedPendingRequests = computed(() =>
  writeRequests.value
    .filter(request => request.sessionId === selectedSessionId.value && request.status === 'pending_approval')
    .slice(0, 12),
)

const canQueueWrite = computed(() => {
  if (!selectedSession.value) return false
  if (!selectedSession.value.writable) return false
  return writeDraft.value.trim().length > 0
})

function formatTimestamp(value: string | null | undefined) {
  if (!value) return '-'
  const timestamp = new Date(value)
  if (Number.isNaN(timestamp.getTime())) return value
  return timestamp.toLocaleString()
}

function frameHasFullPayload(frame: BrowserShellFrame) {
  return browserShellFrameHasFullPayload(frame)
}

function frameContent(frame: BrowserShellFrame) {
  return resolveBrowserShellFrameContent(frame, preferFullFrameContent.value)
}

function writeStatusBadgeClass(status: BrowserShellWriteStatus) {
  const classMap: Record<BrowserShellWriteStatus, string> = {
    pending_approval: 'badge badge-warning badge-sm',
    queued: 'badge badge-info badge-sm',
    dispatching: 'badge badge-secondary badge-sm',
    delivered: 'badge badge-success badge-sm',
    failed: 'badge badge-error badge-sm',
    rejected: 'badge badge-ghost badge-sm',
  }
  return classMap[status]
}

function openToolTest(initialParams: Record<string, unknown>) {
  if (!props.showToolTestActions) return
  emit('open-tool-test', initialParams)
}

function bindSelectedSession(sessionId: string) {
  browserShell.bindSession(sessionId)
  if (props.autoAuthorizeDirectWriteOnBind) {
    browserShell.setDirectWriteEnabled(true)
    dialog.toast.success(`已绑定 browser shell 会话并授权 AI 直接写入: ${sessionId}`)
    return
  }
  dialog.toast.success(`已绑定 browser shell 会话到当前对话上下文: ${sessionId}`)
}

function clearBoundSession() {
  browserShell.clearSession()
  dialog.toast.success('已解除当前 browser shell 会话绑定')
}

function toggleDirectWrite(enabled: boolean) {
  browserShell.setDirectWriteEnabled(enabled)
  dialog.toast.success(
    enabled
      ? '已授权 AI 对当前 browser shell 直接写入'
      : '已恢复 browser shell 写入审批',
  )
}

function reconcileSelectedSession() {
  if (sessions.value.length === 0) {
    selectedSessionId.value = ''
    frames.value = []
    if (boundSessionId.value) {
      browserShell.clearSession()
    }
    return
  }
  const existing = sessions.value.some(session => session.id === selectedSessionId.value)
  if (!existing) {
    selectedSessionId.value = sessions.value[0].id
  }
  if (boundSessionId.value && !sessions.value.some(session => session.id === boundSessionId.value)) {
    browserShell.clearSession()
  }
}

async function loadFrames(sessionId: string, silent: boolean) {
  const normalizedSessionId = String(sessionId || '').trim()
  if (!normalizedSessionId) {
    frames.value = []
    return
  }

  framesLoading.value = true
  try {
    const result = await getBrowserShellFrames(normalizedSessionId, 80)
    if (selectedSessionId.value === normalizedSessionId) {
      frames.value = result.frames
    }
  } catch (error) {
    console.error('Failed to load browser shell frames:', error)
    if (!silent) {
      dialog.toast.error(`加载浏览器 shell 帧失败：${String(error)}`)
    }
  } finally {
    framesLoading.value = false
  }
}

function selectSession(sessionId: string) {
  if (selectedSessionId.value === sessionId) return
  selectedSessionId.value = sessionId
  void loadFrames(sessionId, true)
}

async function refreshAll(silent: boolean) {
  if (refreshInFlight) return
  refreshInFlight = true
  refreshLoading.value = true
  try {
    const [nextSessions, nextRequests] = await Promise.all([
      listBrowserShellSessions(),
      listBrowserShellWriteRequests(),
    ])
    sessions.value = nextSessions
    writeRequests.value = nextRequests
    reconcileSelectedSession()
    if (selectedSessionId.value) {
      await loadFrames(selectedSessionId.value, true)
    } else {
      frames.value = []
    }
  } catch (error) {
    console.error('Failed to refresh browser shell bridge data:', error)
    if (!silent) {
      dialog.toast.error(`刷新浏览器 shell bridge 失败：${String(error)}`)
    }
  } finally {
    refreshLoading.value = false
    initialLoading.value = false
    refreshInFlight = false
  }
}

async function queueWrite() {
  if (!selectedSession.value || !canQueueWrite.value) return
  queueSubmitting.value = true
  try {
    const request = await queueBrowserShellWrite(
      selectedSession.value.id,
      writeDraft.value,
      requiresApproval.value,
    )
    writeDraft.value = ''
    writeRequests.value = [request, ...writeRequests.value.filter(item => item.requestId !== request.requestId)]
    dialog.toast.success(
      request.status === 'pending_approval'
        ? '写入请求已进入审批队列'
        : '写入请求已排队发送',
    )
    await refreshAll(true)
  } catch (error) {
    console.error('Failed to queue browser shell write:', error)
    dialog.toast.error(`提交浏览器 shell 写入失败：${String(error)}`)
  } finally {
    queueSubmitting.value = false
  }
}

async function respondWrite(requestId: string, allowed: boolean) {
  requestActionId.value = requestId
  try {
    await respondBrowserShellWrite(requestId, allowed)
    dialog.toast.success(allowed ? '已批准写入请求' : '已拒绝写入请求')
    await refreshAll(true)
  } catch (error) {
    console.error('Failed to respond browser shell write:', error)
    dialog.toast.error(`处理写入请求失败：${String(error)}`)
  } finally {
    requestActionId.value = ''
  }
}

async function copySessionId(sessionId: string) {
  try {
    await navigator.clipboard.writeText(sessionId)
    dialog.toast.success(`已复制会话 ID: ${sessionId}`)
  } catch (error) {
    console.error('Failed to copy browser shell session id:', error)
    dialog.toast.error('复制会话 ID 失败')
  }
}

onMounted(() => {
  void refreshAll(true)
  refreshTimer = window.setInterval(() => {
    void refreshAll(true)
  }, 2000)
})

onBeforeUnmount(() => {
  if (refreshTimer !== null) {
    window.clearInterval(refreshTimer)
    refreshTimer = null
  }
})
</script>
