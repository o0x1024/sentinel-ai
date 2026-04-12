<template>
  <AppDialog
    :class="['modal', { 'modal-open': open }]"
    @click.self="$emit('close')"
    @close="$emit('close')"
  >
    <div v-if="skill" class="modal-box w-11/12 max-w-6xl p-0">
      <div class="flex max-h-[calc(100vh-5rem)] flex-col">
        <div class="flex items-start justify-between gap-4 border-b border-base-300 px-6 py-4">
          <div>
            <h3 class="flex items-center gap-2 text-lg font-semibold">
              <i :class="isNewSkill ? 'fas fa-plus text-primary' : 'fas fa-edit text-primary'"></i>
              {{ isNewSkill ? t('agent.createSkill') : t('common.edit') }}
            </h3>
            <div class="mt-1 text-sm text-base-content/60">
              {{ isNewSkill ? t('agent.skillManagement') : (skill.name || skill.id) }}
            </div>
          </div>
          <button @click="$emit('close')" class="btn btn-sm btn-ghost btn-circle">
            <i class="fas fa-times"></i>
          </button>
        </div>

        <div class="space-y-3 overflow-y-auto px-6 py-5">
          <div class="form-control">
            <label class="label py-1">
              <span class="label-text">{{ t('agent.skillName') }}</span>
            </label>
            <input
              v-model="skill.name"
              type="text"
              class="input input-sm input-bordered w-full"
              :placeholder="t('agent.skillNamePlaceholder')"
            />
            <div class="mt-1 text-xs text-base-content/60">
              {{ t('agent.skillNameBestPractice') }}
            </div>
            <div v-if="nameError" class="mt-1 text-xs text-error">
              <i class="fas fa-exclamation-triangle mr-1"></i>
              {{ nameError }}
            </div>
          </div>

          <div class="form-control">
            <label class="label py-1">
              <span class="label-text">{{ t('agent.skillDescription') }}</span>
            </label>
            <input
              v-model="skill.description"
              type="text"
              class="input input-sm input-bordered w-full"
              :placeholder="t('agent.skillDescriptionPlaceholder')"
            />
            <div class="mt-1 text-xs text-base-content/60">
              {{ t('agent.skillDescriptionBestPractice') }}
            </div>
            <div v-if="descriptionError" class="mt-1 text-xs text-error">
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
              v-model="skill.argument_hint"
              type="text"
              class="input input-sm input-bordered w-full"
              :placeholder="t('agent.argumentHintPlaceholder')"
            />
          </div>

          <div class="grid grid-cols-1 gap-2 md:grid-cols-2">
            <label class="label cursor-pointer justify-start gap-3">
              <input
                v-model="skill.user_invocable"
                type="checkbox"
                class="checkbox checkbox-sm checkbox-primary"
              />
              <span class="label-text">{{ t('agent.userInvocable') }}</span>
            </label>
            <label class="label cursor-pointer justify-start gap-3">
              <input
                v-model="skill.disable_model_invocation"
                type="checkbox"
                class="checkbox checkbox-sm checkbox-primary"
              />
              <span class="label-text">{{ t('agent.disableModelInvocation') }}</span>
            </label>
          </div>

          <div class="grid grid-cols-1 gap-2 md:grid-cols-3">
            <div class="form-control">
              <label class="label py-1">
                <span class="label-text">{{ t('agent.skillModel') }}</span>
              </label>
              <input
                v-model="skill.model"
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
                v-model="skill.context"
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
                v-model="skill.agent"
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
              v-model="skill.content"
              class="textarea textarea-bordered h-36 w-full text-sm"
              :placeholder="t('agent.skillContentPlaceholder')"
            ></textarea>
            <div class="mt-1 text-xs text-base-content/60">
              {{ t('agent.skillContentBestPractice') }}
            </div>
            <div class="flex justify-end">
              <button @click="$emit('apply-template')" class="btn btn-xs btn-ghost">
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
              v-model="skill.hooks_raw"
              class="textarea textarea-bordered h-28 w-full text-sm"
              :placeholder="t('agent.skillHooksPlaceholder')"
            ></textarea>
            <div v-if="!hooksValid" class="mt-1 text-xs text-error">
              <i class="fas fa-exclamation-triangle mr-1"></i>
              {{ t('agent.skillHooksInvalid') }}
            </div>
          </div>

          <SkillFilesEditor
            :is-new-skill="isNewSkill"
            :loading-files="loadingFiles"
            :uploading-files="uploadingFiles"
            :saving-file="savingFile"
            :new-file-path="newFilePath"
            :display-files="displayFiles"
            :selected-file-path="selectedFilePath"
            :file-content="fileContent"
            :file-dirty="fileDirty"
            @update:new-file-path="$emit('update:new-file-path', $event)"
            @update:file-content="$emit('update:file-content', $event)"
            @create-file="$emit('create-file')"
            @upload-files="$emit('upload-files')"
            @refresh-files="$emit('refresh-files')"
            @select-file="$emit('select-file', $event)"
            @save-file="$emit('save-file')"
            @delete-file="$emit('delete-file')"
          />

          <div class="space-y-2 pt-2">
            <div class="form-control">
              <label class="label py-1">
                <span class="label-text">{{ t('agent.skillBrief') }}</span>
                <span class="label-text-alt text-base-content/60">{{ t('agent.skillBriefHint') }}</span>
              </label>
              <input
                :value="briefDescription"
                type="text"
                class="input input-sm input-bordered w-full"
                :placeholder="t('agent.skillBriefPlaceholder')"
                @input="$emit('update:brief-description', ($event.target as HTMLInputElement).value)"
              />
            </div>
            <button
              @click="$emit('generate-ai')"
              class="btn btn-sm btn-primary btn-outline w-full gap-1"
              :disabled="!canUseAi || aiGenerating"
              :class="{ 'btn-disabled': !canUseAi }"
              :title="!canUseAi ? t('agent.selectToolsOrAddContent') : ''"
            >
              <span v-if="aiGenerating" class="loading loading-spinner loading-xs"></span>
              <i v-else class="fas fa-magic"></i>
              {{ aiButtonText }}
            </button>
            <div v-if="!canUseAi" class="text-center text-xs text-warning">
              <i class="fas fa-info-circle mr-1"></i>
              {{ t('agent.aiGenerateHint') }}
            </div>
          </div>
        </div>

        <div class="flex justify-end gap-2 border-t border-base-300 px-6 py-4">
          <button @click="$emit('close')" class="btn btn-sm btn-ghost">
            {{ t('common.cancel') }}
          </button>
          <button @click="$emit('save')" class="btn btn-sm btn-primary" :disabled="!canSave">
            <i class="fas fa-save"></i>
            {{ isNewSkill ? t('common.create') : t('common.save') }}
          </button>
        </div>
      </div>
    </div>
  </AppDialog>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import SkillFilesEditor from './SkillFilesEditor.vue'
import type { SkillFileEntry, SkillForm } from './skillsManagerTypes'

defineProps<{
  open: boolean
  skill: SkillForm | null
  isNewSkill: boolean
  nameError: string
  descriptionError: string
  hooksValid: boolean
  canSave: boolean
  briefDescription: string
  canUseAi: boolean
  aiGenerating: boolean
  aiButtonText: string
  loadingFiles: boolean
  uploadingFiles: boolean
  savingFile: boolean
  newFilePath: string
  displayFiles: SkillFileEntry[]
  selectedFilePath: string
  fileContent: string
  fileDirty: boolean
}>()

defineEmits<{
  'close': []
  'save': []
  'apply-template': []
  'generate-ai': []
  'update:brief-description': [value: string]
  'update:new-file-path': [value: string]
  'update:file-content': [value: string]
  'create-file': []
  'upload-files': []
  'refresh-files': []
  'select-file': [file: SkillFileEntry]
  'save-file': []
  'delete-file': []
}>()

const { t } = useI18n()
</script>
