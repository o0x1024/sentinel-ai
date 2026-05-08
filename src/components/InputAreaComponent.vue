<template>
  <div
    class="input-area-container border-t border-base-300/50 bg-base-100 flex-shrink-0 relative z-0"
    @dragover.prevent="onDragOver"
    @dragleave.prevent="onDragLeave"
    @drop.prevent="onDrop"
    :class="{ 'drag-over': isDragOver }"
  >
    <!-- Drag overlay -->
    <div v-if="isDragOver" class="drag-overlay">
      <div class="drag-content">
        <i class="fas fa-file-upload text-4xl mb-2"></i>
        <span class="text-lg">{{ t('agent.document.dropDocuments') }}</span>
        <span class="text-sm opacity-70">{{ t('agent.document.supportedTypes') }}</span>
      </div>
    </div>

    <!-- Input area (refactored) -->
    <div class="px-4 pb-3 pt-2">
      <div v-if="props.referencedMessages && props.referencedMessages.length > 0" class="mb-2">
        <div class="flex items-center justify-between mb-1">
          <span class="text-xs text-base-content/60 flex items-center gap-1">
            <i class="fas fa-comment-dots text-info"></i>
            引用的消息 ({{ props.referencedMessages.length }})
          </span>
          <button
            @click="clearReferencedMessages"
            class="btn btn-xs btn-ghost text-base-content/60 hover:text-error"
            title="清除所有消息引用"
          >
            <i class="fas fa-times"></i>
            清除
          </button>
        </div>
        <div class="flex flex-wrap gap-2 max-h-32 overflow-y-auto">
          <div
            v-for="(message, idx) in props.referencedMessages"
            :key="message.id"
            class="group relative flex items-center gap-2 px-2 py-1 bg-info/10 border border-info/25 rounded-lg text-xs"
          >
            <span class="badge badge-xs badge-info">{{ message.roleLabel }}</span>
            <span class="text-base-content/80 truncate max-w-80" :title="message.content">
              {{ message.content }}
            </span>
            <button
              @click="removeReferencedMessage(idx)"
              class="ml-1 flex h-4 w-4 items-center justify-center rounded-full bg-error/80 text-xs text-error-content opacity-0 transition-opacity group-hover:opacity-100 group-focus-within:opacity-100 focus-visible:opacity-100"
              title="移除"
            >
              <i class="fas fa-times text-[10px]"></i>
            </button>
          </div>
        </div>
      </div>

      <div v-if="props.referencedFiles && props.referencedFiles.length > 0" class="mb-2">
        <div class="flex items-center justify-between mb-1">
          <span class="text-xs text-base-content/60 flex items-center gap-1">
            <i class="fas fa-file-code text-secondary"></i>
            引用的文件 ({{ props.referencedFiles.length }})
          </span>
          <button
            @click="clearReferencedFiles"
            class="btn btn-xs btn-ghost text-base-content/60 hover:text-error"
            title="清除所有文件引用"
          >
            <i class="fas fa-times"></i>
            清除
          </button>
        </div>
        <div class="flex flex-wrap gap-2 max-h-32 overflow-y-auto">
          <div
            v-for="(file, idx) in props.referencedFiles"
            :key="file.id"
            class="group relative flex items-center gap-2 px-2 py-1 bg-secondary/10 border border-secondary/25 rounded-lg text-xs"
          >
            <span class="badge badge-xs badge-outline badge-secondary">FILE</span>
            <span
              class="font-medium text-base-content/80 truncate max-w-60"
              :title="file.relativePath"
            >
              {{ file.relativePath }}
            </span>
            <span class="text-base-content/60 whitespace-nowrap">
              {{ formatReferencedFileSize(file.size) }}
            </span>
            <button
              @click="removeReferencedFile(idx)"
              class="ml-1 flex h-4 w-4 items-center justify-center rounded-full bg-error/80 text-xs text-error-content opacity-0 transition-opacity group-hover:opacity-100 group-focus-within:opacity-100 focus-visible:opacity-100"
              title="移除"
            >
              <i class="fas fa-times text-[10px]"></i>
            </button>
          </div>
        </div>
      </div>

      <!-- 流量引用显示区 -->
      <div v-if="props.referencedTraffic && props.referencedTraffic.length > 0" class="mb-2">
        <div class="flex items-center justify-between mb-1">
          <span class="text-xs text-base-content/60 flex items-center gap-1">
            <i class="fas fa-network-wired text-accent"></i>
            引用的流量 ({{ props.referencedTraffic.length }})
          </span>
          <button
            @click="clearReferencedTraffic"
            class="btn btn-xs btn-ghost text-base-content/60 hover:text-error"
            title="清除所有引用"
          >
            <i class="fas fa-times"></i>
            清除
          </button>
        </div>
        <div class="flex flex-wrap gap-2 max-h-32 overflow-y-auto">
          <div
            v-for="(traffic, idx) in props.referencedTraffic"
            :key="traffic.id"
            class="group relative flex items-center gap-2 px-2 py-1 bg-accent/10 border border-accent/30 rounded-lg text-xs"
          >
            <!-- 类型标签 -->
            <span :class="['badge badge-xs', getTypeBadgeClass(traffic.sendType)]">
              {{ getTypeLabel(traffic.sendType) }}
            </span>
            <span :class="['badge badge-xs', getMethodBadgeClass(traffic.method)]">
              {{ traffic.method }}
            </span>
            <span class="text-base-content/80 truncate max-w-40" :title="traffic.url">
              {{ traffic.host }}{{ getUrlPath(traffic.url) }}
            </span>
            <span
              v-if="traffic.sendType !== 'request'"
              :class="['badge badge-xs', getStatusBadgeClass(traffic.status_code)]"
            >
              {{ traffic.status_code || 'N/A' }}
            </span>
            <button
              @click="removeReferencedTraffic(idx)"
              class="ml-1 flex h-4 w-4 items-center justify-center rounded-full bg-error/80 text-xs text-error-content opacity-0 transition-opacity group-hover:opacity-100 group-focus-within:opacity-100 focus-visible:opacity-100"
              title="移除"
            >
              <i class="fas fa-times text-[10px]"></i>
            </button>
          </div>
        </div>
      </div>

      <div v-if="props.referencedAssets && props.referencedAssets.length > 0" class="mb-2">
        <div class="flex items-center justify-between mb-1">
          <span class="text-xs text-base-content/60 flex items-center gap-1">
            <i class="fas fa-cubes text-primary"></i>
            引用的资产 ({{ props.referencedAssets.length }})
          </span>
          <button
            @click="clearReferencedAssets"
            class="btn btn-xs btn-ghost text-base-content/60 hover:text-error"
            title="清除所有资产引用"
          >
            <i class="fas fa-times"></i>
            清除
          </button>
        </div>
        <div class="flex flex-wrap gap-2 max-h-32 overflow-y-auto">
          <div
            v-for="(asset, idx) in props.referencedAssets"
            :key="asset.id"
            class="group relative flex items-center gap-2 px-2 py-1 bg-primary/10 border border-primary/25 rounded-lg text-xs"
          >
            <span class="badge badge-xs badge-outline">{{ asset.asset_type }}</span>
            <span :class="['badge badge-xs', getAssetRiskBadgeClass(asset.risk_level)]">
              {{ asset.risk_level || 'unknown' }}
            </span>
            <span class="font-medium text-base-content/80 truncate max-w-40" :title="asset.name">
              {{ asset.name }}
            </span>
            <span class="text-base-content/60 truncate max-w-44" :title="asset.value">
              {{ asset.value }}
            </span>
            <button
              @click="removeReferencedAsset(idx)"
              class="ml-1 flex h-4 w-4 items-center justify-center rounded-full bg-error/80 text-xs text-error-content opacity-0 transition-opacity group-hover:opacity-100 group-focus-within:opacity-100 focus-visible:opacity-100"
              title="移除"
            >
              <i class="fas fa-times text-[10px]"></i>
            </button>
          </div>
        </div>
      </div>

      <!-- 图片附件预览区 -->
      <div
        v-if="pendingAttachments && pendingAttachments.length > 0"
        class="mb-2 flex flex-wrap gap-2"
      >
        <div v-for="(att, idx) in pendingAttachments" :key="idx" class="relative group">
          <img
            :src="getAttachmentPreview(att)"
            class="h-16 w-16 object-cover rounded border border-base-300 bg-base-200"
            :alt="att.image?.filename || 'attachment'"
          />
          <button
            @click="removeAttachment(idx)"
            class="absolute -top-1 -right-1 flex h-5 w-5 items-center justify-center rounded-full bg-error text-xs text-error-content opacity-0 transition-opacity group-hover:opacity-100 group-focus-within:opacity-100 focus-visible:opacity-100"
            title="移除"
          >
            <i class="fas fa-times"></i>
          </button>
        </div>
      </div>

      <!-- 文档附件预览区 -->
      <div v-if="pendingDocuments && pendingDocuments.length > 0" class="mb-2 flex flex-wrap gap-2">
        <div
          v-for="(doc, idx) in pendingDocuments"
          :key="doc.id"
          class="inline-flex items-center gap-2 px-2 py-1 rounded-lg border border-base-300 bg-base-200 text-xs"
        >
          <i class="fas fa-file-lines text-primary"></i>
          <span class="font-medium truncate max-w-44" :title="doc.original_filename">{{
            doc.original_filename
          }}</span>
          <span class="text-base-content/60">({{ formatFileSize(doc.file_size) }})</span>
          <span v-if="doc.status === 'processing'" class="badge badge-xs badge-info">
            {{ t('common.loading') }}
          </span>
          <span v-else-if="doc.status === 'failed'" class="badge badge-xs badge-error">
            failed
          </span>
          <button
            @click="removeDocument(idx)"
            class="w-4 h-4 rounded-full bg-error text-error-content flex items-center justify-center text-[10px]"
            title="移除"
          >
            <i class="fas fa-times"></i>
          </button>
          <button
            v-if="doc.status === 'ready'"
            @click="runSecurityAnalysis(doc)"
            class="btn btn-ghost btn-xs"
            title="安全分析"
          >
            <i class="fas fa-shield-halved text-warning"></i>
          </button>
          <button
            v-if="doc.status === 'failed'"
            @click="retryUploadDocument(doc)"
            class="btn btn-ghost btn-xs"
            title="重试"
          >
            <i class="fas fa-rotate-right"></i>
          </button>
        </div>
      </div>

      <div
        ref="containerRef"
        class="chat-input rounded-2xl bg-base-200/60 border border-base-300/60 backdrop-blur-sm flex flex-col gap-2 px-3 py-2 shadow-sm focus-within:border-primary transition-colors"
      >
        <!-- Text input (auto-resize textarea) -->
        <div class="input-editor-shell flex-1 min-w-0">
          <div
            v-if="showTextareaMirror"
            ref="textareaMirrorRef"
            class="textarea-mirror"
            aria-hidden="true"
          >
            <pre
              class="textarea-mirror-content"
            ><template v-for="(segment, idx) in mirrorRenderSegments" :key="segment.type === 'mention' ? `${segment.id}-${segment.start}-${idx}` : `${segment.type}-${segment.start}-${idx}`"><span v-if="segment.type === 'text'">{{ segment.value }}</span><span v-else-if="segment.type === 'caret'" class="textarea-mirror-caret" aria-hidden="true"></span><span v-else :class="['mention-inline-token', getMentionInlineClass(segment.kind)]" @mouseenter="showMentionPreview(segment, $event)" @mousemove="showMentionPreview(segment, $event)" @mousedown.prevent="handleMentionTokenPointerDown" @click.prevent="beginMentionReplacement(segment)" @mouseleave="scheduleHideMentionPreview">{{ segment.value }}</span></template>{{ highlightedPlainSuffix }}</pre>
          </div>
          <textarea
            ref="textareaRef"
            :value="inputMessage"
            @input="onInput"
            @keydown="onKeydown"
            @click="onCaretChanged"
            @keyup="onCaretChanged"
            @select="onCaretChanged"
            @scroll="onTextareaScroll"
            @focus="onTextareaFocus"
            @blur="onTextareaBlur"
            @compositionstart="onCompositionStart"
            @compositionend="onCompositionEnd"
            :disabled="isLoading && !allowTakeover"
            :placeholder="placeholderText"
            :class="[
              'w-full bg-transparent outline-none resize-none leading-relaxed text-sm placeholder:text-base-content/50 max-h-40 input-textarea',
              showTextareaMirror
                ? 'text-transparent caret-base-content'
                : 'text-base-content caret-base-content',
            ]"
            rows="1"
          />
        </div>
        <div
          v-if="activeMentionPreview"
          class="mention-preview-popover"
          @mouseenter="cancelHideMentionPreview"
          @mouseleave="scheduleHideMentionPreview"
          :style="{
            left: `${activeMentionPreview.x}px`,
            top: `${activeMentionPreview.y}px`,
          }"
        >
          <div class="mention-preview-title">{{ activeMentionPreview.title }}</div>
          <div class="mention-preview-body">{{ activeMentionPreview.body }}</div>
          <div class="mention-preview-actions">
            <button type="button" class="mention-preview-btn" @click="removePreviewReference">
              移除
            </button>
            <template v-if="activeMentionPreview.kind === 'traffic'">
              <button
                type="button"
                class="mention-preview-btn"
                :class="{
                  'mention-preview-btn-active': activeMentionPreview.sendType === 'request',
                }"
                @click="setPreviewTrafficSendType('request')"
              >
                Request
              </button>
              <button
                type="button"
                class="mention-preview-btn"
                :class="{
                  'mention-preview-btn-active': activeMentionPreview.sendType === 'response',
                }"
                @click="setPreviewTrafficSendType('response')"
              >
                Response
              </button>
              <button
                type="button"
                class="mention-preview-btn"
                :class="{
                  'mention-preview-btn-active':
                    activeMentionPreview.sendType === 'both' || !activeMentionPreview.sendType,
                }"
                @click="setPreviewTrafficSendType('both')"
              >
                Both
              </button>
            </template>
          </div>
        </div>
        <div
          v-if="mentionOpen"
          class="slash-popover border border-base-300 bg-base-100 rounded-xl shadow-xl"
        >
          <div class="px-3 py-2 border-b border-base-300/60 text-xs text-base-content/60">
            引用资源
          </div>
          <div v-if="mentionLoading" class="px-3 py-2 text-sm text-base-content/60">
            正在搜索文件、资产和流量...
          </div>
          <div v-else-if="mentionError" class="px-3 py-2 text-sm text-error">
            {{ mentionError }}
          </div>
          <div
            v-else-if="filteredMentionItems.length === 0"
            class="px-3 py-2 text-sm text-base-content/60"
          >
            无匹配资源
          </div>
          <div v-else class="py-1 max-h-64 overflow-y-auto">
            <button
              v-for="(item, idx) in filteredMentionItems"
              :key="item.id"
              class="w-full text-left px-3 py-2 transition-colors"
              :class="
                idx === mentionActiveIndex
                  ? 'bg-primary/15 text-primary'
                  : 'hover:bg-base-200 text-base-content'
              "
              @mousedown.prevent="applyMentionSelection(item)"
            >
              <div class="flex items-center gap-4">
                <span :class="['badge badge-xs shrink-0', getMentionBadgeClass(item.kind)]">
                  {{ getMentionBadgeLabel(item.kind) }}
                </span>
                <span class="font-semibold text-sm truncate min-w-0 flex-1">{{ item.label }}</span>
                <span class="text-xs opacity-70 whitespace-nowrap">{{ item.description }}</span>
              </div>
            </button>
          </div>
        </div>
        <div
          v-if="slashOpen"
          class="slash-popover border border-base-300 bg-base-100 rounded-xl shadow-xl"
        >
          <div class="px-3 py-2 border-b border-base-300/60 text-xs text-base-content/60">
            Slash Commands
          </div>
          <div
            v-if="filteredSlashCommands.length === 0"
            class="px-3 py-2 text-sm text-base-content/60"
          >
            无匹配命令
          </div>
          <div v-else class="py-1 max-h-64 overflow-y-auto">
            <button
              v-for="(cmd, idx) in filteredSlashCommands"
              :key="cmd.id"
              class="w-full text-left px-3 py-2 transition-colors"
              :class="
                idx === slashActiveIndex
                  ? 'bg-primary/15 text-primary'
                  : 'hover:bg-base-200 text-base-content'
              "
              @mousedown.prevent="applySlashCommand(cmd)"
            >
              <div class="flex items-center gap-4">
                <span class="font-semibold text-base whitespace-nowrap min-w-28"
                  >/{{ cmd.name }}</span
                >
                <div class="text-sm opacity-75 truncate flex-1">
                  {{
                    cmd.description ||
                    (cmd.type === 'action' ? getActionLabel(cmd.action) : '自定义提示词命令')
                  }}
                </div>
                <span class="text-xs opacity-70 whitespace-nowrap">{{
                  cmd.type === 'action' ? '功能' : '提示词'
                }}</span>
              </div>
            </button>
          </div>
        </div>

        <!-- Toolbar: left actions and right send/stop -->
        <div class="flex items-center justify-between gap-2">
          <InputToolbarActions
            :rag-enabled="effectiveRagEnabled"
            :web-search-enabled="effectiveWebSearchEnabled"
            @trigger-file-select="triggerFileSelect"
            @open-tool-config="emit('open-tool-config')"
            @toggle-rag="toggleRAG"
            @toggle-web-search="toggleWebSearch"
            @open-slash-manager="openSlashManager"
            @clear-conversation="clearConversation"
          />

          <!-- Right side icons -->
          <div class="flex items-center gap-2 shrink-0">
            <!-- Context usage indicator -->
            <div
              v-if="effectiveContextUsage"
              class="context-usage-indicator flex items-center gap-1 px-2 py-1 rounded-md text-xs cursor-default"
              :class="contextUsageClass"
              :title="contextUsageTooltip"
            >
              <span class="font-medium">{{ contextUsagePercentage }}%</span>
              <span class="opacity-70">·</span>
              <span class="opacity-80"
                >{{ formatTokenCount(effectiveContextUsage.usedTokens) }} /
                {{ formatTokenCount(effectiveContextUsage.maxTokens) }}</span
              >
              <span class="opacity-70 hidden sm:inline">{{ t('agent.contextUsed') }}</span>
            </div>
            <div class="assistant-agent-switch">
              <SearchableSelect
                :model-value="effectiveSelectedAgent"
                :options="availableAgentOptions"
                :placeholder="agentLoading ? '加载 Agent 中...' : '选择 Agent'"
                search-placeholder="搜索 Agent..."
                no-results-text="无匹配 Agent"
                :disabled="agentLoading || availableAgentOptions.length === 0"
                size="sm"
                direction="up"
                variant="toolbar"
                :auto-width="true"
                align="right"
                @update:model-value="effectiveSelectedAgent = $event"
                @change="onAgentChanged"
              />
            </div>
            <button class="icon-btn" title="语言 / 翻译"><i class="fas fa-language"></i></button>
            <button
              v-if="!isLoading || allowTakeover"
              class="send-btn"
              :disabled="!canSend"
              :class="{ 'opacity-40 cursor-not-allowed': !canSend }"
              @click="emitSend"
              :title="isLoading ? '排队为下一轮 (Enter)' : '发送 (Enter)'"
            >
              <i class="fas fa-arrow-up"></i>
            </button>
            <button
              v-if="isLoading && allowTakeover"
              class="send-btn bg-warning text-warning-content hover:bg-warning/90"
              :disabled="!canSend"
              :class="{ 'opacity-40 cursor-not-allowed': !canSend }"
              @click="emitInterruptSend"
              title="中断当前执行并发送"
            >
              <i class="fas fa-bolt"></i>
            </button>
            <button
              v-if="isLoading"
              class="send-btn bg-error text-error-content hover:bg-error/90"
              @click="handleStop"
              title="停止执行"
            >
              <i class="fas fa-stop"></i>
            </button>
          </div>
        </div>
      </div>
      <!-- Hidden file input for attachments -->
      <input
        ref="fileInputRef"
        type="file"
        class="hidden"
        multiple
        accept="*/*"
        @change="onFilesSelected"
      />

      <Teleport to="body">
        <AppDialog v-if="showSlashManager" class="modal modal-open slash-manager-modal">
          <div class="modal-box max-w-3xl">
            <h3 class="font-bold text-lg">Slash Commands</h3>
            <p class="text-sm text-base-content/70 mt-1">输入框中键入 <code>/</code> 可调用命令</p>
            <div class="mt-3 flex flex-wrap items-center gap-2">
              <select v-model="slashManagerScope" class="select select-bordered select-sm w-40">
                <option value="global">全局命令</option>
                <option value="conversation" :disabled="!conversationScopeEnabled">会话命令</option>
              </select>
              <button type="button" class="btn btn-outline btn-sm" @click="exportSlashCommands">
                导出 JSON
              </button>
              <button
                type="button"
                class="btn btn-outline btn-sm"
                @click="triggerImportSlashCommands"
              >
                导入 JSON
              </button>
              <span
                v-if="slashManagerScope === 'conversation' && conversationScopeEnabled"
                class="text-xs text-base-content/70"
              >
                当前会话：{{ conversationScopeKey }}
              </span>
            </div>

            <div class="grid grid-cols-1 lg:grid-cols-2 gap-4 mt-4">
              <div class="border border-base-300 rounded-lg p-3 max-h-80 overflow-y-auto">
                <div class="text-xs uppercase tracking-wide text-base-content/60 mb-2">
                  命令列表
                </div>
                <div v-if="managerBuiltinCommands.length > 0" class="space-y-2 mb-2">
                  <div
                    v-for="cmd in managerBuiltinCommands"
                    :key="cmd.id"
                    class="rounded-lg border border-base-300/70 px-3 py-2 bg-base-100"
                  >
                    <div class="flex items-center justify-between gap-2">
                      <div>
                        <div class="font-medium">
                          /{{ cmd.name }} <span class="text-xs opacity-60">(内置)</span>
                        </div>
                        <div class="text-xs opacity-70 truncate">{{ cmd.description || '-' }}</div>
                      </div>
                      <span class="badge badge-ghost badge-sm">只读</span>
                    </div>
                  </div>
                </div>

                <draggable
                  v-model="managerCustomCommands"
                  item-key="id"
                  handle=".drag-handle"
                  :animation="150"
                  class="space-y-2"
                  @end="onDragSortEnd"
                >
                  <template #item="{ element: cmd }">
                    <div class="rounded-lg border border-base-300/70 px-3 py-2 bg-base-100">
                      <div class="flex items-center justify-between gap-2">
                        <div class="flex items-center gap-2 min-w-0">
                          <button
                            type="button"
                            class="btn btn-ghost btn-xs drag-handle cursor-grab"
                            title="拖拽排序"
                          >
                            <i class="fas fa-grip-vertical"></i>
                          </button>
                          <div class="min-w-0">
                            <div class="font-medium">
                              /{{ cmd.name }}
                              <span v-if="cmd.scope" class="text-xs opacity-60"
                                >({{ getScopeLabel(cmd.scope) }})</span
                              >
                            </div>
                            <div class="text-xs opacity-70 truncate">
                              {{ cmd.description || '-' }}
                            </div>
                          </div>
                        </div>
                        <div class="flex items-center gap-1">
                          <button
                            type="button"
                            class="btn btn-ghost btn-xs"
                            @click="startEditCommand(cmd)"
                            title="编辑"
                          >
                            编辑
                          </button>
                          <button
                            type="button"
                            class="btn btn-ghost btn-xs text-error"
                            @click="deleteSlashCommand(cmd.id)"
                          >
                            删除
                          </button>
                        </div>
                      </div>
                      <label class="label py-1">
                        <span class="label-text text-xs">启用</span>
                        <input
                          type="checkbox"
                          class="toggle toggle-xs"
                          :checked="cmd.enabled"
                          @change="onCommandEnabledChange(cmd, $event)"
                        />
                      </label>
                    </div>
                  </template>
                </draggable>
                <div
                  v-if="managerCustomCommands.length === 0"
                  class="text-xs text-base-content/60 px-1 py-2"
                >
                  当前作用域暂无自定义命令
                </div>
              </div>

              <div class="border border-base-300 rounded-lg p-3">
                <div class="text-xs uppercase tracking-wide text-base-content/60 mb-2">
                  {{ editingSlashId ? '编辑命令' : '新增自定义命令' }}
                </div>
                <div class="space-y-3">
                  <label class="form-control">
                    <span class="label-text text-xs">命令名</span>
                    <input
                      v-model.trim="newSlashCommand.name"
                      class="input input-bordered input-sm"
                      placeholder="review"
                    />
                  </label>
                  <label class="form-control">
                    <span class="label-text text-xs">描述</span>
                    <input
                      v-model.trim="newSlashCommand.description"
                      class="input input-bordered input-sm"
                      placeholder="审查当前改动"
                    />
                  </label>
                  <label class="form-control">
                    <span class="label-text text-xs">类型</span>
                    <select v-model="newSlashCommand.type" class="select select-bordered select-sm">
                      <option value="prompt">提示词</option>
                      <option value="action">功能</option>
                    </select>
                  </label>
                  <label class="form-control">
                    <span class="label-text text-xs">作用域</span>
                    <select
                      v-model="newSlashCommand.scope"
                      class="select select-bordered select-sm"
                    >
                      <option value="global">全局</option>
                      <option value="conversation" :disabled="!conversationScopeEnabled">
                        会话
                      </option>
                    </select>
                  </label>
                  <label v-if="newSlashCommand.type === 'prompt'" class="form-control">
                    <span class="label-text text-xs">提示词模板</span>
                    <textarea
                      v-model="newSlashCommand.template"
                      class="textarea textarea-bordered textarea-sm h-24"
                      placeholder="请审查当前改动：{{input}}"
                    ></textarea>
                  </label>
                  <label v-else class="form-control">
                    <span class="label-text text-xs">功能</span>
                    <select
                      v-model="newSlashCommand.action"
                      class="select select-bordered select-sm"
                    >
                      <option value="new_conversation">新建会话</option>
                      <option value="clear_conversation">清空会话</option>
                      <option value="open_tool_config">打开 Agent 配置</option>
                    </select>
                  </label>
                  <label
                    v-if="newSlashCommand.type === 'prompt'"
                    class="label cursor-pointer justify-start gap-2"
                  >
                    <input
                      type="checkbox"
                      class="checkbox checkbox-sm"
                      v-model="newSlashCommand.auto_send"
                    />
                    <span class="label-text text-xs">执行后立即发送</span>
                  </label>
                  <div
                    v-if="slashFormError"
                    class="text-xs text-error bg-error/10 border border-error/20 rounded px-2 py-1"
                  >
                    {{ slashFormError }}
                  </div>
                  <button
                    type="button"
                    class="btn btn-primary btn-sm w-full"
                    @click="editingSlashId ? saveEditedSlashCommand() : addCustomSlashCommand()"
                  >
                    {{ editingSlashId ? '保存修改' : '添加命令' }}
                  </button>
                  <button
                    v-if="editingSlashId"
                    type="button"
                    class="btn btn-ghost btn-sm w-full"
                    @click="cancelEditCommand"
                  >
                    取消编辑
                  </button>
                </div>
              </div>
            </div>

            <div class="modal-action">
              <button type="button" class="btn" @click="closeSlashManager">关闭</button>
            </div>
          </div>
          <form method="dialog" class="modal-backdrop">
            <button @click.prevent="closeSlashManager">close</button>
          </form>
        </AppDialog>
      </Teleport>
      <input
        ref="importSlashInputRef"
        type="file"
        class="hidden"
        accept="application/json,.json"
        @change="onImportSlashFileChange"
      />
    </div>
  </div>
</template>

<script setup lang="ts">
import { onMounted, onUnmounted, ref, computed, nextTick, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import draggable from 'vuedraggable'
import SearchableSelect from '@/components/SearchableSelect.vue'
import InputToolbarActions from '@/components/InputArea/InputToolbarActions.vue'
import {
  formatReferencedFileSize,
  getAssetRiskBadgeClass,
  getMentionBadgeClass,
  getMentionBadgeLabel,
  getMentionInlineClass,
  getMethodBadgeClass,
  getStatusBadgeClass,
  getTypeBadgeClass,
  getTypeLabel,
  getUrlPath,
} from '@/components/InputArea/inputAreaDisplay'
import {
  buildHighlightedMentionSegments,
  buildMirrorRenderSegments,
  resolveMirrorCaretIndex,
  type MirrorMentionSegment,
  type MirrorRenderSegment,
  type MirrorTextSegment,
} from '@/components/InputArea/inputMirrorSupport'
import { useInputAttachments } from '@/components/InputArea/useInputAttachments'
import {
  expandSelectionToMentionBoundaries,
  findMentionTokenAdjacentToCursor,
  findMentionTokenForDeletion,
  removeMentionTokenText,
  removeTextRange,
  snapCursorToMentionBoundary,
} from '@/components/InputArea/mentionTokenSupport'
import { useInputMentions } from '@/components/InputArea/useInputMentions'
import { useInputSlashCommands } from '@/components/InputArea/useInputSlashCommands'
import type {
  ReferencedAsset,
  ReferencedConversationMessage,
  ReferencedFile,
  ReferencedTraffic,
  TrafficSendType,
} from '@/components/Agent/agentDraftTypes'
import type {
  AgentMessage,
  PendingDocumentAttachment,
  ProcessedDocumentResult,
} from '@/types/agent'
import type { MentionTokenKind } from '@/components/InputArea/mentionTokenSupport'

const { t } = useI18n()

// Context usage info type
interface MemoryTraceCount {
  label: string
  count: number
}

interface MemoryRetrievalInfo {
  queryPreview: string
  requestedTopK: number
  hitCount: number
  usedCanonicalFallback: boolean
  includeReflection: boolean
  sourceBreakdown: MemoryTraceCount[]
  kindBreakdown: MemoryTraceCount[]
}

interface ContextUsageInfo {
  usedTokens: number
  maxTokens: number
  usagePercentage: number
  effectiveContextTokens?: number
  remainingTokens?: number
  contextPressure?: string | null
  warningThresholdTokens?: number
  autoCompactThresholdTokens?: number
  blockingThresholdTokens?: number
  outputReserveTokens?: number
  shouldCompact?: boolean
  shouldBlock?: boolean
  pressurePhase?: string | null
  taskTokens?: number
  systemPromptTokens: number
  historyTokens: number
  historyCount: number
  summaryTokens: number
  summaryGlobalTokens: number
  summarySegmentTokens: number
  summarySegmentCount: number
  sentinelMode?: boolean
  sentinelIntentId?: string | null
  sentinelIntentConfidence?: number | null
  sentinelIntentTransition?: string | null
  sentinelParentIntentId?: string | null
  sentinelClarificationNeeded?: boolean
  sentinelClarificationStatus?: string | null
  sentinelCompressionAggressiveness?: string | null
  memoryRetrieval?: MemoryRetrievalInfo | null
}

interface AgentOption {
  value: string
  label: string
  description?: string
}

const props = defineProps<{
  inputMessage: string
  conversationId?: string | null
  isLoading: boolean
  showDebugInfo: boolean
  allowTakeover?: boolean
  ragEnabled?: boolean
  webSearchEnabled?: boolean
  pendingAttachments?: any[]
  pendingDocuments?: PendingDocumentAttachment[]
  processedDocuments?: ProcessedDocumentResult[]
  referencedFiles?: ReferencedFile[]
  referencedMessages?: ReferencedConversationMessage[]
  referencedTraffic?: ReferencedTraffic[]
  referencedAssets?: ReferencedAsset[]
  availableConversationMessages?: AgentMessage[]
  contextUsage?: ContextUsageInfo | null
  availableAgents?: AgentOption[]
  selectedAgent?: string
  agentLoading?: boolean
  defaultMaxContextTokens?: number
}>()

const emit = defineEmits<{
  (e: 'update:input-message', value: string): void
  (e: 'send-message'): void
  (e: 'interrupt-message'): void
  (e: 'stop-execution'): void
  (e: 'toggle-debug', value: boolean): void
  (e: 'create-new-conversation'): void
  (e: 'clear-conversation'): void
  (e: 'toggle-rag', enabled: boolean): void
  (e: 'toggle-web-search', enabled: boolean): void
  (e: 'open-tool-config'): void
  (e: 'add-attachments', files: string[]): void
  (e: 'remove-attachment', index: number): void
  (e: 'add-documents', files: PendingDocumentAttachment[]): void
  (e: 'remove-document', index: number): void
  (e: 'document-processed', result: ProcessedDocumentResult): void
  (e: 'add-file-reference', files: ReferencedFile[]): void
  (e: 'add-message-reference', messages: ReferencedConversationMessage[]): void
  (e: 'add-traffic-reference', traffic: ReferencedTraffic[]): void
  (e: 'add-asset-reference', assets: ReferencedAsset[]): void
  (e: 'sync-file-references', files: ReferencedFile[]): void
  (e: 'sync-message-references', messages: ReferencedConversationMessage[]): void
  (e: 'sync-traffic-references', traffic: ReferencedTraffic[]): void
  (e: 'sync-asset-references', assets: ReferencedAsset[]): void
  (e: 'remove-file', index: number): void
  (e: 'clear-files'): void
  (e: 'remove-message', index: number): void
  (e: 'clear-messages'): void
  (e: 'remove-traffic', index: number): void
  (e: 'clear-traffic'): void
  (e: 'remove-asset', index: number): void
  (e: 'clear-assets'): void
  (e: 'change-agent', value: string): void
}>()

// removed architecture utilities

const allowTakeover = computed(() => props.allowTakeover === true)

// --- New input logic ---
const textareaRef = ref<HTMLTextAreaElement | null>(null)
const textareaMirrorRef = ref<HTMLDivElement | null>(null)
const containerRef = ref<HTMLDivElement | null>(null)
const hoveredMentionPreview = ref<{
  id: string
  kind: MentionTokenKind
  x: number
  y: number
} | null>(null)
let hideMentionPreviewTimer: ReturnType<typeof setTimeout> | null = null

// Feature states are fully controlled by parent.
const effectiveRagEnabled = computed(() => !!props.ragEnabled)
const effectiveWebSearchEnabled = computed(() => !!props.webSearchEnabled)
const effectiveSelectedAgent = computed({
  get: () => props.selectedAgent || '',
  set: (value: string) => {
    if (!value) return
    emit('change-agent', value)
  },
})
const availableAgentOptions = computed(() => props.availableAgents || [])

const placeholderText = computed(() => '在这里输入消息，按 Enter 发送')

const selectionState = ref({
  end: 0,
  start: 0,
})
const isTextareaFocused = ref(false)

const highlightedMentionSegments = computed<Array<MirrorTextSegment | MirrorMentionSegment>>(() => {
  return buildHighlightedMentionSegments(props.inputMessage || '')
})

const showTextareaMirror = computed(() => {
  return highlightedMentionSegments.value.some(segment => segment.type === 'mention')
})

const highlightedPlainSuffix = computed(() => '\n')

const mirrorCaretIndex = computed<number | null>(() => {
  if (!showTextareaMirror.value) return null
  return resolveMirrorCaretIndex(
    props.inputMessage || '',
    isTextareaFocused.value,
    selectionState.value.start,
    selectionState.value.end
  )
})

const mirrorRenderSegments = computed<MirrorRenderSegment[]>(() => {
  return buildMirrorRenderSegments(
    props.inputMessage || '',
    highlightedMentionSegments.value,
    mirrorCaretIndex.value
  )
})

const buildMentionReplacementQuery = (segment: {
  id: string
  kind: MentionTokenKind
  label: string
}) => {
  if (segment.kind === 'file') {
    return segment.id
  }
  if (segment.kind === 'asset') {
    return segment.label
  }
  if (segment.kind === 'message') {
    return segment.label
  }
  return segment.label.replace(/^[A-Z]+\s+/, '').trim() || segment.label
}

const getMentionPreviewPayload = (kind: MentionTokenKind, id: string) => {
  if (kind === 'file') {
    const file = (props.referencedFiles || []).find(
      item => item.relativePath === id || item.id === id
    )
    if (!file) return null
    const body = file.preview.length > 280 ? `${file.preview.slice(0, 280)}...` : file.preview
    return {
      body: body || file.relativePath,
      title: `FILE · ${file.relativePath}`,
    }
  }

  if (kind === 'asset') {
    const asset = (props.referencedAssets || []).find(item => item.id === id)
    if (!asset) return null
    const details = [
      asset.value,
      asset.description,
      asset.risk_level ? `risk=${asset.risk_level}` : '',
    ]
      .filter(Boolean)
      .join('\n')
    return {
      body: details || asset.name,
      title: `ASSET · ${asset.name}`,
    }
  }

  if (kind === 'message') {
    const message = (props.referencedMessages || []).find(item => item.id === id)
    if (!message) return null
    return {
      body: message.content,
      title: `MESSAGE · ${message.roleLabel}`,
    }
  }

  const traffic = (props.referencedTraffic || []).find(item => String(item.id) === id)
  if (!traffic) return null
  const responsePart = traffic.response_body
    ? `\n\n${traffic.response_body.slice(0, 220)}${traffic.response_body.length > 220 ? '...' : ''}`
    : ''
  return {
    kind,
    body: `${traffic.method} ${traffic.url}\nstatus=${traffic.status_code || 'N/A'}${responsePart}`,
    sendType: traffic.sendType,
    title: `HTTP · ${traffic.host}`,
  }
}

const activeMentionPreview = computed(() => {
  if (!hoveredMentionPreview.value) return null
  const payload = getMentionPreviewPayload(
    hoveredMentionPreview.value.kind,
    hoveredMentionPreview.value.id
  )
  if (!payload) return null
  return {
    ...payload,
    id: hoveredMentionPreview.value.id,
    kind: hoveredMentionPreview.value.kind,
    x: hoveredMentionPreview.value.x,
    y: hoveredMentionPreview.value.y,
  }
})

const autoResize = () => {
  const el = textareaRef.value
  if (!el) return
  el.style.height = 'auto'
  el.style.height = Math.min(el.scrollHeight, 320) + 'px'
}

const syncSelectionState = (selectionStart: number, selectionEnd: number) => {
  selectionState.value = {
    end: selectionEnd,
    start: selectionStart,
  }
}

const setCursorPosition = (
  cursor: number,
  preference: 'left' | 'right' | 'nearest' = 'nearest'
) => {
  const el = textareaRef.value
  if (!el) return
  const normalizedCursor = snapCursorToMentionBoundary(props.inputMessage || '', cursor, preference)
  el.setSelectionRange(normalizedCursor, normalizedCursor)
  syncSelectionState(normalizedCursor, normalizedCursor)
}

const normalizeSelectionRange = (text: string, selectionStart: number, selectionEnd: number) => {
  if (selectionStart === selectionEnd) {
    const cursor = snapCursorToMentionBoundary(text, selectionStart, 'nearest')
    return { end: cursor, start: cursor }
  }
  return expandSelectionToMentionBoundaries(text, selectionStart, selectionEnd)
}

const setInputValue = (value: string, cursor = value.length) => {
  emit('update:input-message', value)
  nextTick(() => {
    const el = textareaRef.value
    if (!el) return
    el.focus()
    const normalizedCursor = snapCursorToMentionBoundary(value, cursor, 'nearest')
    el.setSelectionRange(normalizedCursor, normalizedCursor)
    syncSelectionState(normalizedCursor, normalizedCursor)
    updateSlashState(value, normalizedCursor)
    updateMentionState(value, normalizedCursor)
    autoResize()
    onTextareaScroll({ target: el } as unknown as Event)
  })
}

const onInput = (e: Event) => {
  const target = e.target as HTMLTextAreaElement
  emit('update:input-message', target.value)
  syncSelectionState(target.selectionStart || 0, target.selectionEnd || target.selectionStart || 0)
  syncMentionBindings(target.value)
  updateSlashState(target.value, target.selectionStart || 0)
  updateMentionState(target.value, target.selectionStart || 0)
  autoResize()
}

const onTextareaScroll = (e: Event) => {
  const target = e.target as HTMLTextAreaElement
  const mirror = textareaMirrorRef.value
  if (!mirror) return
  mirror.scrollTop = target.scrollTop
  mirror.scrollLeft = target.scrollLeft
  hoveredMentionPreview.value = null
}

const showMentionPreview = (
  segment: { id: string; kind: MentionTokenKind; type: 'mention' },
  event: MouseEvent
) => {
  cancelHideMentionPreview()
  const payload = getMentionPreviewPayload(segment.kind, segment.id)
  const container = containerRef.value
  if (!payload || !container) {
    hoveredMentionPreview.value = null
    return
  }
  const rect = container.getBoundingClientRect()
  hoveredMentionPreview.value = {
    id: segment.id,
    kind: segment.kind,
    x: Math.max(8, event.clientX - rect.left),
    y: Math.max(8, event.clientY - rect.top - 12),
  }
}

const hideMentionPreview = () => {
  hoveredMentionPreview.value = null
}

const cancelHideMentionPreview = () => {
  if (!hideMentionPreviewTimer) return
  clearTimeout(hideMentionPreviewTimer)
  hideMentionPreviewTimer = null
}

const scheduleHideMentionPreview = () => {
  cancelHideMentionPreview()
  hideMentionPreviewTimer = setTimeout(() => {
    hoveredMentionPreview.value = null
    hideMentionPreviewTimer = null
  }, 120)
}

const handleMentionTokenPointerDown = () => {
  focusInput()
}

const onTextareaFocus = (e: FocusEvent) => {
  isTextareaFocused.value = true
  const target = e.target as HTMLTextAreaElement
  syncSelectionState(target.selectionStart || 0, target.selectionEnd || target.selectionStart || 0)
}

const onTextareaBlur = () => {
  isTextareaFocused.value = false
}

const beginMentionReplacement = (segment: {
  end: number
  id: string
  kind: MentionTokenKind
  label: string
  start: number
  type: 'mention'
}) => {
  const currentValue = props.inputMessage || ''
  const query = buildMentionReplacementQuery(segment)
  const replacement = `@${query}`
  const nextValue = `${currentValue.slice(0, segment.start)}${replacement}${currentValue.slice(segment.end)}`
  hideMentionPreview()
  setInputValue(nextValue, segment.start + replacement.length)
}

const removePreviewReference = () => {
  const preview = activeMentionPreview.value
  if (!preview) return
  if (preview.kind === 'file') {
    const index = (props.referencedFiles || []).findIndex(
      item => item.relativePath === preview.id || item.id === preview.id
    )
    if (index >= 0) {
      removeReferencedFile(index)
    }
    hideMentionPreview()
    return
  }
  if (preview.kind === 'asset') {
    const index = (props.referencedAssets || []).findIndex(item => item.id === preview.id)
    if (index >= 0) {
      removeReferencedAsset(index)
    }
    hideMentionPreview()
    return
  }
  if (preview.kind === 'message') {
    const index = (props.referencedMessages || []).findIndex(item => item.id === preview.id)
    if (index >= 0) {
      removeReferencedMessage(index)
    }
    hideMentionPreview()
    return
  }
  const index = (props.referencedTraffic || []).findIndex(item => String(item.id) === preview.id)
  if (index >= 0) {
    removeReferencedTraffic(index)
  }
  hideMentionPreview()
}

const setPreviewTrafficSendType = (nextType: TrafficSendType) => {
  const preview = activeMentionPreview.value
  if (!preview || preview.kind !== 'traffic') return
  const nextTraffic = (props.referencedTraffic || []).map(item =>
    String(item.id) === preview.id ? { ...item, sendType: nextType } : item
  )
  emit('sync-traffic-references', nextTraffic)
  hoveredMentionPreview.value = {
    ...hoveredMentionPreview.value!,
    id: preview.id,
    kind: 'traffic',
    x: preview.x,
    y: preview.y,
  }
}

const onCaretChanged = (e: Event) => {
  const target = e.target as HTMLTextAreaElement
  const selectionStart = target.selectionStart || 0
  const selectionEnd = target.selectionEnd || selectionStart
  const normalizedSelection = normalizeSelectionRange(target.value, selectionStart, selectionEnd)
  if (normalizedSelection.start !== selectionStart || normalizedSelection.end !== selectionEnd) {
    target.setSelectionRange(normalizedSelection.start, normalizedSelection.end)
  }
  syncSelectionState(normalizedSelection.start, normalizedSelection.end)
  if (normalizedSelection.start !== normalizedSelection.end) {
    closeMentionPopover()
    closeSlashPopover()
    return
  }
  updateSlashState(target.value, normalizedSelection.start)
  updateMentionState(target.value, normalizedSelection.start)
}
const {
  fileInputRef,
  formatFileSize,
  getAttachmentPreview,
  isDragOver,
  onDragLeave,
  onDragOver,
  onDrop,
  onFilesSelected,
  removeAttachment,
  removeDocument,
  retryUploadDocument,
  runSecurityAnalysis,
  setupNativeDragDrop,
  teardownNativeDragDrop,
  triggerFileSelect,
} = useInputAttachments({
  conversationId: () => props.conversationId ?? null,
  emitAddAttachments: files => {
    emit('add-attachments', files)
  },
  emitAddDocuments: files => {
    emit('add-documents', files)
  },
  emitDocumentProcessed: result => {
    emit('document-processed', result)
  },
  emitRemoveAttachment: index => {
    emit('remove-attachment', index)
  },
  emitRemoveDocument: index => {
    emit('remove-document', index)
  },
})

const {
  addCustomSlashCommand,
  applySlashCommand,
  cancelEditCommand,
  closeSlashManager,
  closeSlashPopover,
  conversationScopeEnabled,
  conversationScopeKey,
  deleteSlashCommand,
  editingSlashId,
  filteredSlashCommands,
  getActionLabel,
  getScopeLabel,
  importSlashInputRef,
  loadSlashCommands,
  managerBuiltinCommands,
  managerCustomCommands,
  newSlashCommand,
  onCommandEnabledChange,
  onDragSortEnd,
  onImportSlashFileChange,
  openSlashManager,
  exportSlashCommands,
  saveEditedSlashCommand,
  showSlashManager,
  slashActiveIndex,
  slashFormError,
  slashManagerScope,
  slashOpen,
  startEditCommand,
  triggerImportSlashCommands,
  updateSlashState,
} = useInputSlashCommands({
  conversationId: computed(() => props.conversationId ?? null),
  createNewConversation: () => {
    createNewConversation()
  },
  clearConversation: () => {
    clearConversation()
  },
  emitOpenToolConfig: () => {
    emit('open-tool-config')
  },
  emitSend: () => {
    emitSend()
  },
  getInputMessage: () => props.inputMessage || '',
  onInputValueChange: value => {
    setInputValue(value)
  },
  toggleRAG: () => {
    toggleRAG()
  },
})

const {
  applyMentionSelection,
  closeMentionPopover,
  filteredMentionItems,
  mentionActiveIndex,
  mentionError,
  mentionLoading,
  mentionOpen,
  syncMentionBindings,
  updateMentionState,
} = useInputMentions({
  getConversationId: () => props.conversationId ?? null,
  getInputMessage: () => props.inputMessage || '',
  getReferencedAssets: () => props.referencedAssets || [],
  getReferencedFiles: () => props.referencedFiles || [],
  getReferencedMessages: () => props.referencedMessages || [],
  getReferencedTraffic: () => props.referencedTraffic || [],
  getConversationMessages: () =>
    (props.availableConversationMessages || [])
      .filter(item => item?.content?.trim())
      .filter(
        item =>
          item.type === 'user' ||
          item.type === 'final' ||
          item.type === 'thinking' ||
          item.type === 'planning'
      )
      .map(item => ({
        content: item.content,
        id: item.id,
        roleLabel: item.type === 'user' ? 'User' : item.type === 'final' ? 'Assistant' : item.type,
        timestamp: item.timestamp,
        type: item.type,
      })),
  onAddReferencedAsset: asset => {
    emit('add-asset-reference', [asset])
  },
  onAddReferencedFile: file => {
    emit('add-file-reference', [file])
  },
  onAddReferencedMessage: message => {
    emit('add-message-reference', [message])
  },
  onAddReferencedTraffic: traffic => {
    emit('add-traffic-reference', [traffic])
  },
  onInputValueChange: (value, cursor) => {
    setInputValue(value, cursor)
  },
  onSyncReferencedAssets: assets => {
    emit('sync-asset-references', assets)
  },
  onSyncReferencedFiles: files => {
    emit('sync-file-references', files)
  },
  onSyncReferencedMessages: messages => {
    emit('sync-message-references', messages)
  },
  onSyncReferencedTraffic: traffic => {
    emit('sync-traffic-references', traffic)
  },
})

// Context usage computed properties
const estimateTokens = (text: string): number => {
  if (!text) return 0
  // Heuristic: CJK chars are closer to 1 token each, others ~4 chars/token
  const cjkCount = (text.match(/[\u4e00-\u9fff]/g) || []).length
  const nonCjk = Math.max(0, text.length - cjkCount)
  const cjkTokens = cjkCount
  const nonCjkTokens = Math.ceil(nonCjk / 4)
  return cjkTokens + nonCjkTokens
}

const inputTokenEstimate = computed(() => estimateTokens(props.inputMessage || ''))
const resolvedDefaultMaxContextTokens = computed(() => {
  const parsed = Number(props.defaultMaxContextTokens)
  if (Number.isFinite(parsed) && parsed > 0) {
    return Math.floor(parsed)
  }
  return 128000
})

const effectiveContextUsage = computed(() => {
  const base = props.contextUsage
  const inputTokens = inputTokenEstimate.value
  if (!base) {
    if (inputTokens === 0) return null
    const maxTokens = resolvedDefaultMaxContextTokens.value
    const usedTokens = inputTokens
    const usagePercentage = maxTokens > 0 ? Math.min(100, (usedTokens / maxTokens) * 100) : 0
    return {
      usedTokens,
      maxTokens,
      usagePercentage,
      systemPromptTokens: 0,
      historyTokens: inputTokens,
      historyCount: 0,
      summaryTokens: 0,
      summaryGlobalTokens: 0,
      summarySegmentTokens: 0,
      summarySegmentCount: 0,
    }
  }
  if (inputTokens === 0) return base
  const usedTokens = base.usedTokens + inputTokens
  const historyTokens = base.historyTokens + inputTokens
  const usagePercentage =
    base.maxTokens > 0 ? Math.min(100, (usedTokens / base.maxTokens) * 100) : 0
  return {
    ...base,
    usedTokens,
    historyTokens,
    usagePercentage,
  }
})

const contextUsagePercentage = computed(() => {
  if (!effectiveContextUsage.value) return 0
  return Math.round(effectiveContextUsage.value.usagePercentage * 10) / 10
})

const contextUsageClass = computed(() => {
  const pressure = effectiveContextUsage.value?.contextPressure
  if (pressure === 'Blocking' || effectiveContextUsage.value?.shouldBlock)
    return 'bg-error/20 text-error border border-error/30'
  if (pressure === 'AutoCompact' || effectiveContextUsage.value?.shouldCompact)
    return 'bg-warning/20 text-warning border border-warning/30'
  if (pressure === 'Warning') return 'bg-info/20 text-info border border-info/30'
  const percentage = contextUsagePercentage.value
  if (percentage >= 90) return 'bg-error/20 text-error border border-error/30'
  if (percentage >= 70) return 'bg-warning/20 text-warning border border-warning/30'
  if (percentage >= 50) return 'bg-info/20 text-info border border-info/30'
  return 'bg-base-300/50 text-base-content/70 border border-base-300'
})

const contextUsageTooltip = computed(() => {
  if (!effectiveContextUsage.value) return ''
  const {
    usedTokens,
    maxTokens,
    systemPromptTokens,
    historyTokens,
    historyCount,
    summaryTokens,
    summaryGlobalTokens,
    summarySegmentTokens,
    summarySegmentCount,
    effectiveContextTokens,
    remainingTokens,
    contextPressure,
    warningThresholdTokens,
    autoCompactThresholdTokens,
    blockingThresholdTokens,
    outputReserveTokens,
    pressurePhase,
    taskTokens,
    sentinelMode,
    sentinelIntentId,
    sentinelIntentConfidence,
    sentinelIntentTransition,
    sentinelParentIntentId,
    sentinelClarificationNeeded,
    sentinelClarificationStatus,
    sentinelCompressionAggressiveness,
    memoryRetrieval,
  } = effectiveContextUsage.value
  const inputHint =
    inputTokenEstimate.value > 0
      ? `\n${t('agent.inputTokens')}: ~${formatTokenCount(inputTokenEstimate.value)}`
      : ''
  const sentinelHint = sentinelMode
    ? `\nSentinel intent: ${sentinelIntentId || '-'}\nSentinel transition: ${sentinelIntentTransition || '-'}\nSentinel parent: ${sentinelParentIntentId || '-'}\nSentinel confidence: ${sentinelIntentConfidence == null ? '-' : sentinelIntentConfidence.toFixed(2)}\nSentinel clarification: ${sentinelClarificationNeeded ? 'needed' : sentinelClarificationStatus || 'stable'}\nSentinel compression: ${sentinelCompressionAggressiveness || '-'}`
    : ''
  const memoryHint = memoryRetrieval
    ? `\nMemory query: ${memoryRetrieval.queryPreview || '-'}\nMemory hits: ${memoryRetrieval.hitCount}/${memoryRetrieval.requestedTopK}\nMemory fallback: ${memoryRetrieval.usedCanonicalFallback ? 'canonical' : 'hybrid'}\nMemory sources: ${formatMemoryTraceBreakdown(memoryRetrieval.sourceBreakdown)}\nMemory kinds: ${formatMemoryTraceBreakdown(memoryRetrieval.kindBreakdown)}`
    : ''
  const pressureHint = contextPressure
    ? `\n${t('agent.contextPressure')}: ${formatContextPressure(contextPressure)}\n${t('agent.effectiveContextTokens')}: ${formatTokenCount(effectiveContextTokens ?? maxTokens)}\n${t('agent.remainingContextTokens')}: ${formatTokenCount(remainingTokens ?? Math.max(0, maxTokens - usedTokens))}\n${t('agent.outputReserveTokens')}: ${formatTokenCount(outputReserveTokens ?? 0)}\n${t('agent.warningThresholdTokens')}: ${formatTokenCount(warningThresholdTokens ?? 0)}\n${t('agent.autoCompactThresholdTokens')}: ${formatTokenCount(autoCompactThresholdTokens ?? 0)}\n${t('agent.blockingThresholdTokens')}: ${formatTokenCount(blockingThresholdTokens ?? 0)}\n${t('agent.contextPressurePhase')}: ${pressurePhase || '-'}`
    : ''
  const taskHint =
    typeof taskTokens === 'number'
      ? `\n${t('agent.currentTaskTokens')}: ${formatTokenCount(taskTokens)}`
      : ''
  return `${t('agent.contextUsageDetails')}
${t('agent.systemPromptTokens')}: ${formatTokenCount(systemPromptTokens)}
${t('agent.summaryTokens')}: ${formatTokenCount(summaryTokens)}
${t('agent.summaryGlobalTokens')}: ${formatTokenCount(summaryGlobalTokens)}
${t('agent.summarySegmentTokens')}: ${formatTokenCount(summarySegmentTokens)} (${t('agent.summarySegments')}: ${summarySegmentCount})
${t('agent.historyTokens')}: ${formatTokenCount(historyTokens)}
${t('agent.historyMessages')}: ${historyCount}
${t('agent.totalUsed')}: ${formatTokenCount(usedTokens)} / ${formatTokenCount(maxTokens)}${taskHint}${inputHint}${pressureHint}${sentinelHint}${memoryHint}`
})

const formatTokenCount = (count: number): string => {
  if (count >= 1000000) {
    return (count / 1000000).toFixed(1) + 'M'
  }
  if (count >= 1000) {
    return (count / 1000).toFixed(1) + 'K'
  }
  return count.toString()
}

const formatContextPressure = (pressure: string): string => {
  if (pressure === 'Low') return t('agent.contextPressureLow')
  if (pressure === 'Warning') return t('agent.contextPressureWarning')
  if (pressure === 'AutoCompact') return t('agent.contextPressureAutoCompact')
  if (pressure === 'Blocking') return t('agent.contextPressureBlocking')
  return pressure
}

const formatMemoryTraceBreakdown = (items: MemoryTraceCount[] | undefined): string => {
  if (!items || items.length === 0) return '-'
  return items.map(item => `${item.label}:${item.count}`).join(', ')
}

// 检查是否可以发送
const canSend = computed(() => {
  if (!props.inputMessage.trim()) return false
  const hasProcessingUploads = (props.pendingDocuments || []).some(d => d.status === 'processing')
  if (hasProcessingUploads) return false
  return true
})

const SEND_EVENT_DEDUP_WINDOW_MS = 300
let lastSendEmitAt = 0
const emitSend = () => {
  const now = Date.now()
  if (now - lastSendEmitAt < SEND_EVENT_DEDUP_WINDOW_MS) {
    return
  }
  if (!canSend.value) return
  if (props.isLoading && !allowTakeover.value) return
  lastSendEmitAt = now
  emit('send-message')
  // 发送后恢复高度
  requestAnimationFrame(() => autoResize())
}

const emitInterruptSend = () => {
  if (!canSend.value) return
  emit('interrupt-message')
  requestAnimationFrame(() => autoResize())
}

const handleStop = () => {
  console.log('InputAreaComponent: 停止按钮被点击')
  emit('stop-execution')
}

// Track IME composition state
const isComposing = ref(false)

const onCompositionStart = () => {
  isComposing.value = true
}

const onCompositionEnd = () => {
  isComposing.value = false
  const el = textareaRef.value
  if (!el) return
  const selectionStart = el.selectionStart || 0
  const selectionEnd = el.selectionEnd || selectionStart
  const normalizedSelection = normalizeSelectionRange(el.value || '', selectionStart, selectionEnd)
  if (normalizedSelection.start !== selectionStart || normalizedSelection.end !== selectionEnd) {
    el.setSelectionRange(normalizedSelection.start, normalizedSelection.end)
  }
  syncSelectionState(normalizedSelection.start, normalizedSelection.end)
}

const onKeydown = (e: KeyboardEvent) => {
  // Ignore Enter key during IME composition
  if (isComposing.value && e.key === 'Enter') {
    return
  }

  if (e.key === 'Backspace' || e.key === 'Delete') {
    const el = textareaRef.value
    const selectionStart = el?.selectionStart || 0
    const selectionEnd = el?.selectionEnd || selectionStart
    const token = findMentionTokenForDeletion(
      props.inputMessage || '',
      selectionStart,
      selectionEnd,
      e.key
    )
    if (token) {
      e.preventDefault()
      const nextValue = removeTextRange(props.inputMessage || '', token.start, token.end)
      setInputValue(nextValue.value, nextValue.cursor)
      return
    }
  }

  if ((e.key === 'ArrowLeft' || e.key === 'ArrowRight') && !e.shiftKey && !e.altKey && !e.metaKey) {
    const el = textareaRef.value
    const selectionStart = el?.selectionStart || 0
    const selectionEnd = el?.selectionEnd || selectionStart
    if (selectionStart === selectionEnd) {
      const direction = e.key === 'ArrowLeft' ? 'left' : 'right'
      const adjacentToken = findMentionTokenAdjacentToCursor(
        props.inputMessage || '',
        selectionStart,
        direction
      )
      if (adjacentToken) {
        e.preventDefault()
        setCursorPosition(direction === 'left' ? adjacentToken.start : adjacentToken.end, direction)
        return
      }
    }
  }

  if (mentionOpen.value) {
    if (e.key === 'ArrowDown') {
      e.preventDefault()
      if (filteredMentionItems.value.length > 0) {
        mentionActiveIndex.value =
          (mentionActiveIndex.value + 1) % filteredMentionItems.value.length
      }
      return
    }
    if (e.key === 'ArrowUp') {
      e.preventDefault()
      if (filteredMentionItems.value.length > 0) {
        mentionActiveIndex.value =
          (mentionActiveIndex.value - 1 + filteredMentionItems.value.length) %
          filteredMentionItems.value.length
      }
      return
    }
    if ((e.key === 'Enter' || e.key === 'Tab') && filteredMentionItems.value.length > 0) {
      e.preventDefault()
      void applyMentionSelection()
      return
    }
    if (e.key === 'Escape') {
      e.preventDefault()
      closeMentionPopover()
      return
    }
  }

  if (slashOpen.value) {
    if (e.key === 'ArrowDown') {
      e.preventDefault()
      if (filteredSlashCommands.value.length > 0) {
        slashActiveIndex.value = (slashActiveIndex.value + 1) % filteredSlashCommands.value.length
      }
      return
    }
    if (e.key === 'ArrowUp') {
      e.preventDefault()
      if (filteredSlashCommands.value.length > 0) {
        slashActiveIndex.value =
          (slashActiveIndex.value - 1 + filteredSlashCommands.value.length) %
          filteredSlashCommands.value.length
      }
      return
    }
    if ((e.key === 'Enter' || e.key === 'Tab') && filteredSlashCommands.value.length > 0) {
      e.preventDefault()
      const cmd = filteredSlashCommands.value[Math.max(0, slashActiveIndex.value)]
      if (cmd) applySlashCommand(cmd)
      return
    }
    if (e.key === 'Escape') {
      e.preventDefault()
      closeSlashPopover()
      return
    }
  }

  if (e.key === 'Enter' && !e.shiftKey) {
    e.preventDefault()
    emitSend()
  } else if (e.key === 'Enter' && e.shiftKey) {
    // allow newline
    return
  } else if (e.key === 'Escape') {
    // blur
    ;(e.target as HTMLTextAreaElement).blur()
  }
}

const clearCurrent = () => {
  emit('update:input-message', '')
  requestAnimationFrame(() => autoResize())
}

const createNewConversation = () => {
  emit('create-new-conversation')
  // 清空输入框
  emit('update:input-message', '')
  requestAnimationFrame(() => autoResize())
}

const clearConversation = () => {
  emit('clear-conversation')
  // 清空输入框
  emit('update:input-message', '')
  requestAnimationFrame(() => autoResize())
}

const toggleRAG = () => {
  emit('toggle-rag', !effectiveRagEnabled.value)
}

const toggleWebSearch = () => {
  emit('toggle-web-search', !effectiveWebSearchEnabled.value)
}

const onAgentChanged = (value: string) => {
  if (!value) return
  emit('change-agent', value)
}

// 点击外部区域关闭弹层
const handleClickOutside = (e: MouseEvent) => {
  const target = e.target as Node | null
  const container = containerRef.value
  if (!target || !container) return
  if (!container.contains(target)) {
    closeSlashPopover()
    closeMentionPopover()
  }
}

const removeReferencedFile = (index: number) => {
  const file = props.referencedFiles?.[index]
  if (file?.mentionText) {
    setInputValue(removeMentionTokenText(props.inputMessage || '', file.mentionText))
  }
  emit('remove-file', index)
}

const clearReferencedFiles = () => {
  let nextValue = props.inputMessage || ''
  for (const file of props.referencedFiles || []) {
    nextValue = removeMentionTokenText(nextValue, file.mentionText)
  }
  setInputValue(nextValue)
  emit('clear-files')
}

const removeReferencedTraffic = (index: number) => {
  const traffic = props.referencedTraffic?.[index]
  if (traffic?.mentionText) {
    setInputValue(removeMentionTokenText(props.inputMessage || '', traffic.mentionText))
  }
  emit('remove-traffic', index)
}

const removeReferencedMessage = (index: number) => {
  const message = props.referencedMessages?.[index]
  if (message?.mentionText) {
    setInputValue(removeMentionTokenText(props.inputMessage || '', message.mentionText))
  }
  emit('remove-message', index)
}

const clearReferencedMessages = () => {
  let nextValue = props.inputMessage || ''
  for (const message of props.referencedMessages || []) {
    nextValue = removeMentionTokenText(nextValue, message.mentionText)
  }
  setInputValue(nextValue)
  emit('clear-messages')
}

const clearReferencedTraffic = () => {
  let nextValue = props.inputMessage || ''
  for (const traffic of props.referencedTraffic || []) {
    nextValue = removeMentionTokenText(nextValue, traffic.mentionText)
  }
  setInputValue(nextValue)
  emit('clear-traffic')
}

const removeReferencedAsset = (index: number) => {
  const asset = props.referencedAssets?.[index]
  if (asset?.mentionText) {
    setInputValue(removeMentionTokenText(props.inputMessage || '', asset.mentionText))
  }
  emit('remove-asset', index)
}

const clearReferencedAssets = () => {
  let nextValue = props.inputMessage || ''
  for (const asset of props.referencedAssets || []) {
    nextValue = removeMentionTokenText(nextValue, asset.mentionText)
  }
  setInputValue(nextValue)
  emit('clear-assets')
}

// 聚焦输入框
const focusInput = () => {
  nextTick(() => {
    const el = textareaRef.value
    if (!el) return
    el.focus()
    syncSelectionState(el.selectionStart || 0, el.selectionEnd || el.selectionStart || 0)
  })
}

onMounted(async () => {
  autoResize()
  await loadSlashCommands()
  window.addEventListener('click', handleClickOutside, true)

  // 设置 Tauri 拖放监听
  await setupNativeDragDrop()

  // 自动聚焦输入框
  focusInput()
})

onUnmounted(() => {
  window.removeEventListener('click', handleClickOutside, true)
  teardownNativeDragDrop()
  cancelHideMentionPreview()
})

watch(
  () => props.inputMessage,
  val => {
    syncMentionBindings(val || '')
    if (!val || !val.includes('/')) {
      closeSlashPopover()
    }
    if (!val || !val.includes('@')) {
      closeMentionPopover()
      return
    }
    const el = textareaRef.value
    updateMentionState(val, el?.selectionStart || val.length)
  }
)

// 暴露方法供父组件调用
defineExpose({
  focusInput,
})

// End script
</script>

<style scoped>
.input-area-container {
  /* Ensure input area doesn't overlap sidebar */
  width: 100%;
  max-width: 100%;
}

.chat-input {
  position: relative;
}

.input-editor-shell {
  position: relative;
}

.input-textarea,
.textarea-mirror-content {
  font: inherit;
  line-height: 1.625;
  letter-spacing: 0;
  tab-size: 2;
  white-space: pre-wrap;
  word-break: break-word;
}

.input-textarea {
  position: relative;
  z-index: 1;
  caret-color: hsl(var(--bc));
}

.textarea-mirror {
  position: absolute;
  inset: 0;
  overflow: hidden;
  pointer-events: none;
  z-index: 2;
}

.textarea-mirror-content {
  margin: 0;
  color: hsl(var(--bc) / 0.85);
}

.textarea-mirror-caret {
  display: inline-block;
  width: 2px;
  min-width: 2px;
  height: 1.15em;
  margin-left: -1px;
  margin-right: -1px;
  background: hsl(var(--bc));
  border-radius: 999px;
  box-shadow: 0 0 0 1px hsl(var(--b1) / 0.35);
  vertical-align: -0.15em;
  pointer-events: none;
  animation: textarea-mirror-caret-blink 1s steps(1, end) infinite;
}

.mention-inline-token {
  display: inline;
  border-radius: 4px;
  padding: 0 2px;
  font-weight: 500;
  pointer-events: auto;
}

.mention-inline-file {
  background: hsl(var(--s) / 0.14);
  color: hsl(var(--s));
}

.mention-inline-asset {
  background: hsl(var(--p) / 0.14);
  color: hsl(var(--p));
}

.mention-inline-traffic {
  background: hsl(var(--a) / 0.14);
  color: hsl(var(--a));
}

.mention-preview-popover {
  position: absolute;
  z-index: 30;
  max-width: min(32rem, calc(100% - 1rem));
  transform: translateY(-100%);
  border: 1px solid hsl(var(--b3) / 0.9);
  border-radius: 8px;
  background: hsl(var(--b1) / 0.98);
  box-shadow: 0 10px 30px rgba(0, 0, 0, 0.18);
  padding: 0.5rem 0.625rem;
  pointer-events: auto;
}

.mention-preview-title {
  font-size: 11px;
  font-weight: 700;
  color: hsl(var(--bc) / 0.75);
  margin-bottom: 0.25rem;
}

.mention-preview-body {
  font-size: 12px;
  line-height: 1.5;
  color: hsl(var(--bc) / 0.88);
  white-space: pre-wrap;
  word-break: break-word;
}

.mention-preview-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 0.375rem;
  margin-top: 0.5rem;
}

.mention-preview-btn {
  border: 1px solid hsl(var(--b3) / 0.9);
  background: hsl(var(--b2) / 0.7);
  color: hsl(var(--bc) / 0.82);
  border-radius: 6px;
  padding: 0.2rem 0.45rem;
  font-size: 11px;
  line-height: 1.2;
  transition:
    background-color 0.15s ease,
    color 0.15s ease,
    border-color 0.15s ease;
}

.mention-preview-btn:hover {
  background: hsl(var(--b3) / 0.85);
}

.mention-preview-btn-active {
  background: hsl(var(--p) / 0.16);
  border-color: hsl(var(--p) / 0.35);
  color: hsl(var(--p));
}

.slash-popover {
  position: absolute;
  left: 0.5rem;
  right: 0.5rem;
  bottom: calc(100% + 6px);
  z-index: 120;
  backdrop-filter: blur(8px);
}

.slash-manager-modal {
  z-index: 1300;
}

.slash-manager-modal .modal-box {
  max-height: calc(100vh - 3rem);
}

.icon-btn {
  width: 1.75rem;
  height: 1.75rem;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 0.375rem;
  font-size: calc(var(--font-size-base, 14px) * 0.75);
  transition:
    background-color 0.15s,
    color 0.15s;
}

.icon-btn:hover {
  background-color: hsl(var(--b3) / 0.7);
}

.icon-btn.active {
  background: hsl(var(--p));
  color: hsl(var(--pc));
  box-shadow: 0 2px 4px rgba(0, 0, 0, 0.15);
}

.assistant-agent-switch {
  min-width: 6rem;
  max-width: 14rem;
  flex-shrink: 1;
}

.assistant-agent-switch :deep(.input) {
  border-radius: 0.5rem;
  font-size: 0.75rem;
}

@media (max-width: 1024px) {
  .assistant-agent-switch {
    max-width: 10rem;
  }
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
  transition:
    background-color 0.15s,
    color 0.15s;
}

.send-btn:hover {
  background: hsl(var(--p));
  color: hsl(var(--pc));
}

.send-btn:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

/* Search popover positioned above the toolbar */
.search-popover {
  position: absolute;
  bottom: calc(100% + 8px);
  left: 0;
  width: 24rem;
  max-width: 90vw;
  z-index: 50;
}

@media (max-width: 640px) {
  .search-popover {
    width: 18rem;
  }
}

/* Drag and drop styles */
.input-area-container.drag-over {
  position: relative;
}

.drag-overlay {
  position: absolute;
  inset: 0;
  background: hsl(var(--p) / 0.1);
  border: 2px dashed hsl(var(--p));
  border-radius: 0.5rem;
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 100;
  backdrop-filter: blur(4px);
}

.drag-content {
  display: flex;
  flex-direction: column;
  align-items: center;
  color: hsl(var(--p));
  text-align: center;
  padding: 1rem;
}

/* Context usage indicator */
.context-usage-indicator {
  font-variant-numeric: tabular-nums;
  white-space: nowrap;
  transition: all 0.2s ease;
}

.context-usage-indicator:hover {
  opacity: 0.9;
}

@keyframes textarea-mirror-caret-blink {
  0%,
  49% {
    opacity: 1;
  }

  50%,
  100% {
    opacity: 0;
  }
}
</style>
