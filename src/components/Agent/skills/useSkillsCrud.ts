import { computed, ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { TranslateFn } from '../skillsManagerHelpers'
import type { Skill, SkillForm } from './skillsManagerTypes'

interface UseSkillsCrudOptions {
  t: TranslateFn
  confirm: (message: string) => Promise<boolean>
  getEditingSkill: () => SkillForm | null
  cancelEdit: () => void
  onChanged: () => void
}

export const useSkillsCrud = ({
  t,
  confirm,
  getEditingSkill,
  cancelEdit,
  onChanged,
}: UseSkillsCrudOptions) => {
  const skills = ref<Skill[]>([])
  const skillEnabledMap = ref<Record<string, boolean>>({})
  const loading = ref(false)
  const deletingSkillIds = ref<string[]>([])
  const bulkUpdatingSkillState = ref(false)
  const bulkDeletingSkills = ref(false)

  const enabledSkillCount = computed(() =>
    skills.value.filter(skill => isSkillEnabled(skill.id)).length
  )

  const disabledSkillCount = computed(() =>
    skills.value.length - enabledSkillCount.value
  )

  const skillsWithContentCount = computed(() =>
    skills.value.filter(skill => !!skill.content).length
  )

  const loadSkillEnabledMap = async () => {
    try {
      const configs = await invoke<Array<{ key: string; value: string }>>('get_config', {
        request: { category: 'skills', key: null },
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
        value: enabled ? 'true' : 'false',
      })
      skillEnabledMap.value = { ...skillEnabledMap.value, [id]: enabled }
    } catch (error) {
      console.error('Failed to save skill enabled setting:', error)
    }
  }

  const setAllSkillsEnabled = async (enabled: boolean) => {
    if (skills.value.length === 0 || bulkUpdatingSkillState.value) return
    bulkUpdatingSkillState.value = true
    const next = { ...skillEnabledMap.value }
    try {
      await Promise.all(skills.value.map(skill =>
        invoke('set_config', {
          category: 'skills',
          key: `enabled::${skill.id}`,
          value: enabled ? 'true' : 'false',
        })
      ))
      for (const skill of skills.value) {
        next[skill.id] = enabled
      }
      skillEnabledMap.value = next
    } catch (error) {
      console.error('Failed to save all skill enabled settings:', error)
    } finally {
      bulkUpdatingSkillState.value = false
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

  const confirmDelete = async (skill: Skill) => {
    if (!(await confirm(t('agent.skillDeleteConfirm')))) return
    if (deletingSkillIds.value.includes(skill.id)) return
    deletingSkillIds.value = [...deletingSkillIds.value, skill.id]
    try {
      const deleted = await invoke<boolean>('delete_skill', { id: skill.id })
      if (!deleted) {
        alert(t('agent.skillDeleteNotFound'))
        return
      }
      if (getEditingSkill()?.id === skill.id) {
        cancelEdit()
      }
      await loadSkills()
      onChanged()
    } catch (error) {
      console.error('Failed to delete skill:', error)
      alert(`${t('agent.skillDeleteFailed')}: ${error}`)
    } finally {
      deletingSkillIds.value = deletingSkillIds.value.filter(id => id !== skill.id)
    }
  }

  const deleteSkillsByIds = async (ids: string[], confirmMessage: string) => {
    const uniqueIds = Array.from(new Set(ids)).filter(Boolean)
    if (uniqueIds.length === 0 || bulkDeletingSkills.value) return
    if (!(await confirm(confirmMessage))) return

    bulkDeletingSkills.value = true
    deletingSkillIds.value = Array.from(new Set([...deletingSkillIds.value, ...uniqueIds]))
    try {
      const results = await Promise.all(uniqueIds.map(id => invoke<boolean>('delete_skill', { id })))
      if (results.some(deleted => !deleted)) {
        throw new Error(t('agent.skillDeleteNotFound'))
      }
      const editingSkill = getEditingSkill()
      if (editingSkill && uniqueIds.includes(editingSkill.id)) {
        cancelEdit()
      }
      await loadSkills()
      onChanged()
    } catch (error) {
      console.error('Failed to delete skills:', error)
      alert(`${t('agent.skillDeleteFailed')}: ${error}`)
    } finally {
      deletingSkillIds.value = deletingSkillIds.value.filter(id => !uniqueIds.includes(id))
      bulkDeletingSkills.value = false
    }
  }

  const confirmDeleteSelected = async (ids: string[]) => {
    await deleteSkillsByIds(ids, t('agent.skillBatchDeleteConfirm', { count: ids.length }))
  }

  const confirmDeleteAll = async () => {
    await deleteSkillsByIds(
      skills.value.map(skill => skill.id),
      t('agent.skillDeleteAllConfirm', { count: skills.value.length })
    )
  }

  return {
    skills,
    loading,
    deletingSkillIds,
    bulkUpdatingSkillState,
    bulkDeletingSkills,
    enabledSkillCount,
    disabledSkillCount,
    skillsWithContentCount,
    isSkillEnabled,
    toggleSkillEnabled,
    setAllSkillsEnabled,
    loadSkills,
    confirmDelete,
    confirmDeleteSelected,
    confirmDeleteAll,
  }
}
