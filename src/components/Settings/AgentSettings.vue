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
            <div class="form-control mb-4">
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
}

interface ImageAttachmentsConfig {
  mode: 'local_ocr' | 'model_vision'
  allow_upload_to_model: boolean
}

interface AgentConfig {
  shell: ShellConfig
  terminal: TerminalConfig
  image_attachments?: ImageAttachmentsConfig
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
  default_execution_mode: 'docker'
})

const imageAttachments = ref<ImageAttachmentsConfig>({
  mode: 'local_ocr',
  allow_upload_to_model: false
})

const newAllowCommand = ref('')
const newDenyCommand = ref('')

// Auto-save debounce
let saveTimeout: ReturnType<typeof setTimeout> | null = null

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
        default_execution_mode: result.terminal.default_execution_mode || 'docker'
      }
    }
    if (result?.image_attachments) {
      imageAttachments.value = {
        mode: (result.image_attachments.mode as ImageAttachmentsConfig['mode']) || 'local_ocr',
        allow_upload_to_model: !!result.image_attachments.allow_upload_to_model
      }
    }
  } catch (e) {
    console.error('Failed to load agent config:', e)
  } finally {
    loading.value = false
  }
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
        image_attachments: imageAttachments.value
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
