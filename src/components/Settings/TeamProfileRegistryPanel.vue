<template>
  <div v-if="loading" class="flex items-center justify-center py-10">
    <span class="loading loading-spinner loading-lg" />
  </div>

  <div v-else class="grid grid-cols-1 xl:grid-cols-[320px_minmax(0,1fr)] gap-4">
    <AgentListPanel
      title="Team Profile 列表"
      empty-text="当前还没有 Team Profile。"
      :loading="loading"
      :items="teamListItems"
      :selected-id="selectedTeamProfileId"
      @select="selectedTeamProfileId = $event"
    >
      <template #actions>
        <div class="flex items-center gap-2">
          <button
            class="btn btn-xs btn-outline"
            :disabled="!canCreateProfiles"
            :title="profileCreationLockedTitle"
            @click="showAiTeamCreator = !showAiTeamCreator"
          >
            AI 创建
          </button>
          <button
            class="btn btn-xs btn-outline"
            :disabled="!canCreateProfiles"
            :title="profileCreationLockedTitle"
            @click="createTeamProfile"
          >
            新增 Team
          </button>
        </div>
      </template>

      <template #body-top>
        <div v-if="showAiTeamCreator" class="rounded-lg border border-primary/30 bg-primary/5 p-3">
          <label class="form-control">
            <span class="label-text mb-2 text-xs font-semibold">描述你要创建的 Team</span>
            <textarea
              v-model.trim="aiTeamDescription"
              class="textarea textarea-bordered min-h-[92px] text-sm"
              placeholder="例如：合建一个漏洞挖掘 Team，包含指挥、信息收集、验证和复盘观察角色。"
              :disabled="isAiCreatingTeam || !canCreateProfiles"
            />
          </label>
          <div class="mt-3 flex items-center justify-end gap-2">
            <button
              class="btn btn-xs btn-ghost"
              :disabled="isAiCreatingTeam"
              @click="showAiTeamCreator = false"
            >
              取消
            </button>
            <button
              class="btn btn-xs btn-primary"
              :disabled="isAiCreatingTeam || !canCreateProfiles || !aiTeamDescription.trim()"
              @click="createTeamWithAi"
            >
              <span v-if="isAiCreatingTeam" class="loading loading-spinner loading-xs" />
              创建
            </button>
          </div>
        </div>
      </template>
    </AgentListPanel>

    <SystemAgentDetailLayout :has-selection="!!selectedTeamProfile">
      <template #toolbar>
        <div class="flex flex-wrap items-center justify-end gap-2">
          <SystemAgentAutoSaveStatusBar
            :status-text="autoSaveStatusText"
            :status-class="autoSaveStatusClass"
          />
          <button class="btn btn-sm btn-ghost" :disabled="loading" @click="reload">
            <i class="fas fa-rotate mr-1"></i>
            刷新
          </button>
        </div>
      </template>

      <template #empty>
        <div class="py-12 text-center text-base-content/60">请选择一个 Team Profile。</div>
      </template>

      <template v-if="selectedTeamProfile">
        <div class="space-y-4">
          <AgentIdentityPanel
            eyebrow="Team Profile"
            :title="selectedTeamProfile.name || selectedTeamProfile.id"
            :description="selectedTeamProfile.description"
            :meta-items="selectedTeamProfileMetaItems"
          >
            <template #title>
              <div class="min-w-0 flex-1">
                <input
                  v-if="editingIdentityField === 'title'"
                  ref="identityTitleInputRef"
                  v-model.trim="identityTitleDraft"
                  class="input input-bordered input-sm w-full max-w-xl text-lg font-semibold"
                  type="text"
                  aria-label="编辑 Team 名称"
                  @blur="commitIdentityTitleEdit"
                  @keydown.enter.prevent="commitIdentityTitleEdit"
                  @keydown.esc.prevent="cancelIdentityEdit"
                />
                <div
                  v-else
                  class="cursor-text truncate text-lg font-semibold text-base-content rounded px-1 -mx-1 hover:bg-base-100"
                  title="点击编辑名称"
                  @click="startIdentityTitleEdit"
                >
                  {{ selectedTeamProfile.name || selectedTeamProfile.id }}
                </div>
              </div>
            </template>

            <template #badges>
              <span
                v-if="draftDefaultTeamProfileId === selectedTeamProfile.id"
                class="badge badge-primary badge-sm"
              >
                默认 Team
              </span>
            </template>

            <template #description>
              <div class="min-w-0">
                <textarea
                  v-if="editingIdentityField === 'description'"
                  ref="identityDescriptionInputRef"
                  v-model.trim="identityDescriptionDraft"
                  class="textarea textarea-bordered textarea-sm min-h-16 w-full max-w-3xl text-sm leading-6"
                  aria-label="编辑 Team 描述"
                  @blur="commitIdentityDescriptionEdit"
                  @keydown.enter.exact.prevent="commitIdentityDescriptionEdit"
                  @keydown.esc.prevent="cancelIdentityEdit"
                />
                <div
                  v-else
                  class="cursor-text rounded px-1 -mx-1 text-sm leading-6 text-base-content/70 hover:bg-base-100"
                  title="点击编辑描述"
                  @click="startIdentityDescriptionEdit"
                >
                  {{ selectedTeamProfile.description }}
                </div>
              </div>
            </template>

            <template #actions>
              <div class="flex flex-wrap items-center justify-end gap-2">
                <label class="label cursor-pointer justify-start gap-3">
                  <input
                    :checked="draftDefaultTeamProfileId === selectedTeamProfile.id"
                    type="radio"
                    name="default-team-profile"
                    class="radio radio-primary radio-sm"
                    @change="markSelectedAsDefault"
                  />
                  <span class="label-text">
                    {{ draftDefaultTeamProfileId === selectedTeamProfile.id ? '默认' : '设为默认' }}
                  </span>
                </label>
                <button
                  class="btn btn-sm btn-error btn-outline"
                  :disabled="draftTeamProfiles.length <= 1"
                  @click="removeSelectedTeamProfile"
                >
                  删除
                </button>
              </div>
            </template>
          </AgentIdentityPanel>

          <div class="grid grid-cols-1 lg:grid-cols-2 gap-4">
            <label class="form-control">
              <span class="label-text mb-2">上下文模式</span>
              <select v-model="selectedTeamProfile.contextMode" class="select select-bordered">
                <option value="claude-like">claude-like</option>
                <option value="codex-like">codex-like</option>
                <option value="sentinel-like">sentinel-like</option>
              </select>
            </label>
          </div>

          <div class="rounded-xl border border-base-300 bg-base-100 p-4">
            <div class="mb-3 text-sm font-semibold">角色绑定</div>
            <div
              class="mb-3 rounded-lg border border-info/30 bg-info/5 px-3 py-2 text-xs leading-5 text-base-content/70"
            >
              Team 默认成员 Profile 不预选工具。实际运行时优先看下方“工具角色矩阵”；只有当成员
              Profile 自己显式限制了工具范围，才会在角色矩阵基础上继续收窄。
            </div>
            <div class="grid grid-cols-1 lg:grid-cols-3 gap-4">
              <label class="form-control">
                <span class="label-text mb-2">Orchestrator Profile</span>
                <select
                  v-model="selectedTeamProfile.orchestratorProfileId"
                  class="select select-bordered"
                >
                  <option
                    v-for="profile in orchestratorOptions"
                    :key="profile.id"
                    :value="profile.id"
                  >
                    {{ profile.label }}
                  </option>
                </select>
              </label>

              <label class="form-control">
                <span class="label-text mb-2">Monitor Profile</span>
                <select
                  v-model="selectedTeamProfile.monitorProfileId"
                  class="select select-bordered"
                >
                  <option v-for="profile in monitorOptions" :key="profile.id" :value="profile.id">
                    {{ profile.label }}
                  </option>
                </select>
              </label>

              <label class="form-control">
                <span class="label-text mb-2">默认模型（可选）</span>
                <input
                  v-model.trim="selectedTeamProfile.defaultModel"
                  class="input input-bordered"
                  type="text"
                  placeholder="跟随角色 Profile"
                />
              </label>
            </div>

            <div class="mt-4 grid grid-cols-1 lg:grid-cols-2 gap-4">
              <label class="form-control">
                <span class="label-text mb-2">Team 编排模板（可选）</span>
                <select
                  v-model="selectedTeamProfile.defaultTeamOrchestrationPresetId"
                  class="select select-bordered"
                >
                  <option :value="null">不设置</option>
                  <option
                    v-for="preset in teamOrchestrationPresetOptions"
                    :key="preset.id"
                    :value="preset.id"
                  >
                    {{ preset.label }}
                  </option>
                </select>
              </label>

              <label class="form-control">
                <span class="label-text mb-2">Team 恢复策略（可选）</span>
                <select
                  v-model="selectedTeamProfile.defaultTeamRecoveryPresetId"
                  class="select select-bordered"
                >
                  <option :value="null">不设置</option>
                  <option
                    v-for="preset in teamRecoveryPresetOptions"
                    :key="preset.id"
                    :value="preset.id"
                  >
                    {{ preset.label }}
                  </option>
                </select>
              </label>
            </div>

            <div class="mt-4">
              <div class="mb-2 text-xs font-semibold uppercase tracking-wide text-base-content/60">
                Specialist Profiles
              </div>
              <div class="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-2">
                <label
                  v-for="profile in specialistOptions"
                  :key="profile.id"
                  class="flex items-center gap-2 rounded-lg border border-base-300 bg-base-200/30 px-3 py-2 text-sm"
                >
                  <input
                    type="checkbox"
                    class="checkbox checkbox-sm"
                    :checked="selectedTeamProfile.specialistProfileIds.includes(profile.id)"
                    @change="toggleSpecialistProfile(profile.id)"
                  />
                  <span>{{ profile.label }}</span>
                </label>
              </div>
            </div>
          </div>

          <TeamProfilePolicyEditors
            v-model:tool-policy-matrix="selectedTeamProfile.toolPolicyMatrix"
            v-model:memory-policy="selectedTeamProfile.memoryPolicy"
            v-model:harness-policy="selectedTeamProfile.harnessPolicy"
            v-model:concurrency-policy="selectedTeamProfile.concurrencyPolicy"
            v-model:safety-policy="selectedTeamProfile.safetyPolicy"
          />
        </div>
      </template>
    </SystemAgentDetailLayout>
  </div>
</template>

<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core'
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from 'vue'
import type {
  AssistantProfileOption,
  TeamProfileOption,
} from '@/components/Agent/assistantProfiles'
import { useAssistantProfiles } from '@/components/Agent/assistantProfiles'
import AgentIdentityPanel from '@/components/Settings/AgentIdentityPanel.vue'
import AgentListPanel from '@/components/Settings/AgentListPanel.vue'
import type { AgentListItemViewModel } from '@/components/Settings/agentListItemSupport'
import SystemAgentAutoSaveStatusBar from '@/components/Settings/system-agent/SystemAgentAutoSaveStatusBar.vue'
import SystemAgentDetailLayout from '@/components/Settings/system-agent/SystemAgentDetailLayout.vue'
import TeamProfilePolicyEditors from '@/components/Settings/TeamProfilePolicyEditors.vue'
import {
  TEAM_ORCHESTRATION_PRESET_METAS,
  TEAM_RECOVERY_PRESETS,
} from '@/components/Settings/teamProfilePresetOptions'
import { dialog } from '@/composables/useDialog'
import { useFeatureEntitlementsState } from '@/services/featureEntitlements'

const {
  defaultTeamProfileId,
  isLoadingAssistantProfiles,
  isLoadingDefaultTeamProfile,
  isLoadingTeamProfiles,
  isSavingDefaultTeamProfile,
  isSavingTeamProfiles,
  loadAssistantProfiles,
  loadDefaultTeamProfile,
  loadTeamProfiles,
  profileOptions,
  saveDefaultTeamProfile,
  saveTeamProfiles,
  teamProfileOptions,
} = useAssistantProfiles()

interface AiCreatedTeamProfileResponse {
  assistantProfiles: AssistantProfileOption[]
  teamProfile: TeamProfileOption
  provider: string
  model: string
}

type AutoSaveState = 'idle' | 'saving' | 'saved' | 'error'
type IdentityEditField = 'title' | 'description' | null

const draftTeamProfiles = ref<TeamProfileOption[]>([])
const entitlements = useFeatureEntitlementsState()
const draftDefaultTeamProfileId = ref('')
const selectedTeamProfileId = ref('')
const editingIdentityField = ref<IdentityEditField>(null)
const identityTitleDraft = ref('')
const identityDescriptionDraft = ref('')
const identityTitleInputRef = ref<HTMLInputElement | null>(null)
const identityDescriptionInputRef = ref<HTMLTextAreaElement | null>(null)
const showAiTeamCreator = ref(false)
const aiTeamDescription = ref('')
const isAiCreatingTeam = ref(false)
const autoSaveState = ref<AutoSaveState>('idle')
const suspendAutoSave = ref(false)
const lastSavedTeamProfilesSnapshot = ref('')
let autoSaveTimer: ReturnType<typeof setTimeout> | null = null
const teamOrchestrationPresetOptions = TEAM_ORCHESTRATION_PRESET_METAS
const teamRecoveryPresetOptions = TEAM_RECOVERY_PRESETS
const canCreateProfiles = computed(() => entitlements.value.is_licensed)
const profileCreationLockedTitle = computed(() =>
  canCreateProfiles.value ? '' : '未激活版本只允许修改配置，创建 Profile 需要输入卡密激活。'
)

const loading = computed(
  () =>
    isLoadingAssistantProfiles.value ||
    isLoadingTeamProfiles.value ||
    isLoadingDefaultTeamProfile.value
)

const assistantProfileById = computed(() => {
  const out = new Map<string, AssistantProfileOption>()
  profileOptions.value.forEach(profile => out.set(profile.id, profile))
  return out
})

const roleOptions = (role: AssistantProfileOption['teamRole']) =>
  profileOptions.value.filter(
    profile => profile.teamRole === role || profile.teamRole === 'assistant'
  )

const orchestratorOptions = computed(() => roleOptions('orchestrator'))
const specialistOptions = computed(() => roleOptions('specialist'))
const monitorOptions = computed(() => roleOptions('monitor'))

const selectedTeamProfile = computed(
  () => draftTeamProfiles.value.find(profile => profile.id === selectedTeamProfileId.value) || null
)

const selectedTeamProfileMetaItems = computed(() => {
  if (!selectedTeamProfile.value) return []
  return [
    { label: 'Team ID', value: selectedTeamProfile.value.id },
    {
      label: 'Orchestrator',
      value:
        assistantProfileById.value.get(selectedTeamProfile.value.orchestratorProfileId)?.label ||
        selectedTeamProfile.value.orchestratorProfileId,
    },
    { label: 'Specialists', value: String(selectedTeamProfile.value.specialistProfileIds.length) },
    {
      label: 'Monitor',
      value:
        assistantProfileById.value.get(selectedTeamProfile.value.monitorProfileId)?.label ||
        selectedTeamProfile.value.monitorProfileId,
    },
  ]
})

const startIdentityTitleEdit = async () => {
  if (!selectedTeamProfile.value || editingIdentityField.value === 'title') return
  editingIdentityField.value = 'title'
  identityTitleDraft.value = selectedTeamProfile.value.name || selectedTeamProfile.value.id
  await nextTick()
  identityTitleInputRef.value?.focus()
  identityTitleInputRef.value?.select()
}

const startIdentityDescriptionEdit = async () => {
  if (!selectedTeamProfile.value || editingIdentityField.value === 'description') return
  editingIdentityField.value = 'description'
  identityDescriptionDraft.value = selectedTeamProfile.value.description || ''
  await nextTick()
  identityDescriptionInputRef.value?.focus()
  identityDescriptionInputRef.value?.select()
}

const commitIdentityTitleEdit = () => {
  if (!selectedTeamProfile.value || editingIdentityField.value !== 'title') return
  const nextTitle = identityTitleDraft.value.trim()
  if (nextTitle) {
    selectedTeamProfile.value.name = nextTitle
  }
  editingIdentityField.value = null
}

const commitIdentityDescriptionEdit = () => {
  if (!selectedTeamProfile.value || editingIdentityField.value !== 'description') return
  const nextDescription = identityDescriptionDraft.value.trim()
  if (nextDescription) {
    selectedTeamProfile.value.description = nextDescription
  }
  editingIdentityField.value = null
}

const cancelIdentityEdit = () => {
  editingIdentityField.value = null
}

const teamListItems = computed<AgentListItemViewModel[]>(() =>
  draftTeamProfiles.value.map(profile => ({
    id: profile.id,
    title: profile.name || profile.id,
    description: profile.description,
    metaLine: `${profile.contextMode} · ${profile.specialistProfileIds.length} specialist`,
    badges: [
      ...(draftDefaultTeamProfileId.value === profile.id
        ? [{ label: '默认', className: 'badge-primary' }]
        : []),
      { label: profile.contextMode, className: 'badge-outline' },
    ],
    searchText: [
      profile.name,
      profile.description,
      profile.orchestratorProfileId,
      profile.monitorProfileId,
      profile.specialistProfileIds.join(' '),
    ].join(' '),
    filterKeys: [],
  }))
)

const canSaveTeamProfiles = computed(
  () =>
    !isSavingTeamProfiles.value &&
    !isSavingDefaultTeamProfile.value &&
    draftTeamProfiles.value.length > 0 &&
    draftTeamProfiles.value.every(
      profile =>
        profile.id.trim() &&
        profile.name.trim() &&
        profile.orchestratorProfileId.trim() &&
        profile.monitorProfileId.trim() &&
        profile.specialistProfileIds.length > 0
    )
)

const buildTeamProfilesSnapshot = (profiles: TeamProfileOption[]) => JSON.stringify(profiles)

const currentDraftStateSnapshot = computed(() =>
  JSON.stringify({
    profiles: draftTeamProfiles.value,
    defaultProfileId: draftDefaultTeamProfileId.value.trim(),
  })
)

const hasTeamProfilesUnsavedChanges = computed(
  () => buildTeamProfilesSnapshot(draftTeamProfiles.value) !== lastSavedTeamProfilesSnapshot.value
)

const hasDefaultTeamUnsavedChanges = computed(
  () => draftDefaultTeamProfileId.value.trim() !== defaultTeamProfileId.value.trim()
)

const hasUnsavedChanges = computed(
  () => hasTeamProfilesUnsavedChanges.value || hasDefaultTeamUnsavedChanges.value
)

const autoSaveStatusText = computed(() => {
  if (autoSaveState.value === 'saving') return '自动保存中'
  if (autoSaveState.value === 'saved') return '已自动保存'
  if (autoSaveState.value === 'error') return '自动保存失败'
  if (hasTeamProfilesUnsavedChanges.value && !canSaveTeamProfiles.value) return '填写完整后自动保存'
  return '修改后自动保存'
})

const autoSaveStatusClass = computed(() => {
  if (autoSaveState.value === 'saving') return 'text-info'
  if (autoSaveState.value === 'saved') return 'text-success'
  if (autoSaveState.value === 'error') return 'text-error'
  if (hasTeamProfilesUnsavedChanges.value && !canSaveTeamProfiles.value) return 'text-warning'
  return 'text-base-content/60'
})

const clearAutoSaveTimer = () => {
  if (!autoSaveTimer) return
  clearTimeout(autoSaveTimer)
  autoSaveTimer = null
}

const cloneTeamProfile = (profile: TeamProfileOption): TeamProfileOption => ({
  ...profile,
  specialistProfileIds: [...profile.specialistProfileIds],
  memoryPolicy: { ...profile.memoryPolicy },
  toolPolicyMatrix: { ...profile.toolPolicyMatrix },
  harnessPolicy: { ...profile.harnessPolicy },
  concurrencyPolicy: { ...profile.concurrencyPolicy },
  safetyPolicy: { ...profile.safetyPolicy },
})

const resetDraftState = () => {
  clearAutoSaveTimer()
  draftTeamProfiles.value = teamProfileOptions.value.map(cloneTeamProfile)
  draftDefaultTeamProfileId.value = defaultTeamProfileId.value
  if (!draftTeamProfiles.value.some(profile => profile.id === selectedTeamProfileId.value)) {
    selectedTeamProfileId.value = draftTeamProfiles.value[0]?.id || ''
  }
  lastSavedTeamProfilesSnapshot.value = buildTeamProfilesSnapshot(draftTeamProfiles.value)
  autoSaveState.value = 'idle'
}

const reload = async () => {
  clearAutoSaveTimer()
  suspendAutoSave.value = true
  try {
    await Promise.all([
      loadAssistantProfiles(true),
      loadTeamProfiles(true),
      loadDefaultTeamProfile(true),
    ])
    resetDraftState()
  } catch (error) {
    console.error('Failed to load Team profiles:', error)
    dialog.toast.error('Team Profile 加载失败')
  } finally {
    suspendAutoSave.value = false
  }
}

const saveTeamProfilesInternal = async (options?: { silent?: boolean }) => {
  const shouldSaveProfiles = hasTeamProfilesUnsavedChanges.value
  const shouldSaveDefault =
    hasDefaultTeamUnsavedChanges.value && !!draftDefaultTeamProfileId.value.trim()

  if (!shouldSaveProfiles && !shouldSaveDefault) return
  if (shouldSaveProfiles && !canSaveTeamProfiles.value) return

  clearAutoSaveTimer()
  suspendAutoSave.value = true
  autoSaveState.value = 'saving'
  try {
    if (shouldSaveProfiles) {
      await saveTeamProfiles(draftTeamProfiles.value)
      lastSavedTeamProfilesSnapshot.value = buildTeamProfilesSnapshot(draftTeamProfiles.value)
    }

    if (shouldSaveDefault) {
      const profileId = draftDefaultTeamProfileId.value.trim()
      await saveDefaultTeamProfile(profileId)
      draftDefaultTeamProfileId.value = profileId
    }

    const savedAllChanges = !hasUnsavedChanges.value
    autoSaveState.value = savedAllChanges ? 'saved' : 'idle'
    if (savedAllChanges) {
      dialog.toast.success(options?.silent ? 'Team Profile 已自动保存' : 'Team Profile 已保存')
    }
  } catch (error) {
    console.error('Failed to save Team profiles:', error)
    autoSaveState.value = 'error'
    dialog.toast.error(options?.silent ? 'Team Profile 自动保存失败' : 'Team Profile 保存失败')
  } finally {
    suspendAutoSave.value = false
    if (hasUnsavedChanges.value) {
      queueAutoSave()
    }
  }
}

const queueAutoSave = () => {
  if (
    suspendAutoSave.value ||
    loading.value ||
    isSavingTeamProfiles.value ||
    isSavingDefaultTeamProfile.value
  )
    return
  if (!hasUnsavedChanges.value) return
  if (hasTeamProfilesUnsavedChanges.value && !canSaveTeamProfiles.value) return

  clearAutoSaveTimer()
  autoSaveTimer = setTimeout(() => {
    autoSaveTimer = null
    if (
      suspendAutoSave.value ||
      loading.value ||
      isSavingTeamProfiles.value ||
      isSavingDefaultTeamProfile.value
    )
      return
    if (!hasUnsavedChanges.value) return
    if (hasTeamProfilesUnsavedChanges.value && !canSaveTeamProfiles.value) return
    void saveTeamProfilesInternal({ silent: true })
  }, 600)
}

const createTeamProfile = () => {
  if (!canCreateProfiles.value) {
    dialog.toast.warning('未激活版本只允许修改配置，创建 Profile 需要输入卡密激活。')
    return
  }
  const ids = new Set(draftTeamProfiles.value.map(profile => profile.id))
  let index = draftTeamProfiles.value.length + 1
  while (ids.has(`team.profile.custom.${index}`)) index += 1
  const profile: TeamProfileOption = {
    id: `team.profile.custom.${index}`,
    name: `Custom Team ${index}`,
    description: '自定义 Team Profile',
    orchestratorProfileId: orchestratorOptions.value[0]?.id || profileOptions.value[0]?.id || '',
    specialistProfileIds: [
      specialistOptions.value[0]?.id || profileOptions.value[0]?.id || '',
    ].filter(Boolean),
    monitorProfileId: monitorOptions.value[0]?.id || profileOptions.value[0]?.id || '',
    defaultModel: null,
    defaultTeamOrchestrationPresetId: null,
    defaultTeamRecoveryPresetId: null,
    contextMode: 'claude-like',
    memoryPolicy: {
      monitorGate: 'candidate_then_orchestrator_accept',
      shareScope: 'high_value_only',
      longTermMemory: true,
    },
    toolPolicyMatrix: {
      orchestrator: { tools: ['ask_user_question'] },
      specialist: {
        tools: [
          'shell',
          'file_read',
          'file_edit',
          'file_write',
          'grep',
          'http_request',
          'web_search',
        ],
      },
      monitor: { tools: ['tenth_man_review'] },
    },
    harnessPolicy: {
      heartbeatSecs: 30,
      leaseSecs: 600,
      checkpoint: 'event_sequence',
      allowResume: true,
    },
    concurrencyPolicy: { maxSpecialists: 2, maxTasksPerSpecialist: 1 },
    safetyPolicy: {
      orchestratorNoDangerousTools: true,
      monitorReadOnly: true,
      requireApprovalForHighRiskTools: true,
    },
  }
  draftTeamProfiles.value.push(profile)
  selectedTeamProfileId.value = profile.id
}

const createTeamWithAi = async () => {
  const description = aiTeamDescription.value.trim()
  if (!canCreateProfiles.value) {
    dialog.toast.warning('未激活版本只允许修改配置，创建 Profile 需要输入卡密激活。')
    return
  }
  if (!description || isAiCreatingTeam.value) return

  isAiCreatingTeam.value = true
  try {
    const result = await invoke<AiCreatedTeamProfileResponse>(
      'ai_create_team_profile_from_description',
      { request: { description } }
    )
    await reload()
    selectedTeamProfileId.value = result.teamProfile.id
    aiTeamDescription.value = ''
    showAiTeamCreator.value = false
    dialog.toast.success(`AI 已创建 Team：${result.teamProfile.name}`)
  } catch (error) {
    console.error('Failed to create Team profile with AI:', error)
    dialog.toast.error('AI 创建 Team 失败')
  } finally {
    isAiCreatingTeam.value = false
  }
}

const removeSelectedTeamProfile = () => {
  if (!selectedTeamProfile.value || draftTeamProfiles.value.length <= 1) return
  draftTeamProfiles.value = draftTeamProfiles.value.filter(
    profile => profile !== selectedTeamProfile.value
  )
  if (draftDefaultTeamProfileId.value === selectedTeamProfile.value.id) {
    draftDefaultTeamProfileId.value = draftTeamProfiles.value[0]?.id || ''
  }
  selectedTeamProfileId.value = draftTeamProfiles.value[0]?.id || ''
}

const markSelectedAsDefault = () => {
  if (!selectedTeamProfile.value) return
  draftDefaultTeamProfileId.value = selectedTeamProfile.value.id
}

const toggleSpecialistProfile = (profileId: string) => {
  if (!selectedTeamProfile.value) return
  const current = new Set(selectedTeamProfile.value.specialistProfileIds)
  if (current.has(profileId)) {
    current.delete(profileId)
  } else {
    current.add(profileId)
  }
  selectedTeamProfile.value.specialistProfileIds = Array.from(current)
}

onMounted(() => {
  void reload()
})

onUnmounted(() => {
  clearAutoSaveTimer()
})

watch(
  teamProfileOptions,
  () => {
    if (suspendAutoSave.value) return
    resetDraftState()
  },
  { deep: true }
)

watch(defaultTeamProfileId, () => {
  if (suspendAutoSave.value) return
  draftDefaultTeamProfileId.value = defaultTeamProfileId.value
})

watch(selectedTeamProfileId, () => {
  cancelIdentityEdit()
})

watch(currentDraftStateSnapshot, snapshot => {
  if (!snapshot || suspendAutoSave.value || loading.value) return
  if (!hasUnsavedChanges.value) return
  autoSaveState.value = 'idle'
  queueAutoSave()
})
</script>
