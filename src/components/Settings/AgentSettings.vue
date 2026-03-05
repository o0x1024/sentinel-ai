<template>
  <div class="agent-settings">
    <div v-if="loading" class="flex justify-center items-center py-12">
      <span class="loading loading-spinner loading-lg"></span>
    </div>

    <template v-else>
      <!-- Terminal Settings Section -->
      <div class="card bg-base-100 shadow-sm mb-6">
        <div class="card-body">
          <h3 class="card-title mb-4">
            <i class="fas fa-terminal"></i>
            {{ t('settings.agent.terminal.title') }}
          </h3>

          <!-- Docker Configuration -->
          <div class="mb-6 p-4 bg-base-200 rounded-lg">
            <h4 class="font-semibold mb-4">
              <i class="fab fa-docker mr-2"></i>
              {{ t('settings.agent.terminal.dockerImage') }}
            </h4>
            
            <!-- Execution Mode Toggle -->
            <div class="grid grid-cols-1 md:grid-cols-2 gap-4 mb-4">
              <div class="form-control">
                <label class="label cursor-pointer justify-start gap-4">
                  <input 
                    type="checkbox" 
                    class="toggle toggle-primary" 
                    :checked="terminalConfig.default_execution_mode === 'docker'"
                    @change="toggleExecutionMode"
                  />
                  <div>
                    <span class="label-text font-medium">{{ t('settings.agent.terminal.useDocker') }}</span>
                    <p class="text-xs text-base-content/60 mt-1">
                      {{ t('settings.agent.terminal.useDockerDesc') }}
                    </p>
                  </div>
                </label>
              </div>

              <div class="form-control">
                <label class="label cursor-pointer justify-start gap-4">
                  <input
                    type="checkbox"
                    class="toggle toggle-primary"
                    :checked="terminalConfig.docker_use_host_network"
                    :disabled="terminalConfig.default_execution_mode !== 'docker'"
                    @change="toggleDockerHostNetwork"
                  />
                  <div>
                    <span class="label-text font-medium">{{ t('settings.agent.terminal.useHostNetwork') }}</span>
                    <p class="text-xs text-base-content/60 mt-1">
                      {{ t('settings.agent.terminal.useHostNetworkDesc') }}
                    </p>
                  </div>
                </label>
              </div>
            </div>

            <!-- Docker Image Input -->
            <div v-if="terminalConfig.default_execution_mode === 'docker'" class="form-control">
              <label class="label">
                <span class="label-text">{{ t('settings.agent.terminal.dockerImage') }}</span>
              </label>
              <input 
                type="text" 
                :value="terminalConfig.docker_image"
                @input="updateDockerImage"
                :placeholder="t('settings.agent.terminal.dockerImagePlaceholder')"
                class="input input-bordered w-full font-mono"
              />
              <label class="label">
                <span class="label-text-alt text-base-content/60">
                  {{ t('settings.agent.terminal.dockerImageDesc') }}
                </span>
              </label>
            </div>

            <!-- Docker Resource Limits -->
            <div v-if="terminalConfig.default_execution_mode === 'docker'" class="grid grid-cols-2 gap-4 mt-4">
              <!-- Memory Limit -->
              <div class="form-control">
                <label class="label">
                  <span class="label-text">{{ t('settings.agent.terminal.memoryLimit') }}</span>
                </label>
                <input 
                  type="text" 
                  :value="terminalConfig.docker_memory_limit"
                  @input="updateDockerMemoryLimit"
                  :placeholder="t('settings.agent.terminal.memoryLimitPlaceholder')"
                  class="input input-bordered w-full font-mono"
                />
                <label class="label">
                  <span class="label-text-alt text-base-content/60">
                    {{ t('settings.agent.terminal.memoryLimitDesc') }}
                  </span>
                </label>
              </div>

              <!-- CPU Limit -->
              <div class="form-control">
                <label class="label">
                  <span class="label-text">{{ t('settings.agent.terminal.cpuLimit') }}</span>
                </label>
                <input 
                  type="text" 
                  :value="terminalConfig.docker_cpu_limit"
                  @input="updateDockerCpuLimit"
                  :placeholder="t('settings.agent.terminal.cpuLimitPlaceholder')"
                  class="input input-bordered w-full font-mono"
                />
                <label class="label">
                  <span class="label-text-alt text-base-content/60">
                    {{ t('settings.agent.terminal.cpuLimitDesc') }}
                  </span>
                </label>
              </div>
            </div>
          </div>

          <!-- Terminal Command Auto Execution -->
          <div class="mb-6">
            <h4 class="font-semibold border-b pb-2 mb-4">{{ t('settings.agent.terminal.autoExecution') }}</h4>
            <p class="text-sm text-base-content/70 mb-4">
              {{ t('settings.agent.terminal.autoExecutionDesc') }}
            </p>
            <div class="flex gap-6">
              <div class="form-control">
                <label class="label cursor-pointer gap-3">
                  <input 
                    type="radio" 
                    name="policy" 
                    class="radio radio-primary" 
                    :checked="shellConfig.default_policy === 'AlwaysProceed'"
                    @change="setDefaultPolicy('AlwaysProceed')"
                  />
                  <span class="label-text">{{ t('settings.agent.terminal.alwaysProceed') }}</span>
                </label>
              </div>
              <div class="form-control">
                <label class="label cursor-pointer gap-3">
                  <input 
                    type="radio" 
                    name="policy" 
                    class="radio radio-primary" 
                    :checked="shellConfig.default_policy === 'RequestReview'"
                    @change="setDefaultPolicy('RequestReview')"
                  />
                  <span class="label-text">{{ t('settings.agent.terminal.requestReview') }}</span>
                </label>
              </div>
            </div>
          </div>

          <div class="grid grid-cols-1 lg:grid-cols-2 gap-6">
            <!-- Allow List Terminal Commands -->
            <div class="space-y-4">
              <div class="flex items-center justify-between">
                <h4 class="font-semibold">{{ t('settings.agent.terminal.allowList') }}</h4>
                <span class="badge badge-sm badge-ghost">{{ shellConfig.allowed_commands.length }}</span>
              </div>
              <p class="text-sm text-base-content/70">
                {{ t('settings.agent.terminal.allowListDesc') }}
              </p>
              
              <!-- Add new command input -->
              <div class="flex gap-2">
                <input 
                  v-model="newAllowCommand"
                  type="text" 
                  :placeholder="t('settings.agent.terminal.enterCommand')"
                  class="input input-bordered input-sm flex-1 font-mono"
                  @keyup.enter="addAllowCommand"
                />
                <button 
                  @click="addAllowCommand" 
                  class="btn btn-sm btn-primary"
                  :disabled="!newAllowCommand.trim()"
                >
                  <i class="fas fa-plus"></i>
                </button>
              </div>

              <!-- Virtual scrollable command list -->
              <div 
                class="border border-base-300 rounded-lg overflow-hidden"
                :class="{ 'bg-base-200/30': shellConfig.allowed_commands.length === 0 }"
              >
                <div 
                  v-if="shellConfig.allowed_commands.length > 0"
                  class="virtual-list-container"
                  style="height: 240px; overflow-y: auto;"
                >
                  <div 
                    v-for="(cmd, index) in shellConfig.allowed_commands" 
                    :key="index"
                    class="flex items-center justify-between px-3 py-2 hover:bg-base-200 border-b border-base-300 last:border-b-0 font-mono text-sm transition-colors"
                  >
                    <span class="truncate flex-1">{{ cmd }}</span>
                    <button 
                      @click="removeAllowCommand(index)"
                      class="btn btn-ghost btn-xs text-base-content/50 hover:text-error ml-2 flex-shrink-0"
                    >
                      <i class="fas fa-times"></i>
                    </button>
                  </div>
                </div>
                <div v-else class="text-center py-8 text-base-content/50 text-sm">
                  {{ t('settings.agent.terminal.noAllowedCommands') }}
                </div>
              </div>
            </div>

            <!-- Deny List Section -->
            <div class="space-y-4">
              <div class="flex items-center justify-between">
                <h4 class="font-semibold">{{ t('settings.agent.terminal.denyList') }}</h4>
                <span class="badge badge-sm badge-ghost">{{ shellConfig.denied_commands.length }}</span>
              </div>
              <p class="text-sm text-base-content/70">
                {{ t('settings.agent.terminal.denyListDesc') }}
              </p>

              <!-- Add new command input -->
              <div class="flex gap-2">
                <input 
                  v-model="newDenyCommand"
                  type="text" 
                  :placeholder="t('settings.agent.terminal.enterCommand')"
                  class="input input-bordered input-sm flex-1 font-mono"
                  @keyup.enter="addDenyCommand"
                />
                <button 
                  @click="addDenyCommand" 
                  class="btn btn-sm btn-primary"
                  :disabled="!newDenyCommand.trim()"
                >
                  <i class="fas fa-plus"></i>
                </button>
              </div>

              <!-- Virtual scrollable command list -->
              <div 
                class="border border-base-300 rounded-lg overflow-hidden"
                :class="{ 'bg-base-200/30': shellConfig.denied_commands.length === 0 }"
              >
                <div 
                  v-if="shellConfig.denied_commands.length > 0"
                  class="virtual-list-container"
                  style="height: 240px; overflow-y: auto;"
                >
                  <div 
                    v-for="(cmd, index) in shellConfig.denied_commands" 
                    :key="index"
                    class="flex items-center justify-between px-3 py-2 hover:bg-base-200 border-b border-base-300 last:border-b-0 font-mono text-sm transition-colors"
                  >
                    <span class="truncate flex-1">{{ cmd }}</span>
                    <button 
                      @click="removeDenyCommand(index)"
                      class="btn btn-ghost btn-xs text-base-content/50 hover:text-error ml-2 flex-shrink-0"
                    >
                      <i class="fas fa-times"></i>
                    </button>
                  </div>
                </div>
                <div v-else class="text-center py-8 text-base-content/50 text-sm">
                  {{ t('settings.agent.terminal.noDeniedCommands') }}
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- Working Directory Section -->
      <div class="card bg-base-100 shadow-sm mb-6">
        <div class="card-body">
          <h3 class="card-title mb-4">
            <i class="fas fa-folder-open"></i>
            {{ t('settings.ai.workingDirectory') }}
          </h3>
          <div class="form-control">
            <label class="label">
              <span class="label-text">{{ t('settings.ai.workingDirectory') }}</span>
            </label>
            <div class="input-group">
              <input
                v-model="workingDirectory"
                type="text"
                class="input input-bordered flex-1"
                :placeholder="t('settings.ai.workingDirectoryPlaceholder')"
              />
              <button class="btn btn-outline" @click="selectWorkingDirectory">
                <i class="fas fa-folder-open mr-1"></i>
                {{ t('settings.ai.selectDirectory') }}
              </button>
              <button class="btn btn-primary" @click="saveWorkingDirectory">
                <i class="fas fa-save mr-1"></i>
                {{ t('settings.ai.save') }}
              </button>
            </div>
            <label class="label">
              <span class="label-text-alt">{{ t('settings.ai.workingDirectoryHint') }}</span>
            </label>
          </div>
        </div>
      </div>

      <!-- ExploitDB Sync Section -->
      <div class="card bg-base-100 shadow-sm mb-6">
        <div class="card-body">
          <h3 class="card-title mb-4">
            <i class="fas fa-bug"></i>
            {{ t('settings.agent.exploitdb.title') }}
          </h3>
          <p class="text-sm text-base-content/70 mb-4">
            {{ t('settings.agent.exploitdb.desc') }}
          </p>

          <div class="grid grid-cols-1 gap-4">
            <label class="form-control">
              <span class="label-text">{{ t('settings.agent.exploitdb.repoUrl') }}</span>
              <input
                v-model="exploitDbSettings.repo_url"
                type="text"
                class="input input-bordered font-mono"
                placeholder="https://gitlab.com/exploit-database/exploitdb"
              />
            </label>

            <label class="form-control">
              <span class="label-text">{{ t('settings.agent.exploitdb.repoPath') }}</span>
              <div class="join w-full">
                <input
                  v-model="exploitDbSettings.repo_path"
                  type="text"
                  class="input input-bordered join-item flex-1 font-mono"
                />
                <button class="btn btn-outline join-item" @click="selectExploitDbPath">
                  <i class="fas fa-folder-open mr-1"></i>
                  {{ t('settings.agent.exploitdb.selectPath') }}
                </button>
              </div>
            </label>
          </div>

          <div class="flex flex-wrap gap-2 mt-4">
            <button class="btn btn-sm btn-outline" @click="saveExploitDbSettings" :disabled="exploitDbLoading || exploitDbSyncing">
              <i class="fas fa-save mr-1"></i>
              {{ t('settings.agent.exploitdb.save') }}
            </button>
            <button class="btn btn-sm btn-primary" @click="syncExploitDb" :disabled="exploitDbSyncing || exploitDbLoading">
              <span v-if="exploitDbSyncing" class="loading loading-spinner loading-xs mr-2"></span>
              <i v-else class="fas fa-sync-alt mr-1"></i>
              {{ t('settings.agent.exploitdb.syncNow') }}
            </button>
            <button class="btn btn-sm btn-ghost" @click="loadExploitDbStatus" :disabled="exploitDbLoading || exploitDbSyncing">
              <i class="fas fa-rotate mr-1"></i>
              {{ t('settings.agent.exploitdb.refreshStatus') }}
            </button>
          </div>

          <div class="mt-4 text-sm bg-base-200 rounded-lg p-3 space-y-1">
            <div><span class="opacity-70">{{ t('settings.agent.exploitdb.repoReady') }}:</span> <span class="font-mono">{{ exploitDbStatus.repo_exists ? 'yes' : 'no' }}</span></div>
            <div><span class="opacity-70">{{ t('settings.agent.exploitdb.indexReady') }}:</span> <span class="font-mono">{{ exploitDbStatus.index_exists ? 'yes' : 'no' }}</span></div>
            <div><span class="opacity-70">{{ t('settings.agent.exploitdb.indexedEntries') }}:</span> <span class="font-mono">{{ exploitDbStatus.indexed_entries }}</span></div>
            <div><span class="opacity-70">{{ t('settings.agent.exploitdb.lastCommit') }}:</span> <span class="font-mono break-all">{{ exploitDbStatus.last_commit || '-' }}</span></div>
            <div><span class="opacity-70">{{ t('settings.agent.exploitdb.lastSync') }}:</span> <span class="font-mono">{{ exploitDbStatus.last_sync_at || '-' }}</span></div>
            <div><span class="opacity-70">{{ t('settings.agent.exploitdb.indexedAt') }}:</span> <span class="font-mono">{{ exploitDbStatus.indexed_at || '-' }}</span></div>
          </div>
        </div>
      </div>

      <!-- Image Attachments Section -->
      <div class="card bg-base-100 shadow-sm mb-6">
        <div class="card-body">
          <h3 class="card-title mb-4">
            <i class="fas fa-image"></i>
            {{ t('settings.agent.imageAttachments.title') }}
          </h3>

          <div class="mb-6">
            <h4 class="font-semibold border-b pb-2 mb-4">{{ t('settings.agent.imageAttachments.mode') }}</h4>
            <p class="text-sm text-base-content/70 mb-4">
              {{ t('settings.agent.imageAttachments.modeDesc') }}
            </p>
            <div class="flex gap-6 flex-wrap">
              <div class="form-control">
                <label class="label cursor-pointer gap-3">
                  <input
                    type="radio"
                    name="imageAttachmentMode"
                    class="radio radio-primary"
                    :checked="imageAttachments.mode === 'local_ocr'"
                    @change="setImageMode('local_ocr')"
                  />
                  <span class="label-text">{{ t('settings.agent.imageAttachments.localOcr') }}</span>
                </label>
              </div>
              <div class="form-control">
                <label class="label cursor-pointer gap-3">
                  <input
                    type="radio"
                    name="imageAttachmentMode"
                    class="radio radio-primary"
                    :checked="imageAttachments.mode === 'model_vision'"
                    :disabled="!imageAttachments.allow_upload_to_model"
                    @change="setImageMode('model_vision')"
                  />
                  <span class="label-text">{{ t('settings.agent.imageAttachments.modelVision') }}</span>
                </label>
              </div>
            </div>
          </div>

          <div class="form-control mb-2">
            <label class="label cursor-pointer justify-start gap-4">
              <input
                type="checkbox"
                class="toggle toggle-primary"
                :checked="imageAttachments.allow_upload_to_model"
                @change="toggleAllowUploadToModel"
              />
              <div>
                <span class="label-text font-medium">{{ t('settings.agent.imageAttachments.allowUpload') }}</span>
                <p class="text-xs text-base-content/60 mt-1">
                  {{ t('settings.agent.imageAttachments.allowUploadDesc') }}
                </p>
              </div>
            </label>
          </div>

          <div
            v-if="imageAttachments.mode === 'model_vision' && !imageAttachments.allow_upload_to_model"
            class="alert alert-warning text-xs"
          >
            <i class="fas fa-exclamation-triangle"></i>
            <span>{{ t('settings.agent.imageAttachments.uploadDisabledWarning') }}</span>
          </div>
        </div>
      </div>

      <!-- Subagent Settings Section -->
      <div class="card bg-base-100 shadow-sm mb-6">
        <div class="card-body">
          <h3 class="card-title mb-4">
            <i class="fas fa-folder-tree"></i>
            {{ t('settings.agent.fileUploads.title') }}
          </h3>
          <p class="text-sm text-base-content/70 mb-4">
            {{ t('settings.agent.fileUploads.desc') }}
          </p>

          <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-3 mb-4">
            <label class="form-control">
              <span class="label-text text-xs">{{ t('settings.agent.fileUploads.maxFileMb') }}</span>
              <input type="number" min="1" class="input input-bordered input-sm" v-model.number="uploadSettings.max_file_mb" @change="saveUploadSettings" />
            </label>
            <label class="form-control">
              <span class="label-text text-xs">{{ t('settings.agent.fileUploads.maxTotalMb') }}</span>
              <input type="number" min="1" class="input input-bordered input-sm" v-model.number="uploadSettings.max_total_mb" @change="saveUploadSettings" />
            </label>
            <label class="form-control">
              <span class="label-text text-xs">{{ t('settings.agent.fileUploads.maxPerConversation') }}</span>
              <input type="number" min="1" class="input input-bordered input-sm" v-model.number="uploadSettings.max_files_per_conversation" @change="saveUploadSettings" />
            </label>
          </div>

          <div class="flex items-center gap-3 mb-4">
            <label class="label cursor-pointer gap-2 py-0">
              <input type="checkbox" class="toggle toggle-sm" v-model="uploadSettings.auto_cleanup_enabled" @change="saveUploadSettings" />
              <span class="label-text text-sm">{{ t('settings.agent.fileUploads.autoCleanup') }}</span>
            </label>
            <label class="form-control max-w-[180px]">
              <span class="label-text text-xs">{{ t('settings.agent.fileUploads.retentionDays') }}</span>
              <input type="number" min="1" class="input input-bordered input-sm" v-model.number="uploadSettings.retention_days" :disabled="!uploadSettings.auto_cleanup_enabled" @change="saveUploadSettings" />
            </label>
          </div>

          <div class="flex flex-wrap items-center gap-2 mb-4">
            <button class="btn btn-sm btn-outline" @click="loadUploadedFiles" :disabled="uploadsLoading">
              <i class="fas fa-sync-alt"></i>
              {{ t('common.refresh') }}
            </button>
            <button class="btn btn-sm btn-error btn-outline" @click="clearAllUploadedFiles" :disabled="uploadsLoading || uploadedFiles.length === 0">
              <i class="fas fa-trash"></i>
              {{ t('settings.agent.fileUploads.clearAll') }}
            </button>
            <div class="ml-auto text-xs text-base-content/60">
              {{ t('settings.agent.fileUploads.totalFiles', { count: uploadedFiles.length }) }}
            </div>
          </div>

          <div class="form-control mb-3 max-w-xs">
            <label class="label py-1">
              <span class="label-text">{{ t('settings.agent.fileUploads.dateFilter') }}</span>
            </label>
            <select class="select select-bordered select-sm" v-model="selectedUploadDate">
              <option value="">{{ t('settings.agent.fileUploads.allDates') }}</option>
              <option v-for="date in uploadDates" :key="date" :value="date">{{ date }}</option>
            </select>
          </div>

          <div class="form-control mb-3 max-w-lg">
            <label class="label py-1">
              <span class="label-text">{{ t('settings.agent.fileUploads.conversationFilter') }}</span>
            </label>
            <input
              v-model="conversationFilter"
              type="text"
              class="input input-bordered input-sm"
              :placeholder="t('settings.agent.fileUploads.conversationPlaceholder')"
            />
          </div>

          <div v-if="uploadsLoading" class="flex justify-center py-6">
            <span class="loading loading-spinner loading-sm"></span>
          </div>
          <div v-else-if="filteredUploadedFiles.length === 0" class="text-sm text-base-content/60 py-4">
            {{ t('settings.agent.fileUploads.empty') }}
          </div>
          <div v-else class="space-y-2">
            <div
              v-for="item in filteredUploadedFiles"
              :key="item.path"
              class="flex items-center gap-2 p-2 rounded border border-base-300 bg-base-200 text-sm"
            >
              <span class="badge badge-sm badge-ghost">{{ item.date }}</span>
              <span class="font-mono text-xs truncate flex-1" :title="item.path">{{ item.filename }}</span>
              <span class="text-xs text-base-content/60">{{ formatBytes(item.size) }}</span>
            </div>
          </div>

          <div class="mt-4 flex items-center gap-2" v-if="selectedUploadDate">
            <button class="btn btn-xs btn-error btn-outline" @click="clearSelectedDateFiles" :disabled="uploadsLoading">
              <i class="fas fa-trash"></i>
              {{ t('settings.agent.fileUploads.clearDate', { date: selectedUploadDate }) }}
            </button>
          </div>
        </div>
      </div>

      <!-- Subagent Settings Section -->
      <div class="card bg-base-100 shadow-sm mb-6">
        <div class="card-body">
          <h3 class="card-title mb-4">
            <i class="fas fa-users-cog"></i>
            {{ t('settings.agent.subagent.title') }}
          </h3>

          <div class="form-control">
            <label class="label">
              <span class="label-text">{{ t('settings.agent.subagent.timeout') }}</span>
            </label>
            <div class="join">
              <input 
                type="number" 
                :value="subagentConfig.timeout_secs"
                @input="updateSubagentTimeout"
                min="60"
                max="7200"
                step="60"
                class="input input-bordered join-item w-32 font-mono"
              />
              <span class="btn btn-disabled join-item">{{ t('settings.agent.subagent.seconds') }}</span>
            </div>
            <label class="label">
              <span class="label-text-alt text-base-content/60">
                {{ t('settings.agent.subagent.timeoutDesc') }}
              </span>
            </label>
            <div class="flex gap-2 mt-2">
              <button 
                class="btn btn-xs btn-outline"
                :class="{ 'btn-primary': subagentConfig.timeout_secs === 300 }"
                @click="subagentConfig.timeout_secs = 300; autoSaveConfig()"
              >5 min</button>
              <button 
                class="btn btn-xs btn-outline"
                :class="{ 'btn-primary': subagentConfig.timeout_secs === 600 }"
                @click="subagentConfig.timeout_secs = 600; autoSaveConfig()"
              >10 min</button>
              <button 
                class="btn btn-xs btn-outline"
                :class="{ 'btn-primary': subagentConfig.timeout_secs === 1800 }"
                @click="subagentConfig.timeout_secs = 1800; autoSaveConfig()"
              >30 min</button>
              <button 
                class="btn btn-xs btn-outline"
                :class="{ 'btn-primary': subagentConfig.timeout_secs === 3600 }"
                @click="subagentConfig.timeout_secs = 3600; autoSaveConfig()"
              >1 hour</button>
            </div>
          </div>
        </div>
      </div>

      <!-- Completion Guard Settings Section -->
      <div class="card bg-base-100 shadow-sm mb-6">
        <div class="card-body">
          <h3 class="card-title mb-4">
            <i class="fas fa-shield-alt"></i>
            {{ t('settings.agent.completionGuard.title') }}
          </h3>
          <p class="text-sm text-base-content/70 mb-4">
            {{ t('settings.agent.completionGuard.desc') }}
          </p>

          <div class="grid grid-cols-1 md:grid-cols-2 gap-4 mb-4">
            <div class="form-control">
              <label class="label cursor-pointer justify-start gap-4">
                <input
                  type="checkbox"
                  class="toggle toggle-primary"
                  :checked="completionGuardConfig.enabled"
                  @change="toggleCompletionGuardEnabled"
                />
                <div>
                  <span class="label-text font-medium">{{ t('settings.agent.completionGuard.enabled') }}</span>
                  <p class="text-xs text-base-content/60 mt-1">
                    {{ t('settings.agent.completionGuard.enabledDesc') }}
                  </p>
                </div>
              </label>
            </div>

            <div class="form-control">
              <label class="label cursor-pointer justify-start gap-4">
                <input
                  type="checkbox"
                  class="toggle toggle-primary"
                  :checked="completionGuardConfig.enforce_artifact_proof"
                  :disabled="!completionGuardConfig.enabled"
                  @change="toggleCompletionGuardArtifactProof"
                />
                <div>
                  <span class="label-text font-medium">{{ t('settings.agent.completionGuard.enforceArtifactProof') }}</span>
                  <p class="text-xs text-base-content/60 mt-1">
                    {{ t('settings.agent.completionGuard.enforceArtifactProofDesc') }}
                  </p>
                </div>
              </label>
            </div>
          </div>

          <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
            <label class="form-control">
              <span class="label-text text-xs">{{ t('settings.agent.completionGuard.toolHeavyMinToolCalls') }}</span>
              <input
                type="number"
                min="1"
                max="100000"
                class="input input-bordered input-sm"
                :value="completionGuardConfig.tool_heavy_min_tool_calls"
                :disabled="!completionGuardConfig.enabled"
                @change="updateCompletionGuardNumericField('tool_heavy_min_tool_calls', $event)"
              />
            </label>

            <label class="form-control">
              <span class="label-text text-xs">{{ t('settings.agent.completionGuard.minResponseCharsToolHeavy') }}</span>
              <input
                type="number"
                min="1"
                max="100000"
                class="input input-bordered input-sm"
                :value="completionGuardConfig.min_response_chars_tool_heavy"
                :disabled="!completionGuardConfig.enabled"
                @change="updateCompletionGuardNumericField('min_response_chars_tool_heavy', $event)"
              />
            </label>

            <label class="form-control">
              <span class="label-text text-xs">{{ t('settings.agent.completionGuard.minResponseCharsAfterTimeout') }}</span>
              <input
                type="number"
                min="1"
                max="100000"
                class="input input-bordered input-sm"
                :value="completionGuardConfig.min_response_chars_after_timeout"
                :disabled="!completionGuardConfig.enabled"
                @change="updateCompletionGuardNumericField('min_response_chars_after_timeout', $event)"
              />
            </label>

            <label class="form-control">
              <span class="label-text text-xs">{{ t('settings.agent.completionGuard.unfinishedPrefixMaxChars') }}</span>
              <input
                type="number"
                min="1"
                max="100000"
                class="input input-bordered input-sm"
                :value="completionGuardConfig.unfinished_prefix_max_chars"
                :disabled="!completionGuardConfig.enabled"
                @change="updateCompletionGuardNumericField('unfinished_prefix_max_chars', $event)"
              />
            </label>
          </div>
        </div>
      </div>

      <!-- Future sections can be added here -->
      <!-- Example: Tool Settings, Memory Settings, etc. -->
    </template>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { invoke } from '@tauri-apps/api/core'
import { dialog } from '@/composables/useDialog'

interface ShellConfig {
  default_policy: 'AlwaysProceed' | 'RequestReview'
  allowed_commands: string[]
  denied_commands: string[]
}

type ExecutionMode = 'docker' | 'host'

interface TerminalConfig {
  docker_image: string
  default_execution_mode: ExecutionMode
  docker_memory_limit: string
  docker_cpu_limit: string
  docker_use_host_network: boolean
}

interface ImageAttachmentsConfig {
  mode: 'local_ocr' | 'model_vision'
  allow_upload_to_model: boolean
}

interface SubagentConfig {
  timeout_secs: number
}

interface CompletionGuardConfig {
  enabled: boolean
  tool_heavy_min_tool_calls: number
  min_response_chars_tool_heavy: number
  min_response_chars_after_timeout: number
  unfinished_prefix_max_chars: number
  enforce_artifact_proof: boolean
}

interface AgentConfig {
  shell: ShellConfig
  terminal: TerminalConfig
  image_attachments?: ImageAttachmentsConfig
  subagent?: SubagentConfig
  completion_guard?: CompletionGuardConfig
}

interface UploadedFileEntry {
  date: string
  filename: string
  path: string
  size: number
  conversation_id?: string
}

interface WorkspaceSettings {
  auto_cleanup_enabled: boolean
  retention_days: number
  max_file_mb: number
  max_total_mb: number
  max_files_per_conversation: number
}

interface ExploitDbSettings {
  repo_url: string
  repo_path: string
}

interface ExploitDbSyncStatus {
  repo_url: string
  repo_path: string
  repo_exists: boolean
  index_exists: boolean
  indexed_entries: number
  last_commit: string | null
  indexed_at: string | null
  last_sync_at: string | null
}

const { t } = useI18n()

const loading = ref(true)
const shellConfig = ref<ShellConfig>({
  default_policy: 'RequestReview',
  allowed_commands: [],
  denied_commands: ['rm', 'rm -rf', 'mkfs', 'dd']
})

const terminalConfig = ref<TerminalConfig>({
  docker_image: 'sentinel-sandbox:latest',
  default_execution_mode: 'docker',
  docker_memory_limit: '2g',
  docker_cpu_limit: '4.0',
  docker_use_host_network: false
})

const imageAttachments = ref<ImageAttachmentsConfig>({
  mode: 'local_ocr',
  allow_upload_to_model: false
})

const subagentConfig = ref<SubagentConfig>({
  timeout_secs: 600 // 10 minutes default
})

const completionGuardConfig = ref<CompletionGuardConfig>({
  enabled: true,
  tool_heavy_min_tool_calls: 4,
  min_response_chars_tool_heavy: 80,
  min_response_chars_after_timeout: 280,
  unfinished_prefix_max_chars: 320,
  enforce_artifact_proof: true
})
const uploadedFiles = ref<UploadedFileEntry[]>([])
const uploadsLoading = ref(false)
const selectedUploadDate = ref('')
const conversationFilter = ref('')
const uploadSettings = ref<WorkspaceSettings>({
  auto_cleanup_enabled: false,
  retention_days: 30,
  max_file_mb: 20,
  max_total_mb: 1024,
  max_files_per_conversation: 100,
})
const workingDirectory = ref('')
const exploitDbSettings = ref<ExploitDbSettings>({
  repo_url: 'https://gitlab.com/exploit-database/exploitdb',
  repo_path: ''
})
const exploitDbStatus = ref<ExploitDbSyncStatus>({
  repo_url: 'https://gitlab.com/exploit-database/exploitdb',
  repo_path: '',
  repo_exists: false,
  index_exists: false,
  indexed_entries: 0,
  last_commit: null,
  indexed_at: null,
  last_sync_at: null
})
const exploitDbLoading = ref(false)
const exploitDbSyncing = ref(false)

const newAllowCommand = ref('')
const newDenyCommand = ref('')

const normalizePositiveInt = (value: unknown, fallback: number, min = 1, max = 100000): number => {
  const parsed = Number(value)
  if (!Number.isFinite(parsed)) return fallback
  return Math.min(max, Math.max(min, Math.round(parsed)))
}

const normalizeCompletionGuardConfig = (
  raw?: Partial<CompletionGuardConfig> | null
): CompletionGuardConfig => {
  return {
    enabled: raw?.enabled !== undefined ? !!raw.enabled : true,
    tool_heavy_min_tool_calls: normalizePositiveInt(raw?.tool_heavy_min_tool_calls, 4),
    min_response_chars_tool_heavy: normalizePositiveInt(raw?.min_response_chars_tool_heavy, 80),
    min_response_chars_after_timeout: normalizePositiveInt(raw?.min_response_chars_after_timeout, 280),
    unfinished_prefix_max_chars: normalizePositiveInt(raw?.unfinished_prefix_max_chars, 320),
    enforce_artifact_proof: raw?.enforce_artifact_proof !== undefined ? !!raw.enforce_artifact_proof : true,
  }
}

// Auto-save debounce
let saveTimeout: ReturnType<typeof setTimeout> | null = null

const uploadDates = computed(() => {
  const set = new Set(uploadedFiles.value.map((f) => f.date))
  return Array.from(set).sort((a, b) => b.localeCompare(a))
})

const filteredUploadedFiles = computed(() => {
  return uploadedFiles.value.filter((f) => {
    const dateOk = selectedUploadDate.value ? f.date === selectedUploadDate.value : true
    const convOk = conversationFilter.value
      ? (f.conversation_id || '').includes(conversationFilter.value.trim())
      : true
    return dateOk && convOk
  })
})

// Load config
async function loadConfig() {
  loading.value = true
  try {
    const result = await invoke<AgentConfig>('get_agent_config')
    if (result?.shell) {
      shellConfig.value = {
        default_policy: result.shell.default_policy || 'RequestReview',
        allowed_commands: result.shell.allowed_commands || [],
        denied_commands: result.shell.denied_commands || ['rm', 'rm -rf', 'mkfs', 'dd']
      }
    }
    if (result?.terminal) {
      terminalConfig.value = {
        docker_image: result.terminal.docker_image || 'sentinel-sandbox:latest',
        default_execution_mode: result.terminal.default_execution_mode || 'docker',
        docker_memory_limit: result.terminal.docker_memory_limit || '2g',
        docker_cpu_limit: result.terminal.docker_cpu_limit || '4.0',
        docker_use_host_network: !!result.terminal.docker_use_host_network
      }
    }
    if (result?.image_attachments) {
      imageAttachments.value = {
        mode: (result.image_attachments.mode as ImageAttachmentsConfig['mode']) || 'local_ocr',
        allow_upload_to_model: !!result.image_attachments.allow_upload_to_model
      }
    }
    if (result?.subagent) {
      subagentConfig.value = {
        timeout_secs: result.subagent.timeout_secs || 600
      }
    }
    completionGuardConfig.value = normalizeCompletionGuardConfig(result?.completion_guard)
    await loadWorkingDirectory()
  } catch (e) {
    console.error('Failed to load agent config:', e)
  } finally {
    loading.value = false
  }
}

async function loadWorkingDirectory() {
  try {
    const items = await invoke<Array<{ key: string; value: string }>>('get_config', {
      request: { category: 'agent', key: null }
    })
    const map = new Map(items.map((i) => [i.key, i.value]))
    let configured = String(map.get('working_directory') || '').trim()
    // Backward compatibility: read legacy ai.working_directory once.
    if (!configured) {
      const aiItems = await invoke<Array<{ key: string; value: string }>>('get_config', {
        request: { category: 'ai', key: null }
      })
      const aiMap = new Map(aiItems.map((i) => [i.key, i.value]))
      configured = String(aiMap.get('working_directory') || '').trim()
    }
    workingDirectory.value = configured
  } catch (e) {
    console.error('Failed to load working directory:', e)
  }
}

async function saveWorkingDirectory() {
  try {
    await invoke('save_config_batch', {
      configs: [
        {
          category: 'agent',
          key: 'working_directory',
          value: workingDirectory.value || '',
          description: 'Agent working directory',
          is_encrypted: false
        }
      ]
    })
    dialog.toast.success(t('settings.saveSuccess'))
  } catch (e) {
    console.error('Failed to save working directory:', e)
    dialog.toast.error(String(e))
  }
}

async function selectWorkingDirectory() {
  try {
    const { open } = await import('@tauri-apps/plugin-dialog')
    const selected = await open({
      directory: true,
      multiple: false,
      title: t('settings.ai.selectDirectory')
    })
    if (selected) {
      workingDirectory.value = selected as string
    }
  } catch (e) {
    console.error('Failed to select directory:', e)
  }
}

async function loadExploitDbSettings() {
  exploitDbLoading.value = true
  try {
    const settings = await invoke<ExploitDbSettings>('get_exploitdb_settings')
    exploitDbSettings.value = settings
  } catch (e) {
    console.error('Failed to load exploitdb settings:', e)
  } finally {
    exploitDbLoading.value = false
  }
}

async function loadExploitDbStatus() {
  exploitDbLoading.value = true
  try {
    const status = await invoke<ExploitDbSyncStatus>('get_exploitdb_sync_status')
    exploitDbStatus.value = status
  } catch (e) {
    console.error('Failed to load exploitdb status:', e)
    dialog.toast.error(String(e))
  } finally {
    exploitDbLoading.value = false
  }
}

async function saveExploitDbSettings() {
  exploitDbLoading.value = true
  try {
    const settings = await invoke<ExploitDbSettings>('save_exploitdb_settings', {
      repo_url: exploitDbSettings.value.repo_url,
      repo_path: exploitDbSettings.value.repo_path
    })
    exploitDbSettings.value = settings
    dialog.toast.success(t('settings.saveSuccess'))
    await loadExploitDbStatus()
  } catch (e) {
    console.error('Failed to save exploitdb settings:', e)
    dialog.toast.error(String(e))
  } finally {
    exploitDbLoading.value = false
  }
}

async function syncExploitDb() {
  exploitDbSyncing.value = true
  try {
    await invoke('sync_exploitdb', { force_reindex: false })
    dialog.toast.success(t('settings.agent.exploitdb.syncSuccess'))
    await loadExploitDbStatus()
  } catch (e) {
    console.error('Failed to sync exploitdb:', e)
    dialog.toast.error(String(e))
  } finally {
    exploitDbSyncing.value = false
  }
}

async function selectExploitDbPath() {
  try {
    const { open } = await import('@tauri-apps/plugin-dialog')
    const selected = await open({
      directory: true,
      multiple: false,
      title: t('settings.agent.exploitdb.selectPath')
    })
    if (selected) {
      exploitDbSettings.value.repo_path = selected as string
    }
  } catch (e) {
    console.error('Failed to select exploitdb path:', e)
  }
}

async function loadUploadedFiles() {
  uploadsLoading.value = true
  try {
    uploadedFiles.value = await invoke<UploadedFileEntry[]>('list_uploaded_files', {
      conversationId: conversationFilter.value.trim() || null,
      date: selectedUploadDate.value || null
    })
  } catch (e) {
    console.error('Failed to load uploaded files:', e)
    dialog.toast.error(String(e))
  } finally {
    uploadsLoading.value = false
  }
}

async function loadUploadSettings() {
  try {
    uploadSettings.value = await invoke<WorkspaceSettings>('get_workspace_settings')
  } catch (e) {
    console.error('Failed to load upload settings:', e)
  }
}

async function saveUploadSettings() {
  try {
    await invoke('save_workspace_settings', { settings: uploadSettings.value })
    dialog.toast.success(t('settings.saveSuccess'))
  } catch (e) {
    console.error('Failed to save upload settings:', e)
    dialog.toast.error(String(e))
  }
}

async function clearAllUploadedFiles() {
  const confirmed = await dialog.confirm(t('settings.agent.fileUploads.clearAllConfirm'))
  if (!confirmed) return
  uploadsLoading.value = true
  try {
    const count = await invoke<number>('clear_uploaded_files', {
      conversationId: conversationFilter.value.trim() || null
    })
    dialog.toast.success(t('settings.agent.fileUploads.clearedCount', { count }))
    await loadUploadedFiles()
  } catch (e) {
    console.error('Failed to clear uploaded files:', e)
    dialog.toast.error(String(e))
  } finally {
    uploadsLoading.value = false
  }
}

async function clearSelectedDateFiles() {
  if (!selectedUploadDate.value) return
  const confirmed = await dialog.confirm(
    t('settings.agent.fileUploads.clearDateConfirm', { date: selectedUploadDate.value })
  )
  if (!confirmed) return
  uploadsLoading.value = true
  try {
    const count = await invoke<number>('clear_uploaded_files', {
      date: selectedUploadDate.value,
      conversationId: conversationFilter.value.trim() || null
    })
    dialog.toast.success(t('settings.agent.fileUploads.clearedCount', { count }))
    await loadUploadedFiles()
  } catch (e) {
    console.error('Failed to clear date uploaded files:', e)
    dialog.toast.error(String(e))
  } finally {
    uploadsLoading.value = false
  }
}

function formatBytes(size: number): string {
  if (size < 1024) return `${size} B`
  if (size < 1024 * 1024) return `${(size / 1024).toFixed(1)} KB`
  return `${(size / (1024 * 1024)).toFixed(1)} MB`
}

// Auto-save config with debounce
async function autoSaveConfig() {
  if (saveTimeout) {
    clearTimeout(saveTimeout)
  }
  saveTimeout = setTimeout(async () => {
    try {
      const agentConfig: AgentConfig = {
        shell: shellConfig.value,
        terminal: terminalConfig.value,
        image_attachments: imageAttachments.value,
        subagent: subagentConfig.value,
        completion_guard: completionGuardConfig.value
      }
      await invoke('save_agent_config', { config: agentConfig })
      console.log('Agent config auto-saved')
    } catch (e) {
      console.error('Failed to auto-save agent config:', e)
      dialog.toast.error(t('settings.agent.autoSaveFailed'))
    }
  }, 300)
}

const setImageMode = (mode: ImageAttachmentsConfig['mode']) => {
  imageAttachments.value.mode = mode
  autoSaveConfig()
}

const toggleAllowUploadToModel = (event: Event) => {
  const target = event.target as HTMLInputElement
  imageAttachments.value.allow_upload_to_model = target.checked
  // If user disabled upload, force mode back to local OCR (safety)
  if (!target.checked && imageAttachments.value.mode === 'model_vision') {
    imageAttachments.value.mode = 'local_ocr'
  }
  autoSaveConfig()
}

// Update docker image
function updateDockerImage(event: Event) {
  const target = event.target as HTMLInputElement
  terminalConfig.value.docker_image = target.value
  autoSaveConfig()
}

// Update docker memory limit
function updateDockerMemoryLimit(event: Event) {
  const target = event.target as HTMLInputElement
  terminalConfig.value.docker_memory_limit = target.value
  autoSaveConfig()
}

// Update docker cpu limit
function updateDockerCpuLimit(event: Event) {
  const target = event.target as HTMLInputElement
  terminalConfig.value.docker_cpu_limit = target.value
  autoSaveConfig()
}

// Toggle docker host network mode
function toggleDockerHostNetwork(event: Event) {
  const target = event.target as HTMLInputElement
  terminalConfig.value.docker_use_host_network = target.checked
  autoSaveConfig()
}

// Update subagent timeout
function updateSubagentTimeout(event: Event) {
  const target = event.target as HTMLInputElement
  const value = parseInt(target.value, 10)
  if (!isNaN(value) && value > 0) {
    subagentConfig.value.timeout_secs = value
    autoSaveConfig()
  }
}

type CompletionGuardNumericField =
  | 'tool_heavy_min_tool_calls'
  | 'min_response_chars_tool_heavy'
  | 'min_response_chars_after_timeout'
  | 'unfinished_prefix_max_chars'

function updateCompletionGuardNumericField(field: CompletionGuardNumericField, event: Event) {
  const target = event.target as HTMLInputElement
  const raw = parseInt(target.value, 10)
  if (isNaN(raw)) return

  const clamped = normalizePositiveInt(raw, completionGuardConfig.value[field])
  completionGuardConfig.value = {
    ...completionGuardConfig.value,
    [field]: clamped
  }
  autoSaveConfig()
}

function toggleCompletionGuardEnabled(event: Event) {
  const target = event.target as HTMLInputElement
  completionGuardConfig.value.enabled = target.checked
  autoSaveConfig()
}

function toggleCompletionGuardArtifactProof(event: Event) {
  const target = event.target as HTMLInputElement
  completionGuardConfig.value.enforce_artifact_proof = target.checked
  autoSaveConfig()
}

// Toggle execution mode
function toggleExecutionMode() {
  terminalConfig.value.default_execution_mode = 
    terminalConfig.value.default_execution_mode === 'docker' ? 'host' : 'docker'
  autoSaveConfig()
}

// Set default policy
function setDefaultPolicy(policy: 'AlwaysProceed' | 'RequestReview') {
  shellConfig.value.default_policy = policy
  autoSaveConfig()
}

// Add allow command
function addAllowCommand() {
  const cmd = newAllowCommand.value.trim()
  if (cmd && !shellConfig.value.allowed_commands.includes(cmd)) {
    shellConfig.value.allowed_commands.push(cmd)
    newAllowCommand.value = ''
    autoSaveConfig()
  }
}

// Remove allow command
function removeAllowCommand(index: number) {
  shellConfig.value.allowed_commands.splice(index, 1)
  autoSaveConfig()
}

// Add deny command
function addDenyCommand() {
  const cmd = newDenyCommand.value.trim()
  if (cmd && !shellConfig.value.denied_commands.includes(cmd)) {
    shellConfig.value.denied_commands.push(cmd)
    newDenyCommand.value = ''
    autoSaveConfig()
  }
}

// Remove deny command
function removeDenyCommand(index: number) {
  shellConfig.value.denied_commands.splice(index, 1)
  autoSaveConfig()
}

onMounted(() => {
  loadConfig()
  loadExploitDbSettings()
  loadExploitDbStatus()
  loadUploadSettings()
  loadUploadedFiles()
})
</script>

<style scoped>
.agent-settings {
  @apply space-y-6;
}

.card {
  @apply transition-all duration-200 hover:shadow-md;
}

.virtual-list-container {
  scrollbar-width: thin;
  scrollbar-color: oklch(var(--bc) / 0.2) transparent;
}

.virtual-list-container::-webkit-scrollbar {
  width: 8px;
}

.virtual-list-container::-webkit-scrollbar-track {
  background: transparent;
}

.virtual-list-container::-webkit-scrollbar-thumb {
  background-color: oklch(var(--bc) / 0.2);
  border-radius: 4px;
}

.virtual-list-container::-webkit-scrollbar-thumb:hover {
  background-color: oklch(var(--bc) / 0.3);
}
</style>
