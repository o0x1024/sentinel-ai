<template>
  <div class="mb-3 flex flex-wrap items-center gap-2">
    <span class="text-sm text-base-content/70">预设</span>
    <div class="flex flex-wrap gap-2">
      <button
        v-for="preset in presets"
        :key="preset.id"
        class="btn btn-xs"
        :class="activePresetId === preset.id ? 'btn-primary' : 'btn-outline'"
        :title="preset.description"
        @click="$emit('apply-preset', preset.id)"
      >
        {{ preset.label }}
      </button>
      <button class="btn btn-ghost btn-xs" @click="$emit('reset-filters')">清空预设</button>
    </div>
    <div class="ml-auto flex flex-wrap items-center gap-2">
      <select
        class="select select-bordered select-xs min-w-[12rem]"
        :value="activeSavedViewId || ''"
        @change="$emit('apply-saved-view', (($event.target as HTMLSelectElement).value || null))"
      >
        <option value="">已保存视图</option>
        <option v-for="view in savedViews" :key="view.id" :value="view.id">
          {{ view.name }}
        </option>
      </select>
      <button class="btn btn-outline btn-xs" @click="$emit('save-current-view')">保存当前视图</button>
      <button class="btn btn-ghost btn-xs" :disabled="savedViews.length === 0" @click="$emit('export-saved-views')">
        导出视图
      </button>
      <button class="btn btn-ghost btn-xs" @click="$emit('import-saved-views')">导入视图</button>
      <button
        class="btn btn-ghost btn-xs"
        :disabled="!activeSavedViewId"
        @click="$emit('delete-saved-view', activeSavedViewId)"
      >
        删除当前视图
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
import type { FindingFilterPresetId } from './vulnerabilitiesFilterPresets'
import type { SavedVulnerabilityFilterView } from './vulnerabilitiesSavedViews'

defineProps<{
  presets: Array<{
    id: FindingFilterPresetId
    label: string
    description: string
  }>
  activePresetId: FindingFilterPresetId | null
  savedViews: SavedVulnerabilityFilterView[]
  activeSavedViewId: string | null
}>()

defineEmits<{
  'apply-preset': [presetId: FindingFilterPresetId]
  'reset-filters': []
  'apply-saved-view': [viewId: string | null]
  'save-current-view': []
  'export-saved-views': []
  'import-saved-views': []
  'delete-saved-view': [viewId: string | null]
}>()
</script>
