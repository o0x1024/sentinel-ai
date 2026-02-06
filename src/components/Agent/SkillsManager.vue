<template>
  <div :class="['skills-manager', { 'fullscreen': isFullscreen }]">
    <!-- Header -->
    <div class="flex items-center justify-between mb-4">
      <h3 class="text-lg font-semibold flex items-center gap-2">
        <i class="fas fa-wand-magic-sparkles text-primary"></i>
        {{ t('agent.skillManagement') }}
        <span class="badge badge-sm badge-ghost">{{ skills.length }}</span>
      </h3>
      <div class="flex items-center gap-2" v-if="!props.embedded">
        <button
          @click="$emit('toggle-fullscreen')"
          class="btn btn-sm btn-ghost btn-circle"
          :title="isFullscreen ? t('common.exitFullscreen') : t('common.fullscreen')"
        >
          <i :class="isFullscreen ? 'fas fa-compress' : 'fas fa-expand'"></i>
        </button>
        <button @click="$emit('close')" class="btn btn-sm btn-ghost btn-circle">
          <i class="fas fa-times"></i>
        </button>
      </div>
    </div>

    <!-- Create / Edit Form - Left-Right Layout -->
    <div v-if="editingSkill" class="card bg-base-200 p-4 mb-4">
      <div class="grid grid-cols-1 lg:grid-cols-5 gap-4">
        <!-- Left Panel: Skill Content (60%) -->
        <div class="lg:col-span-3 space-y-3">
          <div class="text-sm font-medium text-base-content/70 mb-2">
            <i class="fas fa-edit mr-1"></i>
            {{ t('agent.leftPanel') }}
          </div>

          <div class="form-control">
            <label class="label py-1">
              <span class="label-text">{{ t('agent.skillName') }}</span>
            </label>
            <input
              v-model="editingSkill.name"
              type="text"
              class="input input-sm input-bordered w-full"
              :placeholder="t('agent.skillNamePlaceholder')"
            />
            <div class="text-xs text-base-content/60 mt-1">
              {{ t('agent.skillNameBestPractice') }}
            </div>
            <div v-if="nameError" class="text-xs text-error mt-1">
              <i class="fas fa-exclamation-triangle mr-1"></i>
              {{ nameError }}
            </div>
          </div>

          <div class="form-control">
            <label class="label py-1">
              <span class="label-text">{{ t('agent.skillDescription') }}</span>
            </label>
            <input
              v-model="editingSkill.description"
              type="text"
              class="input input-sm input-bordered w-full"
              :placeholder="t('agent.skillDescriptionPlaceholder')"
            />
            <div class="text-xs text-base-content/60 mt-1">
              {{ t('agent.skillDescriptionBestPractice') }}
            </div>
            <div v-if="descriptionError" class="text-xs text-error mt-1">
              <i class="fas fa-exclamation-triangle mr-1"></i>
              {{ descriptionError }}
            </div>
          </div>

          <div class="form-control">
            <label class="label py-1">
              <span class="label-text">{{ t('agent.skillArgumentHint') }}</span>
              <span class="label-text-alt text-base-content/60">{{ t('agent.argumentHintHelp') }}</span>
            </label>
            <input
              v-model="editingSkill.argument_hint"
              type="text"
              class="input input-sm input-bordered w-full"
              :placeholder="t('agent.argumentHintPlaceholder')"
            />
          </div>

          <div class="grid grid-cols-1 md:grid-cols-2 gap-2">
            <label class="label cursor-pointer justify-start gap-3">
              <input
                type="checkbox"
                v-model="editingSkill.user_invocable"
                class="checkbox checkbox-sm checkbox-primary"
              />
              <span class="label-text">{{ t('agent.userInvocable') }}</span>
            </label>
            <label class="label cursor-pointer justify-start gap-3">
              <input
                type="checkbox"
                v-model="editingSkill.disable_model_invocation"
                class="checkbox checkbox-sm checkbox-primary"
              />
              <span class="label-text">{{ t('agent.disableModelInvocation') }}</span>
            </label>
          </div>

          <div class="grid grid-cols-1 md:grid-cols-3 gap-2">
            <div class="form-control">
              <label class="label py-1">
                <span class="label-text">{{ t('agent.skillModel') }}</span>
              </label>
              <input
                v-model="editingSkill.model"
                type="text"
                class="input input-sm input-bordered w-full"
                :placeholder="t('agent.skillModelPlaceholder')"
              />
            </div>
            <div class="form-control">
              <label class="label py-1">
                <span class="label-text">{{ t('agent.skillContext') }}</span>
              </label>
              <input
                v-model="editingSkill.context"
                type="text"
                class="input input-sm input-bordered w-full"
                :placeholder="t('agent.skillContextPlaceholder')"
              />
            </div>
            <div class="form-control">
              <label class="label py-1">
                <span class="label-text">{{ t('agent.skillAgent') }}</span>
              </label>
              <input
                v-model="editingSkill.agent"
                type="text"
                class="input input-sm input-bordered w-full"
                :placeholder="t('agent.skillAgentPlaceholder')"
              />
            </div>
          </div>

          <div class="form-control">
            <label class="label py-1">
              <span class="label-text">{{ t('agent.skillContent') }}</span>
              <span class="label-text-alt text-base-content/60">{{ t('agent.skillContentHint') }}</span>
            </label>
            <textarea
              v-model="editingSkill.content"
              class="textarea textarea-bordered w-full h-36 text-sm"
              :placeholder="t('agent.skillContentPlaceholder')"
            ></textarea>
            <div class="text-xs text-base-content/60 mt-1">
              {{ t('agent.skillContentBestPractice') }}
            </div>
            <div class="flex justify-end">
              <button @click="applyTemplate" class="btn btn-xs btn-ghost">
                <i class="fas fa-file-alt mr-1"></i>
                {{ t('agent.skillTemplate') }}
              </button>
            </div>
          </div>

          <div class="form-control">
            <label class="label py-1">
              <span class="label-text">{{ t('agent.skillHooks') }}</span>
              <span class="label-text-alt text-base-content/60">{{ t('agent.skillHooksHint') }}</span>
            </label>
            <textarea
              v-model="editingSkill.hooks_raw"
              class="textarea textarea-bordered w-full h-28 text-sm"
              :placeholder="t('agent.skillHooksPlaceholder')"
            ></textarea>
            <div v-if="!hooksValid" class="text-xs text-error mt-1">
              <i class="fas fa-exclamation-triangle mr-1"></i>
              {{ t('agent.skillHooksInvalid') }}
            </div>
          </div>

          <!-- Skill Files -->
          <div class="form-control">
            <label class="label py-1">
              <span class="label-text">{{ t('agent.skillFiles') }}</span>
              <span class="label-text-alt text-base-content/60">{{ t('agent.skillFilesHint') }}</span>
            </label>
            <div v-if="isNewSkill" class="text-xs text-base-content/60">
              {{ t('agent.skillFilesNeedSaveFirst') }}
            </div>
            <div v-else class="space-y-2">
              <div class="flex flex-wrap gap-2 items-center">
                <input
                  v-model="newFilePath"
                  type="text"
                  class="input input-xs input-bordered flex-1 min-w-[160px]"
                  :placeholder="t('agent.skillFilesNewPath')"
                />
                <button
                  @click="createFile"
                  class="btn btn-xs btn-primary btn-outline"
                  :disabled="!newFilePath.trim()"
                >
                  <i class="fas fa-plus"></i>
                  {{ t('agent.skillFilesCreate') }}
                </button>
                <button
                  @click="uploadFiles"
                  class="btn btn-xs btn-outline"
                  :disabled="uploadingFiles"
                >
                  <i class="fas fa-upload"></i>
                  {{ t('agent.skillFilesUpload') }}
                </button>
                <button @click="refreshFiles" class="btn btn-xs btn-ghost">
                  <i class="fas fa-sync-alt"></i>
                </button>
              </div>

              <div class="grid grid-cols-1 md:grid-cols-2 gap-2">
                <div class="border border-base-300 rounded-lg bg-base-100 max-h-44 overflow-y-auto">
                  <div v-if="loadingFiles" class="flex justify-center py-4">
                    <span class="loading loading-spinner loading-sm"></span>
                  </div>
                  <div v-else>
                    <button
                      v-for="file in displayFiles"
                      :key="file.path"
                      class="w-full text-left px-2 py-1 text-xs hover:bg-base-200 transition-colors"
                      :class="{ 'bg-base-200': selectedFilePath === file.path }"
                      @click="selectFile(file)"
                    >
                      <div class="flex items-center justify-between gap-2">
                        <span class="truncate">{{ file.path }}</span>
                        <span class="text-[10px] text-base-content/60">{{ formatFileSize(file.size) }}</span>
                      </div>
                    </button>
                    <div v-if="displayFiles.length === 0" class="text-center py-4 text-xs text-base-content/60">
                      {{ t('agent.skillFilesEmpty') }}
                    </div>
                  </div>
                </div>

                <div class="border border-base-300 rounded-lg bg-base-100 p-2">
                  <div v-if="!selectedFilePath" class="text-xs text-base-content/60">
                    {{ t('agent.skillFilesSelect') }}
                  </div>
                  <div v-else class="space-y-2">
                    <div class="text-xs font-mono text-base-content/70 truncate">{{ selectedFilePath }}</div>
                    <textarea
                      v-model="fileContent"
                      class="textarea textarea-bordered w-full h-28 text-xs font-mono"
                      :placeholder="t('agent.skillFilesContentPlaceholder')"
                    ></textarea>
                    <div class="flex gap-2 justify-end">
                      <button
                        @click="saveFile"
                        class="btn btn-xs btn-primary"
                        :disabled="!fileDirty || savingFile"
                      >
                        <i class="fas fa-save"></i>
                        {{ t('agent.skillFilesSave') }}
                      </button>
                      <button
                        @click="deleteFile"
                        class="btn btn-xs btn-error btn-outline"
                      >
                        <i class="fas fa-trash"></i>
                        {{ t('agent.skillFilesDelete') }}
                      </button>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </div>

          <div class="space-y-2 pt-2">
            <div class="form-control">
              <label class="label py-1">
                <span class="label-text">{{ t('agent.skillBrief') }}</span>
                <span class="label-text-alt text-base-content/60">{{ t('agent.skillBriefHint') }}</span>
              </label>
              <input
                v-model="briefDescription"
                type="text"
                class="input input-sm input-bordered w-full"
                :placeholder="t('agent.skillBriefPlaceholder')"
              />
            </div>
            <button
              @click="generateWithAI"
              class="btn btn-sm btn-primary btn-outline gap-1 w-full"
              :disabled="!canUseAI || aiGenerating"
              :class="{ 'btn-disabled': !canUseAI }"
              :title="!canUseAI ? t('agent.selectToolsOrAddContent') : ''"
            >
              <span v-if="aiGenerating" class="loading loading-spinner loading-xs"></span>
              <i v-else class="fas fa-magic"></i>
              {{ aiButtonText }}
            </button>
            <div v-if="!canUseAI" class="text-xs text-warning text-center">
              <i class="fas fa-info-circle mr-1"></i>
              {{ t('agent.aiGenerateHint') }}
            </div>
          </div>

          <div class="flex justify-end gap-2 pt-2 border-t border-base-300">
            <button @click="cancelEdit" class="btn btn-sm btn-ghost">
              {{ t('common.cancel') }}
            </button>
            <button @click="saveSkill" class="btn btn-sm btn-primary" :disabled="!canSave">
              <i class="fas fa-save"></i>
              {{ isNewSkill ? t('common.create') : t('common.save') }}
            </button>
          </div>
        </div>

        <!-- Right Panel: Allowed Tools (40%) -->
        <div class="lg:col-span-2 space-y-3">
          <div class="text-sm font-medium text-base-content/70 mb-2">
            <i class="fas fa-tools mr-1"></i>
            {{ t('agent.allowedTools') }}
          </div>

          <div class="form-control">
            <input
              v-model="toolSearchQuery"
              type="text"
              class="input input-sm input-bordered w-full"
              :placeholder="t('agent.searchTools')"
            />
          </div>

          <div class="form-control">
            <select
              v-model="selectedCategory"
              class="select select-sm select-bordered w-full"
            >
              <option value="">{{ t('agent.filterByCategory') }}</option>
              <option v-for="cat in toolCategories" :key="cat" :value="cat">
                {{ getCategoryDisplayName(cat) }}
              </option>
            </select>
          </div>

          <div class="flex items-center justify-between text-xs text-base-content/60">
            <span>{{ selectedToolCount }} {{ t('agent.toolsSelected') }}</span>
            <div class="flex gap-1">
              <button
                v-if="filteredTools.length > 0 && !isAllFilteredSelected"
                @click="selectAllFilteredTools"
                class="btn btn-xs btn-ghost"
              >
                {{ t('agent.selectAll') }}
              </button>
              <button
                v-if="selectedToolCount > 0"
                @click="clearToolSelection"
                class="btn btn-xs btn-ghost"
              >
                {{ t('agent.clearSelection') }}
              </button>
            </div>
          </div>

          <div class="border border-base-300 rounded-lg p-2 max-h-96 overflow-y-auto bg-base-100">
            <div v-if="loadingTools" class="flex justify-center py-4">
              <span class="loading loading-spinner loading-sm"></span>
            </div>
            <div v-else class="space-y-1">
              <label
                v-for="tool in filteredTools"
                :key="tool.id"
                class="flex items-center gap-2 p-2 hover:bg-base-200 rounded cursor-pointer transition-colors"
              >
                <input
                  type="checkbox"
                  :value="tool.id"
                  v-model="editingSkill.allowed_tools"
                  class="checkbox checkbox-xs checkbox-primary"
                />
                <div class="flex-1 min-w-0">
                  <div class="text-sm font-medium truncate">{{ tool.name }}</div>
                  <div class="text-xs text-base-content/60 truncate">{{ tool.description }}</div>
                </div>
                <span class="badge badge-xs flex-shrink-0" :class="getCategoryBadgeClass(tool.category)">
                  {{ getCategoryDisplayName(tool.category) }}
                </span>
              </label>
              <div v-if="filteredTools.length === 0" class="text-center py-4 text-base-content/60 text-sm">
                {{ t('agent.noToolsAvailable') }}
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- Skill List -->
    <div class="space-y-2">
      <div class="mb-2"></div>

      <div v-if="loading" class="flex justify-center py-8">
        <span class="loading loading-spinner loading-md"></span>
      </div>

      <div v-else-if="skills.length === 0" class="text-center py-8 text-base-content/60">
        <i class="fas fa-inbox text-3xl mb-2"></i>
        <p>{{ t('agent.noSkills') }}</p>
        <p class="text-sm mt-1">{{ t('agent.createFirstSkill') }}</p>
      </div>

      <div v-else class="skills-cards-grid">
        <div
          v-for="skill in skills"
          :key="skill.id"
          class="card bg-base-200 border border-base-300 p-4 hover:shadow-sm hover:border-base-content/20 transition"
        >
          <div class="flex items-start justify-between gap-3">
            <div class="flex items-start gap-3 min-w-0 flex-1">
              <div :class="['skill-icon', getSkillIconClass(skill.id)]">
                <i :class="getSkillIcon(skill.id)"></i>
              </div>
              <div class="min-w-0 flex-1">
                <div class="font-medium truncate">{{ skill.name }}</div>
                <div class="text-sm text-base-content/60 line-clamp-2">{{ skill.description }}</div>
                <div class="text-[10px] text-base-content/50 font-mono mt-1 truncate">{{ skill.id }}</div>
              </div>
            </div>
            <div class="flex items-center gap-2">
              <label class="flex items-center gap-2 text-xs text-base-content/60">
                <input
                  type="checkbox"
                  class="toggle toggle-xs"
                  :checked="isSkillEnabled(skill.id)"
                  @change="toggleSkillEnabled(skill.id, ($event.target as HTMLInputElement).checked)"
                />
              </label>
              <button @click="startEdit(skill)" class="btn btn-xs btn-ghost btn-circle">
                <i class="fas fa-edit"></i>
              </button>
              <button
                @click="confirmDelete(skill)"
                class="btn btn-xs btn-ghost btn-circle text-error"
                :disabled="deletingSkillIds.includes(skill.id)"
              >
                <i :class="deletingSkillIds.includes(skill.id) ? 'fas fa-spinner fa-spin' : 'fas fa-trash'"></i>
              </button>
            </div>
          </div>
          <div class="flex items-center gap-2 mt-3 flex-wrap">
                <span class="badge badge-sm badge-ghost">
                  {{ skill.allowed_tools?.length || 0 }} {{ t('agent.tools') }}
                </span>
                <span v-if="skill.content" class="badge badge-sm badge-info badge-outline">
                  {{ t('agent.hasSkillContent') }}
                </span>
                <span v-if="skill.disable_model_invocation" class="badge badge-sm badge-warning badge-outline">
                  {{ t('agent.modelInvocationDisabled') }}
                </span>
                <span v-if="!skill.user_invocable" class="badge badge-sm badge-neutral badge-outline">
                  {{ t('agent.notUserInvocable') }}
                </span>
                <span v-if="!isSkillEnabled(skill.id)" class="badge badge-sm badge-error badge-outline">
                  {{ t('common.disabled') }}
                </span>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { useI18n } from 'vue-i18n'

interface Skill {
  id: string
  name: string
  description: string
  source_path: string
  argument_hint: string
  disable_model_invocation: boolean
  user_invocable: boolean
  allowed_tools: string[]
  model: string
  context: string
  agent: string
  hooks: Record<string, any> | null
  content?: string
  created_at?: string
  updated_at?: string
}

interface SkillForm extends Skill {
  content: string
  hooks_raw: string
}

interface ToolMetadata {
  id: string
  name: string
  description: string
  category: string
}

interface SkillFileEntry {
  path: string
  size: number
}

interface Props {
  isFullscreen?: boolean
  embedded?: boolean
}

const props = withDefaults(defineProps<Props>(), {
  isFullscreen: false,
  embedded: false
})

const emit = defineEmits<{
  'close': []
  'changed': []
  'toggle-fullscreen': []
}>()

const { t } = useI18n()

const skills = ref<Skill[]>([])
const skillEnabledMap = ref<Record<string, boolean>>({})
const availableTools = ref<ToolMetadata[]>([])
const filteredTools = ref<ToolMetadata[]>([])
const loading = ref(false)
const loadingTools = ref(false)
const editingSkill = ref<SkillForm | null>(null)
const isNewSkill = ref(false)
const aiGenerating = ref(false)
const generatedContent = ref('')
const toolSearchQuery = ref('')
const selectedCategory = ref('')
const briefDescription = ref('')
const skillFiles = ref<SkillFileEntry[]>([])
const loadingFiles = ref(false)
const uploadingFiles = ref(false)
const savingFile = ref(false)
const selectedFilePath = ref('')
const fileContent = ref('')
const fileOriginal = ref('')
const newFilePath = ref('')
const deletingSkillIds = ref<string[]>([])
const skillIconPalette = [
  { icon: 'fas fa-wand-magic-sparkles', cls: 'bg-primary/15 text-primary' },
  { icon: 'fas fa-bug', cls: 'bg-error/15 text-error' },
  { icon: 'fas fa-shield-halved', cls: 'bg-success/15 text-success' },
  { icon: 'fas fa-code', cls: 'bg-info/15 text-info' },
  { icon: 'fas fa-terminal', cls: 'bg-warning/15 text-warning' },
  { icon: 'fas fa-lock', cls: 'bg-accent/15 text-accent' },
  { icon: 'fas fa-sitemap', cls: 'bg-secondary/15 text-secondary' },
  { icon: 'fas fa-cogs', cls: 'bg-neutral/15 text-neutral' }
]

const isFullscreen = computed(() => props.isFullscreen)

const selectedToolCount = computed(() => editingSkill.value?.allowed_tools?.length || 0)

const isAllFilteredSelected = computed(() => {
  if (!editingSkill.value || filteredTools.value.length === 0) return false
  return filteredTools.value.every(tool => editingSkill.value?.allowed_tools.includes(tool.id))
})

const hasExistingContent = computed(() => {
  if (!editingSkill.value) return false
  return !!(
    editingSkill.value.name.trim() ||
    editingSkill.value.description.trim() ||
    editingSkill.value.argument_hint.trim() ||
    editingSkill.value.content.trim() ||
    editingSkill.value.model.trim() ||
    editingSkill.value.context.trim() ||
    editingSkill.value.agent.trim() ||
    editingSkill.value.hooks_raw.trim()
  )
})

const canUseAI = computed(() => {
  return selectedToolCount.value > 0 || hasExistingContent.value || briefDescription.value.trim().length > 0
})

const aiButtonText = computed(() => {
  if (aiGenerating.value) return t('agent.aiGenerating')
  if (briefDescription.value.trim()) return t('agent.aiGenerate')
  return hasExistingContent.value ? t('agent.aiExpand') : t('agent.aiGenerate')
})

const hooksValid = computed(() => {
  if (!editingSkill.value) return true
  if (!editingSkill.value.hooks_raw.trim()) return true
  try {
    JSON.parse(editingSkill.value.hooks_raw)
    return true
  } catch {
    return false
  }
})

const nameError = computed(() => {
  if (!editingSkill.value) return ''
  const name = editingSkill.value.name.trim()
  if (!name) return t('agent.skillNameRequired')
  if (name.length > 64) return t('agent.skillNameTooLong')
  if (name.includes('<') || name.includes('>')) return t('agent.skillNameNoXml')
  const lower = name.toLowerCase()
  if (lower.includes('anthropic') || lower.includes('claude')) return t('agent.skillNameReserved')
  if (!/^[a-z0-9][a-z0-9-]*$/.test(name)) return t('agent.skillNameInvalidChars')
  if (name.endsWith('-')) return t('agent.skillNameNoTrailingDash')
  return ''
})

const descriptionError = computed(() => {
  if (!editingSkill.value) return ''
  const description = editingSkill.value.description.trim()
  if (!description) return t('agent.skillDescriptionRequired')
  if (description.length > 1024) return t('agent.skillDescriptionTooLong')
  if (description.includes('<') || description.includes('>')) return t('agent.skillDescriptionNoXml')
  return ''
})

const canSave = computed(() => {
  if (!editingSkill.value) return false
  return !nameError.value && !descriptionError.value && hooksValid.value
})

const toolCategories = computed(() => {
  const categories = new Set<string>()
  availableTools.value.forEach(tool => {
    if (tool.category) categories.add(tool.category)
  })
  return Array.from(categories).sort()
})

const displayFiles = computed(() => {
  return skillFiles.value.filter(file => file.path.toLowerCase() !== 'skill.md')
})

const fileDirty = computed(() => {
  return fileContent.value !== fileOriginal.value
})

const filterTools = () => {
  let result = availableTools.value

  if (toolSearchQuery.value.trim()) {
    const query = toolSearchQuery.value.toLowerCase()
    result = result.filter(tool =>
      tool.name.toLowerCase().includes(query) ||
      tool.description.toLowerCase().includes(query)
    )
  }

  if (selectedCategory.value) {
    result = result.filter(tool => tool.category === selectedCategory.value)
  }

  filteredTools.value = result
}

const clearToolSelection = () => {
  if (editingSkill.value) {
    editingSkill.value.allowed_tools = []
  }
}

const selectAllFilteredTools = () => {
  if (!editingSkill.value) return
  const currentIds = new Set(editingSkill.value.allowed_tools)
  filteredTools.value.forEach(tool => currentIds.add(tool.id))
  editingSkill.value.allowed_tools = Array.from(currentIds)
}

const resetFileEditor = () => {
  selectedFilePath.value = ''
  fileContent.value = ''
  fileOriginal.value = ''
  newFilePath.value = ''
}

const loadSkillFiles = async () => {
  if (!editingSkill.value || isNewSkill.value) {
    skillFiles.value = []
    resetFileEditor()
    return
  }
  loadingFiles.value = true
  try {
    skillFiles.value = await invoke<SkillFileEntry[]>('list_skill_files', { id: editingSkill.value.id })
  } catch (error) {
    console.error('Failed to load skill files:', error)
  } finally {
    loadingFiles.value = false
  }
}

const refreshFiles = async () => {
  await loadSkillFiles()
}

const selectFile = async (file: SkillFileEntry) => {
  if (!editingSkill.value) return
  selectedFilePath.value = file.path
  fileContent.value = ''
  fileOriginal.value = ''
  try {
    const content = await invoke<string>('read_skill_file', {
      id: editingSkill.value.id,
      path: file.path
    })
    fileContent.value = content
    fileOriginal.value = content
  } catch (error) {
    console.error('Failed to read file:', error)
  }
}

const saveFile = async () => {
  if (!editingSkill.value || !selectedFilePath.value) return
  savingFile.value = true
  try {
    await invoke('save_skill_file', {
      id: editingSkill.value.id,
      path: selectedFilePath.value,
      content: fileContent.value
    })
    fileOriginal.value = fileContent.value
    await loadSkillFiles()
  } catch (error) {
    console.error('Failed to save file:', error)
  } finally {
    savingFile.value = false
  }
}

const deleteFile = async () => {
  if (!editingSkill.value || !selectedFilePath.value) return
  if (!confirm(t('agent.skillFilesDeleteConfirm'))) return
  try {
    await invoke('delete_skill_file', {
      id: editingSkill.value.id,
      path: selectedFilePath.value
    })
    await loadSkillFiles()
    resetFileEditor()
  } catch (error) {
    console.error('Failed to delete file:', error)
  }
}

const createFile = async () => {
  if (!editingSkill.value) return
  const path = newFilePath.value.trim()
  if (!path) return
  try {
    await invoke('save_skill_file', {
      id: editingSkill.value.id,
      path,
      content: ''
    })
    newFilePath.value = ''
    await loadSkillFiles()
    const created = skillFiles.value.find(f => f.path === path)
    if (created) {
      await selectFile(created)
    }
  } catch (error) {
    console.error('Failed to create file:', error)
  }
}

const uploadFiles = async () => {
  if (!editingSkill.value) return
  try {
    const { open } = await import('@tauri-apps/plugin-dialog')
    const selected = await open({
      multiple: true,
      filters: [
        { name: 'Text', extensions: ['md', 'txt', 'json', 'yml', 'yaml'] }
      ]
    })
    const paths = Array.isArray(selected) ? selected : selected ? [selected] : []
    if (paths.length === 0) return
    uploadingFiles.value = true
    for (const p of paths) {
      await invoke('import_skill_file', {
        id: editingSkill.value.id,
        sourcePath: p
      })
    }
    await loadSkillFiles()
  } catch (error) {
    console.error('Failed to upload files:', error)
  } finally {
    uploadingFiles.value = false
  }
}

const loadSkillEnabledMap = async () => {
  try {
    const configs = await invoke<Array<{ key: string, value: string }>>('get_config', {
      request: { category: 'skills', key: null }
    })
    const next: Record<string, boolean> = {}
    for (const cfg of configs) {
      if (!cfg.key?.startsWith('enabled::')) continue
      const id = cfg.key.slice('enabled::'.length)
      const raw = cfg.value?.trim().toLowerCase()
      next[id] = raw === 'true' || raw === '1' || raw === 'yes' || raw === 'on'
    }
    skillEnabledMap.value = next
  } catch (error) {
    console.error('Failed to load skill enabled map:', error)
    skillEnabledMap.value = {}
  }
}

const isSkillEnabled = (id: string) => {
  if (Object.prototype.hasOwnProperty.call(skillEnabledMap.value, id)) {
    return skillEnabledMap.value[id]
  }
  return true
}

const toggleSkillEnabled = async (id: string, enabled: boolean) => {
  try {
    await invoke('set_config', {
      category: 'skills',
      key: `enabled::${id}`,
      value: enabled ? 'true' : 'false'
    })
    skillEnabledMap.value = { ...skillEnabledMap.value, [id]: enabled }
  } catch (error) {
    console.error('Failed to save skill enabled setting:', error)
  }
}

const loadSkills = async () => {
  loading.value = true
  try {
    await invoke('refresh_skills_index')
    skills.value = await invoke<Skill[]>('list_skills_full')
    await loadSkillEnabledMap()
  } catch (error) {
    console.error('Failed to load skills:', error)
  } finally {
    loading.value = false
  }
}

const loadTools = async () => {
  loadingTools.value = true
  try {
    availableTools.value = await invoke<ToolMetadata[]>('get_all_tool_metadata')
  } catch (error) {
    console.error('Failed to load tools:', error)
  } finally {
    loadingTools.value = false
  }
}

const startCreate = () => {
  isNewSkill.value = true
  editingSkill.value = {
    id: '',
    name: '',
    description: '',
    source_path: '',
    content: '',
    argument_hint: '',
    disable_model_invocation: false,
    user_invocable: true,
    allowed_tools: [],
    model: '',
    context: '',
    agent: '',
    hooks: {},
    hooks_raw: '',
  }
  skillFiles.value = []
  resetFileEditor()
  briefDescription.value = ''
}

const startEdit = async (skill: Skill) => {
  isNewSkill.value = false
  let content = ''
  try {
    content = await invoke<string>('get_skill_markdown', { id: skill.id })
  } catch (error) {
    console.error('Failed to load SKILL.md content:', error)
  }
  editingSkill.value = {
    ...skill,
    content,
    allowed_tools: [...(skill.allowed_tools || [])],
    hooks: skill.hooks || {},
    hooks_raw: JSON.stringify(skill.hooks || {}, null, 2) || '',
  }
  briefDescription.value = ''
  await loadSkillFiles()
}

const cancelEdit = () => {
  editingSkill.value = null
  isNewSkill.value = false
  skillFiles.value = []
  resetFileEditor()
  briefDescription.value = ''
}

const saveSkill = async () => {
  if (!editingSkill.value || !canSave.value) return

  let hooksPayload: Record<string, any> | null = {}
  if (editingSkill.value.hooks_raw.trim()) {
    try {
      hooksPayload = JSON.parse(editingSkill.value.hooks_raw)
    } catch (error) {
      console.error('Invalid hooks JSON:', error)
      alert(t('agent.skillHooksInvalid'))
      return
    }
  }

  try {
    if (isNewSkill.value) {
      await invoke('create_skill', {
        payload: {
          name: editingSkill.value.name,
          description: editingSkill.value.description,
          content: editingSkill.value.content,
          argument_hint: editingSkill.value.argument_hint,
          disable_model_invocation: editingSkill.value.disable_model_invocation,
          user_invocable: editingSkill.value.user_invocable,
          allowed_tools: editingSkill.value.allowed_tools,
          model: editingSkill.value.model,
          context: editingSkill.value.context,
          agent: editingSkill.value.agent,
          hooks: hooksPayload,
        }
      })
    } else {
      await invoke('update_skill', {
        id: editingSkill.value.id,
        payload: {
          name: editingSkill.value.name,
          description: editingSkill.value.description,
          content: editingSkill.value.content,
          argument_hint: editingSkill.value.argument_hint,
          disable_model_invocation: editingSkill.value.disable_model_invocation,
          user_invocable: editingSkill.value.user_invocable,
          allowed_tools: editingSkill.value.allowed_tools,
          model: editingSkill.value.model,
          context: editingSkill.value.context,
          agent: editingSkill.value.agent,
          hooks: hooksPayload,
        }
      })
    }

    editingSkill.value = null
    isNewSkill.value = false
    await loadSkills()
    emit('changed')
  } catch (error) {
    console.error('Failed to save skill:', error)
    alert(`${t('agent.skillSaveFailed')}: ${error}`)
  }
}

const confirmDelete = async (skill: Skill) => {
  if (!confirm(t('agent.skillDeleteConfirm'))) return
  if (deletingSkillIds.value.includes(skill.id)) return
  deletingSkillIds.value = [...deletingSkillIds.value, skill.id]
  try {
    const deleted = await invoke<boolean>('delete_skill', { id: skill.id })
    if (!deleted) {
      alert(t('agent.skillDeleteNotFound'))
      return
    }
    if (editingSkill.value?.id === skill.id) {
      cancelEdit()
    }
    await loadSkills()
    emit('changed')
  } catch (error) {
    console.error('Failed to delete skill:', error)
    alert(`${t('agent.skillDeleteFailed')}: ${error}`)
  } finally {
    deletingSkillIds.value = deletingSkillIds.value.filter(id => id !== skill.id)
  }
}

const applyTemplate = () => {
  if (!editingSkill.value) return
  if (editingSkill.value.content.trim() && !confirm(t('agent.skillTemplateOverwriteConfirm'))) {
    return
  }
  editingSkill.value.content = `## Purpose
- Describe what this skill does and when to use it.

## Inputs
- Specify expected inputs or assumptions.

## Steps
1. Step-by-step procedure or guidance.

## Output
- Expected output format or deliverables.

## Notes
- Constraints, tips, or safety considerations.
`
}

const runAiGeneration = async (prompt: string, systemPrompt: string) => {
  const streamId = crypto.randomUUID()
  let unlistenDelta: (() => void) | undefined
  let unlistenComplete: (() => void) | undefined
  let unlistenError: (() => void) | undefined

  aiGenerating.value = true
  generatedContent.value = ''

  try {
    unlistenDelta = await listen(`plugin_gen_delta`, (event) => {
      const payload = event.payload as any
      if (payload.stream_id === streamId) {
        generatedContent.value += payload.delta
      }
    })

    unlistenComplete = await listen(`plugin_gen_complete`, (event) => {
      const payload = event.payload as any
      if (payload.stream_id === streamId) {
        try {
          let jsonStr = generatedContent.value.trim()
          const jsonMatch = jsonStr.match(/```(?:json)?\s*(\{[\s\S]*?\})\s*```/)
          if (jsonMatch) {
            jsonStr = jsonMatch[1]
          } else if (jsonStr.startsWith('```')) {
            jsonStr = jsonStr.replace(/^```(?:json)?/, '').replace(/```$/, '')
          }

          const data = JSON.parse(jsonStr)
          if (editingSkill.value) {
            editingSkill.value.name = data.name || editingSkill.value.name
            editingSkill.value.description = data.description || editingSkill.value.description
            editingSkill.value.argument_hint = data.argument_hint || ''
            editingSkill.value.content = data.content || ''
            editingSkill.value.disable_model_invocation = !!data.disable_model_invocation
            editingSkill.value.user_invocable = data.user_invocable !== false
            editingSkill.value.model = data.model || ''
            editingSkill.value.context = data.context || ''
            editingSkill.value.agent = data.agent || ''
            editingSkill.value.hooks_raw = JSON.stringify(data.hooks || {}, null, 2)

            const invalidFields: string[] = []
            if (nameError.value) invalidFields.push(t('agent.skillName'))
            if (descriptionError.value) invalidFields.push(t('agent.skillDescription'))
            if (invalidFields.length > 0) {
              alert(t('agent.aiGeneratedInvalid', { fields: invalidFields.join(', ') }))
            }
          }
        } catch (e) {
          console.error('Failed to parse AI response', e)
        } finally {
          aiGenerating.value = false
          if (unlistenDelta) unlistenDelta()
          if (unlistenComplete) unlistenComplete()
          if (unlistenError) unlistenError()
        }
      }
    })

    unlistenError = await listen(`plugin_gen_error`, (event) => {
      const payload = event.payload as any
      if (payload.stream_id === streamId) {
        console.error('AI Generation Error:', payload.error)
        aiGenerating.value = false
        if (unlistenDelta) unlistenDelta()
        if (unlistenComplete) unlistenComplete()
        if (unlistenError) unlistenError()
      }
    })

    await invoke('generate_plugin_stream', {
      request: {
        stream_id: streamId,
        message: prompt,
        system_prompt: systemPrompt,
        service_name: 'default'
      }
    })
  } catch (e) {
    console.error(e)
    aiGenerating.value = false
    if (unlistenDelta) unlistenDelta()
    if (unlistenComplete) unlistenComplete()
    if (unlistenError) unlistenError()
  }
}

const generateWithAI = async () => {
  if (!editingSkill.value) return

  // Check if we can use AI (should be prevented by button disabled state, but double check)
  if (!canUseAI.value) {
    alert(t('agent.selectToolsOrAddContent'))
    return
  }

  const { prompt, systemPrompt } = buildSkillPrompt()
  await runAiGeneration(prompt, systemPrompt)
}

const buildSkillPrompt = () => {
  const brief = briefDescription.value.trim()
  const selectedTools = availableTools.value.filter(t => editingSkill.value?.allowed_tools.includes(t.id))
  const toolsContext = selectedTools.map(t => `- ${t.name}: ${t.description}`).join('\n')
  const hasExisting =
    editingSkill.value?.name.trim() ||
    editingSkill.value?.description.trim() ||
    editingSkill.value?.argument_hint.trim() ||
    editingSkill.value?.content.trim() ||
    editingSkill.value?.model.trim() ||
    editingSkill.value?.context.trim() ||
    editingSkill.value?.agent.trim() ||
    editingSkill.value?.hooks_raw.trim()

  if (brief) {
    return {
      prompt: `Generate a skill configuration based on the brief description and tools below.

Brief:
${brief}

Tools:
${toolsContext || '(none)'}

Existing Fields (if any, treat as constraints and keep if valid):
- Name: ${editingSkill.value?.name || '(empty)'}
- Description: ${editingSkill.value?.description || '(empty)'}
- Argument Hint: ${editingSkill.value?.argument_hint || '(empty)'}

Return ONLY a JSON object with this structure:
{
  "name": "skill-name-in-kebab-case",
  "description": "Third-person description: what it does and when to use it",
  "argument_hint": "Short argument hint if needed",
  "content": "Skill content in markdown",
  "disable_model_invocation": false,
  "user_invocable": true,
  "model": "",
  "context": "",
  "agent": "",
  "hooks": {}
}
Ensure the name is lowercase kebab-case, <= 64 chars, and avoid reserved words.`,
      systemPrompt: 'You are a helpful assistant that generates configuration for AI agent skills. Return valid JSON only.'
    }
  }

  const toolsOnly = toolsContext || '(none)'
  const hasExistingContent =
    editingSkill.value?.name.trim() ||
    editingSkill.value?.description.trim() ||
    editingSkill.value?.argument_hint.trim() ||
    editingSkill.value?.content.trim() ||
    editingSkill.value?.model.trim() ||
    editingSkill.value?.context.trim() ||
    editingSkill.value?.agent.trim() ||
    editingSkill.value?.hooks_raw.trim()

  if (hasExistingContent) {
    return {
      prompt: `Based on the following tools and existing content, please enhance and expand the skill configuration.

Tools:
${toolsOnly}

Existing Content:
- Name: ${editingSkill.value?.name || '(empty)'}
- Description: ${editingSkill.value?.description || '(empty)'}
- Argument Hint: ${editingSkill.value?.argument_hint || '(empty)'}
- Content: ${editingSkill.value?.content || '(empty)'}
- Model: ${editingSkill.value?.model || '(empty)'}
- Context: ${editingSkill.value?.context || '(empty)'}
- Agent: ${editingSkill.value?.agent || '(empty)'}
- Hooks (JSON): ${editingSkill.value?.hooks_raw || '(empty)'}

Please:
1. Keep the existing content that is good
2. Enhance empty or incomplete fields
3. Expand the skill content with more detail based on the tools
4. Keep hooks as valid JSON (return an object, or {} if none)

Return ONLY a JSON object with the following structure:
{
  "name": "Enhanced Skill Name",
  "description": "Enhanced short description (one sentence)",
  "argument_hint": "Short argument hint if needed",
  "content": "Enhanced skill content in markdown",
  "disable_model_invocation": false,
  "user_invocable": true,
  "model": "",
  "context": "",
  "agent": "",
  "hooks": {}
}
Ensure the content is in the same language as the existing content (or Chinese if unsure).
`,
      systemPrompt:
        'You are a helpful assistant that enhances and expands configuration for AI agent skills. Keep good existing content, enhance incomplete parts, and add missing details. Return valid JSON only.'
    }
  }

  return {
    prompt: `Based on the following tools, please generate a skill configuration.
Tools:
${toolsOnly}

Return ONLY a JSON object with the following structure:
{
  "name": "Skill Name",
  "description": "Short description (one sentence)",
  "argument_hint": "Short argument hint if needed",
  "content": "Skill content in markdown",
  "disable_model_invocation": false,
  "user_invocable": true,
  "model": "",
  "context": "",
  "agent": "",
  "hooks": {}
}
Ensure the content is in the same language as the tool descriptions (or Chinese if unsure).
`,
    systemPrompt: 'You are a helpful assistant that generates configuration for AI agent skills. Return valid JSON only.'
  }
}

const getCategoryDisplayName = (category: string) => {
  const nameMap: Record<string, string> = {
    'network': '网络',
    'security': '安全',
    'data': '数据',
    'ai': 'AI',
    'system': '系统',
    'mcp': 'MCP',
    'plugin': '插件',
    'workflow': '工作流',
    'browser': '浏览器',
  }
  return nameMap[category.toLowerCase()] || category
}

const getCategoryBadgeClass = (category: string) => {
  const map: Record<string, string> = {
    'network': 'badge-info',
    'security': 'badge-error',
    'data': 'badge-success',
    'ai': 'badge-warning',
    'system': 'badge-neutral',
    'mcp': 'badge-primary',
    'plugin': 'badge-secondary',
    'workflow': 'badge-accent',
    'browser': 'badge-primary',
  }
  return map[category.toLowerCase()] || 'badge-ghost'
}

const formatFileSize = (size: number) => {
  if (size < 1024) return `${size}B`
  if (size < 1024 * 1024) return `${(size / 1024).toFixed(1)}KB`
  return `${(size / (1024 * 1024)).toFixed(1)}MB`
}

const hashSkillId = (value: string) => {
  let hash = 0
  for (let i = 0; i < value.length; i += 1) {
    hash = (hash * 31 + value.charCodeAt(i)) >>> 0
  }
  return hash
}

const getSkillIcon = (id: string) => {
  const idx = hashSkillId(id) % skillIconPalette.length
  return skillIconPalette[idx].icon
}

const getSkillIconClass = (id: string) => {
  const idx = hashSkillId(id) % skillIconPalette.length
  return skillIconPalette[idx].cls
}

watch([toolSearchQuery, selectedCategory], () => {
  filterTools()
})

watch(availableTools, () => {
  filterTools()
})

onMounted(() => {
  loadSkills()
  loadTools()
})

defineExpose({
  refresh: async () => {
    await loadSkills()
    await loadTools()
  },
  startCreate
})
</script>

<style scoped>
.skills-manager {
  max-height: 85vh;
  overflow-y: auto;
}

/* 全屏模式 */
.skills-manager.fullscreen {
  max-height: calc(100vh - 2rem);
  height: calc(100vh - 2rem);
}

.skills-cards-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(260px, 1fr));
  gap: 0.75rem;
}

.skill-icon {
  width: 36px;
  height: 36px;
  border-radius: 10px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  font-size: 14px;
  flex-shrink: 0;
}

/* 响应式布局优化 */
@media (max-width: 1024px) {
  .skills-manager :deep(.grid-cols-5) {
    grid-template-columns: 1fr;
  }
}
</style>
