<template>
  <!-- Backdrop -->
  <Teleport to="body">
    <Transition name="fade">
      <div v-if="modelValue" class="fixed inset-0 z-40 bg-black/40" @click="$emit('update:modelValue', false)" />
    </Transition>

    <!-- Drawer panel -->
    <Transition name="slide-right">
      <div
        v-if="modelValue"
        class="fixed right-0 top-0 bottom-0 z-50 w-full max-w-5xl bg-base-100 shadow-2xl flex flex-col"
      >
        <!-- Drawer Header -->
        <div class="flex items-center justify-between px-6 py-4 border-b border-base-300 shrink-0">
          <h2 class="text-lg font-bold">{{ $t('llmSecurity.suiteDrawer.title') }}</h2>
          <div class="flex items-center gap-2">
            <button class="btn btn-sm btn-outline" @click="openImportJson">
              <svg class="w-4 h-4 mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-8l-4-4m0 0L8 8m4-4v12" /></svg>
              {{ $t('llmSecurity.suiteManager.importJson') }}
            </button>
            <button class="btn btn-sm btn-outline" @click="exportJson">
              <svg class="w-4 h-4 mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4" /></svg>
              {{ $t('llmSecurity.suiteManager.exportJson') }}
            </button>
            <button class="btn btn-sm btn-ghost" @click="$emit('update:modelValue', false)">
              <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" /></svg>
            </button>
          </div>
        </div>

        <!-- Drawer Body -->
        <div class="flex flex-1 overflow-hidden">

          <!-- Left: Suite List -->
          <div class="w-64 shrink-0 border-r border-base-300 flex flex-col">
            <div class="p-3 border-b border-base-300">
              <button class="btn btn-primary btn-sm w-full" @click="startCreateSuite">
                <svg class="w-4 h-4 mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4" /></svg>
                {{ $t('llmSecurity.suiteDrawer.newSuite') }}
              </button>
            </div>
            <div class="flex-1 overflow-y-auto py-1">
              <button
                v-for="suite in suites"
                :key="suite.id"
                class="w-full text-left px-3 py-2.5 hover:bg-base-200 transition-colors flex items-start gap-2 border-b border-base-200/60"
                :class="{ 'bg-primary/10 border-l-2 border-l-primary': selectedSuiteId === suite.id }"
                @click="selectSuite(suite.id)"
              >
                <div class="flex-1 min-w-0">
                  <div class="flex items-center gap-1.5 flex-wrap">
                    <span class="text-sm font-medium truncate">{{ suite.name }}</span>
                  </div>
                  <div class="flex items-center gap-2 mt-0.5">
                    <span class="text-xs opacity-50 font-mono">{{ suite.version }}</span>
                    <span class="text-xs opacity-50">· {{ $t('llmSecurity.suiteDrawer.caseCount', { count: suite.cases?.length ?? 0 }) }}</span>
                  </div>
                </div>
              </button>
            </div>
          </div>

          <!-- Right: Suite Detail -->
          <div class="flex-1 flex flex-col overflow-hidden">
            <!-- No selection placeholder -->
            <div v-if="!selectedSuite" class="flex-1 flex items-center justify-center">
              <div class="text-center opacity-40">
                <svg class="w-16 h-16 mx-auto" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="1" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" /></svg>
                <p class="mt-2 text-sm">{{ $t('llmSecurity.suiteDrawer.noSuiteSelected') }}</p>
              </div>
            </div>

            <!-- Selected suite detail -->
            <template v-else>
              <!-- Suite header -->
              <div class="px-5 py-4 border-b border-base-300 shrink-0">
                <div class="flex items-start justify-between gap-3">
                  <div class="flex-1 min-w-0">
                    <div class="flex items-center gap-2 flex-wrap">
                      <h3 class="text-base font-bold">{{ selectedSuite.name }}</h3>
                    </div>
                    <div class="flex items-center gap-3 mt-1 text-xs opacity-60">
                      <span class="font-mono">{{ selectedSuite.id }}</span>
                      <span>v{{ selectedSuite.version }}</span>
                      <span v-if="selectedSuite.description">· {{ selectedSuite.description }}</span>
                    </div>
                  </div>
                  <div class="flex gap-2 shrink-0">
                      <button class="btn btn-sm btn-outline" @click="cloneSuite(selectedSuite)">
                        <svg class="w-4 h-4 mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z" /></svg>
                        {{ $t('llmSecurity.suiteDrawer.cloneSuite') }}
                      </button>
                      <button class="btn btn-sm btn-outline" @click="startEditSuite(selectedSuite)">
                        <svg class="w-4 h-4 mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z" /></svg>
                        {{ $t('llmSecurity.suiteDrawer.editSuite') }}
                      </button>
                      <button class="btn btn-sm btn-error btn-outline" @click="confirmDeleteSuite(selectedSuite)">
                        <svg class="w-4 h-4 mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" /></svg>
                        {{ $t('llmSecurity.suiteDrawer.confirmDeleteSuite') }}
                      </button>
                  </div>
                </div>
              </div>

              <!-- Cases toolbar -->
              <div class="px-5 py-3 border-b border-base-300 shrink-0 flex items-center justify-between gap-3">
                <span class="text-sm font-medium">
                  {{ $t('llmSecurity.suiteDrawer.caseCount', { count: selectedSuite.cases?.length ?? 0 }) }}
                </span>
                <div class="flex items-center gap-2">
                  <button
                    class="btn btn-sm btn-primary"
                    @click="startAddCase"
                  >
                    <svg class="w-4 h-4 mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4" /></svg>
                    {{ $t('llmSecurity.suiteDrawer.addCase') }}
                  </button>
                </div>
              </div>

              <!-- Cases table -->
              <div class="flex-1 overflow-y-auto">
                <div v-if="!selectedSuite.cases?.length" class="flex items-center justify-center h-full">
                  <div class="text-center opacity-40">
                    <svg class="w-12 h-12 mx-auto" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="1" d="M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2" /></svg>
                    <p class="mt-2 text-sm">{{ $t('llmSecurity.suiteDrawer.noCases') }}</p>
                  </div>
                </div>
                <table v-else class="table table-sm w-full">
                  <thead class="sticky top-0 bg-base-100 z-10">
                    <tr>
                      <th class="w-36">{{ $t('llmSecurity.suiteDrawer.table.caseId') }}</th>
                      <th class="w-24">{{ $t('llmSecurity.suiteDrawer.table.owaspId') }}</th>
                      <th>{{ $t('llmSecurity.suiteDrawer.table.prompt') }}</th>
                      <th class="w-48">{{ $t('llmSecurity.suiteDrawer.table.pattern') }}</th>
                      <th class="w-24">{{ $t('llmSecurity.suiteDrawer.table.actions') }}</th>
                    </tr>
                  </thead>
                  <tbody>
                    <tr
                      v-for="c in selectedSuite.cases"
                      :key="c.case_id"
                      class="hover:bg-base-200/50 transition-colors"
                    >
                      <td>
                        <a class="link link-hover text-primary font-mono text-xs" @click="viewCase(c)">{{ c.case_id }}</a>
                      </td>
                      <td>
                        <span v-if="c.owasp_id" class="badge badge-xs badge-outline">{{ c.owasp_id }}</span>
                        <span v-else class="opacity-40">-</span>
                      </td>
                      <td class="max-w-xs">
                        <p class="text-xs line-clamp-2 leading-relaxed" :title="casePromptPreview(c)">{{ casePromptPreview(c) }}</p>
                      </td>
                      <td>
                        <span v-if="c.regex_not_match" class="font-mono text-xs opacity-70 truncate block max-w-[180px]" :title="c.regex_not_match">{{ c.regex_not_match }}</span>
                        <span v-else class="opacity-30 text-xs">-</span>
                      </td>
                      <td>
                        <div class="flex gap-1">
                          <button class="btn btn-xs btn-ghost" @click="startEditCase(c)">
                            <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z" /></svg>
                          </button>
                          <button class="btn btn-xs btn-ghost text-error" @click="confirmDeleteCase(c.case_id)">
                            <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" /></svg>
                          </button>
                        </div>
                      </td>
                    </tr>
                  </tbody>
                </table>
              </div>
            </template>
          </div>
        </div>

        <!-- Error bar -->
        <div v-if="lastError" class="px-5 py-2 bg-error/10 border-t border-error/30 flex items-center gap-2 shrink-0">
          <svg class="w-4 h-4 text-error shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" /></svg>
          <span class="text-sm text-error flex-1">{{ lastError }}</span>
          <button class="btn btn-xs btn-ghost" @click="lastError = ''">✕</button>
        </div>
      </div>
    </Transition>
  </Teleport>

  <!-- Suite Create/Edit Modal -->
  <Teleport to="body">
  <dialog :class="['modal', { 'modal-open': suiteFormOpen }]" style="z-index:1001" @click.self="closeSuiteForm">
    <div class="modal-box max-w-md">
      <h3 class="font-bold text-lg mb-4">
        {{ suiteFormMode === 'create' ? $t('llmSecurity.suiteDrawer.newSuite') : $t('llmSecurity.suiteDrawer.editSuite') }}
      </h3>
      <div class="space-y-4">
        <div class="form-control">
          <label class="label"><span class="label-text font-medium">{{ $t('llmSecurity.suiteDrawer.suiteForm.suiteId') }} <span class="text-error">*</span></span></label>
          <input
            v-model="suiteForm.id"
            class="input input-bordered"
            :class="{ 'input-error': suiteFormErrors.id }"
            :disabled="suiteFormMode === 'edit'"
            :placeholder="$t('llmSecurity.suiteDrawer.suiteForm.suiteIdPlaceholder')"
          />
          <label class="label">
            <span class="label-text-alt text-error" v-if="suiteFormErrors.id">{{ suiteFormErrors.id }}</span>
            <span class="label-text-alt opacity-50" v-else>{{ $t('llmSecurity.suiteDrawer.suiteForm.suiteIdHelp') }}</span>
          </label>
        </div>
        <div class="form-control">
          <label class="label"><span class="label-text font-medium">{{ $t('llmSecurity.suiteDrawer.suiteForm.suiteName') }} <span class="text-error">*</span></span></label>
          <input
            v-model="suiteForm.name"
            class="input input-bordered"
            :class="{ 'input-error': suiteFormErrors.name }"
            :placeholder="$t('llmSecurity.suiteDrawer.suiteForm.suiteNamePlaceholder')"
          />
          <label class="label"><span class="label-text-alt text-error">{{ suiteFormErrors.name }}</span></label>
        </div>
        <div class="form-control">
          <label class="label"><span class="label-text font-medium">{{ $t('llmSecurity.suiteDrawer.suiteForm.version') }} <span class="text-error">*</span></span></label>
          <input
            v-model="suiteForm.version"
            class="input input-bordered"
            :class="{ 'input-error': suiteFormErrors.version }"
            :placeholder="$t('llmSecurity.suiteDrawer.suiteForm.versionPlaceholder')"
          />
          <label class="label"><span class="label-text-alt text-error">{{ suiteFormErrors.version }}</span></label>
        </div>
        <div class="form-control">
          <label class="label"><span class="label-text font-medium">{{ $t('llmSecurity.suiteDrawer.suiteForm.description') }}</span></label>
          <textarea
            v-model="suiteForm.description"
            class="textarea textarea-bordered h-20 text-sm"
            :placeholder="$t('llmSecurity.suiteDrawer.suiteForm.descriptionPlaceholder')"
          />
        </div>
      </div>
      <div class="modal-action">
        <button class="btn btn-ghost" @click="closeSuiteForm">{{ $t('llmSecurity.suiteDrawer.suiteForm.cancel') }}</button>
        <button class="btn btn-primary" @click="saveSuite">{{ $t('llmSecurity.suiteDrawer.suiteForm.save') }}</button>
      </div>
    </div>
  </dialog>
  </Teleport>

  <!-- Case Create/Edit Modal -->
  <Teleport to="body">
  <dialog :class="['modal', { 'modal-open': caseFormOpen }]" style="z-index:1001" @click.self="closeCaseForm">
    <div class="modal-box max-w-2xl">
      <h3 class="font-bold text-lg mb-4">
        {{ caseFormMode === 'add' ? $t('llmSecurity.suiteDrawer.caseForm.titleAdd') : caseFormMode === 'edit' ? $t('llmSecurity.suiteDrawer.caseForm.titleEdit') : $t('llmSecurity.report.caseDetail') }}
      </h3>
      <div class="space-y-4">
        <div class="grid grid-cols-1 md:grid-cols-3 gap-3">
          <div class="form-control">
            <label class="label"><span class="label-text font-medium">{{ $t('llmSecurity.suiteDrawer.caseForm.caseId') }} <span class="text-error">*</span></span></label>
            <input
              v-model="caseForm.case_id"
              class="input input-bordered input-sm"
              :class="[{ 'input-error': caseFormErrors.case_id }, { 'focus:outline-none': caseFormMode === 'edit' || caseFormMode === 'view' }]"
              :readonly="caseFormMode === 'edit' || caseFormMode === 'view'"
              :placeholder="$t('llmSecurity.suiteDrawer.caseForm.caseIdPlaceholder')"
            />
            <label class="label">
              <span class="label-text-alt text-error" v-if="caseFormErrors.case_id">{{ caseFormErrors.case_id }}</span>
              <span class="label-text-alt opacity-50" v-else>{{ $t('llmSecurity.suiteDrawer.caseForm.caseIdHelp') }}</span>
            </label>
          </div>
          <div class="form-control">
            <label class="label"><span class="label-text font-medium">{{ $t('llmSecurity.suiteDrawer.caseForm.owaspId') }}</span></label>
            <select v-model="caseForm.owasp_id" class="select select-bordered select-sm" :disabled="caseFormMode === 'view'" :class="{'disabled:opacity-100 disabled:text-base-content': caseFormMode === 'view'}">
              <option value="">-</option>
              <option v-for="cat in owaspOptions" :key="cat.id" :value="cat.id">{{ cat.id }}</option>
            </select>
          </div>
          <div class="form-control">
            <label class="label"><span class="label-text font-medium">{{ $t('llmSecurity.suiteDrawer.caseForm.owaspTitle') }}</span></label>
            <input
              v-model="caseForm.owasp_title"
              class="input input-bordered input-sm w-full"
              :class="{ 'focus:outline-none': caseFormMode === 'view' }"
              :readonly="caseFormMode === 'view'"
              :placeholder="$t('llmSecurity.suiteDrawer.caseForm.owaspTitlePlaceholder')"
            />
          </div>
        </div>

        <div class="form-control">
          <label class="label">
            <span class="label-text font-medium">{{ $t('llmSecurity.suiteDrawer.caseForm.userPrompt') }}</span>
          </label>
          <textarea
            v-model="caseForm.user_prompt"
            class="textarea textarea-bordered text-sm h-32"
            :class="[{ 'textarea-error': caseFormErrors.user_prompt }, { 'focus:outline-none': caseFormMode === 'view' }]"
            :readonly="caseFormMode === 'view'"
            :placeholder="$t('llmSecurity.suiteDrawer.caseForm.userPromptPlaceholder')"
          />
          <label class="label">
            <span class="label-text-alt text-error" v-if="caseFormErrors.user_prompt">{{ caseFormErrors.user_prompt }}</span>
            <span class="label-text-alt opacity-50" v-else>{{ $t('llmSecurity.suiteDrawer.caseForm.userPromptHelp') }}</span>
          </label>
        </div>
        <div class="form-control">
          <label class="label">
            <span class="label-text font-medium">Multi-turn Messages JSON (optional)</span>
          </label>
          <textarea
            v-model="caseForm.messages_json"
            class="textarea textarea-bordered text-xs h-28 font-mono"
            :class="[{ 'textarea-error': caseFormErrors.messages_json }, { 'focus:outline-none': caseFormMode === 'view' }]"
            :readonly="caseFormMode === 'view'"
            placeholder='[{"role":"system","content":"You are a helpful assistant."},{"role":"user","content":"..."}]'
          />
          <label class="label">
            <span class="label-text-alt text-error" v-if="caseFormErrors.messages_json">{{ caseFormErrors.messages_json }}</span>
            <span class="label-text-alt opacity-50">Provide a non-empty JSON array of {role, content}. If set, this will be used for test execution.</span>
          </label>
        </div>

        <div class="form-control">
          <label class="label"><span class="label-text font-medium">{{ $t('llmSecurity.suiteDrawer.caseForm.regexNotMatch') }}</span></label>
          <input
            v-model="caseForm.regex_not_match"
            class="input input-bordered font-mono text-sm w-full"
            :class="{ 'focus:outline-none': caseFormMode === 'view' }"
            :readonly="caseFormMode === 'view'"
            :placeholder="$t('llmSecurity.suiteDrawer.caseForm.regexNotMatchPlaceholder')"
          />
          <label class="label"><span class="label-text-alt opacity-50">{{ $t('llmSecurity.suiteDrawer.caseForm.regexNotMatchHelp') }}</span></label>
        </div>
      </div>
      <div class="modal-action">
        <button v-if="caseFormMode === 'view'" class="btn btn-primary" @click="closeCaseForm">{{ $t('llmSecurity.report.closeDetail') }}</button>
        <template v-else>
          <button class="btn btn-ghost" @click="closeCaseForm">{{ $t('llmSecurity.suiteDrawer.caseForm.cancel') }}</button>
          <button class="btn btn-primary" @click="saveCase">{{ $t('llmSecurity.suiteDrawer.caseForm.save') }}</button>
        </template>
      </div>
    </div>
  </dialog>
  </Teleport>

  <!-- Confirm Delete Modal -->
  <Teleport to="body">
  <dialog :class="['modal', { 'modal-open': confirmDialog.open }]" style="z-index:1001" @click.self="confirmDialog.open = false">
    <div class="modal-box max-w-sm">
      <h3 class="font-bold text-lg">{{ confirmDialog.title }}</h3>
      <p class="py-4 text-sm opacity-70">{{ confirmDialog.message }}</p>
      <div class="modal-action">
        <button class="btn btn-ghost" @click="confirmDialog.open = false">{{ $t('llmSecurity.executeDialog.cancel') }}</button>
        <button class="btn btn-error" @click="executeConfirm">{{ $t('llmSecurity.suiteDrawer.confirmDeleteSuite') }}</button>
      </div>
    </div>
  </dialog>
  </Teleport>

  <!-- Import Preview Dialog -->
  <Teleport to="body">
  <dialog :class="['modal', { 'modal-open': importPreview.open }]" style="z-index:1001" @click.self="importPreview.open = false">
    <div class="modal-box w-11/12 max-w-3xl">
      <h3 class="font-bold text-lg mb-3">{{ $t('llmSecurity.importPreview.title') }}</h3>
      <div class="space-y-3 text-sm">
        <p>{{ $t('llmSecurity.importPreview.format') }}: <span class="font-mono">{{ importPreview.formatVersion }}</span></p>
        <div class="grid grid-cols-3 gap-3">
          <div class="bg-base-200 rounded p-2">
            <p class="opacity-70 text-xs">{{ $t('llmSecurity.importPreview.candidates') }}</p>
            <p class="font-semibold">{{ importPreview.candidates.length }}</p>
          </div>
          <div class="bg-base-200 rounded p-2">
            <p class="opacity-70 text-xs">{{ $t('llmSecurity.importPreview.conflicts') }}</p>
            <p class="font-semibold text-warning">{{ importPreview.conflictIds.length }}</p>
          </div>
          <div class="bg-base-200 rounded p-2">
            <p class="opacity-70 text-xs">{{ $t('llmSecurity.importPreview.invalid') }}</p>
            <p class="font-semibold text-error">{{ importPreview.invalidCount }}</p>
          </div>
        </div>
        <div v-if="importPreview.conflictIds.length > 0" class="space-y-1">
          <p class="font-medium">{{ $t('llmSecurity.importPreview.conflictingIds') }}</p>
          <div class="max-h-20 overflow-auto border border-base-300 rounded px-2 py-1 font-mono text-xs">
            <p v-for="id in importPreview.conflictIds" :key="`ci-${id}`">{{ id }}</p>
          </div>
        </div>
        <!-- Preview table -->
        <div class="max-h-48 overflow-auto border border-base-300 rounded">
          <table class="table table-xs">
            <thead>
              <tr>
                <th>ID</th>
                <th>{{ $t('llmSecurity.suiteDrawer.suiteForm.suiteName') }}</th>
                <th>{{ $t('llmSecurity.suiteDrawer.suiteForm.version') }}</th>
                <th>{{ $t('llmSecurity.suiteDrawer.caseCount', { count: '?' }) }}</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="s in importPreview.candidates.slice(0, 20)" :key="`imp-${s.id}`">
                <td class="font-mono">{{ s.id }}</td>
                <td>{{ s.name }}</td>
                <td>{{ s.version }}</td>
                <td>{{ s.cases?.length ?? 0 }}</td>
              </tr>
            </tbody>
          </table>
        </div>
        <div class="flex justify-end gap-2">
          <button class="btn btn-ghost btn-sm" @click="importPreview.open = false">{{ $t('llmSecurity.importPreview.cancel') }}</button>
          <button class="btn btn-outline btn-sm" @click="applyImport(false)">{{ $t('llmSecurity.importPreview.applySkip') }}</button>
          <button class="btn btn-primary btn-sm" @click="applyImport(true)">{{ $t('llmSecurity.importPreview.applyOverwrite') }}</button>
        </div>
      </div>
    </div>
  </dialog>
  </Teleport>
</template>

<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { open, save } from '@tauri-apps/plugin-dialog'
import { readTextFile, writeTextFile } from '@tauri-apps/plugin-fs'
import type { LlmSuiteDefinition } from '../../api/llmTest'

const { t } = useI18n()

const props = defineProps<{
  modelValue: boolean
  suites: LlmSuiteDefinition[]
}>()

const emit = defineEmits<{
  'update:modelValue': [value: boolean]
  'update:suites': [suites: LlmSuiteDefinition[]]
}>()

// OWASP categories for dropdown
const owaspOptions = [
  { id: 'LLM01', title: 'Prompt Injection' },
  { id: 'LLM02', title: 'Sensitive Information Disclosure' },
  { id: 'LLM03', title: 'Supply Chain' },
  { id: 'LLM04', title: 'Data & Model Poisoning' },
  { id: 'LLM05', title: 'Improper Output Handling' },
  { id: 'LLM06', title: 'Excessive Agency' },
  { id: 'LLM07', title: 'System Prompt Leakage' },
  { id: 'LLM08', title: 'Vector & Embedding Weaknesses' },
  { id: 'LLM09', title: 'Misinformation' },
  { id: 'LLM10', title: 'Unbounded Consumption' },
]

// --- Local mutable copy of suites ---
const localSuites = ref<LlmSuiteDefinition[]>([])

watch(() => props.suites, (v) => {
  localSuites.value = JSON.parse(JSON.stringify(v))
}, { immediate: true, deep: true })

const syncUp = () => {
  emit('update:suites', JSON.parse(JSON.stringify(localSuites.value)))
}

// --- Selection state ---
const selectedSuiteId = ref<string | null>(null)

const selectedSuite = computed(() =>
  localSuites.value.find(s => s.id === selectedSuiteId.value) ?? null
)

const selectSuite = (id: string) => {
  selectedSuiteId.value = id
}

// Auto select first when opened
watch(() => props.modelValue, (open) => {
  if (open && !selectedSuiteId.value && localSuites.value.length) {
    selectedSuiteId.value = localSuites.value[0].id
  }
})

// --- Error state ---
const lastError = ref('')

// --- Confirm dialog ---
const confirmDialog = ref({
  open: false,
  title: '',
  message: '',
  action: () => {},
})

const executeConfirm = () => {
  confirmDialog.value.action()
  confirmDialog.value.open = false
}

// --- Suite Form (Create / Edit) ---
const suiteFormOpen = ref(false)
const suiteFormMode = ref<'create' | 'edit'>('create')
const suiteForm = ref({ id: '', name: '', version: '1.0.0', description: '' })
const suiteFormErrors = ref<Record<string, string>>({})

const startCreateSuite = () => {
  suiteFormMode.value = 'create'
  suiteForm.value = { id: '', name: '', version: '1.0.0', description: '' }
  suiteFormErrors.value = {}
  suiteFormOpen.value = true
}

const startEditSuite = (suite: LlmSuiteDefinition) => {
  suiteFormMode.value = 'edit'
  suiteForm.value = { id: suite.id, name: suite.name, version: suite.version, description: suite.description ?? '' }
  suiteFormErrors.value = {}
  suiteFormOpen.value = true
}

const closeSuiteForm = () => {
  suiteFormOpen.value = false
}

const saveSuite = () => {
  suiteFormErrors.value = {}
  const id = suiteForm.value.id.trim()
  const name = suiteForm.value.name.trim()
  const version = suiteForm.value.version.trim()

  if (!id) { suiteFormErrors.value.id = t('llmSecurity.suiteDrawer.errors.suiteIdRequired'); return }
  if (!name) { suiteFormErrors.value.name = t('llmSecurity.suiteDrawer.errors.suiteNameRequired'); return }
  if (!version) { suiteFormErrors.value.version = t('llmSecurity.suiteDrawer.errors.versionRequired'); return }

  if (suiteFormMode.value === 'create') {
    if (localSuites.value.some(s => s.id === id)) {
      suiteFormErrors.value.id = t('llmSecurity.suiteDrawer.errors.suiteIdExists')
      return
    }
    localSuites.value.push({ id, name, version, description: suiteForm.value.description, cases: [] })
    selectedSuiteId.value = id
  } else {
    const idx = localSuites.value.findIndex(s => s.id === id)
    if (idx >= 0) {
      localSuites.value[idx] = { ...localSuites.value[idx], name, version, description: suiteForm.value.description }
    }
  }

  syncUp()
  closeSuiteForm()
}

const cloneSuite = (suite: LlmSuiteDefinition) => {
  const newId = suite.id + '-copy-' + Date.now().toString(36)
  const newName = suite.name + t('llmSecurity.suiteDrawer.clonedSuffix')
  const cloned: LlmSuiteDefinition = {
    id: newId,
    name: newName,
    version: suite.version,
    description: suite.description,
    cases: JSON.parse(JSON.stringify(suite.cases ?? [])),
  }
  localSuites.value.push(cloned)
  selectedSuiteId.value = newId
  syncUp()
}

const confirmDeleteSuite = (suite: LlmSuiteDefinition) => {
  confirmDialog.value = {
    open: true,
    title: t('llmSecurity.suiteDrawer.confirmDeleteSuite'),
    message: t('llmSecurity.suiteDrawer.confirmDeleteSuiteMsg', { name: suite.name }),
    action: () => deleteSuite(suite.id),
  }
}

const deleteSuite = (id: string) => {
  localSuites.value = localSuites.value.filter(s => s.id !== id)
  if (selectedSuiteId.value === id) {
    selectedSuiteId.value = localSuites.value[0]?.id ?? null
  }
  syncUp()
}

// --- Case Form (Add / Edit) ---
const caseFormOpen = ref(false)
const caseFormMode = ref<'add' | 'edit' | 'view'>('add')
const caseForm = ref({ case_id: '', owasp_id: '', owasp_title: '', user_prompt: '', messages_json: '', regex_not_match: '' })
const caseFormErrors = ref<Record<string, string>>({})

// Auto-fill owasp_title when owasp_id changes
watch(() => caseForm.value.owasp_id, (id) => {
  if (id && caseFormMode.value === 'add') {
    const found = owaspOptions.find(o => o.id === id)
    if (found) caseForm.value.owasp_title = found.title
  }
})

const startAddCase = () => {
  caseFormMode.value = 'add'
  caseForm.value = { case_id: '', owasp_id: '', owasp_title: '', user_prompt: '', messages_json: '', regex_not_match: '' }
  caseFormErrors.value = {}
  caseFormOpen.value = true
}

const startEditCase = (c: NonNullable<LlmSuiteDefinition['cases']>[number]) => {
  caseFormMode.value = 'edit'
  caseForm.value = {
    case_id: c.case_id,
    owasp_id: c.owasp_id ?? '',
    owasp_title: c.owasp_title ?? '',
    user_prompt: c.user_prompt,
    messages_json: Array.isArray(c.messages) ? JSON.stringify(c.messages, null, 2) : '',
    regex_not_match: c.regex_not_match ?? '',
  }
  caseFormErrors.value = {}
  caseFormOpen.value = true
}

const viewCase = (c: NonNullable<LlmSuiteDefinition['cases']>[number]) => {
  caseFormMode.value = 'view'
  caseForm.value = {
    case_id: c.case_id,
    owasp_id: c.owasp_id ?? '',
    owasp_title: c.owasp_title ?? '',
    user_prompt: c.user_prompt,
    messages_json: Array.isArray(c.messages) ? JSON.stringify(c.messages, null, 2) : '',
    regex_not_match: c.regex_not_match ?? '',
  }
  caseFormErrors.value = {}
  caseFormOpen.value = true
}

const closeCaseForm = () => {
  caseFormOpen.value = false
}

const saveCase = () => {
  caseFormErrors.value = {}
  const case_id = caseForm.value.case_id.trim()
  const user_prompt = caseForm.value.user_prompt.trim()
  const messagesJson = caseForm.value.messages_json.trim()
  let parsedMessages: Array<{ role: string; content: string }> | undefined

  if (messagesJson) {
    try {
      const parsed = JSON.parse(messagesJson)
      if (!Array.isArray(parsed) || parsed.length === 0) {
        caseFormErrors.value.messages_json = 'messages_json must be a non-empty array'
        return
      }
      parsedMessages = parsed
        .filter((m: any) => m?.role && typeof m?.content === 'string')
        .map((m: any) => ({ role: String(m.role), content: String(m.content) }))
      if (!parsedMessages.length) {
        caseFormErrors.value.messages_json = 'messages_json has no valid message entries'
        return
      }
    } catch (e: any) {
      caseFormErrors.value.messages_json = `Invalid JSON: ${e?.message ?? String(e)}`
      return
    }
  }
  const fallbackUserPrompt = parsedMessages
    ? [...parsedMessages].reverse().find(m => m.role === 'user')?.content || ''
    : ''

  if (!case_id) { caseFormErrors.value.case_id = t('llmSecurity.suiteDrawer.errors.caseIdRequired'); return }
  if (!user_prompt && !parsedMessages?.length) { caseFormErrors.value.user_prompt = t('llmSecurity.suiteDrawer.errors.promptRequired'); return }

  const suiteIdx = localSuites.value.findIndex(s => s.id === selectedSuiteId.value)
  if (suiteIdx < 0) return

  const suite = localSuites.value[suiteIdx]
  const cases = Array.isArray(suite.cases) ? [...suite.cases] : []

  if (caseFormMode.value === 'add') {
    if (cases.some(c => c.case_id === case_id)) {
      caseFormErrors.value.case_id = t('llmSecurity.suiteDrawer.errors.caseIdExistsInSuite')
      return
    }
    cases.push({
      case_id,
      owasp_id: caseForm.value.owasp_id || undefined,
      owasp_title: caseForm.value.owasp_title || undefined,
      user_prompt: user_prompt || fallbackUserPrompt,
      messages: parsedMessages,
      regex_not_match: caseForm.value.regex_not_match.trim() || undefined,
    })
  } else {
    const cIdx = cases.findIndex(c => c.case_id === case_id)
    if (cIdx >= 0) {
      cases[cIdx] = {
        case_id,
        owasp_id: caseForm.value.owasp_id || undefined,
        owasp_title: caseForm.value.owasp_title || undefined,
        user_prompt: user_prompt || fallbackUserPrompt,
        messages: parsedMessages,
        regex_not_match: caseForm.value.regex_not_match.trim() || undefined,
      }
    }
  }

  localSuites.value[suiteIdx] = { ...suite, cases }
  syncUp()
  closeCaseForm()
}

const confirmDeleteCase = (caseId: string) => {
  confirmDialog.value = {
    open: true,
    title: t('llmSecurity.suiteDrawer.confirmDeleteCase'),
    message: t('llmSecurity.suiteDrawer.confirmDeleteCaseMsg', { id: caseId }),
    action: () => deleteCase(caseId),
  }
}

const deleteCase = (caseId: string) => {
  const suiteIdx = localSuites.value.findIndex(s => s.id === selectedSuiteId.value)
  if (suiteIdx < 0) return
  const suite = localSuites.value[suiteIdx]
  localSuites.value[suiteIdx] = {
    ...suite,
    cases: (suite.cases ?? []).filter(c => c.case_id !== caseId),
  }
  syncUp()
}

// --- Import / Export ---
const importPreview = ref({
  open: false,
  formatVersion: '',
  candidates: [] as LlmSuiteDefinition[],
  conflictIds: [] as string[],
  invalidCount: 0,
})

const openImportJson = async () => {
  try {
    lastError.value = ''
    const selected = await open({
      directory: false,
      multiple: false,
      filters: [{ name: t('llmSecurity.report.exportFileFilter'), extensions: ['json'] }],
      title: t('llmSecurity.suiteManager.importJson'),
    })
    if (!selected) return
    const content = await readTextFile(selected as string)
    const parsed = JSON.parse(content)
    const decoded = decodeImportPayload(parsed)
    const existingIds = new Set(localSuites.value.map(s => s.id))
    importPreview.value = {
      open: true,
      formatVersion: decoded.formatVersion,
      candidates: decoded.candidates,
      conflictIds: decoded.candidates.filter(s => existingIds.has(s.id)).map(s => s.id),
      invalidCount: decoded.invalidCount,
    }
  } catch (err: any) {
    lastError.value = err?.message ?? String(err)
  }
}

const applyImport = (overwriteConflicts: boolean) => {
  try {
    const existingMap = new Map(
      localSuites.value.map(s => [s.id, s] as const)
    )
    for (const suite of importPreview.value.candidates) {
      if (existingMap.has(suite.id) && !overwriteConflicts) continue
      existingMap.set(suite.id, suite)
    }
    localSuites.value = Array.from(existingMap.values())
    syncUp()
    importPreview.value.open = false
  } catch (err: any) {
    lastError.value = err?.message ?? String(err)
  }
}

const exportJson = async () => {
  try {
    lastError.value = ''
    const allSuites = localSuites.value
      .map(s => ({
        id: s.id,
        name: s.name,
        version: s.version,
        description: s.description ?? '',
        cases: Array.isArray(s.cases) ? s.cases : [],
      }))
    const selected = await save({
      defaultPath: `llm_test_suites_${new Date().toISOString().split('T')[0]}.json`,
      filters: [{ name: t('llmSecurity.report.exportFileFilter'), extensions: ['json'] }],
      title: t('llmSecurity.suiteManager.exportJson'),
    })
    if (!selected) return
    await writeTextFile(
      selected,
      JSON.stringify({ format_version: '1.0', exported_at: new Date().toISOString(), suites: allSuites }, null, 2)
    )
  } catch (err: any) {
    lastError.value = err?.message ?? String(err)
  }
}

const decodeImportPayload = (parsed: any): { formatVersion: string; candidates: LlmSuiteDefinition[]; invalidCount: number } => {
  let rawSuites: any[] = []
  let formatVersion = 'legacy-array'

  if (Array.isArray(parsed)) {
    rawSuites = parsed
  } else if (parsed && typeof parsed === 'object') {
    formatVersion = String(parsed.format_version ?? 'unknown')
    if (Array.isArray(parsed.suites)) rawSuites = parsed.suites
    else throw new Error('Invalid import payload: suites array missing')
  } else {
    throw new Error('Invalid JSON payload')
  }

  const unique = new Map<string, LlmSuiteDefinition>()
  let invalidCount = 0
  for (const item of rawSuites) {
    const id = String(item?.id ?? '').trim()
    const name = String(item?.name ?? '').trim()
    const version = String(item?.version ?? '').trim()
    if (!id || !name || !version) { invalidCount += 1; continue }
    unique.set(id, {
      id, name, version,
      description: item?.description ? String(item.description) : '',
      cases: Array.isArray(item?.cases)
        ? item.cases
            .filter((c: any) => {
              if (!c?.case_id) return false
              if (typeof c?.user_prompt === 'string' && c.user_prompt.trim().length > 0) return true
              if (!Array.isArray(c?.messages)) return false
              return c.messages.some((m: any) => m?.role && typeof m?.content === 'string')
            })
            .map((c: any) => ({
              case_id: String(c.case_id),
              owasp_id: c.owasp_id ? String(c.owasp_id) : undefined,
              owasp_title: c.owasp_title ? String(c.owasp_title) : undefined,
              user_prompt: c.user_prompt ? String(c.user_prompt) : '',
              messages: Array.isArray(c.messages)
                ? c.messages
                    .filter((m: any) => m?.role && typeof m?.content === 'string')
                    .map((m: any) => ({ role: String(m.role), content: String(m.content) }))
                : undefined,
              regex_not_match: c.regex_not_match ? String(c.regex_not_match) : undefined,
            }))
        : [],

    })
  }
  return { formatVersion, candidates: Array.from(unique.values()), invalidCount }
}

const casePromptPreview = (c: NonNullable<LlmSuiteDefinition['cases']>[number]) => {
  if (c.user_prompt) return c.user_prompt
  if (Array.isArray(c.messages)) {
    const lastUser = [...c.messages].reverse().find(m => m.role === 'user')
    if (lastUser?.content) return lastUser.content
  }
  return '-'
}
</script>

<style scoped>
.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.2s ease;
}
.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}

.slide-right-enter-active,
.slide-right-leave-active {
  transition: transform 0.25s ease;
}
.slide-right-enter-from,
.slide-right-leave-to {
  transform: translateX(100%);
}
</style>
