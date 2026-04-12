<template>
  <div class="mb-4 rounded-2xl border border-base-300 bg-gradient-to-r from-base-100 via-base-100 to-base-200/70 p-4">
    <div class="flex flex-wrap items-start justify-between gap-4">
      <div class="min-w-0">
        <div class="flex items-center gap-3">
          <div class="flex h-11 w-11 items-center justify-center rounded-2xl bg-primary/10 text-primary">
            <i class="fas fa-wand-magic-sparkles"></i>
          </div>
          <div>
            <h3 class="text-lg font-semibold leading-tight">
              {{ t('agent.skillManagement') }}
            </h3>
            <div class="text-sm text-base-content/60">
              {{ t('agent.skillManagementSubtitle') }}
            </div>
          </div>
        </div>
        <div class="mt-4 flex flex-wrap gap-2">
          <span class="badge badge-sm badge-outline">{{ t('agent.skillCount', { count: skillsCount }) }}</span>
          <span class="badge badge-sm badge-success badge-outline">{{ t('agent.enabledCount', { count: enabledSkillCount }) }}</span>
          <span class="badge badge-sm badge-ghost">{{ t('agent.disabledCount', { count: disabledSkillCount }) }}</span>
          <span class="badge badge-sm badge-info badge-outline">{{ t('agent.withContentCount', { count: skillsWithContentCount }) }}</span>
          <span class="badge badge-sm badge-warning badge-outline">{{ t('agent.draftCount', { count: activeDraftCount }) }}</span>
          <span class="badge badge-sm badge-error badge-outline">{{ t('agent.ruleCount', { count: ruleCount }) }}</span>
        </div>
      </div>
      <div class="flex min-w-[280px] flex-col items-stretch gap-3 rounded-2xl border border-base-300 bg-base-100/80 p-3">
        <div class="flex flex-wrap items-center justify-between gap-2">
          <div>
            <div class="text-xs font-semibold uppercase tracking-[0.18em] text-base-content/45">
              {{ t('agent.quickActionsTitle') }}
            </div>
            <div class="text-[11px] text-base-content/55">
              {{ t('agent.quickActionsDescription') }}
            </div>
          </div>
          <span class="badge badge-sm badge-outline">{{ currentViewModeLabel }}</span>
        </div>
        <div class="flex flex-wrap gap-2">
          <button @click="$emit('start-create')" class="btn btn-sm btn-primary">
            <i class="fas fa-plus mr-1"></i>
            {{ t('agent.createSkill') }}
          </button>
          <button
            @click="$emit('refresh-all')"
            class="btn btn-sm btn-outline"
            :disabled="loading || loadingCandidates"
          >
            <i :class="loading || loadingCandidates ? 'fas fa-spinner fa-spin mr-1' : 'fas fa-rotate-right mr-1'"></i>
            {{ t('agent.refreshAll') }}
          </button>
        </div>
        <div
          v-if="!embedded"
          class="flex items-center justify-end gap-2 border-t border-base-300 pt-3"
        >
          <button
            @click="$emit('toggle-fullscreen')"
            class="btn btn-sm btn-ghost btn-circle"
            :title="isFullscreen ? t('common.exitFullscreen') : t('common.fullscreen')"
          >
            <i :class="isFullscreen ? 'fas fa-compress' : 'fas fa-expand'"></i>
          </button>
          <button @click="$emit('close')" class="btn btn-sm btn-ghost btn-circle">
            <i class="fas fa-times"></i>
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'

defineProps<{
  isFullscreen: boolean
  embedded: boolean
  loading: boolean
  loadingCandidates: boolean
  skillsCount: number
  enabledSkillCount: number
  disabledSkillCount: number
  skillsWithContentCount: number
  activeDraftCount: number
  ruleCount: number
  currentViewModeLabel: string
}>()

defineEmits<{
  'start-create': []
  'refresh-all': []
  'toggle-fullscreen': []
  'close': []
}>()

const { t } = useI18n()
</script>
