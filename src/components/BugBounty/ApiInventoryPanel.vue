<template>
  <div class="space-y-4">
    <div class="card bg-base-100 shadow-md">
      <div class="card-body p-4 pt-3">
        <div class="flex flex-wrap items-start justify-between gap-3">
          <div>
            <h2 class="card-title">{{ t('bugBounty.apiInventory.title') }}</h2>
            <p class="text-sm text-base-content/60">
              {{ currentProgramLabel }}
            </p>
          </div>
          <div class="flex items-center gap-2">
            <button
              class="btn btn-sm btn-primary"
              :disabled="!programFilter || requestingProject || requestingBatch || loading"
              :title="programFilter ? t('bugBounty.apiInventory.requestProjectEndpointsHint') : t('bugBounty.apiInventory.selectProgramFirst')"
              @click="requestProjectAllEndpoints"
            >
              <span v-if="requestingProject" class="loading loading-spinner loading-xs"></span>
              <i v-else class="fas fa-layer-group mr-2"></i>
              <span v-if="requestingProject">
                {{ t('bugBounty.apiInventory.requestProjectEndpointsRunning', {
                  done: projectRequestProgress.processedTargets,
                  total: projectRequestProgress.totalTargets,
                }) }}
              </span>
              <span v-else>{{ t('bugBounty.apiInventory.requestProjectEndpoints') }}</span>
            </button>
            <button class="btn btn-sm btn-outline" :disabled="loading" @click="loadTargets(true)">
              <i class="fas fa-sync-alt mr-2"></i>
              {{ t('common.refresh') }}
            </button>
            <button class="btn btn-sm btn-ghost" :disabled="loading" @click="clearFilters">
              <i class="fas fa-filter-circle-xmark mr-2"></i>
              {{ t('bugBounty.apiInventory.clearFilters') }}
            </button>
          </div>
        </div>

        <div class="mt-4 grid grid-cols-1 gap-3 md:grid-cols-2 xl:grid-cols-6">
          <label class="input input-bordered input-sm flex items-center gap-2">
            <i class="fas fa-search text-base-content/50"></i>
            <input
              v-model.trim="search"
              type="text"
              class="grow"
              :placeholder="t('bugBounty.apiInventory.searchCompactPlaceholder')"
            />
          </label>

          <select
            v-model="programFilter"
            class="select select-bordered select-sm"
            @change="handleProgramFilterChange"
          >
            <option value="">{{ t('bugBounty.apiInventory.allPrograms') }}</option>
            <option v-for="program in programOptions" :key="program.id" :value="String(program.id)">
              {{ program.name || program.id }}
            </option>
          </select>

          <div class="join h-8" role="group" :aria-label="t('bugBounty.apiInventory.status')">
            <button
              type="button"
              class="btn btn-sm join-item"
              :class="statusFilter === 'all' ? 'btn-primary' : 'btn-outline'"
              @click="statusFilter = 'all'"
            >
              {{ t('bugBounty.apiInventory.allStatuses') }}
            </button>
            <button
              type="button"
              class="btn btn-sm join-item px-3"
              :class="statusFilter === 'success' ? 'btn-outline border-success bg-success/10' : 'btn-outline'"
              :title="t('common.success')"
              :aria-label="t('common.success')"
              @click="statusFilter = 'success'"
            >
              <span class="inline-block h-2.5 w-2.5 rounded-full bg-success"></span>
            </button>
            <button
              type="button"
              class="btn btn-sm join-item px-3"
              :class="statusFilter === 'failed' ? 'btn-outline border-error bg-error/10' : 'btn-outline'"
              :title="t('common.failed')"
              :aria-label="t('common.failed')"
              @click="statusFilter = 'failed'"
            >
              <span class="inline-block h-2.5 w-2.5 rounded-full bg-error"></span>
            </button>
          </div>
          <label class="flex h-8 items-center gap-2 rounded-lg border border-base-300 px-3 text-sm">
            <input v-model="changedOnly" type="checkbox" class="checkbox checkbox-sm" />
            <span>{{ t('bugBounty.apiInventory.changedOnly') }}</span>
          </label>

          <select v-model="sortBy" class="select select-bordered select-sm">
            <option value="observed_desc">{{ t('bugBounty.apiInventory.sortNewest') }}</option>
            <option value="endpoint_desc">{{ t('bugBounty.apiInventory.sortEndpointCount') }}</option>
            <option value="changes_desc">{{ t('bugBounty.apiInventory.sortChanges') }}</option>
            <option value="base_url_asc">{{ t('bugBounty.apiInventory.sortBaseUrl') }}</option>
          </select>

          <button
            type="button"
            class="btn btn-sm btn-outline"
            :class="{ 'btn-primary': advancedFiltersExpanded || hasAdvancedFilters }"
            @click="advancedFiltersExpanded = !advancedFiltersExpanded"
          >
            <i class="fas" :class="advancedFiltersExpanded ? 'fa-chevron-up' : 'fa-chevron-down'"></i>
            {{ advancedFiltersExpanded ? t('bugBounty.apiInventory.hideMoreFilters') : t('bugBounty.apiInventory.moreFilters') }}
            <span v-if="activeAdvancedFilterCount > 0" class="badge badge-primary badge-sm">
              {{ activeAdvancedFilterCount }}
            </span>
          </button>
        </div>

        <div
          v-if="advancedFiltersExpanded"
          class="mt-3 grid grid-cols-1 gap-3 rounded-lg border border-base-200 bg-base-200/40 p-3 md:grid-cols-2"
        >
          <select v-model="executionModeFilter" class="select select-bordered select-sm">
            <option value="all">{{ t('bugBounty.apiInventory.allExecutionModes') }}</option>
            <option value="scheduler">{{ t('bugBounty.monitor.scheduledRun') }}</option>
            <option value="manual">{{ t('bugBounty.monitor.manualRun') }}</option>
          </select>
        </div>

        <div class="mt-4 grid grid-cols-2 gap-4 xl:grid-cols-5">
          <div class="stat bg-base-200 rounded-lg p-3">
            <div class="stat-title text-xs">{{ t('bugBounty.apiInventory.targets') }}</div>
            <div class="stat-value text-lg">{{ filteredTargetCount }}</div>
          </div>
          <div class="stat bg-base-200 rounded-lg p-3">
            <div class="stat-title text-xs">{{ t('bugBounty.apiInventory.endpoints') }}</div>
            <div class="stat-value text-lg">{{ totalEndpoints }}</div>
          </div>
          <div class="stat bg-base-200 rounded-lg p-3" :title="t('common.success')">
            <div class="stat-title text-xs">
              <span class="inline-block h-2.5 w-2.5 rounded-full bg-success"></span>
            </div>
            <div class="stat-value text-lg">{{ successfulTargets }}</div>
          </div>
          <div class="stat bg-base-200 rounded-lg p-3" :title="t('common.failed')">
            <div class="stat-title text-xs">
              <span class="inline-block h-2.5 w-2.5 rounded-full bg-error"></span>
            </div>
            <div class="stat-value text-lg">{{ failedTargets }}</div>
          </div>
          <div class="stat bg-base-200 rounded-lg p-3">
            <div class="stat-title text-xs">{{ t('bugBounty.apiInventory.changes') }}</div>
            <div class="stat-value text-lg text-warning">{{ changedTargets }}</div>
          </div>
        </div>
                  <span class="text-xs font-normal text-base-content/60">
                    {{ t('bugBounty.apiInventory.filterSummary', {
                      total: totalTargetCount,
                      filtered: filteredTargetCount,
                    }) }}
          </span>
      </div>
    </div>

    <div class="grid grid-cols-1 gap-4 xl:grid-cols-[minmax(360px,440px)_minmax(0,1fr)]">
      <div class="card bg-base-100 shadow-md">
        <div class="card-body p-0">
          <div class="border-b border-base-200 px-4 py-3">
            <div class="flex flex-col gap-3">
              <div class="flex items-center justify-between gap-3">
                <div class="flex items-center gap-2 text-sm font-medium">
                  <span>{{ t('bugBounty.apiInventory.targets') }}</span>

                  <span v-if="selectedTargetKeys.length > 0" class="badge badge-primary badge-sm">
                    {{ selectedTargetKeys.length }} {{ t('bugBounty.batch.selected') }}
                  </span>
                </div>
                <button
                  v-if="selectedTargetKeys.length > 0"
                  class="btn btn-xs btn-ghost"
                  :disabled="loading || deleting"
                  @click="clearSelection"
                >
                  <i class="fas fa-times mr-1"></i>
                  {{ t('bugBounty.batch.clearSelection') }}
                </button>
              </div>

              <div
                v-if="showBatchActions"
                class="flex flex-col gap-2 rounded-lg border border-base-300 bg-base-200/40 p-3"
              >
                <div class="text-xs text-base-content/60">
                  {{ t('bugBounty.batch.selectionSummary', {
                    selected: selectedTargetKeys.length,
                    page: targets.length,
                    total: filteredTargetCount,
                  }) }}
                </div>
                <div class="flex flex-wrap items-center gap-2">
                  <button
                    class="btn btn-xs btn-outline"
                    :disabled="loading || selectingAll || deleting || filteredTargetCount === 0 || allFilteredSelected"
                    @click="selectAllFilteredTargets"
                  >
                    <span v-if="selectingAll" class="loading loading-spinner loading-xs"></span>
                    <i class="fas fa-layer-group mr-2"></i>
                    {{ t('bugBounty.batch.selectAllFiltered') }}
                  </button>
                  <button
                    class="btn btn-xs btn-error btn-outline"
                    :disabled="selectedTargetKeys.length === 0 || loading || deleting"
                    @click="batchDeleteTargets"
                  >
                    <span v-if="deleting" class="loading loading-spinner loading-xs"></span>
                    <i v-else class="fas fa-trash mr-2"></i>
                    {{ t('bugBounty.apiInventory.deleteSelected') }}
                  </button>
                  <button
                    class="btn btn-xs btn-error"
                    :disabled="loading || selectingAll || deleting || filteredTargetCount === 0"
                    @click="deleteAllFilteredTargets"
                  >
                    <span v-if="deleting" class="loading loading-spinner loading-xs"></span>
                    <i v-else class="fas fa-trash-alt mr-2"></i>
                    {{ t('bugBounty.apiInventory.deleteAllFiltered') }}
                  </button>
                </div>
              </div>
            </div>
          </div>

          <div v-if="loading" class="flex justify-center py-10">
            <span class="loading loading-spinner loading-lg"></span>
          </div>

          <div v-else-if="targets.length === 0" class="px-4 py-10 text-center text-base-content/60">
            <i class="fas fa-plug text-4xl opacity-30"></i>
            <p class="mt-3">{{ t('bugBounty.apiInventory.empty') }}</p>
          </div>

          <div v-else class="max-h-[72vh] overflow-auto" @scroll="handleTargetListScroll">
            <table class="table table-sm">
              <thead class="sticky top-0 z-10 bg-base-100">
                <tr>
                  <th class="w-10">
                    <input
                      type="checkbox"
                      class="checkbox checkbox-sm"
                      :checked="allCurrentPageSelected"
                      :indeterminate="hasCurrentPagePartialSelection"
                      :disabled="loading || deleting || targets.length === 0"
                      @change="toggleSelectCurrentPage"
                    />
                  </th>
                  <th>{{ t('bugBounty.apiInventory.target') }}</th>
                  <th>{{ t('bugBounty.apiInventory.status') }}</th>
                  <th>{{ t('bugBounty.apiInventory.observedAt') }}</th>
                  <th class="w-12">{{ t('bugBounty.apiInventory.actions') }}</th>
                </tr>
              </thead>
              <tbody>
                <tr
                  v-for="target in targets"
                  :key="targetKeyOf(target)"
                  class="cursor-pointer"
                  :class="selectedTargetKey === targetKeyOf(target) ? 'bg-primary/10' : ''"
                  @click="selectTarget(target)"
                >
                  <td @click.stop>
                    <input
                      type="checkbox"
                      class="checkbox checkbox-sm"
                      :checked="isTargetSelected(target)"
                      :disabled="loading || deleting"
                      @change="toggleTargetSelection(target)"
                    />
                  </td>
                  <td class="min-w-[14rem] max-w-[18rem]">
                    <div class="truncate font-medium">{{ target.base_url }}</div>
                    <div class="mt-1 flex flex-wrap gap-1 text-xs">
                      <span class="badge badge-outline badge-sm">
                        {{ programNameForId(target.program_id) }}
                      </span>
                      <span v-if="target.execution_mode" class="badge badge-ghost badge-sm">
                        {{ target.execution_mode }}
                      </span>
                      <span v-if="target.task_name" class="badge badge-ghost badge-sm">
                        {{ target.task_name }}
                      </span>
                    </div>
                    <div class="mt-1 flex flex-wrap items-center gap-3 text-xs text-base-content/60">
                      <span>{{ t('bugBounty.apiInventory.endpoints') }}: {{ target.endpoint_count }}</span>
                      <span>
                        {{ t('bugBounty.apiInventory.changes') }}:
                        <span class="text-success">+{{ target.added_endpoints_count }}</span>
                        <span class="ml-1 text-error">-{{ target.removed_endpoints_count }}</span>
                      </span>
                    </div>
                  </td>
                  <td>
                    <span
                      class="inline-block h-3 w-3 rounded-full"
                      :class="target.success ? 'bg-success' : 'bg-error'"
                      :title="target.success ? t('common.success') : t('common.failed')"
                    ></span>
                  </td>
                  <td class="min-w-[8rem] text-xs text-base-content/60">
                    {{ formatDateTime(target.observed_at) }}
                  </td>
                  <td @click.stop>
                    <button
                      type="button"
                      class="btn btn-ghost btn-xs text-error"
                      :disabled="loading || deleting"
                      :title="t('common.delete')"
                      @click="deleteSingleTarget(target)"
                    >
                      <i class="fas fa-trash"></i>
                    </button>
                  </td>
                </tr>
              </tbody>
            </table>

            <div v-if="loadingMore" class="flex justify-center py-4">
              <span class="loading loading-spinner loading-md"></span>
            </div>

            <div v-else-if="hasMoreTargets" class="px-4 py-3 text-center text-xs text-base-content/50">
              {{ t('common.loadMore') }}
            </div>
          </div>
        </div>
      </div>

      <div class="card bg-base-100 shadow-md">
        <div class="card-body">
          <div v-if="loadingDetail" class="flex justify-center py-16">
            <span class="loading loading-spinner loading-lg"></span>
          </div>

          <div v-else-if="!selectedDetail" class="flex flex-col items-center justify-center py-16 text-base-content/60">
            <i class="fas fa-diagram-project text-5xl opacity-30"></i>
            <p class="mt-3">{{ t('bugBounty.apiInventory.selectTarget') }}</p>
          </div>

          <template v-else>
            <div class="flex flex-wrap items-start justify-between gap-3">
              <div class="min-w-0">
                <h3 class="truncate text-lg font-semibold">{{ selectedDetail.base_url }}</h3>
                <div class="mt-2 flex flex-wrap gap-2 text-xs text-base-content/60">
                  <span class="badge badge-outline badge-sm">
                    {{ programNameForId(selectedDetail.program_id) }}
                  </span>
                  <span v-if="selectedDetail.task_name" class="badge badge-ghost badge-sm">
                    {{ selectedDetail.task_name }}
                  </span>
                  <span v-if="selectedDetail.execution_mode" class="badge badge-outline badge-sm">
                    {{ selectedDetail.execution_mode }}
                  </span>
                  <span class="badge badge-outline badge-sm">
                    {{ t('bugBounty.apiInventory.endpoints') }}: {{ selectedDetail.endpoint_count }}
                  </span>
                </div>
              </div>
              <div class="text-right text-xs text-base-content/60">
                <div>{{ formatDateTime(selectedDetail.observed_at) }}</div>
                <div v-if="selectedDetail.last_checked">{{ selectedDetail.last_checked }}</div>
              </div>
            </div>

            <div class="mt-4 grid grid-cols-1 gap-4 md:grid-cols-2">
              <div class="rounded-lg bg-base-200 p-3">
                <div class="text-xs text-base-content/60">{{ t('bugBounty.apiInventory.targetStatus') }}</div>
                <div class="mt-1">
                  <span
                    class="inline-block h-3 w-3 rounded-full"
                    :class="selectedDetail.success ? 'bg-success' : 'bg-error'"
                    :title="selectedDetail.success ? t('common.success') : t('common.failed')"
                  ></span>
                </div>
              </div>
              <div class="rounded-lg bg-base-200 p-3">
                <div class="text-xs text-base-content/60">{{ t('bugBounty.apiInventory.changes') }}</div>
                <div class="mt-1 flex items-center gap-3 text-sm font-medium">
                  <span class="text-success">+{{ selectedDetail.added_endpoints.length }}</span>
                  <span class="text-error">-{{ selectedDetail.removed_endpoints.length }}</span>
                </div>
              </div>
            </div>

            <div v-if="selectedDetail.error_message" class="alert alert-warning mt-4">
              <i class="fas fa-triangle-exclamation"></i>
              <span>{{ selectedDetail.error_message }}</span>
            </div>

            <div class="tabs tabs-boxed mt-4">
              <a class="tab" :class="{ 'tab-active': endpointTab === 'all' }" @click="endpointTab = 'all'">
                {{ t('bugBounty.apiInventory.allEndpoints') }}
              </a>
              <a class="tab" :class="{ 'tab-active': endpointTab === 'added' }" @click="endpointTab = 'added'">
                {{ t('bugBounty.apiInventory.addedEndpoints') }}
              </a>
              <a class="tab" :class="{ 'tab-active': endpointTab === 'removed' }" @click="endpointTab = 'removed'">
                {{ t('bugBounty.apiInventory.removedEndpoints') }}
              </a>
            </div>

            <div class="mt-3 flex flex-wrap items-center gap-2">
              <select v-model="endpointRequestMethod" class="select select-bordered select-sm w-24">
                <option value="GET">GET</option>
                <option value="POST">POST</option>
              </select>
              <button
                type="button"
                class="btn btn-sm btn-outline"
                :disabled="!selectedEndpoint || requestingProject || requestingBatch || isEndpointRequesting(selectedEndpoint)"
                @click="requestSingleEndpoint(selectedEndpoint)"
              >
                <span v-if="selectedEndpoint && isEndpointRequesting(selectedEndpoint)" class="loading loading-spinner loading-xs"></span>
                <i v-else class="fas fa-paper-plane mr-2"></i>
                {{ t('bugBounty.apiInventory.requestSelectedEndpoint') }}
              </button>
              <button
                type="button"
                class="btn btn-sm btn-primary"
                :disabled="visibleEndpoints.length === 0 || requestingProject || requestingBatch"
                @click="requestVisibleEndpoints"
              >
                <span v-if="requestingBatch" class="loading loading-spinner loading-xs"></span>
                <i v-else class="fas fa-layer-group mr-2"></i>
                {{ t('bugBounty.apiInventory.requestVisibleEndpoints', { count: visibleEndpoints.length }) }}
              </button>
              <label
                class="flex h-8 items-center gap-2 rounded-lg border border-base-300 px-3 text-sm"
                :class="hasEndpointRequestResultsForCurrentMethod ? '' : 'opacity-60'"
                :title="t('bugBounty.apiInventory.jsonOnlyHint')"
              >
                <input
                  v-model="jsonOnly"
                  type="checkbox"
                  class="checkbox checkbox-sm"
                  :disabled="!hasEndpointRequestResultsForCurrentMethod"
                />
                <span>{{ t('bugBounty.apiInventory.jsonOnly') }}</span>
              </label>
            </div>

            <textarea
              v-if="endpointRequestMethod === 'POST'"
              v-model="endpointRequestBody"
              class="textarea textarea-bordered mt-3 min-h-20 w-full font-mono text-xs"
              :placeholder="t('bugBounty.apiInventory.postBodyPlaceholder')"
            ></textarea>

            <div class="mt-4 overflow-hidden rounded-lg border border-base-200">
              <div class="max-h-[42vh] overflow-auto">
                <table class="table table-zebra">
                  <thead>
                    <tr>
                      <th>{{ t('bugBounty.apiInventory.path') }}</th>
                      <th>{{ t('bugBounty.apiInventory.source') }}</th>
                      <th>{{ t('bugBounty.apiInventory.requestResult') }}</th>
                      <th class="w-12">{{ t('bugBounty.apiInventory.actions') }}</th>
                    </tr>
                  </thead>
                  <tbody>
                    <tr
	                      v-for="endpoint in visibleEndpoints"
	                      :key="endpointKeyOf(endpoint)"
	                      class="cursor-pointer"
	                      :class="selectedEndpointKey === endpointKeyOf(endpoint) ? 'bg-primary/10' : ''"
	                      @click="openEndpointResultDialog(endpoint)"
	                    >
                      <td class="font-mono text-xs">{{ endpoint.path }}</td>
                      <td class="text-xs text-base-content/60">{{ endpoint.source || '-' }}</td>
	                      <td class="text-xs">
	                        <span v-if="isEndpointRequesting(endpoint)" class="loading loading-spinner loading-xs"></span>
	                        <span v-else-if="endpointRequestResultOf(endpoint)" class="flex flex-wrap items-center gap-1">
	                          <span
	                            class="badge badge-sm"
	                            :class="endpointRequestResultClass(endpointRequestResultOf(endpoint))"
	                            :title="endpointRequestResultTitle(endpointRequestResultOf(endpoint))"
	                          >
	                            {{ endpointRequestResultLabel(endpointRequestResultOf(endpoint)) }}
	                          </span>
	                          <span
	                            v-if="isJsonRequestResult(endpointRequestResultOf(endpoint))"
	                            class="badge badge-info badge-outline badge-sm"
	                          >
	                            JSON
	                          </span>
	                        </span>
	                        <span v-else class="text-base-content/40">-</span>
	                      </td>
                      <td @click.stop>
                        <button
	                          type="button"
	                          class="btn btn-ghost btn-xs"
	                          :disabled="requestingProject || requestingBatch || isEndpointRequesting(endpoint)"
                          :title="t('bugBounty.apiInventory.requestEndpoint')"
                          @click="requestSingleEndpoint(endpoint)"
                        >
                          <span v-if="isEndpointRequesting(endpoint)" class="loading loading-spinner loading-xs"></span>
                          <i v-else class="fas fa-paper-plane"></i>
                        </button>
                      </td>
                    </tr>
                  </tbody>
                </table>
              </div>

              <div v-if="visibleEndpoints.length === 0" class="px-4 py-10 text-center text-sm text-base-content/60">
                {{ t('bugBounty.apiInventory.noEndpointsInTab') }}
              </div>
            </div>

            <div v-if="selectedEndpoint" class="mt-4 rounded-lg border border-base-200 bg-base-200/30 p-4">
              <div class="flex flex-wrap items-start justify-between gap-3">
                <div class="min-w-0">
                  <div class="font-medium">{{ selectedEndpoint.path }}</div>
                </div>
              </div>

              <div class="mt-4 grid grid-cols-1 gap-4 lg:grid-cols-2">
                <div class="rounded-lg bg-base-200 p-3">
                  <div class="text-xs text-base-content/60">{{ t('bugBounty.apiInventory.requestUrl') }}</div>
                  <div class="mt-1 break-all font-mono text-xs">
                    {{ resolveEndpointUrl(selectedDetail.base_url, selectedEndpoint.path) }}
                  </div>
                </div>
                <div class="rounded-lg bg-base-200 p-3">
                  <div class="text-xs text-base-content/60">{{ t('bugBounty.apiInventory.source') }}</div>
                  <div class="mt-1 text-sm font-medium">
                    {{ selectedEndpoint.source || '-' }}
                  </div>
                </div>
              </div>

            </div>
          </template>
        </div>
      </div>
    </div>

    <div
      v-if="endpointResultDialogEndpoint && endpointResultDialogResult"
      ref="endpointResultDialogOverlay"
      class="fixed inset-0 z-50 flex items-center justify-center bg-black/40 p-4"
      tabindex="0"
      @click.self="closeEndpointResultDialog"
      @keydown.esc="closeEndpointResultDialog"
    >
      <div class="max-h-[86vh] w-full max-w-4xl overflow-hidden rounded-lg bg-base-100 shadow-xl">
        <div class="flex items-start justify-between gap-4 border-b border-base-200 p-4">
          <div class="min-w-0">
            <div class="text-sm font-semibold">{{ t('bugBounty.apiInventory.latestRequestResult') }}</div>
            <div class="mt-1 break-all font-mono text-xs text-base-content/70">
              {{ endpointResultDialogResult.method }} {{ endpointResultDialogResult.url }}
            </div>
          </div>
          <button class="btn btn-ghost btn-sm" type="button" @click="closeEndpointResultDialog">
            <i class="fas fa-times"></i>
          </button>
        </div>

        <div class="max-h-[calc(86vh-4rem)] overflow-auto p-4">
          <div class="grid grid-cols-1 gap-3 text-sm md:grid-cols-4">
            <div class="rounded-lg bg-base-200 p-3">
              <div class="text-xs text-base-content/60">{{ t('bugBounty.apiInventory.statusCode') }}</div>
              <span class="badge badge-sm mt-1" :class="endpointRequestResultClass(endpointResultDialogResult)">
                {{ endpointRequestResultLabel(endpointResultDialogResult) }}
              </span>
            </div>
            <div class="rounded-lg bg-base-200 p-3">
              <div class="text-xs text-base-content/60">{{ t('bugBounty.apiInventory.duration') }}</div>
              <div class="mt-1 font-medium">{{ endpointResultDialogResult.durationMs }}ms</div>
            </div>
            <div class="rounded-lg bg-base-200 p-3">
              <div class="text-xs text-base-content/60">{{ t('bugBounty.apiInventory.responseSize') }}</div>
              <div class="mt-1 font-medium">{{ endpointResultDialogResult.responseBytes }}B</div>
            </div>
            <div class="rounded-lg bg-base-200 p-3">
              <div class="text-xs text-base-content/60">{{ t('bugBounty.apiInventory.requestTime') }}</div>
              <div class="mt-1 text-xs">{{ formatDateTime(endpointResultDialogResult.createdAt) }}</div>
            </div>
          </div>

          <div class="mt-3 rounded-lg bg-base-200 p-3">
            <div class="text-xs text-base-content/60">Content-Type</div>
            <div class="mt-1 break-all font-mono text-xs">
              {{ endpointResultDialogResult.responseContentType || '-' }}
            </div>
          </div>

          <div v-if="endpointResultDialogResult.error" class="mt-3 rounded-lg bg-error/10 p-3 text-sm text-error">
            {{ endpointResultDialogResult.error }}
          </div>
          <pre
            v-else-if="endpointResultDialogResult.bodyPreview"
            class="mt-3 max-h-[44vh] overflow-auto whitespace-pre-wrap rounded-lg bg-base-300 p-3 text-xs"
          >{{ endpointResultDialogResult.bodyPreview }}</pre>
          <div v-else class="mt-3 rounded-lg bg-base-200 p-6 text-center text-sm text-base-content/60">
            {{ t('bugBounty.apiInventory.responseEmpty') }}
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, onMounted, ref, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useI18n } from 'vue-i18n'
import { useToast } from '../../composables/useToast'
import { dialog } from '../../composables/useDialog'

interface ApiInventoryEndpoint {
  path: string
  source?: string | null
}

type RawApiInventoryEndpoint = ApiInventoryEndpoint

interface ApiInventoryTargetSummary {
  program_id: string
  base_url: string
  success: boolean
  endpoint_count: number
  last_checked?: string | null
  observed_at: string
  run_id: string
  task_name?: string | null
  execution_mode?: string | null
  added_endpoints_count: number
  removed_endpoints_count: number
  sample_endpoints: string[]
  error_message?: string | null
}

interface ApiInventoryTargetDetail {
  program_id: string
  base_url: string
  success: boolean
  endpoint_count: number
  last_checked?: string | null
  observed_at: string
  run_id: string
  task_name?: string | null
  execution_mode?: string | null
  endpoints: ApiInventoryEndpoint[]
  added_endpoints: ApiInventoryEndpoint[]
  removed_endpoints: ApiInventoryEndpoint[]
  error_message?: string | null
}

interface RawApiInventoryTargetDetail extends Omit<ApiInventoryTargetDetail, 'endpoints' | 'added_endpoints' | 'removed_endpoints'> {
  endpoints: RawApiInventoryEndpoint[]
  added_endpoints: RawApiInventoryEndpoint[]
  removed_endpoints: RawApiInventoryEndpoint[]
}

interface ApiInventoryDeleteTarget {
  program_id: string
  base_url: string
}

interface RawApiInventoryDeleteTarget {
  program_id?: string
  base_url?: string
  programId?: string
  baseUrl?: string
}

interface ApiInventoryListStats {
  totalEndpoints: number
  successfulTargets: number
  failedTargets: number
  changedTargets: number
}

interface ApiInventoryListResponse {
  items: ApiInventoryTargetSummary[]
  total: number
  filteredTotal: number
  hasMore: boolean
  stats: ApiInventoryListStats
}

interface ApiInventoryEndpointRequestResult {
  path: string
  source?: string | null
  method: EndpointRequestMethod
  url: string
  success: boolean
  status?: number | null
  statusText?: string | null
  durationMs: number
  responseBytes: number
  responseContentType?: string | null
  bodyPreview?: string | null
  error?: string | null
  createdAt?: string | null
}

type StatusFilter = 'all' | 'success' | 'failed'
type ExecutionModeFilter = 'all' | 'scheduler' | 'manual'
type CapabilityFilter = 'all' | 'changed'
type EndpointTab = 'all' | 'added' | 'removed'
type EndpointRequestMethod = 'GET' | 'POST'
type SortBy = 'observed_desc' | 'endpoint_desc' | 'changes_desc' | 'base_url_asc'

const TARGET_PAGE_SIZE = 50

const props = defineProps<{
  selectedProgram?: any
  programs?: any[]
}>()

const { t } = useI18n()
const toast = useToast()

const loading = ref(false)
const loadingMore = ref(false)
const loadingDetail = ref(false)
const search = ref('')
const statusFilter = ref<StatusFilter>('all')
const executionModeFilter = ref<ExecutionModeFilter>('all')
const capabilityFilter = ref<CapabilityFilter>('all')
const sortBy = ref<SortBy>('observed_desc')
const programFilter = ref('')
const programFilterTouched = ref(false)
const advancedFiltersExpanded = ref(false)
const deleting = ref(false)
const selectingAll = ref(false)
const targets = ref<ApiInventoryTargetSummary[]>([])
const totalTargetCount = ref(0)
const filteredTargetCount = ref(0)
const hasMoreTargets = ref(false)
const selectedTargetEntries = ref<ApiInventoryDeleteTarget[]>([])
const targetStats = ref<ApiInventoryListStats>({
  totalEndpoints: 0,
  successfulTargets: 0,
  failedTargets: 0,
  changedTargets: 0,
})
const selectedTargetKey = ref('')
const selectedDetail = ref<ApiInventoryTargetDetail | null>(null)
const endpointTab = ref<EndpointTab>('all')
const selectedEndpointKey = ref('')
const endpointRequestMethod = ref<EndpointRequestMethod>('GET')
const endpointRequestBody = ref('')
const jsonOnly = ref(false)
const requestingBatch = ref(false)
const requestingProject = ref(false)
const requestingEndpointKeys = ref<string[]>([])
const endpointRequestResults = ref<Record<string, ApiInventoryEndpointRequestResult>>({})
const endpointResultDialogEndpoint = ref<ApiInventoryEndpoint | null>(null)
const endpointResultDialogOverlay = ref<HTMLElement | null>(null)
const projectRequestProgress = ref({
  totalTargets: 0,
  processedTargets: 0,
  requestedEndpoints: 0,
  failedEndpoints: 0,
})

const propSelectedProgramId = computed(() => String(props.selectedProgram?.id || ''))
const programOptions = computed(() => Array.isArray(props.programs) ? props.programs : [])
const programNameById = computed(() => new Map(
  programOptions.value.map(program => [String(program.id), String(program.name || program.id)]),
))

const targetKeyOf = (target: Pick<ApiInventoryTargetSummary, 'program_id' | 'base_url'>) =>
  `${target.program_id}::${target.base_url}`

const normalizeDeleteTarget = (target: RawApiInventoryDeleteTarget): ApiInventoryDeleteTarget | null => {
  const program_id = String(target.program_id || target.programId || '').trim()
  const base_url = String(target.base_url || target.baseUrl || '').trim()
  if (!program_id || !base_url) return null
  return { program_id, base_url }
}

const endpointKeyOf = (endpoint: ApiInventoryEndpoint) => `${endpoint.path}::${endpoint.source || ''}`

const endpointRequestKeyOf = (
  endpoint: ApiInventoryEndpoint,
  method = endpointRequestMethod.value,
  baseUrl = selectedDetail.value?.base_url || '',
) => `${method}::${resolveEndpointUrl(baseUrl, endpoint.path)}`

const normalizeEndpoint = (endpoint: RawApiInventoryEndpoint): ApiInventoryEndpoint => ({
  path: endpoint.path,
  source: endpoint.source ?? null,
})

const normalizeTargetDetail = (detail: RawApiInventoryTargetDetail | null): ApiInventoryTargetDetail | null => {
  if (!detail) return null
  return {
    ...detail,
    endpoints: Array.isArray(detail.endpoints) ? detail.endpoints.map(normalizeEndpoint) : [],
    added_endpoints: Array.isArray(detail.added_endpoints) ? detail.added_endpoints.map(normalizeEndpoint) : [],
    removed_endpoints: Array.isArray(detail.removed_endpoints) ? detail.removed_endpoints.map(normalizeEndpoint) : [],
  }
}

const programNameForId = (programId: string) => (
  programNameById.value.get(String(programId)) || String(programId || '-')
)

const currentProgramLabel = computed(() => (
  programFilter.value
    ? programNameForId(programFilter.value)
    : t('bugBounty.apiInventory.allPrograms')
))

const hasAdvancedFilters = computed(() => (
  executionModeFilter.value !== 'all'
))

const activeAdvancedFilterCount = computed(() => (
  Number(executionModeFilter.value !== 'all')
))

const changedOnly = computed({
  get: () => capabilityFilter.value === 'changed',
  set: (value: boolean) => {
    capabilityFilter.value = value ? 'changed' : 'all'
  },
})

const selectedTargetKeys = computed(() => (
  selectedTargetEntries.value.map(target => targetKeyOf(target))
))

const selectedTargetSet = computed(() => new Set(selectedTargetKeys.value))

const currentPageTargetEntries = computed<ApiInventoryDeleteTarget[]>(() => (
  targets.value.map(target => ({
    program_id: target.program_id,
    base_url: target.base_url,
  }))
))

const allCurrentPageSelected = computed(() => (
  currentPageTargetEntries.value.length > 0
  && currentPageTargetEntries.value.every(target => selectedTargetSet.value.has(targetKeyOf(target)))
))

const hasCurrentPagePartialSelection = computed(() => {
  const selectedOnPage = currentPageTargetEntries.value.filter(target => (
    selectedTargetSet.value.has(targetKeyOf(target))
  )).length
  return selectedOnPage > 0 && selectedOnPage < currentPageTargetEntries.value.length
})

const allFilteredSelected = computed(() => (
  filteredTargetCount.value > 0
  && selectedTargetEntries.value.length >= filteredTargetCount.value
))

const totalEndpoints = computed(() =>
  targetStats.value.totalEndpoints,
)

const successfulTargets = computed(() =>
  targetStats.value.successfulTargets,
)

const failedTargets = computed(() =>
  targetStats.value.failedTargets,
)

const changedTargets = computed(() =>
  targetStats.value.changedTargets,
)

const visibleEndpoints = computed(() => {
  if (!selectedDetail.value) return []
  let endpoints: ApiInventoryEndpoint[]
  switch (endpointTab.value) {
    case 'added':
      endpoints = selectedDetail.value.added_endpoints
      break
    case 'removed':
      endpoints = selectedDetail.value.removed_endpoints
      break
    default:
      endpoints = selectedDetail.value.endpoints
  }

  if (!jsonOnly.value || !hasEndpointRequestResultsForCurrentMethod.value) return endpoints
  return endpoints.filter(endpoint => isJsonRequestResult(endpointRequestResultOf(endpoint)))
})

const selectedEndpoint = computed(() => {
  if (visibleEndpoints.value.length === 0) return null
  return (
    visibleEndpoints.value.find(endpoint => endpointKeyOf(endpoint) === selectedEndpointKey.value)
    || visibleEndpoints.value[0]
  )
})

const endpointResultDialogResult = computed(() => (
  endpointResultDialogEndpoint.value
    ? endpointRequestResultOf(endpointResultDialogEndpoint.value)
    : null
))

const hasEndpointRequestResultsForCurrentMethod = computed(() => {
  const detail = selectedDetail.value
  if (!detail) return false
  return detail.endpoints.some(endpoint => Boolean(endpointRequestResultOf(endpoint)))
})

const buildFilterPayload = (offset = 0, limit = TARGET_PAGE_SIZE) => ({
  programId: programFilter.value || null,
  search: search.value.trim() || null,
  status: statusFilter.value,
  executionMode: executionModeFilter.value,
  capability: capabilityFilter.value,
  sortBy: sortBy.value,
  offset,
  limit,
})

const buildKeyFilterPayload = () => ({
  programId: programFilter.value || null,
  search: search.value.trim() || null,
  status: statusFilter.value,
  executionMode: executionModeFilter.value,
  capability: capabilityFilter.value,
  sortBy: sortBy.value,
})

const buildProjectRequestTargetFilterPayload = () => ({
  programId: programFilter.value || null,
  search: null,
  status: 'all',
  executionMode: 'all',
  capability: 'all',
  sortBy: 'base_url_asc',
})

const showBatchActions = computed(() => (
  selectedTargetKeys.value.length > 0
))

const syncSelectedTargetWithFilters = async () => {
  const selectedVisible = targets.value.find(target => targetKeyOf(target) === selectedTargetKey.value) || null
  if (selectedVisible) return

  const nextTarget = targets.value[0] || null
  selectedTargetKey.value = nextTarget ? targetKeyOf(nextTarget) : ''
  if (nextTarget) {
    await loadTargetDetail(nextTarget)
  } else {
    selectedDetail.value = null
    selectedEndpointKey.value = ''
  }
}

const loadTargets = async (reset = true) => {
  const offset = reset ? 0 : targets.value.length
  try {
    if (reset) {
      loading.value = true
    } else {
      loadingMore.value = true
    }

    const response = await invoke<ApiInventoryListResponse>('bounty_list_api_inventory_targets', {
      filter: buildFilterPayload(offset),
    })
    targets.value = reset
      ? (Array.isArray(response?.items) ? response.items : [])
      : [...targets.value, ...(Array.isArray(response?.items) ? response.items : [])]
    totalTargetCount.value = Number(response?.total || 0)
    filteredTargetCount.value = Number(response?.filteredTotal || 0)
    hasMoreTargets.value = Boolean(response?.hasMore)
    targetStats.value = response?.stats || {
      totalEndpoints: 0,
      successfulTargets: 0,
      failedTargets: 0,
      changedTargets: 0,
    }
    await syncSelectedTargetWithFilters()
  } catch (error) {
    console.error('Failed to load API inventory targets:', error)
    toast.error(t('bugBounty.errors.loadFailed'))
    if (reset) {
      targets.value = []
      totalTargetCount.value = 0
      filteredTargetCount.value = 0
      hasMoreTargets.value = false
      targetStats.value = {
        totalEndpoints: 0,
        successfulTargets: 0,
        failedTargets: 0,
        changedTargets: 0,
      }
      selectedDetail.value = null
      selectedEndpointKey.value = ''
    }
  } finally {
    if (reset) {
      loading.value = false
    } else {
      loadingMore.value = false
    }
  }
}

const loadMoreTargets = async () => {
  if (loading.value || loadingMore.value || !hasMoreTargets.value) return
  await loadTargets(false)
}

const loadTargetDetail = async (target: ApiInventoryTargetSummary | null) => {
  if (!target?.base_url) {
    selectedDetail.value = null
    selectedEndpointKey.value = ''
    return
  }

  try {
    loadingDetail.value = true
    const detail = await invoke<RawApiInventoryTargetDetail | null>('bounty_get_api_inventory_target', {
      programId: target.program_id,
      baseUrl: target.base_url,
    })
    selectedDetail.value = normalizeTargetDetail(detail)
    endpointTab.value = 'all'
    await loadEndpointRequestHistory(selectedDetail.value)
  } catch (error) {
    console.error('Failed to load API inventory detail:', error)
    toast.error(t('bugBounty.errors.loadFailed'))
    selectedDetail.value = null
    selectedEndpointKey.value = ''
  } finally {
    loadingDetail.value = false
  }
}

const loadTargetDetailByKey = async (target: ApiInventoryDeleteTarget) => {
  const detail = await invoke<RawApiInventoryTargetDetail | null>('bounty_get_api_inventory_target', {
    programId: target.program_id,
    baseUrl: target.base_url,
  })
  return normalizeTargetDetail(detail)
}

const loadEndpointRequestHistory = async (
  detail: ApiInventoryTargetDetail | null,
  method = endpointRequestMethod.value,
) => {
  if (!detail) return
  const paths = detail.endpoints.map(endpoint => endpoint.path)
  if (paths.length === 0) return

  try {
    const results = await invoke<ApiInventoryEndpointRequestResult[]>('bounty_list_api_inventory_endpoint_request_history', {
      programId: detail.program_id,
      baseUrl: detail.base_url,
      method,
      paths,
    })
    mergeEndpointRequestResults(results, detail.base_url)
  } catch (error) {
    console.error('Failed to load API inventory endpoint request history:', error)
  }
}

const selectTarget = async (target: ApiInventoryTargetSummary) => {
  selectedTargetKey.value = targetKeyOf(target)
  await loadTargetDetail(target)
}

const selectEndpoint = (endpoint: ApiInventoryEndpoint) => {
  selectedEndpointKey.value = endpointKeyOf(endpoint)
}

const openEndpointResultDialog = async (endpoint: ApiInventoryEndpoint) => {
  selectEndpoint(endpoint)
  endpointResultDialogEndpoint.value = endpointRequestResultOf(endpoint) ? endpoint : null
  if (endpointResultDialogEndpoint.value) {
    await nextTick()
    endpointResultDialogOverlay.value?.focus()
  }
}

const closeEndpointResultDialog = () => {
  endpointResultDialogEndpoint.value = null
}

const markEndpointsRequesting = (
  endpoints: ApiInventoryEndpoint[],
  method = endpointRequestMethod.value,
  baseUrl = selectedDetail.value?.base_url || '',
) => {
  const next = new Set(requestingEndpointKeys.value)
  for (const endpoint of endpoints) {
    next.add(endpointRequestKeyOf(endpoint, method, baseUrl))
  }
  requestingEndpointKeys.value = [...next]
}

const unmarkEndpointsRequesting = (
  endpoints: ApiInventoryEndpoint[],
  method = endpointRequestMethod.value,
  baseUrl = selectedDetail.value?.base_url || '',
) => {
  const next = new Set(requestingEndpointKeys.value)
  for (const endpoint of endpoints) {
    next.delete(endpointRequestKeyOf(endpoint, method, baseUrl))
  }
  requestingEndpointKeys.value = [...next]
}

const isEndpointRequesting = (endpoint: ApiInventoryEndpoint | null) => (
  Boolean(endpoint && requestingEndpointKeys.value.includes(endpointRequestKeyOf(endpoint)))
)

const endpointRequestResultOf = (endpoint: ApiInventoryEndpoint | null) => {
  if (!endpoint) return null
  return endpointRequestResults.value[endpointRequestKeyOf(endpoint)] || null
}

const endpointRequestResultClass = (result: ApiInventoryEndpointRequestResult | null) => {
  if (!result) return 'badge-ghost'
  if (!result.success || result.error) return 'badge-error'
  if (typeof result.status === 'number' && result.status >= 400) return 'badge-warning'
  return 'badge-success'
}

const endpointRequestResultLabel = (result: ApiInventoryEndpointRequestResult | null) => {
  if (!result) return '-'
  if (result.status) return String(result.status)
  return result.success ? 'OK' : t('common.failed')
}

const endpointRequestResultTitle = (result: ApiInventoryEndpointRequestResult | null) => {
  if (!result) return ''
  if (result.error) return result.error
  return `${result.method} ${result.url} ${result.durationMs}ms ${result.responseBytes}B ${formatDateTime(result.createdAt)}`
}

const isJsonRequestResult = (result: ApiInventoryEndpointRequestResult | null) => {
  if (!result || result.error) return false
  const contentType = String(result.responseContentType || '').toLowerCase()
  if (contentType.includes('application/json') || contentType.includes('+json')) {
    return true
  }
  const preview = String(result.bodyPreview || '').trim()
  if (!preview || !['{', '['].includes(preview[0])) return false
  try {
    JSON.parse(preview)
    return true
  } catch {
    return false
  }
}

const mergeEndpointRequestResults = (
  results: ApiInventoryEndpointRequestResult[] | null | undefined,
  baseUrl: string,
) => {
  const next = { ...endpointRequestResults.value }
  for (const result of Array.isArray(results) ? results : []) {
    const resultUrl = result.url || resolveEndpointUrl(baseUrl, result.path)
    next[`${result.method}::${resultUrl}`] = result
  }
  endpointRequestResults.value = next
}

const isTargetSelected = (target: ApiInventoryTargetSummary) => (
  selectedTargetSet.value.has(targetKeyOf(target))
)

const toggleTargetSelection = (target: ApiInventoryTargetSummary) => {
  const key = targetKeyOf(target)
  const next = new Map(selectedTargetEntries.value.map(item => [targetKeyOf(item), item]))
  if (next.has(key)) {
    next.delete(key)
  } else {
    next.set(key, {
      program_id: target.program_id,
      base_url: target.base_url,
    })
  }
  selectedTargetEntries.value = [...next.values()]
}

const toggleSelectCurrentPage = () => {
  if (!currentPageTargetEntries.value.length) return

  const next = new Map(selectedTargetEntries.value.map(item => [targetKeyOf(item), item]))
  if (allCurrentPageSelected.value) {
    for (const target of currentPageTargetEntries.value) {
      next.delete(targetKeyOf(target))
    }
  } else {
    for (const target of currentPageTargetEntries.value) {
      next.set(targetKeyOf(target), target)
    }
  }
  selectedTargetEntries.value = [...next.values()]
}

const selectAllFilteredTargets = async () => {
  try {
    selectingAll.value = true
    const rows = await invoke<RawApiInventoryDeleteTarget[]>('bounty_list_api_inventory_target_keys', {
      filter: buildKeyFilterPayload(),
    })
    selectedTargetEntries.value = Array.isArray(rows)
      ? rows
        .map(normalizeDeleteTarget)
        .filter((target): target is ApiInventoryDeleteTarget => Boolean(target))
      : []
  } catch (error) {
    console.error('Failed to select filtered API inventory targets:', error)
    toast.error(t('bugBounty.errors.loadFailed'))
  } finally {
    selectingAll.value = false
  }
}

const clearSelection = () => {
  selectedTargetEntries.value = []
}

const deleteTargets = async (targetRows: ApiInventoryDeleteTarget[]) => {
  if (!targetRows.length) return 0

  try {
    deleting.value = true
    const deleted = await invoke<number>('bounty_batch_delete_api_inventory_targets', {
      targets: targetRows.map(target => ({
        programId: target.program_id,
        baseUrl: target.base_url,
      })),
    })
    toast.success(t('bugBounty.batch.deleteSuccess', { count: deleted }))
    const deletedKeys = new Set(targetRows.map(target => targetKeyOf(target)))
    selectedTargetEntries.value = selectedTargetEntries.value.filter(target => !deletedKeys.has(targetKeyOf(target)))
    await loadTargets(true)
    return deleted
  } catch (error) {
    console.error('Failed to delete API inventory targets:', error)
    toast.error(t('bugBounty.errors.deleteFailed'))
    return 0
  } finally {
    deleting.value = false
  }
}

const deleteSingleTarget = async (target: ApiInventoryTargetSummary) => {
  if (!(await dialog.confirm(t('bugBounty.apiInventory.confirmDeleteTarget', { target: target.base_url })))) {
    return
  }
  await deleteTargets([{
    program_id: target.program_id,
    base_url: target.base_url,
  }])
}

const batchDeleteTargets = async () => {
  if (!selectedTargetEntries.value.length) return
  if (!(await dialog.confirm(t('bugBounty.batch.confirmDelete', { count: selectedTargetEntries.value.length })))) {
    return
  }
  await deleteTargets(selectedTargetEntries.value)
}

const deleteAllFilteredTargets = async () => {
  if (!filteredTargetCount.value) return
  if (!(await dialog.confirm(t('bugBounty.apiInventory.confirmDeleteAllFiltered', { count: filteredTargetCount.value })))) {
    return
  }

  try {
    selectingAll.value = true
    const rows = await invoke<RawApiInventoryDeleteTarget[]>('bounty_list_api_inventory_target_keys', {
      filter: buildKeyFilterPayload(),
    })
    const targetRows = Array.isArray(rows)
      ? rows
        .map(normalizeDeleteTarget)
        .filter((target): target is ApiInventoryDeleteTarget => Boolean(target))
      : []
    await deleteTargets(targetRows)
  } catch (error) {
    console.error('Failed to delete filtered API inventory targets:', error)
    toast.error(t('bugBounty.errors.deleteFailed'))
  } finally {
    selectingAll.value = false
  }
}

const requestEndpointBatch = async (
  detail: ApiInventoryTargetDetail,
  endpoints: ApiInventoryEndpoint[],
  markVisible = false,
) => {
  const method = endpointRequestMethod.value
  if (markVisible) {
    markEndpointsRequesting(endpoints, method, detail.base_url)
  }

  try {
    const results = await invoke<ApiInventoryEndpointRequestResult[]>('bounty_request_api_inventory_endpoints', {
      request: {
        programId: detail.program_id,
        baseUrl: detail.base_url,
        method,
        endpoints: endpoints.map(endpoint => ({
          path: endpoint.path,
          source: endpoint.source || null,
        })),
        body: method === 'POST' ? endpointRequestBody.value : null,
      },
    })
    mergeEndpointRequestResults(results, detail.base_url)
    return Array.isArray(results) ? results : []
  } finally {
    if (markVisible) {
      unmarkEndpointsRequesting(endpoints, method, detail.base_url)
    }
  }
}

const requestEndpoints = async (endpoints: ApiInventoryEndpoint[]) => {
  const detail = selectedDetail.value
  if (!detail || endpoints.length === 0) return

  try {
    if (endpoints.length > 1) {
      requestingBatch.value = true
    }
    const results = await requestEndpointBatch(detail, endpoints, true)

    const failedCount = results.filter(result => (
      !result.success || Boolean(result.error)
    )).length
    toast.success(t('bugBounty.apiInventory.requestCompleted', {
      total: results.length,
      failed: failedCount,
    }))
  } catch (error) {
    console.error('Failed to request API inventory endpoints:', error)
    toast.error(t('bugBounty.apiInventory.requestFailed'))
  } finally {
    requestingBatch.value = false
  }
}

const requestSingleEndpoint = async (endpoint: ApiInventoryEndpoint | null) => {
  if (!endpoint) return
  await requestEndpoints([endpoint])
}

const requestVisibleEndpoints = async () => {
  await requestEndpoints(visibleEndpoints.value)
}

const requestProjectAllEndpoints = async () => {
  if (!programFilter.value || requestingProject.value) {
    toast.warning(t('bugBounty.apiInventory.selectProgramFirst'))
    return
  }

  try {
    requestingProject.value = true
    projectRequestProgress.value = {
      totalTargets: 0,
      processedTargets: 0,
      requestedEndpoints: 0,
      failedEndpoints: 0,
    }

    const rows = await invoke<RawApiInventoryDeleteTarget[]>('bounty_list_api_inventory_target_keys', {
      filter: buildProjectRequestTargetFilterPayload(),
    })
    const targetRows = Array.isArray(rows)
      ? rows
        .map(normalizeDeleteTarget)
        .filter((target): target is ApiInventoryDeleteTarget => Boolean(target))
      : []

    projectRequestProgress.value.totalTargets = targetRows.length
    if (targetRows.length === 0) {
      toast.warning(t('bugBounty.apiInventory.noProjectTargets'))
      return
    }

    for (const target of targetRows) {
      const detail = await loadTargetDetailByKey(target)
      projectRequestProgress.value.processedTargets += 1
      if (!detail || detail.endpoints.length === 0) {
        continue
      }

      const shouldMarkVisible = selectedTargetKey.value === targetKeyOf(target)
      const results = await requestEndpointBatch(detail, detail.endpoints, shouldMarkVisible)
      const failedCount = results.filter(result => !result.success || Boolean(result.error)).length
      projectRequestProgress.value.requestedEndpoints += results.length
      projectRequestProgress.value.failedEndpoints += failedCount
    }

    toast.success(t('bugBounty.apiInventory.projectRequestCompleted', {
      targets: projectRequestProgress.value.processedTargets,
      total: projectRequestProgress.value.requestedEndpoints,
      failed: projectRequestProgress.value.failedEndpoints,
    }))
  } catch (error) {
    console.error('Failed to request project API inventory endpoints:', error)
    toast.error(t('bugBounty.apiInventory.requestFailed'))
  } finally {
    requestingProject.value = false
  }
}

const handleTargetListScroll = async (event: Event) => {
  const element = event.target as HTMLElement | null
  if (!element) return
  const threshold = 160
  const distanceToBottom = element.scrollHeight - element.scrollTop - element.clientHeight
  if (distanceToBottom <= threshold) {
    await loadMoreTargets()
  }
}

const handleProgramFilterChange = () => {
  programFilterTouched.value = true
}

const clearFilters = () => {
  search.value = ''
  statusFilter.value = 'all'
  executionModeFilter.value = 'all'
  capabilityFilter.value = 'all'
  sortBy.value = 'observed_desc'
  jsonOnly.value = false
  programFilterTouched.value = false
  programFilter.value = propSelectedProgramId.value
  advancedFiltersExpanded.value = false
}

const formatDateTime = (value?: string | null) => {
  if (!value) return '-'
  const parsed = new Date(value)
  if (Number.isNaN(parsed.getTime())) return value
  return parsed.toLocaleString()
}

const resolveEndpointUrl = (baseUrl: string, path: string) => {
  try {
    return new URL(path, baseUrl).toString()
  } catch {
    return path
  }
}

watch(propSelectedProgramId, next => {
  programFilterTouched.value = false
  programFilter.value = next
}, { immediate: true })

watch(
  () => targets.value.map(target => targetKeyOf(target)).join('|'),
  async () => {
    await syncSelectedTargetWithFilters()
  },
)

watch(
  () => visibleEndpoints.value.map(endpoint => endpointKeyOf(endpoint)).join('|'),
  () => {
    const activeKey = selectedEndpoint.value ? endpointKeyOf(selectedEndpoint.value) : ''
    selectedEndpointKey.value = activeKey
  },
  { immediate: true },
)

watch(endpointTab, () => {
  selectedEndpointKey.value = ''
})

watch(endpointRequestMethod, async method => {
  closeEndpointResultDialog()
  await loadEndpointRequestHistory(selectedDetail.value, method)
})

watch(hasEndpointRequestResultsForCurrentMethod, hasResults => {
  if (!hasResults) {
    closeEndpointResultDialog()
  }
})

watch(
  () => [
    programFilter.value,
    search.value,
    statusFilter.value,
    executionModeFilter.value,
    capabilityFilter.value,
    sortBy.value,
  ].join('|'),
  async () => {
    clearSelection()
    await loadTargets(true)
  },
)

watch(hasAdvancedFilters, next => {
  if (next) {
    advancedFiltersExpanded.value = true
  }
})

watch(
  () => selectedDetail.value?.run_id || '',
  () => {
    selectedEndpointKey.value = ''
  },
)

onMounted(async () => {
  await loadTargets(true)
})
</script>
