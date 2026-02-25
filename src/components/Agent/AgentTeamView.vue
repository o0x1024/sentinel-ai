<template>
  <div class="agent-team-view flex flex-col h-full overflow-hidden bg-gradient-to-br from-base-100 to-base-200">
    <!-- Header -->
    <div class="team-header px-4 py-2.5 border-b border-base-300 bg-base-100/80 backdrop-blur-sm flex items-center justify-between">
      <div class="flex items-center gap-3">
        <!-- Back to single mode -->
        <button
          class="btn btn-xs btn-ghost gap-1 text-base-content/60 hover:text-base-content"
          @click="emit('switch-mode', 'single')"
          title="退出 Team 模式"
        >
          <i class="fas fa-arrow-left text-xs"></i>
          <span class="text-xs">单 Agent</span>
        </button>
        <div class="w-px h-4 bg-base-300"></div>
        <!-- Team badge -->
        <div class="flex items-center gap-1.5 px-2 py-0.5 rounded-full bg-primary/15 border border-primary/30">
          <i class="fas fa-users text-primary text-xs"></i>
          <span class="text-xs font-semibold text-primary">Team 模式</span>
        </div>
        <!-- Session name -->
        <span v-if="session" class="text-sm font-medium text-base-content/70 max-w-52 truncate">
          {{ session.name }}
        </span>
        <!-- State badge -->
        <span
          v-if="session"
          class="badge badge-xs"
          :class="stateBadgeClass(session.state)"
        >{{ stateLabel(session.state) }}</span>
      </div>

      <div class="flex items-center gap-2">
        <button
          v-if="session && session.state === 'PENDING'"
          class="btn btn-xs btn-primary gap-1"
          @click="handleStartExistingSession"
          :disabled="isStarting"
          title="启动当前 Team 会话"
        >
          <i class="fas fa-play text-xs" v-if="!isStarting"></i>
          <i class="fas fa-spinner fa-spin text-xs" v-else></i>
          <span>{{ isStarting ? '启动中' : '开始运行' }}</span>
        </button>
        <!-- Round indicator -->
        <div v-if="session" class="flex items-center gap-1 text-xs text-base-content/50">
          <i class="fas fa-sync-alt text-xs"></i>
          <span>第 {{ session.current_round }}/{{ session.max_rounds }} 轮</span>
        </div>
        <!-- Template library button -->
        <button
          class="btn btn-xs gap-1"
          :class="showTemplateLibrary ? 'btn-accent' : 'btn-ghost text-base-content/50'"
          @click="showTemplateLibrary = !showTemplateLibrary"
          id="template-library-toggle-btn"
        >
          <i class="fas fa-layer-group text-xs"></i>
          <span>模板库</span>
        </button>
        <!-- Session history button -->
        <button
          class="btn btn-xs gap-1"
          :class="showSessionHistory ? 'btn-info' : 'btn-ghost text-base-content/50'"
          @click="toggleSessionHistory"
          id="team-session-history-toggle-btn"
        >
          <i class="fas fa-history text-xs"></i>
          <span>历史</span>
        </button>
        <!-- Artifacts button -->
        <button
          v-if="artifacts.length > 0"
          class="btn btn-xs gap-1"
          :class="showArtifacts ? 'btn-primary' : 'btn-ghost text-primary'"
          @click="showArtifacts = !showArtifacts"
        >
          <i class="fas fa-file-alt text-xs"></i>
          <span>产物</span>
          <span class="badge badge-xs badge-primary">{{ artifacts.length }}</span>
        </button>
        <!-- Blackboard button -->
        <button
          class="btn btn-xs gap-1"
          :class="showBlackboard ? 'btn-secondary' : 'btn-ghost text-secondary'"
          @click="showBlackboard = !showBlackboard"
        >
          <i class="fas fa-chalkboard text-xs"></i>
          <span>白板</span>
          <span v-if="blackboard.length > 0" class="badge badge-xs badge-secondary">{{ blackboard.length }}</span>
        </button>
      </div>
    </div>

    <!-- Template Library Drawer (Teleport to avoid z-index issues) -->
    <Teleport to="body">
      <Transition name="slide-library">
        <div
          v-if="showTemplateLibrary"
          class="fixed inset-0 z-[80] flex"
        >
          <div class="absolute inset-0 bg-black/30" @click="showTemplateLibrary = false"></div>
          <div class="relative ml-auto w-full max-w-md h-full bg-base-100 shadow-2xl flex flex-col">
            <AgentTeamTemplateLibrary
              :conversation-id="conversationId"
              @close="showTemplateLibrary = false"
              @session-created="handleLibrarySessionCreated"
            />
          </div>
        </div>
      </Transition>
    </Teleport>

    <!-- Session History Drawer -->
    <Teleport to="body">
      <Transition name="slide-library">
        <div
          v-if="showSessionHistory"
          class="fixed inset-0 z-[80] flex"
        >
          <div class="absolute inset-0 bg-black/30" @click="showSessionHistory = false"></div>
          <div class="relative ml-auto w-full max-w-md h-full bg-base-100 shadow-2xl flex flex-col">
            <div class="flex items-center justify-between px-4 py-3 border-b border-base-300 bg-base-100/80 backdrop-blur-sm">
              <div class="flex items-center gap-2">
                <i class="fas fa-history text-info"></i>
                <h2 class="text-sm font-bold text-base-content">Team 会话历史</h2>
                <span class="badge badge-sm badge-info">{{ sessionHistory.length }}</span>
              </div>
              <div class="flex items-center gap-2">
                <button
                  class="btn btn-xs btn-ghost"
                  @click="loadSessionHistory"
                  title="刷新"
                >
                  <i class="fas fa-rotate-right"></i>
                </button>
                <button
                  class="btn btn-xs btn-ghost"
                  @click="showSessionHistory = false"
                  title="关闭"
                >
                  <i class="fas fa-times"></i>
                </button>
              </div>
            </div>

            <div class="flex-1 overflow-y-auto p-3 space-y-2">
              <div v-if="historyLoading" class="text-center py-8 text-base-content/50 text-sm">
                <i class="fas fa-spinner fa-spin mr-2"></i>加载中...
              </div>
              <div v-else-if="sessionHistory.length === 0" class="text-center py-10 text-base-content/40 text-sm">
                暂无 Team 会话历史
              </div>
              <button
                v-for="s in sessionHistory"
                :key="s.id"
                class="w-full text-left p-3 rounded-xl border transition-all"
                :class="session?.id === s.id
                  ? 'border-info bg-info/8'
                  : 'border-base-300 hover:border-info/40 hover:bg-base-50'"
                @click="handleSelectHistorySession(s.id)"
              >
                <div class="flex items-start justify-between gap-2">
                  <div class="min-w-0">
                    <div class="text-sm font-semibold text-base-content truncate">{{ s.name }}</div>
                    <div class="text-xs text-base-content/55 line-clamp-2 mt-0.5">{{ s.goal || '无目标描述' }}</div>
                  </div>
                  <span class="badge badge-xs" :class="stateBadgeClass(s.state)">{{ stateLabel(s.state) }}</span>
                </div>
                <div class="text-xs text-base-content/45 mt-2">
                  第 {{ s.current_round }}/{{ s.max_rounds }} 轮 · 更新于 {{ formatDateTime(s.updated_at) }}
                </div>
              </button>
            </div>
          </div>
        </div>
      </Transition>
    </Teleport>

    <!-- Content Area -->
    <div class="flex flex-1 overflow-hidden min-h-0">
      <!-- Main: Session setup or running view -->
      <div class="flex-1 flex flex-col overflow-hidden min-h-0">

        <!-- === Setup Panel (no session yet) === -->
        <div v-if="!session" class="flex-1 flex flex-col items-center justify-center p-8 gap-6">
          <div class="text-center max-w-md">
            <div class="w-20 h-20 mx-auto mb-4 rounded-2xl bg-primary/10 flex items-center justify-center">
              <i class="fas fa-users text-3xl text-primary"></i>
            </div>
            <h2 class="text-xl font-bold text-base-content mb-2">启动 Agent Team</h2>
            <p class="text-sm text-base-content/60 leading-relaxed">
              选择一个团队模板，或自定义角色，开始多角色协作分析
            </p>
          </div>

          <!-- Template selection -->
          <div class="w-full max-w-lg space-y-3">
            <label class="text-sm font-medium text-base-content/70">选择团队模板</label>
            <div class="grid gap-2">
              <button
                v-for="tpl in templates"
                :key="tpl.id"
                class="template-card p-3 rounded-xl border-2 text-left transition-all hover:border-primary/60 hover:bg-primary/5"
                :class="selectedTemplateId === tpl.id ? 'border-primary bg-primary/10' : 'border-base-300'"
                @click="selectedTemplateId = tpl.id"
              >
                <div class="flex items-start justify-between gap-2">
                  <div>
                    <div class="font-medium text-sm flex items-center gap-1.5">
                      {{ tpl.name }}
                      <span v-if="tpl.is_system" class="badge badge-xs badge-info">内置</span>
                    </div>
                    <div class="text-xs text-base-content/50 mt-0.5">{{ tpl.description }}</div>
                  </div>
                  <div class="flex-shrink-0">
                    <i
                      class="fas fa-check-circle text-primary text-sm"
                      v-if="selectedTemplateId === tpl.id"
                    ></i>
                  </div>
                </div>
                <!-- Member chips -->
                <div class="flex gap-1 flex-wrap mt-2">
                  <span
                    v-for="m in tpl.members.slice(0, 5)"
                    :key="m.id"
                    class="badge badge-xs badge-outline"
                  >{{ m.name }}</span>
                  <span v-if="tpl.members.length > 5" class="badge badge-xs badge-ghost">+{{ tpl.members.length - 5 }}</span>
                </div>
              </button>

              <div v-if="templates.length === 0" class="text-center py-6 text-base-content/40 text-sm">
                <i class="fas fa-spinner fa-spin mr-2"></i> 加载模板中...
              </div>
            </div>

            <!-- Goal input -->
            <div class="space-y-1.5 mt-4">
              <label class="text-sm font-medium text-base-content/70">团队目标 <span class="text-error">*</span></label>
              <textarea
                v-model="teamGoal"
                class="textarea textarea-bordered w-full text-sm resize-none h-20 leading-relaxed"
                placeholder="描述你希望团队分析或完成的具体目标，例如：设计一个安全的用户认证模块..."
              ></textarea>
            </div>

            <!-- Start button -->
            <button
              class="btn btn-primary w-full gap-2"
              :disabled="!selectedTemplateId || !teamGoal.trim() || isStarting"
              @click="handleStartTeam"
            >
              <i class="fas fa-play" v-if="!isStarting"></i>
              <i class="fas fa-spinner fa-spin" v-else></i>
              {{ isStarting ? '启动中...' : '启动 Team 会话' }}
            </button>
          </div>
        </div>

        <!-- === Running View === -->
        <template v-else>
          <!-- Members status bar -->
          <div class="members-bar px-4 py-2 border-b border-base-300 bg-base-50/50 flex items-center gap-3 overflow-x-auto">
            <div
              v-for="member in session.members"
              :key="member.id"
              class="member-chip flex items-center gap-1.5 px-2.5 py-1 rounded-full border transition-all flex-shrink-0"
              :class="activeMemberId === member.id
                ? 'border-primary bg-primary/15 text-primary shadow-sm shadow-primary/20'
                : 'border-base-300 bg-base-100 text-base-content/70'"
            >
              <div
                class="w-1.5 h-1.5 rounded-full"
                :class="activeMemberId === member.id ? 'bg-primary animate-pulse' : 'bg-base-300'"
              ></div>
              <span class="text-xs font-medium whitespace-nowrap">{{ member.name }}</span>
              <span v-if="member.token_usage > 0" class="text-xs opacity-50">{{ formatTokens(member.token_usage) }}</span>
            </div>
          </div>

          <!-- Message stream -->
          <div class="flex-1 overflow-y-auto px-4 py-4 space-y-3" ref="messageScrollRef" @scroll="handleMessageScroll">
            <!-- Goal display -->
            <div class="goal-banner flex items-start gap-2 p-3 rounded-xl bg-base-200/60 border border-base-300 text-sm">
              <i class="fas fa-bullseye text-primary mt-0.5 flex-shrink-0"></i>
              <div>
                <span class="font-medium text-base-content/70 block text-xs mb-0.5">团队目标</span>
                <span class="text-base-content/80">{{ session.goal }}</span>
              </div>
            </div>

            <!-- Messages -->
            <TransitionGroup name="message-fade">
              <div
                v-for="msg in renderedMessages"
                :key="msg.id"
                class="team-message flex gap-3"
              >
                <!-- Role avatar -->
                <div class="flex-shrink-0">
                  <div
                    class="w-8 h-8 rounded-full flex items-center justify-center text-xs font-bold"
                    :class="roleAvatarClass(msg.role, msg.member_name)"
                  >
                    {{ roleInitial(msg.member_name, msg.role) }}
                  </div>
                </div>
                <!-- Content -->
                <div class="flex-1 min-w-0">
                  <div class="flex items-center gap-2 mb-1">
                    <span class="text-xs font-semibold text-base-content/80">{{ msg.member_name || roleDisplayName(msg.role) }}</span>
                    <span class="text-xs text-base-content/40">{{ formatTime(msg.timestamp) }}</span>
                    <span v-if="msg.token_count" class="text-xs text-base-content/30">{{ msg.token_count }} tokens</span>
                    <span v-if="msg.is_streaming" class="badge badge-xs badge-primary">流式</span>
                  </div>
                  <div class="message-content text-sm text-base-content/85 leading-relaxed bg-base-200/40 rounded-xl p-3 border border-base-300/50">
                    <MarkdownRenderer :content="msg.content" />
                  </div>
                </div>
              </div>
            </TransitionGroup>

            <!-- Thinking indicator -->
            <div v-if="isRunning && activeMemberName" class="flex gap-3">
              <div class="flex-shrink-0">
                <div class="w-8 h-8 rounded-full bg-primary/20 flex items-center justify-center">
                  <i class="fas fa-spinner fa-spin text-primary text-xs"></i>
                </div>
              </div>
              <div class="flex-1">
                <div class="text-xs font-semibold text-primary/80 mb-1">{{ activeMemberName }} 正在思考...</div>
                <div class="flex gap-1 mt-2">
                  <div class="w-2 h-2 rounded-full bg-primary/50 animate-bounce" style="animation-delay: 0ms"></div>
                  <div class="w-2 h-2 rounded-full bg-primary/50 animate-bounce" style="animation-delay: 150ms"></div>
                  <div class="w-2 h-2 rounded-full bg-primary/50 animate-bounce" style="animation-delay: 300ms"></div>
                </div>
              </div>
            </div>

            <!-- Suspended for human -->
            <div
              v-if="session.state === 'SUSPENDED_FOR_HUMAN'"
              class="suspended-banner p-4 rounded-xl bg-warning/10 border border-warning/40"
            >
              <div class="flex items-center gap-2 mb-2">
                <i class="fas fa-pause-circle text-warning"></i>
                <span class="font-semibold text-warning text-sm">需要人工判断</span>
              </div>
              <p class="text-xs text-base-content/70 mb-3">
                团队意见存在较大分歧（分歧度超过阈值），请人工审阅并提供指导意见以继续。
              </p>
              <div class="flex gap-2">
                <textarea
                  v-model="humanMessage"
                  class="textarea textarea-bordered textarea-xs flex-1 resize-none h-16"
                  placeholder="输入你的指导意见或决策方向..."
                ></textarea>
                <button
                  class="btn btn-sm btn-warning gap-1"
                  :disabled="!humanMessage.trim()"
                  @click="handleHumanSubmit"
                >
                  <i class="fas fa-reply"></i>
                  继续
                </button>
              </div>
            </div>

            <!-- Completed -->
            <div
              v-if="session.state === 'COMPLETED'"
              class="completed-banner p-4 rounded-xl bg-success/10 border border-success/40 flex items-center gap-3"
            >
              <i class="fas fa-check-circle text-success text-xl"></i>
              <div>
                <div class="font-semibold text-success text-sm">Team 会话完成</div>
                <div class="text-xs text-base-content/60">所有讨论轮次结束，产物文档已生成</div>
              </div>
            </div>
          </div>
        </template>
      </div>

      <!-- Side: Blackboard / Artifacts panel -->
      <div
        v-if="session && (showBlackboard || showArtifacts)"
        class="side-panel w-80 flex-shrink-0 border-l border-base-300 flex flex-col overflow-hidden bg-base-100"
      >
        <!-- Panel tabs -->
        <div class="flex border-b border-base-300">
          <button
            class="flex-1 py-2 text-xs font-medium transition-colors"
            :class="showBlackboard ? 'text-secondary border-b-2 border-secondary bg-secondary/5' : 'text-base-content/50 hover:text-base-content'"
            @click="showBlackboard = true; showArtifacts = false"
          >
            <i class="fas fa-chalkboard mr-1"></i> 白板
          </button>
          <button
            class="flex-1 py-2 text-xs font-medium transition-colors"
            :class="showArtifacts ? 'text-primary border-b-2 border-primary bg-primary/5' : 'text-base-content/50 hover:text-base-content'"
            @click="showArtifacts = true; showBlackboard = false"
          >
            <i class="fas fa-file-alt mr-1"></i> 产物 ({{ artifacts.length }})
          </button>
        </div>

        <!-- Blackboard panel -->
        <div v-if="showBlackboard" class="flex-1 overflow-y-auto p-3 space-y-2">
          <div v-if="blackboard.length === 0" class="text-center py-8 text-base-content/40 text-xs">
            白板为空
          </div>
          <div
            v-for="entry in blackboard"
            :key="entry.id"
            class="p-2.5 rounded-lg border"
            :class="entryBorderClass(entry.entry_type)"
          >
            <div class="flex items-center gap-1.5 mb-1">
              <span class="text-xs font-semibold" :class="entryLabelClass(entry.entry_type)">
                <i :class="entryIcon(entry.entry_type)" class="mr-0.5"></i>
                {{ entryTypeLabel(entry.entry_type) }}
              </span>
              <span v-if="entry.contributed_by" class="text-xs text-base-content/40">· {{ entry.contributed_by }}</span>
            </div>
            <div class="text-xs font-medium text-base-content/80 mb-0.5">{{ entry.title }}</div>
            <div class="text-xs text-base-content/60 leading-relaxed line-clamp-4">{{ entry.content }}</div>
          </div>
        </div>

        <!-- Artifacts panel -->
        <div v-if="showArtifacts" class="flex-1 overflow-y-auto p-3 space-y-2">
          <div v-if="artifacts.length === 0" class="text-center py-8 text-base-content/40 text-xs">
            暂无产物文档
          </div>
          <div
            v-for="art in artifacts"
            :key="art.id"
            class="artifact-card p-3 rounded-lg border border-base-300 hover:border-primary/40 cursor-pointer transition-all"
            :class="selectedArtifactId === art.id ? 'border-primary bg-primary/5' : ''"
            @click="toggleArtifact(art)"
          >
            <div class="flex items-start gap-2">
              <i class="fas fa-file-code text-primary text-sm mt-0.5 flex-shrink-0"></i>
              <div class="flex-1 min-w-0">
                <div class="font-medium text-xs text-base-content/90 truncate">{{ art.title }}</div>
                <div class="text-xs text-base-content/40 mt-0.5">
                  v{{ art.version }} · {{ art.artifact_type }} · {{ formatTime(art.created_at) }}
                </div>
              </div>
              <button
                class="btn btn-ghost btn-xs text-base-content/50 hover:text-primary"
                title="下载产物"
                @click.stop="downloadArtifact(art)"
              >
                <i class="fas fa-download"></i>
              </button>
            </div>
            <!-- Content preview when selected -->
            <div
              v-if="selectedArtifactId === art.id"
              class="mt-2 p-2 rounded bg-base-200 text-xs font-mono text-base-content/70 max-h-48 overflow-y-auto leading-relaxed break-words"
            >
              {{ art.content.slice(0, 1000) }}<span v-if="art.content.length > 1000">...</span>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch, nextTick } from 'vue'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { save } from '@tauri-apps/plugin-dialog'
import { writeTextFile } from '@tauri-apps/plugin-fs'
import { agentTeamApi } from '@/api/agentTeam'
import type {
  AgentTeamTemplate,
  AgentTeamSession,
  AgentTeamMessage,
  AgentTeamBlackboardEntry,
  AgentTeamArtifact,
  AgentTeamRoleThinkingEvent,
  AgentTeamRoundEvent,
  AgentTeamStateChangedEvent,
  AgentTeamArtifactEvent,
  AgentTeamMessageStreamStartEvent,
  AgentTeamMessageStreamDeltaEvent,
  AgentTeamMessageStreamDoneEvent,
  ExecutionMode,
} from '@/types/agentTeam'
import AgentTeamTemplateLibrary from './AgentTeamTemplateLibrary.vue'
import MarkdownRenderer from './MarkdownRenderer.vue'

// ==================== Props / Emits ====================

const props = defineProps<{
  conversationId?: string
}>()

const emit = defineEmits<{
  (e: 'switch-mode', mode: ExecutionMode): void
}>()

// ==================== State ====================

const templates = ref<AgentTeamTemplate[]>([])
const session = ref<AgentTeamSession | null>(null)
const teamMessages = ref<AgentTeamMessage[]>([])
const blackboard = ref<AgentTeamBlackboardEntry[]>([])
const artifacts = ref<AgentTeamArtifact[]>([])

const selectedTemplateId = ref<string>('')
const teamGoal = ref('')
const isStarting = ref(false)

const showBlackboard = ref(false)
const showArtifacts = ref(false)
const showTemplateLibrary = ref(false)
const showSessionHistory = ref(false)
const sessionHistory = ref<AgentTeamSession[]>([])
const historyLoading = ref(false)
const selectedArtifactId = ref<string | null>(null)

const humanMessage = ref('')
const activeMemberId = ref<string | null>(null)
const activeMemberName = ref<string | null>(null)
const messageScrollRef = ref<HTMLElement | null>(null)
const shouldStickToBottom = ref(true)
const AUTO_SCROLL_THRESHOLD_PX = 120

const isRunning = computed(() =>
  session.value?.state === 'PROPOSING' ||
  session.value?.state === 'DECIDING' ||
  session.value?.state === 'INITIALIZING' ||
  session.value?.state === 'ARTIFACT_GENERATION'
)

type TeamRenderMessage = AgentTeamMessage & { is_streaming?: boolean }
type TeamStreamingMessage = TeamRenderMessage & { stream_id: string }

const streamingMessagesById = ref<Record<string, TeamStreamingMessage>>({})

const renderedMessages = computed<TeamRenderMessage[]>(() => {
  const base = teamMessages.value.map((m) => ({ ...m, is_streaming: false }))
  const streamings = Object.values(streamingMessagesById.value)
  if (streamings.length === 0) return base
  const combined = [...base, ...streamings]
  combined.sort((a, b) => {
    const at = new Date(a.timestamp).getTime()
    const bt = new Date(b.timestamp).getTime()
    return at - bt
  })
  return combined
})

// Tauri event unlisten fns
const unlistenFns: UnlistenFn[] = []

// ==================== Lifecycle ====================

onMounted(async () => {
  await loadTemplates()
  await setupEventListeners()
  await loadSessionHistory()
  await restoreLastSession()
})

onUnmounted(() => {
  unlistenFns.forEach(fn => fn())
})

watch(() => session.value?.id, () => {
  shouldStickToBottom.value = true
})

// ==================== Data Loading ====================

async function loadTemplates() {
  try {
    const list = await agentTeamApi.listTemplates()
    if (list.length === 0) {
      // Seed built-in templates on first run
      await agentTeamApi.seedBuiltinTemplates()
      templates.value = await agentTeamApi.listTemplates()
    } else {
      templates.value = list
    }
    if (templates.value.length > 0 && !selectedTemplateId.value) {
      selectedTemplateId.value = templates.value[0].id
    }
  } catch (e) {
    console.error('[AgentTeamView] Failed to load templates:', e)
  }
}

async function loadSessionData(sessionId: string) {
  try {
    const [msgs, bb, arts] = await Promise.all([
      agentTeamApi.getMessages(sessionId),
      agentTeamApi.getBlackboard(sessionId),
      agentTeamApi.listArtifacts(sessionId),
    ])
    teamMessages.value = msgs
    blackboard.value = bb
    artifacts.value = arts
    streamingMessagesById.value = {}
    if (arts.length > 0) showArtifacts.value = true
  } catch (e) {
    console.error('[AgentTeamView] Failed to load session data:', e)
  }
}

async function loadSessionHistory() {
  historyLoading.value = true
  try {
    sessionHistory.value = await agentTeamApi.listSessions(props.conversationId, 50)
  } catch (e) {
    console.error('[AgentTeamView] Failed to load session history:', e)
  } finally {
    historyLoading.value = false
  }
}

function toggleSessionHistory() {
  showSessionHistory.value = !showSessionHistory.value
  if (showSessionHistory.value) {
    loadSessionHistory()
  }
}

async function restoreLastSession() {
  try {
    const list = sessionHistory.value.length > 0
      ? sessionHistory.value.slice(0, 1)
      : await agentTeamApi.listSessions(props.conversationId, 1)
    if (list.length === 0) return
    session.value = list[0]
    showBlackboard.value = true
    await loadSessionData(list[0].id)
  } catch (e) {
    console.warn('[AgentTeamView] Failed to restore last team session:', e)
  }
}

// ==================== Start Run ====================

async function handleStartTeam() {
  if (!selectedTemplateId.value || !teamGoal.value.trim()) return
  isStarting.value = true
  try {
    const newSession = await agentTeamApi.createSession({
      name: `Team: ${teamGoal.value.slice(0, 30)}`,
      goal: teamGoal.value.trim(),
      template_id: selectedTemplateId.value,
      conversation_id: props.conversationId,
      max_rounds: 5,
    })
    session.value = newSession
    showBlackboard.value = true
    await loadSessionHistory()

    // Start the run async
    agentTeamApi.startRun(newSession.id).catch(e =>
      console.error('[AgentTeamView] Failed to start run:', e)
    )
  } catch (e) {
    console.error('[AgentTeamView] Failed to start team run:', e)
  } finally {
    isStarting.value = false
  }
}

async function handleSelectHistorySession(sessionId: string) {
  try {
    const s = await agentTeamApi.getSession(sessionId)
    if (!s) return
    session.value = s
    showBlackboard.value = true
    showSessionHistory.value = false
    await loadSessionData(sessionId)
  } catch (e) {
    console.error('[AgentTeamView] Failed to select history session:', e)
  }
}

function handleLibrarySessionCreated(sessionId: string) {
  showTemplateLibrary.value = false
  // load the newly crated session
  agentTeamApi.getSession(sessionId).then(s => {
    if (s) {
      session.value = s
      showBlackboard.value = true
      loadSessionData(sessionId)
      // Template library flow creates session only; auto start run for better UX.
      if (s.state === 'PENDING') {
        handleStartExistingSession()
      }
    }
  }).catch(console.error)
}

async function handleStartExistingSession() {
  if (!session.value) return
  isStarting.value = true
  try {
    await agentTeamApi.startRun(session.value.id)
  } catch (e) {
    console.error('[AgentTeamView] Failed to start existing session:', e)
  } finally {
    isStarting.value = false
  }
}

// ==================== Human-in-the-Loop ====================

async function handleHumanSubmit() {
  if (!session.value || !humanMessage.value.trim()) return
  try {
    await agentTeamApi.submitMessage({
      session_id: session.value.id,
      content: humanMessage.value.trim(),
      resume: true,
    })
    humanMessage.value = ''
    await refreshSession()
  } catch (e) {
    console.error('[AgentTeamView] Failed to submit human message:', e)
  }
}

// ==================== Event Listeners ====================

async function setupEventListeners() {
  const unlistenStateChanged = await listen<AgentTeamStateChangedEvent>('agent_team:state_changed', async (event) => {
    if (!session.value || event.payload.session_id !== session.value.id) return
    session.value = { ...session.value, state: event.payload.state }
    await loadSessionData(session.value.id)
    streamingMessagesById.value = {}
  })

  const unlistenRoleThinking = await listen<AgentTeamRoleThinkingEvent>('agent_team:role_thinking', (event) => {
    if (!session.value) return
    activeMemberId.value = event.payload.member_id
    activeMemberName.value = event.payload.member_name
  })

  const unlistenRoundStarted = await listen<AgentTeamRoundEvent>('agent_team:round_started', async (event) => {
    if (!session.value) return
    session.value = { ...session.value, current_round: event.payload.round }
  })

  const unlistenRoundCompleted = await listen<AgentTeamRoundEvent>('agent_team:round_completed', async (event) => {
    if (!session.value) return
    activeMemberId.value = null
    activeMemberName.value = null
    await loadSessionData(session.value.id)
    streamingMessagesById.value = {}
  })

  const unlistenArtifactGenerated = await listen<AgentTeamArtifactEvent>('agent_team:artifact_generated', async (event) => {
    if (!session.value || event.payload.session_id !== session.value.id) return
    artifacts.value = await agentTeamApi.listArtifacts(session.value.id)
    showArtifacts.value = true
  })

  const unlistenComplete = await listen('agent_team:complete', async (event) => {
    if (!session.value) return
    activeMemberId.value = null
    activeMemberName.value = null
    await refreshSession()
    streamingMessagesById.value = {}
  })

  const unlistenMessageStreamStart = await listen<AgentTeamMessageStreamStartEvent>('agent_team:message_stream_start', (event) => {
    if (!session.value || event.payload.session_id !== session.value.id) return
    const id = `stream-${event.payload.stream_id}`
    streamingMessagesById.value = {
      ...streamingMessagesById.value,
      [id]: {
      stream_id: event.payload.stream_id,
      id,
      session_id: event.payload.session_id,
      member_id: event.payload.member_id,
      member_name: event.payload.member_name,
      role: 'assistant',
      content: '',
      timestamp: new Date().toISOString(),
      is_streaming: true,
      },
    }
  })

  const unlistenMessageStreamDelta = await listen<AgentTeamMessageStreamDeltaEvent>('agent_team:message_stream_delta', (event) => {
    if (!session.value || event.payload.session_id !== session.value.id) return
    const id = `stream-${event.payload.stream_id}`
    const prev = streamingMessagesById.value[id] || {
      stream_id: event.payload.stream_id,
      id,
      session_id: event.payload.session_id,
      member_id: event.payload.member_id,
      member_name: event.payload.member_name,
      role: 'assistant',
      content: '',
      timestamp: new Date().toISOString(),
      is_streaming: true,
    }
    streamingMessagesById.value = {
      ...streamingMessagesById.value,
      [id]: {
        ...prev,
        content: `${prev.content}${event.payload.delta || ''}`,
        timestamp: new Date().toISOString(),
      },
    }
  })

  const unlistenMessageStreamDone = await listen<AgentTeamMessageStreamDoneEvent>('agent_team:message_stream_done', (event) => {
    if (!session.value || event.payload.session_id !== session.value.id) return
    const id = `stream-${event.payload.stream_id}`
    const prev = streamingMessagesById.value[id] || {
      stream_id: event.payload.stream_id,
      id,
      session_id: event.payload.session_id,
      member_id: event.payload.member_id,
      member_name: event.payload.member_name,
      role: 'assistant',
      content: '',
      timestamp: new Date().toISOString(),
      is_streaming: true,
    }
    streamingMessagesById.value = {
      ...streamingMessagesById.value,
      [id]: {
        ...prev,
        content: event.payload.content || prev.content,
        is_streaming: false,
        timestamp: new Date().toISOString(),
      },
    }
  })

  unlistenFns.push(
    unlistenStateChanged,
    unlistenRoleThinking,
    unlistenRoundStarted,
    unlistenRoundCompleted,
    unlistenArtifactGenerated,
    unlistenComplete,
    unlistenMessageStreamStart,
    unlistenMessageStreamDelta,
    unlistenMessageStreamDone,
  )
}

async function refreshSession() {
  if (!session.value) return
  try {
    const updated = await agentTeamApi.getSession(session.value.id)
    if (updated) session.value = updated
    await loadSessionData(session.value.id)
  } catch (e) {
    console.error('[AgentTeamView] Failed to refresh session:', e)
  }
}

// Auto-scroll messages
watch(renderedMessages, async () => {
  const shouldAutoScroll = shouldStickToBottom.value
  await nextTick()
  if (messageScrollRef.value && shouldAutoScroll) {
    messageScrollRef.value.scrollTop = messageScrollRef.value.scrollHeight
  }
})

function handleMessageScroll() {
  const el = messageScrollRef.value
  if (!el) return
  const distanceToBottom = el.scrollHeight - (el.scrollTop + el.clientHeight)
  shouldStickToBottom.value = distanceToBottom <= AUTO_SCROLL_THRESHOLD_PX
}

// ==================== Artifact helpers ====================

function toggleArtifact(art: AgentTeamArtifact) {
  selectedArtifactId.value = selectedArtifactId.value === art.id ? null : art.id
}

function artifactFileExtension(art: AgentTeamArtifact): string {
  const normalizedType = String(art.artifact_type || '').toLowerCase()
  const content = art.content || ''
  if (normalizedType.includes('json')) return 'json'
  if (normalizedType.includes('html')) return 'html'
  if (normalizedType.includes('csv')) return 'csv'
  if (normalizedType.includes('yaml') || normalizedType.includes('yml')) return 'yaml'
  if (normalizedType.includes('xml')) return 'xml'
  if (normalizedType.includes('markdown') || normalizedType.includes('md')) return 'md'
  const trimmed = content.trim()
  if ((trimmed.startsWith('{') && trimmed.endsWith('}')) || (trimmed.startsWith('[') && trimmed.endsWith(']'))) {
    return 'json'
  }
  if (trimmed.startsWith('<!DOCTYPE html') || trimmed.startsWith('<html')) return 'html'
  return 'md'
}

function sanitizeFilenamePart(value: string): string {
  return (value || 'artifact')
    .replace(/[<>:"/\\|?*\x00-\x1f]/g, '_')
    .replace(/\s+/g, '_')
    .slice(0, 80)
}

function artifactFilename(art: AgentTeamArtifact): string {
  const ext = artifactFileExtension(art)
  const title = sanitizeFilenamePart(art.title || 'artifact')
  const type = sanitizeFilenamePart(art.artifact_type || 'doc')
  return `${title}_${type}_v${art.version}.${ext}`
}

async function downloadArtifact(art: AgentTeamArtifact) {
  const filename = artifactFilename(art)
  const content = art.content ?? ''
  const ext = artifactFileExtension(art)
  try {
    const filePath = await save({
      defaultPath: filename,
      filters: [{ name: ext.toUpperCase(), extensions: [ext] }],
    })
    if (!filePath) return
    await writeTextFile(filePath, content)
  } catch (e) {
    console.error('[AgentTeamView] Failed to save artifact:', e)
    try {
      const blob = new Blob([content], { type: 'text/plain;charset=utf-8;' })
      const url = URL.createObjectURL(blob)
      const link = document.createElement('a')
      link.href = url
      link.download = filename
      document.body.appendChild(link)
      link.click()
      document.body.removeChild(link)
      URL.revokeObjectURL(url)
    } catch (fallbackError) {
      console.error('[AgentTeamView] Fallback download also failed:', fallbackError)
    }
  }
}

// ==================== Display helpers ====================

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
    SUSPENDED_FOR_HUMAN: '待人工',
  }
  return map[state] ?? state
}

function stateBadgeClass(state: string): string {
  if (state === 'COMPLETED') return 'badge-success'
  if (state === 'FAILED') return 'badge-error'
  if (state === 'SUSPENDED_FOR_HUMAN') return 'badge-warning'
  if (['PROPOSING', 'DECIDING', 'CHALLENGING', 'ARTIFACT_GENERATION'].includes(state)) return 'badge-primary'
  return 'badge-ghost'
}

function roleAvatarClass(role: string, memberName?: string): string {
  if (role === 'user') return 'bg-neutral text-neutral-content'
  const colors = ['bg-primary/20 text-primary', 'bg-secondary/20 text-secondary', 'bg-accent/20 text-accent', 'bg-success/20 text-success']
  const idx = (memberName?.charCodeAt(0) ?? 0) % colors.length
  return colors[idx]
}

function roleInitial(memberName?: string, role?: string): string {
  if (memberName) return memberName.charAt(0).toUpperCase()
  if (role === 'user') return 'U'
  return 'A'
}

function roleDisplayName(role: string): string {
  if (role === 'user') return '人工介入'
  if (role === 'system') return '系统'
  return role
}

function formatTime(timestamp: string): string {
  try {
    const d = new Date(timestamp)
    return d.toLocaleTimeString('zh-CN', { hour: '2-digit', minute: '2-digit' })
  } catch {
    return ''
  }
}

function formatDateTime(timestamp: string): string {
  try {
    const d = new Date(timestamp)
    return d.toLocaleString('zh-CN', { month: '2-digit', day: '2-digit', hour: '2-digit', minute: '2-digit' })
  } catch {
    return ''
  }
}

function formatTokens(tokens: number): string {
  if (tokens >= 1000) return `${(tokens / 1000).toFixed(1)}k`
  return String(tokens)
}

function entryBorderClass(type: string): string {
  if (type === 'consensus') return 'border-success/40 bg-success/5'
  if (type === 'dispute') return 'border-error/40 bg-error/5'
  return 'border-info/40 bg-info/5'
}

function entryLabelClass(type: string): string {
  if (type === 'consensus') return 'text-success'
  if (type === 'dispute') return 'text-error'
  return 'text-info'
}

function entryIcon(type: string): string {
  if (type === 'consensus') return 'fas fa-handshake'
  if (type === 'dispute') return 'fas fa-exclamation-triangle'
  return 'fas fa-tasks'
}

function entryTypeLabel(type: string): string {
  if (type === 'consensus') return '共识'
  if (type === 'dispute') return '分歧'
  return '待办'
}
</script>

<style scoped>
.agent-team-view {
  font-family: inherit;
}

.template-card {
  cursor: pointer;
}

.message-fade-enter-active,
.message-fade-leave-active {
  transition: all 0.3s ease;
}

.message-fade-enter-from {
  opacity: 0;
  transform: translateY(8px);
}

.message-fade-leave-to {
  opacity: 0;
}

.message-content pre {
  white-space: pre-wrap;
  word-break: break-word;
}

.members-bar::-webkit-scrollbar {
  height: 0;
}

.slide-library-enter-active,
.slide-library-leave-active {
  transition: all 0.25s ease;
}
.slide-library-enter-from .relative,
.slide-library-leave-to .relative {
  transform: translateX(100%);
}
.slide-library-enter-from,
.slide-library-leave-to {
  opacity: 0;
}
</style>
