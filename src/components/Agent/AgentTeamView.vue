<template>
  <div class="agent-team-view flex flex-col h-full overflow-hidden bg-gradient-to-br from-base-100 to-base-200">
    <!-- Header -->
    <div class="team-header px-4 py-2.5 border-b border-base-300 bg-base-100/80 backdrop-blur-sm flex items-center justify-between">
      <div class="flex items-center gap-3">
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
        <div v-if="session" class="flex items-center gap-1 text-xs text-base-content/50">
          <i class="fas fa-sync-alt text-xs"></i>
          <span>第 {{ session.current_round }}/{{ session.max_rounds }} 轮</span>
        </div>
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
        <button
          class="btn btn-xs btn-primary gap-1"
          @click="handleCreateNewSession"
          title="新建 Team 会话"
        >
          <i class="fas fa-plus text-xs"></i>
          <span>新会话</span>
        </button>
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
        <!-- Session manager button -->
        <button
          class="btn btn-xs gap-1"
          :class="showSessionManager ? 'btn-info' : 'btn-ghost text-base-content/50'"
          @click="toggleSessionManager"
          id="team-session-manager-toggle-btn"
        >
          <i class="fas fa-history text-xs"></i>
          <span>会话</span>
        </button>
        <!-- Side panel toggle -->
        <button
          class="btn btn-xs gap-1"
          :class="showSidePanel ? 'btn-secondary' : 'btn-ghost text-secondary'"
          @click="showSidePanel = !showSidePanel"
          title="面板 (白板/产物/时间线/对比)"
        >
          <i class="fas fa-columns text-xs"></i>
          <span>面板</span>
          <span v-if="blackboard.length + artifacts.length > 0" class="badge badge-xs badge-secondary">{{ blackboard.length + artifacts.length }}</span>
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
          <div class="relative ml-auto w-full max-w-md h-full bg-base-100 shadow-2xl flex flex-col drawer-right">
            <AgentTeamTemplateLibrary
              :conversation-id="conversationId"
              @close="showTemplateLibrary = false"
              @templates-updated="handleTemplatesUpdated"
              @session-created="handleLibrarySessionCreated"
            />
          </div>
        </div>
      </Transition>
    </Teleport>

    <!-- Session Manager Drawer -->
    <Teleport to="body">
      <Transition name="slide-library">
        <div
          v-if="showSessionManager"
          class="fixed inset-0 z-[80] flex"
        >
          <div class="absolute inset-0 bg-black/30" @click="showSessionManager = false"></div>
          <div class="relative mr-auto w-full max-w-md h-full bg-base-100 shadow-2xl flex flex-col drawer-left">
            <TeamSessionList
              :sessions="teamSessions"
              :current-session-id="session?.id ?? null"
              :loading="sessionLoading"
              :loading-more="sessionLoadingMore"
              :has-more="hasMoreSessions"
              @create="handleCreateNewSession"
              @refresh="loadTeamSessions(true)"
              @close="showSessionManager = false"
              @select="handleSelectSession"
              @rename="handleRenameSession"
              @archive="handleArchiveSession"
              @restore="handleRestoreSession"
              @delete="handleDeleteSession"
              @load-more="loadTeamSessions(false)"
            />
          </div>
        </div>
      </Transition>
    </Teleport>

    <!-- Content Area -->
    <div class="flex flex-1 overflow-hidden min-h-0">
      <!-- Main: Team running view -->
      <div class="flex-1 flex flex-col overflow-hidden min-h-0">
        <!-- Members status bar -->
        <div
          v-if="session"
          class="members-bar px-4 py-2 border-b border-base-300 bg-base-50/50 flex items-center gap-3 overflow-x-auto"
        >
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
            <div
              v-if="!session"
              class="goal-banner flex items-start gap-2 p-3 rounded-xl bg-base-200/60 border border-base-300 text-sm"
            >
              <i class="fas fa-lightbulb text-primary mt-0.5 flex-shrink-0"></i>
              <div>
                <span class="font-medium text-base-content/70 block text-xs mb-0.5">新 Team 会话</span>
                <span class="text-base-content/80">请在底部输入框输入目标并发送，系统会直接创建并启动 Team 会话。</span>
              </div>
            </div>
            <!-- Goal display -->
            <div v-if="session" class="goal-banner flex items-start gap-2 p-3 rounded-xl bg-base-200/60 border border-base-300 text-sm">
              <i class="fas fa-bullseye text-primary mt-0.5 flex-shrink-0"></i>
              <div class="flex-1 min-w-0">
                <span class="font-medium text-base-content/70 block text-xs mb-0.5">团队目标</span>
                <textarea
                  v-if="isEditingGoal"
                  v-model="editableGoal"
                  class="textarea textarea-bordered textarea-sm w-full text-sm leading-relaxed min-h-[72px]"
                  :disabled="isStarting || isStopping"
                  placeholder="请输入团队目标"
                />
                <span v-else class="text-base-content/80 whitespace-pre-wrap">{{ session.goal }}</span>
              </div>
              <div class="flex items-center gap-1.5 flex-shrink-0">
                <button
                  v-if="isEditingGoal"
                  class="action-btn btn btn-xs btn-ghost bg-base-100/70 hover:bg-base-100 backdrop-blur text-base-content/60 hover:text-base-content"
                  :disabled="isStarting || isStopping"
                  @click="cancelEditGoal"
                  title="取消编辑"
                >
                  <i class="fas fa-times"></i>
                </button>
                <button
                  v-else
                  class="action-btn btn btn-xs btn-ghost bg-base-100/70 hover:bg-base-100 backdrop-blur text-base-content/60 hover:text-base-content"
                  :disabled="isStarting || isStopping || isRunning"
                  @click="startEditGoal"
                  title="编辑团队目标"
                >
                  <i class="fas fa-edit"></i>
                </button>
                <button
                  class="action-btn btn btn-xs btn-ghost bg-base-100/70 hover:bg-base-100 backdrop-blur text-base-content/60 hover:text-base-content"
                  :disabled="!canResendGoal"
                  @click="handleResendGoal"
                  :title="isEditingGoal ? '按编辑后的团队目标重新发送' : '按当前团队目标重新发送'"
                >
                  <i class="fas fa-redo"></i>
                </button>
              </div>
            </div>

            <!-- Messages -->
            <TransitionGroup name="message-fade">
              <div
                v-for="msg in displayMessages"
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
                  <div
                    v-if="msg.role !== 'tool_call'"
                    class="message-content text-sm text-base-content/85 leading-relaxed bg-base-200/40 rounded-xl p-3 border border-base-300/50"
                  >
                    <MarkdownRenderer :content="msg.content" />
                  </div>
                  <ToolCallsDisplay
                    v-if="msg.role === 'tool_call'"
                    :tool-calls="formatTeamToolCalls(msg.tool_calls)"
                    class="mt-1 team-tool-calls"
                  />
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

            <!-- Divergence alert banner -->
            <Transition name="message-fade">
              <div
                v-if="divergenceAlert"
                class="divergence-alert p-3 rounded-xl bg-error/10 border border-error/40 flex items-center gap-2"
              >
                <i class="fas fa-exclamation-triangle text-error flex-shrink-0"></i>
                <div class="flex-1">
                  <span class="font-semibold text-error text-xs">分歧度告警</span>
                  <span class="text-xs text-base-content/70 ml-1">
                    当前分歧度 {{ (divergenceAlert.divergence_score * 100).toFixed(0) }}%，
                    阈值 {{ (divergenceAlert.threshold * 100).toFixed(0) }}%
                  </span>
                </div>
                <button class="btn btn-ghost btn-xs" @click="divergenceAlert = null">
                  <i class="fas fa-times text-xs"></i>
                </button>
              </div>
            </Transition>

            <!-- Suspended for human -->
            <div
              v-if="session && session.state === 'SUSPENDED_FOR_HUMAN'"
              class="suspended-banner p-4 rounded-xl bg-warning/10 border border-warning/40"
            >
              <div class="flex items-center gap-2 mb-2">
                <i class="fas fa-pause-circle text-warning"></i>
                <span class="font-semibold text-warning text-sm">需要人工判断</span>
              </div>
              <p class="text-xs text-base-content/70 mb-3">
                团队意见存在较大分歧（分歧度超过阈值），请在底部输入栏发送人工指导意见继续。
              </p>
              <p v-if="autoResumeSecondsLeft !== null" class="text-xs text-base-content/60 mb-3">
                若无人工输入，{{ formatCountdown(autoResumeSecondsLeft) }} 后将按默认策略自动继续。
              </p>
              <div class="flex flex-wrap gap-2">
                <button
                  class="btn btn-xs btn-outline btn-warning"
                  :disabled="quickIntervening"
                  @click="handleQuickIntervention('conservative')"
                >
                  保守继续
                </button>
                <button
                  class="btn btn-xs btn-outline btn-info"
                  :disabled="quickIntervening"
                  @click="handleQuickIntervention('balanced')"
                >
                  平衡继续
                </button>
                <button
                  class="btn btn-xs btn-outline btn-error"
                  :disabled="quickIntervening"
                  @click="handleQuickIntervention('aggressive')"
                >
                  激进继续
                </button>
              </div>
            </div>

            <!-- Completed -->
            <div
              v-if="session && session.state === 'COMPLETED'"
              class="completed-banner p-4 rounded-xl bg-success/10 border border-success/40 flex items-center gap-3"
            >
              <i class="fas fa-check-circle text-success text-xl"></i>
              <div>
                <div class="font-semibold text-success text-sm">Team 会话完成</div>
                <div class="text-xs text-base-content/60">所有讨论轮次结束，产物文档已生成</div>
              </div>
            </div>
        </div>
        <!-- Team Input Bar (left/main only) -->
        <div class="input-area-container border-t border-base-300/50 bg-base-100 flex-shrink-0 relative z-0">
          <div class="px-4 pb-3 pt-2">
            <div class="chat-input rounded-2xl bg-base-200/60 border border-base-300/60 backdrop-blur-sm flex flex-col gap-2 px-3 py-2 shadow-sm focus-within:border-primary transition-colors">
              <div class="min-w-0">
                <textarea
                  ref="teamTextareaRef"
                  v-model="teamInput"
                  class="w-full bg-transparent outline-none resize-none leading-relaxed text-sm placeholder:text-base-content/50 overflow-hidden"
                  rows="1"
                  :disabled="isStarting"
                  :placeholder="session ? '输入补充需求或人工意见...' : '输入团队目标并发送，自动启动 Team 会话...'"
                  @input="onTeamInput"
                  @keydown="onTeamTextareaKeydown"
                />
              </div>

              <div class="flex items-center justify-between gap-2">
                <div class="flex items-center gap-2 text-base-content/60 shrink-0">
                  <button
                    class="icon-btn"
                    :class="{ active: localToolsEnabled }"
                    @click="toggleTeamTools"
                    title="工具调用"
                  >
                    <i class="fas fa-tools"></i>
                  </button>
                  <button
                    v-if="localToolsEnabled"
                    class="icon-btn"
                    @click="showTeamToolConfig = true"
                    title="工具配置"
                  >
                    <i class="fas fa-cog"></i>
                  </button>
                  <button
                    class="icon-btn"
                    @click="showTemplateLibrary = true"
                    title="打开模板库"
                  >
                    <i class="fas fa-layer-group"></i>
                  </button>
                </div>

                <div class="flex items-center gap-2 shrink-0">
                  <select
                    v-model="selectedTemplateId"
                    class="select select-sm min-h-0 h-7 bg-base-100/80 border border-base-300/70 rounded-lg text-xs"
                    :disabled="isStarting"
                    title="新会话模板"
                  >
                    <option v-for="tpl in templates" :key="tpl.id" :value="tpl.id">
                      {{ tpl.name }}
                    </option>
                  </select>
                  <button
                    v-if="!isRunning"
                    class="send-btn"
                    :disabled="!teamInput.trim() || isStarting || (!session && !selectedTemplateId)"
                    :class="{ 'opacity-40 cursor-not-allowed': !teamInput.trim() || isStarting || (!session && !selectedTemplateId) }"
                    @click="handleTeamSend"
                    :title="session ? '发送人工意见 (Enter)' : '启动 Team 会话 (Enter)'"
                  >
                    <i class="fas fa-arrow-up" v-if="!isStarting"></i>
                    <i class="fas fa-spinner fa-spin" v-else></i>
                  </button>
                  <button
                    v-else
                    class="send-btn bg-error text-error-content hover:bg-error/90"
                    :disabled="isStopping"
                    @click="handleStopRun"
                    title="停止执行"
                  >
                    <i class="fas fa-stop" v-if="!isStopping"></i>
                    <i class="fas fa-spinner fa-spin" v-else></i>
                  </button>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- Side: Multi-panel (Blackboard / Artifacts / Timeline / Challenge) -->
      <div
        v-if="session && showSidePanel"
        class="side-panel flex-shrink-0 border-l border-base-300 flex flex-col overflow-hidden bg-base-100 relative"
        :style="{ width: teamSidePanelWidth + 'px' }"
      >
        <!-- Resize Handle -->
        <div
          class="resize-handle absolute left-0 top-0 bottom-0 w-1 cursor-col-resize hover:bg-primary/50 transition-colors z-10"
          @mousedown="startTeamSideResize"
        ></div>

        <!-- Panel tabs -->
        <div class="flex border-b border-base-300 overflow-x-auto">
          <button
            class="flex-1 py-2 text-xs font-medium transition-colors whitespace-nowrap px-2"
            :class="activeSideTab === 'blackboard' ? 'text-secondary border-b-2 border-secondary bg-secondary/5' : 'text-base-content/50 hover:text-base-content'"
            @click="activeSideTab = 'blackboard'"
          >
            <i class="fas fa-chalkboard mr-1"></i> 白板
            <span v-if="blackboard.length > 0" class="badge badge-xs badge-secondary ml-0.5">{{ blackboard.length }}</span>
          </button>
          <button
            class="flex-1 py-2 text-xs font-medium transition-colors whitespace-nowrap px-2"
            :class="activeSideTab === 'artifacts' ? 'text-primary border-b-2 border-primary bg-primary/5' : 'text-base-content/50 hover:text-base-content'"
            @click="activeSideTab = 'artifacts'"
          >
            <i class="fas fa-file-alt mr-1"></i> 产物
            <span v-if="artifacts.length > 0" class="badge badge-xs badge-primary ml-0.5">{{ artifacts.length }}</span>
          </button>
          <button
            class="flex-1 py-2 text-xs font-medium transition-colors whitespace-nowrap px-2"
            :class="activeSideTab === 'timeline' ? 'text-accent border-b-2 border-accent bg-accent/5' : 'text-base-content/50 hover:text-base-content'"
            @click="activeSideTab = 'timeline'"
          >
            <i class="fas fa-stream mr-1"></i> 时间线
          </button>
          <button
            class="flex-1 py-2 text-xs font-medium transition-colors whitespace-nowrap px-2"
            :class="activeSideTab === 'challenge' ? 'text-warning border-b-2 border-warning bg-warning/5' : 'text-base-content/50 hover:text-base-content'"
            @click="activeSideTab = 'challenge'"
          >
            <i class="fas fa-code-compare mr-1"></i> 对比
          </button>
        </div>

        <!-- Blackboard panel (full sub-component) -->
        <AgentTeamBlackboardPanel
          v-if="activeSideTab === 'blackboard'"
          :entries="blackboard"
          :can-annotate="true"
          @resolve="handleResolveBlackboardEntry"
          @add-entry="handleAddBlackboardEntry"
          @annotate="handleAnnotateBlackboardEntry"
          class="flex-1 overflow-hidden"
        />

        <!-- Artifacts panel (full sub-component) -->
        <AgentTeamArtifactPanel
          v-if="activeSideTab === 'artifacts'"
          :artifacts="artifacts"
          @export="downloadArtifact"
          class="flex-1 overflow-hidden"
        />

        <!-- Timeline panel -->
        <AgentTeamTimeline
          v-if="activeSideTab === 'timeline'"
          :rounds="timelineRounds"
          :messages="teamMessages"
          :is-running="isRunning"
          :current-member-name="activeMemberName"
          class="flex-1 overflow-hidden"
        />

        <!-- Challenge split view -->
        <AgentTeamChallengeSplitView
          v-if="activeSideTab === 'challenge'"
          :messages="teamMessages"
          :divergence-score="latestDivergenceScore"
          :threshold="0.6"
          class="flex-1 overflow-hidden"
        />
      </div>
    </div>

    <dialog class="modal" :class="{ 'modal-open': showTeamToolConfig }">
      <div class="modal-box max-w-2xl">
        <div class="flex items-center justify-between mb-3">
          <h3 class="font-semibold text-base">Team 工具配置</h3>
          <button class="btn btn-ghost btn-xs" @click="showTeamToolConfig = false">
            <i class="fas fa-times"></i>
          </button>
        </div>
        <div class="space-y-3">
          <div class="flex items-center gap-2">
            <input
              type="checkbox"
              class="toggle toggle-sm toggle-primary"
              :checked="localToolsEnabled"
              @change="toggleTeamTools"
            />
            <span class="text-sm text-base-content/80">启用 Team 工具调用</span>
          </div>
          <div class="space-y-1">
            <label class="text-sm text-base-content/80" for="team-human-intervention-max">人工介入上限</label>
            <div class="flex items-center gap-2">
              <input
                id="team-human-intervention-max"
                v-model.number="teamMaxHumanInterventions"
                type="number"
                min="1"
                max="10"
                class="input input-bordered input-sm w-24"
                @blur="normalizeTeamMaxHumanInterventions"
              />
              <span class="text-xs text-base-content/60">超过上限后将强制推进决策，避免死循环</span>
            </div>
          </div>
          <div v-if="localToolsEnabled" class="space-y-2">
            <div class="text-xs text-base-content/60">留空表示不限制（可使用全部工具）</div>
            <div class="relative w-full">
              <input
                v-model="teamToolSearch"
                type="text"
                class="input input-bordered input-sm w-full pr-8"
                placeholder="搜索工具名或描述..."
              />
              <div class="absolute inset-y-0 right-0 flex items-center pr-2">
                <button
                  v-if="teamToolSearch"
                  class="btn btn-ghost btn-xs btn-circle h-5 w-5 min-h-0"
                  @click="teamToolSearch = ''"
                  title="清空搜索"
                >
                  <i class="fas fa-times text-xs"></i>
                </button>
                <i v-else class="fas fa-search text-xs text-base-content/50"></i>
              </div>
            </div>

            <div class="flex flex-wrap items-center gap-2 pb-2 border-b border-base-300">
              <button
                class="btn btn-xs"
                :class="selectedTeamToolCategories.length === 0 && !showSelectedTeamToolsOnly ? 'btn-primary' : 'btn-ghost'"
                @click="clearTeamToolFilters"
              >
                全部
              </button>
              <button
                class="btn btn-xs"
                :class="showSelectedTeamToolsOnly ? 'btn-primary' : 'btn-ghost'"
                @click="toggleShowSelectedTeamTools"
              >
                已选 ({{ teamToolConfig.allowlist.length }})
              </button>
              <button
                v-for="cat in allTeamToolCategories"
                :key="cat"
                class="btn btn-xs"
                :class="selectedTeamToolCategories.includes(cat) ? getTeamToolCategoryBadgeClass(cat) : 'btn-ghost'"
                @click="toggleTeamToolCategory(cat)"
              >
                <i :class="getTeamToolCategoryIcon(cat)" class="mr-1"></i>
                {{ getTeamToolCategoryDisplayName(cat) }}
              </button>
              <div class="ml-auto flex items-center gap-1">
                <button class="btn btn-xs btn-outline btn-success" @click="selectAllFilteredTeamTools">
                  <i class="fas fa-check mr-1"></i> 全选
                </button>
                <button class="btn btn-xs btn-outline btn-error" @click="deselectAllFilteredTeamTools">
                  <i class="fas fa-times mr-1"></i> 取消全选
                </button>
              </div>
            </div>

            <div class="max-h-72 overflow-y-auto border border-base-300 rounded-lg p-2 space-y-1">
              <label
                v-for="tool in filteredTeamTools"
                :key="tool.name"
                class="flex items-center gap-2 px-2 py-1 rounded hover:bg-base-200 cursor-pointer"
              >
                <input
                  type="checkbox"
                  class="checkbox checkbox-xs checkbox-primary"
                  :checked="teamToolConfig.allowlist.includes(tool.name)"
                  @change="toggleTeamTool(tool.name, ($event.target as HTMLInputElement).checked)"
                />
                <span class="text-xs font-mono text-base-content/85 flex-1 truncate">{{ tool.name }}</span>
                <span class="badge badge-xs" :class="getTeamToolCategoryBadgeClass(tool.category)">
                  {{ getTeamToolCategoryDisplayName(tool.category) }}
                </span>
                <span class="text-[11px] text-base-content/50 truncate max-w-64">{{ tool.description || '' }}</span>
              </label>
              <div v-if="filteredTeamTools.length === 0" class="text-xs text-base-content/50 px-2 py-1">
                无匹配工具
              </div>
            </div>
          </div>
        </div>
        <div class="modal-action">
          <button class="btn btn-ghost" @click="showTeamToolConfig = false">关闭</button>
          <button class="btn btn-primary" @click="saveTeamToolConfig">保存配置</button>
        </div>
      </div>
      <form method="dialog" class="modal-backdrop">
        <button @click.prevent="showTeamToolConfig = false">close</button>
      </form>
    </dialog>

  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch, nextTick } from 'vue'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { invoke } from '@tauri-apps/api/core'
import { save } from '@tauri-apps/plugin-dialog'
import { writeTextFile } from '@tauri-apps/plugin-fs'
import { agentTeamApi } from '@/api/agentTeam'
import { useToast } from '@/composables/useToast'
import { dialog } from '@/composables/useDialog'
import type {
  AgentTeamTemplate,
  AgentTeamSession,
  AgentTeamRound,
  AgentTeamMessage,
  AgentTeamBlackboardEntry,
  AgentTeamArtifact,
  AgentTeamRoleThinkingEvent,
  AgentTeamRoundEvent,
  AgentTeamStateChangedEvent,
  AgentTeamArtifactEvent,
  AgentTeamDivergenceAlertEvent,
  AgentTeamMessageStreamStartEvent,
  AgentTeamMessageStreamDeltaEvent,
  AgentTeamMessageStreamDoneEvent,
  AgentTeamToolCallEvent,
  AgentTeamToolResultEvent,
} from '@/types/agentTeam'
import AgentTeamTemplateLibrary from './AgentTeamTemplateLibrary.vue'
import AgentTeamBlackboardPanel from './AgentTeamBlackboardPanel.vue'
import AgentTeamArtifactPanel from './AgentTeamArtifactPanel.vue'
import AgentTeamTimeline from './AgentTeamTimeline.vue'
import AgentTeamChallengeSplitView from './AgentTeamChallengeSplitView.vue'
import TeamSessionList from './TeamSessionList.vue'
import MarkdownRenderer from './MarkdownRenderer.vue'
import ToolCallsDisplay from './ToolCallsDisplay.vue'

const toast = useToast()

// ==================== Props / Emits ====================

const props = defineProps<{
  conversationId?: string
}>()

// ==================== State ====================

const templates = ref<AgentTeamTemplate[]>([])
const session = ref<AgentTeamSession | null>(null)
const teamRounds = ref<AgentTeamRound[]>([])
const teamMessages = ref<AgentTeamMessage[]>([])
const blackboard = ref<AgentTeamBlackboardEntry[]>([])
const artifacts = ref<AgentTeamArtifact[]>([])

const selectedTemplateId = ref<string>('')
const isStarting = ref(false)
const isStopping = ref(false)
const localToolsEnabled = ref(true)
const showTeamToolConfig = ref(false)
const teamToolSearch = ref('')
type TeamToolOption = {
  name: string
  description?: string
  category: string
}
type TeamToolMetadata = {
  id: string
  name: string
  description?: string
  category?: string
}
const availableTeamTools = ref<TeamToolOption[]>([])
const selectedTeamToolCategories = ref<string[]>([])
const showSelectedTeamToolsOnly = ref(false)
const teamToolConfig = ref<{
  enabled: boolean
  allowlist: string[]
}>({
  enabled: true,
  allowlist: [],
})
const TEAM_TOOL_CONFIG_STORAGE_KEY = 'sentinel:team:tool-config'
const DEFAULT_TEAM_MAX_HUMAN_INTERVENTIONS = 2
const MIN_TEAM_MAX_HUMAN_INTERVENTIONS = 1
const MAX_TEAM_MAX_HUMAN_INTERVENTIONS = 10
const DEFAULT_TEMPLATE_MAX_ROUNDS = 5
const MIN_TEMPLATE_MAX_ROUNDS = 1
const MAX_TEMPLATE_MAX_ROUNDS = 10
const teamMaxHumanInterventions = ref<number>(DEFAULT_TEAM_MAX_HUMAN_INTERVENTIONS)
const autoResumeSecondsLeft = ref<number | null>(null)
const quickIntervening = ref(false)
let autoResumeCountdownTimer: ReturnType<typeof setInterval> | null = null

const showSidePanel = ref(false)
const activeSideTab = ref<'blackboard' | 'artifacts' | 'timeline' | 'challenge'>('blackboard')
const showTemplateLibrary = ref(false)
const showSessionManager = ref(false)
const teamSessions = ref<AgentTeamSession[]>([])
const sessionLoading = ref(false)
const sessionLoadingMore = ref(false)
const hasMoreSessions = ref(false)
const teamSessionOffset = ref(0)

const divergenceAlert = ref<{ divergence_score: number; threshold: number } | null>(null)
const latestDivergenceScore = ref<number | null>(null)

const teamInput = ref('')
const teamTextareaRef = ref<HTMLTextAreaElement | null>(null)
const isEditingGoal = ref(false)
const editableGoal = ref('')
const activeMemberId = ref<string | null>(null)
const activeMemberName = ref<string | null>(null)
const messageScrollRef = ref<HTMLElement | null>(null)
const shouldStickToBottom = ref(true)
const AUTO_SCROLL_THRESHOLD_PX = 120
const TEAM_SIDE_MIN_WIDTH = 280
const TEAM_SIDE_MAX_WIDTH = 760
const TEAM_SIDE_DEFAULT_WIDTH = 320
const teamSidePanelWidth = ref(TEAM_SIDE_DEFAULT_WIDTH)
const isTeamSideResizing = ref(false)
const preserveStreamingOnNextStateSync = ref(false)

const NON_RUNNING_STATES = new Set([
  'PENDING',
  'SUSPENDED_FOR_HUMAN',
  'COMPLETED',
  'FAILED',
  'ARCHIVED',
])

const isRunning = computed(() => {
  const state = session.value?.state
  if (!state) return false
  return !NON_RUNNING_STATES.has(state)
})

const canResendGoal = computed(() => {
  const goal = isEditingGoal.value
    ? editableGoal.value.trim()
    : session.value?.goal?.trim()
  return !!goal && !isStarting.value && !isStopping.value && !isRunning.value
})

const filteredTeamTools = computed(() => {
  let tools = availableTeamTools.value
  const q = teamToolSearch.value.trim().toLowerCase()
  if (q) {
    tools = tools.filter((t) =>
      t.name.toLowerCase().includes(q) || (t.description || '').toLowerCase().includes(q),
    )
  }
  if (showSelectedTeamToolsOnly.value) {
    const selected = new Set(teamToolConfig.value.allowlist)
    tools = tools.filter((t) => selected.has(t.name))
  }
  if (selectedTeamToolCategories.value.length === 0) return tools
  return tools.filter((t) => selectedTeamToolCategories.value.includes(t.category))
})

const allTeamToolCategories = computed(() => {
  const cats = new Set(availableTeamTools.value.map((t) => t.category || 'system'))
  cats.add('plugin')
  return Array.from(cats).sort()
})

type TeamRenderMessage = AgentTeamMessage & { is_streaming?: boolean }
type TeamStreamingMessage = TeamRenderMessage & { stream_id: string }
type TeamStreamingToolCallMessage = TeamRenderMessage & {
  stream_id: string
  tool_call_id: string
}

const streamingMessagesById = ref<Record<string, TeamStreamingMessage>>({})
const streamingToolCallsById = ref<Record<string, TeamStreamingToolCallMessage>>({})

const renderedMessages = computed<TeamRenderMessage[]>(() => {
  const base: TeamRenderMessage[] = []
  for (const m of teamMessages.value) {
    const normalized: TeamRenderMessage = { ...m, is_streaming: false }
    const persistedCalls = parseToolCallsForRender(m.tool_calls)
    if (persistedCalls.length > 0) {
      normalized.tool_calls = undefined
    }
    base.push(normalized)
    for (let i = 0; i < persistedCalls.length; i += 1) {
      const ts = new Date(new Date(m.timestamp).getTime() + i).toISOString()
      base.push({
        id: `${m.id}-tool-${i}`,
        session_id: m.session_id,
        round_id: m.round_id,
        member_id: m.member_id,
        member_name: m.member_name,
        role: 'tool_call',
        content: '',
        tool_calls: [persistedCalls[i]],
        token_count: undefined,
        timestamp: ts,
        is_streaming: false,
      } as TeamRenderMessage)
    }
  }
  const streamings = Object.values(streamingMessagesById.value)
  const streamingToolCalls = Object.values(streamingToolCallsById.value)
  if (streamings.length === 0 && streamingToolCalls.length === 0) return base
  const combined = [...base, ...streamings, ...streamingToolCalls]
  combined.sort((a, b) => {
    const at = new Date(a.timestamp).getTime()
    const bt = new Date(b.timestamp).getTime()
    return at - bt
  })
  return combined
})

const displayMessages = computed<TeamRenderMessage[]>(() => {
  const grouped: TeamRenderMessage[] = []
  for (const msg of renderedMessages.value) {
    if (msg.role !== 'tool_call') {
      grouped.push(msg)
      continue
    }

    const currentCalls = parseToolCallsForRender(msg.tool_calls)
    const last = grouped[grouped.length - 1]
    const canMergeToolBlock =
      !!last &&
      last.role === 'tool_call' &&
      (last.member_id || '') === (msg.member_id || '') &&
      (last.member_name || '') === (msg.member_name || '')

    if (canMergeToolBlock) {
      const prevCalls = parseToolCallsForRender(last.tool_calls)
      last.tool_calls = [...prevCalls, ...currentCalls]
      last.timestamp = msg.timestamp
      continue
    }

    grouped.push({
      ...msg,
      tool_calls: currentCalls,
    })
  }
  return grouped
})

function parseToolCallsForRender(toolCalls: unknown): any[] {
  if (!toolCalls) return []
  if (Array.isArray(toolCalls)) return toolCalls
  if (typeof toolCalls === 'string') {
    try {
      const parsed = JSON.parse(toolCalls)
      return Array.isArray(parsed) ? parsed : [parsed]
    } catch {
      return []
    }
  }
  if (typeof toolCalls === 'object') return [toolCalls]
  return []
}

// Tauri event unlisten fns
const unlistenFns: UnlistenFn[] = []

// ==================== Lifecycle ====================

onMounted(async () => {
  loadTeamSidePanelWidth()
  loadSavedGlobalTeamToolConfig()
  await loadAvailableTeamTools()
  await loadTemplates()
  await setupEventListeners()
  await loadTeamSessions(true)
  await restoreLastSession()
  await nextTick()
  autoResizeTeamInput()
})

onUnmounted(() => {
  unlistenFns.forEach(fn => fn())
  stopAutoResumeCountdown()
})

watch(() => session.value?.id, () => {
  shouldStickToBottom.value = true
})

watch(teamInput, async () => {
  await nextTick()
  autoResizeTeamInput()
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
    toast.error('加载模板列表失败')
  }
}

async function loadSessionData(sessionId: string, clearStreaming = true) {
  try {
    const [rounds, msgs, bb, arts] = await Promise.all([
      agentTeamApi.getRounds(sessionId),
      agentTeamApi.getMessages(sessionId),
      agentTeamApi.getBlackboard(sessionId),
      agentTeamApi.listArtifacts(sessionId),
    ])
    teamRounds.value = rounds
    teamMessages.value = msgs
    blackboard.value = bb
    artifacts.value = arts
    if (clearStreaming) {
      streamingMessagesById.value = {}
      streamingToolCallsById.value = {}
    }
    if (arts.length > 0) {
      showSidePanel.value = true
      activeSideTab.value = 'artifacts'
    }
    syncAutoResumeCountdown()
  } catch (e) {
    console.error('[AgentTeamView] Failed to load session data:', e)
    toast.error('加载会话数据失败')
  }
}

const TEAM_SESSION_PAGE_SIZE = 20

async function loadTeamSessions(reset = false) {
  if (reset) {
    sessionLoading.value = true
    teamSessionOffset.value = 0
    teamSessions.value = []
    hasMoreSessions.value = false
  } else {
    if (sessionLoading.value || sessionLoadingMore.value || !hasMoreSessions.value) return
    sessionLoadingMore.value = true
  }
  try {
    const list = await agentTeamApi.listSessions(
      props.conversationId,
      TEAM_SESSION_PAGE_SIZE,
      teamSessionOffset.value,
    )
    teamSessions.value = reset ? list : [...teamSessions.value, ...list]
    teamSessionOffset.value += list.length
    hasMoreSessions.value = list.length === TEAM_SESSION_PAGE_SIZE
  } catch (e) {
    console.error('[AgentTeamView] Failed to load team sessions:', e)
    toast.error('加载会话列表失败')
  } finally {
    if (reset) {
      sessionLoading.value = false
    } else {
      sessionLoadingMore.value = false
    }
  }
}

function toggleSessionManager() {
  showSessionManager.value = !showSessionManager.value
  if (showSessionManager.value) {
    loadTeamSessions(true)
  }
}

async function restoreLastSession() {
  try {
    const list = teamSessions.value.length > 0
      ? teamSessions.value.slice(0, 1)
      : await agentTeamApi.listSessions(props.conversationId, 1)
    const latestActive = list.find((s) => s.state !== 'ARCHIVED')
    if (!latestActive) return
    session.value = latestActive
    isEditingGoal.value = false
    editableGoal.value = ''
    hydrateTeamToolConfigFromSession(latestActive)
    showSidePanel.value = true
    activeSideTab.value = 'blackboard'
    await loadSessionData(latestActive.id)
  } catch (e) {
    console.warn('[AgentTeamView] Failed to restore last team session:', e)
  }
}

async function handleSelectSession(sessionId: string) {
  try {
    const s = await agentTeamApi.getSession(sessionId)
    if (!s) return
    session.value = s
    isEditingGoal.value = false
    editableGoal.value = ''
    hydrateTeamToolConfigFromSession(s)
    showSidePanel.value = true
    activeSideTab.value = 'blackboard'
    showSessionManager.value = false
    await loadSessionData(sessionId)
  } catch (e) {
    console.error('[AgentTeamView] Failed to select team session:', e)
    toast.error('切换会话失败')
  }
}

async function handleRenameSession(target: AgentTeamSession) {
  const nextName = window.prompt('请输入新的 Team 会话名称', target.name || '')
  if (!nextName) return
  const trimmed = nextName.trim()
  if (!trimmed || trimmed === target.name) return
  try {
    await agentTeamApi.updateSession(target.id, { name: trimmed })
    if (session.value?.id === target.id) {
      session.value = { ...session.value, name: trimmed }
    }
    await loadTeamSessions(true)
  } catch (e) {
    console.error('[AgentTeamView] Failed to rename team session:', e)
    toast.error('重命名会话失败')
  }
}

async function handleDeleteSession(target: AgentTeamSession) {
  const confirmed = await dialog.confirm({
    title: '删除会话',
    message: `确定要删除会话「${target.name}」吗？此操作不可撤销。`,
    variant: 'error',
    confirmText: '删除',
    cancelText: '取消',
  })
  if (!confirmed) return

  try {
    await agentTeamApi.deleteSession(target.id)
    const deletedCurrent = session.value?.id === target.id
    await loadTeamSessions(true)
    toast.success('会话已删除')

    if (deletedCurrent) {
      const next = teamSessions.value[0] ?? null
      if (next) {
        session.value = next
        showSidePanel.value = true
        activeSideTab.value = 'blackboard'
        await loadSessionData(next.id)
      } else {
        handleCreateNewSession()
      }
    }
  } catch (e) {
    console.error('[AgentTeamView] Failed to delete team session:', e)
    toast.error('删除会话失败')
  }
}

async function handleArchiveSession(target: AgentTeamSession) {
  const confirmed = await dialog.confirm({
    title: '归档会话',
    message: `确定要归档会话「${target.name}」吗？`,
    variant: 'warning',
    confirmText: '归档',
    cancelText: '取消',
  })
  if (!confirmed) return

  try {
    await agentTeamApi.updateSession(target.id, { state: 'ARCHIVED' })
    const archivedCurrent = session.value?.id === target.id
    await loadTeamSessions(true)
    toast.success('会话已归档')
    if (archivedCurrent) {
      const next = teamSessions.value.find((s) => s.state !== 'ARCHIVED') ?? null
      if (next) {
        session.value = next
        showSidePanel.value = true
        activeSideTab.value = 'blackboard'
        await loadSessionData(next.id)
      } else {
        handleCreateNewSession()
      }
    }
  } catch (e) {
    console.error('[AgentTeamView] Failed to archive team session:', e)
    toast.error('归档会话失败')
  }
}

async function handleRestoreSession(target: AgentTeamSession) {
  try {
    await agentTeamApi.updateSession(target.id, { state: 'PENDING' })
    await loadTeamSessions(true)
    toast.success('会话已恢复')
  } catch (e) {
    console.error('[AgentTeamView] Failed to restore team session:', e)
    toast.error('恢复会话失败')
  }
}

function handleCreateNewSession() {
  session.value = null
  teamRounds.value = []
  teamMessages.value = []
  blackboard.value = []
  artifacts.value = []
  isEditingGoal.value = false
  editableGoal.value = ''

  streamingMessagesById.value = {}
  streamingToolCallsById.value = {}
  activeMemberId.value = null
  activeMemberName.value = null
  showSessionManager.value = false
  showSidePanel.value = false
  divergenceAlert.value = null
  latestDivergenceScore.value = null
  preserveStreamingOnNextStateSync.value = false
  stopAutoResumeCountdown()
}

function handleLibrarySessionCreated(sessionId: string) {
  showTemplateLibrary.value = false
  // load the newly created session
  agentTeamApi.getSession(sessionId).then(s => {
    if (s) {
      session.value = s
      isEditingGoal.value = false
      editableGoal.value = ''
      hydrateTeamToolConfigFromSession(s)
      showSidePanel.value = true
      activeSideTab.value = 'blackboard'
      loadSessionData(sessionId)
      // Template library flow creates session only; auto start run for better UX.
      if (s.state === 'PENDING') {
        handleStartExistingSession()
      }
    }
  }).catch(e => {
    console.error('[AgentTeamView] Failed to load new session:', e)
    toast.error('加载新创建的会话失败')
  })
}

async function handleTemplatesUpdated(templateId?: string) {
  const previousSelected = selectedTemplateId.value
  await loadTemplates()
  if (templateId && templates.value.some((tpl) => tpl.id === templateId)) {
    selectedTemplateId.value = templateId
    return
  }
  if (previousSelected && templates.value.some((tpl) => tpl.id === previousSelected)) {
    selectedTemplateId.value = previousSelected
    return
  }
  selectedTemplateId.value = templates.value[0]?.id ?? ''
}

async function handleStartExistingSession() {
  if (!session.value) return
  isStarting.value = true
  try {
    await persistSessionToolPolicy(session.value.id)
    await agentTeamApi.startRun(session.value.id)
    toast.info('Team 会话已启动')
  } catch (e) {
    console.error('[AgentTeamView] Failed to start existing session:', e)
    toast.error('启动 Team 会话失败')
  } finally {
    isStarting.value = false
  }
}

async function handleStopRun() {
  if (!session.value || !isRunning.value) return
  isStopping.value = true
  preserveStreamingOnNextStateSync.value = true
  try {
    await persistStreamingMessagesBeforeStop()
    await agentTeamApi.stopRun(session.value.id)
    await refreshSession(false)
    activeMemberId.value = null
    activeMemberName.value = null
    toast.warning('Team 运行已停止')
  } catch (e) {
    preserveStreamingOnNextStateSync.value = false
    console.error('[AgentTeamView] Failed to stop team run:', e)
    toast.error('停止 Team 运行失败')
  } finally {
    isStopping.value = false
  }
}

async function persistStreamingMessagesBeforeStop() {
  if (!session.value) return
  const toolCallsByStream = new Map<string, any[]>()
  for (const tcMsg of Object.values(streamingToolCallsById.value)) {
    const calls = parseToolCallsForRender(tcMsg.tool_calls)
    if (calls.length === 0) continue
    const existing = toolCallsByStream.get(tcMsg.stream_id) || []
    toolCallsByStream.set(tcMsg.stream_id, [...existing, ...calls])
  }

  const partials = Object.values(streamingMessagesById.value)
    .map((msg) => ({
      msg,
      toolCalls: toolCallsByStream.get(msg.stream_id) || [],
    }))
    .filter(({ msg, toolCalls }) => msg.is_streaming && (msg.content.trim().length > 0 || toolCalls.length > 0))
    .sort((a, b) => new Date(a.msg.timestamp).getTime() - new Date(b.msg.timestamp).getTime())

  if (partials.length === 0) return

  for (const partial of partials) {
    const { msg, toolCalls } = partial
    try {
      await agentTeamApi.appendPartialMessage({
        session_id: session.value.id,
        member_id: msg.member_id,
        member_name: msg.member_name,
        role: msg.role || 'assistant',
        content: msg.content || '',
        tool_calls: toolCalls.length > 0 ? toolCalls : undefined,
      })
    } catch (e) {
      console.error('[AgentTeamView] Failed to persist partial streaming message before stop:', e)
    }
  }
}

// ==================== Team input ====================

type QuickInterventionPolicy = 'conservative' | 'balanced' | 'aggressive'

function getQuickInterventionPrompt(policy: QuickInterventionPolicy): string {
  if (policy === 'conservative') {
    return '请按保守收敛策略继续：优先安全与稳定，选择风险最低且可回滚方案。请输出唯一执行方案、放弃理由和执行步骤。'
  }
  if (policy === 'aggressive') {
    return '请按激进推进策略继续：优先交付速度与产出，选择实现最快方案，同时给出关键风险与回滚预案。请输出唯一执行方案、放弃理由和执行步骤。'
  }
  return '请按平衡收敛策略继续：在风险、成本、质量间做均衡取舍，输出可执行的单一方案、放弃理由和执行步骤。'
}

async function handleQuickIntervention(policy: QuickInterventionPolicy) {
  if (!session.value || session.value.state !== 'SUSPENDED_FOR_HUMAN' || quickIntervening.value) return
  quickIntervening.value = true
  try {
    await agentTeamApi.submitMessage({
      session_id: session.value.id,
      content: getQuickInterventionPrompt(policy),
      resume: true,
    })
    await refreshSession()
    const label = policy === 'conservative' ? '保守' : policy === 'aggressive' ? '激进' : '平衡'
    toast.info(`已按${label}策略继续执行`)
  } catch (e) {
    console.error('[AgentTeamView] Failed to submit quick intervention:', e)
    toast.error('快速介入失败')
  } finally {
    quickIntervening.value = false
  }
}

function formatCountdown(totalSeconds: number): string {
  const sec = Math.max(0, Math.trunc(totalSeconds))
  const mm = String(Math.floor(sec / 60)).padStart(2, '0')
  const ss = String(sec % 60).padStart(2, '0')
  return `${mm}:${ss}`
}

function stopAutoResumeCountdown() {
  if (autoResumeCountdownTimer !== null) {
    clearInterval(autoResumeCountdownTimer)
    autoResumeCountdownTimer = null
  }
  autoResumeSecondsLeft.value = null
}

function syncAutoResumeCountdown() {
  stopAutoResumeCountdown()
  const current = session.value
  if (!current || current.state !== 'SUSPENDED_FOR_HUMAN') return
  const autoResumeAt = (current.state_machine as any)?.human_intervention?.auto_resume_at
  if (typeof autoResumeAt !== 'string' || !autoResumeAt) return
  const autoResumeAtMs = new Date(autoResumeAt).getTime()
  if (!Number.isFinite(autoResumeAtMs)) return

  const update = () => {
    const left = Math.ceil((autoResumeAtMs - Date.now()) / 1000)
    autoResumeSecondsLeft.value = left > 0 ? left : 0
    if (left <= 0) {
      stopAutoResumeCountdown()
    }
  }

  update()
  autoResumeCountdownTimer = setInterval(update, 1000)
}

async function handleTeamSend() {
  const content = teamInput.value.trim()
  if (!content) return

  if (!session.value) {
    isStarting.value = true
    try {
      const templateId = selectedTemplateId.value || templates.value[0]?.id
      if (!templateId) {
        toast.error('没有可用模板，请先创建或导入模板')
        return
      }
      const newSession = await createAndStartSession(content, templateId)
      session.value = newSession
      hydrateTeamToolConfigFromSession(newSession)
      showSidePanel.value = true
      activeSideTab.value = 'blackboard'
      teamInput.value = ''
      await loadTeamSessions(true)
      toast.success('Team 会话已创建并启动')
    } catch (e) {
      console.error('[AgentTeamView] Failed to create/start session from input:', e)
      toast.error('创建 Team 会话失败')
    } finally {
      isStarting.value = false
    }
    return
  }

  try {
    if (session.value.state === 'PENDING') {
      await persistSessionToolPolicy(session.value.id)
    }
    await agentTeamApi.submitMessage({
      session_id: session.value.id,
      content,
      resume: session.value.state === 'SUSPENDED_FOR_HUMAN',
    })
    teamInput.value = ''
    await refreshSession()
    if (session.value?.state === 'SUSPENDED_FOR_HUMAN') {
      toast.info('人工意见已提交，会话恢复中...')
    }
  } catch (e) {
    console.error('[AgentTeamView] Failed to submit team input message:', e)
    toast.error('发送消息失败')
  }
}

async function createAndStartSession(goal: string, templateId: string): Promise<AgentTeamSession> {
  let template = templates.value.find((tpl) => tpl.id === templateId)
  try {
    const latest = await agentTeamApi.getTemplate(templateId)
    if (latest) {
      template = latest
    }
  } catch (e) {
    console.warn('[AgentTeamView] Failed to fetch latest template before session creation:', e)
  }
  const maxRounds = extractTemplateMaxRoundsFromConfig(template?.default_rounds_config)
  const newSession = await agentTeamApi.createSession({
    name: `Team: ${goal.slice(0, 30)}`,
    goal,
    template_id: templateId,
    conversation_id: props.conversationId,
    max_rounds: maxRounds,
    state_machine: buildSessionStateMachinePatch(),
  })
  await agentTeamApi.startRun(newSession.id)
  return newSession
}

async function handleResendGoal() {
  if (!session.value || !canResendGoal.value) return
  const goal = isEditingGoal.value
    ? editableGoal.value.trim()
    : session.value.goal?.trim()
  if (!goal) return

  isStarting.value = true
  try {
    const templateId = session.value.template_id || selectedTemplateId.value || templates.value[0]?.id
    if (!templateId) {
      toast.error('没有可用模板，请先创建或导入模板')
      return
    }
    selectedTemplateId.value = templateId
    const newSession = await createAndStartSession(goal, templateId)
    session.value = newSession
    isEditingGoal.value = false
    editableGoal.value = ''
    hydrateTeamToolConfigFromSession(newSession)
    showSidePanel.value = true
    activeSideTab.value = 'blackboard'
    teamInput.value = ''
    await loadSessionData(newSession.id)
    await loadTeamSessions(true)
    toast.success('已按团队目标重新启动')
  } catch (e) {
    console.error('[AgentTeamView] Failed to resend goal as restart:', e)
    toast.error('重新发送失败')
  } finally {
    isStarting.value = false
  }
}

function startEditGoal() {
  editableGoal.value = session.value?.goal || ''
  isEditingGoal.value = true
}

function cancelEditGoal() {
  editableGoal.value = session.value?.goal || ''
  isEditingGoal.value = false
}

async function loadAvailableTeamTools() {
  try {
    const metadata = await invoke<TeamToolMetadata[]>('get_all_tool_metadata')
    availableTeamTools.value = (metadata || [])
      .filter((t) => t && typeof t.name === 'string' && t.name.trim().length > 0)
      .sort((a, b) => a.name.localeCompare(b.name))
      .map((t) => ({
        name: t.name,
        description: t.description,
        category: (t.category || 'system').toLowerCase(),
      }))
  } catch (e) {
    console.warn('[AgentTeamView] Failed to load team tool metadata, fallback to list_tool_server_tools:', e)
    try {
      const tools = await invoke<Array<{ name: string; description?: string }>>('list_tool_server_tools')
      availableTeamTools.value = (tools || [])
        .filter((t) => t && typeof t.name === 'string' && t.name.trim().length > 0)
        .sort((a, b) => a.name.localeCompare(b.name))
        .map((t) => ({
          name: t.name,
          description: t.description,
          category: 'system',
        }))
    } catch (inner) {
      console.warn('[AgentTeamView] Failed to load team tool options:', inner)
      availableTeamTools.value = []
    }
  }
}

function toggleTeamTools() {
  localToolsEnabled.value = !localToolsEnabled.value
  teamToolConfig.value.enabled = localToolsEnabled.value
  saveGlobalTeamToolConfig()
}

function toggleTeamTool(toolName: string, checked: boolean) {
  const set = new Set(teamToolConfig.value.allowlist)
  if (checked) {
    set.add(toolName)
  } else {
    set.delete(toolName)
  }
  teamToolConfig.value.allowlist = Array.from(set)
}

function selectAllFilteredTeamTools() {
  const set = new Set(teamToolConfig.value.allowlist)
  filteredTeamTools.value.forEach((t) => set.add(t.name))
  teamToolConfig.value.allowlist = Array.from(set)
}

function deselectAllFilteredTeamTools() {
  const filtered = new Set(filteredTeamTools.value.map((t) => t.name))
  teamToolConfig.value.allowlist = teamToolConfig.value.allowlist.filter((name) => !filtered.has(name))
}

function toggleTeamToolCategory(category: string) {
  const idx = selectedTeamToolCategories.value.indexOf(category)
  if (idx >= 0) {
    selectedTeamToolCategories.value.splice(idx, 1)
  } else {
    selectedTeamToolCategories.value.push(category)
  }
}

function toggleShowSelectedTeamTools() {
  showSelectedTeamToolsOnly.value = !showSelectedTeamToolsOnly.value
  if (showSelectedTeamToolsOnly.value) {
    selectedTeamToolCategories.value = []
  }
}

function clearTeamToolFilters() {
  selectedTeamToolCategories.value = []
  showSelectedTeamToolsOnly.value = false
}

function getTeamToolCategoryDisplayName(category: string) {
  const map: Record<string, string> = {
    network: '网络',
    security: '安全',
    data: '数据',
    ai: 'AI',
    system: '系统',
    mcp: 'MCP',
    plugin: '插件',
    workflow: '工作流',
    browser: '浏览器',
  }
  return map[category.toLowerCase()] || category
}

function getTeamToolCategoryBadgeClass(category: string) {
  const map: Record<string, string> = {
    network: 'btn-info',
    security: 'btn-error',
    data: 'btn-success',
    ai: 'btn-warning',
    system: 'btn-neutral',
    mcp: 'btn-primary',
    plugin: 'btn-secondary',
    workflow: 'btn-accent',
    browser: 'btn-primary',
  }
  return map[category.toLowerCase()] || 'btn-ghost'
}

function getTeamToolCategoryIcon(category: string) {
  const map: Record<string, string> = {
    network: 'fas fa-network-wired',
    security: 'fas fa-shield-alt',
    data: 'fas fa-database',
    ai: 'fas fa-brain',
    system: 'fas fa-cog',
    mcp: 'fas fa-plug',
    plugin: 'fas fa-puzzle-piece',
    workflow: 'fas fa-project-diagram',
    browser: 'fas fa-globe',
  }
  return map[category.toLowerCase()] || 'fas fa-tools'
}

function buildSessionToolPolicyPayload() {
  const payload: any = {
    enabled: !!teamToolConfig.value.enabled,
  }
  if (teamToolConfig.value.enabled && teamToolConfig.value.allowlist.length > 0) {
    payload.allowlist = Array.from(
      new Set(teamToolConfig.value.allowlist.filter((n) => typeof n === 'string' && n.trim().length > 0)),
    )
  }
  return payload
}

function normalizeHumanInterventionMax(value: unknown): number {
  const n = Number(value)
  if (!Number.isFinite(n)) return DEFAULT_TEAM_MAX_HUMAN_INTERVENTIONS
  const normalized = Math.trunc(n)
  return Math.max(MIN_TEAM_MAX_HUMAN_INTERVENTIONS, Math.min(MAX_TEAM_MAX_HUMAN_INTERVENTIONS, normalized))
}

function normalizeTemplateMaxRounds(value: unknown): number {
  const n = Number(value)
  if (!Number.isFinite(n)) return DEFAULT_TEMPLATE_MAX_ROUNDS
  const normalized = Math.trunc(n)
  return Math.max(MIN_TEMPLATE_MAX_ROUNDS, Math.min(MAX_TEMPLATE_MAX_ROUNDS, normalized))
}

function extractTemplateMaxRoundsFromConfig(config: unknown): number {
  if (typeof config === 'number') {
    return normalizeTemplateMaxRounds(config)
  }
  if (config && typeof config === 'object') {
    const obj = config as Record<string, unknown>
    const candidate =
      obj.max_rounds ??
      obj.maxRounds ??
      obj.default_rounds ??
      obj.rounds
    return normalizeTemplateMaxRounds(candidate)
  }
  return DEFAULT_TEMPLATE_MAX_ROUNDS
}

function normalizeTeamMaxHumanInterventions() {
  teamMaxHumanInterventions.value = normalizeHumanInterventionMax(teamMaxHumanInterventions.value)
}

function buildSessionStateMachinePatch() {
  return {
    tool_policy: buildSessionToolPolicyPayload(),
    max_human_interventions: normalizeHumanInterventionMax(teamMaxHumanInterventions.value),
  }
}

function loadSavedGlobalTeamToolConfig() {
  try {
    const raw = localStorage.getItem(TEAM_TOOL_CONFIG_STORAGE_KEY)
    if (!raw) return
    const parsed = JSON.parse(raw)
    const policy = parsed?.tool_policy && typeof parsed.tool_policy === 'object'
      ? parsed.tool_policy
      : parsed
    const enabled = typeof policy?.enabled === 'boolean' ? policy.enabled : true
    const allowlist = Array.isArray(policy?.allowlist)
      ? policy.allowlist.filter((n: any) => typeof n === 'string' && n.trim().length > 0)
      : []
    const maxHumanInterventions = normalizeHumanInterventionMax(parsed?.max_human_interventions)
    localToolsEnabled.value = enabled
    teamToolConfig.value = { enabled, allowlist }
    teamMaxHumanInterventions.value = maxHumanInterventions
  } catch (e) {
    console.warn('[AgentTeamView] Failed to load saved team tool config:', e)
  }
}

function saveGlobalTeamToolConfig() {
  try {
    normalizeTeamMaxHumanInterventions()
    localStorage.setItem(TEAM_TOOL_CONFIG_STORAGE_KEY, JSON.stringify(buildSessionStateMachinePatch()))
  } catch (e) {
    console.warn('[AgentTeamView] Failed to save global team tool config:', e)
  }
}

function hydrateTeamToolConfigFromSession(nextSession: AgentTeamSession | null) {
  const policy = nextSession?.state_machine?.tool_policy
  if (policy && typeof policy === 'object') {
    const enabled = typeof policy?.enabled === 'boolean' ? policy.enabled : true
    const allowlist = Array.isArray(policy?.allowlist)
      ? policy.allowlist.filter((n: any) => typeof n === 'string' && n.trim().length > 0)
      : []
    localToolsEnabled.value = enabled
    teamToolConfig.value = {
      enabled,
      allowlist,
    }
  }
  const maxHumanInterventions = nextSession?.state_machine?.max_human_interventions
  teamMaxHumanInterventions.value = normalizeHumanInterventionMax(
    maxHumanInterventions ?? teamMaxHumanInterventions.value,
  )
}

async function persistSessionToolPolicy(sessionId: string) {
  const current = await agentTeamApi.getSession(sessionId)
  if (!current) return
  const nextStateMachine = {
    ...(current.state_machine || {}),
    ...buildSessionStateMachinePatch(),
  }
  await agentTeamApi.updateSession(sessionId, {
    state_machine: nextStateMachine,
  })
}

async function saveTeamToolConfig() {
  try {
    saveGlobalTeamToolConfig()
    if (session.value) {
      await persistSessionToolPolicy(session.value.id)
    }
    showTeamToolConfig.value = false
    toast.success('Team 工具配置已保存')
  } catch (e) {
    console.error('[AgentTeamView] Failed to save team tool config:', e)
    toast.error('保存 Team 工具配置失败')
  }
}

function autoResizeTeamInput() {
  const el = teamTextareaRef.value
  if (!el) return
  el.style.height = 'auto'
  el.style.height = `${Math.min(el.scrollHeight, 160)}px`
}

function onTeamInput() {
  autoResizeTeamInput()
}

function onTeamTextareaKeydown(e: KeyboardEvent) {
  if (e.key === 'Enter' && !e.shiftKey) {
    e.preventDefault()
    handleTeamSend()
  }
}

// ==================== Event Listeners ====================

async function setupEventListeners() {
  const unlistenStateChanged = await listen<AgentTeamStateChangedEvent>('agent_team:state_changed', async (event) => {
    if (!session.value || event.payload.session_id !== session.value.id) return
    session.value = { ...session.value, state: event.payload.state }
    const shouldClearStreaming = !(isStopping.value || preserveStreamingOnNextStateSync.value)
    await refreshSession(shouldClearStreaming)
    preserveStreamingOnNextStateSync.value = false
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
  })

  const unlistenArtifactGenerated = await listen<AgentTeamArtifactEvent>('agent_team:artifact_generated', async (event) => {
    if (!session.value || event.payload.session_id !== session.value.id) return
    artifacts.value = await agentTeamApi.listArtifacts(session.value.id)
    showSidePanel.value = true
    activeSideTab.value = 'artifacts'
    toast.success(`产物已生成: ${event.payload.title}`)
  })

  // Divergence alert event
  const unlistenDivergenceAlert = await listen<AgentTeamDivergenceAlertEvent>('agent_team:divergence_alert', (event) => {
    if (!session.value || event.payload.session_id !== session.value.id) return
    divergenceAlert.value = {
      divergence_score: event.payload.divergence_score,
      threshold: event.payload.threshold,
    }
    latestDivergenceScore.value = event.payload.divergence_score
    toast.warning(`分歧度告警: ${(event.payload.divergence_score * 100).toFixed(0)}%`)
  })

  const unlistenComplete = await listen('agent_team:complete', async (event) => {
    if (!session.value) return
    activeMemberId.value = null
    activeMemberName.value = null
    await refreshSession()
    streamingMessagesById.value = {}
    streamingToolCallsById.value = {}
    toast.success('Team 会话已完成')
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

  const upsertStreamingToolCall = (
    payload: {
      session_id: string
      stream_id: string
      member_id?: string
      member_name?: string
      tool_call_id: string
      name?: string
      arguments?: string
      result?: string
      success?: boolean
      timestamp?: string
    },
  ) => {
    const id = `tool-${payload.stream_id}-${payload.tool_call_id}`
    const prev = streamingToolCallsById.value[id]
    const existingCall = prev?.tool_calls && Array.isArray(prev.tool_calls)
      ? prev.tool_calls[0]
      : {}
    const mergedCall = {
      ...existingCall,
      id: payload.tool_call_id,
      name: payload.name ?? existingCall?.name ?? 'tool',
      arguments: payload.arguments ?? existingCall?.arguments,
      result: payload.result ?? existingCall?.result,
      success: payload.success ?? existingCall?.success,
    }
    streamingToolCallsById.value = {
      ...streamingToolCallsById.value,
      [id]: {
        stream_id: payload.stream_id,
        tool_call_id: payload.tool_call_id,
        id,
        session_id: payload.session_id,
        member_id: payload.member_id,
        member_name: payload.member_name || prev?.member_name,
        role: 'tool_call',
        content: '',
        tool_calls: [mergedCall],
        timestamp: payload.timestamp || new Date().toISOString(),
        is_streaming: true,
      } as TeamStreamingToolCallMessage,
    }
  }

  const unlistenToolCall = await listen<AgentTeamToolCallEvent>('agent_team:tool_call', (event) => {
    if (!session.value || event.payload.session_id !== session.value.id) return
    upsertStreamingToolCall({
      session_id: event.payload.session_id,
      stream_id: event.payload.stream_id,
      member_id: event.payload.member_id,
      member_name: event.payload.member_name,
      tool_call_id: event.payload.tool_call_id,
      name: event.payload.name,
      arguments: event.payload.arguments,
      timestamp: event.payload.timestamp,
    })
  })

  const unlistenToolResult = await listen<AgentTeamToolResultEvent>('agent_team:tool_result', (event) => {
    if (!session.value || event.payload.session_id !== session.value.id) return
    upsertStreamingToolCall({
      session_id: event.payload.session_id,
      stream_id: event.payload.stream_id,
      member_id: event.payload.member_id,
      member_name: event.payload.member_name,
      tool_call_id: event.payload.tool_call_id,
      result: event.payload.result,
      success: event.payload.success,
      timestamp: event.payload.timestamp,
    })
  })

  unlistenFns.push(
    unlistenStateChanged,
    unlistenRoleThinking,
    unlistenRoundStarted,
    unlistenRoundCompleted,
    unlistenArtifactGenerated,
    unlistenDivergenceAlert,
    unlistenComplete,
    unlistenMessageStreamStart,
    unlistenMessageStreamDelta,
    unlistenMessageStreamDone,
    unlistenToolCall,
    unlistenToolResult,
  )
}

async function refreshSession(clearStreaming = true) {
  if (!session.value) return
  try {
    const updated = await agentTeamApi.getSession(session.value.id)
    if (updated) {
      session.value = updated
      hydrateTeamToolConfigFromSession(updated)
    }
    await loadSessionData(session.value.id, clearStreaming)
    syncAutoResumeCountdown()
  } catch (e) {
    console.error('[AgentTeamView] Failed to refresh session:', e)
    toast.error('刷新会话状态失败')
  }
}

// ==================== Timeline & Side Panel Data ====================

const timelineRounds = computed(() => {
  if (teamRounds.value.length > 0) {
    return teamRounds.value.map((round) => ({
      id: round.id,
      round_number: round.round_number,
      phase: round.phase,
      divergence_score: round.divergence_score ?? null,
      status: round.status,
      started_at: round.started_at ?? null,
      completed_at: round.completed_at ?? null,
    }))
  }
  return []
})

// ==================== Blackboard Panel Handlers ====================

async function handleResolveBlackboardEntry(entryId: string) {
  // For now just log - can be extended when backend supports resolve
  toast.info('标记条目为已解决')
}

async function handleAddBlackboardEntry(type: string, title: string, content: string) {
  if (!session.value) return
  try {
    await agentTeamApi.addBlackboardEntry({
      session_id: session.value.id,
      entry_type: type,
      title,
      content,
    })
    // Reload blackboard
    blackboard.value = await agentTeamApi.getBlackboard(session.value.id)
    toast.success('白板条目已添加')
  } catch (e) {
    console.error('[AgentTeamView] Failed to add blackboard entry:', e)
    toast.error('添加白板条目失败')
  }
}

async function handleAnnotateBlackboardEntry(entryId: string, text: string) {
  // Annotation can be implemented as a new blackboard entry referencing the original
  if (!session.value) return
  try {
    await agentTeamApi.addBlackboardEntry({
      session_id: session.value.id,
      entry_type: 'action_item',
      title: `批注 - ${entryId.slice(0, 8)}`,
      content: text,
      contributed_by: '人工批注',
    })
    blackboard.value = await agentTeamApi.getBlackboard(session.value.id)
    toast.success('批注已添加')
  } catch (e) {
    console.error('[AgentTeamView] Failed to annotate blackboard entry:', e)
    toast.error('添加批注失败')
  }
}

// Auto-scroll messages
watch(displayMessages, async () => {
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

function loadTeamSidePanelWidth() {
  try {
    const raw = localStorage.getItem('sentinel:team:side-panel:width')
    if (!raw) return
    const width = parseInt(raw, 10)
    if (Number.isFinite(width) && width >= TEAM_SIDE_MIN_WIDTH && width <= TEAM_SIDE_MAX_WIDTH) {
      teamSidePanelWidth.value = width
    }
  } catch (e) {
    console.warn('[AgentTeamView] Failed to load side panel width:', e)
  }
}

function saveTeamSidePanelWidth(width: number) {
  try {
    localStorage.setItem('sentinel:team:side-panel:width', String(width))
  } catch (e) {
    console.warn('[AgentTeamView] Failed to save side panel width:', e)
  }
}

function startTeamSideResize(e: MouseEvent) {
  e.preventDefault()
  isTeamSideResizing.value = true
  const startX = e.clientX
  const startWidth = teamSidePanelWidth.value

  document.body.classList.add('resizing')
  document.body.style.cursor = 'col-resize'

  const onMouseMove = (moveEvent: MouseEvent) => {
    if (!isTeamSideResizing.value) return
    const delta = startX - moveEvent.clientX
    const next = Math.max(TEAM_SIDE_MIN_WIDTH, Math.min(TEAM_SIDE_MAX_WIDTH, startWidth + delta))
    teamSidePanelWidth.value = next
  }

  const onMouseUp = () => {
    if (isTeamSideResizing.value) {
      isTeamSideResizing.value = false
      saveTeamSidePanelWidth(teamSidePanelWidth.value)
    }
    document.body.classList.remove('resizing')
    document.body.style.cursor = ''
    document.removeEventListener('mousemove', onMouseMove)
    document.removeEventListener('mouseup', onMouseUp)
  }

  document.addEventListener('mousemove', onMouseMove)
  document.addEventListener('mouseup', onMouseUp)
}

// ==================== Artifact helpers ====================

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
    toast.success('产物已导出')
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
      toast.success('产物已导出（浏览器下载）')
    } catch (fallbackError) {
      console.error('[AgentTeamView] Fallback download also failed:', fallbackError)
      toast.error('导出产物失败')
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

function roleAvatarClass(role: string, memberName?: string): string {
  if (role === 'user') return 'bg-neutral text-neutral-content'
  if (role === 'tool_call') return 'bg-warning/20 text-warning'
  const colors = ['bg-primary/20 text-primary', 'bg-secondary/20 text-secondary', 'bg-accent/20 text-accent', 'bg-success/20 text-success']
  const idx = (memberName?.charCodeAt(0) ?? 0) % colors.length
  return colors[idx]
}

function roleInitial(memberName?: string, role?: string): string {
  if (memberName) return memberName.charAt(0).toUpperCase()
  if (role === 'user') return 'U'
  if (role === 'tool_call') return 'T'
  return 'A'
}

function roleDisplayName(role: string): string {
  if (role === 'user') return '人工介入'
  if (role === 'system') return '系统'
  if (role === 'tool_call') return '工具调用'
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

function formatTeamToolCalls(toolCalls: unknown): string {
  if (!toolCalls) return ''
  if (typeof toolCalls === 'string') {
    return toolCalls
  }
  try {
    return JSON.stringify(toolCalls)
  } catch {
    return ''
  }
}

function formatTokens(tokens: number): string {
  if (tokens >= 1000) return `${(tokens / 1000).toFixed(1)}k`
  return String(tokens)
}
</script>

<style scoped>
.agent-team-view {
  font-family: inherit;
}

.template-card {
  cursor: pointer;
}

.input-area-container {
  width: 100%;
  max-width: 100%;
}

.chat-input {
  position: relative;
}

.icon-btn {
  width: 1.75rem;
  height: 1.75rem;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 0.375rem;
  font-size: calc(var(--font-size-base, 14px) * 0.75);
  transition: background-color .15s, color .15s;
}

.icon-btn:hover {
  background-color: hsl(var(--b3)/0.7);
}
.icon-btn.active {
  background: hsl(var(--p));
  color: hsl(var(--pc));
  box-shadow: 0 2px 4px rgba(0, 0, 0, 0.15);
}

.send-btn {
  width: 2rem;
  height: 2rem;
  border-radius: 9999px;
  background: hsl(var(--b3));
  color: hsl(var(--bc));
  display: flex;
  align-items: center;
  justify-content: center;
  transition: background-color .15s, color .15s;
}

.send-btn:hover {
  background: hsl(var(--p));
  color: hsl(var(--pc));
}

.send-btn:disabled {
  opacity: .4;
  cursor: not-allowed;
}

.resize-handle {
  transition: background-color 0.2s;
}

.resize-handle:hover {
  width: 4px;
}

:global(body.resizing) {
  user-select: none;
  cursor: col-resize !important;
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

.team-tool-calls:deep(.tool-call-item) {
  background: transparent;
  border: none;
  border-radius: 0;
  overflow: visible;
}

.team-tool-calls:deep(.tool-calls-display) {
  gap: 0.25rem;
}

.members-bar::-webkit-scrollbar {
  height: 0;
}

.slide-library-enter-active,
.slide-library-leave-active {
  transition: all 0.25s ease;
}
.slide-library-enter-from .drawer-right,
.slide-library-leave-to .drawer-right {
  transform: translateX(100%);
}
.slide-library-enter-from .drawer-left,
.slide-library-leave-to .drawer-left {
  transform: translateX(-100%);
}
.slide-library-enter-from,
.slide-library-leave-to {
  opacity: 0;
}

@media (max-width: 768px) {
  .side-panel {
    width: 100% !important;
    border-left: none;
    border-top: 1px solid hsl(var(--b3));
  }

  .resize-handle {
    display: none;
  }
}
</style>
