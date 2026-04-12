<template>
  <div class="space-y-4">
    <div class="rounded-2xl border border-base-300 bg-base-100 px-4 py-3">
      <div class="flex flex-wrap items-start justify-between gap-3">
        <div>
          <div class="text-sm font-semibold">{{ t('agent.skillLibraryTitle') }}</div>
          <div class="text-xs text-base-content/60">
            {{ t('agent.skillLibraryDescription', { mode: currentViewModeLabel }) }}
          </div>
        </div>
        <div class="flex flex-wrap gap-2 text-[11px]">
          <span class="badge badge-sm badge-outline">{{ currentViewModeLabel }}</span>
          <span class="badge badge-sm badge-success badge-outline">{{ t('agent.enabledCountLower', { count: enabledSkillCount }) }}</span>
          <span class="badge badge-sm badge-ghost">{{ t('agent.disabledCountLower', { count: disabledSkillCount }) }}</span>
        </div>
      </div>
    </div>

    <div v-if="loading" class="flex justify-center py-8">
      <span class="loading loading-spinner loading-md"></span>
    </div>

    <div v-else-if="skills.length === 0" class="py-8 text-center text-base-content/60">
      <i class="fas fa-inbox mb-2 text-3xl"></i>
      <p>{{ t('agent.noSkills') }}</p>
      <p class="mt-1 text-sm">{{ t('agent.createFirstSkill') }}</p>
      <button class="btn btn-sm btn-primary mt-4" @click="$emit('start-create')">
        <i class="fas fa-plus mr-1"></i>
        {{ t('agent.createSkill') }}
      </button>
    </div>

    <div v-else-if="viewMode === 'list'" class="overflow-x-auto">
      <table class="table table-zebra table-sm">
        <thead>
          <tr>
            <th>{{ t('agent.skillName') }}</th>
            <th>{{ t('agent.skillDescription') }}</th>
            <th>{{ t('common.status') }}</th>
            <th class="w-40">{{ t('common.actions') }}</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="skill in skills" :key="skill.id" class="align-top">
            <td>
              <div class="flex min-w-0 items-center gap-3">
                <div :class="['skill-icon', getSkillIconClass(skill.id)]">
                  <i :class="getSkillIcon(skill.id)"></i>
                </div>
                <div class="min-w-0">
                  <div class="truncate font-medium">{{ skill.name }}</div>
                  <div class="truncate font-mono text-[10px] text-base-content/50">{{ skill.id }}</div>
                  <div v-if="skill.source_path" class="truncate text-[11px] text-base-content/45">
                    {{ skill.source_path }}
                  </div>
                </div>
              </div>
            </td>
            <td>
              <div class="max-w-xl space-y-2">
                <div class="line-clamp-2 text-sm text-base-content/70">{{ skill.description || '-' }}</div>
                <div class="flex flex-wrap gap-2">
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
            </td>
            <td>
              <div class="space-y-2">
                <span class="badge badge-sm" :class="skillEnabledBadgeClass(isSkillEnabled(skill.id))">
                  {{ skillEnabledLabel(isSkillEnabled(skill.id)) }}
                </span>
                <label class="flex items-center gap-2 text-xs text-base-content/60">
                  <input
                    type="checkbox"
                    class="toggle toggle-xs"
                    :checked="isSkillEnabled(skill.id)"
                    @change="$emit('toggle-enabled', skill.id, ($event.target as HTMLInputElement).checked)"
                  />
                  <span>{{ t('agent.quickToggle') }}</span>
                </label>
              </div>
            </td>
            <td>
              <div class="flex items-center gap-2">
                <button @click="$emit('edit', skill)" class="btn btn-xs btn-ghost">
                  <i class="fas fa-edit mr-1"></i>
                  {{ t('common.edit') }}
                </button>
                <button
                  @click="$emit('delete', skill)"
                  class="btn btn-xs btn-error btn-outline"
                  :disabled="deletingSkillIds.includes(skill.id)"
                >
                  <i :class="deletingSkillIds.includes(skill.id) ? 'fas fa-spinner fa-spin mr-1' : 'fas fa-trash mr-1'"></i>
                  {{ t('common.delete') }}
                </button>
              </div>
            </td>
          </tr>
        </tbody>
      </table>
    </div>

    <div v-else class="skills-cards-grid">
      <div
        v-for="skill in skills"
        :key="skill.id"
        class="group rounded-2xl border border-base-300 bg-base-100 p-4 shadow-sm transition hover:-translate-y-0.5 hover:border-base-content/20 hover:shadow-md"
      >
        <div class="flex h-full flex-col">
          <div class="flex items-start justify-between gap-3">
            <div class="flex min-w-0 flex-1 items-start gap-3">
              <div :class="['skill-icon', getSkillIconClass(skill.id)]">
                <i :class="getSkillIcon(skill.id)"></i>
              </div>
              <div class="min-w-0 flex-1">
                <div class="flex flex-wrap items-center gap-2">
                  <div class="truncate font-medium">{{ skill.name }}</div>
                  <span class="badge badge-sm" :class="skillEnabledBadgeClass(isSkillEnabled(skill.id))">
                    {{ skillEnabledLabel(isSkillEnabled(skill.id)) }}
                  </span>
                </div>
                <div class="mt-1 truncate font-mono text-[10px] text-base-content/50">{{ skill.id }}</div>
                <div v-if="skill.source_path" class="mt-1 truncate text-[11px] text-base-content/45">
                  {{ skill.source_path }}
                </div>
              </div>
            </div>
            <div class="flex items-center gap-1">
              <button @click="$emit('edit', skill)" class="btn btn-xs btn-ghost btn-circle" :title="t('common.edit')">
                <i class="fas fa-edit"></i>
              </button>
              <button
                @click="$emit('delete', skill)"
                class="btn btn-xs btn-ghost btn-circle text-error"
                :disabled="deletingSkillIds.includes(skill.id)"
                :title="t('common.delete')"
              >
                <i :class="deletingSkillIds.includes(skill.id) ? 'fas fa-spinner fa-spin' : 'fas fa-trash'"></i>
              </button>
            </div>
          </div>

          <div class="mt-4 min-h-[3.5rem] line-clamp-3 text-sm leading-6 text-base-content/70">
            {{ skill.description || t('agent.noSkillDescriptionYet') }}
          </div>

          <div class="mt-4 flex flex-wrap gap-2">
            <span v-if="skill.content" class="badge badge-sm badge-info badge-outline">
              {{ t('agent.hasSkillContent') }}
            </span>
            <span v-if="skill.disable_model_invocation" class="badge badge-sm badge-warning badge-outline">
              {{ t('agent.modelInvocationDisabled') }}
            </span>
            <span v-if="!skill.user_invocable" class="badge badge-sm badge-neutral badge-outline">
              {{ t('agent.notUserInvocable') }}
            </span>
          </div>

          <div class="mt-auto pt-4">
            <div class="flex items-center justify-between gap-3 rounded-xl border border-base-300 bg-base-200/60 px-3 py-2">
              <div class="text-[11px] text-base-content/60">
                {{ isSkillEnabled(skill.id) ? t('agent.skillRoutingEnabled') : t('agent.skillRoutingHidden') }}
              </div>
              <label class="flex items-center gap-2 text-xs text-base-content/60">
                <span>{{ isSkillEnabled(skill.id) ? t('agent.switchOn') : t('agent.switchOff') }}</span>
                <input
                  type="checkbox"
                  class="toggle toggle-xs"
                  :checked="isSkillEnabled(skill.id)"
                  @change="$emit('toggle-enabled', skill.id, ($event.target as HTMLInputElement).checked)"
                />
              </label>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import type { Skill } from './skillsManagerTypes'

defineProps<{
  loading: boolean
  skills: Skill[]
  viewMode: 'card' | 'list'
  currentViewModeLabel: string
  enabledSkillCount: number
  disabledSkillCount: number
  deletingSkillIds: string[]
  isSkillEnabled: (id: string) => boolean
  getSkillIcon: (id: string) => string
  getSkillIconClass: (id: string) => string
}>()

defineEmits<{
  'start-create': []
  'edit': [skill: Skill]
  'delete': [skill: Skill]
  'toggle-enabled': [id: string, enabled: boolean]
}>()

const { t } = useI18n()

const skillEnabledLabel = (enabled: boolean) =>
  enabled ? t('common.enabled') : t('common.disabled')

const skillEnabledBadgeClass = (enabled: boolean) =>
  enabled ? 'badge-success badge-outline' : 'badge-error badge-outline'
</script>

<style scoped>
.skills-cards-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
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
</style>
