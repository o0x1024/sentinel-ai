<template>
  <div :class="['skills-manager', { 'fullscreen': isFullscreen }]">
    <SkillEditorDialog
      :open="showSkillDialog"
      :skill="editingSkill"
      :is-new-skill="isNewSkill"
      :name-error="nameError"
      :description-error="descriptionError"
      :hooks-valid="hooksValid"
      :can-save="canSave"
      :brief-description="briefDescription"
      :can-use-ai="canUseAI"
      :ai-generating="aiGenerating"
      :ai-button-text="aiButtonText"
      :loading-files="loadingFiles"
      :uploading-files="uploadingFiles"
      :saving-file="savingFile"
      :new-file-path="newFilePath"
      :display-files="displayFiles"
      :selected-file-path="selectedFilePath"
      :file-content="fileContent"
      :file-dirty="fileDirty"
      @close="handleSkillDialogClose"
      @save="saveSkill"
      @apply-template="applyTemplate"
      @generate-ai="generateWithAI"
      @update:brief-description="briefDescription = $event"
      @update:new-file-path="newFilePath = $event"
      @update:file-content="fileContent = $event"
      @create-file="createFile"
      @upload-files="uploadFiles"
      @refresh-files="refreshFiles"
      @select-file="selectFile"
      @save-file="saveFile"
      @delete-file="deleteFile"
    />
    <AppDialog :class="['modal', { 'modal-open': showMemoryFeedbackDialog }]">
      <div class="modal-box w-11/12 max-w-6xl">
        <div class="mb-4 flex items-start justify-between gap-3">
          <div>
            <h3 class="text-lg font-semibold">{{ t('agent.memoryFeedbackTitle') }}</h3>
            <p class="text-sm text-base-content/60">
              {{ t('agent.memoryFeedbackDescription') }}
            </p>
          </div>
          <button class="btn btn-sm btn-ghost btn-circle" @click="closeMemoryFeedbackDialog">
            <i class="fas fa-times"></i>
          </button>
        </div>
        <div class="max-h-[70vh] space-y-4 overflow-y-auto pr-1">
          <MemoryFeedbackSummary
            :skills-count="skills.length"
            :total-draft-count="skillCandidates.length"
            :active-draft-count="activeSkillCandidates.length"
            :reviewed-draft-count="reviewedSkillCandidateCount"
            :rule-count="suppressionRules.length"
            :total-rule-hits="suppressionRuleTotalHits"
            :loading-candidates="loadingCandidates"
            :show-candidate-details="showCandidateDetails"
            :show-suppression-rule-details="showSuppressionRuleDetails"
            @refresh-candidates="loadSkillCandidates"
            @refresh-rules="loadSuppressionRules"
            @toggle-candidate-details="toggleCandidateDetails"
            @toggle-rule-details="toggleSuppressionRuleDetails"
          />

          <div
            v-if="showCandidateDetails && (skillCandidates.length > 0 || loadingCandidates)"
          >
            <CandidateDraftPanel
              :loading-candidates="loadingCandidates"
              :show-reviewed-candidates="showReviewedCandidates"
              :visible-skill-candidates="visibleSkillCandidates"
              :promoting-candidate-ids="promotingCandidateIds"
              :reviewing-candidate-ids="reviewingCandidateIds"
              :ai-generating="aiGenerating"
              :rejection-category-options="rejectionCategoryOptions"
              @update:show-reviewed-candidates="showReviewedCandidates = $event"
              @use-draft="useCandidateAsDraft"
              @refine-ai="refineCandidateWithAI"
              @promote="promoteCandidate"
              @review="reviewCandidate"
            />
          </div>

          <div
            v-if="showSuppressionRuleDetails && suppressionRules.length > 0"
          >
            <SuppressionRulePanel
              :suppression-rules="suppressionRules"
              :action-by-id="suppressionRuleActionById"
              @refresh="loadSuppressionRules"
              @extend="extendSuppressionRule"
              @expire="expireSuppressionRule"
              @delete="deleteSuppressionRule"
            />
          </div>
        </div>
        <div class="modal-action">
          <button class="btn" @click="closeMemoryFeedbackDialog">{{ t('common.close') }}</button>
        </div>
      </div>
      <form method="dialog" class="modal-backdrop bg-black/50" @click="closeMemoryFeedbackDialog">
        <button>{{ t('common.close') }}</button>
      </form>
    </AppDialog>
    <div class="space-y-4">
      <SkillLibraryPanel
        :loading="loading"
        :skills="skills"
        :view-mode="props.viewMode"
        :current-view-mode-label="currentViewModeLabel"
        :enabled-skill-count="enabledSkillCount"
        :disabled-skill-count="disabledSkillCount"
        :skills-with-content-count="skillsWithContentCount"
        :active-draft-count="activeSkillCandidates.length"
        :rule-count="suppressionRules.length"
        :bulk-updating-skill-state="bulkUpdatingSkillState"
        :deleting-skill-ids="deletingSkillIds"
        :is-skill-enabled="isSkillEnabled"
        :get-skill-icon="getSkillIcon"
        :get-skill-icon-class="getSkillIconClass"
        @start-create="startCreate"
        @edit="startEdit"
        @delete="confirmDelete"
        @toggle-enabled="toggleSkillEnabled"
        @set-all-enabled="setAllSkillsEnabled"
        @open-memory-feedback="openMemoryFeedbackDialog"
      />
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import AppDialog from '@/components/AppDialog.vue'
import { dialog } from '../../composables/useDialog'
import SkillEditorDialog from './skills/SkillEditorDialog.vue'
import MemoryFeedbackSummary from './skills/MemoryFeedbackSummary.vue'
import CandidateDraftPanel from './skills/CandidateDraftPanel.vue'
import SuppressionRulePanel from './skills/SuppressionRulePanel.vue'
import SkillLibraryPanel from './skills/SkillLibraryPanel.vue'
import { useSkillsCrud } from './skills/useSkillsCrud'
import { useSkillCandidates } from './skills/useSkillCandidates'
import { useSuppressionRules } from './skills/useSuppressionRules'
import { useSkillEditorState } from './skills/useSkillEditorState'
import {
  createRejectionCategoryOptions,
  getCurrentViewModeLabel,
  hashSkillId,
} from './skillsManagerHelpers'

interface Props {
  isFullscreen?: boolean
  embedded?: boolean
  viewMode?: 'card' | 'list'
}

const props = withDefaults(defineProps<Props>(), {
  isFullscreen: false,
  embedded: false,
  viewMode: 'card'
})

const emit = defineEmits<{
  'close': []
  'changed': []
  'toggle-fullscreen': []
}>()

const { t, locale } = useI18n()

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
const showMemoryFeedbackDialog = ref(false)

const rejectionCategoryOptions = computed(() => createRejectionCategoryOptions(t))

const currentViewModeLabel = computed(() =>
  getCurrentViewModeLabel(t, props.viewMode)
)

const {
  skills,
  loading,
  deletingSkillIds,
  bulkUpdatingSkillState,
  enabledSkillCount,
  disabledSkillCount,
  skillsWithContentCount,
  isSkillEnabled,
  toggleSkillEnabled,
  setAllSkillsEnabled,
  loadSkills,
  confirmDelete,
} = useSkillsCrud({
  t,
  confirm: dialog.confirm,
  getEditingSkill: () => editingSkill.value,
  cancelEdit: () => cancelEdit(),
  onChanged: () => emit('changed'),
})

const {
  suppressionRules,
  suppressionRuleActionById,
  showSuppressionRuleDetails,
  suppressionRuleTotalHits,
  toggleSuppressionRuleDetails,
  loadSuppressionRules,
  extendSuppressionRule,
  expireSuppressionRule,
  deleteSuppressionRule,
} = useSuppressionRules({
  t,
  confirm: dialog.confirm,
})

const {
  skillCandidates,
  loadingCandidates,
  showReviewedCandidates,
  promotingCandidateIds,
  reviewingCandidateIds,
  showCandidateDetails,
  activeSkillCandidates,
  reviewedSkillCandidateCount,
  visibleSkillCandidates,
  toggleCandidateDetails,
  loadSkillCandidates,
  promoteCandidate,
  reviewCandidate,
} = useSkillCandidates({
  t,
  confirm: dialog.confirm,
  loadSkills,
  loadSuppressionRules,
  onChanged: () => emit('changed'),
})

const {
  editingSkill,
  isNewSkill,
  aiGenerating,
  briefDescription,
  loadingFiles,
  uploadingFiles,
  savingFile,
  selectedFilePath,
  fileContent,
  newFilePath,
  showSkillDialog,
  canUseAI,
  aiButtonText,
  hooksValid,
  nameError,
  descriptionError,
  canSave,
  displayFiles,
  fileDirty,
  refreshFiles,
  selectFile,
  saveFile,
  deleteFile,
  createFile,
  uploadFiles,
  startCreate,
  useCandidateAsDraft,
  refineCandidateWithAI,
  startEdit,
  cancelEdit,
  handleSkillDialogClose,
  saveSkill,
  applyTemplate,
  generateWithAI,
} = useSkillEditorState({
  t,
  locale,
  confirm: dialog.confirm,
  loadSkills,
  onChanged: () => emit('changed'),
})

const refreshAllData = async () => {
  await Promise.all([loadSkills(), loadSkillCandidates(), loadSuppressionRules()])
}

const getSkillIcon = (id: string) => {
  const idx = hashSkillId(id) % skillIconPalette.length
  return skillIconPalette[idx].icon
}

const getSkillIconClass = (id: string) => {
  const idx = hashSkillId(id) % skillIconPalette.length
  return skillIconPalette[idx].cls
}

const openMemoryFeedbackDialog = () => {
  showMemoryFeedbackDialog.value = true
}

const closeMemoryFeedbackDialog = () => {
  showMemoryFeedbackDialog.value = false
}

onMounted(() => {
  loadSkills()
  loadSkillCandidates()
  loadSuppressionRules()
})

defineExpose({
  refresh: refreshAllData,
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
</style>
