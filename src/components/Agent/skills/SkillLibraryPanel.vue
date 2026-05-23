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
        <div class="flex flex-wrap items-center justify-end gap-2 text-[11px]">
          <span class="badge badge-sm badge-outline">{{ t('agent.skillCount', { count: skills.length }) }}</span>
          <span class="badge badge-sm badge-success badge-outline">{{ t('agent.enabledCount', { count: enabledSkillCount }) }}</span>
          <span class="badge badge-sm badge-ghost">{{ t('agent.disabledCount', { count: disabledSkillCount }) }}</span>
          <span class="badge badge-sm badge-info badge-outline">{{ t('agent.withContentCount', { count: skillsWithContentCount }) }}</span>
          <span class="badge badge-sm badge-warning badge-outline">{{ t('agent.draftCount', { count: activeDraftCount }) }}</span>
          <span class="badge badge-sm badge-error badge-outline">{{ t('agent.ruleCount', { count: ruleCount }) }}</span>
          <button class="btn btn-xs btn-outline" @click="$emit('open-memory-feedback')">
            <i class="fas fa-comment-dots mr-1"></i>
            {{ t('agent.memoryFeedbackTitle') }}
          </button>
          <div class="join">
            <button
              class="join-item btn btn-xs btn-outline"
              :disabled="bulkUpdatingSkillState || skills.length === 0 || disabledSkillCount === 0"
              @click="$emit('set-all-enabled', true)"
            >
              <i :class="bulkUpdatingSkillState ? 'fas fa-spinner fa-spin mr-1' : 'fas fa-toggle-on mr-1'"></i>
              {{ t('agent.enableAllSkills') }}
            </button>
            <button
              class="join-item btn btn-xs btn-outline"
              :disabled="bulkUpdatingSkillState || skills.length === 0 || enabledSkillCount === 0"
              @click="$emit('set-all-enabled', false)"
            >
              <i :class="bulkUpdatingSkillState ? 'fas fa-spinner fa-spin mr-1' : 'fas fa-toggle-off mr-1'"></i>
              {{ t('agent.disableAllSkills') }}
            </button>
          </div>
          <span class="badge badge-sm badge-outline">{{ currentViewModeLabel }}</span>
        </div>
      </div>
    </div>

    <div v-if="skills.length > 0" class="rounded-lg border border-base-300 bg-base-100">
      <div class="flex flex-col gap-3 p-3 lg:flex-row lg:items-center lg:justify-between">
        <label class="input input-bordered input-sm flex min-w-0 flex-1 items-center gap-2 lg:max-w-md">
          <i class="fas fa-search text-base-content/40"></i>
          <input
            v-model="searchQuery"
            type="search"
            class="grow"
            :placeholder="t('agent.skillSearchPlaceholder')"
          />
        </label>
        <div class="flex flex-wrap items-center justify-end gap-2">
          <span class="badge badge-sm badge-ghost">
            {{ t('agent.skillFilteredCount', { count: filteredSkills.length, total: skills.length }) }}
          </span>
          <template v-if="viewMode === 'list'">
            <span class="badge badge-sm badge-outline">
              {{ t('agent.skillSelectedCount', { count: selectedSkillIds.length }) }}
            </span>
            <button
              class="btn btn-xs btn-error btn-outline"
              :disabled="selectedSkillIds.length === 0 || bulkDeletingSkills"
              @click="emitDeleteSelected"
            >
              <i :class="bulkDeletingSkills ? 'fas fa-spinner fa-spin mr-1' : 'fas fa-trash mr-1'"></i>
              {{ t('agent.deleteSelectedSkills') }}
            </button>
          </template>
          <button
            class="btn btn-xs btn-error"
            :disabled="skills.length === 0 || bulkDeletingSkills"
            @click="$emit('delete-all')"
          >
            <i :class="bulkDeletingSkills ? 'fas fa-spinner fa-spin mr-1' : 'fas fa-trash-can mr-1'"></i>
            {{ t('agent.deleteAllSkills') }}
          </button>
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

    <div v-else-if="filteredSkills.length === 0" class="py-8 text-center text-base-content/60">
      <i class="fas fa-search mb-2 text-3xl"></i>
      <p>{{ t('agent.skillSearchEmpty') }}</p>
    </div>

    <div v-else-if="viewMode === 'list'" class="overflow-x-auto rounded-lg border border-base-300 bg-base-100">
      <table class="table table-sm w-full">
        <thead>
          <tr>
            <th class="w-10">
              <input
                type="checkbox"
                class="checkbox checkbox-xs checkbox-primary"
                :checked="allCurrentPageSelected"
                :indeterminate="someCurrentPageSelected && !allCurrentPageSelected"
                :disabled="paginatedSkills.length === 0 || bulkDeletingSkills"
                @change="toggleCurrentPageSelection(($event.target as HTMLInputElement).checked)"
              />
            </th>
            <th>{{ t('agent.skillName') }}</th>
            <th>{{ t('agent.skillDescription') }}</th>
            <th>{{ t('common.status') }}</th>
            <th class="w-40">{{ t('common.actions') }}</th>
          </tr>
        </thead>
        <tbody>
          <tr
            v-for="skill in paginatedSkills"
            :key="skill.id"
            :class="skillRowClass(skill)"
          >
            <td>
              <input
                v-model="selectedSkillIds"
                type="checkbox"
                class="checkbox checkbox-xs checkbox-primary"
                :value="skill.id"
                :disabled="deletingSkillIds.includes(skill.id) || bulkDeletingSkills"
              />
            </td>
            <td>
              <div class="min-w-0">
                <div class="truncate font-medium">{{ skill.name }}</div>
                <div class="truncate font-mono text-[10px] text-base-content/50">{{ skill.id }}</div>
                <div v-if="skill.source_path" class="truncate text-[11px] text-base-content/45">
                  {{ skill.source_path }}
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
                    :disabled="bulkUpdatingSkillState"
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
        v-for="skill in paginatedSkills"
        :key="skill.id"
        :class="skillCardClass(skill)"
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
                  :disabled="bulkUpdatingSkillState"
                  @change="$emit('toggle-enabled', skill.id, ($event.target as HTMLInputElement).checked)"
                />
              </label>
            </div>
          </div>
        </div>
      </div>
    </div>

    <div
      v-if="!loading && skills.length > 0 && filteredSkills.length > 0"
      class="flex flex-col gap-3 rounded-lg border border-base-300 bg-base-100 px-3 py-2 md:flex-row md:items-center md:justify-between"
    >
      <div class="text-xs text-base-content/60">
        {{ paginationSummary }}
      </div>
      <div class="flex flex-wrap items-center gap-2">
        <select v-model.number="pageSize" class="select select-bordered select-xs w-24">
          <option v-for="option in pageSizeOptions" :key="option" :value="option">
            {{ option }}
          </option>
        </select>
        <div class="join">
          <button class="join-item btn btn-xs" :disabled="currentPage <= 1" @click="currentPage -= 1">
            <i class="fas fa-chevron-left mr-1"></i>
            {{ t('common.previous') }}
          </button>
          <button class="join-item btn btn-xs btn-ghost cursor-default">
            {{ t('agent.skillPageInfo', { page: currentPage, total: totalPages }) }}
          </button>
          <button class="join-item btn btn-xs" :disabled="currentPage >= totalPages" @click="currentPage += 1">
            {{ t('common.next') }}
            <i class="fas fa-chevron-right ml-1"></i>
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import type { Skill } from './skillsManagerTypes'

const props = defineProps<{
  loading: boolean
  skills: Skill[]
  viewMode: 'card' | 'list'
  currentViewModeLabel: string
  enabledSkillCount: number
  disabledSkillCount: number
  skillsWithContentCount: number
  activeDraftCount: number
  ruleCount: number
  bulkUpdatingSkillState: boolean
  bulkDeletingSkills: boolean
  deletingSkillIds: string[]
  isSkillEnabled: (id: string) => boolean
  getSkillIcon: (id: string) => string
  getSkillIconClass: (id: string) => string
}>()

const emit = defineEmits<{
  'start-create': []
  'edit': [skill: Skill]
  'delete': [skill: Skill]
  'delete-selected': [ids: string[]]
  'delete-all': []
  'toggle-enabled': [id: string, enabled: boolean]
  'set-all-enabled': [enabled: boolean]
  'open-memory-feedback': []
}>()

const { t } = useI18n()
const pageSizeOptions = [10, 20, 50]
const searchQuery = ref('')
const currentPage = ref(1)
const pageSize = ref(10)
const selectedSkillIds = ref<string[]>([])

const orderedSkills = computed(() =>
  props.skills
    .map((skill, index) => ({ skill, index }))
    .sort((left, right) => {
      const leftEnabled = props.isSkillEnabled(left.skill.id)
      const rightEnabled = props.isSkillEnabled(right.skill.id)
      if (leftEnabled !== rightEnabled) {
        return leftEnabled ? -1 : 1
      }
      return left.index - right.index
    })
    .map(item => item.skill)
)

const normalizedSearchQuery = computed(() => searchQuery.value.trim().toLowerCase())

const filteredSkills = computed(() => {
  const query = normalizedSearchQuery.value
  if (!query) return orderedSkills.value
  return orderedSkills.value.filter(skill => {
    const searchable = [
      skill.id,
      skill.name,
      skill.description,
      skill.source_path,
      skill.argument_hint,
      skill.model,
      skill.context,
      skill.agent,
      ...(skill.allowed_tools || [])
    ].join(' ').toLowerCase()
    return searchable.includes(query)
  })
})

const totalPages = computed(() =>
  Math.max(1, Math.ceil(filteredSkills.value.length / pageSize.value))
)

const paginatedSkills = computed(() => {
  const start = (currentPage.value - 1) * pageSize.value
  return filteredSkills.value.slice(start, start + pageSize.value)
})

const currentPageIds = computed(() => paginatedSkills.value.map(skill => skill.id))

const allCurrentPageSelected = computed(() =>
  currentPageIds.value.length > 0
  && currentPageIds.value.every(id => selectedSkillIds.value.includes(id))
)

const someCurrentPageSelected = computed(() =>
  currentPageIds.value.some(id => selectedSkillIds.value.includes(id))
)

const paginationSummary = computed(() => {
  const total = filteredSkills.value.length
  if (total === 0) return t('agent.skillPaginationEmpty')
  const start = (currentPage.value - 1) * pageSize.value + 1
  const end = Math.min(start + pageSize.value - 1, total)
  return t('agent.skillPaginationSummary', { start, end, total })
})

watch([searchQuery, pageSize], () => {
  currentPage.value = 1
})

watch(totalPages, pages => {
  if (currentPage.value > pages) {
    currentPage.value = pages
  }
})

watch(
  () => props.skills.map(skill => skill.id),
  ids => {
    const availableIds = new Set(ids)
    selectedSkillIds.value = selectedSkillIds.value.filter(id => availableIds.has(id))
  }
)

const toggleCurrentPageSelection = (checked: boolean) => {
  const pageIds = currentPageIds.value
  if (checked) {
    selectedSkillIds.value = Array.from(new Set([...selectedSkillIds.value, ...pageIds]))
    return
  }
  selectedSkillIds.value = selectedSkillIds.value.filter(id => !pageIds.includes(id))
}

const emitDeleteSelected = () => {
  emit('delete-selected', [...selectedSkillIds.value])
}

const skillEnabledLabel = (enabled: boolean) =>
  enabled ? t('common.enabled') : t('common.disabled')

const skillEnabledBadgeClass = (enabled: boolean) =>
  enabled ? 'badge-success badge-outline' : 'badge-error badge-outline'

const skillRowClass = (skill: Skill) => [
  'align-top border-l-4 hover:bg-base-200/70',
  props.isSkillEnabled(skill.id)
    ? 'border-l-success bg-success/5'
    : 'border-l-transparent'
]

const skillCardClass = (skill: Skill) => [
  'group rounded-2xl border bg-base-100 p-4 shadow-sm transition hover:-translate-y-0.5 hover:shadow-md',
  props.isSkillEnabled(skill.id)
    ? 'border-success/70 ring-1 ring-success/25 shadow-success/10 hover:border-success'
    : 'border-base-300 hover:border-base-content/20'
]
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
