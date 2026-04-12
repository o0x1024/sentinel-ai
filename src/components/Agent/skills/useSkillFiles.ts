import { computed, ref, type Ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { TranslateFn } from '../skillsManagerHelpers'
import type { SkillFileEntry, SkillForm } from './skillsManagerTypes'

interface UseSkillFilesOptions {
  t: TranslateFn
  confirm: (message: string) => Promise<boolean>
  editingSkill: Ref<SkillForm | null>
  isNewSkill: Ref<boolean>
}

export const useSkillFiles = ({
  t,
  confirm,
  editingSkill,
  isNewSkill,
}: UseSkillFilesOptions) => {
  const skillFiles = ref<SkillFileEntry[]>([])
  const loadingFiles = ref(false)
  const uploadingFiles = ref(false)
  const savingFile = ref(false)
  const selectedFilePath = ref('')
  const fileContent = ref('')
  const fileOriginal = ref('')
  const newFilePath = ref('')

  const displayFiles = computed(() =>
    skillFiles.value.filter(file => file.path.toLowerCase() !== 'skill.md')
  )

  const fileDirty = computed(() => fileContent.value !== fileOriginal.value)

  const resetFileEditor = () => {
    selectedFilePath.value = ''
    fileContent.value = ''
    fileOriginal.value = ''
    newFilePath.value = ''
  }

  const clearFileState = () => {
    skillFiles.value = []
    resetFileEditor()
  }

  const loadSkillFiles = async () => {
    if (!editingSkill.value || isNewSkill.value) {
      clearFileState()
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
        path: file.path,
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
        content: fileContent.value,
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
    if (!(await confirm(t('agent.skillFilesDeleteConfirm')))) return
    try {
      await invoke('delete_skill_file', {
        id: editingSkill.value.id,
        path: selectedFilePath.value,
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
        content: '',
      })
      newFilePath.value = ''
      await loadSkillFiles()
      const created = skillFiles.value.find(file => file.path === path)
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
          { name: t('agent.textFiles'), extensions: ['md', 'txt', 'json', 'yml', 'yaml'] },
        ],
      })
      const paths = Array.isArray(selected) ? selected : selected ? [selected] : []
      if (paths.length === 0) return
      uploadingFiles.value = true
      for (const path of paths) {
        await invoke('import_skill_file', {
          id: editingSkill.value.id,
          sourcePath: path,
        })
      }
      await loadSkillFiles()
    } catch (error) {
      console.error('Failed to upload files:', error)
    } finally {
      uploadingFiles.value = false
    }
  }

  return {
    skillFiles,
    loadingFiles,
    uploadingFiles,
    savingFile,
    selectedFilePath,
    fileContent,
    newFilePath,
    displayFiles,
    fileDirty,
    resetFileEditor,
    clearFileState,
    loadSkillFiles,
    refreshFiles,
    selectFile,
    saveFile,
    deleteFile,
    createFile,
    uploadFiles,
  }
}
