<template>
  <div class="page-content-padded safe-top min-h-0">
    <div class="flex flex-col gap-4">
      <div class="flex flex-col gap-2 lg:flex-row lg:items-end lg:justify-between">
        <div>
          <h2 class="text-2xl font-bold">{{ t('botConsole.title') }}</h2>
          <p class="text-sm text-base-content/70 mt-1">
            {{ t('botConsole.description') }}
          </p>
        </div>

        <div class="flex flex-wrap items-center gap-2">
          <select v-model="selectedTransport" class="select select-bordered select-sm min-w-36" @change="handleTransportChange">
            <option value="">{{ t('botConsole.allTransports') }}</option>
            <option v-for="transport in transportOptions" :key="transport" :value="transport">
              {{ transportLabel(transport) }}
            </option>
          </select>

          <select v-model="selectedPeerType" class="select select-bordered select-sm min-w-32" @change="handlePeerFilterChange">
            <option value="">{{ t('botConsole.allPeerTypes') }}</option>
            <option value="dm">{{ t('botConsole.dm') }}</option>
            <option value="group">{{ t('botConsole.group') }}</option>
          </select>

          <select v-model="peerStateFilter" class="select select-bordered select-sm min-w-36" @change="handlePeerStateFilterChange">
            <option value="all">{{ t('botConsole.peerFilters.all') }}</option>
            <option value="failures">{{ t('botConsole.peerFilters.failures') }}</option>
            <option value="running">{{ t('botConsole.peerFilters.running') }}</option>
          </select>

          <button class="btn btn-sm btn-primary" :disabled="loadingPeers || loadingDetails" @click="refreshConsole">
            <span v-if="loadingPeers || loadingDetails" class="loading loading-spinner loading-xs"></span>
            {{ t('botConsole.refresh') }}
          </button>
        </div>
      </div>

      <div v-if="loadError" class="alert alert-error">
        <span>{{ loadError }}</span>
      </div>

      <div v-if="!isBotConsoleActivated" class="alert alert-warning">
        <i class="fas fa-lock"></i>
        <span>当前未完成本地 License 激活，Bot 控制台可预览会话、消息和历史执行，账号配置、登录、Mission 变更和运行会被限制。</span>
      </div>

      <section class="rounded-lg border border-base-300 bg-base-100 px-4 py-3">
        <div class="flex flex-col gap-3 lg:flex-row lg:items-center lg:justify-between">
          <div class="min-w-0">
            <div class="text-sm font-semibold">{{ t('botConsole.accounts.summaryTitle') }}</div>
            <div class="mt-1 flex flex-wrap items-center gap-2 text-xs text-base-content/70">
              <span class="badge badge-outline badge-sm">
                {{ t('botConsole.accounts.currentTransport') }}: {{ selectedTransport ? transportLabel(selectedTransport) : t('botConsole.labels.all') }}
              </span>
              <span class="badge badge-ghost badge-sm">
                {{ t('botConsole.accounts.accountCount', { count: visibleAccountCount }) }}
              </span>
              <span class="badge badge-ghost badge-sm">
                {{ t('botConsole.accounts.runningCount', { count: runningAccountCount }) }}
              </span>
              <span v-if="failedAccountCount > 0" class="badge badge-error badge-sm">
                {{ t('botConsole.accounts.failedCount', { count: failedAccountCount }) }}
              </span>
              <span v-if="latestVisibleAccountAt" class="badge badge-ghost badge-sm">
                {{ t('botConsole.accounts.lastSeenAt') }}: {{ formatTimestamp(latestVisibleAccountAt) }}
              </span>
              <span v-if="selectedBotAccount" class="badge badge-primary badge-sm max-w-full truncate">
                {{ t('botConsole.accounts.selectedAccount') }}: {{ selectedBotAccount.display_name || selectedBotAccount.account_id }}
              </span>
            </div>
          </div>

          <div class="flex items-center justify-end">
            <button class="btn btn-sm btn-outline" :disabled="!isBotConsoleActivated" @click="openAccountsDialog">
              {{ t('botConsole.accounts.configure') }}
            </button>
          </div>
        </div>
      </section>

      <div class="grid min-h-0 grid-cols-1 gap-4 xl:grid-cols-[22rem_minmax(0,1fr)]">
        <section class="rounded-lg border border-base-300 bg-base-100 min-h-[42rem] overflow-hidden">
          <div class="border-b border-base-300 px-4 py-3">
            <div class="flex items-center justify-between gap-3">
              <div>
                <div class="text-sm font-semibold">{{ t('botConsole.sessionsTitle') }}</div>
                <div class="text-xs text-base-content/60">
                  {{ t('botConsole.sessionsCount', { count: filteredPeers.length }) }}
                </div>
              </div>
              <span class="badge badge-ghost badge-sm">
                {{ selectedTransport ? transportLabel(selectedTransport) : t('botConsole.labels.all') }}
              </span>
            </div>
          </div>

          <div class="max-h-[calc(100vh-15rem)] overflow-y-auto">
            <div v-if="loadingPeers" class="p-4 text-sm text-base-content/60">{{ t('botConsole.loadingPeers') }}</div>
            <div v-else-if="filteredPeers.length === 0" class="p-4 text-sm text-base-content/60">{{ t('botConsole.emptyPeers') }}</div>
            <button
              v-for="peer in filteredPeers"
              :key="peerKey(peer)"
              class="w-full border-b border-base-200 px-4 py-3 text-left transition-colors hover:bg-base-200/60"
              :class="selectedPeerKey === peerKey(peer) ? 'bg-primary/10' : ''"
              @click="selectPeer(peer)"
            >
              <div class="flex items-start justify-between gap-3">
                <div class="min-w-0">
                  <div class="flex items-center gap-2">
                    <span class="truncate font-medium">
                      {{ peer.display_name || peer.peer_id }}
                    </span>
                    <span class="badge badge-outline badge-xs">
                      {{ peerTypeLabel(peer.peer_type) }}
                    </span>
                  </div>
                  <div class="mt-1 truncate text-xs text-base-content/60">
                    {{ peer.account_id }} / {{ peer.peer_id }}
                  </div>
                  <div class="mt-2 flex flex-wrap items-center gap-2 text-xs">
                    <span class="badge badge-ghost badge-xs">{{ t('botConsole.peerBadges.messages', { count: peer.message_count }) }}</span>
                    <span v-if="peer.running_execution_count > 0" class="badge badge-primary badge-xs">
                      {{ t('botConsole.peerBadges.runningExecutions', { count: peer.running_execution_count }) }}
                    </span>
                    <span v-if="peer.failed_execution_count > 0" class="badge badge-error badge-xs">
                      {{ t('botConsole.peerBadges.failedExecutions', { count: peer.failed_execution_count }) }}
                    </span>
                  </div>
                </div>
                <div class="text-right text-xs text-base-content/60">
                  <div v-if="peer.latest_execution_status">
                    <span class="badge badge-xs" :class="statusBadgeClass(peer.latest_execution_status)">
                      {{ statusLabel(peer.latest_execution_status) }}
                    </span>
                  </div>
                  <div>{{ formatTimestamp(peer.last_message_at) }}</div>
                </div>
              </div>
            </button>
          </div>
        </section>

        <section class="rounded-lg border border-base-300 bg-base-100 min-h-[42rem] overflow-hidden">
          <div class="border-b border-base-300 px-4 py-3">
            <div class="flex flex-col gap-3 lg:flex-row lg:items-center lg:justify-between">
              <div v-if="selectedPeer" class="min-w-0">
                <div class="flex flex-wrap items-center gap-2">
                  <h3 class="truncate text-lg font-semibold">
                    {{ selectedPeer.display_name || selectedPeer.peer_id }}
                  </h3>
                  <span class="badge badge-outline badge-sm">
                    {{ transportLabel(selectedPeer.transport) }}
                  </span>
                  <span class="badge badge-ghost badge-sm">
                    {{ peerTypeLabel(selectedPeer.peer_type) }}
                  </span>
                </div>
                <div class="mt-1 text-xs text-base-content/60">
                  {{ selectedPeer.account_id }} / {{ selectedPeer.peer_id }}
                </div>
              </div>
              <div v-else class="text-sm text-base-content/60">
                {{ t('botConsole.selectPeer') }}
              </div>

              <div class="flex flex-wrap items-center gap-2">
                <button
                  v-if="selectedConversationId"
                  class="btn btn-sm btn-ghost"
                  @click="openConversationInAssistant"
                >
                  {{ t('botConsole.openAiConversation') }}
                </button>
                <button
                  class="btn btn-sm btn-ghost"
                  :disabled="!selectedPeer || loadingDetails"
                  @click="refreshSelectedPeer"
                >
                  {{ t('botConsole.refreshCurrentPeer') }}
                </button>
              </div>
            </div>
          </div>

          <div v-if="!selectedPeer" class="p-6 text-sm text-base-content/60">
            {{ t('botConsole.selectPeer') }}
          </div>

          <template v-else>
            <div class="border-b border-base-300 px-4 py-3">
              <div class="grid grid-cols-2 gap-3 xl:grid-cols-3">
                <div class="rounded-lg bg-base-200/60 px-4 py-3">
                  <div class="text-xs text-base-content/60">{{ t('botConsole.peerSummary.messages') }}</div>
                  <div class="mt-1 text-lg font-semibold">{{ selectedPeer.message_count }}</div>
                </div>
                <div class="rounded-lg bg-base-200/60 px-4 py-3">
                  <div class="text-xs text-base-content/60">{{ t('botConsole.peerSummary.executions') }}</div>
                  <div class="mt-1 text-lg font-semibold">{{ selectedPeer.execution_run_count }}</div>
                </div>
                <div class="rounded-lg bg-base-200/60 px-4 py-3">
                  <div class="text-xs text-base-content/60">{{ t('botConsole.peerSummary.failedExecutions') }}</div>
                  <div class="mt-1 text-lg font-semibold">{{ selectedPeer.failed_execution_count }}</div>
                </div>
              </div>
              <div class="mt-3 flex flex-wrap items-center gap-2 text-xs text-base-content/70">
                <span v-if="selectedPeer.running_execution_count > 0" class="badge badge-primary badge-sm">
                  {{ t('botConsole.peerBadges.runningExecutions', { count: selectedPeer.running_execution_count }) }}
                </span>
                <span v-if="selectedPeer.latest_execution_status">
                  {{ t('botConsole.peerSummary.latestExecution') }}:
                  {{ statusLabel(selectedPeer.latest_execution_status) }}
                  <span v-if="selectedPeer.latest_execution_started_at">
                    / {{ formatTimestamp(selectedPeer.latest_execution_started_at) }}
                  </span>
                </span>
              </div>
            </div>

            <div class="border-b border-base-300 px-4 py-3">
              <div class="tabs tabs-boxed bg-base-200/60 inline-flex">
                <button
                  class="tab"
                  :class="{ 'tab-active': activeTab === 'messages' }"
                  @click="setActiveTab('messages')"
                >
                  {{ t('botConsole.tabs.messages') }}
                </button>
                <button
                  class="tab"
                  :class="{ 'tab-active': activeTab === 'executions' }"
                  @click="setActiveTab('executions')"
                >
                  {{ t('botConsole.tabs.executions') }}
                </button>
                <button
                  class="tab"
                  :class="{ 'tab-active': activeTab === 'turnLogs' }"
                  @click="setActiveTab('turnLogs')"
                >
                  {{ t('botConsole.tabs.turnLogs') }}
                </button>
                <button
                  class="tab"
                  :class="{ 'tab-active': activeTab === 'missions' }"
                  @click="setActiveTab('missions')"
                >
                  {{ t('botConsole.tabs.missions') }}
                </button>
                <button
                  class="tab"
                  :class="{ 'tab-active': activeTab === 'observer' }"
                  @click="setActiveTab('observer')"
                >
                  {{ t('botConsole.tabs.observer') }}
                </button>
              </div>
            </div>

            <div v-if="loadingDetails" class="p-6 text-sm text-base-content/60">{{ t('botConsole.messages.loading') }}</div>

            <div v-else-if="activeTab === 'messages'" class="max-h-[calc(100vh-18rem)] overflow-y-auto p-4">
              <div v-if="messageTimeline.length === 0" class="text-sm text-base-content/60">{{ t('botConsole.messages.empty') }}</div>
              <div v-else class="space-y-3">
                <article
                  v-for="message in messageTimeline"
                  :key="message.id"
                  class="rounded-lg border border-base-300 p-3"
                >
                  <div class="mb-2 flex flex-wrap items-center justify-between gap-2 text-xs text-base-content/60">
                    <div class="flex items-center gap-2">
                      <span class="badge badge-sm" :class="message.direction === 'inbound' ? 'badge-primary' : 'badge-success'">
                        {{ directionLabel(message.direction) }}
                      </span>
                      <span>{{ message.sender_id }}</span>
                      <span v-if="message.linked_execution_run_id" class="font-mono">
                        {{ shortId(message.linked_execution_run_id) }}
                      </span>
                    </div>
                    <div>{{ formatTimestamp(message.created_at) }}</div>
                  </div>

                  <pre class="whitespace-pre-wrap break-words font-sans text-sm leading-6">{{ message.content }}</pre>

                  <div class="mt-3 flex flex-wrap items-center gap-3 text-xs text-base-content/60">
                    <span v-if="message.transport_message_id">{{ t('botConsole.messages.messageId') }}: {{ message.transport_message_id }}</span>
                    <span v-if="message.context_token">{{ t('botConsole.messages.context') }}: {{ message.context_token }}</span>
                    <button
                      v-if="message.linked_execution_run_id"
                      class="link link-hover"
                      @click="focusExecutionRun(message.linked_execution_run_id)"
                    >
                      {{ t('botConsole.messages.viewExecution') }}
                    </button>
                  </div>

                  <details v-if="message.metadata_json" class="mt-3">
                    <summary class="cursor-pointer text-xs text-base-content/60">{{ t('botConsole.messages.metadata') }}</summary>
                    <pre class="mt-2 overflow-x-auto rounded bg-base-200 p-3 text-xs">{{ prettyJson(parseJson(message.metadata_json)) }}</pre>
                  </details>
                </article>
              </div>
            </div>

            <div v-else-if="activeTab === 'executions'" class="grid min-h-0 grid-cols-1 gap-4 p-4 xl:grid-cols-[20rem_minmax(0,1fr)]">
              <div class="rounded-lg border border-base-300 overflow-hidden">
                <div class="border-b border-base-300 px-3 py-2 text-sm font-semibold">
                  <div class="flex items-center justify-between gap-3">
                    <span>{{ t('botConsole.executions.listTitle') }}</span>
                    <select v-model="executionFilter" class="select select-bordered select-xs min-w-32" @change="handleExecutionFilterChange">
                      <option value="all">{{ t('botConsole.executionFilters.all') }}</option>
                      <option value="failures">{{ t('botConsole.executionFilters.failures') }}</option>
                      <option value="running">{{ t('botConsole.executionFilters.running') }}</option>
                      <option value="message">{{ t('botConsole.executionFilters.message') }}</option>
                    </select>
                  </div>
                </div>
                <div class="max-h-[calc(100vh-23rem)] overflow-y-auto">
                  <div v-if="visibleExecutionRuns.length === 0" class="p-4 text-sm text-base-content/60">{{ t('botConsole.executions.empty') }}</div>
                  <button
                    v-for="run in visibleExecutionRuns"
                    :key="run.id"
                    class="w-full border-b border-base-200 px-3 py-3 text-left hover:bg-base-200/60"
                    :class="selectedExecutionRunId === run.id ? 'bg-primary/10' : ''"
                    @click="selectExecutionRun(run.id)"
                  >
                    <div class="flex items-start justify-between gap-3">
                      <div class="min-w-0">
                        <div class="truncate text-sm font-medium">{{ run.task_text }}</div>
                        <div class="mt-1 flex flex-wrap items-center gap-2 text-xs text-base-content/60">
                          <span class="badge badge-outline badge-xs">{{ triggerKindLabel(run.trigger_kind) }}</span>
                          <span class="badge badge-ghost badge-xs">{{ executionMemoryScopeLabel() }}</span>
                          <span>{{ shortId(run.id) }}</span>
                        </div>
                      </div>
                      <span class="badge badge-sm" :class="statusBadgeClass(run.status)">
                        {{ statusLabel(run.status) }}
                      </span>
                    </div>
                    <div class="mt-2 text-xs text-base-content/60">
                      {{ formatTimestamp(run.started_at) }}
                    </div>
                  </button>
                </div>
              </div>

              <div class="rounded-lg border border-base-300 min-h-[32rem] overflow-hidden">
                <div v-if="selectedExecutionRun" class="flex h-full flex-col">
                  <div class="border-b border-base-300 px-4 py-3">
                    <div class="flex flex-wrap items-center justify-between gap-3">
                      <div>
                        <div class="text-base font-semibold">{{ selectedExecutionRun.task_text }}</div>
                        <div class="mt-1 flex flex-wrap items-center gap-2 text-xs text-base-content/60">
                          <span class="font-mono">{{ selectedExecutionRun.id }}</span>
                          <span>{{ formatTimestamp(selectedExecutionRun.started_at) }}</span>
                          <span v-if="selectedExecutionRun.completed_at">
                            {{ t('botConsole.labels.completedAt', { time: formatTimestamp(selectedExecutionRun.completed_at) }) }}
                          </span>
                        </div>
                      </div>
                      <span class="badge badge-sm" :class="statusBadgeClass(selectedExecutionRun.status)">
                        {{ statusLabel(selectedExecutionRun.status) }}
                      </span>
                    </div>
                  </div>

                  <div v-if="loadingExecutionDetail" class="p-4 text-sm text-base-content/60">{{ t('botConsole.executions.loadingDetail') }}</div>

                  <div v-else class="flex-1 overflow-y-auto p-4 space-y-4">
                    <section class="rounded-lg border border-base-300 p-4">
                      <div class="text-sm font-semibold">{{ t('botConsole.executions.summary') }}</div>
                      <div class="mt-3 grid grid-cols-1 gap-3 text-sm lg:grid-cols-2">
                        <div>
                          <div class="text-xs text-base-content/60">{{ t('botConsole.executions.triggerKind') }}</div>
                          <div>{{ triggerKindLabel(selectedExecutionRun.trigger_kind) }}</div>
                        </div>
                        <div>
                          <div class="text-xs text-base-content/60">{{ t('botConsole.executions.memoryScope') }}</div>
                          <div>{{ executionMemoryScopeLabel() }}</div>
                        </div>
                        <div>
                          <div class="text-xs text-base-content/60">{{ t('botConsole.executions.assistantProfile') }}</div>
                          <div>{{ selectedExecutionRun.assistant_profile_id || t('botConsole.executions.defaultAssistantProfile') }}</div>
                        </div>
                        <div>
                          <div class="text-xs text-base-content/60">{{ t('botConsole.executions.conversationId') }}</div>
                          <div class="font-mono break-all">{{ selectedExecutionRun.conversation_id }}</div>
                        </div>
                        <div>
                          <div class="text-xs text-base-content/60">{{ t('botConsole.executions.executionId') }}</div>
                          <div class="font-mono break-all">{{ selectedExecutionRun.ai_execution_id }}</div>
                        </div>
                      </div>
                      <div v-if="selectedExecutionRun.result_text" class="mt-4">
                        <div class="text-xs text-base-content/60 mb-1">{{ t('botConsole.executions.result') }}</div>
                        <pre class="whitespace-pre-wrap break-words rounded bg-base-200 p-3 text-sm">{{ selectedExecutionRun.result_text }}</pre>
                      </div>
                      <div v-if="selectedExecutionRun.error_message" class="mt-4">
                        <div class="text-xs text-error mb-1">{{ t('botConsole.executions.error') }}</div>
                        <pre class="whitespace-pre-wrap break-words rounded bg-error/10 p-3 text-sm text-error">{{ selectedExecutionRun.error_message }}</pre>
                      </div>
                    </section>

                    <section class="rounded-lg border border-base-300 p-4">
                      <div class="mb-3 text-sm font-semibold">{{ t('botConsole.executions.taskLedger') }}</div>
                      <div v-if="executionTasks.length === 0" class="text-sm text-base-content/60">{{ t('botConsole.executions.emptyTasks') }}</div>
                      <div v-else class="space-y-2">
                        <div
                          v-for="task in orderedExecutionTasks"
                          :key="task.id"
                          class="rounded border border-base-300 p-3"
                        >
                          <div class="flex items-start justify-between gap-3">
                            <div>
                              <div class="text-sm font-medium">{{ task.item_index + 1 }}. {{ task.content }}</div>
                              <div v-if="task.result" class="mt-1 text-xs text-base-content/70 whitespace-pre-wrap break-words">
                                {{ task.result }}
                              </div>
                            </div>
                            <span class="badge badge-sm" :class="statusBadgeClass(task.status)">
                              {{ statusLabel(task.status) }}
                            </span>
                          </div>
                        </div>
                      </div>
                    </section>

                    <section class="rounded-lg border border-base-300 p-4">
                      <div class="mb-3 text-sm font-semibold">{{ t('botConsole.executions.harnessEvents') }}</div>
                      <div v-if="harnessEvents.length === 0" class="text-sm text-base-content/60">{{ t('botConsole.executions.emptyHarnessEvents') }}</div>
                      <div v-else class="space-y-2">
                        <details
                          v-for="event in harnessEvents"
                          :key="event.id"
                          class="rounded border border-base-300 p-3"
                        >
                          <summary class="cursor-pointer list-none">
                            <div class="flex items-center justify-between gap-3">
                              <div class="text-sm font-medium">{{ event.event_type }}</div>
                              <div class="text-xs text-base-content/60">{{ formatTimestamp(event.created_at) }}</div>
                            </div>
                          </summary>
                          <pre v-if="event.payload !== null" class="mt-3 overflow-x-auto rounded bg-base-200 p-3 text-xs">{{ prettyJson(event.payload) }}</pre>
                        </details>
                      </div>
                    </section>

                    <section class="rounded-lg border border-base-300 p-4">
                      <div class="mb-3 text-sm font-semibold">{{ t('botConsole.executions.harnessCheckpoints') }}</div>
                      <div v-if="harnessCheckpoints.length === 0" class="text-sm text-base-content/60">{{ t('botConsole.executions.emptyHarnessCheckpoints') }}</div>
                      <div v-else class="space-y-2">
                        <details
                          v-for="checkpoint in harnessCheckpoints"
                          :key="checkpoint.id"
                          class="rounded border border-base-300 p-3"
                        >
                          <summary class="cursor-pointer list-none">
                            <div class="flex items-center justify-between gap-3">
                              <div class="text-sm font-medium">{{ checkpoint.checkpoint_type }}</div>
                              <div class="text-xs text-base-content/60">{{ formatTimestamp(checkpoint.created_at) }}</div>
                            </div>
                          </summary>
                          <pre v-if="checkpoint.payload !== null" class="mt-3 overflow-x-auto rounded bg-base-200 p-3 text-xs">{{ prettyJson(checkpoint.payload) }}</pre>
                        </details>
                      </div>
                    </section>
                  </div>
                </div>

                <div v-else class="p-4 text-sm text-base-content/60">
                  {{ t('botConsole.executions.emptyDetail') }}
                </div>
              </div>
            </div>

            <div v-else-if="activeTab === 'turnLogs'" class="p-4 space-y-4">
              <div class="flex flex-col gap-3 rounded-lg border border-base-300 p-4 lg:flex-row lg:items-end">
                <label class="form-control w-full lg:max-w-48">
                  <span class="label-text text-xs">{{ t('botConsole.turnLogs.date') }}</span>
                  <input v-model="turnLogDate" type="date" class="input input-sm input-bordered" />
                </label>
                <label class="form-control w-full lg:max-w-32">
                  <span class="label-text text-xs">{{ t('botConsole.turnLogs.limit') }}</span>
                  <input v-model.number="turnLogLimit" type="number" min="1" max="500" class="input input-sm input-bordered" />
                </label>
                <div class="flex items-center gap-2">
                  <button class="btn btn-sm btn-primary" :disabled="loadingTurnLogs" @click="loadTurnLogs">
                    <span v-if="loadingTurnLogs" class="loading loading-spinner loading-xs"></span>
                    {{ t('botConsole.refresh') }}
                  </button>
                </div>
              </div>

              <div v-if="!selectedConversationId" class="rounded-lg border border-base-300 p-4 text-sm text-base-content/60">
                {{ t('botConsole.turnLogs.emptyWithoutConversation') }}
              </div>
              <div v-else-if="turnLogError" class="alert alert-error">
                <span>{{ turnLogError }}</span>
              </div>
              <div v-else-if="loadingTurnLogs" class="rounded-lg border border-base-300 p-6 text-sm text-base-content/60">
                {{ t('botConsole.turnLogs.loading') }}
              </div>
              <div v-else-if="turnLogs.length === 0" class="rounded-lg border border-base-300 p-6 text-sm text-base-content/60">
                {{ t('botConsole.turnLogs.empty') }}
              </div>
              <div v-else class="space-y-3">
                <details
                  v-for="entry in turnLogs"
                  :key="`${entry.session_id}-${entry.timestamp}`"
                  class="rounded-lg border border-base-300 bg-base-100 p-4"
                  @toggle="handleTurnLogToggle(entry, $event)"
                >
                  <summary class="cursor-pointer list-none">
                    <div class="flex flex-col gap-3 lg:flex-row lg:items-start lg:justify-between">
                      <div class="min-w-0">
                        <div class="flex flex-wrap items-center gap-2">
                          <span class="badge badge-outline">#{{ entry.turn ?? '-' }}</span>
                          <span class="badge badge-sm" :class="statusBadgeClass(entry.status)">
                            {{ statusLabel(entry.status) }}
                          </span>
                          <span class="text-sm font-medium">{{ entry.provider }} / {{ entry.model }}</span>
                          <span class="text-xs text-base-content/60">{{ formatTimestamp(entry.timestamp) }}</span>
                        </div>
                        <div class="mt-2 text-xs text-base-content/60 font-mono break-all">
                          {{ t('botConsole.turnLogs.session') }}: {{ entry.session_id }}
                        </div>
                      </div>
                      <button
                        class="btn btn-xs btn-outline"
                        @click.stop="openTurnLogConversation(entry.conversation_id)"
                      >
                        {{ t('botConsole.openAiConversation') }}
                      </button>
                    </div>
                  </summary>

                  <div class="mt-4 space-y-4">
                    <div class="grid grid-cols-1 gap-3 lg:grid-cols-3">
                      <div class="rounded-lg bg-base-200/60 px-4 py-3">
                        <div class="text-xs text-base-content/60">{{ t('botConsole.turnLogs.duration') }}</div>
                        <div class="mt-1 text-sm font-medium">{{ formatDuration(entry.duration_ms) }}</div>
                      </div>
                      <div class="rounded-lg bg-base-200/60 px-4 py-3">
                        <div class="text-xs text-base-content/60">{{ t('botConsole.turnLogs.tokens') }}</div>
                        <div class="mt-1 text-sm font-medium">{{ formatTokens(entry.input_tokens, entry.output_tokens) }}</div>
                      </div>
                      <div class="rounded-lg bg-base-200/60 px-4 py-3">
                        <div class="text-xs text-base-content/60">{{ t('botConsole.turnLogs.toolCalls') }}</div>
                        <div class="mt-1 text-sm font-medium">{{ entry.tool_call_count }}</div>
                      </div>
                    </div>

                    <div class="grid grid-cols-1 gap-4 xl:grid-cols-2">
                      <section>
                        <div class="mb-2 text-sm font-semibold">{{ t('botConsole.turnLogs.userPromptPreview') }}</div>
                        <pre class="whitespace-pre-wrap break-words rounded bg-base-200 p-3 text-sm">{{ entry.user_request_preview || '—' }}</pre>
                      </section>
                      <section>
                        <div class="mb-2 text-sm font-semibold">{{ t('botConsole.turnLogs.assistantResponsePreview') }}</div>
                        <pre class="whitespace-pre-wrap break-words rounded bg-base-200 p-3 text-sm">{{ entry.assistant_response_preview || '—' }}</pre>
                      </section>
                    </div>

                    <div v-if="isTurnLogDetailLoading(entry.session_id)" class="text-sm text-base-content/60">
                      {{ t('botConsole.turnLogs.loadingDetail') }}
                    </div>
                    <div v-else-if="getTurnLogDetailError(entry.session_id)" class="alert alert-error">
                      <span>{{ getTurnLogDetailError(entry.session_id) }}</span>
                    </div>
                    <template v-else-if="getTurnLogDetail(entry.session_id)">
                      <section v-if="getTurnLogDetail(entry.session_id)?.summary?.reasoning">
                        <div class="mb-2 text-sm font-semibold">{{ t('botConsole.turnLogs.reasoning') }}</div>
                        <pre class="whitespace-pre-wrap break-words rounded bg-base-200 p-3 text-sm">{{ stringify(getTurnLogDetail(entry.session_id)?.summary?.reasoning) }}</pre>
                      </section>
                      <div class="grid grid-cols-1 gap-4 xl:grid-cols-2">
                        <section>
                          <div class="mb-2 text-sm font-semibold">{{ t('botConsole.turnLogs.userPrompt') }}</div>
                          <pre class="whitespace-pre-wrap break-words rounded bg-base-200 p-3 text-sm">{{ stringify(getTurnLogDetail(entry.session_id)?.summary?.user_request) }}</pre>
                        </section>
                        <section>
                          <div class="mb-2 text-sm font-semibold">{{ t('botConsole.turnLogs.assistantResponse') }}</div>
                          <pre class="whitespace-pre-wrap break-words rounded bg-base-200 p-3 text-sm">{{ stringify(getTurnLogDetail(entry.session_id)?.summary?.assistant_response) }}</pre>
                        </section>
                      </div>
                      <section
                        v-if="Array.isArray(getTurnLogDetail(entry.session_id)?.summary?.tool_calls) && getTurnLogDetail(entry.session_id)?.summary?.tool_calls.length > 0"
                      >
                        <div class="mb-2 text-sm font-semibold">{{ t('botConsole.turnLogs.toolCallDetail') }}</div>
                        <div class="space-y-3">
                          <article
                            v-for="(toolCall, index) in getTurnLogDetail(entry.session_id)?.summary?.tool_calls"
                            :key="`${entry.session_id}-tool-${index}`"
                            class="rounded-lg border border-base-300 bg-base-200/40 p-3"
                          >
                            <div class="flex flex-wrap items-center gap-2">
                              <span class="badge badge-outline">{{ toolCall.tool_name || `tool-${index + 1}` }}</span>
                              <span class="badge badge-sm" :class="toolCall.success === false ? 'badge-error' : 'badge-success'">
                                {{ toolCall.success === false ? statusLabel('failed') : statusLabel('completed') }}
                              </span>
                              <span class="text-xs text-base-content/60">{{ toolCall.tool_call_id }}</span>
                            </div>
                            <div class="mt-3 grid grid-cols-1 gap-3 xl:grid-cols-2">
                              <div>
                                <div class="mb-1 text-xs uppercase tracking-wide text-base-content/60">{{ t('botConsole.turnLogs.arguments') }}</div>
                                <pre class="whitespace-pre-wrap break-words rounded bg-base-100 p-3 text-xs">{{ prettyJson(toolCall.arguments ?? toolCall.arguments_raw) }}</pre>
                              </div>
                              <div>
                                <div class="mb-1 text-xs uppercase tracking-wide text-base-content/60">{{ t('botConsole.turnLogs.result') }}</div>
                                <pre class="whitespace-pre-wrap break-words rounded bg-base-100 p-3 text-xs">{{ prettyJson(toolCall.result ?? toolCall.result_raw) }}</pre>
                              </div>
                            </div>
                          </article>
                        </div>
                      </section>
                    </template>
                  </div>
                </details>
              </div>
            </div>

            <div v-else-if="activeTab === 'missions'" class="flex-1 overflow-hidden">
              <MissionsPanel
                :owner-kind="selectedPeer ? 'bot_peer' : undefined"
                :owner-ref="selectedPeer ? `${selectedPeer.transport}:${selectedPeer.account_id}:${selectedPeer.peer_type}:${selectedPeer.peer_id}` : undefined"
              />
            </div>

            <div v-else-if="activeTab === 'observer'" class="flex-1 overflow-hidden p-4">
              <ObserverPanel />
            </div>
          </template>
        </section>
      </div>

      <AppDialog ref="accountsDialogRef" class="modal" @click.self="closeAccountsDialog">
        <div class="modal-box w-11/12 max-w-7xl max-h-[90vh] overflow-hidden p-0">
          <div class="flex items-center justify-between border-b border-base-300 px-5 py-4">
            <div>
              <div class="text-base font-semibold">{{ t('botConsole.accounts.dialogTitle') }}</div>
              <div class="mt-1 text-xs text-base-content/70">{{ t('botConsole.accounts.dialogDescription') }}</div>
            </div>
            <form method="dialog">
              <button class="btn btn-sm btn-ghost" @click="closeAccountsDialog">
                {{ t('botConsole.accounts.close') }}
              </button>
            </form>
          </div>

          <div class="max-h-[calc(90vh-5rem)] overflow-y-auto p-5">
            <BotAccountsPanel
              :accounts="accounts"
              :selected-transport="selectedTransport"
              :selected-account-id="selectedAccountId"
              :loading="loadingPeers"
              @account-change="handleBotAccountChange"
              @updated="handleBotAccountUpdated"
            />
          </div>
        </div>
        <form method="dialog" class="modal-backdrop">
          <button @click="closeAccountsDialog">{{ t('botConsole.accounts.close') }}</button>
        </form>
      </AppDialog>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRoute, useRouter } from 'vue-router'
import AppDialog from '@/components/AppDialog.vue'
import BotAccountsPanel from '@/components/Bot/BotAccountsPanel.vue'
import MissionsPanel from '@/components/Bot/MissionsPanel.vue'
import ObserverPanel from '@/components/Bot/ObserverPanel.vue'
import { SUPPORTED_BOT_TRANSPORTS } from '@/components/Bot/botTransportCatalog'
import {
  getAiTurnLogDetail,
  getAiTurnLogs,
  type AiTurnLogEntry,
  type AiTurnLogSummaryEntry,
} from '@/api/aiLogs'
import {
  getBotExecutionTasks,
  getBotHarnessCheckpoints,
  getBotHarnessEvents,
  listBotAccounts,
  listBotExecutionRunsForPeer,
  listBotMessagesForPeer,
  listBotPeers,
  type AgentHarnessCheckpoint,
  type AgentHarnessEvent,
  type AgentTaskHistoryItem,
  type BotAccount,
  type BotExecutionRun,
  type BotMessage,
  type BotPeer,
} from '@/api/botConsole'
import { useFeatureEntitlementsState } from '@/services/featureEntitlements'

defineOptions({
  name: 'BotConsole',
})

const { t } = useI18n()
const route = useRoute()
const router = useRouter()
const entitlements = useFeatureEntitlementsState()

const selectedTransport = ref(typeof route.query.transport === 'string' ? route.query.transport : 'weixin')
const selectedAccountId = ref(typeof route.query.accountId === 'string' ? route.query.accountId : '')
const selectedPeerType = ref(typeof route.query.peerType === 'string' ? route.query.peerType : '')
const peerStateFilter = ref(typeof route.query.peerState === 'string' ? route.query.peerState : 'all')
const executionFilter = ref(typeof route.query.executionFilter === 'string' ? route.query.executionFilter : 'all')
const selectedPeerKey = ref('')
const selectedExecutionRunId = ref('')
const activeTab = ref<'messages' | 'executions' | 'turnLogs' | 'missions' | 'observer'>(
  route.query.tab === 'executions' || route.query.tab === 'turnLogs' || route.query.tab === 'missions' || route.query.tab === 'observer'
    ? route.query.tab
    : 'messages',
)
const isBotConsoleActivated = computed(() => entitlements.value.can_access_bot_console)

const accounts = ref<BotAccount[]>([])
const peers = ref<BotPeer[]>([])
const messages = ref<BotMessage[]>([])
const executionRuns = ref<BotExecutionRun[]>([])
const executionTasks = ref<AgentTaskHistoryItem[]>([])
const harnessEvents = ref<AgentHarnessEvent[]>([])
const harnessCheckpoints = ref<AgentHarnessCheckpoint[]>([])
const turnLogs = ref<AiTurnLogSummaryEntry[]>([])
const turnLogDetails = ref<Record<string, AiTurnLogEntry | null>>({})
const turnLogDetailLoading = ref<Record<string, boolean>>({})
const turnLogDetailErrors = ref<Record<string, string>>({})

const loadingPeers = ref(false)
const loadingDetails = ref(false)
const loadingExecutionDetail = ref(false)
const loadingTurnLogs = ref(false)
const loadError = ref('')
const turnLogError = ref('')
const turnLogDate = ref(new Date().toISOString().slice(0, 10))
const turnLogLimit = ref(50)
const accountsDialogRef = ref<{ showModal: () => void; close: () => void } | null>(null)

function isBotConsoleRouteActive(): boolean {
  return route.name === 'BotConsole' || route.path === '/bot-console'
}

function peerKey(peer: Pick<BotPeer, 'transport' | 'account_id' | 'peer_type' | 'peer_id'>): string {
  return [peer.transport, peer.account_id, peer.peer_type, peer.peer_id].join('::')
}

function shortId(value?: string | null): string {
  const normalized = String(value || '').trim()
  return normalized ? normalized.slice(0, 8) : '—'
}

function formatTimestamp(value?: string | null): string {
  const normalized = String(value || '').trim()
  if (!normalized) return '—'
  const date = new Date(normalized)
  if (Number.isNaN(date.getTime())) return normalized
  return date.toLocaleString()
}

function parseJson(value?: string | null): unknown {
  if (!value) return null
  try {
    return JSON.parse(value)
  } catch {
    return value
  }
}

function prettyJson(value: unknown): string {
  if (value === null || value === undefined) return ''
  if (typeof value === 'string') return value
  try {
    return JSON.stringify(value, null, 2)
  } catch {
    return String(value)
  }
}

function statusBadgeClass(status: string): string {
  switch ((status || '').trim().toLowerCase()) {
    case 'completed':
    case 'succeeded':
      return 'badge-success'
    case 'running':
    case 'in_progress':
      return 'badge-primary'
    case 'failed':
    case 'error':
      return 'badge-error'
    case 'cancelled':
      return 'badge-warning'
    default:
      return 'badge-ghost'
  }
}

function normalizedStatus(status?: string | null): string {
  return String(status || '').trim().toLowerCase()
}

function isFailedStatus(status?: string | null): boolean {
  return ['failed', 'error'].includes(normalizedStatus(status))
}

function isRunningStatus(status?: string | null): boolean {
  return ['running', 'in_progress'].includes(normalizedStatus(status))
}

function statusLabel(status?: string | null): string {
  switch (normalizedStatus(status)) {
    case 'completed':
      return t('botConsole.status.completed')
    case 'succeeded':
      return t('botConsole.status.succeeded')
    case 'running':
      return t('botConsole.status.running')
    case 'in_progress':
      return t('botConsole.status.inProgress')
    case 'failed':
      return t('botConsole.status.failed')
    case 'error':
      return t('botConsole.status.error')
    case 'cancelled':
      return t('botConsole.status.cancelled')
    case 'interrupted':
      return t('botConsole.status.interrupted')
    case 'empty_response':
      return t('botConsole.status.emptyResponse')
    default:
      return status || t('botConsole.status.unknown')
  }
}

function peerTypeLabel(peerType?: string | null): string {
  return String(peerType || '').trim().toLowerCase() === 'group'
    ? t('botConsole.group')
    : t('botConsole.dm')
}

function transportLabel(transport?: string | null): string {
  const normalized = String(transport || '').trim().toLowerCase()
  if (!normalized) return '—'
  if (normalized === 'weixin') return t('botConsole.accounts.transports.weixin')
  if (normalized === 'feishu') return t('botConsole.accounts.transports.feishu')
  if (normalized === 'discord') return t('botConsole.accounts.transports.discord')
  return transport || '—'
}

function directionLabel(direction?: string | null): string {
  return String(direction || '').trim().toLowerCase() === 'inbound'
    ? t('botConsole.messages.inbound')
    : t('botConsole.messages.outbound')
}

function triggerKindLabel(triggerKind?: string | null): string {
  switch ((triggerKind || '').trim().toLowerCase()) {
    case 'message':
      return t('botConsole.labels.message')
    default:
      return triggerKind || t('botConsole.status.unknown')
  }
}

function executionMemoryScopeLabel(): string {
  return t('botConsole.labels.chatMemory')
}

function formatDuration(value?: number | null): string {
  const ms = Number(value || 0)
  if (!Number.isFinite(ms) || ms <= 0) return '—'
  if (ms < 1000) return `${ms} ms`
  return `${(ms / 1000).toFixed(2)} s`
}

function formatTokens(input?: number | null, output?: number | null): string {
  return `${Number(input || 0)}/${Number(output || 0)}`
}

function stringify(value: unknown): string {
  if (typeof value === 'string') return value
  if (value == null) return ''
  return JSON.stringify(value, null, 2)
}

const transportOptions = computed(() => {
  const values = new Set<string>(SUPPORTED_BOT_TRANSPORTS)
  for (const account of accounts.value) {
    const transport = String(account.transport || '').trim()
    if (transport) values.add(transport)
  }
  if (selectedTransport.value) values.add(selectedTransport.value)
  return Array.from(values).sort((a, b) => a.localeCompare(b))
})

const visibleAccounts = computed(() => {
  if (!selectedTransport.value) return accounts.value
  return accounts.value.filter((account) => account.transport === selectedTransport.value)
})

const visibleAccountCount = computed(() => visibleAccounts.value.length)

const runningAccountCount = computed(() =>
  visibleAccounts.value.filter((account) => normalizedStatus(account.status) === 'running').length,
)

const failedAccountCount = computed(() =>
  visibleAccounts.value.filter((account) => isFailedStatus(account.status)).length,
)

const latestVisibleAccountAt = computed(() => {
  const timestamps = visibleAccounts.value
    .map((account) => String(account.last_seen_at || '').trim())
    .filter((value) => value.length > 0)
  if (timestamps.length === 0) return ''
  return timestamps.reduce((latest, current) => {
    if (!latest) return current
    const latestTime = new Date(latest).getTime()
    const currentTime = new Date(current).getTime()
    if (Number.isNaN(currentTime)) return latest
    if (Number.isNaN(latestTime) || currentTime > latestTime) return current
    return latest
  }, '')
})

const selectedBotAccount = computed(() =>
  accounts.value.find((account) => account.account_id === selectedAccountId.value) ?? null,
)

const filteredPeers = computed(() => {
  switch (peerStateFilter.value) {
    case 'failures':
      return peers.value.filter((peer) => peer.failed_execution_count > 0)
    case 'running':
      return peers.value.filter((peer) => peer.running_execution_count > 0)
    default:
      return peers.value
  }
})

const selectedPeer = computed(() =>
  peers.value.find((peer) => peerKey(peer) === selectedPeerKey.value) ?? null,
)

const selectedExecutionRun = computed(() =>
  executionRuns.value.find((run) => run.id === selectedExecutionRunId.value) ?? null,
)

const selectedConversationId = computed(() => {
  if (!selectedPeer.value) return null
  return [
    selectedPeer.value.transport,
    selectedPeer.value.account_id,
    selectedPeer.value.peer_type,
    selectedPeer.value.peer_id,
  ].join(':')
})

const messageTimeline = computed(() => [...messages.value].reverse())
const visibleExecutionRuns = computed(() => {
  switch (executionFilter.value) {
    case 'failures':
      return executionRuns.value.filter((run) => isFailedStatus(run.status))
    case 'running':
      return executionRuns.value.filter((run) => isRunningStatus(run.status))
    case 'message':
      return executionRuns.value.filter((run) => normalizedStatus(run.trigger_kind) === 'message')
    default:
      return executionRuns.value
  }
})
const orderedExecutionTasks = computed(() =>
  [...executionTasks.value].sort((a, b) => a.item_index - b.item_index || a.created_at_ms - b.created_at_ms),
)

async function syncRoute() {
  if (!isBotConsoleRouteActive()) return

  const query: Record<string, string> = {}
  if (selectedTransport.value) query.transport = selectedTransport.value
  if (selectedAccountId.value) query.accountId = selectedAccountId.value
  if (selectedPeerType.value) query.peerType = selectedPeerType.value
  if (peerStateFilter.value !== 'all') query.peerState = peerStateFilter.value
  if (executionFilter.value !== 'all') query.executionFilter = executionFilter.value
  if (activeTab.value !== 'messages') query.tab = activeTab.value
  if (selectedPeer.value) {
    query.accountId = selectedPeer.value.account_id
    query.peerType = selectedPeer.value.peer_type
    query.peerId = selectedPeer.value.peer_id
  }

  await router.replace({ path: '/bot-console', query })
}

function clearExecutionDetail() {
  executionTasks.value = []
  harnessEvents.value = []
  harnessCheckpoints.value = []
}

function clearPeerDetails() {
  messages.value = []
  executionRuns.value = []
  selectedExecutionRunId.value = ''
  clearExecutionDetail()
  clearTurnLogs()
}

function clearTurnLogs() {
  turnLogs.value = []
  turnLogError.value = ''
  turnLogDetails.value = {}
  turnLogDetailLoading.value = {}
  turnLogDetailErrors.value = {}
}

async function loadExecutionDetail(runId: string) {
  if (!runId) {
    clearExecutionDetail()
    return
  }

  loadingExecutionDetail.value = true
  try {
    const [tasks, events, checkpoints] = await Promise.all([
      getBotExecutionTasks(runId),
      getBotHarnessEvents(runId),
      getBotHarnessCheckpoints(runId),
    ])
    executionTasks.value = tasks
    harnessEvents.value = events
    harnessCheckpoints.value = checkpoints
  } finally {
    loadingExecutionDetail.value = false
  }
}

async function syncSelectedExecutionForCurrentFilter() {
  const nextRunId = visibleExecutionRuns.value.some((run) => run.id === selectedExecutionRunId.value)
    ? selectedExecutionRunId.value
    : visibleExecutionRuns.value[0]?.id || ''
  selectedExecutionRunId.value = nextRunId
  await loadExecutionDetail(nextRunId)
}

async function loadTurnLogs() {
  const conversationId = String(selectedConversationId.value || '').trim()
  if (!conversationId) {
    clearTurnLogs()
    return
  }

  loadingTurnLogs.value = true
  turnLogError.value = ''
  turnLogDetails.value = {}
  turnLogDetailLoading.value = {}
  turnLogDetailErrors.value = {}
  try {
    turnLogs.value = await getAiTurnLogs({
      date: turnLogDate.value,
      conversationId,
      limit: turnLogLimit.value,
    })
  } catch (error) {
    turnLogs.value = []
    turnLogError.value = String(error)
  } finally {
    loadingTurnLogs.value = false
  }
}

async function ensureTurnLogDetailLoaded(entry: AiTurnLogSummaryEntry) {
  if (turnLogDetails.value[entry.session_id] || turnLogDetailLoading.value[entry.session_id]) {
    return
  }

  turnLogDetailLoading.value[entry.session_id] = true
  turnLogDetailErrors.value[entry.session_id] = ''
  try {
    const detail = await getAiTurnLogDetail(turnLogDate.value, entry.session_id)
    turnLogDetails.value[entry.session_id] = detail
    if (!detail) {
      turnLogDetailErrors.value[entry.session_id] = t('botConsole.turnLogs.detailMissing')
    }
  } catch (error) {
    turnLogDetailErrors.value[entry.session_id] = String(error)
  } finally {
    turnLogDetailLoading.value[entry.session_id] = false
  }
}

function getTurnLogDetail(sessionId: string): AiTurnLogEntry | null {
  return turnLogDetails.value[sessionId] || null
}

function isTurnLogDetailLoading(sessionId: string): boolean {
  return turnLogDetailLoading.value[sessionId] === true
}

function getTurnLogDetailError(sessionId: string): string {
  return turnLogDetailErrors.value[sessionId] || ''
}

async function handleTurnLogToggle(entry: AiTurnLogSummaryEntry, event: Event) {
  const target = event.target as HTMLDetailsElement | null
  if (!target?.open) return
  await ensureTurnLogDetailLoaded(entry)
}

async function loadPeerDetails(peer: BotPeer) {
  loadingDetails.value = true
  loadError.value = ''

  try {
    const [nextMessages, nextRuns] = await Promise.all([
      listBotMessagesForPeer({
        transport: peer.transport,
        accountId: peer.account_id,
        peerType: peer.peer_type,
        peerId: peer.peer_id,
        limit: 200,
      }),
      listBotExecutionRunsForPeer({
        transport: peer.transport,
        accountId: peer.account_id,
        peerType: peer.peer_type,
        peerId: peer.peer_id,
        limit: 100,
      }),
    ])

    messages.value = nextMessages
    executionRuns.value = nextRuns

    selectedExecutionRunId.value = nextRuns.some((run) => run.id === selectedExecutionRunId.value)
      ? selectedExecutionRunId.value
      : nextRuns[0]?.id || ''
    await syncSelectedExecutionForCurrentFilter()
    await loadTurnLogs()
  } catch (error) {
    loadError.value = String(error)
    clearPeerDetails()
  } finally {
    loadingDetails.value = false
  }
}

async function loadPeerList() {
  loadingPeers.value = true
  loadError.value = ''

  try {
    accounts.value = await listBotAccounts(selectedTransport.value || null)
    peers.value = await listBotPeers({
      transport: selectedTransport.value || null,
      accountId: selectedAccountId.value || null,
      peerType: selectedPeerType.value || null,
      limit: 200,
    })

    const routePeerId = typeof route.query.peerId === 'string' ? route.query.peerId : ''
    const routePeerType = typeof route.query.peerType === 'string' ? route.query.peerType : ''
    const visiblePeers = filteredPeers.value
    const preferredPeer =
      visiblePeers.find((peer) => {
        if (!routePeerId || !routePeerType) return false
        return peer.peer_id === routePeerId && peer.peer_type === routePeerType
      }) ??
      visiblePeers.find((peer) => peerKey(peer) === selectedPeerKey.value) ??
      visiblePeers[0]

    selectedPeerKey.value = preferredPeer ? peerKey(preferredPeer) : ''
    if (preferredPeer) {
      await loadPeerDetails(preferredPeer)
    } else {
      clearPeerDetails()
    }
  } catch (error) {
    loadError.value = String(error)
    peers.value = []
    clearPeerDetails()
  } finally {
    loadingPeers.value = false
  }
}

async function refreshConsole() {
  await loadPeerList()
  await syncRoute()
}

function openAccountsDialog() {
  if (!isBotConsoleActivated.value) return
  accountsDialogRef.value?.showModal()
}

function closeAccountsDialog() {
  accountsDialogRef.value?.close()
}

async function refreshSelectedPeer() {
  if (!selectedPeer.value) return
  await loadPeerDetails(selectedPeer.value)
  await syncRoute()
}

async function handleTransportChange() {
  selectedAccountId.value = ''
  selectedPeerKey.value = ''
  await refreshConsole()
}

async function handleBotAccountChange(payload: { accountId: string; transport: string }) {
  selectedTransport.value = payload.transport.trim()
  selectedAccountId.value = payload.accountId.trim()
  selectedPeerKey.value = ''
  await refreshConsole()
}

async function handleBotAccountUpdated(payload: { accountId: string; transport: string }) {
  selectedTransport.value = payload.transport.trim()
  selectedAccountId.value = payload.accountId.trim()
  selectedPeerKey.value = ''
  await refreshConsole()
}

async function handlePeerFilterChange() {
  selectedPeerKey.value = ''
  await refreshConsole()
}

async function handlePeerStateFilterChange() {
  const nextPeer = filteredPeers.value.find((peer) => peerKey(peer) === selectedPeerKey.value) ?? filteredPeers.value[0] ?? null
  selectedPeerKey.value = nextPeer ? peerKey(nextPeer) : ''
  if (nextPeer) {
    await loadPeerDetails(nextPeer)
  } else {
    clearPeerDetails()
  }
  await syncRoute()
}

async function selectPeer(peer: BotPeer) {
  const nextKey = peerKey(peer)
  if (selectedPeerKey.value === nextKey) return
  selectedPeerKey.value = nextKey
  selectedAccountId.value = peer.account_id
  selectedTransport.value = peer.transport
  selectedPeerType.value = peer.peer_type
  await loadPeerDetails(peer)
  await syncRoute()
}

async function selectExecutionRun(runId: string) {
  if (!runId || selectedExecutionRunId.value === runId) return
  selectedExecutionRunId.value = runId
  await loadExecutionDetail(runId)
  await loadTurnLogs()
}

async function handleExecutionFilterChange() {
  await syncSelectedExecutionForCurrentFilter()
  await loadTurnLogs()
  await syncRoute()
}

async function focusExecutionRun(runId?: string | null) {
  const normalized = String(runId || '').trim()
  if (!normalized) return
  activeTab.value = 'executions'
  if (!visibleExecutionRuns.value.some((run) => run.id === normalized)) {
    executionFilter.value = 'all'
  }
  if (!executionRuns.value.some((run) => run.id === normalized) && selectedPeer.value) {
    await loadPeerDetails(selectedPeer.value)
  }
  await selectExecutionRun(normalized)
  await syncRoute()
}

async function setActiveTab(tab: 'messages' | 'executions' | 'turnLogs' | 'missions' | 'observer') {
  if (activeTab.value === tab) return
  activeTab.value = tab
  await syncRoute()
}

async function openConversationInAssistant() {
  if (!selectedConversationId.value) return
  await router.push({
    path: '/ai-assistant',
    query: { conversationId: selectedConversationId.value },
  })
}

async function openTurnLogConversation(conversationId?: string | null) {
  const normalized = String(conversationId || '').trim()
  if (!normalized) return
  await router.push({
    path: '/ai-assistant',
    query: { conversationId: normalized },
  })
}

onMounted(async () => {
  await loadPeerList()
  await syncRoute()
})
</script>
