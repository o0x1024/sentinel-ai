<template>
  <div class="space-y-4">
    <div
      v-if="isDragOver && skillsEnabled"
      class="fixed inset-0 z-[1000] pointer-events-none flex items-center justify-center bg-primary/10 border-2 border-dashed border-primary"
    >
      <div class="px-5 py-3 rounded-lg bg-base-100 shadow-lg text-sm font-medium text-base-content">
        {{ $t('Tools.skillsDropInstallHint') }}
      </div>
    </div>

    <div class="flex items-center justify-between">
      <div>
        <h2 class="text-2xl font-semibold">{{ $t('Tools.skills') }}</h2>
        <p class="text-sm text-base-content/70">{{ $t('Tools.skillsDescription') }}</p>
      </div>
      <div class="flex items-center gap-3">
        <div class="flex items-center gap-2 px-3 py-1 rounded border border-base-300 bg-base-100">
          <span class="text-sm">{{ $t('Tools.skillsEnabledLabel') }}</span>
          <input
            type="checkbox"
            class="toggle toggle-sm"
            v-model="skillsEnabled"
            @change="persistSkillsEnabled"
          />
          <span class="text-xs text-base-content/60">
            {{ skillsEnabled ? $t('common.enabled') : $t('common.disabled') }}
          </span>
        </div>
        <button @click="openGitInstall" class="btn btn-sm btn-outline" :disabled="installLoading || !skillsEnabled">
          <i class="fab fa-github mr-2"></i>
          {{ $t('Tools.skillsInstallFromGithub') }}
        </button>
        <button @click="importFromFile" class="btn btn-sm btn-outline" :disabled="installLoading || !skillsEnabled">
          <i class="fas fa-file-import mr-2"></i>
          {{ $t('Tools.skillsImportFile') }}
        </button>
        <button @click="importFromFolder" class="btn btn-sm btn-outline" :disabled="installLoading || !skillsEnabled">
          <i class="fas fa-folder-open mr-2"></i>
          {{ $t('Tools.skillsImportFolder') }}
        </button>
        <button @click="refresh" class="btn btn-sm btn-outline" :disabled="installLoading || !skillsEnabled">
          <i class="fas fa-sync-alt mr-2"></i>
          {{ $t('common.refresh') }}
        </button>
        <button @click="createSkill" class="btn btn-sm btn-primary btn-outline" :disabled="installLoading || !skillsEnabled">
          <i class="fas fa-plus mr-2"></i>
          {{ $t('agent.createSkill') }}
        </button>
      </div>
    </div>

    <div class="card bg-base-200 p-4">
      <div v-if="skillsEnabled">
        <SkillsManager ref="skillsManagerRef" :embedded="true" @changed="handleSkillsChanged" />
      </div>
      <div v-else class="alert alert-warning">
        <i class="fas fa-exclamation-triangle"></i>
        <span>{{ $t('Tools.skillsDisabledWarning') }}</span>
      </div>
    </div>

    <div class="card bg-base-200 p-4">
      <div class="flex items-center justify-between mb-2">
        <h3 class="font-semibold">{{ $t('Tools.skillsInstallHistory') }}</h3>
        <button @click="loadHistory" class="btn btn-xs btn-ghost">
          <i class="fas fa-sync-alt"></i>
        </button>
      </div>
      <div v-if="historyLoading" class="flex justify-center py-4">
        <span class="loading loading-spinner loading-sm"></span>
      </div>
      <div v-else-if="installHistory.length === 0" class="text-sm text-base-content/60">
        {{ $t('Tools.skillsInstallHistoryEmpty') }}
      </div>
      <div v-else class="space-y-2">
        <div v-for="record in installHistory" :key="record.id" class="p-2 rounded border border-base-300 bg-base-100 text-sm">
          <div class="flex items-center justify-between">
            <div class="font-mono text-xs">{{ record.timestamp }}</div>
            <div class="flex items-center gap-2">
              <span class="badge badge-xs" :class="record.status === 'success' ? 'badge-success' : 'badge-error'">
                {{ record.status }}
              </span>
              <button @click="deleteHistory(record.id)" class="btn btn-xs btn-ghost text-error">
                <i class="fas fa-trash"></i>
              </button>
            </div>
          </div>
          <div class="mt-1">
            <span class="font-medium">{{ record.source_type }}</span>
            <span class="text-base-content/60 ml-2">{{ record.source }}</span>
          </div>
          <div class="text-base-content/70 mt-1">
            {{ record.skills.join(', ') }}
          </div>
          <div v-if="record.message" class="text-error text-xs mt-1">
            {{ record.message }}
          </div>
        </div>
      </div>
    </div>

    <dialog :class="['modal', { 'modal-open': showInstallModal }]">
      <div class="modal-box w-11/12 max-w-2xl">
        <div class="flex items-center justify-between mb-3">
          <h3 class="font-bold text-lg">{{ $t('Tools.skillsInstallSelect') }}</h3>
          <button @click="closeInstallModal" class="btn btn-sm btn-ghost">✕</button>
        </div>
        <div v-if="installLoading" class="flex justify-center py-6">
          <span class="loading loading-spinner loading-md"></span>
        </div>
        <div v-else>
          <div v-if="installCandidates.length === 0" class="text-sm text-base-content/60">
            {{ $t('Tools.skillsInstallNoCandidates') }}
          </div>
          <div v-else class="space-y-2 max-h-64 overflow-y-auto">
            <label v-for="skill in installCandidates" :key="skill.id" class="flex items-start gap-2 p-2 rounded hover:bg-base-200 cursor-pointer">
              <input
                type="checkbox"
                class="checkbox checkbox-sm checkbox-primary mt-1"
                v-model="selectedInstallSkills"
                :value="skill.id"
              />
              <div class="min-w-0">
                <div class="font-medium">{{ skill.name }}</div>
                <div class="text-xs text-base-content/60">{{ skill.description }}</div>
                <div class="text-xs font-mono text-base-content/50">{{ skill.id }}</div>
              </div>
            </label>
          </div>
        </div>
        <div class="modal-action">
          <button @click="closeInstallModal" class="btn">{{ $t('common.cancel') }}</button>
          <button @click="installSelected" class="btn btn-primary" :disabled="installCandidates.length > 1 && selectedInstallSkills.length === 0">
            {{ $t('Tools.skillsInstallConfirm') }}
          </button>
        </div>
      </div>
    </dialog>

    <dialog :class="['modal', { 'modal-open': showGitModal }]">
      <div class="modal-box w-11/12 max-w-2xl">
        <div class="flex items-center justify-between mb-3">
          <h3 class="font-bold text-lg">{{ $t('Tools.skillsInstallFromGithub') }}</h3>
          <button @click="closeGitModal" class="btn btn-sm btn-ghost">✕</button>
        </div>
        <div class="form-control">
          <label class="label">
            <span class="label-text">{{ $t('Tools.skillsGitUrl') }}</span>
          </label>
          <input v-model="gitUrl" class="input input-bordered" :placeholder="$t('Tools.skillsGitUrlPlaceholder')" />
        </div>
        <div class="modal-action">
          <button @click="closeGitModal" class="btn">{{ $t('common.cancel') }}</button>
          <button @click="resolveGitUrl" class="btn btn-primary" :disabled="!gitUrl.trim() || installLoading">
            {{ $t('Tools.skillsGitResolve') }}
          </button>
        </div>
      </div>
    </dialog>
  </div>
</template>

<script setup lang="ts">
import { onMounted, onUnmounted, ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow'
import type { UnlistenFn } from '@tauri-apps/api/event'
import SkillsManager from '@/components/Agent/SkillsManager.vue'
import { open } from '@tauri-apps/plugin-dialog'
import { dialog } from '@/composables/useDialog'
import { useI18n } from 'vue-i18n'

const skillsManagerRef = ref<InstanceType<typeof SkillsManager> | null>(null)
const { t } = useI18n()
const showGitModal = ref(false)
const gitUrl = ref('')
const showInstallModal = ref(false)
const installCandidates = ref<any[]>([])
const selectedInstallSkills = ref<string[]>([])
const installLoading = ref(false)
const installSourcePath = ref('')
const installSourceType = ref('')
const installHistory = ref<any[]>([])
const historyLoading = ref(false)
const skillsEnabled = ref(true)
const isDragOver = ref(false)
let unlistenDragDrop: UnlistenFn | null = null

const refresh = () => {
  skillsManagerRef.value?.refresh?.()
  loadHistory()
}

const createSkill = () => {
  skillsManagerRef.value?.startCreate?.()
}

const handleSkillsChanged = () => {
  loadHistory()
}

const loadHistory = async () => {
  historyLoading.value = true
  try {
    installHistory.value = await invoke('list_skill_install_history')
  } catch (error) {
    console.error('Failed to load skill install history:', error)
    dialog.toast.error(`${error}`)
  } finally {
    historyLoading.value = false
  }
}

const loadSkillsEnabled = async () => {
  try {
    const configs = await invoke<Array<{ key: string, value: string }>>('get_config', {
      request: { category: 'agent', key: 'skills_enabled' }
    })
    if (configs.length > 0) {
      const raw = configs[0].value?.trim().toLowerCase()
      skillsEnabled.value = raw === 'true' || raw === '1' || raw === 'yes' || raw === 'on'
    } else {
      skillsEnabled.value = true
    }
  } catch (error) {
    console.error('Failed to load skills enabled setting:', error)
    skillsEnabled.value = true
  }
}

const persistSkillsEnabled = async () => {
  try {
    await invoke('set_config', {
      category: 'agent',
      key: 'skills_enabled',
      value: skillsEnabled.value ? 'true' : 'false'
    })
    dialog.toast.success(skillsEnabled.value ? t('common.enabled') : t('common.disabled'))
  } catch (error) {
    console.error('Failed to save skills enabled setting:', error)
    dialog.error(`${error}`)
  }
}

const openGitInstall = () => {
  gitUrl.value = ''
  showGitModal.value = true
}

const closeGitModal = () => {
  showGitModal.value = false
}

const closeInstallModal = () => {
  isDragOver.value = false
  showInstallModal.value = false
  installCandidates.value = []
  selectedInstallSkills.value = []
  installSourcePath.value = ''
  installSourceType.value = ''
}

const resolveGitUrl = async () => {
  if (!gitUrl.value.trim()) return
  installLoading.value = true
  showInstallModal.value = true
  try {
    const [path, candidates] = await invoke<[string, any[]]>('discover_skills_from_git', {
      url: gitUrl.value.trim()
    })
    installSourcePath.value = path
    installSourceType.value = 'git'
    installCandidates.value = candidates
    selectedInstallSkills.value = candidates.length === 1 ? [candidates[0].id] : []
    showGitModal.value = false
  } catch (error) {
    console.error('Failed to discover skills from git:', error)
    dialog.error(`${t('Tools.skillsInstallFailed')}\n${error}`)
    closeInstallModal()
  } finally {
    installLoading.value = false
  }
}

const importFromFile = async () => {
  try {
    const selected = await open({
      multiple: false,
      filters: [{ name: 'Skill File', extensions: ['md'] }]
    })
    if (!selected) return
    await discoverFromPath(Array.isArray(selected) ? selected[0] : selected, 'file')
  } catch (error) {
    console.error('Failed to import from file:', error)
    dialog.error(`${t('Tools.skillsImportFailed')}\n${error}`)
  }
}

const importFromFolder = async () => {
  try {
    const selected = await open({
      directory: true,
      multiple: false
    })
    if (!selected) return
    await discoverFromPath(Array.isArray(selected) ? selected[0] : selected, 'folder')
  } catch (error) {
    console.error('Failed to import from folder:', error)
    dialog.error(`${t('Tools.skillsImportFailed')}\n${error}`)
  }
}

const discoverFromPath = async (path: string, sourceType: string) => {
  if (!skillsEnabled.value) {
    dialog.toast.warning(t('Tools.skillsDisabledWarning'))
    return
  }

  installLoading.value = true
  showInstallModal.value = true
  try {
    const candidates = await invoke<any[]>('discover_skills_from_path', { sourcePath: path })
    installSourcePath.value = path
    installSourceType.value = sourceType
    installCandidates.value = candidates
    selectedInstallSkills.value = candidates.length === 1 ? [candidates[0].id] : []
  } catch (error) {
    console.error('Failed to discover skills:', error)
    dialog.error(`${t('Tools.skillsImportFailed')}\n${error}`)
    closeInstallModal()
  } finally {
    installLoading.value = false
  }
}

const installSelected = async () => {
  if (!installSourcePath.value) return
  installLoading.value = true
  try {
    await invoke('install_skills_from_path', {
      sourcePath: installSourcePath.value,
      skillIds: selectedInstallSkills.value,
      sourceType: installSourceType.value
    })
    closeInstallModal()
    refresh()
    dialog.toast.success(t('Tools.skillsInstallSuccess'))
  } catch (error) {
    console.error('Failed to install skills:', error)
    dialog.error(`${t('Tools.skillsInstallFailed')}\n${error}`)
  } finally {
    installLoading.value = false
  }
}

const deleteHistory = async (id: string) => {
  const confirmed = await dialog.confirm(t('Tools.skillsInstallHistoryDeleteConfirm'))
  if (!confirmed) return
  try {
    await invoke('delete_skill_install_history', { id })
    await loadHistory()
    dialog.toast.success(t('Tools.skillsInstallHistoryDeleted'))
  } catch (error) {
    console.error('Failed to delete skill install history:', error)
    dialog.error(`${t('Tools.skillsInstallHistoryDeleteFailed')}\n${error}`)
  }
}

const setupTauriDragDrop = async () => {
  try {
    const webview = getCurrentWebviewWindow()
    unlistenDragDrop = await webview.onDragDropEvent(async (event) => {
      if (!skillsEnabled.value) {
        isDragOver.value = false
        return
      }

      if (event.payload.type === 'enter' || event.payload.type === 'over') {
        isDragOver.value = true
        return
      }

      if (event.payload.type === 'leave') {
        isDragOver.value = false
        return
      }

      if (event.payload.type === 'drop') {
        isDragOver.value = false
        const droppedPaths = event.payload.paths ?? []
        if (droppedPaths.length === 0) return
        if (installLoading.value) return
        if (showInstallModal.value || showGitModal.value) return

        if (droppedPaths.length > 1) {
          dialog.toast.info(t('Tools.skillsDropMultipleHint'))
        }

        const sourcePath = droppedPaths[0]
        await discoverFromPath(sourcePath, 'drop')
      }
    })
  } catch (error) {
    console.error('Failed to setup skills drag-drop listener:', error)
  }
}

onMounted(async () => {
  await setupTauriDragDrop()
})

onUnmounted(() => {
  if (unlistenDragDrop) {
    unlistenDragDrop()
    unlistenDragDrop = null
  }
  isDragOver.value = false
})

loadHistory()
loadSkillsEnabled()

defineExpose({ refresh })
</script>
