<template>
  <Teleport to="body">
    <div v-if="open" class="modal modal-open">
      <div class="modal-box max-w-6xl max-h-[90vh] overflow-y-auto overflow-x-hidden">
        <div class="flex items-start justify-between gap-4">
          <div>
            <h3 class="font-bold text-lg">{{ t('bugBounty.monitor.seeds.title') }}</h3>
            <p class="mt-1 text-sm text-base-content/60">
              {{ t('bugBounty.monitor.seeds.description') }}
            </p>
          </div>
          <button type="button" class="btn btn-ghost btn-sm btn-circle" @click="emit('close')">
            <i class="fas fa-times"></i>
          </button>
        </div>

        <div class="mt-4">
      <div class="flex flex-wrap items-center justify-between gap-3">
        <div class="flex flex-wrap items-center gap-2">
          <select
            v-if="(props.programs || []).length > 1"
            v-model="localProgramId"
            class="select select-bordered select-sm w-56"
          >
            <option value="">{{ t('bugBounty.program.selectProgramPlaceholder') }}</option>
            <option v-for="program in props.programs || []" :key="program.id" :value="program.id">
              {{ program.name }}
            </option>
          </select>
          <button
            type="button"
            class="btn btn-sm btn-outline"
            :disabled="!resolvedProgramId || syncing"
            @click="syncSeedsFromAssets"
          >
            <span v-if="syncing" class="loading loading-spinner loading-xs"></span>
            <i v-else class="fas fa-rotate"></i>
            {{ t('bugBounty.monitor.seeds.generateCandidates') }}
          </button>
          <button
            type="button"
            class="btn btn-sm btn-outline"
            :disabled="!resolvedProgramId"
            @click="openCandidatesModal"
          >
            <i class="fas fa-inbox"></i>
            {{ t('bugBounty.monitor.seeds.openCandidates') }}
            <span class="badge badge-sm badge-primary">{{ candidateGroups.length }}</span>
          </button>
          <button
            type="button"
            class="btn btn-sm btn-primary"
            :disabled="!resolvedProgramId"
            @click="openCreateModal"
          >
            <i class="fas fa-plus mr-1"></i>
            {{ t('bugBounty.monitor.seeds.addSeed') }}
          </button>
        </div>
      </div>

      <div v-if="!resolvedProgramId" class="alert mt-4">
        <i class="fas fa-circle-info"></i>
        <span>{{ t('bugBounty.monitor.seeds.selectProgramFirst') }}</span>
      </div>

      <div v-else-if="loading" class="flex justify-center py-8">
        <span class="loading loading-spinner loading-lg"></span>
      </div>

        <div v-else class="mt-4 space-y-3">
          <div class="rounded-lg border border-base-300 bg-base-100 p-3">
            <div class="grid gap-3 xl:grid-cols-[minmax(0,1fr)_180px_180px_180px_auto]">
            <label class="input input-bordered input-sm flex items-center gap-2">
              <i class="fas fa-search text-base-content/50"></i>
              <input
                v-model.trim="searchQuery"
                type="text"
                class="grow"
                :placeholder="t('bugBounty.monitor.seeds.searchPlaceholder')"
              />
            </label>
            <select v-model="typeFilter" class="select select-bordered select-sm w-full">
              <option value="all">{{ t('bugBounty.monitor.seeds.allTypes') }}</option>
              <option v-for="option in seedTypeOptions" :key="option.value" :value="option.value">
                {{ option.label }}
              </option>
            </select>
            <select v-model="statusFilter" class="select select-bordered select-sm w-full">
              <option value="all">{{ t('bugBounty.monitor.seeds.allStatuses') }}</option>
              <option value="active">{{ t('bugBounty.monitor.seeds.active') }}</option>
              <option value="disabled">{{ t('bugBounty.monitor.seeds.disabled') }}</option>
            </select>
            <select v-model="sourceFilter" class="select select-bordered select-sm w-full">
              <option value="all">{{ t('bugBounty.monitor.seeds.allSources') }}</option>
              <option v-for="item in sourceDistribution" :key="item.key" :value="item.key">
                {{ item.label }}
              </option>
            </select>
            <button type="button" class="btn btn-sm btn-ghost" @click="resetFilters">
              {{ t('bugBounty.monitor.seeds.clearFilters') }}
            </button>
          </div>

          <div v-if="hasDistribution" class="mt-3 rounded-lg bg-base-200/60 px-3 py-2">
            <div class="flex flex-wrap items-center gap-2 text-xs">
              <span class="text-base-content/50">{{ t('bugBounty.monitor.seeds.typeDistribution') }}</span>
              <button
                v-for="item in typeDistribution"
                :key="item.key"
                type="button"
                class="badge badge-sm badge-outline"
                :class="typeFilter === item.key ? 'badge-primary' : 'badge-ghost'"
                @click="typeFilter = typeFilter === item.key ? 'all' : item.key"
              >
                {{ item.label }} · {{ item.count }}
              </button>
              <span class="ml-2 text-base-content/50">{{ t('bugBounty.monitor.seeds.statusDistribution') }}</span>
              <button
                v-for="item in statusDistribution"
                :key="item.key"
                type="button"
                class="badge badge-sm badge-outline"
                :class="statusFilter === item.key ? 'badge-primary' : 'badge-ghost'"
                @click="statusFilter = statusFilter === item.key ? 'all' : item.key"
              >
                {{ item.label }} · {{ item.count }}
              </button>
              <span class="ml-2 text-base-content/50">{{ t('bugBounty.monitor.seeds.sourceDistribution') }}</span>
              <button
                v-for="item in sourceDistribution"
                :key="item.key"
                type="button"
                class="badge badge-sm badge-outline"
                :class="sourceFilter === item.key ? 'badge-primary' : 'badge-ghost'"
                @click="sourceFilter = sourceFilter === item.key ? 'all' : item.key"
              >
                {{ item.label }} · {{ item.count }}
              </button>
            </div>
            </div>
          </div>

          <div
            v-if="legacySyncedSeedCount > 0"
            class="alert alert-warning rounded-lg border border-warning/30"
          >
            <i class="fas fa-triangle-exclamation"></i>
            <div class="min-w-0">
              <div class="font-medium">{{ t('bugBounty.monitor.seeds.legacySyncedTitle') }}</div>
              <div class="text-sm text-base-content/70">
                {{ t('bugBounty.monitor.seeds.legacySyncedDescription', { count: legacySyncedSeedCount }) }}
              </div>
            </div>
            <button
              type="button"
              class="btn btn-sm btn-warning"
              :disabled="retiringLegacySynced"
              @click="retireLegacySyncedSeeds"
            >
              <span v-if="retiringLegacySynced" class="loading loading-spinner loading-xs"></span>
              {{ t('bugBounty.monitor.seeds.retireLegacySynced') }}
            </button>
          </div>

          <div
            v-if="totalSeeds > 0 && selectedSeedIds.length > 0"
            class="flex flex-col gap-3 rounded-lg border border-base-300 bg-base-200/40 p-3 lg:flex-row lg:items-center lg:justify-between"
          >
            <div class="text-sm text-base-content/70">
              {{ t('bugBounty.monitor.seeds.selectionSummary', { selected: selectedSeedIds.length, page: seeds.length, total: totalSeeds }) }}
            </div>
            <div class="flex flex-wrap items-center gap-2">
              <button
                type="button"
                class="btn btn-sm btn-warning"
                :disabled="!selectedSeedIds.length || loading || batchDeleting"
                @click="deleteSelectedSeeds"
              >
                <span v-if="batchDeleting" class="loading loading-spinner loading-xs"></span>
                {{ t('bugBounty.monitor.seeds.deleteSelected') }}
              </button>
              <button
                type="button"
                class="btn btn-sm btn-error btn-outline"
                :disabled="!selectedSeedIds.length || loading || batchDeleting"
                @click="deleteAllSeeds"
              >
                <span v-if="batchDeleting" class="loading loading-spinner loading-xs"></span>
                {{ t('bugBounty.monitor.seeds.deleteAll') }}
              </button>
            </div>
          </div>

          <div v-if="totalSeeds === 0 && !hasActiveFilters" class="text-center py-8 text-base-content/60">
            <i class="fas fa-seedling text-3xl mb-3"></i>
            <p>{{ t('bugBounty.monitor.seeds.empty') }}</p>
        </div>

        <div v-else-if="totalSeeds === 0" class="text-center py-8 text-base-content/60">
          <i class="fas fa-filter text-3xl mb-3"></i>
          <p>{{ t('bugBounty.monitor.seeds.emptyFiltered') }}</p>
        </div>

        <div v-else class="overflow-x-auto rounded-lg border border-base-300">
          <table class="table table-sm">
            <thead>
              <tr>
                <th class="w-12">
                  <input
                    type="checkbox"
                    class="checkbox checkbox-sm"
                    :checked="allCurrentPageSelected"
                    :disabled="!seeds.length || loading"
                    @click.stop
                    @change="toggleSelectCurrentPage"
                  />
                </th>
                <th>{{ t('bugBounty.monitor.seeds.type') }}</th>
                <th>{{ t('bugBounty.monitor.seeds.value') }}</th>
                <th>{{ t('bugBounty.monitor.seeds.status') }}</th>
                <th>{{ t('bugBounty.monitor.seeds.source') }}</th>
                <th>{{ t('bugBounty.monitor.seeds.confidence') }}</th>
                <th class="w-28 text-right">{{ t('common.actions') }}</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="seed in seeds" :key="seed.id">
                <td @click.stop>
                  <input
                    type="checkbox"
                    class="checkbox checkbox-sm"
                    :checked="selectedSeedIdSet.has(seed.id)"
                    @change="toggleSeedSelection(seed.id)"
                  />
                </td>
                <td>
                  <span class="font-medium">{{ seedTypeLabel(seed.seed_type) }}</span>
                </td>
                <td class="max-w-[360px]">
                  <div class="truncate font-mono text-xs" :title="seed.seed_value">{{ seed.seed_value }}</div>
                </td>
                <td>
                  <span class="badge badge-sm" :class="seed.status === 'active' ? 'badge-success badge-outline' : 'badge-ghost'">
                    {{ seed.status === 'active' ? t('bugBounty.monitor.seeds.active') : t('bugBounty.monitor.seeds.disabled') }}
                  </span>
                </td>
                <td>{{ sourceLabel(seed.source) }}</td>
                <td>{{ formatConfidence(seed.confidence_score) }}</td>
                <td>
                  <div class="flex justify-end gap-1">
                    <button type="button" class="btn btn-ghost btn-xs" @click="openEditModal(seed)">
                      <i class="fas fa-pen"></i>
                    </button>
                    <button type="button" class="btn btn-ghost btn-xs text-error" @click="openDeleteModal(seed)">
                      <i class="fas fa-trash"></i>
                    </button>
                  </div>
                </td>
              </tr>
            </tbody>
          </table>
        </div>

        <div
          v-if="totalSeeds > 0"
          class="flex flex-col gap-3 rounded-lg border border-base-300 bg-base-100 px-3 py-3 xl:flex-row xl:items-center xl:justify-between"
        >
          <div class="flex items-center gap-2 text-sm">
            <span class="text-base-content/70">{{ t('bugBounty.surface.inventory.pageSizeLabel') }}</span>
            <select v-model.number="pageSize" class="select select-bordered select-sm">
              <option v-for="size in pageSizeOptions" :key="size" :value="size">
                {{ size }}
              </option>
            </select>
          </div>

          <div class="flex flex-col gap-2 sm:flex-row sm:flex-wrap sm:items-center sm:justify-end">
            <div class="join">
              <button class="join-item btn btn-sm" :disabled="currentPage <= 1" @click="goToFirstPage">
                {{ t('bugBounty.surface.inventory.firstPage') }}
              </button>
              <button class="join-item btn btn-sm" :disabled="currentPage <= 1" @click="goToPreviousPage">
                {{ t('common.previous') }}
              </button>
              <button class="join-item btn btn-sm">
                {{ t('bugBounty.surface.inventory.pageInfo', { page: currentPage, total: pageCount }) }}
              </button>
              <button class="join-item btn btn-sm" :disabled="currentPage >= pageCount" @click="goToNextPage">
                {{ t('common.next') }}
              </button>
              <button class="join-item btn btn-sm" :disabled="currentPage >= pageCount" @click="goToLastPage">
                {{ t('bugBounty.surface.inventory.lastPage') }}
              </button>
            </div>

            <div class="flex items-center gap-2">
              <input
                v-model="pageInput"
                type="number"
                min="1"
                :max="pageCount"
                class="input input-bordered input-sm w-24"
                :placeholder="t('bugBounty.surface.inventory.jumpPlaceholder')"
                @keyup.enter="applyPageJump"
              />
              <button class="btn btn-sm btn-outline" @click="applyPageJump">
                {{ t('bugBounty.surface.inventory.jump') }}
              </button>
            </div>
          </div>
        </div>
      </div>
        </div>
      </div>
      <div class="modal-backdrop" @click="emit('close')"></div>
    </div>

    <div v-if="open && showCandidatesModal" class="modal modal-open">
      <div class="modal-box max-w-6xl max-h-[90vh] overflow-y-auto overflow-x-hidden">
        <div class="flex items-start justify-between gap-4">
          <div>
            <h3 class="font-bold text-lg">{{ t('bugBounty.monitor.seeds.candidateDialogTitle') }}</h3>
            <p class="mt-1 text-sm text-base-content/60">
              {{ t('bugBounty.monitor.seeds.candidatesDescription') }}
            </p>
          </div>
          <button type="button" class="btn btn-ghost btn-sm btn-circle" @click="closeCandidatesModal">
            <i class="fas fa-times"></i>
          </button>
        </div>

        <div class="mt-4 flex flex-wrap items-center justify-between gap-3">
          <div class="flex flex-wrap items-center gap-2">
            <select
              v-model="candidateStatusFilter"
              class="select select-bordered select-sm w-40"
              :disabled="candidateLoading || candidateRefreshing || candidateReviewing"
            >
              <option value="pending">{{ t('bugBounty.monitor.seeds.candidateStatusPending') }}</option>
              <option value="rejected">{{ t('bugBounty.monitor.seeds.candidateStatusRejected') }}</option>
            </select>
            <div class="badge badge-outline badge-sm">
              {{ candidateStatusCountLabel }}
            </div>
          </div>
          <span v-if="candidateRefreshing" class="inline-flex items-center gap-2 text-xs text-base-content/50">
            <span class="loading loading-spinner loading-xs"></span>
            {{ t('common.loading') }}
          </span>
        </div>

        <div v-if="candidateLoading" class="flex justify-center py-10">
          <span class="loading loading-spinner loading-lg"></span>
        </div>

        <template v-else-if="candidateGroups.length > 0">
          <div
            v-if="selectedCandidateIds.length > 0"
            class="mt-4 flex flex-col gap-3 rounded-lg border border-base-300 bg-base-200/40 p-3 lg:flex-row lg:items-center lg:justify-between"
          >
            <div class="text-sm text-base-content/70">
              {{ t('bugBounty.monitor.seeds.candidateSelectionSummary', { selected: selectedCandidateIds.length, total: candidateGroups.length }) }}
            </div>
            <div class="flex flex-wrap items-center gap-2">
              <button
                type="button"
                class="btn btn-sm btn-success"
                :disabled="candidateReviewing"
                @click="reviewSeedCandidates(selectedCandidateIds, 'approved')"
              >
                <span v-if="candidateReviewing" class="loading loading-spinner loading-xs"></span>
                {{ t('bugBounty.monitor.seeds.approveSelected') }}
              </button>
              <button
                v-if="canRejectVisibleCandidates"
                type="button"
                class="btn btn-sm btn-error"
                :disabled="candidateReviewing"
                @click="reviewSeedCandidates(selectedCandidateIds, 'rejected')"
              >
                <span v-if="candidateReviewing" class="loading loading-spinner loading-xs"></span>
                {{ t('bugBounty.monitor.seeds.rejectSelected') }}
              </button>
            </div>
          </div>

          <div class="mt-4 overflow-x-auto rounded-lg border border-base-300">
            <table class="table table-sm">
              <thead>
                <tr>
                  <th class="w-12">
                    <input
                      type="checkbox"
                      class="checkbox checkbox-sm"
                      :checked="allPendingCandidatesSelected"
                      :disabled="!pendingCandidates.length || candidateReviewing"
                      @click.stop
                      @change="toggleSelectAllCandidates"
                    />
                  </th>
                  <th>{{ t('bugBounty.monitor.seeds.type') }}</th>
                  <th>{{ t('bugBounty.monitor.seeds.value') }}</th>
                  <th>{{ t('bugBounty.monitor.seeds.candidateSourceCount') }}</th>
                  <th>{{ t('bugBounty.monitor.seeds.candidateLatestSource') }}</th>
                  <th>{{ t('bugBounty.monitor.seeds.confidence') }}</th>
                  <th class="w-48 text-right">{{ t('common.actions') }}</th>
                </tr>
              </thead>
              <tbody>
                <template v-for="group in candidateGroups" :key="group.key">
                  <tr>
                    <td @click.stop>
                      <input
                        type="checkbox"
                        class="checkbox checkbox-sm"
                        :checked="isCandidateGroupSelected(group)"
                        @change="toggleCandidateGroupSelection(group)"
                      />
                    </td>
                    <td>
                      <span class="font-medium">{{ seedTypeLabel(group.seed_type) }}</span>
                    </td>
                    <td class="max-w-[260px]">
                      <div class="truncate font-mono text-xs" :title="group.seed_value">{{ group.seed_value }}</div>
                    </td>
                    <td>
                      <span class="badge badge-sm badge-outline">{{ group.source_count }}</span>
                    </td>
                    <td class="max-w-[320px]">
                      <a
                        v-if="group.latest_source_canonical_url"
                        :href="group.latest_source_canonical_url"
                        target="_blank"
                        rel="noopener noreferrer"
                        class="link link-primary break-all text-xs"
                      >
                        {{ group.latest_source_display_value }}
                      </a>
                      <div v-else class="break-all text-xs">{{ group.latest_source_display_value }}</div>
                    </td>
                    <td>{{ formatConfidence(group.max_confidence_score) }}</td>
                    <td>
                      <div class="flex justify-end gap-1 flex-wrap">
                        <button
                          type="button"
                          class="btn btn-ghost btn-xs"
                          @click="toggleCandidateGroupExpanded(group.key)"
                        >
                          {{ isCandidateGroupExpanded(group.key) ? t('bugBounty.monitor.seeds.hideSources') : t('bugBounty.monitor.seeds.showSources') }}
                        </button>
                        <button
                          type="button"
                          class="btn btn-ghost btn-xs text-success"
                          :disabled="candidateReviewing"
                          @click="reviewSeedCandidates(group.candidate_ids, 'approved')"
                        >
                          <i class="fas fa-check"></i>
                        </button>
                        <button
                          v-if="canRejectVisibleCandidates"
                          type="button"
                          class="btn btn-ghost btn-xs text-error"
                          :disabled="candidateReviewing"
                          @click="reviewSeedCandidates(group.candidate_ids, 'rejected')"
                        >
                          <i class="fas fa-ban"></i>
                        </button>
                      </div>
                    </td>
                  </tr>
                  <tr v-if="isCandidateGroupExpanded(group.key)">
                    <td colspan="7" class="bg-base-200/30">
                      <div class="space-y-2 py-1">
                        <div
                          v-for="candidate in group.items"
                          :key="candidate.id"
                          class="flex flex-col gap-2 rounded-lg border border-base-300 bg-base-100 p-3 lg:flex-row lg:items-center lg:justify-between"
                        >
                          <div class="flex items-start gap-3">
                            <input
                              type="checkbox"
                              class="checkbox checkbox-sm mt-0.5"
                              :checked="selectedCandidateIdSet.has(candidate.id)"
                              @change="toggleCandidateSelection(candidate.id)"
                            />
                            <div class="space-y-1">
                              <div class="text-xs text-base-content/50">
                                {{ t('bugBounty.monitor.seeds.candidateSource') }}
                              </div>
                              <a
                                v-if="candidate.source_canonical_url"
                                :href="candidate.source_canonical_url"
                                target="_blank"
                                rel="noopener noreferrer"
                                class="link link-primary break-all text-xs"
                              >
                                {{ candidate.source_display_value }}
                              </a>
                              <div v-else class="break-all text-xs">{{ candidate.source_display_value }}</div>
                              <div class="text-[11px] text-base-content/50">
                                {{ t('bugBounty.monitor.seeds.candidateObservedAt', { time: candidate.observed_at }) }}
                              </div>
                            </div>
                          </div>
                          <div class="flex items-center justify-end gap-1">
                            <button
                              type="button"
                              class="btn btn-ghost btn-xs text-success"
                              :disabled="candidateReviewing"
                              @click="reviewSeedCandidates([candidate.id], 'approved')"
                            >
                              <i class="fas fa-check"></i>
                            </button>
                            <button
                              v-if="canRejectVisibleCandidates"
                              type="button"
                              class="btn btn-ghost btn-xs text-error"
                              :disabled="candidateReviewing"
                              @click="reviewSeedCandidates([candidate.id], 'rejected')"
                            >
                              <i class="fas fa-ban"></i>
                            </button>
                          </div>
                        </div>
                      </div>
                    </td>
                  </tr>
                </template>
              </tbody>
            </table>
          </div>
        </template>

        <div v-else class="py-10 text-center text-sm text-base-content/60">
          {{ t('bugBounty.monitor.seeds.emptyCandidates') }}
        </div>
      </div>
      <div class="modal-backdrop" @click="showCandidatesModal = false"></div>
    </div>

    <div v-if="open && showEditorModal" class="modal modal-open">
      <div class="modal-box max-w-lg">
        <h3 class="font-bold text-lg mb-4">
          {{ editingSeed ? t('bugBounty.monitor.seeds.editSeed') : t('bugBounty.monitor.seeds.addSeed') }}
        </h3>
        <div class="space-y-4">
          <div class="form-control">
            <label class="label">
              <span class="label-text">{{ t('bugBounty.monitor.seeds.type') }}</span>
            </label>
            <select v-model="seedForm.seed_type" class="select select-bordered">
              <option v-for="option in seedTypeOptions" :key="option.value" :value="option.value">
                {{ option.label }}
              </option>
            </select>
          </div>
          <div class="form-control">
            <label class="label">
              <span class="label-text">{{ t('bugBounty.monitor.seeds.value') }}</span>
            </label>
            <input
              v-model="seedForm.seed_value"
              type="text"
              class="input input-bordered"
              :placeholder="t('bugBounty.monitor.seeds.valuePlaceholder')"
            />
          </div>
          <div class="form-control">
            <label class="label">
              <span class="label-text">{{ t('bugBounty.monitor.seeds.status') }}</span>
            </label>
            <select v-model="seedForm.status" class="select select-bordered">
              <option value="active">{{ t('bugBounty.monitor.seeds.active') }}</option>
              <option value="disabled">{{ t('bugBounty.monitor.seeds.disabled') }}</option>
            </select>
          </div>
        </div>
        <div class="modal-action">
          <button type="button" class="btn" @click="closeEditorModal">
            {{ t('common.cancel') }}
          </button>
          <button type="button" class="btn btn-primary" :disabled="saving" @click="saveSeed">
            <span v-if="saving" class="loading loading-spinner loading-xs"></span>
            {{ t('common.save') }}
          </button>
        </div>
      </div>
      <div class="modal-backdrop" @click="closeEditorModal"></div>
    </div>

    <div v-if="open && seedPendingDelete" class="modal modal-open">
      <div class="modal-box max-w-md">
        <h3 class="font-bold text-lg">{{ t('bugBounty.monitor.seeds.deleteTitle') }}</h3>
        <p class="py-4 text-sm text-base-content/70">
          {{ t('bugBounty.monitor.seeds.deleteConfirm') }}
        </p>
        <div class="rounded-md bg-base-200 px-3 py-2 font-mono text-xs">
          {{ seedPendingDelete.seed_value }}
        </div>
        <div class="modal-action">
          <button type="button" class="btn" @click="seedPendingDelete = null">
            {{ t('common.cancel') }}
          </button>
          <button type="button" class="btn btn-error" :disabled="deleting" @click="deleteSeed">
            <span v-if="deleting" class="loading loading-spinner loading-xs"></span>
            {{ t('common.delete') }}
          </button>
        </div>
      </div>
      <div class="modal-backdrop" @click="seedPendingDelete = null"></div>
    </div>
  </Teleport>
</template>

<script setup lang="ts">
import { computed, reactive, ref, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useI18n } from 'vue-i18n'
import { dialog } from '../../composables/useDialog'
import { useToast } from '../../composables/useToast'

defineOptions({ name: 'MonitorSeedsPanel' })

const props = defineProps<{
  open: boolean
  selectedProgram?: any
  programs?: any[]
}>()

const emit = defineEmits<{
  (e: 'close'): void
}>()

interface SurfaceSeedRow {
  id: string
  program_id: string
  seed_type: string
  seed_value: string
  status: string
  source?: string | null
  confidence_score?: number | null
  last_run_at?: string | null
  metadata_json?: string | null
  created_at: string
  updated_at: string
}

interface SurfaceSeedCandidateRow {
  id: string
  program_id: string
  seed_type: string
  seed_value: string
  status: string
  source_asset_id: string
  source_asset_type: string
  source_detail_key: string
  source_display_value: string
  source_canonical_url?: string | null
  confidence_score?: number | null
  observed_at: string
  reviewed_at?: string | null
  metadata_json?: string | null
  created_at: string
  updated_at: string
}

interface SeedSyncResult {
  requested: number
  created: number
  refreshed: number
  pending_review: number
}

interface SurfaceLegacySeedRetireResult {
  matched: number
  retired: number
}

interface DistributionBucket {
  key: string
  count: number
}

interface SurfaceSeedDistribution {
  types: DistributionBucket[]
  statuses: DistributionBucket[]
  sources: DistributionBucket[]
}

interface SurfaceSeedQueryResult {
  items: SurfaceSeedRow[]
  total: number
}

interface CandidateGroup {
  key: string
  seed_type: string
  seed_value: string
  candidate_ids: string[]
  items: SurfaceSeedCandidateRow[]
  source_count: number
  latest_source_display_value: string
  latest_source_canonical_url?: string | null
  latest_observed_at: string
  max_confidence_score?: number | null
}

const { t } = useI18n()
const toast = useToast()

const loading = ref(false)
const saving = ref(false)
const deleting = ref(false)
const syncing = ref(false)
const batchDeleting = ref(false)
const candidateLoading = ref(false)
const candidateRefreshing = ref(false)
const candidateReviewing = ref(false)
const retiringLegacySynced = ref(false)
const seeds = ref<SurfaceSeedRow[]>([])
const pendingCandidates = ref<SurfaceSeedCandidateRow[]>([])
const totalSeeds = ref(0)
const legacySyncedSeedCount = ref(0)
const distribution = ref<SurfaceSeedDistribution>({
  types: [],
  statuses: [],
  sources: [],
})
const localProgramId = ref('')
const showCandidatesModal = ref(false)
const showEditorModal = ref(false)
const editingSeed = ref<SurfaceSeedRow | null>(null)
const seedPendingDelete = ref<SurfaceSeedRow | null>(null)
const searchQuery = ref('')
const typeFilter = ref('all')
const statusFilter = ref('all')
const sourceFilter = ref('all')
const candidateStatusFilter = ref<'pending' | 'rejected'>('pending')
const currentPage = ref(1)
const pageSize = ref(20)
const pageInput = ref('')
const pageSizeOptions = [10, 20, 50, 100]
const selectedSeedIds = ref<string[]>([])
const selectedCandidateIds = ref<string[]>([])
const expandedCandidateGroupKeys = ref<string[]>([])
const seedForm = reactive({
  id: '',
  seed_type: 'root_domain',
  seed_value: '',
  status: 'active',
})

const resolvedProgramId = computed(() => String(localProgramId.value || props.selectedProgram?.id || ''))

const seedTypeOptions = computed(() => [
  { value: 'root_domain', label: t('bugBounty.monitor.seeds.types.root_domain') },
  { value: 'favicon_hash', label: t('bugBounty.monitor.seeds.types.favicon_hash') },
  { value: 'brand_keyword', label: t('bugBounty.monitor.seeds.types.brand_keyword') },
  { value: 'domain', label: t('bugBounty.monitor.seeds.types.domain') },
  { value: 'org_name', label: t('bugBounty.monitor.seeds.types.org_name') },
  { value: 'asn', label: t('bugBounty.monitor.seeds.types.asn') },
  { value: 'cname_keyword', label: t('bugBounty.monitor.seeds.types.cname_keyword') },
  { value: 'title_keyword', label: t('bugBounty.monitor.seeds.types.title_keyword') },
  { value: 'body_keyword', label: t('bugBounty.monitor.seeds.types.body_keyword') },
  { value: 'header_keyword', label: t('bugBounty.monitor.seeds.types.header_keyword') },
])
const typeDistribution = computed(() => distribution.value.types.map(item => ({
  key: item.key,
  label: seedTypeLabel(item.key),
  count: item.count,
})))
const statusDistribution = computed(() => distribution.value.statuses.map(item => ({
  key: item.key,
  label: item.key === 'active' ? t('bugBounty.monitor.seeds.active') : t('bugBounty.monitor.seeds.disabled'),
  count: item.count,
})))
const sourceDistribution = computed(() => distribution.value.sources.map(item => ({
  key: item.key,
  label: sourceLabel(item.key),
  count: item.count,
})))
const hasDistribution = computed(() => (
  typeDistribution.value.length > 0
  || statusDistribution.value.length > 0
  || sourceDistribution.value.length > 0
))
const hasActiveFilters = computed(() => (
  searchQuery.value.trim().length > 0
  || typeFilter.value !== 'all'
  || statusFilter.value !== 'all'
  || sourceFilter.value !== 'all'
))
const pageCount = computed(() => Math.max(1, Math.ceil(totalSeeds.value / pageSize.value)))
const currentPageSeedIds = computed(() => seeds.value.map((seed) => seed.id).filter(Boolean))
const selectedSeedIdSet = computed(() => new Set(selectedSeedIds.value))
const pendingCandidateIds = computed(() => pendingCandidates.value.map((candidate) => candidate.id).filter(Boolean))
const selectedCandidateIdSet = computed(() => new Set(selectedCandidateIds.value))
const candidateGroups = computed<CandidateGroup[]>(() => {
  const groups = new Map<string, CandidateGroup>()
  for (const candidate of pendingCandidates.value) {
    const key = `${candidate.seed_type}::${candidate.seed_value}`
    const existing = groups.get(key)
    if (existing) {
      existing.candidate_ids.push(candidate.id)
      existing.items.push(candidate)
      existing.source_count += 1
      if ((candidate.confidence_score ?? -1) > (existing.max_confidence_score ?? -1)) {
        existing.max_confidence_score = candidate.confidence_score
      }
      if (candidate.observed_at > existing.latest_observed_at) {
        existing.latest_observed_at = candidate.observed_at
        existing.latest_source_display_value = candidate.source_display_value
        existing.latest_source_canonical_url = candidate.source_canonical_url
      }
      continue
    }

    groups.set(key, {
      key,
      seed_type: candidate.seed_type,
      seed_value: candidate.seed_value,
      candidate_ids: [candidate.id],
      items: [candidate],
      source_count: 1,
      latest_source_display_value: candidate.source_display_value,
      latest_source_canonical_url: candidate.source_canonical_url,
      latest_observed_at: candidate.observed_at,
      max_confidence_score: candidate.confidence_score,
    })
  }

  return Array.from(groups.values()).sort((left, right) => (
    right.source_count - left.source_count
    || right.latest_observed_at.localeCompare(left.latest_observed_at)
    || left.seed_type.localeCompare(right.seed_type)
    || left.seed_value.localeCompare(right.seed_value)
  ))
})
const allCurrentPageSelected = computed(() =>
  currentPageSeedIds.value.length > 0
  && currentPageSeedIds.value.every((seedId) => selectedSeedIdSet.value.has(seedId))
)
const allPendingCandidatesSelected = computed(() =>
  pendingCandidateIds.value.length > 0
  && pendingCandidateIds.value.every((candidateId) => selectedCandidateIdSet.value.has(candidateId))
)
const canRejectVisibleCandidates = computed(() => candidateStatusFilter.value === 'pending')
const candidateStatusCountLabel = computed(() => {
  if (candidateStatusFilter.value === 'rejected') {
    return t('bugBounty.monitor.seeds.rejectedCount', { count: candidateGroups.value.length })
  }
  return t('bugBounty.monitor.seeds.pendingCount', { count: candidateGroups.value.length })
})

const buildSeedQueryRequest = () => ({
  program_id: resolvedProgramId.value || null,
  search: searchQuery.value || null,
  seed_type: typeFilter.value === 'all' ? null : typeFilter.value,
  status: statusFilter.value === 'all' ? null : statusFilter.value,
  source: sourceFilter.value === 'all' ? null : sourceFilter.value,
})

const loadSeeds = async () => {
  if (!resolvedProgramId.value) {
    seeds.value = []
    totalSeeds.value = 0
    selectedSeedIds.value = []
    return
  }
  try {
    loading.value = true
    const result = await invoke<SurfaceSeedQueryResult>('surface_query_seeds', {
      request: {
        ...buildSeedQueryRequest(),
        limit: pageSize.value,
        offset: (currentPage.value - 1) * pageSize.value,
      },
    })
    seeds.value = result.items
    totalSeeds.value = result.total
    selectedSeedIds.value = selectedSeedIds.value.filter((id) => currentPageSeedIds.value.includes(id))
  } catch (error) {
    console.error('Failed to load surface seeds:', error)
    seeds.value = []
    totalSeeds.value = 0
    selectedSeedIds.value = []
    toast.error(t('bugBounty.monitor.seeds.loadFailed'))
  } finally {
    loading.value = false
  }
}

const loadDistribution = async () => {
  if (!resolvedProgramId.value) {
    distribution.value = { types: [], statuses: [], sources: [] }
    return
  }
  try {
    distribution.value = await invoke<SurfaceSeedDistribution>('surface_get_seed_distribution', {
      programId: resolvedProgramId.value,
      search: searchQuery.value || null,
      seedType: typeFilter.value === 'all' ? null : typeFilter.value,
      status: statusFilter.value === 'all' ? null : statusFilter.value,
      source: sourceFilter.value === 'all' ? null : sourceFilter.value,
    })
  } catch (error) {
    console.error('Failed to load surface seed distribution:', error)
    distribution.value = { types: [], statuses: [], sources: [] }
  }
}

const loadPendingCandidates = async (options?: { silent?: boolean }) => {
  const silent = options?.silent === true
  if (!resolvedProgramId.value) {
    pendingCandidates.value = []
    selectedCandidateIds.value = []
    expandedCandidateGroupKeys.value = []
    return
  }
  try {
    if (silent) {
      candidateRefreshing.value = true
    } else {
      candidateLoading.value = true
    }
    pendingCandidates.value = await invoke<SurfaceSeedCandidateRow[]>('surface_list_seed_candidates', {
      programId: resolvedProgramId.value,
      status: candidateStatusFilter.value,
    })
    selectedCandidateIds.value = selectedCandidateIds.value.filter((id) => pendingCandidateIds.value.includes(id))
    expandedCandidateGroupKeys.value = expandedCandidateGroupKeys.value.filter((key) =>
      candidateGroups.value.some((group) => group.key === key))
  } catch (error) {
    console.error('Failed to load pending seed candidates:', error)
    pendingCandidates.value = []
    selectedCandidateIds.value = []
    expandedCandidateGroupKeys.value = []
    toast.error(t('bugBounty.monitor.seeds.candidatesLoadFailed'))
  } finally {
    if (silent) {
      candidateRefreshing.value = false
    } else {
      candidateLoading.value = false
    }
  }
}

const loadLegacySyncedSeedCount = async () => {
  if (!resolvedProgramId.value) {
    legacySyncedSeedCount.value = 0
    return
  }
  try {
    legacySyncedSeedCount.value = await invoke<number>('surface_count_legacy_synced_seeds', {
      programId: resolvedProgramId.value,
    })
  } catch (error) {
    console.error('Failed to load legacy synced seed count:', error)
    legacySyncedSeedCount.value = 0
  }
}

const resetSeedForm = () => {
  seedForm.id = ''
  seedForm.seed_type = 'root_domain'
  seedForm.seed_value = ''
  seedForm.status = 'active'
}

const ensureResolvedProgramSelection = () => {
  const programs = Array.isArray(props.programs) ? props.programs : []
  if (programs.length === 0) {
    localProgramId.value = ''
    return
  }

  if (localProgramId.value && programs.some(program => String(program.id) === localProgramId.value)) {
    return
  }

  const selectedProgramId = String(props.selectedProgram?.id || '')
  if (selectedProgramId && programs.some(program => String(program.id) === selectedProgramId)) {
    localProgramId.value = selectedProgramId
    return
  }

  localProgramId.value = String(programs[0]?.id || '')
}

const resetFilters = () => {
  searchQuery.value = ''
  typeFilter.value = 'all'
  statusFilter.value = 'all'
  sourceFilter.value = 'all'
  currentPage.value = 1
  pageInput.value = ''
  selectedSeedIds.value = []
}

const openCreateModal = () => {
  editingSeed.value = null
  resetSeedForm()
  showEditorModal.value = true
}

const openCandidatesModal = async () => {
  showCandidatesModal.value = true
  if (candidateStatusFilter.value !== 'pending') {
    candidateStatusFilter.value = 'pending'
    return
  }
  await loadPendingCandidates({ silent: true })
}

const closeCandidatesModal = () => {
  showCandidatesModal.value = false
  if (candidateStatusFilter.value !== 'pending') {
    candidateStatusFilter.value = 'pending'
  }
}

const openEditModal = (seed: SurfaceSeedRow) => {
  editingSeed.value = seed
  seedForm.id = seed.id
  seedForm.seed_type = seed.seed_type
  seedForm.seed_value = seed.seed_value
  seedForm.status = seed.status || 'active'
  showEditorModal.value = true
}

const closeEditorModal = () => {
  showEditorModal.value = false
  editingSeed.value = null
  resetSeedForm()
}

const saveSeed = async () => {
  if (!resolvedProgramId.value) return
  try {
    saving.value = true
    await invoke<SurfaceSeedRow>('surface_upsert_seed', {
      request: {
        id: seedForm.id || null,
        program_id: resolvedProgramId.value,
        seed_type: seedForm.seed_type,
        seed_value: seedForm.seed_value,
        status: seedForm.status,
        source: 'manual',
        confidence_score: 1.0,
        metadata: null,
      },
    })
    toast.success(editingSeed.value ? t('bugBounty.monitor.seeds.updated') : t('bugBounty.monitor.seeds.created'))
    closeEditorModal()
    await loadSeeds()
    await loadDistribution()
    await loadLegacySyncedSeedCount()
  } catch (error) {
    console.error('Failed to save surface seed:', error)
    toast.error(t('bugBounty.monitor.seeds.saveFailed'))
  } finally {
    saving.value = false
  }
}

const openDeleteModal = (seed: SurfaceSeedRow) => {
  seedPendingDelete.value = seed
}

const deleteSeed = async () => {
  if (!seedPendingDelete.value) return
  try {
    deleting.value = true
    await invoke('surface_delete_seed', { seedId: seedPendingDelete.value.id })
    seedPendingDelete.value = null
    toast.success(t('bugBounty.monitor.seeds.deleted'))
    await loadSeeds()
    await loadDistribution()
    await loadLegacySyncedSeedCount()
  } catch (error) {
    console.error('Failed to delete surface seed:', error)
    toast.error(t('bugBounty.monitor.seeds.deleteFailed'))
  } finally {
    deleting.value = false
  }
}

const toggleSeedSelection = (seedId: string) => {
  if (!seedId) return
  selectedSeedIds.value = selectedSeedIds.value.includes(seedId)
    ? selectedSeedIds.value.filter((id) => id !== seedId)
    : [...selectedSeedIds.value, seedId]
}

const toggleCandidateSelection = (candidateId: string) => {
  if (!candidateId) return
  selectedCandidateIds.value = selectedCandidateIds.value.includes(candidateId)
    ? selectedCandidateIds.value.filter((id) => id !== candidateId)
    : [...selectedCandidateIds.value, candidateId]
}

const toggleSelectAllCandidates = () => {
  if (!pendingCandidateIds.value.length) return
  selectedCandidateIds.value = allPendingCandidatesSelected.value ? [] : [...pendingCandidateIds.value]
}

const isCandidateGroupExpanded = (groupKey: string) => expandedCandidateGroupKeys.value.includes(groupKey)

const toggleCandidateGroupExpanded = (groupKey: string) => {
  expandedCandidateGroupKeys.value = isCandidateGroupExpanded(groupKey)
    ? expandedCandidateGroupKeys.value.filter((key) => key !== groupKey)
    : [...expandedCandidateGroupKeys.value, groupKey]
}

const isCandidateGroupSelected = (group: CandidateGroup) => (
  group.candidate_ids.length > 0
  && group.candidate_ids.every((candidateId) => selectedCandidateIdSet.value.has(candidateId))
)

const toggleCandidateGroupSelection = (group: CandidateGroup) => {
  if (!group.candidate_ids.length) return
  if (isCandidateGroupSelected(group)) {
    selectedCandidateIds.value = selectedCandidateIds.value.filter((id) => !group.candidate_ids.includes(id))
    return
  }
  selectedCandidateIds.value = Array.from(new Set([...selectedCandidateIds.value, ...group.candidate_ids]))
}

const toggleSelectCurrentPage = () => {
  if (!currentPageSeedIds.value.length) return
  selectedSeedIds.value = allCurrentPageSelected.value ? [] : [...currentPageSeedIds.value]
}

const deleteSelectedSeeds = async () => {
  if (!selectedSeedIds.value.length) return
  if (!(await dialog.confirm(t('bugBounty.monitor.seeds.confirmDeleteSelected', { count: selectedSeedIds.value.length })))) {
    return
  }

  try {
    batchDeleting.value = true
    const deleted = await invoke<number>('surface_batch_delete_seeds', { seedIds: selectedSeedIds.value })
    selectedSeedIds.value = []
    toast.success(t('bugBounty.monitor.seeds.deleteSuccess', { count: deleted }))
    await loadSeeds()
    await loadDistribution()
    await loadLegacySyncedSeedCount()
  } catch (error) {
    console.error('Failed to batch delete surface seeds:', error)
    toast.error(t('bugBounty.monitor.seeds.deleteFailed'))
  } finally {
    batchDeleting.value = false
  }
}

const deleteAllSeeds = async () => {
  if (!resolvedProgramId.value || totalSeeds.value <= 0) return
  if (!(await dialog.confirm(t('bugBounty.monitor.seeds.confirmDeleteAll', { count: totalSeeds.value })))) {
    return
  }

  try {
    batchDeleting.value = true
    const deleted = await invoke<number>('surface_delete_filtered_seeds', {
      request: buildSeedQueryRequest(),
    })
    selectedSeedIds.value = []
    toast.success(t('bugBounty.monitor.seeds.deleteAllSuccess', { count: deleted }))
    await loadSeeds()
    await loadDistribution()
    await loadLegacySyncedSeedCount()
  } catch (error) {
    console.error('Failed to delete all filtered surface seeds:', error)
    toast.error(t('bugBounty.monitor.seeds.deleteFailed'))
  } finally {
    batchDeleting.value = false
  }
}

const reviewSeedCandidates = async (candidateIds: string[], decision: 'approved' | 'rejected') => {
  if (!candidateIds.length) return
  if (decision === 'rejected' && !canRejectVisibleCandidates.value) return

  try {
    candidateReviewing.value = true
    const reviewed = await invoke<number>('surface_review_seed_candidates', {
      request: {
        candidate_ids: candidateIds,
        decision,
      },
    })
    toast.success(
      t(
        decision === 'approved'
          ? 'bugBounty.monitor.seeds.approveSuccess'
          : 'bugBounty.monitor.seeds.rejectSuccess',
        { count: reviewed },
      ),
    )
    await Promise.all([
      loadPendingCandidates({ silent: true }),
      loadSeeds(),
      loadDistribution(),
      loadLegacySyncedSeedCount(),
    ])
  } catch (error) {
    console.error('Failed to review seed candidates:', error)
    toast.error(t('bugBounty.monitor.seeds.reviewFailed'))
  } finally {
    candidateReviewing.value = false
  }
}

const retireLegacySyncedSeeds = async () => {
  if (!resolvedProgramId.value || legacySyncedSeedCount.value <= 0) return
  if (!(await dialog.confirm(t('bugBounty.monitor.seeds.confirmRetireLegacySynced', { count: legacySyncedSeedCount.value })))) {
    return
  }

  try {
    retiringLegacySynced.value = true
    const result = await invoke<SurfaceLegacySeedRetireResult>('surface_retire_legacy_synced_seeds', {
      programId: resolvedProgramId.value,
    })
    toast.success(t('bugBounty.monitor.seeds.retireLegacySyncedSuccess', { count: result.retired }))
    await Promise.all([loadSeeds(), loadDistribution(), loadLegacySyncedSeedCount()])
  } catch (error) {
    console.error('Failed to retire legacy synced seeds:', error)
    toast.error(t('bugBounty.monitor.seeds.retireLegacySyncedFailed'))
  } finally {
    retiringLegacySynced.value = false
  }
}

const syncSeedsFromAssets = async () => {
  if (!resolvedProgramId.value) return
  try {
    syncing.value = true
    const result = await invoke<SeedSyncResult>('surface_sync_program_seeds', {
      programId: resolvedProgramId.value,
    })
    toast.success(
      t('bugBounty.monitor.seeds.syncedSummary', {
        created: result.created,
        refreshed: result.refreshed,
        pendingReview: result.pending_review,
      })
    )
    await Promise.all([loadPendingCandidates(), loadSeeds(), loadDistribution(), loadLegacySyncedSeedCount()])
  } catch (error) {
    console.error('Failed to sync surface seeds:', error)
    toast.error(t('bugBounty.monitor.seeds.syncFailed'))
  } finally {
    syncing.value = false
  }
}

const seedTypeLabel = (value: string) => {
  const match = seedTypeOptions.value.find(option => option.value === value)
  return match?.label || value
}

const sourceLabel = (value?: string | null) => {
  if (!value) return '-'
  if (value === 'surface_asset_sync') {
    return t('bugBounty.monitor.seeds.sourceSynced')
  }
  if (value === 'legacy_surface_asset_sync') {
    return t('bugBounty.monitor.seeds.sourceLegacySynced')
  }
  if (value === 'manual') {
    return t('bugBounty.monitor.seeds.sourceManual')
  }
  if (value === 'candidate_review') {
    return t('bugBounty.monitor.seeds.sourceCandidateReview')
  }
  return value
}

const goToFirstPage = () => {
  currentPage.value = 1
}

const goToPreviousPage = () => {
  currentPage.value = Math.max(1, currentPage.value - 1)
}

const goToNextPage = () => {
  currentPage.value = Math.min(pageCount.value, currentPage.value + 1)
}

const goToLastPage = () => {
  currentPage.value = pageCount.value
}

const applyPageJump = () => {
  const parsed = Number.parseInt(pageInput.value, 10)
  if (!Number.isFinite(parsed)) return
  currentPage.value = Math.min(pageCount.value, Math.max(1, parsed))
  pageInput.value = ''
}

const formatConfidence = (value?: number | null) => {
  if (typeof value !== 'number' || Number.isNaN(value)) return '-'
  return value.toFixed(2)
}

watch(
  () => props.selectedProgram?.id,
  () => {
    ensureResolvedProgramSelection()
  },
  { immediate: true },
)

watch(
  () => props.programs,
  () => {
    ensureResolvedProgramSelection()
  },
  { immediate: true, deep: true },
)

watch(resolvedProgramId, () => {
  selectedSeedIds.value = []
  selectedCandidateIds.value = []
  expandedCandidateGroupKeys.value = []
  loadSeeds()
  loadDistribution()
  loadPendingCandidates()
  loadLegacySyncedSeedCount()
}, { immediate: true })

watch(candidateStatusFilter, () => {
  selectedCandidateIds.value = []
  expandedCandidateGroupKeys.value = []
  loadPendingCandidates()
})

watch([searchQuery, typeFilter, statusFilter, sourceFilter], () => {
  currentPage.value = 1
  pageInput.value = ''
  loadSeeds()
  loadDistribution()
})

watch(pageSize, () => {
  currentPage.value = 1
  pageInput.value = ''
  loadSeeds()
})

watch(pageCount, value => {
  if (currentPage.value > value) {
    currentPage.value = value
  }
})

watch(currentPage, () => {
  loadSeeds()
})

watch(
  () => props.open,
  open => {
    if (open) {
      ensureResolvedProgramSelection()
      if (resolvedProgramId.value) {
        loadSeeds()
        loadDistribution()
        loadPendingCandidates()
        loadLegacySyncedSeedCount()
      }
      return
    }
    if (!open) {
      showCandidatesModal.value = false
      showEditorModal.value = false
      editingSeed.value = null
      seedPendingDelete.value = null
      selectedSeedIds.value = []
      selectedCandidateIds.value = []
      expandedCandidateGroupKeys.value = []
      ensureResolvedProgramSelection()
      resetFilters()
      resetSeedForm()
    }
  }
)
</script>
