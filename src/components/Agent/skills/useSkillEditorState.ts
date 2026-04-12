import { computed, nextTick, ref, type Ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import {
  buildSkillTemplateContent,
  getCandidateArgumentHint,
  getRefineCandidateBrief,
  type TranslateFn,
} from '../skillsManagerHelpers'
import { useSkillAiGeneration } from './useSkillAiGeneration'
import { useSkillFiles } from './useSkillFiles'
import type { Skill, SkillCandidate, SkillForm } from './skillsManagerTypes'

interface UseSkillEditorStateOptions {
  t: TranslateFn
  locale: Ref<string>
  confirm: (message: string) => Promise<boolean>
  loadSkills: () => Promise<void>
  onChanged: () => void
}

export const useSkillEditorState = ({
  t,
  locale,
  confirm,
  loadSkills,
  onChanged,
}: UseSkillEditorStateOptions) => {
  const editingSkill = ref<SkillForm | null>(null)
  const isNewSkill = ref(false)
  const briefDescription = ref('')
  const candidateRefinementContext = ref<SkillCandidate | null>(null)

  const showSkillDialog = computed(() => !!editingSkill.value)

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

  const {
    skillFiles,
    loadingFiles,
    uploadingFiles,
    savingFile,
    selectedFilePath,
    fileContent,
    newFilePath,
    displayFiles,
    fileDirty,
    clearFileState,
    loadSkillFiles,
    refreshFiles,
    selectFile,
    saveFile,
    deleteFile,
    createFile,
    uploadFiles,
  } = useSkillFiles({
    t,
    confirm,
    editingSkill,
    isNewSkill,
  })

  const {
    aiGenerating,
    canUseAI,
    aiButtonText,
    generateWithAI,
  } = useSkillAiGeneration({
    t,
    locale,
    editingSkill,
    briefDescription,
    candidateRefinementContext,
    hasExistingContent,
    nameError,
    descriptionError,
  })

  const startCreate = () => {
    candidateRefinementContext.value = null
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
    clearFileState()
    briefDescription.value = ''
  }

  const useCandidateAsDraft = (candidate: SkillCandidate) => {
    candidateRefinementContext.value = candidate
    isNewSkill.value = true
    editingSkill.value = {
      id: '',
      name: candidate.suggested_skill_name,
      description: candidate.description,
      source_path: '',
      content: candidate.content,
      argument_hint: getCandidateArgumentHint(t, candidate.id),
      disable_model_invocation: false,
      user_invocable: true,
      allowed_tools: [],
      model: '',
      context: '',
      agent: '',
      hooks: {
        memory_candidate_id: candidate.id,
        memory_kind: candidate.memory_kind,
        memory_scope: candidate.scope,
        memory_source: candidate.source,
        memory_confidence: candidate.confidence,
      },
      hooks_raw: JSON.stringify({
        memory_candidate_id: candidate.id,
        memory_kind: candidate.memory_kind,
        memory_scope: candidate.scope,
        memory_source: candidate.source,
        memory_confidence: candidate.confidence,
      }, null, 2),
    }
    clearFileState()
    briefDescription.value = ''
  }

  const refineCandidateWithAI = async (candidate: SkillCandidate) => {
    if (aiGenerating.value) return
    useCandidateAsDraft(candidate)
    briefDescription.value = getRefineCandidateBrief(t, candidate.memory_kind)
    await nextTick()
    await generateWithAI()
  }

  const startEdit = async (skill: Skill) => {
    candidateRefinementContext.value = null
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
      allowed_tools: [],
      hooks: skill.hooks || {},
      hooks_raw: JSON.stringify(skill.hooks || {}, null, 2) || '',
    }
    briefDescription.value = ''
    await loadSkillFiles()
  }

  const cancelEdit = () => {
    candidateRefinementContext.value = null
    editingSkill.value = null
    isNewSkill.value = false
    clearFileState()
    briefDescription.value = ''
  }

  const handleSkillDialogClose = () => {
    if (editingSkill.value) {
      cancelEdit()
    }
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
            allowed_tools: [],
            model: editingSkill.value.model,
            context: editingSkill.value.context,
            agent: editingSkill.value.agent,
            hooks: hooksPayload,
          },
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
            allowed_tools: [],
            model: editingSkill.value.model,
            context: editingSkill.value.context,
            agent: editingSkill.value.agent,
            hooks: hooksPayload,
          },
        })
      }

      cancelEdit()
      await loadSkills()
      onChanged()
    } catch (error) {
      console.error('Failed to save skill:', error)
      alert(`${t('agent.skillSaveFailed')}: ${error}`)
    }
  }

  const applyTemplate = async () => {
    if (!editingSkill.value) return
    if (editingSkill.value.content.trim() && !(await confirm(t('agent.skillTemplateOverwriteConfirm')))) {
      return
    }
    editingSkill.value.content = buildSkillTemplateContent(t)
  }

  return {
    editingSkill,
    isNewSkill,
    aiGenerating,
    briefDescription,
    skillFiles,
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
    loadSkillFiles,
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
  }
}
