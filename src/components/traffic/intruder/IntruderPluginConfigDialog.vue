<template>
  <dialog :open="open" class="modal" @click.self="$emit('close')">
    <div class="modal-box max-w-2xl">
      <div class="mb-4 flex items-center justify-between">
        <h3 class="text-base font-semibold">{{ title }}</h3>
        <button class="btn btn-ghost btn-xs" type="button" @click="$emit('close')">✕</button>
      </div>

      <div class="mb-4 flex items-center justify-between gap-3">
        <div class="tabs tabs-boxed tabs-sm">
          <button
            type="button"
            class="tab"
            :class="{ 'tab-active': editorMode === 'form' }"
            @click="editorMode = 'form'"
          >
            {{ $t('trafficAnalysis.intruder.labels.formMode') }}
          </button>
          <button
            type="button"
            class="tab"
            :class="{ 'tab-active': editorMode === 'json' }"
            @click="editorMode = 'json'"
          >
            {{ $t('trafficAnalysis.intruder.labels.jsonMode') }}
          </button>
        </div>

        <div class="flex items-center gap-2">
          <select
            v-if="pluginId && availablePresets.length"
            v-model="selectedPresetName"
            class="select select-bordered select-xs w-40"
            @change="applySelectedPreset"
          >
            <option value="">{{ $t('trafficAnalysis.intruder.labels.selectPreset') }}</option>
            <option v-for="preset in availablePresets" :key="preset.name" :value="preset.name">
              {{ preset.name }}
            </option>
          </select>
          <button
            v-if="pluginId"
            class="btn btn-ghost btn-xs"
            type="button"
            @click="savePreset"
          >
            {{ $t('trafficAnalysis.intruder.actions.savePreset') }}
          </button>
          <button
            v-if="pluginId"
            class="btn btn-ghost btn-xs"
            type="button"
            :disabled="!selectedPresetName"
            @click="deletePreset"
          >
            {{ $t('trafficAnalysis.intruder.actions.deletePreset') }}
          </button>
          <button
            class="btn btn-ghost btn-xs"
            type="button"
            @click="importPreset"
          >
            {{ $t('trafficAnalysis.intruder.actions.importPreset') }}
          </button>
          <button
            class="btn btn-ghost btn-xs"
            type="button"
            @click="exportPreset"
          >
            {{ $t('trafficAnalysis.intruder.actions.exportPreset') }}
          </button>
          <button
            v-if="hasEditableSchema"
            class="btn btn-ghost btn-xs"
            type="button"
            @click="resetToDefaults"
          >
            {{ $t('trafficAnalysis.intruder.actions.resetDefaults') }}
          </button>
        </div>
      </div>

      <div
        v-if="editableSchemaDescription"
        class="mb-4 rounded-lg border border-base-300 bg-base-200 px-3 py-2 text-xs text-base-content/70"
      >
        {{ editableSchemaDescription }}
      </div>

      <div v-if="editorMode === 'form' && hasEditableSchema" class="space-y-4">
        <details
          v-for="group in fieldGroups"
          :key="group.id"
          class="rounded-lg border border-base-300 bg-base-100"
          :open="!group.collapsed"
        >
          <summary class="cursor-pointer list-none px-4 py-3">
            <div class="flex items-center justify-between gap-3">
              <div>
                <div class="text-sm font-semibold">{{ group.label }}</div>
                <div v-if="group.description" class="mt-1 text-xs text-base-content/60">
                  {{ group.description }}
                </div>
              </div>
              <span class="badge badge-ghost badge-sm">{{ group.fields.length }}</span>
            </div>
          </summary>

          <div class="grid gap-4 border-t border-base-300 px-4 py-4 lg:grid-cols-2">
            <div
              v-for="field in group.fields"
              :key="field.key"
              class="form-control rounded-lg border border-base-300 p-3"
              :class="[getFieldContainerClass(field.prop), getFieldStateClass(field.key, field.prop)]"
            >
          <label class="label py-1">
            <span class="label-text text-xs font-semibold">
              {{ formatFieldLabel(field.key) }}
              <span v-if="editableSchema.required?.includes(field.key)" class="text-error">*</span>
            </span>
          </label>

          <textarea
            v-if="field.prop['x-ui-widget'] === 'textarea' || field.prop['x-ui-widget'] === 'textarea-lines'"
            v-model="paramValues[field.key]"
            class="textarea textarea-bordered textarea-sm w-full font-mono text-xs"
            :rows="field.prop['x-ui-widget'] === 'textarea-lines' ? 5 : 3"
            :disabled="isFieldDisabled(field.key, field.prop)"
          ></textarea>

          <input
            v-else-if="field.prop.type === 'string' && !field.prop.enum"
            v-model="paramValues[field.key]"
            type="text"
            class="input input-bordered input-sm"
            :disabled="isFieldDisabled(field.key, field.prop)"
          />

          <input
            v-else-if="field.prop.type === 'integer' || field.prop.type === 'number'"
            v-model.number="paramValues[field.key]"
            type="number"
            class="input input-bordered input-sm"
            :disabled="isFieldDisabled(field.key, field.prop)"
          />

          <select
            v-else-if="field.prop.enum?.length"
            v-model="paramValues[field.key]"
            class="select select-bordered select-sm"
            :disabled="isFieldDisabled(field.key, field.prop)"
          >
            <option value="">{{ $t('trafficAnalysis.intruder.labels.pleaseSelect') }}</option>
            <option v-for="option in field.prop.enum" :key="option" :value="option">{{ option }}</option>
          </select>

          <label v-else-if="field.prop.type === 'boolean'" class="flex items-center gap-2">
            <input v-model="paramValues[field.key]" type="checkbox" class="toggle toggle-sm toggle-primary" :disabled="isFieldDisabled(field.key, field.prop)" />
            <span class="text-xs">{{ paramValues[field.key] ? $t('trafficAnalysis.intruder.labels.enabled') : $t('trafficAnalysis.intruder.labels.disabled') }}</span>
          </label>

          <div
            v-else-if="field.prop.type === 'array'"
            class="space-y-3"
          >
            <div
              v-if="supportsDictionaryField(field.prop)"
              class="flex flex-wrap gap-2"
            >
              <button
                v-if="supportsDictionaryInsert(field.prop)"
                class="btn btn-outline btn-xs"
                type="button"
                :disabled="isFieldDisabled(field.key, field.prop)"
                @click="openDictionaryPicker(field.key, field.prop, 'insert')"
              >
                {{ $t('trafficAnalysis.intruder.actions.insertDictionaryWords') }}
              </button>
              <button
                v-if="supportsDictionaryReference(field.prop)"
                class="btn btn-outline btn-xs"
                type="button"
                :disabled="isFieldDisabled(field.key, field.prop)"
                @click="openDictionaryPicker(field.key, field.prop, 'reference')"
              >
                {{ $t('trafficAnalysis.intruder.actions.selectDictionaryReference') }}
              </button>
            </div>

            <div class="flex flex-wrap gap-2 rounded-lg border border-base-300 bg-base-100 p-3 min-h-16">
              <span
                v-for="(item, index) in getArrayItems(field.key)"
                :key="`${field.key}-${item}-${index}`"
                class="badge badge-primary badge-outline gap-2 py-3"
              >
                <span class="max-w-56 truncate">{{ formatArrayItemLabel(field.prop, item) }}</span>
                <button
                  class="btn btn-ghost btn-xs min-h-0 h-4 w-4 p-0"
                  type="button"
                  :disabled="isFieldDisabled(field.key, field.prop)"
                  @click="removeArrayItem(field.key, index)"
                >
                  ×
                </button>
              </span>

              <span
                v-if="!getArrayItems(field.key).length"
                class="text-xs text-base-content/50"
              >
                {{ $t('trafficAnalysis.intruder.empty.noArrayItems') }}
              </span>
            </div>

            <div class="flex gap-2">
              <input
                v-model="arrayDraftValues[field.key]"
                type="text"
                class="input input-bordered input-sm flex-1"
                :placeholder="$t('trafficAnalysis.intruder.placeholders.arrayItem')"
                :disabled="isFieldDisabled(field.key, field.prop)"
                @keydown.enter.prevent="addArrayItem(field.key)"
              />
              <button
                class="btn btn-outline btn-sm"
                type="button"
                :disabled="isFieldDisabled(field.key, field.prop)"
                @click="addArrayItem(field.key)"
              >
                {{ $t('trafficAnalysis.intruder.actions.add') }}
              </button>
            </div>
          </div>

          <div
            v-else-if="field.prop.type === 'object' && supportsKeyValueObjectEditor(field.prop)"
            class="space-y-3"
          >
            <div class="space-y-2 rounded-lg border border-base-300 bg-base-100 p-3">
              <div
                v-for="(entry, index) in getObjectEntries(field.key)"
                :key="`${field.key}-${entry.key}-${index}`"
                class="grid gap-2 md:grid-cols-[minmax(0,1fr)_minmax(0,1fr)_auto]"
              >
                <input
                  :value="entry.key"
                  type="text"
                  class="input input-bordered input-sm"
                  :placeholder="$t('trafficAnalysis.intruder.placeholders.objectKey')"
                  :disabled="isFieldDisabled(field.key, field.prop)"
                  @input="updateObjectEntry(field.key, index, 'key', ($event.target as HTMLInputElement).value)"
                />
                <input
                  :value="entry.value"
                  type="text"
                  class="input input-bordered input-sm"
                  :placeholder="$t('trafficAnalysis.intruder.placeholders.objectValue')"
                  :disabled="isFieldDisabled(field.key, field.prop)"
                  @input="updateObjectEntry(field.key, index, 'value', ($event.target as HTMLInputElement).value)"
                />
                <button
                  class="btn btn-ghost btn-sm"
                  type="button"
                  :disabled="isFieldDisabled(field.key, field.prop)"
                  @click="removeObjectEntry(field.key, index)"
                >
                  {{ $t('trafficAnalysis.intruder.actions.remove') }}
                </button>
              </div>

              <div
                v-if="!getObjectEntries(field.key).length"
                class="text-xs text-base-content/50"
              >
                {{ $t('trafficAnalysis.intruder.empty.noObjectItems') }}
              </div>
            </div>

            <div class="grid gap-2 md:grid-cols-[minmax(0,1fr)_minmax(0,1fr)_auto]">
              <input
                :value="getObjectDraft(field.key).key"
                type="text"
                class="input input-bordered input-sm"
                :placeholder="$t('trafficAnalysis.intruder.placeholders.objectKey')"
                :disabled="isFieldDisabled(field.key, field.prop)"
                @input="updateObjectDraft(field.key, 'key', ($event.target as HTMLInputElement).value)"
                @keydown.enter.prevent="addObjectEntry(field.key)"
              />
              <input
                :value="getObjectDraft(field.key).value"
                type="text"
                class="input input-bordered input-sm"
                :placeholder="$t('trafficAnalysis.intruder.placeholders.objectValue')"
                :disabled="isFieldDisabled(field.key, field.prop)"
                @input="updateObjectDraft(field.key, 'value', ($event.target as HTMLInputElement).value)"
                @keydown.enter.prevent="addObjectEntry(field.key)"
              />
              <button
                class="btn btn-outline btn-sm"
                type="button"
                :disabled="isFieldDisabled(field.key, field.prop)"
                @click="addObjectEntry(field.key)"
              >
                {{ $t('trafficAnalysis.intruder.actions.add') }}
              </button>
            </div>
          </div>

          <textarea
            v-else-if="field.prop.type === 'object'"
            v-model="paramValues[field.key]"
            class="textarea textarea-bordered textarea-sm w-full font-mono text-xs"
            rows="5"
            :disabled="isFieldDisabled(field.key, field.prop)"
          ></textarea>

          <textarea v-else v-model="paramValues[field.key]" class="textarea textarea-bordered textarea-sm w-full" rows="3" :disabled="isFieldDisabled(field.key, field.prop)"></textarea>

          <label v-if="field.prop.description" class="label py-1">
            <span class="label-text-alt text-xs opacity-70">{{ field.prop.description }}</span>
          </label>
          <div
            v-if="field.prop.enum?.length && getSelectedEnumDescription(field.prop, paramValues[field.key])"
            class="mt-1 text-xs text-base-content/70"
          >
            {{ getSelectedEnumDescription(field.prop, paramValues[field.key]) }}
          </div>
          <div v-if="field.prop.type === 'array'" class="mt-1 text-xs opacity-60">
            {{ $t('trafficAnalysis.intruder.labels.arrayInputHint') }}
          </div>
          <div v-else-if="field.prop.type === 'object' && supportsKeyValueObjectEditor(field.prop)" class="mt-1 text-xs opacity-60">
            {{ $t('trafficAnalysis.intruder.labels.objectInputHint') }}
          </div>
          <div v-if="isFieldDisabled(field.key, field.prop) && getFieldDisabledReason(field.key, field.prop)" class="mt-1 text-xs text-base-content/60">
            {{ getFieldDisabledReason(field.key, field.prop) }}
          </div>
          <div v-if="jsonErrors[field.key]" class="mt-1 text-xs text-error">{{ jsonErrors[field.key] }}</div>
            </div>
          </div>
        </details>
      </div>

      <div v-else-if="editorMode === 'json'" class="space-y-3">
        <textarea
          v-model="rawConfigText"
          class="textarea textarea-bordered h-80 w-full font-mono text-xs"
          spellcheck="false"
        ></textarea>
        <div class="text-xs text-base-content/60">
          {{ $t('trafficAnalysis.intruder.labels.advancedJsonHint') }}
        </div>
      </div>

      <div v-else class="rounded-lg border border-base-300 bg-base-200 p-4 text-sm text-base-content/70">
        {{ $t('trafficAnalysis.intruder.empty.noPluginParameters') }}
      </div>

      <div v-if="generalError" class="mt-3 text-sm text-error">{{ generalError }}</div>

      <div class="modal-action">
        <button class="btn btn-primary btn-sm" type="button" :disabled="hasErrors" @click="handleSave">
          {{ $t('trafficAnalysis.intruder.actions.saveConfig') }}
        </button>
        <button class="btn btn-outline btn-sm" type="button" @click="$emit('close')">
          {{ $t('trafficAnalysis.intruder.actions.cancel') }}
        </button>
      </div>
    </div>
  </dialog>

  <dialog :open="dictionaryPickerOpen" class="modal" @click.self="closeDictionaryPicker">
    <div class="modal-box max-w-3xl">
      <div class="mb-4 flex items-center justify-between">
        <div>
          <h3 class="text-base font-semibold">
            {{ dictionaryPickerAction === 'reference'
              ? $t('trafficAnalysis.intruder.labels.selectDictionaryReference')
              : $t('trafficAnalysis.intruder.labels.insertDictionaryWords') }}
          </h3>
          <p class="mt-1 text-xs text-base-content/60">
            {{ dictionaryPickerAction === 'reference'
              ? $t('trafficAnalysis.intruder.labels.dictionaryReferenceHint')
              : $t('trafficAnalysis.intruder.labels.dictionaryInsertHint') }}
          </p>
        </div>
        <button class="btn btn-ghost btn-xs" type="button" @click="closeDictionaryPicker">✕</button>
      </div>

      <div class="grid gap-3 md:grid-cols-[minmax(0,1fr)_12rem]">
        <input
          v-model="dictionaryPickerSearch"
          type="text"
          class="input input-bordered input-sm"
          :placeholder="$t('trafficAnalysis.intruder.placeholders.searchDictionary')"
        />
        <select
          v-model="dictionaryPickerType"
          class="select select-bordered select-sm"
        >
          <option value="">{{ $t('trafficAnalysis.intruder.labels.allDictionaryTypes') }}</option>
          <option v-for="option in dictionaryPickerTypeOptions" :key="option" :value="option">
            {{ getDictionaryTypeLabel(option) }}
          </option>
        </select>
      </div>

      <div v-if="dictionaryPickerAction === 'insert'" class="mt-3">
        <label class="label py-1">
          <span class="label-text text-xs font-medium">{{ $t('trafficAnalysis.intruder.labels.dictionaryWordLimit') }}</span>
        </label>
        <input
          v-model.number="dictionaryPickerWordLimit"
          type="number"
          min="1"
          max="5000"
          class="input input-bordered input-sm w-40"
        />
      </div>

      <div
        v-if="dictionaryPickerFieldProp && supportsDictionaryDefault(dictionaryPickerFieldProp)"
        class="mt-3 flex flex-wrap items-center gap-2"
      >
        <button
          class="btn btn-outline btn-xs"
          type="button"
          :disabled="!activeDictionaryType"
          @click="applyDefaultDictionarySelection"
        >
          {{ dictionaryPickerAction === 'reference'
            ? $t('trafficAnalysis.intruder.actions.useDefaultDictionaryReference')
            : $t('trafficAnalysis.intruder.actions.insertDefaultDictionaryWords') }}
        </button>
        <span class="text-xs text-base-content/60">
          {{ activeDictionaryType
            ? $t('trafficAnalysis.intruder.labels.defaultDictionaryTypeHint', { type: getDictionaryTypeLabel(activeDictionaryType) })
            : $t('trafficAnalysis.intruder.labels.selectDictionaryTypeFirst') }}
        </span>
      </div>

      <div v-if="dictionaryPickerError" class="mt-3 rounded-lg border border-error/30 bg-error/10 px-3 py-2 text-sm text-error">
        {{ dictionaryPickerError }}
      </div>

      <div class="mt-4 overflow-hidden rounded-lg border border-base-300">
        <table class="table table-sm">
          <thead class="bg-base-200">
            <tr>
              <th class="w-12"></th>
              <th>{{ $t('trafficAnalysis.intruder.labels.dictionary') }}</th>
              <th class="w-32">{{ $t('trafficAnalysis.intruder.labels.dictionaryType') }}</th>
              <th class="w-24 text-right">{{ $t('trafficAnalysis.intruder.labels.words') }}</th>
            </tr>
          </thead>
          <tbody>
            <tr v-if="dictionaryPickerLoading">
              <td colspan="4" class="py-10 text-center">
                <span class="loading loading-spinner loading-md"></span>
              </td>
            </tr>
            <template v-else>
              <tr
                v-for="dictionary in filteredDictionaries"
                :key="dictionary.id"
                class="cursor-pointer"
                :class="selectedDictionaryId === dictionary.id ? 'bg-primary/10' : ''"
                @click="focusDictionary(dictionary.id)"
              >
                <td>
                  <input
                    :checked="selectedDictionaryIds.includes(dictionary.id)"
                    type="checkbox"
                    class="checkbox checkbox-xs"
                    @click.stop
                    @change="toggleDictionarySelection(dictionary.id, ($event.target as HTMLInputElement).checked)"
                  />
                </td>
                <td>
                  <div class="flex items-center gap-2">
                    <div class="font-medium">{{ dictionary.name }}</div>
                    <span
                      v-if="defaultDictionaryMap[dictionary.dict_type] === dictionary.id"
                      class="badge badge-success badge-xs"
                    >
                      {{ $t('trafficAnalysis.intruder.labels.defaultDictionaryBadge') }}
                    </span>
                  </div>
                  <div class="text-xs font-mono text-base-content/60">{{ dictionary.id }}</div>
                  <div v-if="dictionary.description" class="mt-1 text-xs text-base-content/70 line-clamp-2">
                    {{ dictionary.description }}
                  </div>
                </td>
                <td class="text-xs text-base-content/70">{{ getDictionaryTypeLabel(dictionary.dict_type) }}</td>
                <td class="text-right text-sm">{{ dictionary.word_count || 0 }}</td>
              </tr>
            </template>
            <tr v-if="!dictionaryPickerLoading && !filteredDictionaries.length">
              <td colspan="4" class="py-10 text-center text-sm text-base-content/60">
                {{ $t('trafficAnalysis.intruder.empty.noDictionaries') }}
              </td>
            </tr>
          </tbody>
        </table>
      </div>

      <div class="mt-3 flex items-center justify-between gap-3">
        <div class="space-y-1 text-xs text-base-content/60">
          <div>
            {{ $t('trafficAnalysis.intruder.labels.dictionaryPageSummary', {
              page: dictionaryPickerPage,
              total: dictionaryPickerTotalPages,
              count: dictionaryPickerTotal,
            }) }}
          </div>
          <div>
            {{ $t('trafficAnalysis.intruder.labels.dictionarySelectionSummary', {
              selected: selectedDictionaryIds.length,
              estimate: dictionarySelectionEstimate,
            }) }}
          </div>
        </div>
        <div class="flex items-center gap-2">
          <button
            class="btn btn-outline btn-xs"
            type="button"
            :disabled="!filteredDictionaries.length || dictionaryPickerLoading"
            @click="selectAllVisibleDictionaries"
          >
            {{ $t('trafficAnalysis.intruder.actions.selectPage') }}
          </button>
          <button
            class="btn btn-outline btn-xs"
            type="button"
            :disabled="!selectedDictionaryIds.length"
            @click="clearDictionarySelection"
          >
            {{ $t('trafficAnalysis.intruder.actions.clearSelection') }}
          </button>
          <button
            class="btn btn-outline btn-xs"
            type="button"
            :disabled="dictionaryPickerPage <= 1 || dictionaryPickerLoading"
            @click="changeDictionaryPickerPage(-1)"
          >
            {{ $t('trafficAnalysis.intruder.actions.previousPage') }}
          </button>
          <button
            class="btn btn-outline btn-xs"
            type="button"
            :disabled="dictionaryPickerPage >= dictionaryPickerTotalPages || dictionaryPickerLoading"
            @click="changeDictionaryPickerPage(1)"
          >
            {{ $t('trafficAnalysis.intruder.actions.nextPage') }}
          </button>
        </div>
      </div>

      <div class="mt-4 rounded-lg border border-base-300 bg-base-100 p-3">
        <div class="flex items-center justify-between gap-3">
          <div class="text-sm font-medium">
            {{ $t('trafficAnalysis.intruder.labels.dictionaryWordPreview') }}
          </div>
          <div v-if="selectedDictionary" class="text-xs text-base-content/60">
            {{ selectedDictionary.name }}
          </div>
        </div>

        <div v-if="dictionaryPreviewLoading" class="mt-3 text-sm text-base-content/60">
          {{ $t('trafficAnalysis.intruder.labels.loadingDictionaryPreview') }}
        </div>
        <div v-else-if="dictionaryPreviewError" class="mt-3 text-sm text-error">
          {{ dictionaryPreviewError }}
        </div>
        <div v-else-if="!selectedDictionary" class="mt-3 text-sm text-base-content/60">
          {{ $t('trafficAnalysis.intruder.empty.noDictionarySelected') }}
        </div>
        <div v-else-if="!dictionaryPreviewWords.length" class="mt-3 text-sm text-base-content/60">
          {{ $t('trafficAnalysis.intruder.empty.noDictionaryPreviewWords') }}
        </div>
        <div v-else class="mt-3 flex flex-wrap gap-2">
          <span
            v-for="(word, index) in dictionaryPreviewWords"
            :key="`${selectedDictionary.id}-${index}-${word}`"
            class="badge badge-outline gap-2 py-3"
          >
            <span class="max-w-56 truncate">{{ word }}</span>
          </span>
        </div>
      </div>

      <div class="modal-action">
        <button
          class="btn btn-primary btn-sm"
          type="button"
          :disabled="selectedDictionaryIds.length === 0"
          @click="applyDictionarySelection"
        >
          {{ dictionaryPickerAction === 'reference'
            ? $t('trafficAnalysis.intruder.actions.useDictionaryReference')
            : $t('trafficAnalysis.intruder.actions.insertDictionaryWords') }}
        </button>
        <button class="btn btn-outline btn-sm" type="button" @click="closeDictionaryPicker">
          {{ $t('trafficAnalysis.intruder.actions.cancel') }}
        </button>
      </div>
    </div>
  </dialog>
</template>

<script setup lang="ts">
import { open as openDialog, save as saveDialog } from '@tauri-apps/plugin-dialog'
import { readTextFile, writeTextFile } from '@tauri-apps/plugin-fs'
import { computed, onUnmounted, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { dialog } from '@/composables/useDialog'
import {
  createIntruderDefaultDictionaryReference,
  getIntruderDefaultDictionaryMap,
  getIntruderDefaultDictionaryId,
  getIntruderDictionaryTypeTranslationKey,
  isIntruderDefaultDictionaryReference,
  listIntruderDictionaries,
  listIntruderDictionariesPaged,
  listIntruderDictionaryWords,
  parseIntruderDefaultDictionaryReference,
  type IntruderDictionarySummary,
} from './intruderDictionaries'
import {
  decodeIntruderStructuredDictionarySources,
  encodeIntruderStructuredDictionarySources,
  isIntruderStructuredDictionarySourceArray,
  normalizeIntruderDictionaryReferenceValues,
} from './intruderDictionarySources'
import {
  loadIntruderDictionaryPickerType,
  saveIntruderDictionaryPickerType,
} from './intruderDictionaryPickerPreferences'
import {
  deleteIntruderPluginConfigPreset,
  loadIntruderPluginConfigPresets,
  saveIntruderPluginConfigPreset,
  type IntruderPluginConfigPreset,
} from './pluginConfigPresets'

const props = defineProps<{
  open: boolean
  title: string
  pluginId: string
  presetName: string
  schema: Record<string, any> | null
  modelValue: string
}>()

const emit = defineEmits<{
  (e: 'update:modelValue', value: string): void
  (e: 'update:presetName', value: string): void
  (e: 'close'): void
}>()
const { t } = useI18n()

const paramValues = ref<Record<string, any>>({})
const arrayDraftValues = ref<Record<string, string>>({})
const objectDraftValues = ref<Record<string, { key: string; value: string }>>({})
const jsonErrors = ref<Record<string, string>>({})
const generalError = ref('')
const rawConfigText = ref('{}')
const editorMode = ref<'form' | 'json'>('form')
const availablePresets = ref<IntruderPluginConfigPreset[]>([])
const selectedPresetName = ref('')
const dictionaryPickerOpen = ref(false)
const dictionaryPickerLoading = ref(false)
const dictionaryPickerError = ref('')
const dictionaryPickerFieldKey = ref('')
const dictionaryPickerFieldProp = ref<Record<string, any> | null>(null)
const dictionaryPickerAction = ref<'insert' | 'reference'>('insert')
const dictionaryPickerSearch = ref('')
const dictionaryPickerType = ref('')
const dictionaryPickerWordLimit = ref(200)
const dictionaryPickerDebouncedSearch = ref('')
const availableDictionaries = ref<IntruderDictionarySummary[]>([])
const selectedDictionaryId = ref('')
const selectedDictionaryIds = ref<string[]>([])
const dictionarySummaryById = ref<Record<string, IntruderDictionarySummary>>({})
const dictionaryPickerPage = ref(1)
const dictionaryPickerTotal = ref(0)
const defaultDictionaryMap = ref<Record<string, string>>({})
const DICTIONARY_PICKER_PAGE_SIZE = 20
const DICTIONARY_SEARCH_DEBOUNCE_MS = 250
const DICTIONARY_PREVIEW_WORD_LIMIT = 12
const dictionaryPreviewWords = ref<string[]>([])
const dictionaryPreviewLoading = ref(false)
const dictionaryPreviewError = ref('')
let dictionarySearchDebounceTimer: ReturnType<typeof setTimeout> | null = null

const hasErrors = computed(() => Boolean(generalError.value) || Object.values(jsonErrors.value).some(Boolean))
const editableSchema = computed(() => {
  const configSchema = props.schema?.properties?.config
  if (configSchema && configSchema.type === 'object' && configSchema.properties) {
    return configSchema as Record<string, any>
  }
  return props.schema
})
const editableSchemaProperties = computed(() => editableSchema.value?.properties || {})
const hasEditableSchema = computed(() => Object.keys(editableSchemaProperties.value).length > 0)
const editableSchemaDescription = computed(() => {
  const configSchema = props.schema?.properties?.config
  return configSchema?.description || editableSchema.value?.description || ''
})
const fieldGroups = computed(() => {
  const groups = new Map<string, {
    id: string
    label: string
    description: string
    collapsed: boolean
    fields: Array<{ key: string; prop: Record<string, any> }>
  }>()

  for (const [key, prop] of Object.entries(editableSchemaProperties.value) as Array<[string, Record<string, any>]>) {
    if (!isFieldVisible(key, prop)) {
      continue
    }

    const config = resolveFieldGroupConfig(key, prop)
    const existing = groups.get(config.id)
    if (existing) {
      existing.fields.push({ key, prop })
      continue
    }

    groups.set(config.id, {
      ...config,
      fields: [{ key, prop }],
    })
  }

  return Array.from(groups.values())
})
const dictionaryPickerTypeOptions = computed(() => {
  const allowedTypes = getDictionaryAllowedTypes(dictionaryPickerFieldProp.value)
  if (allowedTypes.length > 0) {
    return allowedTypes
  }

  return Array.from(new Set(availableDictionaries.value.map((item) => item.dict_type).filter(Boolean))).sort()
})
const filteredDictionaries = computed(() => availableDictionaries.value)
const selectedDictionary = computed(() =>
  filteredDictionaries.value.find((item) => item.id === selectedDictionaryId.value)
  || availableDictionaries.value.find((item) => item.id === selectedDictionaryId.value)
  || dictionarySummaryById.value[selectedDictionaryId.value]
  || null,
)
const selectedDictionaries = computed(() =>
  selectedDictionaryIds.value
    .map((id) => dictionarySummaryById.value[id])
    .filter((item): item is IntruderDictionarySummary => Boolean(item)),
)
const activeDictionaryType = computed(() => {
  const selectedType = dictionaryPickerType.value.trim()
  if (selectedType) {
    return selectedType
  }

  const allowedTypes = getDictionaryAllowedTypes(dictionaryPickerFieldProp.value)
  return allowedTypes.length === 1 ? allowedTypes[0] : ''
})
const dictionaryPickerTotalPages = computed(() =>
  Math.max(1, Math.ceil(dictionaryPickerTotal.value / DICTIONARY_PICKER_PAGE_SIZE)),
)
const dictionarySelectionEstimate = computed(() => {
  if (dictionaryPickerAction.value === 'reference') {
    return selectedDictionaryIds.value.length
  }

  const perDictionaryLimit = Math.max(1, Number(dictionaryPickerWordLimit.value) || 200)
  return selectedDictionaries.value.reduce((total, dictionary) => {
    const wordCount = Math.max(0, Number(dictionary.word_count) || 0)
    return total + Math.min(wordCount || perDictionaryLimit, perDictionaryLimit)
  }, 0)
})

watch(
  () => [props.open, props.schema, props.modelValue, props.pluginId, props.presetName] as const,
  ([open]) => {
    if (!open) return
    initializeForm()
    refreshAvailablePresets()
  },
  { immediate: true },
)

watch(editorMode, (mode, previousMode) => {
  if (mode === previousMode) return

  if (mode === 'json') {
    try {
      rawConfigText.value = JSON.stringify(serializeParams(), null, 2)
      generalError.value = ''
    } catch {
      // Keep the current form state and let existing field errors speak for themselves.
    }
    return
  }

  try {
    const parsed = parseRawConfigText()
    applyParsedConfigToForm(parsed)
    generalError.value = ''
  } catch (error) {
    generalError.value = error instanceof Error ? error.message : 'Failed to parse plugin config'
  }
})

function initializeForm() {
  jsonErrors.value = {}
  generalError.value = ''
  editorMode.value = 'form'
  arrayDraftValues.value = {}
  objectDraftValues.value = {}
  selectedPresetName.value = props.presetName

  let parsedConfig: Record<string, any> = {}
  try {
    parsedConfig = props.modelValue.trim() ? JSON.parse(props.modelValue) : {}
  } catch {
    generalError.value = 'Invalid plugin configuration JSON'
  }

  applyParsedConfigToForm(parsedConfig)
  rawConfigText.value = JSON.stringify(parsedConfig, null, 2)
  void preloadReferencedDictionarySummaries()
}

onUnmounted(() => {
  if (dictionarySearchDebounceTimer) {
    clearTimeout(dictionarySearchDebounceTimer)
    dictionarySearchDebounceTimer = null
  }
})

function refreshAvailablePresets() {
  availablePresets.value = loadIntruderPluginConfigPresets(props.pluginId)
  if (props.presetName && availablePresets.value.some((preset) => preset.name === props.presetName)) {
    selectedPresetName.value = props.presetName
    return
  }

  if (selectedPresetName.value && !availablePresets.value.some((preset) => preset.name === selectedPresetName.value)) {
    selectedPresetName.value = ''
  }
}

function supportsDictionaryField(prop: Record<string, any>): boolean {
  return prop.type === 'array' && Boolean(prop['x-dictionary'])
}

function supportsDictionaryInsert(prop: Record<string, any>): boolean {
  const dictionaryConfig = prop['x-dictionary']
  if (!dictionaryConfig) {
    return false
  }

  if (dictionaryConfig.mode === 'insert') {
    return true
  }
  if (dictionaryConfig.mode === 'reference') {
    return false
  }

  return dictionaryConfig.allowInsert !== false
}

function supportsDictionaryReference(prop: Record<string, any>): boolean {
  const dictionaryConfig = prop['x-dictionary']
  if (!dictionaryConfig) {
    return false
  }

  if (dictionaryConfig.mode === 'reference') {
    return true
  }
  if (dictionaryConfig.mode === 'insert') {
    return false
  }

  return Boolean(dictionaryConfig.allowReference)
}

function supportsDictionaryDefault(prop: Record<string, any>): boolean {
  return Boolean(prop['x-dictionary']?.allowDefault)
}

function getDictionaryAllowedTypes(prop: Record<string, any> | null): string[] {
  const dictTypes = prop?.['x-dictionary']?.dictTypes
  if (!Array.isArray(dictTypes)) {
    return []
  }

  return dictTypes.filter((item: unknown): item is string => typeof item === 'string' && item.trim().length > 0)
}

function usesStructuredDictionarySources(prop: Record<string, any>): boolean {
  return prop['x-dictionary']?.storeAs === 'structuredSources'
}

function getDictionaryTypeLabel(dictType: string): string {
  const translationKey = getIntruderDictionaryTypeTranslationKey(dictType)
  return t(`dictionary.types.${translationKey}`, dictType)
}

function formatArrayItemLabel(prop: Record<string, any>, item: string): string {
  if (!supportsDictionaryReference(prop)) {
    return item
  }

  const defaultDictType = parseIntruderDefaultDictionaryReference(item)
  if (defaultDictType) {
    return t('trafficAnalysis.intruder.labels.defaultDictionaryLabel', { type: getDictionaryTypeLabel(defaultDictType) })
  }

  const dictionary = dictionarySummaryById.value[item]
  return dictionary ? `${dictionary.name} (${dictionary.id})` : item
}

function handleSave() {
  try {
    const parsed = buildCurrentConfigObject()
    emit('update:modelValue', JSON.stringify(parsed, null, 2))
    emit('update:presetName', selectedPresetName.value)
    emit('close')
  } catch (error) {
    generalError.value = error instanceof Error ? error.message : 'Failed to save plugin config'
  }
}

function buildCurrentConfigObject(): Record<string, any> {
  return editorMode.value === 'json' ? parseRawConfigText() : serializeParams()
}

function applyImportedConfig(config: Record<string, any>) {
  rawConfigText.value = JSON.stringify(config, null, 2)
  applyParsedConfigToForm(config)
  jsonErrors.value = {}
  generalError.value = ''
}

async function openDictionaryPicker(
  fieldKey: string,
  fieldProp: Record<string, any>,
  action: 'insert' | 'reference',
) {
  dictionaryPickerFieldKey.value = fieldKey
  dictionaryPickerFieldProp.value = fieldProp
  dictionaryPickerAction.value = action
  dictionaryPickerSearch.value = ''
  dictionaryPickerDebouncedSearch.value = ''
  const allowedTypes = getDictionaryAllowedTypes(fieldProp)
  const storedType = loadIntruderDictionaryPickerType()
  dictionaryPickerType.value = storedType && (allowedTypes.length === 0 || allowedTypes.includes(storedType))
    ? storedType
    : allowedTypes[0] || ''
  dictionaryPickerWordLimit.value = Math.max(1, Number(fieldProp?.['x-dictionary']?.maxWords ?? 200))
  dictionaryPickerError.value = ''
  dictionaryPickerPage.value = 1
  dictionaryPickerTotal.value = 0
  selectedDictionaryId.value = ''
  selectedDictionaryIds.value = []
  dictionaryPreviewWords.value = []
  dictionaryPreviewError.value = ''
  dictionaryPickerOpen.value = true
  await refreshDictionariesForPicker()
}

function closeDictionaryPicker() {
  dictionaryPickerOpen.value = false
  dictionaryPickerError.value = ''
  selectedDictionaryIds.value = []
  dictionaryPreviewWords.value = []
  dictionaryPreviewError.value = ''
  if (dictionarySearchDebounceTimer) {
    clearTimeout(dictionarySearchDebounceTimer)
    dictionarySearchDebounceTimer = null
  }
}

async function refreshDictionariesForPicker() {
  dictionaryPickerLoading.value = true
  dictionaryPickerError.value = ''
  try {
    const allowedTypes = getDictionaryAllowedTypes(dictionaryPickerFieldProp.value)
    const activeType = dictionaryPickerType.value.trim()
    const searchTerm = dictionaryPickerDebouncedSearch.value.trim()
    const resolvedType = activeType || (allowedTypes.length === 1 ? allowedTypes[0] : '')
    const [page, defaultMap] = await Promise.all([
      listIntruderDictionariesPaged({
        dictType: resolvedType || null,
        searchTerm: searchTerm || null,
        offset: (dictionaryPickerPage.value - 1) * DICTIONARY_PICKER_PAGE_SIZE,
        limit: DICTIONARY_PICKER_PAGE_SIZE,
      }),
      getIntruderDefaultDictionaryMap(),
    ])

    availableDictionaries.value = allowedTypes.length > 0
      ? page.items.filter((item) => allowedTypes.includes(item.dict_type))
      : page.items
    dictionaryPickerTotal.value = page.total
    defaultDictionaryMap.value = defaultMap
    if (!availableDictionaries.value.some((item) => item.id === selectedDictionaryId.value)) {
      selectedDictionaryId.value = availableDictionaries.value[0]?.id || ''
    }

    dictionarySummaryById.value = availableDictionaries.value.reduce<Record<string, IntruderDictionarySummary>>((acc, item) => {
      acc[item.id] = item
      return acc
    }, { ...dictionarySummaryById.value })
  } catch (error) {
    console.error('Failed to load dictionaries for Intruder plugin config', error)
    dictionaryPickerError.value = error instanceof Error ? error.message : 'Failed to load dictionaries'
    availableDictionaries.value = []
    dictionaryPickerTotal.value = 0
  } finally {
    dictionaryPickerLoading.value = false
  }
}

function changeDictionaryPickerPage(delta: number) {
  const nextPage = dictionaryPickerPage.value + delta
  dictionaryPickerPage.value = Math.min(dictionaryPickerTotalPages.value, Math.max(1, nextPage))
}

function focusDictionary(dictionaryId: string) {
  selectedDictionaryId.value = dictionaryId
}

function toggleDictionarySelection(dictionaryId: string, checked: boolean) {
  if (checked) {
    if (!selectedDictionaryIds.value.includes(dictionaryId)) {
      selectedDictionaryIds.value = [...selectedDictionaryIds.value, dictionaryId]
    }
    selectedDictionaryId.value = dictionaryId
    return
  }

  selectedDictionaryIds.value = selectedDictionaryIds.value.filter((id) => id !== dictionaryId)
  if (selectedDictionaryId.value === dictionaryId) {
    selectedDictionaryId.value = selectedDictionaryIds.value[0] || availableDictionaries.value[0]?.id || ''
  }
}

function selectAllVisibleDictionaries() {
  selectedDictionaryIds.value = Array.from(new Set([
    ...selectedDictionaryIds.value,
    ...filteredDictionaries.value.map((item) => item.id),
  ]))
  if (!selectedDictionaryId.value && filteredDictionaries.value[0]) {
    selectedDictionaryId.value = filteredDictionaries.value[0].id
  }
}

function clearDictionarySelection() {
  selectedDictionaryIds.value = []
}

async function applyDictionarySelection() {
  if (!dictionaryPickerFieldKey.value || selectedDictionaryIds.value.length === 0) {
    return
  }

  const fieldKey = dictionaryPickerFieldKey.value
  try {
    if (dictionaryPickerAction.value === 'reference') {
      const currentItems = getArrayItems(fieldKey)
      paramValues.value[fieldKey] = Array.from(new Set([
        ...currentItems,
        ...selectedDictionaryIds.value,
      ]))
      closeDictionaryPicker()
      return
    }

    const words = (await Promise.all(
      selectedDictionaryIds.value.map((dictionaryId) =>
        listIntruderDictionaryWords(
          dictionaryId,
          Math.max(1, Number(dictionaryPickerWordLimit.value) || 200),
        ),
      ),
    )).flat()
    const currentItems = getArrayItems(fieldKey)
    paramValues.value[fieldKey] = Array.from(new Set([...currentItems, ...words]))
    closeDictionaryPicker()
  } catch (error) {
    console.error('Failed to apply dictionary selection to Intruder plugin config', error)
    dictionaryPickerError.value = error instanceof Error ? error.message : 'Failed to apply dictionary'
  }
}

async function applyDefaultDictionarySelection() {
  const fieldKey = dictionaryPickerFieldKey.value
  const dictType = activeDictionaryType.value.trim()
  if (!fieldKey || !dictType) {
    return
  }

  try {
    if (dictionaryPickerAction.value === 'reference') {
      const defaultReference = createIntruderDefaultDictionaryReference(dictType)
      const currentItems = getArrayItems(fieldKey)
      if (!currentItems.includes(defaultReference)) {
        paramValues.value[fieldKey] = [...currentItems, defaultReference]
      }
      closeDictionaryPicker()
      return
    }

    const dictionaryId = await getIntruderDefaultDictionaryId(dictType)
    if (!dictionaryId) {
      throw new Error(t('trafficAnalysis.intruder.messages.defaultDictionaryUnavailable', { type: dictType }))
    }

    const words = await listIntruderDictionaryWords(
      dictionaryId,
      Math.max(1, Number(dictionaryPickerWordLimit.value) || 200),
    )
    const currentItems = getArrayItems(fieldKey)
    paramValues.value[fieldKey] = Array.from(new Set([...currentItems, ...words]))
    closeDictionaryPicker()
  } catch (error) {
    console.error('Failed to apply default dictionary selection to Intruder plugin config', error)
    dictionaryPickerError.value = error instanceof Error ? error.message : 'Failed to apply default dictionary'
  }
}

async function loadSelectedDictionaryPreview() {
  if (!selectedDictionary.value) {
    dictionaryPreviewWords.value = []
    dictionaryPreviewError.value = ''
    dictionaryPreviewLoading.value = false
    return
  }

  dictionaryPreviewLoading.value = true
  dictionaryPreviewError.value = ''
  try {
    dictionaryPreviewWords.value = await listIntruderDictionaryWords(
      selectedDictionary.value.id,
      DICTIONARY_PREVIEW_WORD_LIMIT,
    )
  } catch (error) {
    console.error('Failed to load dictionary preview words for Intruder plugin config', error)
    dictionaryPreviewWords.value = []
    dictionaryPreviewError.value = error instanceof Error ? error.message : 'Failed to load dictionary preview'
  } finally {
    dictionaryPreviewLoading.value = false
  }
}

async function preloadReferencedDictionarySummaries() {
  const referenceIds = new Set<string>()

  for (const [key, prop] of Object.entries(editableSchemaProperties.value) as Array<[string, Record<string, any>]>) {
    if (!supportsDictionaryReference(prop)) {
      continue
    }

    for (const item of getArrayItems(key)) {
      const normalizedItem = item.trim()
      if (normalizedItem && !isIntruderDefaultDictionaryReference(normalizedItem)) {
        referenceIds.add(normalizedItem)
      }
    }
  }

  if (referenceIds.size === 0) {
    return
  }

  try {
    const dictionaries = await listIntruderDictionaries({ limit: 200 })
    dictionarySummaryById.value = dictionaries.reduce<Record<string, IntruderDictionarySummary>>((acc, item) => {
      acc[item.id] = item
      return acc
    }, { ...dictionarySummaryById.value })
  } catch (error) {
    console.error('Failed to preload dictionary summaries for Intruder plugin config', error)
  }
}

async function importPreset() {
  try {
    const selected = await openDialog({
      multiple: false,
      directory: false,
      filters: [{ name: 'JSON', extensions: ['json'] }],
    })
    if (!selected || Array.isArray(selected)) return

    const content = await readTextFile(selected)
    const parsed = JSON.parse(content)
    if (!parsed || typeof parsed !== 'object' || Array.isArray(parsed)) {
      throw new Error('Preset must be a JSON object')
    }

    applyImportedConfig(parsed as Record<string, any>)
    selectedPresetName.value = ''
    dialog.toast.success(t('trafficAnalysis.intruder.messages.presetImported'))
  } catch (error) {
    console.error('Failed to import Intruder plugin preset', error)
    dialog.toast.error(error instanceof Error ? error.message : t('trafficAnalysis.intruder.messages.presetImportFailed'))
  }
}

async function exportPreset() {
  try {
    const selected = await saveDialog({
      defaultPath: 'intruder-plugin-preset.json',
      filters: [{ name: 'JSON', extensions: ['json'] }],
    })
    if (!selected || Array.isArray(selected)) return

    await writeTextFile(selected, JSON.stringify(buildCurrentConfigObject(), null, 2))
    dialog.toast.success(t('trafficAnalysis.intruder.messages.presetExported'))
  } catch (error) {
    console.error('Failed to export Intruder plugin preset', error)
    dialog.toast.error(error instanceof Error ? error.message : t('trafficAnalysis.intruder.messages.presetExportFailed'))
  }
}

function applySelectedPreset() {
  if (!selectedPresetName.value) {
    return
  }

  const preset = availablePresets.value.find((item) => item.name === selectedPresetName.value)
  if (!preset) {
    return
  }

  try {
    const parsed = JSON.parse(preset.configText)
    if (!parsed || typeof parsed !== 'object' || Array.isArray(parsed)) {
      throw new Error('Preset must be a JSON object')
    }

    applyImportedConfig(parsed as Record<string, any>)
    dialog.toast.success(t('trafficAnalysis.intruder.messages.presetApplied'))
  } catch (error) {
    console.error('Failed to apply Intruder plugin preset', error)
    dialog.toast.error(error instanceof Error ? error.message : t('trafficAnalysis.intruder.messages.presetApplyFailed'))
  }
}

async function savePreset() {
  if (!props.pluginId) {
    return
  }

  const presetName = await dialog.input({
    title: t('trafficAnalysis.intruder.actions.savePreset'),
    message: t('trafficAnalysis.intruder.messages.presetName'),
    defaultValue: selectedPresetName.value || '',
    placeholder: t('trafficAnalysis.intruder.placeholders.presetName'),
  })
  if (!presetName?.trim()) {
    return
  }

  saveIntruderPluginConfigPreset(
    props.pluginId,
    presetName,
    JSON.stringify(buildCurrentConfigObject(), null, 2),
  )
  refreshAvailablePresets()
  selectedPresetName.value = presetName.trim()
  dialog.toast.success(t('trafficAnalysis.intruder.messages.presetSaved'))
}

async function deletePreset() {
  if (!props.pluginId || !selectedPresetName.value) {
    return
  }

  const confirmed = await dialog.confirm({
    title: t('trafficAnalysis.intruder.actions.deletePreset'),
    message: t('trafficAnalysis.intruder.messages.confirmDeletePreset', { name: selectedPresetName.value }),
  })
  if (!confirmed) {
    return
  }

  deleteIntruderPluginConfigPreset(props.pluginId, selectedPresetName.value)
  selectedPresetName.value = ''
  refreshAvailablePresets()
  dialog.toast.success(t('trafficAnalysis.intruder.messages.presetDeleted'))
}

function serializeParams(): Record<string, any> {
  const output: Record<string, any> = {}
  const properties = editableSchemaProperties.value
  const nextJsonErrors: Record<string, string> = {}

  for (const [key, prop] of Object.entries(properties) as Array<[string, any]>) {
    if (!isFieldVisible(key, prop)) {
      continue
    }
    if (isFieldDisabled(key, prop)) {
      continue
    }

    const value = paramValues.value[key]

    if (prop.type === 'array') {
      const arrayValues = Array.isArray(value) ? value : []
      output[key] = usesStructuredDictionarySources(prop)
        ? encodeIntruderStructuredDictionarySources(
            normalizeIntruderDictionaryReferenceValues(arrayValues),
            dictionarySummaryById.value,
          )
        : arrayValues
      continue
    }

    if (prop.type === 'object') {
      if (supportsKeyValueObjectEditor(prop)) {
        output[key] = Array.isArray(value)
          ? value.reduce((record: Record<string, string>, entry: { key: string; value: string }) => {
              const normalizedKey = typeof entry?.key === 'string' ? entry.key.trim() : ''
              if (!normalizedKey) {
                return record
              }
              record[normalizedKey] = typeof entry?.value === 'string' ? entry.value : String(entry?.value ?? '')
              return record
            }, {})
          : {}
        continue
      }

      if (!value || (typeof value === 'string' && !value.trim())) {
        output[key] = {}
        continue
      }

      try {
        output[key] = typeof value === 'string' ? JSON.parse(value) : value
      } catch {
        nextJsonErrors[key] = 'Invalid JSON object'
      }
      continue
    }

    output[key] = value
  }

  jsonErrors.value = nextJsonErrors
  if (Object.keys(nextJsonErrors).length > 0) {
    throw new Error('Fix invalid JSON fields before saving')
  }

  return output
}

function parseRawConfigText(): Record<string, any> {
  generalError.value = ''
  const trimmed = rawConfigText.value.trim()
  if (!trimmed) {
    return {}
  }

  const parsed = JSON.parse(trimmed)
  if (!parsed || typeof parsed !== 'object' || Array.isArray(parsed)) {
    throw new Error('Plugin configuration must be a JSON object')
  }

  return parsed as Record<string, any>
}

function buildDefaultFieldValue(prop: Record<string, any>): any {
  if (prop.default !== undefined) {
    if (prop.type === 'array' && Array.isArray(prop.default)) {
      return [...prop.default]
    }
    if (prop.type === 'object' && supportsKeyValueObjectEditor(prop) && prop.default && typeof prop.default === 'object') {
      return toObjectEntries(prop.default as Record<string, unknown>)
    }
    if (prop.type === 'object' && prop.default && typeof prop.default === 'object') {
      return JSON.stringify(prop.default, null, 2)
    }
    return prop.default
  }

  switch (prop.type) {
    case 'boolean':
      return false
    case 'array':
      return []
    case 'object':
      return supportsKeyValueObjectEditor(prop) ? [] : '{}'
    default:
      return ''
  }
}

function resetToDefaults() {
  const nextValues: Record<string, any> = {}
  const nextConfig: Record<string, any> = {}
  for (const [key, prop] of Object.entries(editableSchemaProperties.value) as Array<[string, any]>) {
    const fieldValue = buildDefaultFieldValue(prop)
    nextValues[key] = fieldValue

    if (prop.default !== undefined) {
      if (prop.type === 'array') {
        nextConfig[key] = Array.isArray(prop.default) ? prop.default : []
      } else if (prop.type === 'object') {
        nextConfig[key] = prop.default && typeof prop.default === 'object' ? prop.default : {}
      } else {
        nextConfig[key] = prop.default
      }
    }

    if (prop.type === 'object' && supportsKeyValueObjectEditor(prop)) {
      objectDraftValues.value[key] = { key: '', value: '' }
    }
  }
  paramValues.value = nextValues
  rawConfigText.value = JSON.stringify(nextConfig, null, 2)
  arrayDraftValues.value = {}
  objectDraftValues.value = {}
  selectedPresetName.value = ''
  jsonErrors.value = {}
  generalError.value = ''
}

function formatFieldLabel(key: string): string {
  return key
    .replace(/[_-]+/g, ' ')
    .replace(/([a-z0-9])([A-Z])/g, '$1 $2')
    .replace(/\b\w/g, (char) => char.toUpperCase())
}

function resolveFieldGroupConfig(key: string, prop: Record<string, any>) {
  const rawGroup = prop['x-ui-group']
  if (typeof rawGroup === 'string' && rawGroup.trim()) {
    return {
      id: rawGroup.trim(),
      label: formatFieldLabel(rawGroup.trim()),
      description: '',
      collapsed: false,
    }
  }

  if (rawGroup && typeof rawGroup === 'object') {
    const groupId = typeof rawGroup.key === 'string' && rawGroup.key.trim()
      ? rawGroup.key.trim()
      : typeof rawGroup.label === 'string' && rawGroup.label.trim()
        ? rawGroup.label.trim()
        : key

    return {
      id: groupId,
      label: typeof rawGroup.label === 'string' && rawGroup.label.trim()
        ? rawGroup.label.trim()
        : formatFieldLabel(groupId),
      description: typeof rawGroup.description === 'string' ? rawGroup.description : '',
      collapsed: Boolean(rawGroup.collapsed),
    }
  }

  return {
    id: 'general',
    label: t('trafficAnalysis.intruder.labels.generalGroup'),
    description: '',
    collapsed: false,
  }
}

function isFieldVisible(_key: string, prop: Record<string, any>): boolean {
  if (!prop['x-ui-visible-when']) {
    return true
  }
  return evaluateFieldRule(prop['x-ui-visible-when'])
}

function isFieldDisabled(_key: string, prop: Record<string, any>): boolean {
  return evaluateFieldRule(prop['x-ui-disabled-when'])
}

function getFieldDisabledReason(_key: string, prop: Record<string, any>): string {
  if (!isFieldDisabled(_key, prop)) {
    return ''
  }
  return typeof prop['x-ui-disabled-reason'] === 'string' ? prop['x-ui-disabled-reason'] : ''
}

function evaluateFieldRule(rule: unknown): boolean {
  if (!rule) {
    return false
  }

  if (typeof rule === 'string') {
    return Boolean(paramValues.value[rule])
  }

  if (typeof rule !== 'object') {
    return false
  }

  const dependencyField = typeof (rule as Record<string, unknown>).field === 'string'
    ? (rule as Record<string, unknown>).field as string
    : ''
  if (!dependencyField) {
    return false
  }

  const dependencyValue = paramValues.value[dependencyField]
  const condition = rule as Record<string, any>

  if (Object.prototype.hasOwnProperty.call(condition, 'equals')) {
    return dependencyValue === condition.equals
  }

  if (Object.prototype.hasOwnProperty.call(condition, 'notEquals')) {
    return dependencyValue !== condition.notEquals
  }

  if (Array.isArray(condition.in)) {
    return condition.in.includes(dependencyValue)
  }

  if (Array.isArray(condition.notIn)) {
    return !condition.notIn.includes(dependencyValue)
  }

  if (condition.truthy === true) {
    return Boolean(dependencyValue)
  }

  if (condition.falsy === true) {
    return !dependencyValue
  }

  return false
}

function applyParsedConfigToForm(parsedConfig: Record<string, any>) {
  const nextValues: Record<string, any> = {}
  const properties = editableSchemaProperties.value

  for (const [key, prop] of Object.entries(properties) as Array<[string, any]>) {
    const aliasKey = typeof prop?.['x-dictionary']?.aliasFrom === 'string'
      ? prop['x-dictionary'].aliasFrom.trim()
      : ''
    const rawValue = parsedConfig[key] ?? (aliasKey ? parsedConfig[aliasKey] : undefined)
    const currentValue = rawValue ?? prop.default

    if (prop.type === 'array') {
      if (usesStructuredDictionarySources(prop) && isIntruderStructuredDictionarySourceArray(currentValue)) {
        nextValues[key] = decodeIntruderStructuredDictionarySources(currentValue)
        continue
      }

      nextValues[key] = Array.isArray(currentValue)
        ? currentValue.filter((item): item is string => typeof item === 'string').map((item) => item.trim()).filter(Boolean)
        : []
      continue
    }

    if (prop.type === 'object') {
      if (supportsKeyValueObjectEditor(prop)) {
        nextValues[key] = currentValue && typeof currentValue === 'object' && !Array.isArray(currentValue)
          ? toObjectEntries(currentValue as Record<string, unknown>)
          : []
        objectDraftValues.value[key] = { key: '', value: '' }
        continue
      }

      nextValues[key] = currentValue && typeof currentValue === 'object' ? JSON.stringify(currentValue, null, 2) : '{}'
      continue
    }

    if (prop.type === 'boolean') {
      nextValues[key] = Boolean(currentValue)
      continue
    }

    nextValues[key] = currentValue ?? ''
  }

  paramValues.value = nextValues
}

function supportsKeyValueObjectEditor(prop: Record<string, any>): boolean {
  if (prop['x-ui-widget'] === 'json') {
    return false
  }
  return !prop.properties
}

function getFieldContainerClass(prop: Record<string, any>): string {
  return isWideField(prop) ? 'lg:col-span-2' : ''
}

function getFieldStateClass(key: string, prop: Record<string, any>): string {
  return isFieldDisabled(key, prop) ? 'opacity-60' : ''
}

function getSelectedEnumDescription(
  prop: Record<string, any>,
  value: unknown,
): string {
  if (value == null || value === '') {
    return ''
  }

  const descriptions = prop['x-ui-enum-descriptions']
  if (!descriptions || typeof descriptions !== 'object' || Array.isArray(descriptions)) {
    return ''
  }

  const matched = descriptions[String(value)]
  return typeof matched === 'string' ? matched : ''
}

function isWideField(prop: Record<string, any>): boolean {
  if (prop.type === 'array' || prop.type === 'object') {
    return true
  }
  if (prop['x-ui-widget'] === 'textarea' || prop['x-ui-widget'] === 'textarea-lines') {
    return true
  }
  return false
}

function toObjectEntries(value: Record<string, unknown>) {
  return Object.entries(value).map(([entryKey, entryValue]) => ({
    key: entryKey,
    value: entryValue == null ? '' : String(entryValue),
  }))
}

function getArrayItems(key: string): string[] {
  return Array.isArray(paramValues.value[key]) ? paramValues.value[key] : []
}

function addArrayItem(key: string) {
  const draft = arrayDraftValues.value[key]?.trim() || ''
  if (!draft) return

  const currentItems = getArrayItems(key)
  if (!currentItems.includes(draft)) {
    paramValues.value[key] = [...currentItems, draft]
  }
  arrayDraftValues.value[key] = ''
}

function removeArrayItem(key: string, index: number) {
  const currentItems = getArrayItems(key)
  paramValues.value[key] = currentItems.filter((_, currentIndex) => currentIndex !== index)
}

function getObjectEntries(key: string): Array<{ key: string; value: string }> {
  return Array.isArray(paramValues.value[key]) ? paramValues.value[key] : []
}

function getObjectDraft(key: string): { key: string; value: string } {
  const draft = objectDraftValues.value[key]
  if (draft) {
    return draft
  }
  const nextDraft = { key: '', value: '' }
  objectDraftValues.value[key] = nextDraft
  return nextDraft
}

function addObjectEntry(key: string) {
  const draft = getObjectDraft(key)
  const normalizedKey = draft.key.trim()
  if (!normalizedKey) return

  const currentEntries = getObjectEntries(key)
  const nextEntries = currentEntries.filter((entry) => entry.key !== normalizedKey)
  nextEntries.push({
    key: normalizedKey,
    value: draft.value,
  })
  paramValues.value[key] = nextEntries
  objectDraftValues.value[key] = { key: '', value: '' }
}

function updateObjectDraft(
  key: string,
  field: 'key' | 'value',
  value: string,
) {
  const draft = getObjectDraft(key)
  objectDraftValues.value[key] = {
    ...draft,
    [field]: value,
  }
}

function updateObjectEntry(
  key: string,
  index: number,
  field: 'key' | 'value',
  value: string,
) {
  const currentEntries = getObjectEntries(key)
  paramValues.value[key] = currentEntries.map((entry, currentIndex) => (
    currentIndex === index ? { ...entry, [field]: value } : entry
  ))
}

function removeObjectEntry(key: string, index: number) {
  const currentEntries = getObjectEntries(key)
  paramValues.value[key] = currentEntries.filter((_, currentIndex) => currentIndex !== index)
}

watch(dictionaryPickerType, async (value, previousValue) => {
  if (!dictionaryPickerOpen.value || value === previousValue) {
    return
  }
  saveIntruderDictionaryPickerType(value)
  dictionaryPickerPage.value = 1
  await refreshDictionariesForPicker()
})

watch(dictionaryPickerSearch, (value, previousValue) => {
  if (!dictionaryPickerOpen.value || value === previousValue) {
    return
  }
  if (dictionarySearchDebounceTimer) {
    clearTimeout(dictionarySearchDebounceTimer)
  }
  dictionarySearchDebounceTimer = setTimeout(() => {
    dictionaryPickerPage.value = 1
    dictionaryPickerDebouncedSearch.value = value
  }, DICTIONARY_SEARCH_DEBOUNCE_MS)
})

watch(dictionaryPickerDebouncedSearch, async (value, previousValue) => {
  if (!dictionaryPickerOpen.value || value === previousValue) {
    return
  }
  await refreshDictionariesForPicker()
})

watch(dictionaryPickerPage, async (value, previousValue) => {
  if (!dictionaryPickerOpen.value || value === previousValue) {
    return
  }
  await refreshDictionariesForPicker()
})

watch(selectedDictionaryId, async (value, previousValue) => {
  if (!dictionaryPickerOpen.value || value === previousValue) {
    return
  }
  await loadSelectedDictionaryPreview()
})
</script>
