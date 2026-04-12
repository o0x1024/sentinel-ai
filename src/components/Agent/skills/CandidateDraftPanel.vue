<template>
  <div class="card bg-base-200 border border-base-300 p-4">
    <div class="mb-3 flex items-center justify-between gap-3">
      <div>
        <div class="text-sm font-semibold">{{ t('agent.candidateDraftDetailsTitle') }}</div>
        <div class="text-xs text-base-content/60">
          {{ t('agent.candidateDraftDetailsDescription') }}
        </div>
      </div>
      <div class="flex items-center gap-2">
        <label class="label cursor-pointer gap-2 py-0">
          <span class="label-text text-xs">{{ t('agent.showReviewed') }}</span>
          <input
            :checked="showReviewedCandidates"
            type="checkbox"
            class="toggle toggle-xs"
            @change="$emit('update:show-reviewed-candidates', ($event.target as HTMLInputElement).checked)"
          />
        </label>
        <span class="badge badge-sm badge-neutral badge-outline">{{ visibleSkillCandidates.length }}</span>
      </div>
    </div>
    <div v-if="loadingCandidates" class="flex justify-center py-6">
      <span class="loading loading-spinner loading-sm"></span>
    </div>
    <div v-else-if="visibleSkillCandidates.length === 0" class="py-4 text-center text-xs text-base-content/60">
      {{ t('agent.noCandidateDraftsInFilter') }}
    </div>
    <div v-else class="space-y-3">
      <div
        v-for="candidate in visibleSkillCandidates"
        :key="candidate.id"
        class="rounded-xl border border-base-300 bg-base-100 p-3"
      >
        <div class="flex items-start justify-between gap-3">
          <div class="min-w-0 flex-1">
            <div class="font-medium">{{ candidate.title }}</div>
            <div class="mt-1 text-xs text-base-content/60">{{ candidate.description }}</div>
            <div class="mt-2 truncate font-mono text-[10px] text-base-content/50">{{ candidate.id }}</div>
          </div>
          <div class="flex shrink-0 items-center gap-2">
            <button class="btn btn-xs btn-ghost" @click="$emit('use-draft', candidate)">
              <i class="fas fa-pen mr-1"></i>
              {{ t('agent.useAsDraft') }}
            </button>
            <button
              v-if="candidate.status === 'draft'"
              class="btn btn-xs btn-info btn-outline"
              :disabled="aiGenerating"
              @click="$emit('refine-ai', candidate)"
            >
              <i :class="aiGenerating ? 'fas fa-spinner fa-spin mr-1' : 'fas fa-magic mr-1'"></i>
              {{ t('agent.aiRefine') }}
            </button>
            <button
              v-if="candidate.status === 'draft'"
              class="btn btn-xs btn-warning btn-outline"
              :disabled="reviewingCandidateIds.includes(candidate.id)"
              @click="$emit('review', candidate, 'archived')"
            >
              <i
                :class="reviewingCandidateIds.includes(candidate.id)
                  ? 'fas fa-spinner fa-spin mr-1'
                  : 'fas fa-box-archive mr-1'"
              ></i>
              {{ t('agent.archiveCandidate') }}
            </button>
            <button
              class="btn btn-xs btn-primary"
              :disabled="promotingCandidateIds.includes(candidate.id) || candidate.status !== 'draft'"
              @click="$emit('promote', candidate)"
            >
              <i
                :class="promotingCandidateIds.includes(candidate.id)
                  ? 'fas fa-spinner fa-spin mr-1'
                  : 'fas fa-arrow-up-right-from-square mr-1'"
              ></i>
              {{ candidate.status === 'promoted' ? getCandidateStatusLabel(candidate.status) : t('agent.promoteCandidate') }}
            </button>
          </div>
        </div>
        <div class="mt-3 flex flex-wrap gap-2">
          <span class="badge badge-sm badge-outline">{{ candidate.memory_kind }}</span>
          <span class="badge badge-sm badge-outline">{{ candidate.scope }}</span>
          <span class="badge badge-sm badge-outline">{{ candidate.source }}</span>
          <span class="badge badge-sm badge-outline">{{ candidate.stability }}</span>
          <span class="badge badge-sm" :class="candidateStatusBadgeClass(candidate.status)">
            {{ getCandidateStatusLabel(candidate.status) }}
          </span>
          <span class="badge badge-sm badge-ghost">
            {{ t('agent.candidateConfidence', { confidence: candidate.confidence.toFixed(2) }) }}
          </span>
          <span v-if="candidate.promoted_skill_id" class="badge badge-sm badge-success badge-outline">
            {{ candidate.promoted_skill_id }}
          </span>
        </div>
        <details
          v-if="candidate.status === 'draft'"
          class="mt-3 rounded-lg border border-base-300 bg-base-200/40 px-3 py-2"
        >
          <summary class="cursor-pointer text-[11px] font-medium text-base-content/70">
            {{ t('agent.rejectAs') }}
          </summary>
          <div class="mt-3 flex flex-wrap gap-2">
            <button
              v-for="option in rejectionCategoryOptions"
              :key="option.category"
              class="btn btn-xs btn-outline"
              :class="option.buttonClass"
              :disabled="reviewingCandidateIds.includes(candidate.id)"
              @click="$emit('review', candidate, 'rejected', option.category)"
            >
              <i
                :class="reviewingCandidateIds.includes(candidate.id)
                  ? 'fas fa-spinner fa-spin mr-1'
                  : `${option.icon} mr-1`"
              ></i>
              {{ option.label }}
            </button>
          </div>
          <div class="mt-2 text-[11px] text-base-content/55">
            {{ t('agent.structuredRejectionReasonHint') }}
          </div>
        </details>
        <div class="mt-3 rounded-lg bg-base-200/60 px-3 py-2">
          <div class="text-[11px] font-medium text-base-content/70">{{ t('agent.provenance') }}</div>
          <div class="mt-2 space-y-2 text-xs text-base-content/70">
            <div v-if="candidate.source_memory_id" class="break-all font-mono text-[11px] text-base-content/55">
              {{ t('agent.sourceMemory') }}: {{ candidate.source_memory_id }}
            </div>
            <div class="rounded-md bg-base-100/80 px-2 py-2 text-base-content/75">
              {{ candidate.memory_excerpt || candidate.description }}
            </div>
            <ul class="space-y-1">
              <li
                v-for="reason in candidate.capture_reasons"
                :key="reason"
                class="flex items-start gap-2"
              >
                <span class="mt-1 h-1.5 w-1.5 shrink-0 rounded-full bg-info"></span>
                <span>{{ reason }}</span>
              </li>
            </ul>
          </div>
        </div>
        <div class="mt-3 rounded-lg bg-base-200/60 px-3 py-2">
          <div class="text-[11px] font-medium text-base-content/70">{{ t('agent.reviewTimeline') }}</div>
          <div class="mt-2 space-y-2 text-xs text-base-content/70">
            <div class="flex items-start gap-2">
              <span class="mt-1 h-2 w-2 shrink-0 rounded-full bg-info"></span>
              <div>
                <div class="font-medium text-base-content/80">{{ t('agent.candidateCreated') }}</div>
                <div>{{ formatDateTime(candidate.created_at_ms) }}</div>
              </div>
            </div>
            <div v-if="candidate.reviewed_at_ms" class="flex items-start gap-2">
              <span
                class="mt-1 h-2 w-2 shrink-0 rounded-full"
                :class="candidateReviewDotClass(candidate.status)"
              ></span>
              <div>
                <div class="font-medium text-base-content/80">
                  {{ getCandidateStatusLabel(candidate.status) }}
                </div>
                <div>{{ formatDateTime(candidate.reviewed_at_ms) }}</div>
                <div v-if="candidate.review_note" class="mt-1 text-base-content/60">
                  {{ candidate.review_note }}
                </div>
              </div>
            </div>
            <div v-if="candidate.promoted_skill_id" class="flex items-start gap-2">
              <span class="mt-1 h-2 w-2 shrink-0 rounded-full bg-success"></span>
              <div>
                <div class="font-medium text-base-content/80">{{ t('agent.publishedAsSkill') }}</div>
                <div class="font-mono text-[11px]">{{ candidate.promoted_skill_id }}</div>
              </div>
            </div>
          </div>
        </div>
        <details class="mt-3 rounded-lg bg-base-200/60 px-3 py-2">
          <summary class="cursor-pointer text-xs font-medium text-base-content/70">
            {{ t('agent.draftContent') }}
          </summary>
          <pre class="mt-2 whitespace-pre-wrap text-xs leading-5 text-base-content/80">{{ candidate.content }}</pre>
        </details>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import {
  candidateReviewDotClass,
  candidateStatusBadgeClass,
  candidateStatusLabel,
  formatCandidateDateTime,
} from '../skillsManagerHelpers'
import type {
  RejectionCategoryOption,
  SkillCandidate,
  SuppressionRuleCategory,
} from './skillsManagerTypes'

defineProps<{
  loadingCandidates: boolean
  showReviewedCandidates: boolean
  visibleSkillCandidates: SkillCandidate[]
  promotingCandidateIds: string[]
  reviewingCandidateIds: string[]
  aiGenerating: boolean
  rejectionCategoryOptions: RejectionCategoryOption[]
}>()

defineEmits<{
  'update:show-reviewed-candidates': [value: boolean]
  'use-draft': [candidate: SkillCandidate]
  'refine-ai': [candidate: SkillCandidate]
  'promote': [candidate: SkillCandidate]
  'review': [
    candidate: SkillCandidate,
    status: 'rejected' | 'archived',
    suppressionCategory?: SuppressionRuleCategory
  ]
}>()

const { t, locale } = useI18n()

const formatDateTime = (timestamp?: number | null) =>
  formatCandidateDateTime(timestamp, locale.value)

const getCandidateStatusLabel = (status: string) => candidateStatusLabel(t, status)
</script>
