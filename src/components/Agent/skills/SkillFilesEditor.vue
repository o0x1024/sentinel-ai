<template>
  <div class="form-control">
    <label class="label py-1">
      <span class="label-text">{{ t('agent.skillFiles') }}</span>
      <span class="label-text-alt text-base-content/60">{{ t('agent.skillFilesHint') }}</span>
    </label>
    <div v-if="isNewSkill" class="text-xs text-base-content/60">
      {{ t('agent.skillFilesNeedSaveFirst') }}
    </div>
    <div v-else class="space-y-2">
      <div class="flex flex-wrap items-center gap-2">
        <input
          :value="newFilePath"
          type="text"
          class="input input-xs input-bordered min-w-[160px] flex-1"
          :placeholder="t('agent.skillFilesNewPath')"
          @input="$emit('update:new-file-path', ($event.target as HTMLInputElement).value)"
        />
        <button
          @click="$emit('create-file')"
          class="btn btn-xs btn-primary btn-outline"
          :disabled="!newFilePath.trim()"
        >
          <i class="fas fa-plus"></i>
          {{ t('agent.skillFilesCreate') }}
        </button>
        <button
          @click="$emit('upload-files')"
          class="btn btn-xs btn-outline"
          :disabled="uploadingFiles"
        >
          <i class="fas fa-upload"></i>
          {{ t('agent.skillFilesUpload') }}
        </button>
        <button @click="$emit('refresh-files')" class="btn btn-xs btn-ghost">
          <i class="fas fa-sync-alt"></i>
        </button>
      </div>

      <div class="grid grid-cols-1 gap-2 md:grid-cols-2">
        <div class="max-h-44 overflow-y-auto rounded-lg border border-base-300 bg-base-100">
          <div v-if="loadingFiles" class="flex justify-center py-4">
            <span class="loading loading-spinner loading-sm"></span>
          </div>
          <div v-else>
            <button
              v-for="file in displayFiles"
              :key="file.path"
              class="w-full px-2 py-1 text-left text-xs transition-colors hover:bg-base-200"
              :class="{ 'bg-base-200': selectedFilePath === file.path }"
              @click="$emit('select-file', file)"
            >
              <div class="flex items-center justify-between gap-2">
                <span class="truncate">{{ file.path }}</span>
                <span class="text-[10px] text-base-content/60">{{ formatFileSize(file.size) }}</span>
              </div>
            </button>
            <div v-if="displayFiles.length === 0" class="py-4 text-center text-xs text-base-content/60">
              {{ t('agent.skillFilesEmpty') }}
            </div>
          </div>
        </div>

        <div class="rounded-lg border border-base-300 bg-base-100 p-2">
          <div v-if="!selectedFilePath" class="text-xs text-base-content/60">
            {{ t('agent.skillFilesSelect') }}
          </div>
          <div v-else class="space-y-2">
            <div class="truncate font-mono text-xs text-base-content/70">{{ selectedFilePath }}</div>
            <textarea
              :value="fileContent"
              class="textarea textarea-bordered h-28 w-full font-mono text-xs"
              :placeholder="t('agent.skillFilesContentPlaceholder')"
              @input="$emit('update:file-content', ($event.target as HTMLTextAreaElement).value)"
            ></textarea>
            <div class="flex justify-end gap-2">
              <button
                @click="$emit('save-file')"
                class="btn btn-xs btn-primary"
                :disabled="!fileDirty || savingFile"
              >
                <i class="fas fa-save"></i>
                {{ t('agent.skillFilesSave') }}
              </button>
              <button
                @click="$emit('delete-file')"
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
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { formatFileSize } from '../skillsManagerHelpers'
import type { SkillFileEntry } from './skillsManagerTypes'

defineProps<{
  isNewSkill: boolean
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
